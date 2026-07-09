//! Transient combat-subsystem state and combat result value objects.
//!
//! Ported from `src/harsh_realm/models/combat_runtime.py`. These represent an
//! in-progress encounter, not durable ownership.

use serde::{Deserialize, Serialize};

use crate::engine_runtime::{HarvestableRecord, PendingVeteranLuckRecord};
use crate::runtime::JsonObject;

/// Minimal shape required for flee-difficulty computation.
pub trait FleeOpponent {
    /// Difficulty of fleeing from this opponent.
    fn flee_difficulty(&self) -> i32;
}

/// Outcome of a pre-combat awareness check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AwarenessResult {
    /// Player surprises the enemy.
    PlayerSurprise,
    /// Both sides aware.
    MutualAwareness,
    /// Enemy surprises the player.
    EnemySurprise,
}

/// Full details of an awareness check outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwarenessCheckResult {
    /// Outcome.
    pub result: AwarenessResult,
    /// Player roll.
    pub player_roll: i32,
    /// Player total.
    pub player_total: i32,
    /// Player difficulty.
    pub player_difficulty: i32,
    /// Whether the player succeeded.
    pub player_succeeded: bool,
    /// Enemy roll.
    pub enemy_roll: i32,
    /// Enemy difficulty.
    pub enemy_difficulty: i32,
    /// Whether the enemy succeeded.
    pub enemy_succeeded: bool,
    /// Skill used.
    pub skill_used: String,
    /// Narration text.
    pub narration: String,
}

fn d_true() -> bool {
    true
}
fn d_melee() -> String {
    "melee".to_string()
}

/// Represents one participant in combat.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Combatant {
    /// Entity id.
    pub entity_id: String,
    /// Internal name.
    pub name: String,
    /// Display name.
    pub display_name: String,
    /// Whether this is the player.
    pub is_player: bool,
    /// Initiative roll.
    pub initiative: i32,
    /// Current HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// Armour class.
    pub ac: i32,
    /// Attack bonus.
    pub attack_bonus: i32,
    /// Damage expression.
    pub damage_expr: String,
    /// Attack narration phrase.
    pub attack_description: String,
    /// AI behaviour.
    pub behavior: String,
    /// Attacks per turn.
    pub num_attacks: i32,
    /// Alive flag.
    #[serde(default = "d_true")]
    pub alive: bool,
    /// Equipped weapon id.
    #[serde(default)]
    pub weapon_id: Option<String>,
    /// Range band (default `"melee"`).
    #[serde(default = "d_melee")]
    pub range_band: String,
    /// Battle-grid column (0-based, assigned by `assign_positions`).
    #[serde(default)]
    pub q: i32,
    /// Battle-grid row (0-based, assigned by `assign_positions`).
    #[serde(default)]
    pub r: i32,
    /// Intrinsic IR trait ids this combatant carries (from creature/character
    /// data). Sourced into the eval context so intrinsic traits fire triggers
    /// and apply modifiers, alongside equipped-item grants and active statuses.
    #[serde(default)]
    pub traits: Vec<String>,
    /// Typed health pools beyond the canonical `hp` (e.g. shields, MDC), with
    /// live current values. Empty for the common single-`hp` combatant.
    #[serde(default)]
    pub pools: Vec<crate::resolution::damage::Pool>,
    /// Named defenses carried from creature/character data (avoidance values;
    /// `dr`/`armor` also act as IR damage mitigation).
    #[serde(default)]
    pub defenses: std::collections::BTreeMap<String, i64>,
    /// Authored action ids this combatant can perform on its turn (from
    /// creature/character data; resolved via `combat::action_resolver`).
    #[serde(default)]
    pub actions: Vec<String>,
    /// Inventory payloads.
    #[serde(default)]
    pub inventory: Vec<JsonObject>,
    /// Carried gold.
    #[serde(default)]
    pub gold: i32,
}

/// Full state for an active combat encounter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatState {
    /// Combatants.
    pub combatants: Vec<Combatant>,
    /// Initiative order (entity ids).
    pub initiative_order: Vec<String>,
    /// Current turn index.
    #[serde(default)]
    pub current_turn_index: i32,
    /// Round number.
    #[serde(default = "d_one")]
    pub round_number: i32,
    /// Terrain id of the world cell where the encounter takes place
    /// (used as the backdrop terrain for every cell in the battle grid).
    #[serde(default)]
    pub terrain: String,
    /// World-cell feature ids stamped onto the corner cells of the battle grid
    /// (up to 4; i-th feature → i-th corner).
    #[serde(default)]
    pub features: Vec<String>,
    /// Player surprise flag.
    #[serde(default)]
    pub player_surprise: bool,
    /// Enemy surprise flag.
    #[serde(default)]
    pub enemy_surprise: bool,
    /// Veteran's Luck used.
    #[serde(default)]
    pub veteran_luck_used: bool,
    /// First aid used.
    #[serde(default)]
    pub first_aid_used: bool,
    /// Enemy detail revealed.
    #[serde(default)]
    pub enemy_detail_revealed: bool,
    /// Fled flag.
    #[serde(default)]
    pub fled: bool,
    /// Combat over flag.
    #[serde(default)]
    pub combat_over: bool,
    /// Last stand active.
    #[serde(default)]
    pub last_stand_active: bool,
    /// Pending Veteran's Luck hit.
    #[serde(default)]
    pub pending_veteran_luck: Option<PendingVeteranLuckRecord>,
    /// Pending harvestables.
    #[serde(default)]
    pub pending_harvest: Vec<HarvestableRecord>,
    /// Defeated enemy ids.
    #[serde(default)]
    pub defeated_enemies: Vec<String>,
    /// Loot dropped on the battle grid by defeated enemies (HR-787). One entry
    /// per defeated enemy (recorded even when empty, so loot is rolled once);
    /// aggregated into the player's inventory at victory.
    #[serde(default)]
    pub ground_loot: Vec<GroundLoot>,
}

fn d_one() -> i32 {
    1
}

/// A defeated enemy's loot drop, positioned at its grid tile (HR-787).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroundLoot {
    /// Entity id of the defeated enemy that dropped this pile.
    pub entity_id: String,
    /// Grid column of the drop.
    pub q: i32,
    /// Grid row of the drop.
    pub r: i32,
    /// Item payloads dropped (name/type/value objects).
    #[serde(default)]
    pub items: Vec<JsonObject>,
    /// Currency dropped.
    #[serde(default)]
    pub currency: i32,
}

/// Result of a flee attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleeResult {
    /// Whether the flee succeeded.
    pub success: bool,
    /// Whether it was a clean getaway.
    pub clean: bool,
    /// Skill-check roll.
    pub skill_check_roll: i32,
    /// Skill-check total.
    pub skill_check_total: i32,
    /// Skill-check difficulty.
    pub skill_check_difficulty: i32,
    /// Consequence, if any.
    pub consequence: Option<String>,
    /// Destination column.
    pub destination_q: i32,
    /// Destination row.
    pub destination_r: i32,
    /// Damage taken.
    pub damage_taken: i32,
    /// Item lost, if any.
    pub item_lost: Option<String>,
    /// Narration text.
    pub narration: String,
}

/// Result of a Last Stand action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastStandResult {
    /// Action taken.
    pub action_taken: String,
    /// Whether the character survived.
    pub survived: bool,
    /// Narration text.
    pub narration: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awareness_enum_snake_case() {
        assert_eq!(
            serde_json::to_string(&AwarenessResult::EnemySurprise).unwrap(),
            "\"enemy_surprise\""
        );
    }

    #[test]
    fn combat_state_defaults() {
        let s: CombatState =
            serde_json::from_str(r#"{"combatants":[],"initiative_order":[]}"#).unwrap();
        assert_eq!(s.round_number, 1);
        assert_eq!(s.current_turn_index, 0);
        assert!(!s.combat_over);
        // New fields with serde defaults.
        assert_eq!(s.terrain, "");
        assert!(s.features.is_empty());
    }

    #[test]
    fn combatant_grid_defaults() {
        // Existing serialized combatants without q/r should still deserialize cleanly.
        let c: Combatant = serde_json::from_str(r#"{
            "entity_id":"e1","name":"e","display_name":"e","is_player":false,
            "initiative":0,"hp":5,"max_hp":5,"ac":10,"attack_bonus":0,
            "damage_expr":"1d4","attack_description":"hits","behavior":"melee",
            "num_attacks":1
        }"#).unwrap();
        assert_eq!(c.q, 0);
        assert_eq!(c.r, 0);
    }
}
