//! Scene-context payloads. Ported from `payloads/contexts.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Typed handoff state for entering the social scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialSceneContext {
    /// NPC entity id.
    pub npc_entity_id: String,
    /// NPC display name.
    pub npc_name: String,
    /// NPC payload.
    pub npc_data: JsonObject,
}

/// Typed handoff state for entering the shopping scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingSceneContext {
    /// Settlement payload.
    pub settlement_data: JsonObject,
    /// Building type.
    #[serde(default)]
    pub building_type: Option<String>,
    /// Building tier.
    #[serde(default)]
    pub building_tier: Option<String>,
    /// Building name.
    #[serde(default)]
    pub building_name: Option<String>,
}

/// Typed handoff state for entering the dungeon scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DungeonSceneContext {
    /// Dungeon id.
    pub dungeon_id: String,
    /// Rooms.
    pub rooms: Vec<JsonObject>,
    /// Connections.
    pub connections: Vec<JsonObject>,
    /// Entry column.
    pub entry_q: i32,
    /// Entry row.
    pub entry_r: i32,
}

/// Payload for `exploration.town_entry_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TownEntryRequested {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Settlement payload.
    pub settlement_data: JsonObject,
    /// Town cells.
    pub town_cells: Vec<JsonObject>,
}

/// Typed handoff state for entering the town scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TownSceneContext {
    /// Settlement payload.
    pub settlement_data: JsonObject,
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Town cells.
    pub town_cells: Vec<JsonObject>,
}
