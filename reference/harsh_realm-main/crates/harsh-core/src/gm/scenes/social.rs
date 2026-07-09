//! Social scene handler — NPC conversation, skill checks, disposition, healing.
//!
//! Ported from the Python `SocialScene` class (social.py +
//! _social_commands_mixin.py + _social_healer_mixin.py +
//! _social_skillcheck_mixin.py).

use crate::character::Character;
use crate::command::ParsedCommand;
use crate::db::WorldDatabase;
use crate::engine_results::SkillCheckResult;
use crate::events::GameEvent;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::gm::scenes::social_support::disposition_label;
use crate::healing::HealingSystem;
use crate::payloads::notices_combat::CharacterSnapshot;
use crate::payloads::notices_world::{ActionSkillCheckNotice, CharacterExpertRerollNotice};
use crate::payloads::requests::{
    CharacterUpdateRequested, SocialDispositionUpdateRequested, SocialHealerRequested,
};
use crate::repositories::entity::EntityRepository;
use crate::repositories::resources::{ResourceInstance, ResourceRepository, GOLD_RESOURCE_ID};
use crate::runtime::{JsonObject, JsonValue, SceneNpcRecord};
use crate::skill_checks::SkillCheckResolver;
use rand::thread_rng;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

/// State for a pending Expert-class reroll offer.
struct ExpertRerollState {
    verb: String,
    original_result: SkillCheckResult,
}

// ---------------------------------------------------------------------------
// Public struct
// ---------------------------------------------------------------------------

pub struct SocialScene {
    npc_entity_id: String,
    npc_name: String,
    npc_data: SceneNpcRecord,
    tick: i64,
    return_to: Option<String>,
    disposition: i32,
    exit_to_exploration: bool,
    exit_to_combat: bool,
    pending_reroll: Option<ExpertRerollState>,
}

// ---------------------------------------------------------------------------
// Constructor helpers
// ---------------------------------------------------------------------------

fn npc_data_disposition(npc_data: &SceneNpcRecord) -> i32 {
    let raw = match &npc_data.disposition {
        JsonValue::Number(n) => {
            let v = n.as_i64().unwrap_or(0) as i32;
            return v.clamp(-3, 3);
        }
        JsonValue::String(s) => s.as_str(),
        _ => return 0,
    };
    let mapped = match raw {
        "hostile" => -3,
        "wary" => -1,
        "neutral" => 0,
        "friendly" => 2,
        "helpful" => 3,
        _ => 0,
    };
    mapped.clamp(-3, 3)
}

fn is_healer_npc(npc_data: &SceneNpcRecord) -> bool {
    let occ = npc_data.occupation.to_lowercase();
    ["healer", "doctor", "medic", "herbalist", "physician", "chirurgeon"]
        .iter()
        .any(|&kw| occ.contains(kw))
}

// ---------------------------------------------------------------------------
// impl SocialScene
// ---------------------------------------------------------------------------

impl SocialScene {
    pub fn new(
        npc_entity_id: String,
        npc_name: String,
        npc_data: SceneNpcRecord,
        tick: i64,
        return_to: Option<String>,
    ) -> Self {
        let disposition = npc_data_disposition(&npc_data);
        Self {
            npc_entity_id,
            npc_name,
            npc_data,
            tick,
            return_to,
            disposition,
            exit_to_exploration: false,
            exit_to_combat: false,
            pending_reroll: None,
        }
    }

    // -----------------------------------------------------------------------
    // Narration helper
    // -----------------------------------------------------------------------

    fn narrate(&self, text: impl Into<String>) -> GameEvent {
        let mut data = JsonObject::new();
        data.insert("text".to_string(), JsonValue::String(text.into()));
        GameEvent::new(self.tick, "gm.narrate", data).with_source("gm")
    }

    // -----------------------------------------------------------------------
    // Command handlers
    // -----------------------------------------------------------------------

    fn handle_leave(&mut self, _cmd: &ParsedCommand, _db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        self.exit_to_exploration = true;
        let mood = disposition_label(self.disposition);
        let msg = format!(
            "You take your leave of {}. They watch you go, their demeanour {}.",
            self.npc_name, mood
        );
        Ok(vec![self.narrate(msg)])
    }

    fn handle_status(&self, _cmd: &ParsedCommand, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let repo = EntityRepository::new(db);
        match repo.load_first_character() {
            Ok(Some(c)) => {
                let text = format!(
                    "{} | {} Lv{} | HP {}/{} | AC {} | XP {}/{}\nTalking with: {} ({})",
                    c.name,
                    c.character_class,
                    c.level,
                    c.hp,
                    c.max_hp,
                    c.ac,
                    c.xp,
                    c.xp_next,
                    self.npc_name,
                    disposition_label(self.disposition),
                );
                Ok(vec![self.narrate(text)])
            }
            _ => Ok(vec![self.narrate("[ERROR] No character found.")]),
        }
    }

    fn handle_inventory(&self, _cmd: &ParsedCommand, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let repo = EntityRepository::new(db);
        match repo.load_first_character() {
            Ok(Some(c)) => {
                if c.equipment.is_empty() {
                    Ok(vec![self.narrate("You are carrying nothing.")])
                } else {
                    let names: Vec<String> = c
                        .equipment
                        .iter()
                        .map(|e| {
                            e.get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown item")
                                .to_string()
                        })
                        .collect();
                    Ok(vec![self.narrate(format!("You are carrying: {}.", names.join(", ")))])
                }
            }
            _ => Ok(vec![self.narrate("[ERROR] No character found.")]),
        }
    }

    fn handle_help(&self, _cmd: &ParsedCommand, _db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let text = "\
Commands available in conversation:
  convince  — persuade the NPC with a Charisma check
  intimidate — coerce the NPC with a Strength check
  deceive   — mislead the NPC with a Charisma check
  bribe     — offer coin (Charisma + gold)
  connect   — find common ground (Charisma)
  ask       — ask the NPC about a topic
  heal      — ask the NPC for healing (healers only)
  leave     — end the conversation
  status    — show your current stats
  inventory — list what you are carrying
  help      — show this message";
        Ok(vec![self.narrate(text)])
    }

    fn handle_ask(&self, cmd: &ParsedCommand, _db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let topic = if cmd.args.is_empty() {
            "something".to_string()
        } else {
            cmd.args.join(" ")
        };
        let text = if self.disposition >= 1 {
            format!(
                "{} speaks openly about {}. \"I'll tell you what I know,\" they say.",
                self.npc_name, topic
            )
        } else if self.disposition == 0 {
            format!(
                "{} is guarded on the subject of {}. They share little.",
                self.npc_name, topic
            )
        } else {
            format!(
                "{} refuses to discuss {}. They eye you with suspicion.",
                self.npc_name, topic
            )
        };
        Ok(vec![self.narrate(text)])
    }

    fn handle_skill_check(
        &mut self,
        verb: &str,
        _cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let repo = EntityRepository::new(db);
        let char = match repo.load_first_character() {
            Ok(Some(c)) => c,
            _ => return Ok(vec![self.narrate("[ERROR] No character found.")]),
        };

        let mut resolver = SkillCheckResolver::new(db);
        let result = match resolver.resolve(verb, &char, None, 0, 0, &mut thread_rng()) {
            Ok(r) => r,
            Err(e) => return Ok(vec![self.narrate(format!("[ERROR] Skill check failed: {}", e))]),
        };

        let mut events: Vec<GameEvent> = Vec::new();

        // Narrate the check result
        events.push(self.narrate(result.narration.clone()));

        // Apply disposition delta
        let old_disp = self.disposition;
        let new_disp = (self.disposition + result.disposition_delta).clamp(-3, 3);
        self.disposition = new_disp;
        self.npc_data.disposition = JsonValue::Number(serde_json::Number::from(new_disp));

        if new_disp != old_disp {
            let shift_msg = format!(
                "{}'s disposition shifts from {} to {}.",
                self.npc_name,
                disposition_label(old_disp),
                disposition_label(new_disp)
            );
            events.push(self.narrate(shift_msg));
        }

        // Emit disposition update request (HR-778: carry npc_name so the handler
        // can enrich the notice without a second DB round-trip).
        let disp_payload = SocialDispositionUpdateRequested {
            entity_id: self.npc_entity_id.clone(),
            new_disposition: self.disposition,
            npc_name: Some(self.npc_name.clone()),
        };
        let disp_data = to_json_object(&disp_payload);
        events.push(
            GameEvent::new(self.tick, "social.disposition_update_requested", disp_data)
                .with_source("gm"),
        );

        // Emit skill check notice
        let check_notice = ActionSkillCheckNotice {
            verb: verb.to_string(),
            skill: result.skill.clone(),
            attribute: result.attribute.clone(),
            roll: result.roll,
            total: result.total,
            difficulty: result.difficulty,
            margin: result.margin,
            success: result.success,
            outcome_key: result.outcome_key.clone(),
            disposition_delta: result.disposition_delta,
            npc_entity_id: Some(self.npc_entity_id.clone()),
        };
        events.push(
            GameEvent::new(self.tick, "action.skill_check", to_json_object(&check_notice))
                .with_source("gm"),
        );

        // Flavor text
        if result.success {
            events.push(self.narrate(success_flavor(verb, &result.outcome_key)));
        } else {
            events.push(self.narrate(failure_flavor(verb, &result.outcome_key)));
        }

        // Expert reroll offer
        let expert_reroll_available = char
            .class_abilities
            .get("expert_reroll_available")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if !result.success && char.character_class == "expert" && expert_reroll_available {
            self.pending_reroll = Some(ExpertRerollState {
                verb: verb.to_string(),
                original_result: result.clone(),
            });
            events.push(self.narrate(
                "Your Expert ability allows you to reroll this check. Use it? (yes/no)".to_string(),
            ));
        }

        // Combat escalation
        if self.disposition <= -3 {
            events.push(self.narrate(format!(
                "{} has had enough. They reach for a weapon!",
                self.npc_name
            )));
            self.exit_to_combat = true;
        }

        Ok(events)
    }

    fn handle_expert_reroll(&mut self, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let reroll_state = match self.pending_reroll.take() {
            Some(s) => s,
            None => return Ok(vec![self.narrate("No reroll available.")]),
        };

        let repo = EntityRepository::new(db);
        let mut char = match repo.load_first_character() {
            Ok(Some(c)) => c,
            _ => return Ok(vec![self.narrate("[ERROR] No character found.")]),
        };

        let mut resolver = SkillCheckResolver::new(db);
        let new_result =
            match resolver.resolve(&reroll_state.verb, &char, None, 0, 0, &mut thread_rng()) {
                Ok(r) => r,
                Err(e) => {
                    return Ok(vec![self.narrate(format!("[ERROR] Reroll failed: {}", e))])
                }
            };

        let original = &reroll_state.original_result;
        let use_new = new_result.total >= original.total;
        let (best, used_label) = if use_new {
            (&new_result, "reroll")
        } else {
            (original, "original")
        };

        let mut events: Vec<GameEvent> = Vec::new();
        events.push(self.narrate(format!(
            "Original roll: {} | Reroll: {} — using the {} (total {}).",
            original.total, new_result.total, used_label, best.total
        )));
        events.push(self.narrate(best.narration.clone()));

        // Mark reroll used
        char.class_abilities
            .insert("expert_reroll_available".to_string(), JsonValue::Bool(false));

        let snapshot = character_snapshot(&char);
        let update_req = CharacterUpdateRequested {
            character_id: char.id.clone(),
            character_data: snapshot,
        };
        events.push(
            GameEvent::new(
                self.tick,
                "social.update_character_requested",
                to_json_object(&update_req),
            )
            .with_source("gm"),
        );

        // If new result is a success but original wasn't, apply disposition delta
        if best.success && !original.success {
            let old_disp = self.disposition;
            let new_disp = (self.disposition + best.disposition_delta).clamp(-3, 3);
            self.disposition = new_disp;
            self.npc_data.disposition = JsonValue::Number(serde_json::Number::from(new_disp));
            if new_disp != old_disp {
                events.push(self.narrate(format!(
                    "{}'s disposition shifts from {} to {}.",
                    self.npc_name,
                    disposition_label(old_disp),
                    disposition_label(new_disp)
                )));
            }
            let disp_payload = SocialDispositionUpdateRequested {
                entity_id: self.npc_entity_id.clone(),
                new_disposition: self.disposition,
                npc_name: Some(self.npc_name.clone()), // HR-778
            };
            events.push(
                GameEvent::new(
                    self.tick,
                    "social.disposition_update_requested",
                    to_json_object(&disp_payload),
                )
                .with_source("gm"),
            );
        }

        let reroll_notice = CharacterExpertRerollNotice {
            verb: reroll_state.verb.clone(),
            original_total: original.total,
            reroll_total: new_result.total,
            used: used_label.to_string(),
        };
        events.push(
            GameEvent::new(
                self.tick,
                "character.expert_reroll",
                to_json_object(&reroll_notice),
            )
            .with_source("gm"),
        );

        Ok(events)
    }

    fn handle_heal(&self, _cmd: &ParsedCommand, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        if !is_healer_npc(&self.npc_data) {
            return Ok(vec![self.narrate(format!(
                "{} is not a healer and cannot tend to your wounds.",
                self.npc_name
            ))]);
        }

        let repo = EntityRepository::new(db);
        let char = match repo.load_first_character() {
            Ok(Some(c)) => c,
            _ => return Ok(vec![self.narrate("[ERROR] No character found.")]),
        };

        if char.hp >= char.max_hp {
            return Ok(vec![self.narrate("You are already at full health.")]);
        }

        let res_repo = ResourceRepository::new(db);
        let gold_current = res_repo
            .get(&char.id, GOLD_RESOURCE_ID)
            .ok()
            .flatten()
            .map(|r| r.current)
            .unwrap_or(0);

        let mut char_mut = char.clone();
        let healing = HealingSystem.town_healer(&mut char_mut, 5, Some(gold_current), false);

        let mut events: Vec<GameEvent> = Vec::new();

        if healing.healed {
            let gold_spent = gold_current.min(healing.cost);
            let new_gold = gold_current - gold_spent;

            let _ = res_repo.set(&ResourceInstance {
                entity_id: char.id.clone(),
                resource_id: GOLD_RESOURCE_ID.to_string(),
                current: new_gold,
                max: None,
                last_regen_tick: 0,
            });

            let heal_req = SocialHealerRequested {
                character_id: char.id.clone(),
                character_data: character_snapshot(&char_mut),
                hp_restored: healing.hp_restored,
                gold_spent,
                npc_name: self.npc_name.clone(),
            };
            events.push(
                GameEvent::new(self.tick, "social.healer_requested", to_json_object(&heal_req))
                    .with_source("gm"),
            );
        }

        events.push(self.narrate(healing.narration));
        Ok(events)
    }
}

// ---------------------------------------------------------------------------
// SceneHandler impl
// ---------------------------------------------------------------------------

impl SceneHandler for SocialScene {
    fn get_valid_commands(&self) -> Vec<String> {
        let mut cmds = vec![
            "convince".to_string(),
            "intimidate".to_string(),
            "deceive".to_string(),
            "bribe".to_string(),
            "connect".to_string(),
            "ask".to_string(),
            "leave".to_string(),
            "status".to_string(),
            "inventory".to_string(),
            "help".to_string(),
        ];
        if is_healer_npc(&self.npc_data) {
            cmds.push("heal".to_string());
        }
        cmds
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        format!(
            "[Social — {} ({})]",
            self.npc_name,
            disposition_label(self.disposition)
        )
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        // Pending reroll intercept
        if self.pending_reroll.is_some() {
            return match cmd.verb.as_str() {
                "yes" => self.handle_expert_reroll(db),
                "no" => {
                    self.pending_reroll = None;
                    Ok(vec![self.narrate("You decline to use your Expert ability.")])
                }
                _ => Ok(vec![self.narrate(
                    "You have a pending Expert reroll. Answer yes or no.",
                )]),
            };
        }

        match cmd.verb.as_str() {
            "convince" | "intimidate" | "deceive" | "bribe" | "connect" => {
                let verb = cmd.verb.clone();
                self.handle_skill_check(&verb, cmd, db)
            }
            "ask" | "talk" => self.handle_ask(cmd, db),
            "leave" => self.handle_leave(cmd, db),
            "heal" | "use" => self.handle_heal(cmd, db),
            "status" => self.handle_status(cmd, db),
            "inventory" => self.handle_inventory(cmd, db),
            "help" => self.handle_help(cmd, db),
            _ => Ok(vec![self.narrate(format!(
                "\"{}\" is not valid here. Try: convince, intimidate, deceive, bribe, connect, ask, or leave.",
                cmd.raw
            ))]),
        }
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        if self.exit_to_combat {
            return Some(SceneState::Combat);
        }
        if self.exit_to_exploration {
            if self.return_to.as_deref() == Some("town") {
                return Some(SceneState::Town);
            }
            return Some(SceneState::Exploration);
        }
        None
    }

    fn get_suggestions(&self) -> Vec<String> {
        if self.disposition <= -2 {
            vec!["leave".to_string(), "bribe".to_string(), "convince".to_string()]
        } else if self.disposition >= 2 {
            vec![
                "ask".to_string(),
                "convince".to_string(),
                "connect".to_string(),
                "leave".to_string(),
            ]
        } else {
            vec![
                "ask".to_string(),
                "convince".to_string(),
                "intimidate".to_string(),
                "leave".to_string(),
            ]
        }
    }
}

// ---------------------------------------------------------------------------
// Flavor text helpers
// ---------------------------------------------------------------------------

fn success_flavor(verb: &str, outcome_key: &str) -> String {
    let base = match verb {
        "convince" => "Your words find their mark. The NPC seems persuaded.",
        "intimidate" => "Your intimidating presence cows them into compliance.",
        "deceive" => "They believe every word. A flawless performance.",
        "bribe" => "Their eyes light up at the coin. Agreement comes swiftly.",
        "connect" => "You find common ground. A rapport begins to form.",
        _ => "The attempt succeeds.",
    };
    if outcome_key == "exceptional_success" {
        format!("{} Exceptional — they are truly won over.", base)
    } else {
        base.to_string()
    }
}

fn failure_flavor(verb: &str, outcome_key: &str) -> String {
    let base = match verb {
        "convince" => "Your argument fails to land. They remain unconvinced.",
        "intimidate" => "Your bluster falls flat. They are unimpressed.",
        "deceive" => "Something in your manner gives you away.",
        "bribe" => "The offer insults them — or they are simply not for sale.",
        "connect" => "The attempt at connection lands awkwardly.",
        _ => "The attempt fails.",
    };
    if outcome_key == "critical_failure" {
        format!("{} Badly. They are clearly offended.", base)
    } else {
        base.to_string()
    }
}

// ---------------------------------------------------------------------------
// Module-level helpers
// ---------------------------------------------------------------------------

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

fn to_json_object<T: serde::Serialize>(value: &T) -> JsonObject {
    match serde_json::to_value(value) {
        Ok(Value::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use serde_json::json;

    fn make_npc(occupation: &str, disposition: serde_json::Value) -> SceneNpcRecord {
        let mut npc: SceneNpcRecord = serde_json::from_str("{}").unwrap();
        npc.name = "Talbot".to_string();
        npc.occupation = occupation.to_string();
        npc.disposition = disposition;
        npc
    }

    fn make_scene(occupation: &str, disposition: serde_json::Value) -> SocialScene {
        SocialScene::new(
            "npc-1".to_string(),
            "Talbot".to_string(),
            make_npc(occupation, disposition),
            1,
            None,
        )
    }

    fn make_scene_with_return(return_to: Option<&str>) -> SocialScene {
        SocialScene::new(
            "npc-1".to_string(),
            "Talbot".to_string(),
            make_npc("merchant", json!(0)),
            1,
            return_to.map(|s| s.to_string()),
        )
    }

    fn parsed(verb: &str) -> ParsedCommand {
        ParsedCommand::new(verb, verb, vec![], None)
    }

    fn parsed_with_args(verb: &str, args: Vec<&str>) -> ParsedCommand {
        ParsedCommand::new(verb, verb, args.into_iter().map(|s| s.to_string()).collect(), None)
    }

    // 1 — get_valid_commands includes core social verbs
    #[test]
    fn get_valid_commands_includes_social_verbs() {
        let scene = make_scene("merchant", json!(0));
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"convince".to_string()));
        assert!(cmds.contains(&"ask".to_string()));
        assert!(cmds.contains(&"leave".to_string()));
    }

    // 2 — get_prompt shows NPC name and mood word
    #[test]
    fn get_prompt_shows_npc_name_and_mood() {
        let scene = make_scene("merchant", json!(0));
        let db = WorldDatabase::open_in_memory().unwrap();
        let prompt = scene.get_prompt(&db);
        assert!(prompt.contains("Talbot"), "expected NPC name in prompt: {}", prompt);
        assert!(
            prompt.contains("Indifferent"),
            "expected mood label in prompt: {}",
            prompt
        );
    }

    // 3 — help command returns a narrate event
    #[test]
    fn help_command_returns_narrate_event() {
        let scene = make_scene("merchant", json!(0));
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene
            .handle_help(&parsed("help"), &db)
            .expect("help should not error");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "gm.narrate");
    }

    // 4 — leave transitions to Exploration when return_to is None
    #[test]
    fn leave_command_transitions_to_exploration() {
        let mut scene = make_scene_with_return(None);
        let db = WorldDatabase::open_in_memory().unwrap();
        let _ = scene.handle_leave(&parsed("leave"), &db);
        let transition = scene.check_transitions(&[]);
        assert_eq!(transition, Some(SceneState::Exploration));
    }

    // 5 — leave transitions to Town when return_to is "town"
    #[test]
    fn leave_command_transitions_to_town_when_return_to_town() {
        let mut scene = make_scene_with_return(Some("town"));
        let db = WorldDatabase::open_in_memory().unwrap();
        let _ = scene.handle_leave(&parsed("leave"), &db);
        let transition = scene.check_transitions(&[]);
        assert_eq!(transition, Some(SceneState::Town));
    }

    // 6 — ask friendly NPC speaks openly
    #[test]
    fn ask_friendly_npc_speaks_openly() {
        let scene = make_scene("merchant", json!(2));
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = parsed_with_args("ask", vec!["the ruins"]);
        let events = scene
            .handle_ask(&cmd, &db)
            .expect("ask should not error");
        assert!(!events.is_empty());
        let text = events[0]
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(text.contains("openly"), "expected 'openly' in: {}", text);
    }

    // 7 — ask hostile NPC refuses
    #[test]
    fn ask_hostile_npc_refuses() {
        let scene = make_scene("merchant", json!(-2));
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = parsed_with_args("ask", vec!["the ruins"]);
        let events = scene
            .handle_ask(&cmd, &db)
            .expect("ask should not error");
        let text = events[0]
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(text.contains("refuses"), "expected 'refuses' in: {}", text);
    }

    // 8 — unknown command returns guidance narrate
    #[test]
    fn unknown_command_returns_guidance() {
        let mut scene = make_scene("merchant", json!(0));
        let db = WorldDatabase::open_in_memory().unwrap();
        let cmd = ParsedCommand::new("blarg", "blarg", vec![], None);
        let events = scene
            .handle_command(&cmd, &db)
            .expect("unknown cmd should not error");
        assert!(!events.is_empty());
        let text = events[0]
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            text.contains("not valid here"),
            "expected guidance in: {}",
            text
        );
    }

    // 9 — healer NPC valid commands includes heal
    #[test]
    fn healer_npc_valid_commands_includes_heal() {
        let scene = make_scene("healer", json!(0));
        assert!(scene.get_valid_commands().contains(&"heal".to_string()));
    }

    // 10 — non-healer NPC valid commands excludes heal
    #[test]
    fn non_healer_npc_valid_commands_excludes_heal() {
        let scene = make_scene("merchant", json!(0));
        assert!(!scene.get_valid_commands().contains(&"heal".to_string()));
    }

    // 11 — initial state has no transition
    #[test]
    fn check_transitions_none_before_command() {
        let scene = make_scene("merchant", json!(0));
        assert_eq!(scene.check_transitions(&[]), None);
    }
}
