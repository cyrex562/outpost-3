//! Base scene-state enum and scene-handler interface for the GM.
//!
//! Ported from `src/harsh_realm/gm/scenes/base.py`. The Python `SceneHandler`
//! Protocol becomes a Rust trait; `handle_command` is synchronous here (the DB
//! layer is sync `rusqlite`) and returns a `Result` instead of awaiting.

use serde::{Deserialize, Serialize};

use crate::command::ParsedCommand;
use crate::db::WorldDatabase;
use crate::events::GameEvent;

/// All possible GM scene states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SceneState {
    /// Character creation flow.
    #[serde(rename = "char_create")]
    CharacterCreation,
    /// Open-world exploration.
    #[serde(rename = "exploration")]
    Exploration,
    /// Combat encounter.
    #[serde(rename = "combat")]
    Combat,
    /// Social/NPC interaction.
    #[serde(rename = "social")]
    Social,
    /// Shopping at a store.
    #[serde(rename = "shopping")]
    Shopping,
    /// Level-up flow.
    #[serde(rename = "level_up")]
    LevelUp,
    /// Respawn after death.
    #[serde(rename = "respawn")]
    Respawn,
    /// Inside a dungeon.
    #[serde(rename = "dungeon")]
    Dungeon,
    /// Inside a town.
    #[serde(rename = "town")]
    Town,
}

impl SceneState {
    /// The stable string value (matches the Python `Enum` `.value`).
    pub fn as_str(self) -> &'static str {
        match self {
            SceneState::CharacterCreation => "char_create",
            SceneState::Exploration => "exploration",
            SceneState::Combat => "combat",
            SceneState::Social => "social",
            SceneState::Shopping => "shopping",
            SceneState::LevelUp => "level_up",
            SceneState::Respawn => "respawn",
            SceneState::Dungeon => "dungeon",
            SceneState::Town => "town",
        }
    }

    /// Parse a scene state from its stable string value.
    pub fn from_value(value: &str) -> Option<SceneState> {
        Some(match value {
            "char_create" => SceneState::CharacterCreation,
            "exploration" => SceneState::Exploration,
            "combat" => SceneState::Combat,
            "social" => SceneState::Social,
            "shopping" => SceneState::Shopping,
            "level_up" => SceneState::LevelUp,
            "respawn" => SceneState::Respawn,
            "dungeon" => SceneState::Dungeon,
            "town" => SceneState::Town,
            _ => return None,
        })
    }
}

/// Interface every scene handler must satisfy.
pub trait SceneHandler {
    /// Canonical verb strings accepted in this scene.
    fn get_valid_commands(&self) -> Vec<String>;

    /// The current input prompt shown to the player.
    fn get_prompt(&self, db: &WorldDatabase) -> String;

    /// Process player input and return any resulting events.
    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String>;

    /// Inspect the event list and return a new state if a transition is needed.
    fn check_transitions(&self, events: &[GameEvent]) -> Option<SceneState>;

    /// Contextual command suggestions (clickable hints); empty by default.
    fn get_suggestions(&self) -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_state_value_roundtrip() {
        for state in [
            SceneState::CharacterCreation,
            SceneState::Exploration,
            SceneState::Combat,
            SceneState::Social,
            SceneState::Shopping,
            SceneState::LevelUp,
            SceneState::Respawn,
            SceneState::Dungeon,
            SceneState::Town,
        ] {
            assert_eq!(SceneState::from_value(state.as_str()), Some(state));
        }
        assert_eq!(SceneState::CharacterCreation.as_str(), "char_create");
        assert!(SceneState::from_value("nonexistent").is_none());
    }

    #[test]
    fn serde_uses_value_strings() {
        let json = serde_json::to_string(&SceneState::LevelUp).unwrap();
        assert_eq!(json, "\"level_up\"");
        let parsed: SceneState = serde_json::from_str("\"dungeon\"").unwrap();
        assert_eq!(parsed, SceneState::Dungeon);
    }
}
