//! World-map summary API models. Ported from `models/api/map.py`.

use serde::{Deserialize, Serialize};

/// One lightweight cell entry from `/api/worlds/current/map`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMapCellSummary {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Terrain id.
    pub terrain: String,
    /// Feature ids.
    #[serde(default)]
    pub features: Vec<String>,
    /// Whether explored.
    pub explored: bool,
}

/// Response payload for `/api/worlds/current/map`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldMapSummary {
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Grid topology.
    pub grid_type: String,
    /// Cells.
    #[serde(default)]
    pub cells: Vec<WorldMapCellSummary>,
}
