//! Searchable loot sources on a cell (HR-786).
//!
//! A `LootSource` is any lootable thing on a cell the player can `search` to
//! reveal, then `take` from: a `ground` find, a placed `container`
//! (crate/chest), or a `corpse` of a defeated enemy/NPC. Sources are stored as
//! JSON under `cell.data.loot_sources`, mirroring `death_markers` (no schema
//! migration). This module owns the model and the pure read/write helpers over
//! a cell's `data` object; the search/take flow lives in the exploration scene.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::runtime::JsonObject;

/// Ground find created by a successful `search`.
pub const KIND_GROUND: &str = "ground";
/// A placed crate/chest, revealed by searching it.
pub const KIND_CONTAINER: &str = "container";
/// A defeated enemy/NPC's body, revealed by searching it.
pub const KIND_CORPSE: &str = "corpse";

/// The JSON key under `cell.data` holding the source array.
pub const CELL_KEY: &str = "loot_sources";

/// A searchable loot source on a cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LootSource {
    /// Unique id within the cell (stable across searches).
    pub id: String,
    /// One of `KIND_GROUND` / `KIND_CONTAINER` / `KIND_CORPSE`.
    pub kind: String,
    /// Display name, e.g. `"weathered crate"` or `"goblin's corpse"`.
    pub name: String,
    /// Item payloads still in the source.
    #[serde(default)]
    pub contents: Vec<JsonObject>,
    /// Currency still in the source.
    #[serde(default)]
    pub gold: i32,
    /// Skill-check DC to reveal the source (0 = trivially searched).
    #[serde(default)]
    pub difficulty: i32,
    /// Whether the source has been revealed by a successful search.
    #[serde(default)]
    pub searched: bool,
    /// Whether the source has been fully looted / came up empty.
    #[serde(default)]
    pub empty: bool,
}

impl LootSource {
    /// A source with no items and no gold has nothing left to take.
    pub fn has_loot(&self) -> bool {
        !self.contents.is_empty() || self.gold > 0
    }

    /// The representative value of the pile (top item value or gold) — used by
    /// the frontend rarity tier. Returns 0 for a value-less source.
    pub fn representative_value(&self) -> i32 {
        let mut best = self.gold.max(0);
        for item in &self.contents {
            if let Some(v) = item.get("value").and_then(|v| v.as_i64()) {
                best = best.max(v as i32);
            }
        }
        best
    }
}

/// Read the `loot_sources` array out of a cell's `data` object.
pub fn read_sources(cell_data: &JsonObject) -> Vec<LootSource> {
    cell_data
        .get(CELL_KEY)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| serde_json::from_value(v.clone()).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Write the `loot_sources` array back into a cell's `data` object. The key is
/// removed entirely when there are no sources, so an emptied cell stays clean.
pub fn write_sources(cell_data: &mut JsonObject, sources: &[LootSource]) {
    if sources.is_empty() {
        cell_data.remove(CELL_KEY);
        return;
    }
    let arr: Vec<JsonValue> = sources
        .iter()
        .map(|s| serde_json::to_value(s).unwrap_or(JsonValue::Null))
        .collect();
    cell_data.insert(CELL_KEY.to_string(), JsonValue::Array(arr));
}

/// Add a source to a cell's `data` object.
pub fn push_source(cell_data: &mut JsonObject, source: LootSource) {
    let mut sources = read_sources(cell_data);
    sources.push(source);
    write_sources(cell_data, &sources);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(name: &str, value: i64) -> JsonObject {
        serde_json::from_value(serde_json::json!({
            "name": name, "type": "misc", "value": value
        }))
        .unwrap()
    }

    fn sample() -> LootSource {
        LootSource {
            id: "crate_1".into(),
            kind: KIND_CONTAINER.into(),
            name: "weathered crate".into(),
            contents: vec![item("Rope", 5), item("Data Chip", 150)],
            gold: 10,
            difficulty: 8,
            searched: false,
            empty: false,
        }
    }

    #[test]
    fn round_trips_through_cell_data() {
        let mut data = JsonObject::new();
        push_source(&mut data, sample());
        let back = read_sources(&data);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0], sample());
    }

    #[test]
    fn write_removes_key_when_empty() {
        let mut data = JsonObject::new();
        push_source(&mut data, sample());
        assert!(data.contains_key(CELL_KEY));
        write_sources(&mut data, &[]);
        assert!(!data.contains_key(CELL_KEY), "empty source list clears the key");
    }

    #[test]
    fn representative_value_is_top_item_or_gold() {
        let s = sample();
        assert_eq!(s.representative_value(), 150);
        let coins = LootSource {
            contents: vec![],
            gold: 40,
            ..sample()
        };
        assert_eq!(coins.representative_value(), 40);
    }

    #[test]
    fn has_loot_reflects_contents_and_gold() {
        assert!(sample().has_loot());
        let empty = LootSource {
            contents: vec![],
            gold: 0,
            ..sample()
        };
        assert!(!empty.has_loot());
    }

    #[test]
    fn missing_or_malformed_key_reads_as_empty() {
        assert!(read_sources(&JsonObject::new()).is_empty());
        let mut data = JsonObject::new();
        data.insert("loot_sources".into(), JsonValue::String("nope".into()));
        assert!(read_sources(&data).is_empty());
    }
}
