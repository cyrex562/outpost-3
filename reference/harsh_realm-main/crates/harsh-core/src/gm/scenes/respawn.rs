//! Respawn scene handler.
//!
//! Ported from `src/harsh_realm/gm/scenes/respawn.py`.
//! Presents the player with two choices after death: respawn at the nearest
//! settlement (lose one item, lose 15 % XP) or start a new character.

use serde_json::Value as JsonValue;

use crate::character::Character;
use crate::command::ParsedCommand;
use crate::db::WorldDatabase;
use crate::events::GameEvent;
use crate::grid::{Grid, GridCoord, SquareGrid};
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::payloads::notices_combat::{CharacterSnapshot, NarrationNotice};
use crate::payloads::notices_world::CharacterDeathFinalNotice;
use crate::payloads::requests::CharacterRespawnRequested;
use crate::repositories::cell::CellRepository;
use crate::runtime::{InventoryItemRecord, JsonObject};

/// Scene handler for player death and respawn.
pub struct RespawnScene {
    character: Character,
    death_q: i32,
    death_r: i32,
    tick: i64,
    pending_transition: Option<SceneState>,
}

impl RespawnScene {
    /// Create a new respawn scene for the given character at its death location.
    pub fn new(character: Character, death_q: i32, death_r: i32) -> Self {
        Self {
            character,
            death_q,
            death_r,
            tick: 0,
            pending_transition: None,
        }
    }

    /// Create with an explicit tick value (useful for tests).
    pub fn with_tick(mut self, tick: i64) -> Self {
        self.tick = tick;
        self
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn narrate(&self, text: impl Into<String>) -> GameEvent {
        let notice = NarrationNotice { text: text.into() };
        let data = to_json_object(&notice);
        GameEvent::new(self.tick, "gm.narrate", data).with_source("respawn")
    }

    fn execute_respawn(&mut self, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        // Find nearest settlement.
        let (settle_q, settle_r) = self.find_nearest_settlement(db);

        // Restore HP to 50 %.
        let new_hp = (self.character.max_hp / 2).max(1);

        // Choose item to lose (without mutating the inventory).
        let lost_item = self.get_lost_item_name();

        // Reduce XP by 15 %.
        let xp_loss = (self.character.xp as f64 * 0.15).floor() as i32;
        let new_xp = (self.character.xp - xp_loss).max(0);

        let snapshot = character_snapshot(&self.character);
        let request = CharacterRespawnRequested {
            character_id: self.character.id.clone(),
            character_data: snapshot,
            new_hp,
            xp_loss,
            lost_item: lost_item.clone(),
            settlement_q: settle_q,
            settlement_r: settle_r,
            death_q: self.death_q,
            death_r: self.death_r,
        };
        let req_event = GameEvent::new(
            self.tick,
            "character.respawn_requested",
            to_json_object(&request),
        )
        .with_source("respawn");

        let mut narration_lines = vec![
            "You open your eyes. You're in a rough settlement — someone carried you here."
                .to_string(),
            format!("HP restored to {new_hp}/{}.", self.character.max_hp),
        ];
        if xp_loss > 0 {
            narration_lines.push(format!(
                "You lost {xp_loss} XP in the chaos ({new_xp} remaining)."
            ));
        }
        if let Some(ref name) = lost_item {
            narration_lines.push(format!(
                "Your {name} was lost at the scene of the battle. \
                 Return to recover it if you dare."
            ));
        }
        narration_lines.push("Take stock of yourself and continue.".to_string());

        self.pending_transition = Some(SceneState::Exploration);
        Ok(vec![req_event, self.narrate(narration_lines.join("\n"))])
    }

    fn execute_new_character(&mut self) -> Result<Vec<GameEvent>, String> {
        let notice = CharacterDeathFinalNotice {
            name: self.character.name.clone(),
        };
        let death_event =
            GameEvent::new(self.tick, "character.death_final", to_json_object(&notice))
                .with_source("respawn");

        self.pending_transition = Some(SceneState::CharacterCreation);
        Ok(vec![
            self.narrate(format!(
                "{}'s story ends here, in the dust of this hostile world. \
                 Another soul must take up the fight…",
                self.character.name
            )),
            death_event,
        ])
    }

    /// Return the name of one item to lose on death (weapon > armor > any).
    fn get_lost_item_name(&self) -> Option<String> {
        let equipment = &self.character.equipment;
        if equipment.is_empty() {
            return None;
        }

        let parse = |raw: &JsonObject| -> Option<InventoryItemRecord> {
            serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone())).ok()
        };

        for raw in equipment {
            if let Some(item) = parse(raw) {
                if item.r#type == "weapon" || item.name.to_lowercase().contains("weapon") {
                    return Some(item.name);
                }
            }
        }
        for raw in equipment {
            if let Some(item) = parse(raw) {
                if item.r#type == "armor" || item.name.to_lowercase().contains("armor") {
                    return Some(item.name);
                }
            }
        }
        equipment
            .first()
            .and_then(parse)
            .map(|item| item.name)
    }

    /// Find the nearest settlement cell to the death location using Chebyshev distance.
    fn find_nearest_settlement(&self, db: &WorldDatabase) -> (i32, i32) {
        let repo = CellRepository::new(db);
        let settlements = match repo.list_cells_with_feature("settlement") {
            Ok(v) => v,
            Err(_) => return (self.death_q, self.death_r),
        };
        if settlements.is_empty() {
            return (self.death_q, self.death_r);
        }

        let grid = SquareGrid;
        let death = GridCoord {
            q: self.death_q,
            r: self.death_r,
        };
        let best = settlements
            .into_iter()
            .min_by_key(|coord| grid.distance(death, *coord))
            .unwrap_or(death);

        (best.q, best.r)
    }
}

impl SceneHandler for RespawnScene {
    fn get_valid_commands(&self) -> Vec<String> {
        ["1", "respawn", "yes", "2", "new", "help"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        let name = &self.character.name;
        vec![
            format!("--- {name} has fallen ---"),
            String::new(),
            "You drift in and out of consciousness. The battle is lost.".to_string(),
            "A distant voice speaks:".to_string(),
            String::new(),
            "  1. Respawn — Wake at the nearest settlement".to_string(),
            "     (HP 50%, lose one item, lose 15% XP)".to_string(),
            String::new(),
            "  2. New character — This hero's story ends here".to_string(),
            String::new(),
            "Choose: (1 or 2)".to_string(),
        ]
        .join("\n")
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let verb = cmd.verb.as_str();
        let raw = cmd.raw.trim().to_lowercase();

        if ["1", "respawn", "yes"].contains(&verb) || ["1", "respawn", "yes"].contains(&raw.as_str()) {
            return self.execute_respawn(db);
        }
        if ["2", "new"].contains(&verb)
            || ["2", "new", "new character"].contains(&raw.as_str())
        {
            return self.execute_new_character();
        }
        if verb == "help" || raw == "help" {
            return Ok(vec![self.narrate(
                "You are at the respawn screen.\n\
                 \x20 1 or 'respawn' — respawn at nearest settlement\n\
                 \x20 2 or 'new'     — create a new character",
            )]);
        }

        Ok(vec![self.narrate(
            "Please enter 1 (respawn) or 2 (new character).",
        )])
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        self.pending_transition
    }

    fn get_suggestions(&self) -> Vec<String> {
        vec![
            "1 (respawn at settlement)".to_string(),
            "2 (new character)".to_string(),
        ]
    }
}

// ------------------------------------------------------------------
// Serde helpers
// ------------------------------------------------------------------

fn to_json_object<T: serde::Serialize>(value: &T) -> JsonObject {
    match serde_json::to_value(value) {
        Ok(JsonValue::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

fn character_snapshot(c: &Character) -> CharacterSnapshot {
    CharacterSnapshot {
        id: c.id.clone(),
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
        equipment: c.equipment.clone(),
        class_abilities: c.class_abilities.clone(),
        position_q: c.position_q,
        position_r: c.position_r,
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;

    fn make_character(name: &str) -> Character {
        Character::new(name, "warrior")
    }

    #[test]
    fn get_valid_commands_includes_expected_verbs() {
        let scene = RespawnScene::new(make_character("Hero"), 0, 0);
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"respawn".to_string()));
        assert!(cmds.contains(&"1".to_string()));
        assert!(cmds.contains(&"2".to_string()));
        assert!(cmds.contains(&"new".to_string()));
    }

    #[test]
    fn get_prompt_mentions_character_name() {
        let scene = RespawnScene::new(make_character("Korrath"), 5, 7);
        let db = WorldDatabase::open_in_memory().unwrap();
        let prompt = scene.get_prompt(&db);
        assert!(prompt.contains("Korrath"));
        assert!(prompt.contains("HP 50%"));
    }

    #[test]
    fn get_suggestions_returns_two_options() {
        let scene = RespawnScene::new(make_character("Hero"), 0, 0);
        let suggestions = scene.get_suggestions();
        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn help_command_returns_narration_event() {
        let mut scene = RespawnScene::new(make_character("Hero"), 0, 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = ParsedCommand {
            verb: "help".to_string(),
            args: vec![],
            raw: "help".to_string(),
            direction: None,
        };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "gm.narrate");
    }

    #[test]
    fn unknown_command_returns_guidance_narration() {
        let mut scene = RespawnScene::new(make_character("Hero"), 0, 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = ParsedCommand {
            verb: "go".to_string(),
            args: vec![],
            raw: "go north".to_string(),
            direction: None,
        };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "gm.narrate");
        assert!(events[0]
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("1 (respawn)"));
    }

    #[test]
    fn new_character_command_transitions_to_character_creation() {
        let mut scene = RespawnScene::new(make_character("Vex"), 0, 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = ParsedCommand {
            verb: "2".to_string(),
            args: vec![],
            raw: "2".to_string(),
            direction: None,
        };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert!(events
            .iter()
            .any(|e| e.event_type == "character.death_final"));
        assert_eq!(
            scene.check_transitions(&events),
            Some(SceneState::CharacterCreation)
        );
    }

    #[test]
    fn respawn_command_emits_respawn_requested_and_transitions_to_exploration() {
        let mut c = make_character("Dura");
        c.max_hp = 20;
        c.hp = 0;
        c.xp = 100;
        let mut scene = RespawnScene::new(c, 2, 3);
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = ParsedCommand {
            verb: "1".to_string(),
            args: vec![],
            raw: "1".to_string(),
            direction: None,
        };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert!(events
            .iter()
            .any(|e| e.event_type == "character.respawn_requested"));
        assert_eq!(
            scene.check_transitions(&events),
            Some(SceneState::Exploration)
        );
    }

    #[test]
    fn respawn_restores_hp_to_half_max() {
        let mut c = make_character("Dura");
        c.max_hp = 20;
        c.hp = 0;
        let mut scene = RespawnScene::new(c, 0, 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = ParsedCommand {
            verb: "respawn".to_string(),
            args: vec![],
            raw: "respawn".to_string(),
            direction: None,
        };
        let events = scene.handle_command(&cmd, &db).unwrap();
        let req = events
            .iter()
            .find(|e| e.event_type == "character.respawn_requested")
            .expect("missing respawn_requested event");
        let new_hp = req.data["new_hp"].as_i64().unwrap();
        assert_eq!(new_hp, 10); // 20 / 2
    }

    #[test]
    fn get_lost_item_name_prefers_weapon() {
        let mut c = make_character("Boro");
        let armor_obj: JsonObject = serde_json::from_str(
            r#"{"name":"leather armor","type":"armor","quantity":1,"value":10}"#,
        )
        .unwrap();
        let weapon_obj: JsonObject = serde_json::from_str(
            r#"{"name":"short sword","type":"weapon","quantity":1,"value":20}"#,
        )
        .unwrap();
        c.equipment = vec![armor_obj, weapon_obj];
        let scene = RespawnScene::new(c, 0, 0);
        assert_eq!(
            scene.get_lost_item_name(),
            Some("short sword".to_string())
        );
    }

    #[test]
    fn get_lost_item_name_none_when_empty() {
        let c = make_character("Ghost");
        let scene = RespawnScene::new(c, 0, 0);
        assert!(scene.get_lost_item_name().is_none());
    }

    #[test]
    fn check_transitions_none_before_command() {
        let scene = RespawnScene::new(make_character("X"), 0, 0);
        assert_eq!(scene.check_transitions(&[]), None);
    }
}
