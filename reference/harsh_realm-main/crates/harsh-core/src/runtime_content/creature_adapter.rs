//! Adapt an authored [`ir::Creature`](crate::ir::Creature) into the engine's
//! runtime [`CreatureData`](crate::creature::CreatureData).
//!
//! The IR uses `i64` and carries action-model fields (`actions`/`pools`/
//! `defenses`) the combat runtime does not yet consume; the engine type uses
//! `i32` and the legacy `attack_skill`/`innate_attacks` shape. Conversion is
//! lossless for the fields combat reads, clamps out-of-range integers with a
//! warning (never panics), and reports the dropped action-model fields so the gap
//! is explicit rather than silent.

use crate::creature::{CreatureData, CreatureHarvestable};
use crate::ir::Creature;

fn clamp_i64(value: i64, field: &str, warnings: &mut Vec<String>) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| {
        let clamped = if value > i32::MAX as i64 {
            i32::MAX
        } else {
            i32::MIN
        };
        warnings.push(format!(
            "creature field '{field}' value {value} out of i32 range; clamped to {clamped}"
        ));
        clamped
    })
}

/// Convert an IR creature to engine `CreatureData`, returning any conversion
/// warnings (clamps, dropped fields).
pub fn ir_creature_to_data(c: &Creature) -> (CreatureData, Vec<String>) {
    let mut warnings = Vec::new();

    let harvestable = c.harvestable.as_ref().map(|h| CreatureHarvestable {
        material: h.material.clone(),
        skill: h.skill.clone(),
        difficulty: clamp_i64(h.difficulty, "harvestable.difficulty", &mut warnings),
    });

    // `pools`/`defenses` are now carried into combat (the IR damage pipeline
    // consumes them). `actions` are carried but the turn loop does not yet perform
    // them, so that one stays an explicit gap.
    if !c.actions.is_empty() {
        warnings.push(format!(
            "creature '{}': authored `actions` are carried but the turn loop does not \
             yet perform them (deferred)",
            c.id
        ));
    }

    let mut data = CreatureData {
        id: c.id.clone(),
        name: c.name.clone(),
        hd: clamp_i64(c.hd, "hd", &mut warnings),
        hp_per_hd: clamp_i64(c.hp_per_hd, "hp_per_hd", &mut warnings),
        ac: clamp_i64(c.ac, "ac", &mut warnings),
        attack_bonus: clamp_i64(c.attack_bonus, "attack_bonus", &mut warnings),
        damage: c.damage.clone(),
        damage_type: c.damage_type.clone(),
        // IR retired `attack_skill` (subsumed by the action's skill); the legacy
        // engine type still needs it. Fall back to its own default.
        attack_skill: "punch".to_string(),
        attack_description: c.attack_description.clone(),
        num_attacks: clamp_i64(c.num_attacks, "num_attacks", &mut warnings),
        behavior: c.behavior.clone(),
        awareness_difficulty: clamp_i64(c.awareness_difficulty, "awareness_difficulty", &mut warnings),
        flee_difficulty: clamp_i64(c.flee_difficulty, "flee_difficulty", &mut warnings),
        unavoidable: c.unavoidable,
        morale: clamp_i64(c.morale, "morale", &mut warnings),
        loot_table: c.loot_table.clone(),
        harvestable,
        description_unseen: c.description_unseen.clone(),
        description_seen: c.description_seen.clone(),
        description_short: c.description_short.clone(),
        xp_value: clamp_i64(c.xp_value, "xp_value", &mut warnings),
        tags: c.tags.clone(),
        special_abilities: c.special_abilities.clone(),
        traits: c.traits.clone(),
        innate_attacks: Vec::new(),
        actions: c.actions.clone(),
        pools: c.pools.clone(),
        defenses: c.defenses.clone(),
    };
    data.apply_defaults();
    (data, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ir(value: serde_json::Value) -> Creature {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn maps_core_fields_with_defaults() {
        let (data, warnings) = ir_creature_to_data(&ir(json!({
            "id": "scrip_hound", "name": "Scrip-Hound", "hd": 2, "ac": 14,
            "attack_bonus": 3, "damage": "1d8"
        })));
        assert!(warnings.is_empty());
        assert_eq!(data.id, "scrip_hound");
        assert_eq!(data.hd, 2);
        assert_eq!(data.hp_per_hd, 4);
        assert_eq!(data.ac, 14);
        assert_eq!(data.attack_bonus, 3);
        assert_eq!(data.damage, "1d8");
        assert_eq!(data.num_attacks, 1);
        assert_eq!(data.morale, 8);
        // apply_defaults fills description_short from the name.
        assert_eq!(data.description_short, "scrip-hound");
    }

    #[test]
    fn carries_pools_and_defenses_and_warns_only_for_actions() {
        let (data, warnings) = ir_creature_to_data(&ir(json!({
            "id": "ash_ghoul", "name": "Ash Ghoul", "hd": 3, "ac": 13,
            "attack_bonus": 2, "damage": "1d6", "actions": ["corrosive_bite"],
            "defenses": {"ac": 13, "dr": 2},
            "pools": [{"id": "shell", "tags": ["sd"], "current": 6, "max": 6}]
        })));
        // pools + defenses are now carried into the engine model.
        assert_eq!(data.pools.len(), 1);
        assert_eq!(data.pools[0].id, "shell");
        assert_eq!(data.defenses.get("dr"), Some(&2));
        assert_eq!(data.actions, vec!["corrosive_bite"]);
        // Only `actions` still warns (the turn loop doesn't perform them yet).
        assert!(
            warnings.iter().any(|w| w.contains("actions")),
            "{warnings:?}"
        );
        assert!(
            !warnings.iter().any(|w| w.contains("pools") || w.contains("defenses")),
            "pools/defenses must not warn: {warnings:?}"
        );
    }

    #[test]
    fn maps_intrinsic_traits() {
        let (data, warnings) = ir_creature_to_data(&ir(json!({
            "id": "ash_ghoul", "name": "Ash Ghoul", "hd": 3, "ac": 13,
            "attack_bonus": 2, "damage": "1d6", "traits": ["corrosive_bite"]
        })));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(data.traits, vec!["corrosive_bite"]);
    }

    #[test]
    fn maps_harvestable() {
        let (data, _) = ir_creature_to_data(&ir(json!({
            "id": "ash_ghoul", "name": "Ash Ghoul", "hd": 3, "ac": 13,
            "attack_bonus": 2, "damage": "1d6",
            "harvestable": {"material": "ash_gland", "skill": "Heal", "difficulty": 9}
        })));
        let h = data.harvestable.unwrap();
        assert_eq!(h.material, "ash_gland");
        assert_eq!(h.skill, "Heal");
        assert_eq!(h.difficulty, 9);
    }

    #[test]
    fn clamps_out_of_range_integer_with_warning() {
        let big = i32::MAX as i64 + 1;
        let (data, warnings) = ir_creature_to_data(&ir(json!({
            "id": "colossus", "name": "Colossus", "hd": big, "ac": 20,
            "attack_bonus": 5, "damage": "2d10"
        })));
        assert_eq!(data.hd, i32::MAX);
        assert!(warnings.iter().any(|w| w.contains("'hd'")), "{warnings:?}");
    }
}
