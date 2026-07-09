//! Typed relational persistence models for character and NPC aggregates.
//!
//! Ported from `src/harsh_realm/models/entity_state.py`. These are the durable
//! row payloads owned by the entity-state repository.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::character::Character;
use crate::item::SaveBonusProfile;
use crate::npc::{disposition_str_to_int, UNEPersonality};
use crate::runtime::JsonObject;

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

/// Accept either an integer disposition or a legacy string label.
fn de_disposition<'de, D>(d: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(d)? {
        serde_json::Value::String(s) => Ok(disposition_str_to_int(&s)),
        serde_json::Value::Number(n) => Ok(n.as_i64().unwrap_or(0) as i32),
        _ => Ok(0),
    }
}

/// Typed relational snapshot for a character aggregate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterState {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Class id.
    pub character_class: String,
    /// Level.
    #[serde(default = "d_one")]
    pub level: i32,
    /// XP.
    #[serde(default)]
    pub xp: i32,
    /// XP to next.
    #[serde(default = "d_xp_next")]
    pub xp_next: i32,
    /// Attribute scores.
    #[serde(default)]
    pub attributes: BTreeMap<String, i32>,
    /// Attribute mods.
    #[serde(default)]
    pub attr_mods: BTreeMap<String, i32>,
    /// Skills.
    #[serde(default)]
    pub skills: BTreeMap<String, i32>,
    /// HP.
    #[serde(default)]
    pub hp: i32,
    /// Max HP.
    #[serde(default)]
    pub max_hp: i32,
    /// AC.
    #[serde(default = "d_ten")]
    pub ac: i32,
    /// Attack bonus.
    #[serde(default)]
    pub attack_bonus: i32,
    /// Physical save.
    #[serde(default = "d_fifteen")]
    pub physical_save: i32,
    /// Evasion save.
    #[serde(default = "d_fifteen")]
    pub evasion_save: i32,
    /// Mental save.
    #[serde(default = "d_fifteen")]
    pub mental_save: i32,
    /// Save bonuses.
    #[serde(default)]
    pub save_bonuses: SaveBonusProfile,
    /// Equipment.
    #[serde(default)]
    pub equipment: Vec<JsonObject>,
    /// Class abilities.
    #[serde(default)]
    pub class_abilities: JsonObject,
    /// Position column.
    #[serde(default)]
    pub position_q: i32,
    /// Position row.
    #[serde(default)]
    pub position_r: i32,
}

impl CharacterState {
    /// Build relational state from a character model.
    pub fn from_character(c: &Character) -> Self {
        CharacterState {
            entity_id: c.id.clone(),
            name: c.name.clone(),
            character_class: c.character_class.clone(),
            level: c.level,
            xp: c.xp,
            xp_next: c.xp_next,
            attributes: c.attributes.clone(),
            attr_mods: c.attr_mods.clone(),
            skills: c.skills.clone(),
            hp: c.hp,
            max_hp: c.max_hp,
            ac: c.ac,
            attack_bonus: c.attack_bonus,
            physical_save: c.physical_save,
            evasion_save: c.evasion_save,
            mental_save: c.mental_save,
            save_bonuses: c.save_bonuses,
            equipment: c.equipment.clone(),
            class_abilities: c.class_abilities.clone(),
            position_q: c.position_q,
            position_r: c.position_r,
        }
    }

    /// Rebuild a character model from relational state.
    pub fn to_character(&self) -> Character {
        Character {
            id: self.entity_id.clone(),
            name: self.name.clone(),
            character_class: self.character_class.clone(),
            level: self.level,
            xp: self.xp,
            xp_next: self.xp_next,
            attributes: self.attributes.clone(),
            attr_mods: self.attr_mods.clone(),
            skills: self.skills.clone(),
            hp: self.hp,
            max_hp: self.max_hp,
            ac: self.ac,
            attack_bonus: self.attack_bonus,
            physical_save: self.physical_save,
            evasion_save: self.evasion_save,
            mental_save: self.mental_save,
            save_bonuses: self.save_bonuses,
            equipment: self.equipment.clone(),
            class_abilities: self.class_abilities.clone(),
            position_q: self.position_q,
            position_r: self.position_r,
            ..Character::default()
        }
    }
}

/// Typed relational snapshot for an NPC aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpcState {
    /// Entity id.
    pub entity_id: String,
    /// Occupation.
    #[serde(default)]
    pub occupation: String,
    /// Personality traits.
    #[serde(default)]
    pub personality_traits: Vec<String>,
    /// Motivation.
    #[serde(default)]
    pub motivation: String,
    /// Appearance.
    #[serde(default)]
    pub appearance: String,
    /// Greeting.
    #[serde(default)]
    pub greeting: String,
    /// Faction id.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// Disposition (int; legacy string labels accepted on input).
    #[serde(default, deserialize_with = "de_disposition")]
    pub disposition: i32,
    /// UNE personality profile.
    #[serde(default)]
    pub une_personality: Option<UNEPersonality>,
    /// Building id.
    #[serde(default)]
    pub building_id: Option<String>,
    /// Establishment type.
    #[serde(default)]
    pub establishment_type: Option<String>,
    /// Establishment name.
    #[serde(default)]
    pub establishment_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_state_round_trips_through_character() {
        let mut c = Character::new("Hero", "warrior");
        c.level = 3;
        c.hp = 12;
        let state = CharacterState::from_character(&c);
        assert_eq!(state.entity_id, c.id);
        let back = state.to_character();
        assert_eq!(back.level, 3);
        assert_eq!(back.hp, 12);
        assert_eq!(back.id, c.id);
    }

    #[test]
    fn npc_state_normalizes_legacy_disposition() {
        let n: NpcState =
            serde_json::from_str(r#"{"entity_id":"n1","disposition":"friendly"}"#).unwrap();
        assert_eq!(n.disposition, 2);
    }
}
