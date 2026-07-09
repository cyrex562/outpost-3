//! Entity IR records (R5-A2) — `creature` and `item`.
//!
//! Field sets mirror the Python `CreatureData` / `ItemData` models (the host maps
//! IR ↔ Pydantic), with the action-model changes signed off in the primitives
//! design §9: `creature` gains `actions` / `pools` / `defenses`, retires
//! `attack_skill` (subsumed by the skill each action names) and folds
//! `innate_attacks` into `actions`; `item` gains `grants_traits` (C2 — an item
//! grants a trait that carries its modifiers, reusing the trait machinery).
//!
//! `npc` and `character` are deferred (C1); only `creature` + `item` are
//! authorable in v1.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::resolution::damage::Pool;

// ---------------------------------------------------------------------------
// creature
// ---------------------------------------------------------------------------

/// A material harvestable from a creature's corpse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Harvestable {
    pub material: String,
    #[serde(default = "default_survive")]
    pub skill: String,
    #[serde(default = "default_8")]
    pub difficulty: i64,
}

/// A creature/monster record. Mirrors `CreatureData` + the action-model fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Creature {
    pub id: String,
    pub name: String,
    pub hd: i64,
    #[serde(default = "default_4")]
    pub hp_per_hd: i64,
    pub ac: i64,
    pub attack_bonus: i64,
    pub damage: String,
    #[serde(default = "default_melee")]
    pub damage_type: String,
    #[serde(default = "default_attacks")]
    pub attack_description: String,
    #[serde(default = "default_1")]
    pub num_attacks: i64,
    #[serde(default = "default_aggressive")]
    pub behavior: String,
    #[serde(default = "default_8")]
    pub awareness_difficulty: i64,
    #[serde(default = "default_8")]
    pub flee_difficulty: i64,
    #[serde(default)]
    pub unavoidable: bool,
    #[serde(default = "default_8")]
    pub morale: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loot_table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harvestable: Option<Harvestable>,
    #[serde(default)]
    pub description_unseen: String,
    #[serde(default)]
    pub description_seen: String,
    #[serde(default)]
    pub description_short: String,
    #[serde(default = "default_15")]
    pub xp_value: i64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub special_abilities: Vec<String>,
    /// Intrinsic trait ids the creature carries (e.g. an engineered beast's
    /// venomous bite). These source `has_trait(self, …)` and their modifiers in
    /// combat without needing an equipped item to grant them.
    #[serde(default)]
    pub traits: Vec<String>,
    // --- action-model additions ---
    /// Action ids the creature can perform (subsumes the former `innate_attacks`).
    #[serde(default)]
    pub actions: Vec<String>,
    /// Typed health pools. Empty → the host derives an `hp` pool from `hd * hp_per_hd`.
    #[serde(default)]
    pub pools: Vec<Pool>,
    /// Named defenses. Empty → the host derives `{ac}` from the `ac` field.
    #[serde(default)]
    pub defenses: BTreeMap<String, i64>,
}

// ---------------------------------------------------------------------------
// item
// ---------------------------------------------------------------------------

/// Save bonuses an item confers (mirrors `SaveBonusProfile`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct SaveBonusProfile {
    #[serde(default)]
    pub physical: i64,
    #[serde(default)]
    pub evasion: i64,
    #[serde(default)]
    pub mental: i64,
    #[serde(default)]
    pub luck: i64,
}

/// A consumable's effect (discriminated on `type`, mirroring the Python union).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ItemEffect {
    Food {
        days: i64,
    },
    Heal {
        dice: String,
    },
    SaveBonus {
        save_type: String,
        bonus: i64,
        duration: i64,
    },
}

/// An item record. Mirrors `ItemData` + `grants_traits`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Item {
    pub id: String,
    pub name: String,
    /// Encumbrance (may be fractional).
    #[serde(default)]
    pub enc: f64,
    #[serde(default)]
    pub cost: i64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    #[serde(default)]
    pub shock_damage: i64,
    #[serde(default)]
    pub shock_ac_threshold: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_band: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ammo_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac_bonus: Option<i64>,
    #[serde(default)]
    pub weapon_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<ItemEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_charges: Option<i64>,
    #[serde(default)]
    pub save_bonuses: SaveBonusProfile,
    /// Traits this item grants while equipped (carries the item's modifiers — C2).
    #[serde(default)]
    pub grants_traits: Vec<String>,
}

// --- defaults mirroring the Python models ---
fn default_4() -> i64 {
    4
}
fn default_1() -> i64 {
    1
}
fn default_8() -> i64 {
    8
}
fn default_15() -> i64 {
    15
}
fn default_melee() -> String {
    "melee".to_string()
}
fn default_attacks() -> String {
    "attacks".to_string()
}
fn default_aggressive() -> String {
    "aggressive".to_string()
}
fn default_survive() -> String {
    "survive".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn creature_applies_python_defaults() {
        let c: Creature = serde_json::from_value(json!({
            "id": "scrip_hound", "name": "Scrip-Hound", "hd": 2, "ac": 14,
            "attack_bonus": 3, "damage": "1d8"
        }))
        .unwrap();
        assert_eq!(c.hp_per_hd, 4);
        assert_eq!(c.num_attacks, 1);
        assert_eq!(c.damage_type, "melee");
        assert_eq!(c.morale, 8);
        assert_eq!(c.xp_value, 15);
        assert!(c.actions.is_empty());
        assert!(c.pools.is_empty()); // host derives hp from hd*hp_per_hd
    }

    #[test]
    fn creature_with_actions_and_harvestable() {
        let c: Creature = serde_json::from_value(json!({
            "id": "scrip_hound", "name": "Scrip-Hound", "hd": 2, "ac": 14,
            "attack_bonus": 3, "damage": "1d8",
            "actions": ["corrosive_bite"],
            "harvestable": { "material": "corrosive_gland", "skill": "Heal", "difficulty": 9 },
            "tags": ["engineered", "beast"]
        }))
        .unwrap();
        assert_eq!(c.actions, vec!["corrosive_bite"]);
        let h = c.harvestable.unwrap();
        assert_eq!(h.material, "corrosive_gland");
        assert_eq!(h.skill, "Heal");
    }

    #[test]
    fn item_weapon_and_effect() {
        let weapon: Item = serde_json::from_value(json!({
            "id": "reaver_knife", "name": "Reaver's Knife", "enc": 1, "cost": 15,
            "tags": ["weapon", "melee"], "damage": "1d6", "attribute": "DEX",
            "shock_damage": 1, "shock_ac_threshold": 15, "weapon_tags": ["light"],
            "grants_traits": ["keen_edge"]
        }))
        .unwrap();
        assert_eq!(weapon.damage.as_deref(), Some("1d6"));
        assert_eq!(weapon.shock_damage, 1);
        assert_eq!(weapon.grants_traits, vec!["keen_edge"]);

        let potion: Item = serde_json::from_value(json!({
            "id": "med_kit", "name": "Med Kit",
            "effect": { "type": "heal", "dice": "1d6+1" }
        }))
        .unwrap();
        match potion.effect.unwrap() {
            ItemEffect::Heal { dice } => assert_eq!(dice, "1d6+1"),
            other => panic!("expected heal, got {other:?}"),
        }
    }

    #[test]
    fn save_bonus_effect_round_trips() {
        let value =
            json!({"type": "save_bonus", "save_type": "physical", "bonus": 1, "duration": 6});
        let e: ItemEffect = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&e).unwrap(), value);
    }
}
