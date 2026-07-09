//! Faction models, reputation, and turn-engine logic.

pub mod reputation;
pub mod turn_engine;
pub mod turn_support;

pub use reputation::{delta_for_action, reputation_to_disposition, REPUTATION_DELTAS};
pub use turn_engine::{
    resolve_attack, resolve_create, resolve_expand, resolve_harvest, resolve_refit, resolve_repair,
    resolve_seize, AttackOutcome, TICKS_PER_WEEK,
};
pub use turn_support::{parse_dice_max, parse_dice_with, SIGNIFICANT_ACTIONS};

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

fn d_seven() -> i32 {
    7
}
fn d_one() -> i32 {
    1
}

/// Typed runtime state for a faction asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FactionAssetState {
    /// Whether the asset is expanded.
    #[serde(default)]
    pub expanded: bool,
}

/// Typed runtime state for a faction relation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FactionRelationState {
    /// Relation history.
    #[serde(default)]
    pub history: Vec<String>,
}

/// In-memory representation of a faction row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionData {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// HP (default 7).
    #[serde(default = "d_seven")]
    pub hp: i32,
    /// Max HP (default 7).
    #[serde(default = "d_seven")]
    pub max_hp: i32,
    /// Force stat.
    #[serde(default)]
    pub force: i32,
    /// Cunning stat.
    #[serde(default)]
    pub cunning: i32,
    /// Wealth stat.
    #[serde(default)]
    pub wealth: i32,
    /// XP.
    #[serde(default)]
    pub xp: i32,
    /// Home column.
    #[serde(default)]
    pub home_q: Option<i32>,
    /// Home row.
    #[serde(default)]
    pub home_r: Option<i32>,
    /// Goals.
    #[serde(default)]
    pub goals: Vec<String>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}

/// In-memory representation of a faction asset row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionAssetData {
    /// Id.
    pub id: String,
    /// Owning faction id.
    pub faction_id: String,
    /// Asset type.
    pub asset_type: String,
    /// Category.
    pub category: String,
    /// HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}

/// Result of a single faction action during a turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionActionResult {
    /// Faction id.
    pub faction_id: String,
    /// Faction name.
    pub faction_name: String,
    /// Action name.
    pub action: String,
    /// Whether it succeeded.
    pub success: bool,
    /// Details payload.
    #[serde(default)]
    pub details: JsonObject,
    /// Narration text.
    #[serde(default)]
    pub narration: String,
}

/// Outcome of checking whether weekly faction turns should run.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WeeklyFactionTurnResult {
    /// Whether turns ran.
    pub ran: bool,
    /// Per-action results.
    #[serde(default)]
    pub results: Vec<FactionActionResult>,
}

/// Parsed special-rule payload from faction asset stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionAssetSpecialRule {
    /// Ability key.
    #[serde(default)]
    pub ability: String,
    /// Amount (default 1).
    #[serde(default = "d_one")]
    pub amount: i32,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl FactionAssetSpecialRule {
    /// Parse a faction asset special-rule JSON string, or `None` if invalid.
    pub fn from_raw(raw: &str) -> Option<Self> {
        if raw.is_empty() {
            return None;
        }
        serde_json::from_str(raw).ok()
    }
}

/// Chosen action and typed parameters for one faction turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactionActionChoice {
    /// Action name.
    pub action_name: String,
    /// Typed parameters.
    pub params: FactionActionParams,
}

/// Typed parameters for a faction action (replaces the Python params hierarchy).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FactionActionParams {
    /// Attack: an attacker and a target asset.
    Attack {
        /// Attacking asset.
        attacker_asset: FactionAssetData,
        /// Target asset.
        target_asset: FactionAssetData,
    },
    /// Operate on a single asset.
    Asset {
        /// The asset.
        asset: FactionAssetData,
    },
    /// Create an asset of a given type.
    Create {
        /// Asset type to create.
        asset_type: String,
    },
    /// No extra data beyond an optional details map.
    Empty {
        /// Details.
        #[serde(default)]
        details: std::collections::BTreeMap<String, String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faction_defaults() {
        let f: FactionData = serde_json::from_str(r#"{"id":"f1","name":"Reavers"}"#).unwrap();
        assert_eq!(f.hp, 7);
        assert_eq!(f.max_hp, 7);
        assert_eq!(f.force, 0);
    }

    #[test]
    fn special_rule_from_raw() {
        assert!(FactionAssetSpecialRule::from_raw("").is_none());
        assert!(FactionAssetSpecialRule::from_raw("not json").is_none());
        let r = FactionAssetSpecialRule::from_raw(r#"{"ability":"raid","amount":3}"#).unwrap();
        assert_eq!(r.ability, "raid");
        assert_eq!(r.amount, 3);
    }
}
