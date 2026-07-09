//! [`EvalContextBuilder`] — materialize an [`EvalContext`] from an entity snapshot.
//!
//! The DSL evaluator is pure: the host must resolve entity tags/traits/statuses
//! and scalar fields ahead of time. This builder does that from an
//! [`EntitySnapshot`] (a scene-agnostic view of an entity), pulling active-status
//! tags from the [`StatusEffectService`] and trait/item tags from the
//! [`RuntimeContentStore`], so trigger `when` conditions like
//! `has_tag(self, 'poisoned')` or `event.hit == true` evaluate correctly.
//!
//! Combat supplies snapshots via [`EntitySnapshot::from_combatant`]; other scenes
//! build their own, so the trigger runtime is decoupled from combat types.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::character::Character;
use crate::combat_runtime::Combatant;
use crate::dsl::{EntityView, EvalContext};
use crate::resolution::damage::{Mitigation, MitigationKind, Pool};
use crate::status_effects::service::{ContentLookup, StatusEffectService, WorldClock};

use super::store::RuntimeContentStore;

/// Derive damage-pipeline mitigations from named defenses: a defense named `dr`
/// reduces incoming damage (GURPS/XWN armor-as-DR), `armor` negates hits at or
/// below its value (Savage-style threshold). Other defenses are avoidance values
/// (used in the attack contest, not the damage pipeline) and are ignored here.
pub(crate) fn mitigations_from_defenses(defenses: &BTreeMap<String, i64>) -> Vec<Mitigation> {
    let mut out = Vec::new();
    if let Some(&v) = defenses.get("dr") {
        out.push(Mitigation {
            kind: MitigationKind::DamageResistance,
            value: v,
            applies_to_types: Vec::new(),
        });
    }
    if let Some(&v) = defenses.get("armor") {
        out.push(Mitigation {
            kind: MitigationKind::ArmorRating,
            value: v,
            applies_to_types: Vec::new(),
        });
    }
    out
}

/// The canonical `hp` pool derived from a combatant's live hp, plus any extra
/// authored pools (shields/MDC). The `hp` pool is first so an untiered packet
/// routes to it by default.
pub(crate) fn pools_with_hp(hp: i32, max_hp: i32, extra: &[Pool]) -> Vec<Pool> {
    let mut pools = Vec::with_capacity(1 + extra.len());
    pools.push(Pool {
        id: "hp".into(),
        tags: vec!["hp".into()],
        current: hp as i64,
        max: max_hp as i64,
        can_go_negative: false,
    });
    pools.extend(extra.iter().cloned());
    pools
}

/// A scene-agnostic snapshot of one entity for a single trigger-evaluation pass.
///
/// It carries exactly what the trigger runtime needs to source an entity's
/// triggers and materialize its [`EntityView`]: the ids of its equipped items and
/// intrinsic traits, plus the scalar fields the DSL exposes (`hp`, `ac`, …).
/// Active statuses/tags are not stored here — they are read live from the
/// [`StatusEffectService`] by id. Any scene (combat, exploration, …) can produce
/// snapshots, which is what lets the interpreter run outside combat.
#[derive(Debug, Clone, PartialEq)]
pub struct EntitySnapshot {
    /// Entity id (keys status/tag lookups and the eval-context entity map).
    pub entity_id: String,
    /// Equipped item ids, already expanded to bridge weapon-id schemes.
    pub equipped_item_ids: Vec<String>,
    /// Intrinsic trait ids the entity carries (distinct from item-granted traits).
    pub intrinsic_trait_ids: Vec<String>,
    /// Scalar fields exposed to the DSL (e.g. `hp`, `max_hp`, `ac`).
    pub scalar_fields: Map<String, Value>,
    /// Health pools for the IR damage pipeline: the derived `hp` pool first, then
    /// any extra authored pools (with live current values).
    pub pools: Vec<Pool>,
    /// Damage mitigations derived from named defenses (`dr`/`armor`).
    pub mitigations: Vec<Mitigation>,
}

impl EntitySnapshot {
    /// Snapshot a combat [`Combatant`]. Equipped-item ids are expanded via
    /// [`weapon_id_candidates`] so an IR item authored under either weapon-id
    /// scheme matches; scalar fields mirror the combat stats the DSL reads.
    pub fn from_combatant(c: &Combatant) -> Self {
        let mut equipped_item_ids = Vec::new();
        if let Some(id) = &c.weapon_id {
            for cand in weapon_id_candidates(id) {
                if !equipped_item_ids.contains(&cand) {
                    equipped_item_ids.push(cand);
                }
            }
        }

        let mut scalar_fields = Map::new();
        scalar_fields.insert("hp".into(), Value::from(c.hp));
        scalar_fields.insert("max_hp".into(), Value::from(c.max_hp));
        scalar_fields.insert("ac".into(), Value::from(c.ac));
        scalar_fields.insert("attack_bonus".into(), Value::from(c.attack_bonus));
        scalar_fields.insert("num_attacks".into(), Value::from(c.num_attacks));
        scalar_fields.insert("alive".into(), Value::from(c.alive));
        scalar_fields.insert("is_player".into(), Value::from(c.is_player));

        Self {
            entity_id: c.entity_id.clone(),
            equipped_item_ids,
            intrinsic_trait_ids: c.traits.clone(),
            scalar_fields,
            pools: pools_with_hp(c.hp, c.max_hp, &c.pools),
            mitigations: mitigations_from_defenses(&c.defenses),
        }
    }

    /// Snapshot a player [`Character`] for non-combat scenes (exploration, etc.).
    /// The wielded weapon is resolved + scheme-bridged the same way combat does,
    /// and the character's intrinsic traits source its triggers.
    pub fn from_character(c: &Character) -> Self {
        let mut equipped_item_ids = Vec::new();
        if let Some(id) = crate::combat::creation::player_weapon_id(c) {
            for cand in weapon_id_candidates(&id) {
                if !equipped_item_ids.contains(&cand) {
                    equipped_item_ids.push(cand);
                }
            }
        }

        let mut scalar_fields = Map::new();
        scalar_fields.insert("hp".into(), Value::from(c.hp));
        scalar_fields.insert("max_hp".into(), Value::from(c.max_hp));
        scalar_fields.insert("ac".into(), Value::from(c.ac));
        scalar_fields.insert("is_player".into(), Value::from(true));

        Self {
            entity_id: c.id.clone(),
            equipped_item_ids,
            intrinsic_trait_ids: c.traits.clone(),
            scalar_fields,
            pools: pools_with_hp(c.hp, c.max_hp, &[]),
            mitigations: Vec::new(),
        }
    }
}

/// Candidate item ids for a wielded weapon, reconciling the kit scheme (bare
/// ids like `short_sword`) with the registry/IR scheme (`weapon.short_sword`).
///
/// A bare id also yields its `weapon.`-prefixed form and vice versa, so an IR
/// item authored under either convention matches the equipped weapon. Ids in
/// some other namespace (already dotted, non-`weapon.`) are passed through
/// unchanged.
fn weapon_id_candidates(weapon_id: &str) -> Vec<String> {
    let mut out = vec![weapon_id.to_string()];
    if let Some(bare) = weapon_id.strip_prefix("weapon.") {
        out.push(bare.to_string());
    } else if !weapon_id.contains('.') {
        out.push(format!("weapon.{weapon_id}"));
    }
    out
}

/// Builds an [`EvalContext`] for trigger evaluation from combatants + services.
pub struct EvalContextBuilder<'a, C: ContentLookup, K: WorldClock> {
    store: &'a RuntimeContentStore,
    status: &'a StatusEffectService<'a, C, K>,
}

impl<'a, C: ContentLookup, K: WorldClock> EvalContextBuilder<'a, C, K> {
    /// Construct a builder over the content store and status service.
    pub fn new(store: &'a RuntimeContentStore, status: &'a StatusEffectService<'a, C, K>) -> Self {
        Self { store, status }
    }

    /// Traits granted by the entity's equipped items.
    fn granted_trait_ids(&self, snap: &EntitySnapshot) -> Vec<String> {
        let mut out = Vec::new();
        for item_id in &snap.equipped_item_ids {
            if let Some(item) = self.store.get_item(item_id) {
                out.extend(item.grants_traits.iter().cloned());
            }
        }
        out
    }

    /// Materialize a single entity view from a snapshot + live status state.
    pub fn entity_view(&self, snap: &EntitySnapshot) -> Result<EntityView, String> {
        let mut tags: BTreeSet<String> = self.status.get_tags(&snap.entity_id)?;

        // Both intrinsic and item-granted traits surface for has_trait() and
        // contribute their provides_tags for has_tag().
        let mut traits = snap.intrinsic_trait_ids.clone();
        traits.extend(self.granted_trait_ids(snap));
        traits.sort();
        traits.dedup();
        for trait_id in &traits {
            if let Some(tr) = self.store.get_trait(trait_id) {
                tags.extend(tr.provides_tags.iter().cloned());
            }
        }
        for item_id in &snap.equipped_item_ids {
            if let Some(item) = self.store.get_item(item_id) {
                tags.extend(item.tags.iter().cloned());
            }
        }

        let statuses: Vec<String> = self
            .status
            .list_for_entity(&snap.entity_id)?
            .into_iter()
            .map(|active| active.effect_id)
            .collect();

        Ok(EntityView {
            id: snap.entity_id.clone(),
            fields: snap.scalar_fields.clone(),
            tags: tags.into_iter().collect(),
            traits,
            statuses,
            actions: Vec::new(),
            skills: Vec::new(),
        })
    }

    /// Build the full context for an event with a `self` and optional `target`.
    pub fn build(
        &self,
        self_snap: &EntitySnapshot,
        target_snap: Option<&EntitySnapshot>,
        event_type: &str,
        event: &Map<String, Value>,
    ) -> Result<EvalContext, String> {
        let mut entities = BTreeMap::new();
        entities.insert(self_snap.entity_id.clone(), self.entity_view(self_snap)?);

        let target_id = match target_snap {
            Some(t) => {
                entities.insert(t.entity_id.clone(), self.entity_view(t)?);
                Some(t.entity_id.clone())
            }
            None => None,
        };

        let mut event_map = event.clone();
        event_map
            .entry("event_type".to_string())
            .or_insert_with(|| Value::String(event_type.to_string()));

        Ok(EvalContext {
            entities,
            self_id: self_snap.entity_id.clone(),
            target_id,
            event: event_map,
            world: Map::new(),
            local: Map::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use crate::ir::ComponentRecord;
    use crate::status_effects::repository::StatusEffectRepository;
    use serde_json::json;

    struct FixedClock(i32);
    impl WorldClock for FixedClock {
        fn tick(&self) -> i32 {
            self.0
        }
    }

    fn seeded_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn store() -> RuntimeContentStore {
        let records: Vec<ComponentRecord> = vec![
            json!({"component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
                   "provides_tags": ["poisoned"], "stacking": "replace"}),
            json!({"component_type": "trait", "id": "envenom_on_hit", "name": "Envenom",
                   "category": "weapon_quality", "provides_tags": ["venomous"]}),
            json!({"component_type": "item", "id": "envenomed_dagger", "name": "Envenomed Dagger",
                   "damage": "1d4", "tags": ["weapon"], "grants_traits": ["envenom_on_hit"]}),
            // Authored under the registry scheme (`weapon.` prefix) to exercise
            // cross-scheme sourcing against a bare kit weapon id.
            json!({"component_type": "item", "id": "weapon.short_sword", "name": "Short Sword",
                   "damage": "1d6", "tags": ["blade"], "grants_traits": ["envenom_on_hit"]}),
        ]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        RuntimeContentStore::from_records(records)
    }

    fn combatant(id: &str, weapon: Option<&str>) -> Combatant {
        Combatant {
            entity_id: id.into(),
            name: id.into(),
            display_name: id.into(),
            is_player: id == "pc",
            initiative: 0,
            hp: 8,
            max_hp: 8,
            ac: 12,
            attack_bonus: 1,
            damage_expr: "1d4".into(),
            attack_description: "stabs".into(),
            behavior: "aggressive".into(),
            num_attacks: 1,
            alive: true,
            weapon_id: weapon.map(str::to_string),
            range_band: "melee".into(),
            traits: vec![],
            pools: vec![],
            defenses: Default::default(),
            actions: vec![],
            inventory: vec![],
            gold: 0,
            q: 0,
            r: 0,
        }
    }

    /// Snapshot helper: a combatant viewed as the scene-agnostic [`EntitySnapshot`].
    fn snap(id: &str, weapon: Option<&str>) -> EntitySnapshot {
        EntitySnapshot::from_combatant(&combatant(id, weapon))
    }

    #[test]
    fn entity_view_maps_scalars_and_item_tags() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let builder = EvalContextBuilder::new(&store, &svc);

        let view = builder
            .entity_view(&snap("pc", Some("envenomed_dagger")))
            .unwrap();
        assert_eq!(view.fields["hp"], json!(8));
        assert_eq!(view.fields["ac"], json!(12));
        // Item tag + granted-trait provides_tags both surface.
        assert!(view.tags.contains(&"weapon".to_string()));
        assert!(view.tags.contains(&"venomous".to_string()));
        // Granted trait id is listed for has_trait().
        assert!(view.traits.contains(&"envenom_on_hit".to_string()));
    }

    #[test]
    fn from_character_snapshot_carries_traits_and_weapon() {
        let mut c = Character::new("Reaver", "warrior");
        c.id = "pc".into();
        c.hp = 12;
        c.max_hp = 16;
        c.ac = 14;
        c.traits = vec!["veteran_grit".into()];
        c.equipment = vec![serde_json::json!({
            "id": "short_sword", "name": "Short Sword", "type": "melee_weapon", "damage": "1d6"
        })
        .as_object()
        .unwrap()
        .clone()];

        let snap = EntitySnapshot::from_character(&c);
        assert_eq!(snap.entity_id, "pc");
        assert_eq!(snap.intrinsic_trait_ids, vec!["veteran_grit"]);
        // Weapon id is resolved and scheme-bridged (bare + `weapon.`-prefixed).
        assert!(snap.equipped_item_ids.contains(&"short_sword".to_string()));
        assert!(snap.equipped_item_ids.contains(&"weapon.short_sword".to_string()));
        assert_eq!(snap.scalar_fields["hp"], json!(12));
        assert_eq!(snap.scalar_fields["max_hp"], json!(16));
        assert_eq!(snap.scalar_fields["is_player"], json!(true));
    }

    #[test]
    fn weapon_id_candidates_bridge_both_schemes() {
        assert_eq!(
            weapon_id_candidates("short_sword"),
            vec!["short_sword", "weapon.short_sword"]
        );
        assert_eq!(
            weapon_id_candidates("weapon.short_sword"),
            vec!["weapon.short_sword", "short_sword"]
        );
        // An id already in another namespace is passed through unchanged.
        assert_eq!(weapon_id_candidates("ammo.bolt"), vec!["ammo.bolt"]);
    }

    #[test]
    fn bare_kit_weapon_id_sources_registry_scheme_item() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let builder = EvalContextBuilder::new(&store, &svc);

        // PC wields the kit-scheme bare id; the IR item is `weapon.short_sword`.
        let view = builder.entity_view(&snap("pc", Some("short_sword"))).unwrap();
        assert!(view.tags.contains(&"blade".to_string()));
        // Its granted trait (and that trait's provides_tags) surface too.
        assert!(view.traits.contains(&"envenom_on_hit".to_string()));
        assert!(view.tags.contains(&"venomous".to_string()));
    }

    #[test]
    fn entity_view_surfaces_intrinsic_traits_and_their_tags() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let builder = EvalContextBuilder::new(&store, &svc);

        // A creature carrying the trait intrinsically (no weapon equipped).
        let mut hound = combatant("hound", None);
        hound.traits = vec!["envenom_on_hit".into()];
        let view = builder
            .entity_view(&EntitySnapshot::from_combatant(&hound))
            .unwrap();
        // The intrinsic trait is listed for has_trait().
        assert!(view.traits.contains(&"envenom_on_hit".to_string()));
        // Its provides_tags surface for has_tag(), exactly as granted traits do.
        assert!(view.tags.contains(&"venomous".to_string()));
    }

    #[test]
    fn entity_view_includes_active_status_tags() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        svc.apply("npc", "poisoned", None, None, None).unwrap();
        let builder = EvalContextBuilder::new(&store, &svc);

        let view = builder.entity_view(&snap("npc", None)).unwrap();
        assert!(view.tags.contains(&"poisoned".to_string()));
        assert!(view.statuses.contains(&"poisoned".to_string()));
    }

    #[test]
    fn build_sets_self_target_and_event() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let builder = EvalContextBuilder::new(&store, &svc);

        let pc = snap("pc", Some("envenomed_dagger"));
        let npc = snap("npc", None);
        let mut event = Map::new();
        event.insert("hit".into(), Value::Bool(true));
        let ctx = builder.build(&pc, Some(&npc), "combat.attack", &event).unwrap();

        assert_eq!(ctx.self_id, "pc");
        assert_eq!(ctx.target_id.as_deref(), Some("npc"));
        assert_eq!(ctx.event["hit"], json!(true));
        assert_eq!(ctx.event["event_type"], json!("combat.attack"));
        assert!(ctx.entities.contains_key("pc"));
        assert!(ctx.entities.contains_key("npc"));
    }
}
