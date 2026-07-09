//! Frozen engine result value objects returned by engine systems.
//!
//! Ported from `src/harsh_realm/models/engine_results.py`. The Python
//! `DiceResult` defined here is the same shape as [`crate::dice::DiceResult`];
//! the canonical Rust definition lives in `dice.rs` and is re-used (not
//! duplicated).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::engine_runtime::{DiscoveryFindRecord, EncounterRecord};
use crate::runtime::JsonObject;

pub use crate::dice::DiceResult;

/// Result of a damage roll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageResult {
    /// Summed dice.
    pub roll: i32,
    /// Flat modifier.
    pub modifier: i32,
    /// Total damage.
    pub total: i32,
    /// Source weapon damage expression.
    pub weapon_expr: String,
}

/// Result of a full attack resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackResult {
    /// d20 roll.
    pub roll: i32,
    /// Roll + modifiers.
    pub total: i32,
    /// Target armor class.
    pub target_ac: i32,
    /// Whether the attack hit.
    pub hit: bool,
    /// Natural 1.
    pub natural_1: bool,
    /// Natural 20.
    pub natural_20: bool,
    /// Damage dealt, when the attack hit.
    pub damage: Option<DamageResult>,
    /// Attacker name.
    pub attacker_name: String,
    /// Target name.
    pub target_name: String,
    /// Narration text.
    pub narration: String,
}

/// Result of awarding XP after combat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XPAwardResult {
    /// XP gained.
    pub xp_gained: i32,
    /// New XP total.
    pub new_total: i32,
    /// Previous XP total.
    pub old_total: i32,
    /// Whether a level-up was triggered.
    pub level_up: bool,
    /// New level.
    pub new_level: i32,
    /// Narration text.
    pub narration: String,
}

/// Result of applying a level up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelUpResult {
    /// New level.
    pub new_level: i32,
    /// HP gained.
    pub hp_gained: i32,
    /// New max HP.
    pub new_max_hp: i32,
    /// Skill points granted.
    pub skill_points: i32,
    /// New attack bonus.
    pub new_attack_bonus: i32,
    /// Whether saves improved.
    pub saves_improved: bool,
    /// Narration text.
    pub narration: String,
}

/// A decision made by an enemy combatant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnemyAction {
    /// Action kind.
    pub action_type: String,
    /// Target entity id.
    pub target_id: String,
}

/// Result of a healing action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealingResult {
    /// Whether healing occurred.
    pub healed: bool,
    /// HP restored.
    pub hp_restored: i32,
    /// New HP.
    pub new_hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// Healing method.
    pub method: String,
    /// Cost paid.
    pub cost: i32,
    /// Skill-check roll, if a check was made.
    pub skill_check_roll: Option<i32>,
    /// Skill-check total, if a check was made.
    pub skill_check_total: Option<i32>,
    /// Skill-check difficulty, if a check was made.
    pub skill_check_difficulty: Option<i32>,
    /// Narration text.
    pub narration: String,
}

/// Result of using an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemUseResult {
    /// Whether the use succeeded.
    pub success: bool,
    /// Item name.
    pub item_name: String,
    /// Effect description.
    pub effect: String,
    /// Narration text.
    pub narration: String,
    /// Error message, when unsuccessful.
    pub error: Option<String>,
}

/// Result of a table roll.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableResult {
    /// Table id.
    pub table_id: String,
    /// Chain of nested table ids rolled.
    pub roll_chain: Vec<String>,
    /// Result type.
    pub result_type: String,
    /// Result payload.
    pub result: JsonObject,
    /// Raw text, when present.
    pub raw_text: Option<String>,
}

/// The result of an encounter check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncounterResult {
    /// Encounter type.
    pub encounter_type: String,
    /// Name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Spawned entity id, if any.
    pub entity_id: Option<String>,
    /// Typed encounter payload.
    pub data: EncounterRecord,
}

/// All derived values from a character recalculation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecalcResult {
    /// Attribute modifiers by attribute key.
    pub attr_mods: BTreeMap<String, i32>,
    /// Max HP.
    pub max_hp: i32,
    /// Armor class.
    pub ac: i32,
    /// Attack bonus.
    pub attack_bonus: i32,
    /// Melee attack total.
    pub melee_attack: i32,
    /// Ranged attack total.
    pub ranged_attack: i32,
    /// Physical save.
    pub physical_save: i32,
    /// Evasion save.
    pub evasion_save: i32,
    /// Mental save.
    pub mental_save: i32,
}

/// Result of a 2d6 skill check rolled during a hex search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoverySkillCheck {
    /// Skill used.
    pub skill: String,
    /// Attribute used.
    pub attribute: String,
    /// Raw roll.
    pub roll: i32,
    /// Modifier applied.
    pub modifier: i32,
    /// Total.
    pub total: i32,
    /// Difficulty.
    pub difficulty: i32,
    /// Whether it succeeded.
    pub success: bool,
}

/// Result of a hex search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveryResult {
    /// Whether anything was found.
    pub found: bool,
    /// Broad category of the find, or `None`.
    pub category: Option<String>,
    /// Typed result payload, or `None`.
    pub result: Option<DiscoveryFindRecord>,
    /// Narration text.
    pub message: String,
    /// Skill-check details, if one was performed.
    pub skill_check: Option<DiscoverySkillCheck>,
    /// Item definitions for `category == "item"` results.
    #[serde(default)]
    pub discovered_items: Vec<JsonObject>,
    /// Updated cell data, if any.
    #[serde(default)]
    pub updated_data: Option<JsonObject>,
    /// Updated cell features, if any.
    #[serde(default)]
    pub updated_features: Option<Vec<String>>,
}

/// Result of a unified skill check resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCheckResult {
    /// Triggering command verb.
    pub verb: String,
    /// XWN skill used.
    pub skill: String,
    /// Attribute modifier applied.
    pub attribute: String,
    /// Raw 2d6 roll.
    pub roll: i32,
    /// Total modifier (skill level + attr mod).
    pub modifier: i32,
    /// Roll + modifier.
    pub total: i32,
    /// Target number.
    pub difficulty: i32,
    /// total - difficulty.
    pub margin: i32,
    /// Whether the check succeeded.
    pub success: bool,
    /// Classification band.
    pub outcome_key: String,
    /// NPC disposition change.
    pub disposition_delta: i32,
    /// Narration text.
    pub narration: String,
}

/// Result of a saving throw roll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveResult {
    /// Save category.
    pub save_type: String,
    /// d20 roll.
    pub roll: i32,
    /// Modifier applied.
    pub modifier: i32,
    /// Total.
    pub total: i32,
    /// Target number.
    pub target: i32,
    /// Whether the save passed.
    pub passed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_result_round_trips() {
        let a = AttackResult {
            roll: 18,
            total: 20,
            target_ac: 15,
            hit: true,
            natural_1: false,
            natural_20: false,
            damage: Some(DamageResult {
                roll: 5,
                modifier: 1,
                total: 6,
                weapon_expr: "1d8+1".to_string(),
            }),
            attacker_name: "Hero".to_string(),
            target_name: "Orc".to_string(),
            narration: "hits".to_string(),
        };
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<AttackResult>(&json).unwrap(), a);
    }

    #[test]
    fn discovery_result_optional_fields_default() {
        let d: DiscoveryResult = serde_json::from_str(
            r#"{"found":false,"category":null,"result":null,"message":"nothing","skill_check":null}"#,
        )
        .unwrap();
        assert!(!d.found);
        assert!(d.discovered_items.is_empty());
        assert!(d.updated_data.is_none());
    }

    #[test]
    fn save_result_fields() {
        let s = SaveResult {
            save_type: "physical".to_string(),
            roll: 12,
            modifier: 2,
            total: 14,
            target: 15,
            passed: false,
        };
        assert!(!s.passed);
        assert_eq!(s.total, 14);
    }
}
