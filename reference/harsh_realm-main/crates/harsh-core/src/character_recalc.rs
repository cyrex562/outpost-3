//! Rule-aware character recalculation engine.
//!
//! Ported from `src/harsh_realm/engine/character_recalc.py`. The derivation
//! itself lives in `resolvers::character`; this marshals inputs (resolving the
//! class progression host-side) and builds the result.

use std::collections::BTreeMap;

use crate::character::Character;
use crate::class_progression::get_progression;
use crate::engine_results::RecalcResult;
use crate::resolvers::character::{recalculate, EquipmentItem, RecalcInput};
use crate::runtime::InventoryItemRecord;

/// Base saving-throw target before the per-level reduction (XWN: 15).
const SAVE_BASE: i64 = 15;
/// Unarmored AC (XWN: 10).
const AC_BASE: i64 = 10;
/// Armor bonus at/above which DEX no longer applies (XWN: 4).
const HEAVY_ARMOR_THRESHOLD: i64 = 4;

/// Recompute `max_hp` and proportionally scale `hp` when the `player_hp`
/// difficulty multiplier changes.
///
/// Uses `character.base_max_hp` as the unbuffed XWN value. If `base_max_hp`
/// is 0 (legacy save that predates the field), it is initialised to the
/// current `max_hp` before applying the multiplier — this treats legacy
/// characters as having mult = 1.0.
///
/// `max_hp = round(base_max_hp × mult).max(1)`
/// `hp = round(hp × new_max / old_max).clamp(1..=max_hp)`
pub fn recalc_hp_for_difficulty(character: &mut Character, player_hp_mult: f64) {
    // Initialise base_max_hp if this is a legacy character.
    if character.base_max_hp == 0 {
        character.base_max_hp = character.max_hp.max(1);
    }
    let base = character.base_max_hp.max(1) as f64;
    let new_max = (base * player_hp_mult).round().max(1.0) as i32;
    let old_max = character.max_hp.max(1) as f64;
    let new_hp = ((character.hp as f64 * new_max as f64 / old_max).round() as i32)
        .clamp(1, new_max);
    character.max_hp = new_max;
    character.hp = new_hp;
}

/// Given base fields, compute all derived character values.
pub fn recalculate_character(
    character_class: &str,
    level: i32,
    attributes: &BTreeMap<String, i32>,
    equipment: &[InventoryItemRecord],
    hp_rolls: Option<Vec<i32>>,
) -> RecalcResult {
    let progression = get_progression(character_class);
    let input = RecalcInput {
        level: level as i64,
        attributes: attributes.iter().map(|(k, v)| (k.clone(), *v as i64)).collect(),
        equipment: equipment
            .iter()
            .map(|it| EquipmentItem {
                item_type: it.r#type.clone(),
                name: it.name.clone(),
                ac_bonus: it.ac_bonus.map(|x| x as i64),
            })
            .collect(),
        hp_rolls: hp_rolls.map(|v| v.into_iter().map(|x| x as i64).collect()),
        attack_bonus: progression.attack_bonus(level) as i64,
        hit_die: progression.hit_die as i64,
        save_base: SAVE_BASE,
        ac_base: AC_BASE,
        heavy_armor_threshold: HEAVY_ARMOR_THRESHOLD,
    };
    let out = recalculate(&input);
    RecalcResult {
        attr_mods: out.attr_mods.into_iter().map(|(k, v)| (k, v as i32)).collect(),
        max_hp: out.max_hp as i32,
        ac: out.ac as i32,
        attack_bonus: out.attack_bonus as i32,
        melee_attack: out.melee_attack as i32,
        ranged_attack: out.ranged_attack as i32,
        physical_save: out.physical_save as i32,
        evasion_save: out.evasion_save as i32,
        mental_save: out.mental_save as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::Character;

    #[test]
    fn recalculates_with_default_progression() {
        let mut attrs = BTreeMap::new();
        attrs.insert("str".to_string(), 14);
        attrs.insert("dex".to_string(), 12);
        attrs.insert("con".to_string(), 13);
        let result = recalculate_character("warrior", 1, &attrs, &[], None);
        // STR 14 -> +1 modifier (pinned by dice::attr_modifier).
        assert_eq!(result.attr_mods.get("str"), Some(&1));
        assert!(result.max_hp > 0);
        assert_eq!(result.ac, 10); // unarmored
    }

    // ---- recalc_hp_for_difficulty ------------------------------------------

    #[test]
    fn recalc_hp_mult_1_is_identity() {
        let mut c = Character {
            hp: 8,
            max_hp: 10,
            base_max_hp: 10,
            ..Character::default()
        };
        recalc_hp_for_difficulty(&mut c, 1.0);
        assert_eq!(c.max_hp, 10);
        assert_eq!(c.hp, 8);
        assert_eq!(c.base_max_hp, 10);
    }

    #[test]
    fn recalc_hp_mult_increases_max_and_scales_hp() {
        // base=10, mult=1.5 → max=15; current 8/10 → round(8*15/10)=12 clamped to 15.
        let mut c = Character {
            hp: 8,
            max_hp: 10,
            base_max_hp: 10,
            ..Character::default()
        };
        recalc_hp_for_difficulty(&mut c, 1.5);
        assert_eq!(c.max_hp, 15);
        assert_eq!(c.hp, 12);
    }

    #[test]
    fn recalc_hp_mult_decreases_max_and_scales_hp() {
        // base=10, mult=0.7 → max=7; current 8/10 → round(8*7/10)=6 clamped to 7.
        let mut c = Character {
            hp: 8,
            max_hp: 10,
            base_max_hp: 10,
            ..Character::default()
        };
        recalc_hp_for_difficulty(&mut c, 0.7);
        assert_eq!(c.max_hp, 7);
        assert_eq!(c.hp, 6);
    }

    #[test]
    fn recalc_hp_floors_max_at_1() {
        // Tiny base with a very small mult should still be at least 1.
        let mut c = Character {
            hp: 1,
            max_hp: 1,
            base_max_hp: 1,
            ..Character::default()
        };
        recalc_hp_for_difficulty(&mut c, 0.01);
        assert_eq!(c.max_hp, 1);
        assert_eq!(c.hp, 1);
    }

    #[test]
    fn recalc_hp_clamps_current_hp_to_max() {
        // If current hp > new max after reduction, it is clamped down.
        let mut c = Character {
            hp: 10,
            max_hp: 10,
            base_max_hp: 10,
            ..Character::default()
        };
        recalc_hp_for_difficulty(&mut c, 0.5);
        assert_eq!(c.max_hp, 5);
        assert!(c.hp <= c.max_hp);
    }

    #[test]
    fn recalc_hp_legacy_character_with_zero_base() {
        // Legacy character: base_max_hp = 0 → initialised from max_hp.
        let mut c = Character {
            hp: 7,
            max_hp: 10,
            base_max_hp: 0,
            ..Character::default()
        };
        recalc_hp_for_difficulty(&mut c, 1.5);
        // base initialised to 10, new max = round(10 * 1.5) = 15
        assert_eq!(c.max_hp, 15);
        assert_eq!(c.base_max_hp, 10);
    }

    #[test]
    fn recalc_hp_round_trip_grade_3_restores_base() {
        // Apply a non-identity mult, then apply grade-3 mult (1.0) — should
        // restore max to base_max_hp exactly.
        let mut c = Character {
            hp: 10,
            max_hp: 10,
            base_max_hp: 10,
            ..Character::default()
        };
        recalc_hp_for_difficulty(&mut c, 1.6);
        assert_eq!(c.max_hp, 16);
        recalc_hp_for_difficulty(&mut c, 1.0);
        assert_eq!(c.max_hp, 10);
    }
}
