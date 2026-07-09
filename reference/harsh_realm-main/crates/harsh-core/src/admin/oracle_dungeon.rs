//! Oracle, entity, and dungeon editor admin models. Ported from
//! `models/admin/oracle_dungeon.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Oracle thread editor row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleThreadRecord {
    /// Id.
    pub id: String,
    /// Type.
    pub r#type: String,
    /// Title.
    pub title: String,
    /// Status.
    pub status: String,
    /// Progress.
    #[serde(default)]
    pub progress: i32,
}

/// Oracle NPC editor row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleNpcRecord {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Status.
    pub status: String,
    /// Notes.
    #[serde(default)]
    pub notes: String,
    /// Linked entity id.
    #[serde(default)]
    pub entity_id: Option<String>,
}

/// Current oracle runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OracleStateRecord {
    /// Chaos factor.
    pub chaos_factor: i32,
}

/// Compact entity listing grouped by world location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityLocationRecord {
    /// Id.
    pub id: String,
    /// Entity type.
    pub entity_type: String,
    /// Name.
    pub name: String,
    /// Column.
    #[serde(default)]
    pub q: Option<i32>,
    /// Row.
    #[serde(default)]
    pub r: Option<i32>,
}

/// Compact dungeon list entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DungeonSummary {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Room count.
    #[serde(default)]
    pub room_count: i32,
}

/// Full dungeon editor payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DungeonRecord {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Rooms.
    #[serde(default)]
    pub rooms: Vec<JsonObject>,
    /// Connections.
    #[serde(default)]
    pub connections: Vec<JsonObject>,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}

/// Dungeon creation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DungeonCreated {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
}

fn d_true() -> bool {
    true
}

/// Typed editor-facing entity row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorEntityRecord {
    /// Id.
    pub id: String,
    /// Entity type.
    pub entity_type: String,
    /// Name.
    pub name: String,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Alive flag (default true).
    #[serde(default = "d_true")]
    pub alive: bool,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}

/// Entity creation response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityCreated {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
}

/// Preview of recalculated derived character stats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterPreviewResult {
    /// Attribute modifiers.
    #[serde(default)]
    pub attr_mods: BTreeMap<String, i32>,
    /// Max HP.
    pub max_hp: i32,
    /// Armour class.
    pub ac: i32,
    /// Attack bonus.
    pub attack_bonus: i32,
    /// Melee attack total.
    pub melee_attack: i32,
    /// Ranged attack total.
    pub ranged_attack: i32,
    /// Physical save.
    pub physical_save: i32,
    /// Evasion save.
    pub evasion_save: i32,
    /// Mental save.
    pub mental_save: i32,
}

/// Count result for bulk editor updates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BulkUpdateResult {
    /// Number updated.
    pub updated: i32,
}
