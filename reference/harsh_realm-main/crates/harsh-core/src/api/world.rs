//! World-management API models. Ported from `models/api/world.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Pack binding returned by world management endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldPackSummary {
    /// Pack id.
    pub pack_id: String,
    /// Version.
    pub version: String,
    /// Load order.
    pub load_order: i32,
}

/// Reference to one world database file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRef {
    /// World name.
    pub name: String,
    /// File path.
    pub file: String,
}

/// Result payload for world creation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldCreateResult {
    /// Embedded world reference.
    #[serde(flatten)]
    pub world: WorldRef,
    /// Cell count.
    pub cell_count: i32,
    /// Terrain counts payload.
    #[serde(default)]
    pub terrain_counts: JsonObject,
    /// Open edge, if any.
    #[serde(default)]
    pub open_edge: Option<String>,
    /// Width.
    pub width: i32,
    /// Height.
    pub height: i32,
    /// Pack bindings.
    #[serde(default)]
    pub packs: Vec<WorldPackSummary>,
}

/// Result payload for world snapshot creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotResult {
    /// Snapshot file path.
    pub snapshot_file: String,
}
