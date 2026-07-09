//! NPC data models for Harsh Realm.
//!
//! Ported from `src/harsh_realm/models/npc.py`. The legacy string-disposition
//! normalization (`"hostile"` -> -3, …) is preserved via a custom deserializer.

use serde::{Deserialize, Deserializer, Serialize};

/// Map a legacy string disposition to its integer value.
pub fn disposition_str_to_int(label: &str) -> i32 {
    match label {
        "hostile" => -3,
        "wary" => -1,
        "neutral" => 0,
        "friendly" => 2,
        _ => 0,
    }
}

/// Accept either an integer disposition or a legacy string label.
fn de_disposition<'de, D>(d: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::String(s) => Ok(disposition_str_to_int(&s)),
        serde_json::Value::Number(n) => Ok(n.as_i64().unwrap_or(0) as i32),
        _ => Ok(0),
    }
}

/// A full UNE personality profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UNEPersonality {
    /// Power level word.
    pub power_level: String,
    /// Descriptor word.
    pub descriptor: String,
    /// Motivation verb.
    pub motivation_verb: String,
    /// Motivation noun.
    pub motivation_noun: String,
    /// Bearing.
    pub bearing: String,
    /// Bearing focus.
    pub bearing_focus: String,
    /// Base disposition.
    #[serde(default)]
    pub base_disposition: i32,
}

/// Data for a non-player character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NPCData {
    /// Occupation/role.
    pub occupation: String,
    /// Personality trait words.
    #[serde(default)]
    pub personality_traits: Vec<String>,
    /// Motivation phrase.
    #[serde(default)]
    pub motivation: String,
    /// Appearance description.
    #[serde(default)]
    pub appearance: String,
    /// Greeting line.
    #[serde(default)]
    pub greeting: String,
    /// Owning faction id.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// Disposition (int; legacy string labels accepted on input).
    #[serde(default, deserialize_with = "de_disposition")]
    pub disposition: i32,
    /// UNE personality profile.
    #[serde(default)]
    pub une_personality: Option<UNEPersonality>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Traits.
    #[serde(default)]
    pub traits: Vec<String>,
}

/// Optional contextual inputs that influence NPC generation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NPCGenerationContext {
    /// Settlement size.
    #[serde(default)]
    pub settlement_size: Option<String>,
    /// Terrain id.
    #[serde(default)]
    pub terrain: Option<String>,
    /// Forced occupation.
    #[serde(default)]
    pub occupation_override: Option<String>,
}

/// Fully generated NPC entity payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedNPC {
    /// Name.
    pub name: String,
    /// Occupation.
    pub occupation: String,
    /// Personality traits.
    #[serde(default)]
    pub personality_traits: Vec<String>,
    /// Motivation.
    #[serde(default)]
    pub motivation: String,
    /// Appearance.
    #[serde(default)]
    pub appearance: String,
    /// Greeting.
    #[serde(default)]
    pub greeting: String,
    /// Faction id.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// Disposition label.
    pub disposition: String,
    /// Building id.
    #[serde(default)]
    pub building_id: Option<String>,
    /// Establishment type.
    #[serde(default)]
    pub establishment_type: Option<String>,
    /// Establishment name.
    #[serde(default)]
    pub establishment_name: Option<String>,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Traits.
    #[serde(default)]
    pub traits: Vec<String>,
}

/// A UNE motivation pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UNEMotivation {
    /// Verb.
    pub verb: String,
    /// Noun.
    pub noun: String,
}

/// A UNE bearing result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UNEBearing {
    /// Bearing.
    pub bearing: String,
    /// Focus.
    pub focus: String,
}

/// One bearing-table entry loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UNEBearingResult {
    /// Bearing.
    pub bearing: String,
    /// Focus.
    pub focus: String,
}

/// One scalar UNE table entry loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UNEScalarEntry {
    /// Result.
    pub result: String,
}

/// One bearing UNE table entry loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UNEBearingEntry {
    /// Result.
    pub result: UNEBearingResult,
}

/// A UNE YAML table with scalar string results.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UNEScalarTable {
    /// Entries.
    #[serde(default)]
    pub entries: Vec<UNEScalarEntry>,
}

/// A UNE YAML table with bearing/focus results.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UNEBearingTable {
    /// Entries.
    #[serde(default)]
    pub entries: Vec<UNEBearingEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disposition_accepts_int_and_legacy_string() {
        let from_str: NPCData =
            serde_json::from_str(r#"{"occupation":"smith","disposition":"hostile"}"#).unwrap();
        assert_eq!(from_str.disposition, -3);
        let from_int: NPCData =
            serde_json::from_str(r#"{"occupation":"smith","disposition":2}"#).unwrap();
        assert_eq!(from_int.disposition, 2);
        let absent: NPCData = serde_json::from_str(r#"{"occupation":"smith"}"#).unwrap();
        assert_eq!(absent.disposition, 0);
    }
}
