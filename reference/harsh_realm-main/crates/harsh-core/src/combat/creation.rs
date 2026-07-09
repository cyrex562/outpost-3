//! Combat initialization and player weapon resolution.
//!
//! Ported from `src/harsh_realm/engine/combat/creation.py`.

use std::collections::BTreeMap;

use rand::Rng;

use crate::character::Character;
use crate::combat::positioning::assign_positions;
use crate::combat_runtime::{AwarenessResult, Combatant, CombatState};
use crate::creature::CreatureData;
use crate::runtime::{InventoryItemRecord, JsonObject, JsonValue};

/// Initialize a [`CombatState`] from a character, creatures, awareness, and the
/// encounter world-cell's terrain/features.
///
/// `terrain` is the terrain id of the cell where the encounter occurs (stored on
/// `CombatState` and used as the backdrop for every cell in the battle grid).
/// `features` are the world-cell feature ids (up to 4) stamped onto the grid
/// corner cells. Pass an empty string and empty slice when no terrain context is
/// available (e.g. tests, demo combat).
pub fn create_combat<R: Rng>(
    character: &Character,
    creatures_list: &[CreatureData],
    awareness_result: AwarenessResult,
    rng: &mut R,
    terrain: &str,
    features: &[String],
    enemy_hp_mult: f64,
) -> CombatState {
    // Count duplicate creature names for display numbering.
    let mut name_counts: BTreeMap<String, i32> = BTreeMap::new();
    for c in creatures_list {
        *name_counts.entry(c.name.clone()).or_insert(0) += 1;
    }

    let mut name_seen: BTreeMap<String, i32> = BTreeMap::new();
    let mut enemy_combatants: Vec<Combatant> = Vec::new();

    for (i, creature) in creatures_list.iter().enumerate() {
        let count = name_counts.get(&creature.name).copied().unwrap_or(1);
        let seen = name_seen.entry(creature.name.clone()).or_insert(0);
        *seen += 1;
        let display_name = if count > 1 {
            format!("{} ({})", creature.name, *seen)
        } else {
            creature.name.clone()
        };

        let base_hp = creature.hd * creature.hp_per_hd;
        let variation: i32 = (0..creature.hd).map(|_| rng.gen_range(-1..=1)).sum();
        let hp = ((base_hp + variation) as f64 * enemy_hp_mult)
            .round()
            .max(1.0) as i32;
        let initiative = rng.gen_range(1..=8);
        let entity_id = format!("{}_{i}", creature.id);

        let mut enemy = make_combatant(
            &entity_id,
            &creature.name,
            &display_name,
            false,
            initiative,
            hp,
            hp,
            creature.ac,
            creature.attack_bonus,
            &creature.damage,
            &creature.attack_description,
            &creature.behavior,
            creature.num_attacks,
            None,
            creature.traits.clone(),
            Vec::new(),
            0,
        );
        // Carry the IR damage-model fields so the pipeline can route to extra
        // pools and apply named (dr/armor) mitigation, plus authored actions the
        // turn loop can perform.
        enemy.pools = creature.pools.clone();
        enemy.defenses = creature.defenses.clone();
        enemy.actions = creature.actions.clone();
        enemy_combatants.push(enemy);
    }

    let dex_mod = character.attr_mods.get("dex").copied().unwrap_or(0);
    let player_initiative = rng.gen_range(1..=8) + dex_mod;
    let player_dex = character.attributes.get("dex").copied().unwrap_or(10);
    let player_damage = player_damage(character);
    let player_weapon_id = player_weapon_id(character);
    let gold = character
        .class_abilities
        .get("gold")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;

    let player_combatant = make_combatant(
        &character.id,
        &character.name,
        &character.name,
        true,
        player_initiative,
        character.hp,
        character.max_hp,
        character.ac,
        character.attack_bonus,
        &player_damage,
        "attacks",
        "player",
        1,
        player_weapon_id,
        character.traits.clone(),
        character.equipment.clone(),
        gold,
    );

    let mut all_combatants: Vec<Combatant> = vec![player_combatant];
    all_combatants.extend(enemy_combatants);

    // Sort by initiative desc, ties broken by DEX (player real DEX, enemies 10).
    all_combatants.sort_by(|a, b| {
        let ka = (a.initiative, if a.is_player { player_dex } else { 10 });
        let kb = (b.initiative, if b.is_player { player_dex } else { 10 });
        kb.cmp(&ka)
    });
    let initiative_order: Vec<String> =
        all_combatants.iter().map(|c| c.entity_id.clone()).collect();

    let player_surprise = awareness_result == AwarenessResult::PlayerSurprise;
    let enemy_surprise = awareness_result == AwarenessResult::EnemySurprise;

    let notice_level = character.skills.get("notice").copied().unwrap_or(-1);
    let wis_mod = character.attr_mods.get("wis").copied().unwrap_or(0);
    let notice_roll = rng.gen_range(1..=6) + rng.gen_range(1..=6);
    let enemy_detail_revealed = notice_roll + notice_level + wis_mod >= 8;

    let mut state = build_combat_state(
        all_combatants,
        initiative_order,
        player_surprise,
        enemy_surprise,
        enemy_detail_revealed,
    );

    // Thread the encounter terrain + features onto the state so the battle grid
    // can propagate them to all cells and corner stamps.
    state.terrain = terrain.to_string();
    state.features = features.to_vec();

    // Assign initial grid positions from the range bands.
    assign_positions(&mut state);

    state
}

#[allow(clippy::too_many_arguments)]
fn make_combatant(
    entity_id: &str,
    name: &str,
    display_name: &str,
    is_player: bool,
    initiative: i32,
    hp: i32,
    max_hp: i32,
    ac: i32,
    attack_bonus: i32,
    damage_expr: &str,
    attack_description: &str,
    behavior: &str,
    num_attacks: i32,
    weapon_id: Option<String>,
    traits: Vec<String>,
    inventory: Vec<JsonObject>,
    gold: i32,
) -> Combatant {
    Combatant {
        entity_id: entity_id.to_string(),
        name: name.to_string(),
        display_name: display_name.to_string(),
        is_player,
        initiative,
        hp,
        max_hp,
        ac,
        attack_bonus,
        damage_expr: damage_expr.to_string(),
        attack_description: attack_description.to_string(),
        behavior: behavior.to_string(),
        num_attacks,
        alive: true,
        weapon_id,
        range_band: "melee".to_string(),
        traits,
        pools: Vec::new(),
        defenses: std::collections::BTreeMap::new(),
        actions: Vec::new(),
        inventory,
        gold,
        // Grid positions are set to (0, 0) here; `assign_positions` overwrites
        // them with the correct battle-grid coordinates after the state is built.
        q: 0,
        r: 0,
    }
}

/// Construct a [`CombatState`] with the supplied fields and serde defaults for
/// the rest (avoids enumerating every transient combat flag).
fn build_combat_state(
    combatants: Vec<Combatant>,
    initiative_order: Vec<String>,
    player_surprise: bool,
    enemy_surprise: bool,
    enemy_detail_revealed: bool,
) -> CombatState {
    let mut obj = JsonObject::new();
    obj.insert(
        "combatants".into(),
        serde_json::to_value(&combatants).expect("combatants serialize"),
    );
    obj.insert(
        "initiative_order".into(),
        serde_json::to_value(&initiative_order).expect("order serialize"),
    );
    obj.insert("current_turn_index".into(), JsonValue::from(0));
    obj.insert("round_number".into(), JsonValue::from(1));
    obj.insert("player_surprise".into(), JsonValue::from(player_surprise));
    obj.insert("enemy_surprise".into(), JsonValue::from(enemy_surprise));
    obj.insert(
        "enemy_detail_revealed".into(),
        JsonValue::from(enemy_detail_revealed),
    );
    serde_json::from_value(JsonValue::Object(obj)).expect("combat state from defaults")
}

/// Whether an equipment record is a weapon. Accepts the canonical `"weapon"`
/// type and the kit-authored `"melee_weapon"`/`"ranged_weapon"` types.
fn is_weapon(item: &InventoryItemRecord) -> bool {
    matches!(
        item.r#type.as_str(),
        "weapon" | "melee_weapon" | "ranged_weapon"
    )
}

/// The id of a weapon record: the canonical `item_id` if present, else the
/// authored `id` key (kits use a bare `id`, which lands in `extra`).
fn weapon_id_of(item: &InventoryItemRecord) -> Option<String> {
    item.item_id
        .clone()
        .or_else(|| item.extra.get("id").and_then(|v| v.as_str()).map(str::to_string))
        .filter(|id| !id.is_empty())
}

fn player_damage(character: &Character) -> String {
    for raw in &character.equipment {
        if let Ok(item) = serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone()))
        {
            if is_weapon(&item) {
                if let Some(dmg) = item.weapon_damage.or(item.damage) {
                    if !dmg.is_empty() {
                        return dmg;
                    }
                }
            }
        }
    }
    "1d2".to_string()
}

/// The id of the character's wielded weapon (canonical `item_id` or kit `id`),
/// or `None` if unarmed. Shared with the runtime-content snapshot builder.
pub fn player_weapon_id(character: &Character) -> Option<String> {
    for raw in &character.equipment {
        if let Ok(item) = serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone()))
        {
            if is_weapon(&item) {
                if let Some(id) = weapon_id_of(&item) {
                    return Some(id);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn creature(id: &str, name: &str) -> CreatureData {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": name, "hd": 2, "hp_per_hd": 4, "ac": 12,
            "attack_bonus": 1, "damage": "1d6", "attack_description": "bites",
            "behavior": "aggressive", "num_attacks": 1,
        }))
        .unwrap()
    }

    #[test]
    fn builds_state_with_player_and_numbered_enemies() {
        let mut rng = SmallRng::seed_from_u64(3);
        let creatures = vec![creature("wolf", "Wolf"), creature("wolf", "Wolf")];
        let state = create_combat(&Character::default(), &creatures, AwarenessResult::MutualAwareness, &mut rng, "", &[], 1.0);
        assert_eq!(state.combatants.len(), 3);
        // Player present.
        assert!(state.combatants.iter().any(|c| c.is_player));
        // Duplicate enemy names are numbered.
        let names: Vec<&str> = state
            .combatants
            .iter()
            .filter(|c| !c.is_player)
            .map(|c| c.display_name.as_str())
            .collect();
        assert!(names.contains(&"Wolf (1)"));
        assert!(names.contains(&"Wolf (2)"));
        // Initiative order has all combatants.
        assert_eq!(state.initiative_order.len(), 3);
        assert_eq!(state.round_number, 1);
    }

    #[test]
    fn threads_intrinsic_traits_from_creature_and_character() {
        let mut rng = SmallRng::seed_from_u64(7);
        let mut creature = creature("ash_ghoul", "Ash Ghoul");
        creature.traits = vec!["corrosive_bite".into()];
        let mut character = Character::default();
        character.traits = vec!["veteran_grit".into()];

        let state = create_combat(
            &character,
            &[creature],
            AwarenessResult::MutualAwareness,
            &mut rng,
            "",
            &[],
            1.0,
        );

        let player = state.combatants.iter().find(|c| c.is_player).unwrap();
        assert_eq!(player.traits, vec!["veteran_grit"]);
        let enemy = state.combatants.iter().find(|c| !c.is_player).unwrap();
        assert_eq!(enemy.traits, vec!["corrosive_bite"]);
    }

    #[test]
    fn applies_surprise_flags() {
        let mut rng = SmallRng::seed_from_u64(1);
        let state = create_combat(
            &Character::default(),
            &[creature("rat", "Rat")],
            AwarenessResult::PlayerSurprise,
            &mut rng,
            "",
            &[],
            1.0,
        );
        assert!(state.player_surprise);
        assert!(!state.enemy_surprise);
    }

    #[test]
    fn player_damage_defaults_to_unarmed() {
        assert_eq!(player_damage(&Character::default()), "1d2");
    }

    fn char_with_equipment(equipment: serde_json::Value) -> Character {
        let mut c = Character::default();
        c.equipment = equipment
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_object().unwrap().clone())
            .collect();
        c
    }

    #[test]
    fn kit_melee_weapon_resolves_damage_and_id() {
        // Kit-authored shape: `type: melee_weapon`, bare `id`, `damage`.
        let c = char_with_equipment(serde_json::json!([
            { "id": "short_sword", "name": "Short Sword", "type": "melee_weapon", "damage": "1d6" }
        ]));
        assert_eq!(player_damage(&c), "1d6");
        assert_eq!(player_weapon_id(&c).as_deref(), Some("short_sword"));
    }

    #[test]
    fn canonical_weapon_with_item_id_still_resolves() {
        let c = char_with_equipment(serde_json::json!([
            { "name": "Knife", "type": "weapon", "item_id": "weapon.knife", "weapon_damage": "1d4" }
        ]));
        assert_eq!(player_damage(&c), "1d4");
        assert_eq!(player_weapon_id(&c).as_deref(), Some("weapon.knife"));
    }

    #[test]
    fn ranged_weapon_type_is_recognized() {
        let c = char_with_equipment(serde_json::json!([
            { "id": "short_bow", "name": "Short Bow", "type": "ranged_weapon", "damage": "1d6" }
        ]));
        assert_eq!(player_weapon_id(&c).as_deref(), Some("short_bow"));
    }

    #[test]
    fn enemy_hp_mult_scales_enemy_hp() {
        // Use a fixed seed so the hp variation is deterministic.
        let mut rng_normal = SmallRng::seed_from_u64(99);
        let mut rng_doubled = SmallRng::seed_from_u64(99);
        // Creature with hd=2, hp_per_hd=4 → base_hp=8, variation deterministic.
        let c = creature("goblin", "Goblin");
        let state_normal = create_combat(
            &Character::default(),
            &[c.clone()],
            AwarenessResult::MutualAwareness,
            &mut rng_normal,
            "",
            &[],
            1.0,
        );
        let state_doubled = create_combat(
            &Character::default(),
            &[c],
            AwarenessResult::MutualAwareness,
            &mut rng_doubled,
            "",
            &[],
            2.0,
        );
        let enemy_normal = state_normal.combatants.iter().find(|c| !c.is_player).unwrap();
        let enemy_doubled = state_doubled.combatants.iter().find(|c| !c.is_player).unwrap();
        // With mult=2.0, hp should be roughly twice the normal value (min 1).
        assert!(enemy_doubled.max_hp >= enemy_normal.max_hp * 2 - 2);
        assert!(enemy_doubled.hp >= 1);
    }

    #[test]
    fn enemy_hp_mult_grade_3_unchanged() {
        // mult=1.0 (grade 3 / Normal) leaves HP identical to pre-feature value.
        let mut rng1 = SmallRng::seed_from_u64(42);
        let mut rng2 = SmallRng::seed_from_u64(42);
        let c = creature("goblin", "Goblin");
        let s1 = create_combat(&Character::default(), &[c.clone()], AwarenessResult::MutualAwareness, &mut rng1, "", &[], 1.0);
        let s2 = create_combat(&Character::default(), &[c], AwarenessResult::MutualAwareness, &mut rng2, "", &[], 1.0);
        let hp1 = s1.combatants.iter().find(|c| !c.is_player).map(|c| c.hp).unwrap_or(0);
        let hp2 = s2.combatants.iter().find(|c| !c.is_player).map(|c| c.hp).unwrap_or(0);
        assert_eq!(hp1, hp2);
    }
}
