//! Typed runtime payload models for discovery, encounters, and loot.
//!
//! Ported from `src/harsh_realm/models/engine_runtime.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

fn d_environmental() -> String {
    "environmental".to_string()
}
fn d_unknown_encounter() -> String {
    "Unknown Encounter".to_string()
}
fn d_misc() -> String {
    "misc".to_string()
}
fn d_one() -> i32 {
    1
}
fn d_eight() -> i32 {
    8
}

/// Typed discovery-table result payload (preserves unknown keys).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiscoveryFindRecord {
    /// Find type.
    #[serde(default)]
    pub r#type: String,
    /// Find name.
    #[serde(default)]
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Minimum difficulty to find.
    #[serde(default)]
    pub min_difficulty: i32,
    /// Item payloads.
    #[serde(default)]
    pub items: Vec<JsonObject>,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

/// Typed encounter-table result payload (preserves unknown keys).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncounterRecord {
    /// Encounter type (default `"environmental"`).
    #[serde(default = "d_environmental")]
    pub r#type: String,
    /// Encounter name.
    #[serde(default = "d_unknown_encounter")]
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl Default for EncounterRecord {
    fn default() -> Self {
        EncounterRecord {
            r#type: d_environmental(),
            name: d_unknown_encounter(),
            description: String::new(),
            extra: JsonObject::new(),
        }
    }
}

/// Structured loot item payload loaded from YAML (preserves unknown keys).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootItemData {
    /// Loot item type (default `"misc"`).
    #[serde(default = "d_misc")]
    pub r#type: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Value.
    #[serde(default)]
    pub value: i32,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl Default for LootItemData {
    fn default() -> Self {
        LootItemData {
            r#type: d_misc(),
            name: String::new(),
            value: 0,
            extra: JsonObject::new(),
        }
    }
}

/// One weighted entry in a loot table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootTableEntry {
    /// Selection weight (default 1).
    #[serde(default = "d_one")]
    pub weight: i32,
    /// Result key.
    #[serde(default)]
    pub result: String,
    /// Loot payload.
    #[serde(default)]
    pub data: LootItemData,
}

/// One authored loot-table YAML document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LootTableDocument {
    /// Table id.
    #[serde(default)]
    pub table_id: String,
    /// Document type.
    #[serde(default)]
    pub r#type: String,
    /// Entries.
    #[serde(default)]
    pub entries: Vec<LootTableEntry>,
}

/// Harvestable material made available by a defeated creature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarvestableRecord {
    /// Source creature name.
    pub creature_name: String,
    /// Material yielded.
    pub material: String,
    /// Skill required to harvest, if any.
    #[serde(default)]
    pub skill: Option<String>,
    /// Difficulty (default 8).
    #[serde(default = "d_eight")]
    pub difficulty: i32,
}

/// Pending Veteran's Luck hit data kept in combat state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingVeteranLuckRecord {
    /// Attacker name.
    pub attacker_name: String,
    /// Damage that would be dealt.
    pub damage: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encounter_defaults() {
        let e = EncounterRecord::default();
        assert_eq!(e.r#type, "environmental");
        assert_eq!(e.name, "Unknown Encounter");
    }

    #[test]
    fn loot_entry_default_weight_and_data() {
        let entry: LootTableEntry = serde_json::from_str("{}").unwrap();
        assert_eq!(entry.weight, 1);
        assert_eq!(entry.data.r#type, "misc");
    }

    #[test]
    fn harvestable_default_difficulty() {
        let h: HarvestableRecord =
            serde_json::from_str(r#"{"creature_name":"wolf","material":"pelt"}"#).unwrap();
        assert_eq!(h.difficulty, 8);
        assert!(h.skill.is_none());
    }

    #[test]
    fn discovery_preserves_unknown_and_default_value() {
        let d: DiscoveryFindRecord =
            serde_json::from_str(r#"{"name":"relic","odd":1}"#).unwrap();
        assert_eq!(d.extra.get("odd"), Some(&serde_json::Value::from(1)));
    }
}
