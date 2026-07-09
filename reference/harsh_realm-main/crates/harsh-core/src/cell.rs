//! Cell data models and terrain registry for Harsh Realm.
//!
//! Ported from `src/harsh_realm/models/cell.py`. The Python `CellDataPayload`
//! RootModel sugar is replaced by a plain `JsonObject` `data` field.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};
use crate::scene_data::{SettlementBuilding, TownCell};

/// Definition of a terrain type loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainType {
    /// Terrain id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether the terrain is passable.
    pub passable: bool,
    /// Whether the terrain blocks line of sight.
    pub blocks_vision: bool,
    /// Description pool id.
    #[serde(default)]
    pub description_pool: String,
}

/// Mutable representation of a single grid cell from the database.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellData {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Terrain id.
    pub terrain: String,
    /// Feature ids.
    #[serde(default)]
    pub features: Vec<String>,
    /// Whether the cell is explored.
    #[serde(default)]
    pub explored: bool,
    /// Optional description override.
    #[serde(default)]
    pub description: Option<String>,
    /// Controlling faction id.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// JSON payload stored in the cell's `data` column.
    #[serde(default)]
    pub data: JsonObject,
}

/// Typed settlement metadata attached to one world cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellSettlementState {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Name.
    pub name: String,
    /// Size.
    pub size: String,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Settlement id.
    pub settlement_id: String,
    /// Whether this is the starting settlement.
    #[serde(default)]
    pub starting: bool,
    /// Establishments.
    #[serde(default)]
    pub establishments: Vec<SettlementBuilding>,
    /// Resident NPC ids.
    #[serde(default)]
    pub resident_npc_ids: Vec<String>,
    /// Town-grid cells.
    #[serde(default)]
    pub town_cells: Vec<TownCell>,
}

impl CellSettlementState {
    /// Return the legacy-compatible settlement payload shape.
    pub fn to_payload(&self) -> JsonObject {
        let mut out = JsonObject::new();
        out.insert("name".into(), JsonValue::from(self.name.clone()));
        out.insert("size".into(), JsonValue::from(self.size.clone()));
        out.insert("description".into(), JsonValue::from(self.description.clone()));
        out.insert(
            "settlement_id".into(),
            JsonValue::from(self.settlement_id.clone()),
        );
        out.insert("starting".into(), JsonValue::from(self.starting));
        let establishments: Vec<JsonValue> = self
            .establishments
            .iter()
            .map(|e| serde_json::to_value(e).unwrap_or(JsonValue::Null))
            .collect();
        out.insert("establishments".into(), JsonValue::from(establishments));
        out.insert(
            "resident_npc_ids".into(),
            JsonValue::from(self.resident_npc_ids.clone()),
        );
        out
    }
}

/// Typed search cooldown state for one cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSearchState {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Tick of the last search.
    pub last_searched_tick: i32,
}

/// Typed loot marker persisted for one cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellDeathMarker {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Marker index.
    pub marker_index: i32,
    /// Item payloads.
    #[serde(default)]
    pub items: Vec<JsonObject>,
}

/// A terrain YAML document (`{terrains: [...]}`).
#[derive(Debug, Clone, Deserialize)]
struct TerrainDocument {
    #[serde(default)]
    terrains: Vec<TerrainType>,
}

/// Registry of terrain type definitions loaded from YAML.
#[derive(Debug, Clone, Default)]
pub struct TerrainRegistry {
    terrains: BTreeMap<String, TerrainType>,
}

impl TerrainRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        TerrainRegistry::default()
    }

    /// Build a registry directly from terrain types, keyed by id.
    pub fn from_types(types: Vec<TerrainType>) -> Self {
        let mut terrains = BTreeMap::new();
        for terrain in types {
            terrains.insert(terrain.id.clone(), terrain);
        }
        TerrainRegistry { terrains }
    }

    /// Load terrain definitions from a YAML file.
    ///
    /// When `clear` is true the registry is emptied first. Returns an error if
    /// the file is missing or cannot be parsed.
    pub fn load(&mut self, path: &Path, clear: bool) -> Result<(), String> {
        if !path.exists() {
            return Err(format!("Terrain definitions file not found: {path:?}"));
        }
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let doc: TerrainDocument = serde_yaml::from_str(&text).map_err(|e| e.to_string())?;
        if clear {
            self.terrains.clear();
        }
        for terrain in doc.terrains {
            self.terrains.insert(terrain.id.clone(), terrain);
        }
        Ok(())
    }

    /// Load `terrain.yaml` (then `terrain_square.yaml`, merged) from a directory.
    pub fn load_all(&mut self, data_dir: &Path) -> Result<(), String> {
        let base = data_dir.join("terrain.yaml");
        if base.exists() {
            self.load(&base, true)?;
        }
        let square = data_dir.join("terrain_square.yaml");
        if square.exists() {
            self.load(&square, false)?;
        }
        Ok(())
    }

    /// Retrieve a terrain type by id.
    pub fn get(&self, terrain_id: &str) -> Option<&TerrainType> {
        self.terrains.get(terrain_id)
    }

    /// All registered terrain types.
    pub fn all(&self) -> Vec<&TerrainType> {
        self.terrains.values().collect()
    }

    /// Only terrain types that are passable.
    pub fn passable_types(&self) -> Vec<&TerrainType> {
        self.terrains.values().filter(|t| t.passable).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_defaults() {
        let c: CellData = serde_json::from_str(r#"{"q":1,"r":2,"terrain":"plains"}"#).unwrap();
        assert!(!c.explored);
        assert!(c.features.is_empty());
        assert!(c.data.is_empty());
    }

    #[test]
    fn settlement_payload_shape() {
        let s = CellSettlementState {
            q: 0,
            r: 0,
            name: "Ashford".into(),
            size: "village".into(),
            description: "".into(),
            settlement_id: "s1".into(),
            starting: true,
            establishments: vec![],
            resident_npc_ids: vec!["n1".into()],
            town_cells: vec![],
        };
        let p = s.to_payload();
        assert_eq!(p["name"], JsonValue::from("Ashford"));
        assert_eq!(p["starting"], JsonValue::from(true));
        assert!(p["establishments"].is_array());
    }

    #[test]
    fn terrain_registry_loads_and_filters() {
        let dir = std::env::temp_dir().join(format!("hr_terrain_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("terrain.yaml");
        std::fs::write(
            &path,
            "terrains:\n  - id: plains\n    name: Plains\n    passable: true\n    blocks_vision: false\n  - id: wall\n    name: Wall\n    passable: false\n    blocks_vision: true\n",
        )
        .unwrap();
        let mut reg = TerrainRegistry::new();
        reg.load(&path, true).unwrap();
        assert_eq!(reg.get("plains").unwrap().name, "Plains");
        assert_eq!(reg.all().len(), 2);
        assert_eq!(reg.passable_types().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
