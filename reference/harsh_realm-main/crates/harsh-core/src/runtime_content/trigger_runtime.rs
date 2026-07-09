//! [`TriggerRuntime`] — the live loop that makes IR content drive the game.
//!
//! On each game event it: resolves the acting (`self`) entity and optional
//! `target` from the event payload, gathers that entity's triggers for the event
//! ([`triggers_for_event`]), materializes an [`EvalContext`](crate::dsl::EvalContext)
//! ([`EvalContextBuilder`]), lowers the fired effects to intents
//! ([`dispatch`](crate::dispatch::dispatch)), applies them ([`IntentApplier`]),
//! and cascades any emitted events with a bounded depth.
//!
//! Triggers are evaluated for the **acting** entity only (the event's `self`):
//! the attacker's weapon/trait triggers and the attacker's own status triggers.
//! This is what makes a "poison ticks when the poisoned creature acts" status fire
//! at the right time without a world-clock subscription (deferred).
//!
//! The whole loop is fail-safe: every error path is collected into
//! [`RuntimeOutcome::errors`]; nothing panics or is silently dropped.

use std::cell::RefCell;
use std::collections::BTreeMap;

use rand::rngs::SmallRng;
use rand::SeedableRng;

use crate::db::WorldDatabase;
use crate::dispatch::dispatch;
use crate::events::GameEvent;
use crate::intent::Intent;
use crate::ir::Trigger;
use crate::resolution::damage::{apply_damage, PoolDelta};
use crate::status_effects::service::{ContentLookup, StatusEffectService, WorldClock};

use super::eval_context::{EntitySnapshot, EvalContextBuilder};
use super::index::triggers_for_event;
use super::intent_applier::{ApplyOutcome, IntentApplier};
use super::store::RuntimeContentStore;

/// Supplies entity state to the trigger runtime and receives resource changes.
///
/// Any scene implements this so the runtime can read an entity's snapshot and
/// write back resource deltas (e.g. `hp`) produced by intents. It is deliberately
/// scene-agnostic: combat snapshots [`Combatant`](crate::combat_runtime::Combatant)s,
/// while exploration/other scenes snapshot their own entities — this is what lets
/// the interpreter run outside combat.
pub trait EntityResolver {
    /// Snapshot an entity for one evaluation pass (`None` if absent).
    fn snapshot(&self, entity_id: &str) -> Option<EntitySnapshot>;
    /// Apply a resource delta (e.g. `hp`) to an entity's transient state.
    fn apply_resource_delta(&mut self, entity_id: &str, resource: &str, delta: i64);
}

/// What running the triggers for a batch of events produced.
#[derive(Debug, Default)]
pub struct RuntimeOutcome {
    /// Events emitted by fired effects (already applied/cascaded), for publishing.
    pub emitted: Vec<GameEvent>,
    /// Collected, non-fatal errors.
    pub errors: Vec<String>,
}

/// Orchestrates IR trigger evaluation over game events.
pub struct TriggerRuntime<'a, C: ContentLookup, K: WorldClock> {
    store: &'a RuntimeContentStore,
    status: &'a StatusEffectService<'a, C, K>,
    db: &'a WorldDatabase,
    tick: i64,
    max_depth: usize,
    /// RNG for `roll_dice` compute effects (interior-mutable so `run` stays `&self`).
    rng: RefCell<SmallRng>,
}

impl<'a, C: ContentLookup, K: WorldClock> TriggerRuntime<'a, C, K> {
    /// Construct a runtime over the content store, status service, and world db.
    /// Dice rolls use a fresh entropy-seeded RNG; use [`with_seed`](Self::with_seed)
    /// for deterministic dice in tests.
    pub fn new(
        store: &'a RuntimeContentStore,
        status: &'a StatusEffectService<'a, C, K>,
        db: &'a WorldDatabase,
        tick: i64,
        max_depth: usize,
    ) -> Self {
        Self {
            store,
            status,
            db,
            tick,
            max_depth,
            rng: RefCell::new(SmallRng::from_entropy()),
        }
    }

    /// Like [`new`](Self::new) but with a deterministic dice RNG seed.
    pub fn with_seed(
        store: &'a RuntimeContentStore,
        status: &'a StatusEffectService<'a, C, K>,
        db: &'a WorldDatabase,
        tick: i64,
        max_depth: usize,
        seed: u64,
    ) -> Self {
        Self {
            store,
            status,
            db,
            tick,
            max_depth,
            rng: RefCell::new(SmallRng::seed_from_u64(seed)),
        }
    }

    /// Run triggers for `events`, applying intents to `resolver` and cascading.
    ///
    /// `R: ?Sized` so a `&mut dyn EntityResolver` (the controller's per-scene
    /// resolver selection) works as well as a concrete resolver.
    pub fn run<R: EntityResolver + ?Sized>(
        &self,
        resolver: &mut R,
        events: &[GameEvent],
    ) -> RuntimeOutcome {
        let builder = EvalContextBuilder::new(self.store, self.status);
        let applier = IntentApplier::new(self.db, self.status, self.tick);
        let mut out = RuntimeOutcome::default();
        self.run_events(events, &builder, &applier, resolver, 0, &mut out);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn run_events<R: EntityResolver + ?Sized>(
        &self,
        events: &[GameEvent],
        builder: &EvalContextBuilder<'a, C, K>,
        applier: &IntentApplier<'a, C, K>,
        resolver: &mut R,
        depth: usize,
        out: &mut RuntimeOutcome,
    ) {
        if depth >= self.max_depth {
            if !events.is_empty() {
                out.errors
                    .push(format!("trigger cascade depth limit ({}) reached", self.max_depth));
            }
            return;
        }
        for event in events {
            let (self_id, target_id) = match roles(event) {
                Some(roles) => roles,
                None => continue, // non-entity event (e.g. gm.narrate) — nothing to fire
            };
            // Snapshot the participants (owned) so we can mutate the resolver after.
            let self_snap = match resolver.snapshot(&self_id) {
                Some(s) => s,
                None => continue,
            };
            let target_snap = target_id.as_ref().and_then(|id| resolver.snapshot(id));

            let status_ids = match self.active_status_ids(&self_id) {
                Ok(ids) => ids,
                Err(e) => {
                    out.errors.push(format!("status load for {self_id}: {e}"));
                    continue;
                }
            };

            let trigger_refs = triggers_for_event(
                self.store,
                &event.event_type,
                &status_ids,
                &self_snap.equipped_item_ids,
                &self_snap.intrinsic_trait_ids,
            );
            if trigger_refs.is_empty() {
                continue;
            }
            // Clone the (small) matched triggers so `dispatch` owns its slice and
            // we needn't touch the existing `dispatch` signature.
            let triggers: Vec<Trigger> = trigger_refs.into_iter().cloned().collect();

            let ctx = match builder.build(
                &self_snap,
                target_snap.as_ref(),
                &event.event_type,
                &event.data,
            ) {
                Ok(ctx) => ctx,
                Err(e) => {
                    out.errors
                        .push(format!("context build for {}: {e}", event.event_type));
                    continue;
                }
            };

            let intents = match dispatch(
                &triggers,
                &event.event_type,
                &ctx,
                &mut *self.rng.borrow_mut(),
            ) {
                Ok(intents) => intents,
                Err(e) => {
                    out.errors
                        .push(format!("dispatch {}: {e}", event.event_type));
                    continue;
                }
            };

            let applied = applier.apply(&intents);
            self.route_applied(resolver, &applied);
            out.errors.extend(applied.errors);
            out.emitted.extend(applied.events.iter().cloned());

            // Cascade events emitted by effects (bounded).
            self.run_events(&applied.events, builder, applier, resolver, depth + 1, out);
        }
    }

    /// Apply an [`ApplyOutcome`]'s resource deltas and damage packets to the
    /// resolver — damage routed through the IR pipeline against each target's live
    /// pools/mitigations (snapshot taken fresh so it reflects prior deltas).
    fn route_applied<R: EntityResolver + ?Sized>(&self, resolver: &mut R, applied: &ApplyOutcome) {
        for (entity_id, resource, delta) in &applied.resource_deltas {
            resolver.apply_resource_delta(entity_id, resource, *delta);
        }
        for (entity_id, packet) in &applied.damage_packets {
            let Some(snap) = resolver.snapshot(entity_id) else {
                continue;
            };
            let application = apply_damage(packet, &BTreeMap::new(), &snap.mitigations, &snap.pools);
            for PoolDelta { pool_id, delta } in application.deltas {
                resolver.apply_resource_delta(entity_id, &pool_id, delta);
            }
        }
    }

    /// Apply already-lowered intents (e.g. from an action resolution) to the
    /// resolver, returning emitted events + errors. Unlike [`run`](Self::run) this
    /// does not source or cascade triggers — it is the apply half, exposed so the
    /// turn loop can spend an action's [`Intent`]s through the same path triggers use.
    pub fn apply_intents<R: EntityResolver + ?Sized>(
        &self,
        resolver: &mut R,
        intents: &[Intent],
    ) -> RuntimeOutcome {
        let applier = IntentApplier::new(self.db, self.status, self.tick);
        let applied = applier.apply(intents);
        self.route_applied(resolver, &applied);
        RuntimeOutcome {
            emitted: applied.events,
            errors: applied.errors,
        }
    }

    fn active_status_ids(&self, entity_id: &str) -> Result<Vec<String>, String> {
        Ok(self
            .status
            .list_for_entity(entity_id)?
            .into_iter()
            .map(|active| active.effect_id)
            .collect())
    }
}

/// Extract the `self` (acting) entity and optional `target` from an event's
/// payload. Supports `attacker_id`/`target_id` (combat) and `self_id`/`target_id`.
fn roles(event: &GameEvent) -> Option<(String, Option<String>)> {
    let data = &event.data;
    let self_id = data
        .get("attacker_id")
        .or_else(|| data.get("self_id"))
        .and_then(|v| v.as_str())?
        .to_string();
    let target_id = data
        .get("target_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Some((self_id, target_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat_runtime::Combatant;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use crate::ir::ComponentRecord;
    use crate::status_effects::repository::StatusEffectRepository;
    use serde_json::{json, Map, Value};

    struct DemoClock(i32);
    impl WorldClock for DemoClock {
        fn tick(&self) -> i32 {
            self.0
        }
    }

    /// Minimal resolver over a fixed combatant list.
    struct VecResolver {
        combatants: Vec<Combatant>,
    }
    impl VecResolver {
        /// Inherent accessor for test assertions (reads back mutated hp).
        fn combatant(&self, entity_id: &str) -> Option<&Combatant> {
            self.combatants.iter().find(|c| c.entity_id == entity_id)
        }
    }
    impl EntityResolver for VecResolver {
        fn snapshot(&self, entity_id: &str) -> Option<EntitySnapshot> {
            self.combatant(entity_id).map(EntitySnapshot::from_combatant)
        }
        fn apply_resource_delta(&mut self, entity_id: &str, resource: &str, delta: i64) {
            let Some(c) = self.combatants.iter_mut().find(|c| c.entity_id == entity_id) else {
                return;
            };
            if resource == "hp" {
                c.hp = ((c.hp as i64 + delta).max(0)).min(c.max_hp as i64) as i32;
                if c.hp == 0 {
                    c.alive = false;
                }
            } else if let Some(pool) = c.pools.iter_mut().find(|p| p.id == resource) {
                pool.current += delta;
            }
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
            json!({
                "component_type": "item", "id": "envenomed_dagger", "name": "Envenomed Dagger",
                "damage": "1d4", "tags": ["weapon"], "grants_traits": ["envenom_on_hit"]
            }),
            json!({
                "component_type": "trait", "id": "envenom_on_hit", "name": "Envenom on Hit",
                "category": "weapon_quality",
                "triggers": [{"id": "envenom_strike", "on": "combat.attack",
                              "when": "event.hit == true",
                              "do": [{"kind": "apply_status",
                                      "params": {"entity_id": "target", "status_id": "poisoned",
                                                 "duration_ticks": 3}}]}]
            }),
            json!({
                "component_type": "trait", "id": "corrosive_bite", "name": "Corrosive Bite",
                "category": "creature_quality",
                "triggers": [{"id": "corrode_strike", "on": "combat.attack",
                              "when": "event.hit == true",
                              "do": [{"kind": "apply_status",
                                      "params": {"entity_id": "target", "status_id": "poisoned",
                                                 "duration_ticks": 3}}]}]
            }),
            json!({
                "component_type": "trait", "id": "flame_fang", "name": "Flame Fang",
                "category": "weapon_quality",
                "triggers": [{"id": "flame_strike", "on": "combat.attack",
                              "when": "event.hit == true",
                              "do": [{"kind": "emit_damage",
                                      "params": {"entity_id": "target",
                                                 "packet": {"amount": 5, "types": ["fire"], "tier": "sd"}}}]}]
            }),
            json!({
                "component_type": "trait", "id": "void_lash", "name": "Void Lash",
                "category": "weapon_quality",
                "triggers": [{"id": "void_strike", "on": "combat.attack",
                              "when": "event.hit == true",
                              "do": [{"kind": "emit_damage",
                                      "params": {"entity_id": "target",
                                                 "packet": {"amount": 4, "types": [], "tier": "md"}}}]}]
            }),
            json!({
                "component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
                "default_duration_ticks": 3, "provides_tags": ["poisoned"], "stacking": "replace",
                "triggers": [{"id": "poison_tick", "on": "combat.attack",
                              "when": "has_tag(self, 'poisoned')",
                              "do": [{"kind": "change_resource",
                                      "params": {"resource": "hp", "delta": -2}}]}]
            }),
        ]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        RuntimeContentStore::from_records(records)
    }

    fn combatant(id: &str, weapon: Option<&str>, hp: i32) -> Combatant {
        Combatant {
            entity_id: id.into(),
            name: id.into(),
            display_name: id.into(),
            is_player: id == "pc",
            initiative: 0,
            hp,
            max_hp: hp,
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

    fn attack_event(attacker: &str, target: &str, hit: bool) -> GameEvent {
        let mut data = Map::new();
        data.insert("attacker_id".into(), Value::String(attacker.into()));
        data.insert("target_id".into(), Value::String(target.into()));
        data.insert("hit".into(), Value::Bool(hit));
        GameEvent::new(0, "combat.attack", data)
    }

    #[test]
    fn envenom_weapon_applies_status_to_target_on_hit() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(DemoClock(0)));
        let runtime = TriggerRuntime::new(&store, &svc, &db, 0, 8);
        let mut resolver = VecResolver {
            combatants: vec![
                combatant("pc", Some("envenomed_dagger"), 8),
                combatant("hound", None, 6),
            ],
        };
        let out = runtime.run(&mut resolver, &[attack_event("pc", "hound", true)]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let active = svc.list_for_entity("hound").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].effect_id, "poisoned");
    }

    #[test]
    fn intrinsic_creature_trait_fires_on_hit_without_a_weapon() {
        // A creature whose intrinsic trait (not an equipped item) carries the
        // trigger applies its status to the target on a hit.
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(DemoClock(0)));
        let runtime = TriggerRuntime::new(&store, &svc, &db, 0, 8);
        let mut hound = combatant("hound", None, 6);
        hound.traits = vec!["corrosive_bite".into()];
        let mut resolver = VecResolver {
            combatants: vec![combatant("pc", None, 8), hound],
        };
        let out = runtime.run(&mut resolver, &[attack_event("hound", "pc", true)]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let active = svc.list_for_entity("pc").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].effect_id, "poisoned");
    }

    #[test]
    fn poison_ticks_when_poisoned_entity_acts() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(DemoClock(0)));
        svc.apply("hound", "poisoned", Some(3), None, None).unwrap();
        let runtime = TriggerRuntime::new(&store, &svc, &db, 0, 8);
        let mut resolver = VecResolver {
            combatants: vec![combatant("pc", None, 8), combatant("hound", None, 6)],
        };
        // The hound (poisoned) acts → poison_tick fires on self → -2 hp.
        let out = runtime.run(&mut resolver, &[attack_event("hound", "pc", false)]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let hound = resolver.combatant("hound").unwrap();
        assert_eq!(hound.hp, 4);
    }

    #[test]
    fn emit_damage_routes_through_pipeline_with_mitigation() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(DemoClock(0)));
        let runtime = TriggerRuntime::new(&store, &svc, &db, 0, 8);

        let mut attacker = combatant("pc", None, 8);
        attacker.traits = vec!["flame_fang".into()];
        let mut target = combatant("hound", None, 10);
        target.defenses.insert("dr".into(), 2);
        let mut resolver = VecResolver {
            combatants: vec![attacker, target],
        };
        let out = runtime.run(&mut resolver, &[attack_event("pc", "hound", true)]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        // 5 fire damage − dr 2 = 3 hp lost (the pipeline applied mitigation).
        assert_eq!(resolver.combatant("hound").unwrap().hp, 7);
    }

    #[test]
    fn emit_damage_md_packet_routes_to_md_pool() {
        use crate::resolution::damage::Pool;
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(DemoClock(0)));
        let runtime = TriggerRuntime::new(&store, &svc, &db, 0, 8);

        let mut attacker = combatant("pc", None, 8);
        attacker.traits = vec!["void_lash".into()];
        let mut target = combatant("hound", None, 10);
        target.pools = vec![Pool {
            id: "ablative".into(),
            tags: vec!["md".into()],
            current: 6,
            max: 6,
            can_go_negative: false,
        }];
        let mut resolver = VecResolver {
            combatants: vec![attacker, target],
        };
        let out = runtime.run(&mut resolver, &[attack_event("pc", "hound", true)]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let t = resolver.combatant("hound").unwrap();
        // An MD packet bypasses the SD hp pool and depletes the MD pool.
        assert_eq!(t.hp, 10, "hp untouched by an MD packet");
        assert_eq!(t.pools[0].current, 2, "md pool absorbed 4");
    }

    #[test]
    fn non_entity_events_are_skipped() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(DemoClock(0)));
        let runtime = TriggerRuntime::new(&store, &svc, &db, 0, 8);
        let mut resolver = VecResolver {
            combatants: vec![combatant("pc", None, 8)],
        };
        let out = runtime.run(&mut resolver, &[GameEvent::new(0, "gm.narrate", Map::new())]);
        assert!(out.emitted.is_empty());
        assert!(out.errors.is_empty());
    }

    /// HR-756 acceptance: a resolver NOT backed by `Combatant` drives the loop on
    /// a non-combat event — proving the runtime is scene-agnostic.
    #[test]
    fn non_combat_resolver_drives_the_loop() {
        use std::collections::HashMap;

        /// A resolver over hand-built snapshots (no combat types involved).
        struct MapResolver {
            entities: HashMap<String, EntitySnapshot>,
        }
        impl EntityResolver for MapResolver {
            fn snapshot(&self, entity_id: &str) -> Option<EntitySnapshot> {
                self.entities.get(entity_id).cloned()
            }
            fn apply_resource_delta(&mut self, entity_id: &str, resource: &str, delta: i64) {
                if let Some(s) = self.entities.get_mut(entity_id) {
                    if let Some(v) = s.scalar_fields.get_mut(resource) {
                        if let Some(n) = v.as_i64() {
                            *v = Value::from(n + delta);
                        }
                    }
                }
            }
        }

        let db = seeded_db();
        // An intrinsic trait whose trigger fires on a non-combat event type.
        let records: Vec<ComponentRecord> = vec![
            json!({"component_type": "trait", "id": "ash_cursed", "name": "Ash-Cursed",
                   "category": "regional",
                   "triggers": [{"id": "dread_on_enter", "on": "exploration.enter_hex",
                                 "when": "true",
                                 "do": [{"kind": "apply_status",
                                         "params": {"status_id": "dread", "duration_ticks": 5}}]}]}),
            json!({"component_type": "status_effect", "id": "dread", "name": "Dread",
                   "provides_tags": ["afraid"], "stacking": "replace"}),
        ]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        let store = RuntimeContentStore::from_records(records);
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(DemoClock(0)));
        let runtime = TriggerRuntime::new(&store, &svc, &db, 0, 8);

        let mut scalar = Map::new();
        scalar.insert("hp".into(), json!(10));
        let snap = EntitySnapshot {
            entity_id: "pc".into(),
            equipped_item_ids: vec![],
            intrinsic_trait_ids: vec!["ash_cursed".into()],
            scalar_fields: scalar,
            pools: vec![],
            mitigations: vec![],
        };
        let mut resolver = MapResolver {
            entities: HashMap::from([("pc".to_string(), snap)]),
        };

        // A non-combat event: entering a hex. `self_id` drives trigger sourcing.
        let mut data = Map::new();
        data.insert("self_id".into(), Value::String("pc".into()));
        let event = GameEvent::new(0, "exploration.enter_hex", data);

        let out = runtime.run(&mut resolver, &[event]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let active = svc.list_for_entity("pc").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].effect_id, "dread");
    }
}
