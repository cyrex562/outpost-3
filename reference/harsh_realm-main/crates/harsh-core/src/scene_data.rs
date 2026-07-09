//! Typed scene and random-table record models.
//!
//! Ported from `src/harsh_realm/models/scene_data.py`. Legacy dict-access shims
//! are dropped in favour of direct field access.

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};

fn d_one_f64() -> f64 {
    1.0
}

/// One generated town-grid cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TownCell {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Terrain id.
    pub terrain: String,
    /// Feature ids.
    #[serde(default)]
    pub features: Vec<String>,
    /// Arbitrary data.
    #[serde(default)]
    pub data: JsonObject,
}

/// One establishment entry in settlement metadata (preserves unknown keys).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementBuilding {
    /// Building id.
    #[serde(default)]
    pub id: Option<String>,
    /// Name.
    pub name: String,
    /// Building type.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Column.
    #[serde(default)]
    pub q: Option<i32>,
    /// Row.
    #[serde(default)]
    pub r: Option<i32>,
    /// Width.
    #[serde(default)]
    pub w: Option<i32>,
    /// Height.
    #[serde(default)]
    pub h: Option<i32>,
    /// Occupant NPC id.
    #[serde(default)]
    pub npc_id: Option<String>,
    /// Occupant NPC name.
    #[serde(default)]
    pub npc_name: Option<String>,
    /// Interior building id.
    #[serde(default)]
    pub building_id: Option<String>,
    /// Unknown extra keys (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

/// One dungeon room definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DungeonRoom {
    /// Room id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Room type.
    pub r#type: String,
    /// X position.
    #[serde(default)]
    pub x: Option<i32>,
    /// Y position.
    #[serde(default)]
    pub y: Option<i32>,
    /// Width.
    #[serde(default)]
    pub w: Option<i32>,
    /// Height.
    #[serde(default)]
    pub h: Option<i32>,
    /// Center column.
    #[serde(default)]
    pub center_q: Option<i32>,
    /// Center row.
    #[serde(default)]
    pub center_r: Option<i32>,
    /// Editor position payload.
    #[serde(default)]
    pub editor_pos: Option<JsonObject>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Loot payloads.
    #[serde(default)]
    pub loot: Vec<JsonObject>,
    /// Hidden loot payloads (`{item, difficulty}`).
    #[serde(default)]
    pub hidden_loot: Vec<JsonObject>,
    /// Trap payloads.
    #[serde(default)]
    pub traps: Vec<JsonObject>,
    /// Encounter payload.
    #[serde(default)]
    pub encounter: Option<JsonObject>,
    /// Arbitrary data.
    #[serde(default)]
    pub data: JsonObject,
}

/// One generated Adventure Crafter scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdventureScene {
    /// Theme.
    pub theme: String,
    /// Element.
    pub element: String,
    /// Character focus.
    pub character: String,
    /// Plot.
    pub plot: String,
    /// Narration text.
    pub narration: String,
}

/// One directional connection between dungeon rooms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DungeonConnection {
    /// Source room id (JSON key `from`).
    #[serde(rename = "from")]
    pub from_room: String,
    /// Destination room id (JSON key `to`).
    #[serde(rename = "to")]
    pub to_room: String,
    /// Direction label.
    #[serde(default)]
    pub direction: String,
}

/// One weighted random-table entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomTableEntry {
    /// Selection weight.
    #[serde(default = "d_one_f64")]
    pub weight: f64,
    /// Range expression, e.g. `"1-10"` or `"42"`.
    #[serde(default)]
    pub range: Option<String>,
    /// Result payload.
    pub result: JsonValue,
}

impl RandomTableEntry {
    /// Parse `"1-10"` or `"42"` into `(1, 10)` or `(42, 42)`.
    pub fn range_min_max(&self) -> Option<(i32, i32)> {
        let s = self.range.as_ref()?.trim();
        if let Some((lo, hi)) = s.split_once('-') {
            match (lo.trim().parse::<i32>(), hi.trim().parse::<i32>()) {
                (Ok(a), Ok(b)) => Some((a, b)),
                _ => None,
            }
        } else {
            s.parse::<i32>().ok().map(|v| (v, v))
        }
    }
}

/// One decoded random-table record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomTableRow {
    /// Table id.
    pub id: String,
    /// Category.
    pub category: String,
    /// Name.
    pub name: String,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Entries.
    #[serde(default)]
    pub entries: Vec<RandomTableEntry>,
    /// Source.
    #[serde(default)]
    pub source: String,
}

/// One YAML generator step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratorStep {
    /// Roll expression.
    pub roll: String,
    /// Assignment target.
    pub assign: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_uses_from_to_aliases() {
        let c: DungeonConnection =
            serde_json::from_str(r#"{"from":"a","to":"b","direction":"north"}"#).unwrap();
        assert_eq!(c.from_room, "a");
        assert_eq!(c.to_room, "b");
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"from\":\"a\""));
    }

    #[test]
    fn range_min_max_parses() {
        let mk = |range: Option<&str>| RandomTableEntry {
            weight: 1.0,
            range: range.map(str::to_string),
            result: JsonValue::Null,
        };
        assert_eq!(mk(Some("1-10")).range_min_max(), Some((1, 10)));
        assert_eq!(mk(Some("42")).range_min_max(), Some((42, 42)));
        assert_eq!(mk(Some("bad")).range_min_max(), None);
        assert_eq!(mk(None).range_min_max(), None);
    }

    #[test]
    fn settlement_building_preserves_extra() {
        let b: SettlementBuilding =
            serde_json::from_str(r#"{"name":"Inn","weird":true}"#).unwrap();
        assert_eq!(b.extra.get("weird"), Some(&JsonValue::Bool(true)));
    }
}
