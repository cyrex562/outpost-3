//! Character derived-stat recomputation (R5-R8).
//!
//! Pure XWN derivation: attribute modifiers, max HP, AC (from equipped armor +
//! DEX), attack bonus, and saves — from a character's class/level/attributes and
//! equipment. The Rust mirror of `engine/character_recalc.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dice::attr_modifier;

/// One equipped item, as far as AC cares.
#[derive(Debug, Clone, Deserialize)]
pub struct EquipmentItem {
    #[serde(rename = "type", default)]
    pub item_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ac_bonus: Option<i64>,
}

/// Inputs to a recalculation. The system-specific bases (`save_base`, `ac_base`,
/// `heavy_armor_threshold`) are supplied by the host, not hardcoded — the engine
/// stays system-agnostic.
#[derive(Debug, Clone, Deserialize)]
pub struct RecalcInput {
    pub level: i64,
    #[serde(default)]
    pub attributes: BTreeMap<String, i64>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItem>,
    #[serde(default)]
    pub hp_rolls: Option<Vec<i64>>,
    /// Attack bonus at this level (host-resolved from the class progression data).
    pub attack_bonus: i64,
    /// Hit-die sides (host-resolved from the class progression data).
    pub hit_die: i64,
    /// Base saving-throw target before the per-level reduction (XWN: 15).
    pub save_base: i64,
    /// Unarmored AC (XWN: 10).
    pub ac_base: i64,
    /// Armor bonus at/above which DEX no longer applies (XWN: 4).
    pub heavy_armor_threshold: i64,
}

/// All derived values (mirrors the Python `RecalcResult`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecalcOutput {
    pub attr_mods: BTreeMap<String, i64>,
    pub max_hp: i64,
    pub ac: i64,
    pub attack_bonus: i64,
    pub melee_attack: i64,
    pub ranged_attack: i64,
    pub physical_save: i64,
    pub evasion_save: i64,
    pub mental_save: i64,
}

/// Recompute every derived value from base fields.
pub fn recalculate(input: &RecalcInput) -> RecalcOutput {
    let attr_mods: BTreeMap<String, i64> = input
        .attributes
        .iter()
        .map(|(k, &score)| (k.clone(), i64::from(attr_modifier(score as i32))))
        .collect();
    let get = |k: &str| attr_mods.get(k).copied().unwrap_or(0);
    let con_mod = get("con");
    let dex_mod = get("dex");
    let str_mod = get("str");

    let max_hp = calc_max_hp(input.hit_die, input.level, con_mod, input.hp_rolls.as_deref());
    let ac = calc_ac(
        &input.equipment,
        dex_mod,
        input.ac_base,
        input.heavy_armor_threshold,
    );
    let attack_bonus = input.attack_bonus;
    // Saves: host-supplied base, -1 every even level.
    let save_base = input.save_base - input.level / 2;

    RecalcOutput {
        attr_mods,
        max_hp,
        ac,
        attack_bonus,
        melee_attack: attack_bonus + str_mod,
        ranged_attack: attack_bonus + dex_mod,
        physical_save: save_base,
        evasion_save: save_base,
        mental_save: save_base,
    }
}

fn calc_max_hp(hd: i64, level: i64, con_mod: i64, hp_rolls: Option<&[i64]>) -> i64 {
    let total: i64 = match hp_rolls {
        Some(rolls) if (rolls.len() as i64) >= level => rolls
            .iter()
            .take(level.max(0) as usize)
            .map(|&roll| (roll + con_mod).max(1))
            .sum(),
        _ => {
            // Estimate: average roll per level, `int((hd+1)/2)` truncates.
            let avg = (hd + 1) / 2;
            (0..level.max(0)).map(|_| (avg + con_mod).max(1)).sum()
        }
    };
    total.max(1)
}

fn calc_ac(
    equipment: &[EquipmentItem],
    dex_mod: i64,
    ac_base: i64,
    heavy_armor_threshold: i64,
) -> i64 {
    let mut armor_bonus = 0i64;
    let mut shield_bonus = 0i64;
    for item in equipment {
        if item.item_type == "armor" {
            let bonus = item.ac_bonus.unwrap_or(0); // `ac_bonus or 0`
            armor_bonus = armor_bonus.max(bonus);
        } else if item.item_type == "shield" || item.name.to_lowercase().contains("shield") {
            // `ac_bonus or 1`: 0/None → 1.
            let bonus = item.ac_bonus.filter(|&v| v != 0).unwrap_or(1);
            shield_bonus = shield_bonus.max(bonus);
        }
    }
    // Heavy armor (bonus at/above the threshold) doesn't benefit from DEX.
    if armor_bonus >= heavy_armor_threshold {
        ac_base + armor_bonus + shield_bonus
    } else {
        ac_base + armor_bonus + shield_bonus + dex_mod
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(attack_bonus: i64, hit_die: i64, level: i64, attrs: &[(&str, i64)]) -> RecalcInput {
        RecalcInput {
            level,
            attributes: attrs.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            equipment: Vec::new(),
            hp_rolls: None,
            attack_bonus,
            hit_die,
            save_base: 15,
            ac_base: 10,
            heavy_armor_threshold: 4,
        }
    }

    #[test]
    fn warrior_level_5_derives() {
        // host-resolved: warrior L5 attack bonus 5, d8.
        let out = recalculate(&input(5, 8, 5, &[("str", 14), ("con", 12), ("dex", 10)]));
        assert_eq!(out.attr_mods["str"], 1);
        assert_eq!(out.attack_bonus, 5);
        assert_eq!(out.melee_attack, 6); // ab + str_mod
        assert_eq!(out.physical_save, 13); // 15 - 5/2 = 15 - 2
    }

    #[test]
    fn ac_uses_best_armor_shield_and_heavy_armor_ignores_dex() {
        let dex_15 = input(1, 6, 1, &[("dex", 15)]); // dex mod +1
        // Light armor (+2) keeps DEX; shield with no ac_bonus → +1.
        let light = calc_ac(
            &[
                EquipmentItem { item_type: "armor".into(), name: "Leather".into(), ac_bonus: Some(2) },
                EquipmentItem { item_type: "shield".into(), name: "Buckler".into(), ac_bonus: None },
            ],
            *recalculate(&dex_15).attr_mods.get("dex").unwrap(),
            10,
            4,
        );
        assert_eq!(light, 10 + 2 + 1 + 1);
        // Heavy armor (+5) ignores DEX.
        let heavy = calc_ac(
            &[EquipmentItem { item_type: "armor".into(), name: "Plate".into(), ac_bonus: Some(5) }],
            1,
            10,
            4,
        );
        assert_eq!(heavy, 10 + 5);
    }

    #[test]
    fn max_hp_uses_rolls_when_available() {
        let mut inp = input(2, 8, 2, &[("con", 14)]); // warrior L2, d8; con +1
        inp.hp_rolls = Some(vec![8, 6]);
        let out = recalculate(&inp);
        assert_eq!(out.max_hp, (8 + 1) + (6 + 1)); // 16
    }
}
