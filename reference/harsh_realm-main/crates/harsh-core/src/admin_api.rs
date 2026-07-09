//! Admin API request models.
//!
//! Ported from `src/harsh_realm/models/admin_api.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

fn d_eight() -> i32 {
    8
}

/// Request body for creating or updating a skill mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMappingUpdateRequest {
    /// Skill.
    pub skill: String,
    /// Attribute.
    pub attribute: String,
    /// Base difficulty (default 8).
    #[serde(default = "d_eight")]
    pub base_difficulty: i32,
    /// Whether opposed.
    #[serde(default)]
    pub opposed: bool,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Notes.
    #[serde(default)]
    pub notes: String,
}

/// Request body for creating or updating a difficulty target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifficultyTargetUpdateRequest {
    /// Target number.
    pub target: i32,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Request body for creating or updating a disposition outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositionOutcomeUpdateRequest {
    /// Disposition delta.
    pub delta: i32,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Request body for creating or updating an encounter weight modifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterWeightUpdateRequest {
    /// Weight modifier.
    pub weight_modifier: i32,
}

/// Request body for partially updating a faction asset stat record.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FactionAssetUpdateRequest {
    /// Category.
    #[serde(default)]
    pub category: Option<String>,
    /// Minimum attribute.
    #[serde(default)]
    pub min_attribute: Option<i32>,
    /// Cost.
    #[serde(default)]
    pub cost: Option<i32>,
    /// Upkeep.
    #[serde(default)]
    pub upkeep: Option<i32>,
    /// Max HP.
    #[serde(default)]
    pub max_hp: Option<i32>,
    /// Attack stat.
    #[serde(default)]
    pub attack_stat: Option<String>,
    /// Counter stat.
    #[serde(default)]
    pub counter_stat: Option<String>,
    /// Attack roll.
    #[serde(default)]
    pub attack_roll: Option<String>,
    /// Special rule.
    #[serde(default)]
    pub special: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
}

/// Request body for creating or updating an XP progression entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct XpProgressionUpdateRequest {
    /// XP needed.
    pub xp_needed: i32,
}

/// Request body for creating or updating a random table.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RandomTableUpdateRequest {
    /// Category.
    #[serde(default)]
    pub category: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Entries.
    #[serde(default)]
    pub entries: Vec<JsonObject>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Source.
    #[serde(default)]
    pub source: String,
}
