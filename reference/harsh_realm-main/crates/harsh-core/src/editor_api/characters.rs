//! Character/entity-editor API models. Ported from `models/editor_api/characters.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Editable entity fields (all optional for partial updates).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CharacterUpdateRequest {
    /// Name.
    #[serde(default)]
    pub name: Option<String>,
    /// Entity type.
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Alive flag.
    #[serde(default)]
    pub alive: Option<bool>,
    /// Data payload.
    #[serde(default)]
    pub data: Option<JsonObject>,
}

/// Preview input for derived character stats.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterRecalcPreviewRequest {
    /// Class id.
    pub character_class: String,
    /// Level.
    pub level: i32,
    /// Attribute scores.
    pub attributes: BTreeMap<String, i32>,
    /// Equipment payloads.
    #[serde(default)]
    pub equipment: Vec<JsonObject>,
}

fn d_character() -> String {
    "character".to_string()
}

/// Payload for creating a new entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterCreateRequest {
    /// Name.
    pub name: String,
    /// Entity type (default `"character"`).
    #[serde(default = "d_character")]
    pub entity_type: String,
    /// Location column.
    #[serde(default)]
    pub location_q: i32,
    /// Location row.
    #[serde(default)]
    pub location_r: i32,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}
