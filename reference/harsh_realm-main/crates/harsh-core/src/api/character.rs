//! Public character summary API model. Ported from `models/api/character.py`.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Public character summary returned by the REST API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSummary {
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
    /// Class.
    pub character_class: String,
    /// Level.
    pub level: i32,
    /// Current HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// Armour class.
    pub ac: i32,
    /// XP.
    pub xp: i32,
    /// XP to next.
    pub xp_next: i32,
    /// Attack bonus.
    pub attack_bonus: i32,
    /// Equipment payloads.
    #[serde(default)]
    pub equipment: Vec<JsonObject>,
    /// Class abilities payload.
    #[serde(default)]
    pub class_abilities: JsonObject,
}
