//! Typed generation and content-source models.
//!
//! Ported from `src/harsh_realm/models/generation.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};
use crate::scene_data::{
    DungeonConnection, DungeonRoom, GeneratorStep, RandomTableEntry, SettlementBuilding,
};

/// Terrain id -> base weight.
pub type TerrainWeightMap = BTreeMap<String, f64>;
/// Terrain id -> (neighbour terrain id -> weight modifier).
pub type AdjacencyWeightMap = BTreeMap<String, BTreeMap<String, f64>>;
/// Terrain id -> count.
pub type TerrainCountMap = BTreeMap<String, i32>;

fn d_one() -> i32 {
    1
}

/// One item entry in a shop-tier YAML file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopInventoryEntry {
    /// Canonical item id.
    pub item_id: String,
    /// Quantity (default 1).
    #[serde(default = "d_one")]
    pub quantity: i32,
}

/// One named tier in a shop inventory YAML file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopTier {
    /// Tier name.
    pub name: String,
    /// Items.
    #[serde(default)]
    pub items: Vec<ShopInventoryEntry>,
}

/// Complete tiered shop inventory definition.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ShopCatalog {
    /// Small tier.
    #[serde(default)]
    pub small: Option<ShopTier>,
    /// Medium tier.
    #[serde(default)]
    pub medium: Option<ShopTier>,
    /// Large tier.
    #[serde(default)]
    pub large: Option<ShopTier>,
}

impl ShopCatalog {
    /// Return one tier by name.
    pub fn get_tier(&self, tier: &str) -> Option<&ShopTier> {
        match tier {
            "small" => self.small.as_ref(),
            "medium" => self.medium.as_ref(),
            "large" => self.large.as_ref(),
            _ => None,
        }
    }
}

/// Terrain base weights and adjacency modifiers.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TerrainWeightsConfig {
    /// Base weights.
    #[serde(default)]
    pub base_weights: TerrainWeightMap,
    /// Adjacency modifiers.
    #[serde(default)]
    pub adjacency_modifiers: AdjacencyWeightMap,
}

/// Typed YAML document for simple name-list content files.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NameListDocument {
    /// Names.
    #[serde(default)]
    pub names: Vec<String>,
}

/// Summary of one generated world region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldGenerationSummary {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Cell count.
    pub cell_count: i32,
    /// Terrain counts.
    #[serde(default)]
    pub terrain_counts: TerrainCountMap,
    /// Open edge.
    pub open_edge: String,
}

/// One table definition loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomTableDefinition {
    /// Table id.
    pub id: String,
    /// Category.
    #[serde(default)]
    pub category: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Entries.
    #[serde(default)]
    pub entries: Vec<RandomTableEntry>,
}

/// One multi-step generator definition loaded from YAML.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GeneratorDefinition {
    /// Generator id.
    #[serde(default)]
    pub id: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Steps.
    #[serde(default)]
    pub steps: Vec<GeneratorStep>,
}

/// Typed assignment map returned by a generator execution.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GeneratorExecutionResult {
    /// Assignments.
    #[serde(default)]
    pub assignments: BTreeMap<String, JsonValue>,
}

/// A single cell in a square grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquareCell {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Terrain id.
    pub terrain: String,
    /// Owning room id.
    #[serde(default)]
    pub room_id: Option<String>,
    /// Feature ids.
    #[serde(default)]
    pub features: Vec<String>,
    /// Arbitrary data.
    #[serde(default)]
    pub data: JsonObject,
}

/// Result of dungeon generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DungeonResult {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Cells.
    pub cells: Vec<SquareCell>,
    /// Rooms.
    pub rooms: Vec<DungeonRoom>,
    /// Connections.
    pub connections: Vec<DungeonConnection>,
    /// Entrance column.
    pub entrance_q: i32,
    /// Entrance row.
    pub entrance_r: i32,
}

/// Result of town generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TownResult {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Cells.
    pub cells: Vec<SquareCell>,
    /// Buildings.
    pub buildings: Vec<SettlementBuilding>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_catalog_get_tier() {
        let cat: ShopCatalog = serde_json::from_str(
            r#"{"small":{"name":"basic","items":[{"item_id":"weapon.club"}]}}"#,
        )
        .unwrap();
        let tier = cat.get_tier("small").unwrap();
        assert_eq!(tier.name, "basic");
        assert_eq!(tier.items[0].quantity, 1);
        assert!(cat.get_tier("large").is_none());
    }
}
