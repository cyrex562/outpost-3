//! Character data model for Harsh Realm.
//!
//! Ported from `src/harsh_realm/models/character.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::item::SaveBonusProfile;
use crate::runtime::JsonObject;

/// Attribute code -> score.
pub type AttributeScores = BTreeMap<String, i32>;
/// Attribute code -> modifier.
pub type AttributeMods = BTreeMap<String, i32>;
/// Skill name -> level.
pub type SkillLevels = BTreeMap<String, i32>;

fn d_one() -> i32 {
    1
}
fn d_xp_next() -> i32 {
    1500
}
fn d_ten() -> i32 {
    10
}
fn d_fifteen() -> i32 {
    15
}

/// A player character in Harsh Realm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Character {
    /// Unique id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Class id.
    pub character_class: String,
    /// Level (default 1).
    #[serde(default = "d_one")]
    pub level: i32,
    /// Current XP.
    #[serde(default)]
    pub xp: i32,
    /// XP to next level (default 1500).
    #[serde(default = "d_xp_next")]
    pub xp_next: i32,
    /// Attribute scores.
    #[serde(default)]
    pub attributes: AttributeScores,
    /// Attribute modifiers.
    #[serde(default)]
    pub attr_mods: AttributeMods,
    /// Skill levels.
    #[serde(default)]
    pub skills: SkillLevels,
    /// Current HP.
    #[serde(default)]
    pub hp: i32,
    /// Effective max HP (= base_max_hp × player_hp_mult, floored at 1).
    #[serde(default)]
    pub max_hp: i32,
    /// Unbuffed XWN base max HP, set at character creation and accumulated at
    /// each level-up. Effective `max_hp` is derived from this via the
    /// `player_hp` difficulty multiplier. Serialised with a default of 0 so
    /// existing saves without this field still deserialise cleanly; treated as
    /// equal to `max_hp` when zero (implying mult = 1.0).
    #[serde(default)]
    pub base_max_hp: i32,
    /// Armour class (default 10).
    #[serde(default = "d_ten")]
    pub ac: i32,
    /// Attack bonus.
    #[serde(default)]
    pub attack_bonus: i32,
    /// Physical save (default 15).
    #[serde(default = "d_fifteen")]
    pub physical_save: i32,
    /// Evasion save (default 15).
    #[serde(default = "d_fifteen")]
    pub evasion_save: i32,
    /// Mental save (default 15).
    #[serde(default = "d_fifteen")]
    pub mental_save: i32,
    /// Save bonuses from items/effects.
    #[serde(default)]
    pub save_bonuses: SaveBonusProfile,
    /// Equipment payloads.
    #[serde(default)]
    pub equipment: Vec<JsonObject>,
    /// Class ability payload.
    #[serde(default)]
    pub class_abilities: JsonObject,
    /// Unspent skill points.
    #[serde(default)]
    pub unspent_skill_points: i32,
    /// Tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Traits.
    #[serde(default)]
    pub traits: Vec<String>,
    /// Position column.
    #[serde(default)]
    pub position_q: i32,
    /// Position row.
    #[serde(default)]
    pub position_r: i32,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            id: String::new(),
            name: String::new(),
            character_class: String::new(),
            level: 1,
            xp: 0,
            xp_next: 1500,
            attributes: AttributeScores::new(),
            attr_mods: AttributeMods::new(),
            skills: SkillLevels::new(),
            hp: 0,
            max_hp: 0,
            base_max_hp: 0,
            ac: 10,
            attack_bonus: 0,
            physical_save: 15,
            evasion_save: 15,
            mental_save: 15,
            save_bonuses: SaveBonusProfile::default(),
            equipment: Vec::new(),
            class_abilities: JsonObject::new(),
            unspent_skill_points: 0,
            tags: Vec::new(),
            traits: Vec::new(),
            position_q: 0,
            position_r: 0,
        }
    }
}

impl Character {
    /// Create a minimal, uninitialized character shell with a fresh UUID id.
    pub fn new(name: impl Into<String>, character_class: impl Into<String>) -> Self {
        Character {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            character_class: character_class.into(),
            ..Character::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_uuid_and_defaults() {
        let c = Character::new("Hero", "warrior");
        assert_eq!(c.name, "Hero");
        assert_eq!(c.level, 1);
        assert_eq!(c.ac, 10);
        assert_eq!(c.physical_save, 15);
        assert_eq!(c.xp_next, 1500);
        assert_eq!(c.id.len(), 36); // canonical UUID
    }

    #[test]
    fn minimal_json_fills_defaults() {
        let c: Character =
            serde_json::from_str(r#"{"id":"x","name":"H","character_class":"expert"}"#).unwrap();
        assert_eq!(c.level, 1);
        assert_eq!(c.mental_save, 15);
        assert!(c.skills.is_empty());
    }
}
