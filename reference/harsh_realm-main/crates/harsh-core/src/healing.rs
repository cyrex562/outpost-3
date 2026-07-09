//! Healing system for Harsh Realm.
//!
//! Ported from `src/harsh_realm/engine/healing.py`. The first-aid arithmetic
//! lives in `resolvers::scene`; the rest is host logic. The Python
//! `(character, result)` return becomes `&mut Character` + a returned
//! `HealingResult`.

use rand::Rng;
use serde_json::Value;

use crate::character::Character;
use crate::damage::parse_damage_expr;
use crate::engine_results::HealingResult;
use crate::resolvers::scene::resolve_first_aid;
use crate::runtime::{InventoryItemRecord, JsonObject, JsonValue};

const FIRST_AID_DIFFICULTY: i32 = 8;

/// Tick threshold at or above which resting counts as a full rest (restores
/// `level + CON-mod` HP instead of a short rest's flat 1 HP).
pub const FULL_REST_TICKS: i32 = 50;

/// Handles all healing methods for the player character.
#[derive(Debug, Clone, Default)]
pub struct HealingSystem;

impl HealingSystem {
    /// Attempt first aid (2d6 + Heal + WIS vs 8; restores 1d6 + skill on success).
    pub fn first_aid<R: Rng + ?Sized>(
        &self,
        character: &mut Character,
        rng: &mut R,
    ) -> HealingResult {
        let heal_level = character.skills.get("heal").copied().unwrap_or(-1);
        let wis_mod = character.attr_mods.get("wis").copied().unwrap_or(0);

        let roll = rng.gen_range(1..=6) + rng.gen_range(1..=6);
        let probe = resolve_first_aid(
            roll as i64,
            heal_level as i64,
            wis_mod as i64,
            0,
            character.hp as i64,
            character.max_hp as i64,
            FIRST_AID_DIFFICULTY as i64,
        );
        let total = probe.total as i32;

        if !probe.success {
            return HealingResult {
                healed: false,
                hp_restored: 0,
                new_hp: character.hp,
                max_hp: character.max_hp,
                method: "first_aid".into(),
                cost: 0,
                skill_check_roll: Some(roll),
                skill_check_total: Some(total),
                skill_check_difficulty: Some(FIRST_AID_DIFFICULTY),
                narration: format!(
                    "You try to apply first aid but fail. (roll {roll} → {total} vs {FIRST_AID_DIFFICULTY})"
                ),
            };
        }

        let heal_roll = rng.gen_range(1..=6);
        let outcome = resolve_first_aid(
            roll as i64,
            heal_level as i64,
            wis_mod as i64,
            heal_roll as i64,
            character.hp as i64,
            character.max_hp as i64,
            FIRST_AID_DIFFICULTY as i64,
        );
        let new_hp = outcome.new_hp as i32;
        let actual = outcome.actual_restored as i32;
        character.hp = new_hp;

        if actual <= 0 {
            return HealingResult {
                healed: false,
                hp_restored: 0,
                new_hp,
                max_hp: character.max_hp,
                method: "first_aid".into(),
                cost: 0,
                skill_check_roll: Some(roll),
                skill_check_total: Some(total),
                skill_check_difficulty: Some(FIRST_AID_DIFFICULTY),
                narration: format!(
                    "You apply first aid, but you're already as healthy as you can be. (roll {roll} → {total} vs {FIRST_AID_DIFFICULTY})"
                ),
            };
        }

        HealingResult {
            healed: true,
            hp_restored: actual,
            new_hp,
            max_hp: character.max_hp,
            method: "first_aid".into(),
            cost: 0,
            skill_check_roll: Some(roll),
            skill_check_total: Some(total),
            skill_check_difficulty: Some(FIRST_AID_DIFFICULTY),
            narration: format!(
                "You apply first aid and restore {actual} HP ({}/{}). (roll {roll} → {total} vs {FIRST_AID_DIFFICULTY})",
                character.hp, character.max_hp
            ),
        }
    }

    /// Rest to restore HP. Short rest (<50 ticks) restores 1; full rest restores
    /// `level + CON mod`.
    pub fn rest(&self, character: &mut Character, ticks: i32) -> HealingResult {
        let con_mod = character.attr_mods.get("con").copied().unwrap_or(0);
        let (hp_restored, method_phrase) = if ticks >= FULL_REST_TICKS {
            ((character.level + con_mod).max(1), "a full rest")
        } else {
            (1, "a short rest")
        };
        let old_hp = character.hp;
        let new_hp = (character.hp + hp_restored).min(character.max_hp);
        let actual = new_hp - old_hp;
        character.hp = new_hp;

        let narration = if actual <= 0 {
            "You rest for a while, but you are already fully recovered.".to_string()
        } else {
            format!(
                "After {method_phrase}, you recover {actual} HP ({}/{}).",
                character.hp, character.max_hp
            )
        };
        HealingResult {
            healed: actual > 0,
            hp_restored: actual,
            new_hp,
            max_hp: character.max_hp,
            method: "rest".into(),
            cost: 0,
            skill_check_roll: None,
            skill_check_total: None,
            skill_check_difficulty: None,
            narration,
        }
    }

    /// Use a healing item (`data.healing` dice expr, default `"1d6"`).
    pub fn use_healing_item<R: Rng + ?Sized>(
        &self,
        character: &mut Character,
        item: &InventoryItemRecord,
        rng: &mut R,
    ) -> HealingResult {
        let healing_expr = item
            .data
            .get("healing")
            .and_then(|v| v.as_str())
            .unwrap_or("1d6")
            .to_string();
        let item_name = if item.name.is_empty() {
            "healing item".to_string()
        } else {
            item.name.clone()
        };
        let (num_dice, sides, flat) =
            parse_damage_expr(&healing_expr).unwrap_or((1, 6, 0));
        let roll: i32 = (0..num_dice.max(0)).map(|_| rng.gen_range(1..=sides.max(2))).sum();
        let hp_restored = (roll + flat).max(1);
        let old_hp = character.hp;
        let new_hp = (character.hp + hp_restored).min(character.max_hp);
        let actual = new_hp - old_hp;
        character.hp = new_hp;

        HealingResult {
            healed: true,
            hp_restored: actual,
            new_hp,
            max_hp: character.max_hp,
            method: "item".into(),
            cost: 0,
            skill_check_roll: None,
            skill_check_total: None,
            skill_check_difficulty: None,
            narration: format!(
                "You use the {item_name} and recover {actual} HP ({}/{}).",
                character.hp, character.max_hp
            ),
        }
    }

    /// Pay a town healer to restore all HP (uses equipment coin + legacy gold).
    pub fn town_healer(
        &self,
        character: &mut Character,
        cost_per_hp: i32,
        available_gold: Option<i32>,
        spend_payment: bool,
    ) -> HealingResult {
        let hp_needed = character.max_hp - character.hp;
        if hp_needed <= 0 {
            return HealingResult {
                healed: false,
                hp_restored: 0,
                new_hp: character.hp,
                max_hp: character.max_hp,
                method: "healer".into(),
                cost: 0,
                skill_check_roll: None,
                skill_check_total: None,
                skill_check_difficulty: None,
                narration: "The healer looks you over and sends you away — you don't need healing.".into(),
            };
        }
        let total_cost = hp_needed * cost_per_hp;
        let coin_total =
            equipment_coin(character) + available_gold.unwrap_or_else(|| legacy_gold(character));

        if coin_total < total_cost {
            return HealingResult {
                healed: false,
                hp_restored: 0,
                new_hp: character.hp,
                max_hp: character.max_hp,
                method: "healer".into(),
                cost: total_cost,
                skill_check_roll: None,
                skill_check_total: None,
                skill_check_difficulty: None,
                narration: format!(
                    "The healer quotes {total_cost} coin for a full restoration. You only have {coin_total} coin. You cannot afford it."
                ),
            };
        }

        if spend_payment {
            deduct_legacy_payment(character, total_cost);
        }
        character.hp = character.max_hp;
        HealingResult {
            healed: true,
            hp_restored: hp_needed,
            new_hp: character.max_hp,
            max_hp: character.max_hp,
            method: "healer".into(),
            cost: total_cost,
            skill_check_roll: None,
            skill_check_total: None,
            skill_check_difficulty: None,
            narration: format!(
                "The healer works for an hour and fully restores your health. Cost: {total_cost} coin. HP: {}/{}.",
                character.hp, character.max_hp
            ),
        }
    }

    /// Sum all currency items in character equipment.
    pub fn equipment_coin(&self, character: &Character) -> i32 {
        equipment_coin(character)
    }

    /// Deduct currency items from character equipment.
    pub fn deduct_equipment_coin(&self, character: &mut Character, amount: i32) {
        deduct_coin(character, amount);
    }
}

fn as_record(raw: &JsonObject) -> Option<InventoryItemRecord> {
    serde_json::from_value(Value::Object(raw.clone())).ok()
}

fn record_type(item: &InventoryItemRecord) -> String {
    if !item.r#type.is_empty() {
        item.r#type.clone()
    } else {
        item.item_type.clone().unwrap_or_default()
    }
}

fn legacy_gold(character: &Character) -> i32 {
    match character.class_abilities.get("gold") {
        Some(Value::Bool(b)) => *b as i32,
        Some(Value::Number(n)) => n
            .as_i64()
            .map(|x| x.max(0) as i32)
            .or_else(|| n.as_f64().map(|f| f.max(0.0) as i32))
            .unwrap_or(0),
        Some(Value::String(s)) => s.parse::<f64>().ok().map(|f| f.max(0.0) as i32).unwrap_or(0),
        _ => 0,
    }
}

fn deduct_legacy_payment(character: &mut Character, amount: i32) {
    let gold = legacy_gold(character);
    if gold >= amount {
        character
            .class_abilities
            .insert("gold".into(), JsonValue::from(gold - amount));
        return;
    }
    character.class_abilities.insert("gold".into(), JsonValue::from(0));
    deduct_coin(character, amount - gold);
}

fn equipment_coin(character: &Character) -> i32 {
    let mut total = 0i64;
    for raw in &character.equipment {
        if let Some(item) = as_record(raw) {
            if record_type(&item) == "currency" {
                total += item.currency_amount();
            }
        }
    }
    total as i32
}

fn to_object(item: &InventoryItemRecord) -> JsonObject {
    match serde_json::to_value(item) {
        Ok(Value::Object(m)) => m,
        _ => JsonObject::new(),
    }
}

fn deduct_coin(character: &mut Character, amount: i32) {
    let mut remaining = amount as i64;
    let mut new_equipment: Vec<JsonObject> = Vec::new();
    for raw in &character.equipment {
        let Some(item) = as_record(raw) else {
            new_equipment.push(raw.clone());
            continue;
        };
        if remaining <= 0 || record_type(&item) != "currency" {
            new_equipment.push(to_object(&item));
            continue;
        }
        let coin_in_item = item.currency_amount();
        if coin_in_item <= remaining {
            remaining -= coin_in_item;
            // Drop this item entirely.
        } else {
            let mut updated = item.clone();
            if updated.data.contains_key("amount") {
                updated
                    .data
                    .insert("amount".into(), JsonValue::from(coin_in_item - remaining));
            } else if updated.value != 0 {
                updated.value = (coin_in_item - remaining) as i32;
            }
            remaining = 0;
            new_equipment.push(to_object(&updated));
        }
    }
    character.equipment = new_equipment;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn rest_restores_and_clamps() {
        let mut c = Character::new("Hero", "warrior");
        c.max_hp = 10;
        c.hp = 5;
        let r = HealingSystem.rest(&mut c, 50);
        assert!(r.healed);
        assert_eq!(c.hp, (5 + c.level).min(10).max(6));
        // resting at full does nothing
        c.hp = 10;
        let r2 = HealingSystem.rest(&mut c, 50);
        assert!(!r2.healed);
    }

    #[test]
    fn town_healer_requires_coin() {
        let mut c = Character::new("Hero", "warrior");
        c.max_hp = 10;
        c.hp = 6;
        // no coin -> cannot afford
        let r = HealingSystem.town_healer(&mut c, 5, Some(0), true);
        assert!(!r.healed);
        assert_eq!(r.cost, 20);
        // with external gold -> heals fully
        let r2 = HealingSystem.town_healer(&mut c, 5, Some(100), false);
        assert!(r2.healed);
        assert_eq!(c.hp, 10);
    }

    #[test]
    fn use_item_heals() {
        let mut c = Character::new("Hero", "warrior");
        c.max_hp = 20;
        c.hp = 1;
        let mut item: InventoryItemRecord =
            serde_json::from_str(r#"{"name":"Potion","data":{"healing":"2d6+2"}}"#).unwrap();
        item.name = "Potion".into();
        let mut rng = StdRng::seed_from_u64(3);
        let r = HealingSystem.use_healing_item(&mut c, &item, &mut rng);
        assert!(r.healed);
        assert!(c.hp > 1);
    }

    #[test]
    fn deduct_legacy_gold_first() {
        let mut c = Character::new("Hero", "warrior");
        c.class_abilities.insert("gold".into(), JsonValue::from(50));
        deduct_legacy_payment(&mut c, 20);
        assert_eq!(legacy_gold(&c), 30);
    }
}
