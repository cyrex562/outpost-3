//! Cell-editor API models. Ported from `models/editor_api/cells.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// A grid coordinate used in bulk cell updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellCoordinate {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
}

/// Editable cell fields (all optional for partial updates).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CellUpdateBody {
    /// Terrain id.
    #[serde(default)]
    pub terrain: Option<String>,
    /// Feature ids.
    #[serde(default)]
    pub features: Option<Vec<String>>,
    /// Explored flag.
    #[serde(default)]
    pub explored: Option<bool>,
    /// Description.
    #[serde(default)]
    pub description: Option<String>,
    /// Faction id.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// Data payload.
    #[serde(default)]
    pub data: Option<JsonObject>,
}

/// Shared cell updates applied to many coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BulkCellUpdateBody {
    /// Target coordinates.
    #[serde(default)]
    pub cells: Vec<CellCoordinate>,
    /// Terrain id.
    #[serde(default)]
    pub terrain: Option<String>,
    /// Faction id.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// Explored flag.
    #[serde(default)]
    pub explored: Option<bool>,
}
