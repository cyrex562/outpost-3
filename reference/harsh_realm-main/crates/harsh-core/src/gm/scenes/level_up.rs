//! Level-up scene handler.
//!
//! Ported from `src/harsh_realm/gm/scenes/level_up.py`.
//! Allows the player to spend unspent skill points; finishes by persisting the
//! character and transitioning back to Exploration.

use serde_json::Value as JsonValue;

use crate::character::Character;
use crate::command::ParsedCommand;
use crate::db::WorldDatabase;
use crate::events::GameEvent;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::payloads::notices_combat::NarrationNotice;
use crate::repositories::entity::EntityRepository;
use crate::runtime::JsonObject;

/// Scene handler for allocating level-up skill points.
pub struct LevelUpScene {
    character: Character,
    tick: i64,
    points_remaining: i32,
    finished: bool,
}

impl LevelUpScene {
    /// Create a new level-up scene for the given character.
    pub fn new(character: Character) -> Self {
        let points = character.unspent_skill_points;
        Self {
            character,
            tick: 0,
            points_remaining: points,
            finished: false,
        }
    }

    /// Set the game tick (useful for tests).
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
        GameEvent::new(self.tick, "gm.narrate", data).with_source("level_up")
    }

    fn handle_spend(&mut self, cmd: &ParsedCommand) -> Result<Vec<GameEvent>, String> {
        // Accept "spend <skill> [level]" or bare "<skill> [level]".
        let mut text = cmd.raw.trim().to_lowercase();
        if text.starts_with("spend ") {
            text = text["spend ".len()..].trim().to_string();
        }
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(vec![self.narrate("Enter a skill name to spend points.")]);
        }

        let skill_id = parts[0].to_string();
        let target_level: Option<i32> = parts.get(1).and_then(|s| s.parse().ok());

        let current = self.character.skills.get(&skill_id).copied().unwrap_or(-1);
        let desired = target_level.unwrap_or(current + 1);

        if desired <= current {
            return Ok(vec![self.narrate(format!(
                "{skill_id} is already level {current}. You can only raise skills here."
            ))]);
        }
        if desired > 4 {
            return Ok(vec![self.narrate("Maximum skill level is 4.")]);
        }

        // XWN cost: raising to level N costs (N + 1) points per step.
        // Accumulated cost from current+1 to desired inclusive.
        let cost: i32 = (current + 1..=desired).map(|lv| lv + 1).sum();

        if cost > self.points_remaining {
            return Ok(vec![self.narrate(format!(
                "Raising {skill_id} from {current} to {desired} costs {cost} points, \
                 but you only have {}.",
                self.points_remaining
            ))]);
        }

        self.character.skills.insert(skill_id.clone(), desired);
        self.points_remaining -= cost;
        self.character.unspent_skill_points = self.points_remaining;

        Ok(vec![self.narrate(format!(
            "{} raised to {desired}. {} point(s) remaining.",
            capitalize(&skill_id),
            self.points_remaining
        ))])
    }

    fn handle_done_sync(&mut self, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        EntityRepository::new(db)
            .save_character_state(&mut self.character)
            .map_err(|e| e.to_string())?;
        self.finished = true;
        Ok(vec![self.narrate(
            "Level-up complete! Returning to exploration.",
        )])
    }

    fn handle_status(&self) -> GameEvent {
        let skills: Vec<String> = self
            .character
            .skills
            .iter()
            .map(|(s, l)| format!("{s}:{l}"))
            .collect();
        let skills_str = if skills.is_empty() {
            "none".to_string()
        } else {
            skills.join(", ")
        };
        self.narrate(format!(
            "--- {} (Level {}) ---\nUnspent Points: {}\nCurrent Skills: {skills_str}\n\
             Type 'done' to finish.",
            self.character.name, self.character.level, self.points_remaining,
        ))
    }

    fn handle_help(&self) -> GameEvent {
        self.narrate(
            "Level Up Commands:\n\
             \x20 spend <skill> [level] — spend points to raise a skill\n\
             \x20 status                — show current points and skills\n\
             \x20 done                  — finalize level up and return to game\n\
             Cost: Level 0 costs 1 pt, Level 1 costs 2 pts, Level 2 costs 3 pts.",
        )
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

impl SceneHandler for LevelUpScene {
    fn get_valid_commands(&self) -> Vec<String> {
        ["spend", "done", "help", "status"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        format!("[Level Up — {} pts]", self.points_remaining)
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        match cmd.verb.as_str() {
            "done" => self.handle_done_sync(db),
            "status" => Ok(vec![self.handle_status()]),
            "help" => Ok(vec![self.handle_help()]),
            _ => self.handle_spend(cmd),
        }
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        if self.finished {
            Some(SceneState::Exploration)
        } else {
            None
        }
    }

    fn get_suggestions(&self) -> Vec<String> {
        let mut s = vec!["done".to_string(), "status".to_string(), "help".to_string()];
        for skill in &["stab", "shoot", "notice", "talk", "sneak"] {
            s.push(skill.to_string());
        }
        s
    }
}

fn to_json_object<T: serde::Serialize>(value: &T) -> JsonObject {
    match serde_json::to_value(value) {
        Ok(JsonValue::Object(map)) => map,
        _ => JsonObject::new(),
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
        let mut c = Character::new(name, "expert");
        c.level = 2;
        c.unspent_skill_points = 3;
        c
    }

    fn cmd(verb: &str, raw: &str) -> ParsedCommand {
        ParsedCommand {
            verb: verb.to_string(),
            args: vec![],
            raw: raw.to_string(),
            direction: None,
        }
    }

    #[test]
    fn prompt_shows_points() {
        let scene = LevelUpScene::new(make_character("Hero"));
        let db = WorldDatabase::open_in_memory().unwrap();
        let prompt = scene.get_prompt(&db);
        assert!(prompt.contains("3 pts"), "prompt: {prompt}");
    }

    #[test]
    fn get_valid_commands_contains_spend_and_done() {
        let scene = LevelUpScene::new(make_character("Hero"));
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"spend".to_string()));
        assert!(cmds.contains(&"done".to_string()));
    }

    #[test]
    fn help_returns_narrate_event() {
        let mut scene = LevelUpScene::new(make_character("Hero"));
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("help", "help"), &db).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "gm.narrate");
    }

    #[test]
    fn status_returns_narrate_with_character_name() {
        let mut scene = LevelUpScene::new(make_character("Valka"));
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("status", "status"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("Valka"), "text: {text}");
    }

    #[test]
    fn spend_raises_skill_and_deducts_points() {
        let mut scene = LevelUpScene::new(make_character("Torr"));
        let db = WorldDatabase::open_in_memory().unwrap();
        // Raise "stab" from -1 to 0: costs 1 point.
        let events = scene.handle_command(&cmd("stab", "stab"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("2 point(s) remaining"), "text: {text}");
        assert_eq!(*scene.character.skills.get("stab").unwrap(), 0);
    }

    #[test]
    fn spend_rejects_above_max_level() {
        let mut c = make_character("Blex");
        c.skills.insert("stab".to_string(), 4);
        c.unspent_skill_points = 10;
        let mut scene = LevelUpScene::new(c);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene
            .handle_command(&cmd("spend", "spend stab 5"), &db)
            .unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("Maximum skill level is 4"), "text: {text}");
    }

    #[test]
    fn spend_rejects_insufficient_points() {
        let mut c = make_character("Krix");
        c.unspent_skill_points = 0;
        c.skills.insert("talk".to_string(), -1);
        let mut scene = LevelUpScene::new(c);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("talk", "talk"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("only have 0"), "text: {text}");
    }

    #[test]
    fn done_transitions_to_exploration() {
        let scene_char = make_character("Vix");
        let mut scene = LevelUpScene::new(scene_char);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("done", "done"), &db).unwrap();
        assert!(!events.is_empty());
        assert_eq!(
            scene.check_transitions(&events),
            Some(SceneState::Exploration)
        );
    }

    #[test]
    fn no_transition_before_done() {
        let scene = LevelUpScene::new(make_character("Nex"));
        assert_eq!(scene.check_transitions(&[]), None);
    }
}
