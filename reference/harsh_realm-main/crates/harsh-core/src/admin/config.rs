//! Admin config models (mappings, weights, tables). Ported from
//! `models/admin/config.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

fn d_eight() -> i32 {
    8
}

/// Maps a social/action verb to an XWN skill check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillMapping {
    /// Verb.
    pub verb: String,
    /// Skill.
    pub skill: String,
    /// Attribute.
    pub attribute: String,
    /// Base difficulty (default 8).
    #[serde(default = "d_eight")]
    pub base_difficulty: i32,
    /// Whether the check is opposed.
    #[serde(default)]
    pub opposed: bool,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Notes.
    #[serde(default)]
    pub notes: String,
}

/// Named difficulty level for the rules engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifficultyTarget {
    /// Name.
    pub name: String,
    /// Target number.
    pub target: i32,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Disposition change per skill check outcome band.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispositionOutcome {
    /// Outcome key.
    pub outcome_key: String,
    /// Disposition delta.
    pub delta: i32,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Encounter weight modifier based on faction disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterWeight {
    /// Faction disposition.
    pub faction_disposition: String,
    /// Encounter tag.
    pub encounter_tag: String,
    /// Weight modifier.
    pub weight_modifier: i32,
}

/// WWN faction asset definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactionAssetStat {
    /// Asset type.
    pub asset_type: String,
    /// Category.
    pub category: String,
    /// Minimum attribute.
    pub min_attribute: i32,
    /// Cost.
    pub cost: i32,
    /// Upkeep.
    #[serde(default)]
    pub upkeep: i32,
    /// Max HP (default 1).
    #[serde(default = "one")]
    pub max_hp: i32,
    /// Attack stat.
    #[serde(default)]
    pub attack_stat: String,
    /// Counter stat.
    #[serde(default)]
    pub counter_stat: String,
    /// Attack roll.
    #[serde(default)]
    pub attack_roll: String,
    /// Special rule.
    #[serde(default)]
    pub special: String,
    /// Description.
    #[serde(default)]
    pub description: String,
}

fn one() -> i32 {
    1
}

/// XP threshold for a character level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct XpProgression {
    /// Level.
    pub level: i32,
    /// XP needed.
    pub xp_needed: i32,
}

/// Full export snapshot for world-scoped admin config tables.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdminConfigExport {
    /// Skill mappings.
    #[serde(default)]
    pub skill_mappings: Vec<SkillMapping>,
    /// Difficulty targets.
    #[serde(default)]
    pub difficulty_targets: Vec<DifficultyTarget>,
    /// Disposition outcomes.
    #[serde(default)]
    pub disposition_outcomes: Vec<DispositionOutcome>,
    /// Encounter weights.
    #[serde(default)]
    pub encounter_weights: Vec<EncounterWeight>,
    /// Faction asset stats.
    #[serde(default)]
    pub faction_asset_stats: Vec<FactionAssetStat>,
}

/// A random table stored in per-world SQLite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomTable {
    /// Id.
    pub id: String,
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
