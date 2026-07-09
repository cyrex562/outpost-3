//! GM command request/response API models. Ported from `models/api/gm_commands.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::{InventoryItemRecord, JsonObject};

fn d_npc() -> String {
    "npc".to_string()
}

/// Request body for the GM teleport command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMTeleportBody {
    /// Entity id.
    pub entity_id: String,
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
}

/// Request body for the GM spawn-entity command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GMSpawnBody {
    /// Entity type (default `"npc"`).
    #[serde(default = "d_npc")]
    pub entity_type: String,
    /// Name.
    pub name: String,
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Optional data payload.
    #[serde(default)]
    pub data: Option<JsonObject>,
}

/// Request body for the GM give-item command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GMGiveItemBody {
    /// Entity id.
    pub entity_id: String,
    /// Item to give.
    pub item: InventoryItemRecord,
}

/// Request body for the GM set-hp command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetHPBody {
    /// Entity id.
    pub entity_id: String,
    /// HP.
    pub hp: i32,
}

/// Request body for the GM set-gold command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetGoldBody {
    /// Entity id.
    pub entity_id: String,
    /// Gold.
    pub gold: i32,
}

/// Request body for the GM set-xp command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetXPBody {
    /// Entity id.
    pub entity_id: String,
    /// XP.
    pub xp: i32,
}

/// Request body for the GM set-attr command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetAttrBody {
    /// Entity id.
    pub entity_id: String,
    /// Attribute key.
    pub attribute: String,
    /// New value.
    pub value: i32,
}

/// Response payload for the GM teleport command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMTeleportResult {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
}

/// Response payload for the GM spawn command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSpawnResult {
    /// Entity id.
    pub entity_id: String,
    /// Entity type.
    pub entity_type: String,
    /// Name.
    pub name: String,
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
}

/// Response payload for the GM give-item command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GMGiveItemResult {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Item payload.
    pub item: JsonObject,
    /// New equipment count.
    pub equipment_count: i32,
}

/// Response payload for the GM set-hp command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetHPResult {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// HP.
    pub hp: i32,
}

/// Response payload for the GM set-gold command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetGoldResult {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Gold.
    pub gold: i32,
}

/// Response payload for the GM set-xp command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetXPResult {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// XP.
    pub xp: i32,
}

/// Response payload for the GM set-attr command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetAttrResult {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Attribute key.
    pub attribute: String,
    /// Value.
    pub value: i32,
}
