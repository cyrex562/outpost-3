//! Dungeon-editor API models. Ported from `models/editor_api/dungeons.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;
use crate::scene_data::{DungeonConnection, DungeonRoom};

/// Editable dungeon fields (all optional for partial updates).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DungeonUpdateRequest {
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Rooms.
    #[serde(default)]
    pub rooms: Option<Vec<DungeonRoom>>,
    /// Connections.
    #[serde(default)]
    pub connections: Option<Vec<DungeonConnection>>,
    /// Data payload.
    #[serde(default)]
    pub data: Option<JsonObject>,
}
