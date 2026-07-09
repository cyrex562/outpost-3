//! Typed oracle chart, event-table, and Adventure Crafter models.
//!
//! Ported from `src/harsh_realm/models/oracle.py`. (The oracle *engine* logic
//! lives in `oracle.rs`; this module holds the data value types.)

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;
use crate::scene_data::AdventureScene;

/// Mythic GME likelihood levels, ordered from least to most likely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Likelihood {
    /// Impossible.
    Impossible,
    /// No way.
    NoWay,
    /// Very unlikely.
    VeryUnlikely,
    /// Unlikely.
    Unlikely,
    /// Even odds.
    EvenOdds,
    /// Somewhat likely.
    SomewhatLikely,
    /// Likely.
    Likely,
    /// Very likely.
    VeryLikely,
    /// Certain.
    Certain,
}

/// Possible outcomes of a fate chart check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FateResult {
    /// Exceptional yes.
    ExceptionalYes,
    /// Yes.
    Yes,
    /// No.
    No,
    /// Exceptional no.
    ExceptionalNo,
}

/// Possible scene modification outcomes from a Mythic scene check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SceneModification {
    /// Scene plays out as expected.
    Normal,
    /// Scene plays out but differently.
    Altered,
    /// New random event fires instead.
    Interrupt,
}

/// Result of a fate chart check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FateCheckResult {
    /// Likelihood label.
    pub likelihood: String,
    /// Chaos factor.
    pub chaos_factor: i32,
    /// d100 roll.
    pub roll: i32,
    /// Outcome.
    pub result: FateResult,
    /// Narration text.
    pub narration: String,
}

/// Result of a Mythic scene check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneCheckResult {
    /// Roll.
    pub roll: i32,
    /// Chaos factor.
    pub chaos_factor: i32,
    /// Modification.
    pub modification: SceneModification,
    /// Narration text.
    pub narration: String,
}

/// A generated Mythic random event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomEvent {
    /// Focus.
    pub focus: String,
    /// Action.
    pub action: String,
    /// Subject.
    pub subject: String,
    /// Focus roll.
    pub focus_roll: i32,
    /// Action roll.
    pub action_roll: i32,
    /// Subject roll.
    pub subject_roll: i32,
    /// Narration text.
    pub narration: String,
}

fn d_active() -> String {
    "active".to_string()
}

/// An active Adventure Crafter plotline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plotline {
    /// Id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Theme.
    pub theme: String,
    /// Status (default `"active"`).
    #[serde(default = "d_active")]
    pub status: String,
    /// Scenes.
    #[serde(default)]
    pub scenes: Vec<AdventureScene>,
}

/// A story or character thread tracked by the oracle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thread {
    /// Id.
    pub id: String,
    /// `"story"` or `"character"`.
    pub r#type: String,
    /// Title.
    pub title: String,
    /// Status (active, resolved, abandoned).
    #[serde(default = "d_active")]
    pub status: String,
    /// Progress counter.
    #[serde(default)]
    pub progress: i32,
}

/// An NPC tracked on the Mythic oracle cast list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleNPC {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Status.
    #[serde(default = "d_active")]
    pub status: String,
    /// Notes.
    #[serde(default)]
    pub notes: String,
    /// Linked entity id.
    #[serde(default)]
    pub entity_id: Option<String>,
}

/// Threshold row for one likelihood/chaos combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FateThresholds {
    /// Yes threshold.
    pub yes_threshold: i32,
    /// Exceptional-yes threshold.
    pub exceptional_yes: i32,
    /// Exceptional-no threshold.
    pub exceptional_no: i32,
}

/// Threshold rows for one likelihood across all chaos values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FateLikelihoodThresholds {
    /// Chaos 1.
    pub chaos_1: FateThresholds,
    /// Chaos 2.
    pub chaos_2: FateThresholds,
    /// Chaos 3.
    pub chaos_3: FateThresholds,
    /// Chaos 4.
    pub chaos_4: FateThresholds,
    /// Chaos 5.
    pub chaos_5: FateThresholds,
    /// Chaos 6.
    pub chaos_6: FateThresholds,
    /// Chaos 7.
    pub chaos_7: FateThresholds,
    /// Chaos 8.
    pub chaos_8: FateThresholds,
    /// Chaos 9.
    pub chaos_9: FateThresholds,
}

impl FateLikelihoodThresholds {
    /// Thresholds for a specific chaos factor (1–9).
    pub fn for_chaos(&self, chaos_factor: i32) -> Option<FateThresholds> {
        match chaos_factor {
            1 => Some(self.chaos_1),
            2 => Some(self.chaos_2),
            3 => Some(self.chaos_3),
            4 => Some(self.chaos_4),
            5 => Some(self.chaos_5),
            6 => Some(self.chaos_6),
            7 => Some(self.chaos_7),
            8 => Some(self.chaos_8),
            9 => Some(self.chaos_9),
            _ => None,
        }
    }
}

/// All fate-chart likelihood rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FateLikelihoodCatalog {
    /// Impossible.
    pub impossible: FateLikelihoodThresholds,
    /// No way.
    pub no_way: FateLikelihoodThresholds,
    /// Very unlikely.
    pub very_unlikely: FateLikelihoodThresholds,
    /// Unlikely.
    pub unlikely: FateLikelihoodThresholds,
    /// Even odds.
    pub even_odds: FateLikelihoodThresholds,
    /// Somewhat likely.
    pub somewhat_likely: FateLikelihoodThresholds,
    /// Likely.
    pub likely: FateLikelihoodThresholds,
    /// Very likely.
    pub very_likely: FateLikelihoodThresholds,
    /// Certain.
    pub certain: FateLikelihoodThresholds,
}

impl FateLikelihoodCatalog {
    /// Return one likelihood row by its canonical string key.
    pub fn get_likelihood(&self, name: &str) -> Option<FateLikelihoodThresholds> {
        match name {
            "impossible" => Some(self.impossible),
            "no_way" => Some(self.no_way),
            "very_unlikely" => Some(self.very_unlikely),
            "unlikely" => Some(self.unlikely),
            "even_odds" => Some(self.even_odds),
            "somewhat_likely" => Some(self.somewhat_likely),
            "likely" => Some(self.likely),
            "very_likely" => Some(self.very_likely),
            "certain" => Some(self.certain),
            _ => None,
        }
    }
}

/// Full Mythic fate-chart YAML document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FateChartDocument {
    /// Likelihood rows.
    pub likelihoods: FateLikelihoodCatalog,
}

fn d_one() -> i32 {
    1
}
fn d_hundred() -> i32 {
    100
}

/// One ranged entry in an oracle event table (preserves unknown keys).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleTableEntry {
    /// Range minimum (default 1).
    #[serde(default = "d_one")]
    pub min: i32,
    /// Range maximum (default 100).
    #[serde(default = "d_hundred")]
    pub max: i32,
    /// Focus result.
    #[serde(default)]
    pub focus: Option<String>,
    /// Action result.
    #[serde(default)]
    pub action: Option<String>,
    /// Subject result.
    #[serde(default)]
    pub subject: Option<String>,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

/// One oracle event table document (preserves unknown keys).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OracleEventTable {
    /// Entries.
    #[serde(default)]
    pub entries: Vec<OracleTableEntry>,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

/// One Adventure Crafter theme definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventureCrafterTheme {
    /// Theme name.
    pub name: String,
    /// Element words.
    #[serde(default)]
    pub elements: Vec<String>,
}

fn d_narration_template() -> String {
    "[{theme}] {character}. {plot}. (Focus: {element})".to_string()
}

/// Adventure Crafter theme table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventureCrafterThemesDocument {
    /// Themes.
    #[serde(default)]
    pub themes: Vec<AdventureCrafterTheme>,
    /// Scene narration template (content, not code).
    #[serde(default = "d_narration_template")]
    pub narration_template: String,
}

impl Default for AdventureCrafterThemesDocument {
    fn default() -> Self {
        AdventureCrafterThemesDocument {
            themes: Vec::new(),
            narration_template: d_narration_template(),
        }
    }
}

/// Simple string-entry table used by Adventure Crafter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdventureCrafterEntriesDocument {
    /// Entries.
    #[serde(default)]
    pub entries: Vec<String>,
}

/// Typed plotline record loaded from persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlotlineRecord {
    /// Id.
    pub id: String,
    /// Title.
    pub title: String,
    /// Theme.
    pub theme: String,
    /// Status.
    #[serde(default = "d_active")]
    pub status: String,
    /// Scenes.
    #[serde(default)]
    pub scenes: Vec<AdventureScene>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&FateResult::ExceptionalYes).unwrap(),
            "\"exceptional_yes\""
        );
        assert_eq!(
            serde_json::to_string(&Likelihood::VeryUnlikely).unwrap(),
            "\"very_unlikely\""
        );
        assert_eq!(
            serde_json::to_string(&SceneModification::Interrupt).unwrap(),
            "\"interrupt\""
        );
    }

    #[test]
    fn oracle_table_entry_defaults_and_extra() {
        let e: OracleTableEntry = serde_json::from_str(r#"{"focus":"npc","weird":1}"#).unwrap();
        assert_eq!(e.min, 1);
        assert_eq!(e.max, 100);
        assert_eq!(e.extra.get("weird"), Some(&serde_json::Value::from(1)));
    }

    #[test]
    fn themes_doc_default_template() {
        let d = AdventureCrafterThemesDocument::default();
        assert!(d.narration_template.contains("{theme}"));
    }
}
