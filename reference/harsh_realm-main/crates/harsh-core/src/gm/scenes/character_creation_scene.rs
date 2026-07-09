//! Character creation scene handler.
//!
//! Ported from `src/harsh_realm/gm/scenes/character_creation*.py` and
//! `_cc_*.py` (9 Python files).

use std::collections::BTreeMap;

use serde_json::json;

use crate::character::Character;
use crate::character_build::{
    apply_kit, auto_pick_skills, auto_pick_summary, class_hit_die, class_skill_points,
    compute_derived_stats, load_kits, load_skill_definitions, load_valid_skills,
    skill_cost, skill_lock_reasons, suggest_skill, SkillDraftContext, ATTRIBUTES,
    CURRENT_MILESTONE, MAX_SKILL_LEVEL, UNTRAINED_SKILL_LEVEL,
    VALID_CLASSES,
};
use crate::character_creation::{EquipmentKit, RecommendedSkill};
use crate::command::ParsedCommand;
use crate::db::WorldDatabase;
use crate::dice::{attr_modifier, DiceRoller};
use crate::events::GameEvent;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::payloads::notices_combat::NarrationNotice;
use crate::repositories::cell::CellRepository;
use crate::repositories::entity::EntityRepository;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn narrate_event(text: impl Into<String>, tick: i64) -> GameEvent {
    let notice = NarrationNotice { text: text.into() };
    let mut data = serde_json::Map::new();
    data.insert("text".to_string(), json!(notice.text));
    data.insert("type".to_string(), json!("narration"));
    GameEvent::new(tick, "gm.narrate", data).with_source("gm")
}

// ---------------------------------------------------------------------------
// Scene state
// ---------------------------------------------------------------------------

/// One step in the creation flow.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Name,
    Class,
    AssignAttr(usize), // index into ATTRIBUTES
    ReviewAttrs,
    Skills,
    Kit,
    Confirm,
}

/// Character creation scene implementing `SceneHandler`.
pub struct CharacterCreationScene {
    step: Step,
    character: Option<Character>,
    rolled_scores: Vec<i32>,
    remaining_scores: Vec<i32>,
    skill_points_remaining: i32,
    kits: BTreeMap<String, EquipmentKit>,
    indexed_kits: Vec<EquipmentKit>,
    tick: i64,
}

impl Default for CharacterCreationScene {
    fn default() -> Self {
        Self::new(0)
    }
}

impl CharacterCreationScene {
    pub fn new(tick: i64) -> Self {
        Self {
            step: Step::Name,
            character: None,
            rolled_scores: vec![],
            remaining_scores: vec![],
            skill_points_remaining: 0,
            kits: BTreeMap::new(),
            indexed_kits: vec![],
            tick,
        }
    }

    /// Return the opening prompts for the scene.
    pub fn start(&self) -> Vec<GameEvent> {
        self.prompt_for_step()
    }

    fn narrate(&self, text: impl Into<String>) -> GameEvent {
        narrate_event(text, self.tick)
    }

    // -----------------------------------------------------------------------
    // Step dispatch
    // -----------------------------------------------------------------------

    fn handle_text(&mut self, text: &str, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let text = text.trim().to_string();
        match self.step.clone() {
            Step::Name => Ok(self.handle_name(&text)),
            Step::Class => Ok(self.handle_class(&text)),
            Step::AssignAttr(_) => {
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() == 6 && parts.iter().all(|p| p.parse::<i32>().is_ok()) {
                    Ok(self.handle_bulk_assign(&parts))
                } else {
                    Ok(self.handle_assign_attr(&text))
                }
            }
            Step::ReviewAttrs => Ok(self.handle_review_attrs(&text)),
            Step::Skills => {
                if text.contains(',') {
                    Ok(self.handle_bulk_skills(&text))
                } else {
                    self.handle_skills(&text)
                }
            }
            Step::Kit => Ok(self.handle_kit(&text)),
            Step::Confirm => self.handle_confirm(&text, db),
        }
    }

    // -----------------------------------------------------------------------
    // Name step
    // -----------------------------------------------------------------------

    fn handle_name(&mut self, text: &str) -> Vec<GameEvent> {
        if text.is_empty() {
            return vec![self.narrate("Please enter a name for your character.")];
        }
        self.character = Some(Character::new(text, ""));
        self.step = Step::Class;
        let mut events = vec![self.narrate(format!("Name set to \"{text}\".",))];
        events.extend(self.prompt_for_step());
        events
    }

    // -----------------------------------------------------------------------
    // Class step
    // -----------------------------------------------------------------------

    fn handle_class(&mut self, text: &str) -> Vec<GameEvent> {
        let chosen = text.to_lowercase();
        if !VALID_CLASSES.contains(&chosen.as_str()) {
            return vec![self.narrate(format!(
                "\"{text}\" is not a valid class. Choose: warrior, expert, or adventurer."
            ))];
        }
        let char = self.character.as_mut().unwrap();
        char.character_class = chosen.clone();
        let mut roller = DiceRoller::new(rand::thread_rng());
        let scores = roller.roll_attribute_set();
        let score_display: String = scores
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("  ");
        self.rolled_scores = scores.clone();
        self.remaining_scores = scores;
        self.step = Step::AssignAttr(0);
        let mut events = vec![
            self.narrate(format!("Class set to {}.", capitalize(&chosen))),
            self.narrate(format!(
                "Rolled attribute scores: {score_display}\nYou will assign them in order: STR, DEX, CON, INT, WIS, CHA."
            )),
        ];
        events.extend(self.prompt_for_step());
        events
    }

    // -----------------------------------------------------------------------
    // Attribute assignment
    // -----------------------------------------------------------------------

    fn attr_index(&self) -> usize {
        if let Step::AssignAttr(i) = &self.step {
            *i
        } else {
            0
        }
    }

    fn handle_assign_attr(&mut self, text: &str) -> Vec<GameEvent> {
        let idx = self.attr_index();
        let attr_name = ATTRIBUTES[idx];
        let remaining_display = self
            .remaining_scores
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("  ");
        let text_lc = text.to_lowercase();

        if text_lc == "undo" || text_lc == "back" {
            if idx == 0 {
                return vec![self.narrate("Nothing to undo — no attributes assigned yet.")];
            }
            let prev_idx = idx - 1;
            let prev_attr = ATTRIBUTES[prev_idx];
            let char = self.character.as_mut().unwrap();
            if let Some(prev_score) = char.attributes.remove(prev_attr) {
                char.attr_mods.remove(prev_attr);
                self.remaining_scores.push(prev_score);
            }
            self.step = Step::AssignAttr(prev_idx);
            let msg = format!("Undone: {} cleared.", prev_attr.to_uppercase());
            let mut events = vec![self.narrate(msg)];
            events.extend(self.prompt_for_step());
            return events;
        }

        let chosen: i32 = match text.parse() {
            Ok(v) => v,
            Err(_) => {
                return vec![self.narrate(format!(
                    "Enter a number from your remaining scores: {remaining_display}\n(Type 'undo' to revisit the previous attribute.)"
                ))];
            }
        };

        if !self.remaining_scores.contains(&chosen) {
            return vec![self.narrate(format!(
                "{chosen} is not in your remaining scores: {remaining_display}"
            ))];
        }

        let char = self.character.as_mut().unwrap();
        char.attributes.insert(attr_name.to_string(), chosen);
        char.attr_mods
            .insert(attr_name.to_string(), attr_modifier(chosen));
        let pos = self
            .remaining_scores
            .iter()
            .position(|&s| s == chosen)
            .unwrap();
        self.remaining_scores.remove(pos);
        let new_idx = idx + 1;

        let mut events = vec![self.narrate(format!(
            "{} set to {chosen} (modifier {:+}).",
            attr_name.to_uppercase(),
            attr_modifier(chosen)
        ))];
        if new_idx < ATTRIBUTES.len() {
            self.step = Step::AssignAttr(new_idx);
        } else {
            self.step = Step::ReviewAttrs;
        }
        events.extend(self.prompt_for_step());
        events
    }

    fn handle_bulk_assign(&mut self, parts: &[&str]) -> Vec<GameEvent> {
        let scores: Vec<i32> = match parts
            .iter()
            .map(|p| p.parse::<i32>())
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(v) => v,
            Err(_) => return vec![self.narrate("Invalid scores. Please enter 6 numbers.")],
        };
        let mut rolled_sorted = self.rolled_scores.clone();
        rolled_sorted.sort_unstable();
        let mut input_sorted = scores.clone();
        input_sorted.sort_unstable();
        if rolled_sorted != input_sorted {
            let rolled_display = self
                .rolled_scores
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join("  ");
            return vec![self.narrate(format!(
                "Scores do not match your rolls: {rolled_display}"
            ))];
        }
        let char = self.character.as_mut().unwrap();
        for (i, attr) in ATTRIBUTES.iter().enumerate() {
            char.attributes.insert(attr.to_string(), scores[i]);
            char.attr_mods
                .insert(attr.to_string(), attr_modifier(scores[i]));
        }
        self.remaining_scores.clear();
        self.step = Step::ReviewAttrs;
        let mut events = vec![self.narrate("All attributes assigned.")];
        events.extend(self.prompt_for_step());
        events
    }

    fn handle_review_attrs(&mut self, text: &str) -> Vec<GameEvent> {
        let lower = text.trim().to_lowercase();
        let parts: Vec<&str> = lower.split_whitespace().collect();
        if parts.is_empty() {
            return self.prompt_for_step();
        }
        match parts[0] {
            "done" | "ok" | "accept" | "continue" => {
                self.finalise_derived_stats();
                self.step = Step::Skills;
                self.prompt_for_step()
            }
            "reassign" => {
                self.remaining_scores = self.rolled_scores.clone();
                let char = self.character.as_mut().unwrap();
                char.attributes.clear();
                char.attr_mods.clear();
                self.step = Step::AssignAttr(0);
                let mut events = vec![self.narrate("Reassigning attributes.")];
                events.extend(self.prompt_for_step());
                events
            }
            "swap" => {
                if parts.len() != 3 {
                    return vec![self
                        .narrate("Usage: swap <attr1> <attr2>  (e.g. 'swap str dex')")];
                }
                let (first, second) = (parts[1].to_string(), parts[2].to_string());
                let attrs_list: Vec<&str> = ATTRIBUTES.to_vec();
                if !attrs_list.contains(&first.as_str()) || !attrs_list.contains(&second.as_str())
                {
                    return vec![self.narrate(format!(
                        "Unknown attribute. Valid: {}",
                        ATTRIBUTES.join(", ")
                    ))];
                }
                if first == second {
                    return vec![self.narrate("Pick two different attributes.")];
                }
                let char = self.character.as_mut().unwrap();
                let va = char.attributes[&first];
                let vb = char.attributes[&second];
                *char.attributes.get_mut(&first).unwrap() = vb;
                *char.attributes.get_mut(&second).unwrap() = va;
                char.attr_mods.insert(first.clone(), attr_modifier(vb));
                char.attr_mods.insert(second.clone(), attr_modifier(va));
                let mut events = vec![self.narrate(format!(
                    "Swapped: {} is now {vb} ({:+}), {} is now {va} ({:+}).",
                    first.to_uppercase(),
                    attr_modifier(vb),
                    second.to_uppercase(),
                    attr_modifier(va)
                ))];
                events.extend(self.prompt_for_step());
                events
            }
            _ => vec![self
                .narrate("Enter 'swap <attr1> <attr2>' to swap scores, or 'done' to continue.")],
        }
    }

    // -----------------------------------------------------------------------
    // Derived stats
    // -----------------------------------------------------------------------

    fn finalise_derived_stats(&mut self) {
        let char = self.character.as_mut().unwrap();
        let cls = char.character_class.clone();
        let hit_die = class_hit_die(&cls);
        let mut roller = DiceRoller::new(rand::thread_rng());
        let hp_roll = roller.roll(1, hit_die as u32, 0).map(|r| r.final_total).unwrap_or(1);
        compute_derived_stats(char, hp_roll);
        char.skills.clear();
        self.skill_points_remaining = class_skill_points(&cls);
    }

    // -----------------------------------------------------------------------
    // Skills step
    // -----------------------------------------------------------------------

    fn handle_skills(&mut self, text: &str) -> Result<Vec<GameEvent>, String> {
        let text_lc = text.trim().to_lowercase();
        if text_lc == "done" {
            self.kits = load_kits();
            self.step = Step::Kit;
            return Ok(self.prompt_for_step());
        }
        if text_lc == "auto" || text_lc == "autopick" || text_lc == "auto-pick" {
            return Ok(self.auto_pick_skills_action());
        }
        let parts: Vec<&str> = text_lc.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(vec![self
                .narrate("Enter a skill name to spend points, or 'done' to continue.")]);
        }
        let skill_id = parts[0].to_string();
        let valid_skills = load_valid_skills();
        if !valid_skills.is_empty() && !valid_skills.contains_key(&skill_id) {
            let suggestion = suggest_skill(&skill_id, &valid_skills);
            let mut message = format!("Unknown skill '{skill_id}'.");
            if let Some(sug) = suggestion {
                message.push_str(&format!(" Did you mean '{sug}'?"));
            }
            let mut sorted_keys: Vec<&str> = valid_skills.keys().map(|s| s.as_str()).collect();
            sorted_keys.sort_unstable();
            message.push_str(&format!("\nValid skills: {}", sorted_keys.join(", ")));
            return Ok(vec![self.narrate(message)]);
        }

        let target_level: Option<i32> = if parts.len() >= 2 {
            match parts[1].parse::<i32>() {
                Ok(v) => Some(v),
                Err(_) => {
                    return Ok(vec![self.narrate(format!(
                        "Invalid level '{}'. Use a number 0–4.",
                        parts[1]
                    ))])
                }
            }
        } else {
            None
        };

        let char = self.character.as_mut().unwrap();
        let current = char.skills.get(&skill_id).copied().unwrap_or(-1);
        let desired = target_level.unwrap_or(current + 1);

        if desired > MAX_SKILL_LEVEL {
            return Ok(vec![self.narrate("Maximum skill level is 4.")]);
        }
        if desired < -1 {
            return Ok(vec![self.narrate(
                "Minimum skill level is -1 (untrained). Use -1 to remove a skill.",
            )]);
        }
        if desired == current {
            return Ok(vec![self.narrate(format!(
                "{skill_id} is already at level {current}."
            ))]);
        }

        if desired < current {
            let refund = current - desired;
            if desired == UNTRAINED_SKILL_LEVEL {
                char.skills.remove(&skill_id);
            } else {
                char.skills.insert(skill_id.clone(), desired);
            }
            self.skill_points_remaining += refund;
            let label = if desired == UNTRAINED_SKILL_LEVEL {
                "removed (untrained)".to_string()
            } else {
                format!("lowered to {desired}")
            };
            return Ok(vec![self.narrate(format!(
                "{} {label}. {refund} point(s) refunded. {} remaining.",
                capitalize(&skill_id),
                self.skill_points_remaining
            ))]);
        }

        // Check lock
        let lock = self.skill_lock_reasons_for(&skill_id);
        if !lock.is_empty() {
            return Ok(vec![self.narrate(format!(
                "Cannot take {skill_id}: {}",
                lock[0]
            ))]);
        }

        let cost = desired - current;
        if cost > self.skill_points_remaining {
            return Ok(vec![self.narrate(format!(
                "Raising {skill_id} to {desired} costs {cost} point(s), but you only have {}.",
                self.skill_points_remaining
            ))]);
        }

        let char = self.character.as_mut().unwrap();
        char.skills.insert(skill_id.clone(), desired);
        self.skill_points_remaining -= cost;
        Ok(vec![self.narrate(format!(
            "{} raised to {desired}. {} point(s) remaining.",
            capitalize(&skill_id),
            self.skill_points_remaining
        ))])
    }

    fn skill_lock_reasons_for(&self, skill_id: &str) -> Vec<String> {
        let char = self.character.as_ref().unwrap();
        let defs = load_skill_definitions();
        let def = match defs.get(skill_id) {
            Some(d) => d.clone(),
            None => return vec![],
        };
        let ctx = SkillDraftContext {
            character_class: char.character_class.clone(),
            attributes: char.attributes.clone(),
            current_milestone: CURRENT_MILESTONE,
            traits: vec![],
        };
        skill_lock_reasons(def.available_milestone, &def.requirements, &ctx)
    }

    fn auto_pick_skills_action(&mut self) -> Vec<GameEvent> {
        let char = self.character.as_mut().unwrap();
        let cls = char.character_class.clone();
        let defs = load_skill_definitions();
        let recs = load_class_recommendations(&cls);
        let picked = auto_pick_skills(
            &cls,
            &char.attributes.clone(),
            &defs,
            &recs,
            CURRENT_MILESTONE,
        );
        char.skills = picked.clone();
        let spent: i32 = picked.values().map(|&l| skill_cost(l)).sum();
        self.skill_points_remaining = class_skill_points(&cls) - spent;
        let summary = auto_pick_summary(&picked, &defs);
        vec![self.narrate(format!(
            "{summary} {} point(s) remaining. Adjust as you like, or type 'done'.",
            self.skill_points_remaining
        ))]
    }

    fn handle_bulk_skills(&mut self, text: &str) -> Vec<GameEvent> {
        let parts: Vec<&str> = text.split(',').map(|p| p.trim()).collect();
        let cls = self.character.as_ref().unwrap().character_class.clone();
        let total_points = class_skill_points(&cls);
        let backup_skills = self.character.as_ref().unwrap().skills.clone();
        let backup_points = self.skill_points_remaining;

        self.character.as_mut().unwrap().skills.clear();
        self.skill_points_remaining = total_points;

        let mut all_events = vec![];
        for part in parts {
            if part.is_empty() {
                continue;
            }
            match self.handle_skills(part) {
                Ok(events) => {
                    let failed = events.iter().any(|e| {
                        let t = e.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                        t.contains("costs")
                            || t.contains("Invalid")
                            || t.contains("Cannot take")
                    });
                    if failed {
                        self.character.as_mut().unwrap().skills = backup_skills;
                        self.skill_points_remaining = backup_points;
                        return events;
                    }
                    all_events.extend(events);
                }
                Err(e) => {
                    self.character.as_mut().unwrap().skills = backup_skills;
                    self.skill_points_remaining = backup_points;
                    return vec![self.narrate(e)];
                }
            }
        }
        let mut result = vec![self.narrate("All skills assigned.")];
        result.extend(all_events);
        result
    }

    // -----------------------------------------------------------------------
    // Kit step
    // -----------------------------------------------------------------------

    fn handle_kit(&mut self, text: &str) -> Vec<GameEvent> {
        let choice = text.trim().to_lowercase();
        let cls = self.character.as_ref().unwrap().character_class.clone();
        if self.indexed_kits.is_empty() {
            let filtered: Vec<EquipmentKit> = self
                .kits
                .values()
                .filter(|k| k.character_class == cls)
                .cloned()
                .collect();
            self.indexed_kits = if filtered.is_empty() {
                self.kits.values().cloned().collect()
            } else {
                filtered
            };
        }

        let kit: Option<EquipmentKit> = if let Ok(idx) = choice.parse::<usize>() {
            if idx >= 1 && idx <= self.indexed_kits.len() {
                Some(self.indexed_kits[idx - 1].clone())
            } else {
                None
            }
        } else {
            self.kits.get(&choice).cloned().or_else(|| {
                self.kits
                    .values()
                    .find(|k| k.name.to_lowercase() == choice)
                    .cloned()
            })
        };

        let kit = match kit {
            Some(k) => k,
            None => {
                let names: String = self
                    .indexed_kits
                    .iter()
                    .enumerate()
                    .map(|(i, k)| format!("[{}] {}", i + 1, k.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                return vec![
                    self.narrate(format!("Unknown kit \"{text}\". Options: {names}"))
                ];
            }
        };

        let kit_name = kit.name.clone();
        apply_kit(self.character.as_mut().unwrap(), &kit);
        self.step = Step::Confirm;
        let mut events = vec![self.narrate(format!("Kit \"{kit_name}\" selected."))];
        events.extend(self.prompt_for_step());
        events
    }

    // -----------------------------------------------------------------------
    // Confirm + persist
    // -----------------------------------------------------------------------

    fn handle_confirm(&mut self, text: &str, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let answer = text.trim().to_lowercase();
        if !["yes", "y", "no", "n"].contains(&answer.as_str()) {
            return Ok(vec![
                self.narrate("Type \"yes\" to confirm or \"no\" to start over.")
            ]);
        }
        if answer == "no" || answer == "n" {
            let tick = self.tick;
            *self = Self::new(tick);
            let mut events = vec![self.narrate("Starting over.")];
            events.extend(self.prompt_for_step());
            return Ok(events);
        }
        self.persist_character(db)
    }

    fn persist_character(&mut self, db: &WorldDatabase) -> Result<Vec<GameEvent>, String> {
        let character = self.character.as_mut().unwrap();
        let cell_repo = CellRepository::new(db);
        let entity_repo = EntityRepository::new(db);

        let (start_q, start_r) = cell_repo.find_starting_hex()?;
        character.position_q = start_q as i32;
        character.position_r = start_r as i32;

        cell_repo.mark_explored(start_q, start_r, 1)?;
        let grid = crate::grid::SquareGrid::default();
        cell_repo.reveal_neighbors(start_q, start_r, &grid)?;

        entity_repo.create_character(character)?;

        let char_id = character.id.clone();
        let char_name = character.name.clone();

        let mut created_data = serde_json::Map::new();
        created_data.insert("character_id".to_string(), json!(char_id));

        let mut scene_data = serde_json::Map::new();
        scene_data.insert("to".to_string(), json!("exploration"));

        Ok(vec![
            self.narrate(format!(
                "Character \"{char_name}\" created! Your adventure begins\u{2026}"
            )),
            GameEvent::new(self.tick, "character.created", created_data)
                .with_source("gm"),
            GameEvent::new(self.tick, "gm.scene_change", scene_data)
                .with_source("gm"),
        ])
    }

    // -----------------------------------------------------------------------
    // Prompts
    // -----------------------------------------------------------------------

    fn prompt_for_step(&self) -> Vec<GameEvent> {
        let tick = self.tick;
        match &self.step {
            Step::Name => vec![narrate_event(
                "Welcome to Harsh Realm.\nWhat is your character's name?",
                tick,
            )],
            Step::Class => vec![narrate_event(
                "Choose your class:\n  warrior   — d8 HD, best combat, Veteran's Luck & Killing Blow\n  expert    — d6 HD, 3 skill points, Masterful Expertise & Quick Learner\n  adventurer — d6 HD, hybrid of both\nEnter: warrior / expert / adventurer",
                tick,
            )],
            Step::AssignAttr(idx) => {
                let attr = ATTRIBUTES[*idx];
                let remaining = self
                    .remaining_scores
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join("  ");
                vec![narrate_event(
                    format!(
                        "Assign a score to {}.\nRemaining scores: {remaining}",
                        attr.to_uppercase()
                    ),
                    tick,
                )]
            }
            Step::ReviewAttrs => {
                let char = self.character.as_ref().unwrap();
                let attr_line = ATTRIBUTES
                    .iter()
                    .map(|attr| {
                        let score = char.attributes.get(*attr).copied().unwrap_or(0);
                        let mmod = char.attr_mods.get(*attr).copied().unwrap_or(0);
                        format!("{} {score} ({mmod:+})", attr.to_uppercase())
                    })
                    .collect::<Vec<_>>()
                    .join("  ");
                vec![narrate_event(
                    format!(
                        "Review your attributes:\n  {attr_line}\nType 'swap <attr1> <attr2>' to swap two scores, 'reassign' to start over, or 'done' to continue."
                    ),
                    tick,
                )]
            }
            Step::Skills => vec![narrate_event(
                format!(
                    "Skill allocation — {} point(s) to spend.\nSkills start at -1 (untrained). Raising to 0 costs 1 point; to 1 costs 1 more.\nEnter: <skill_name> [target_level]  — 'auto' to pick for your class — or 'done' to continue.\nExamples: 'stab 1' sets Stab to 1, 'survive' raises Survive by 1.\nTo undo: retype a skill at a lower level (e.g. 'stab 0' or 'stab -1' to remove).",
                    self.skill_points_remaining
                ),
                tick,
            )],
            Step::Kit => {
                let cls = self
                    .character
                    .as_ref()
                    .map(|c| c.character_class.as_str())
                    .unwrap_or("");
                let kits: Vec<&EquipmentKit> = self
                    .kits
                    .values()
                    .filter(|k| k.character_class == cls)
                    .collect();
                let kits: Vec<&EquipmentKit> = if kits.is_empty() {
                    self.kits.values().collect()
                } else {
                    kits
                };
                let kit_list = kits
                    .iter()
                    .enumerate()
                    .map(|(i, k)| format!("  [{}] {} — {}", i + 1, k.name, k.description))
                    .collect::<Vec<_>>()
                    .join("\n");
                vec![narrate_event(
                    format!(
                        "Choose your starting kit:\n{kit_list}\nEnter a number or kit name:"
                    ),
                    tick,
                )]
            }
            Step::Confirm => {
                let char = self.character.as_ref().unwrap();
                let attr_lines = ATTRIBUTES
                    .iter()
                    .map(|attr| {
                        let score = char.attributes.get(*attr).copied().unwrap_or(0);
                        let mmod = char.attr_mods.get(*attr).copied().unwrap_or(0);
                        format!("{} {score} ({mmod:+})", attr.to_uppercase())
                    })
                    .collect::<Vec<_>>()
                    .join("  ");
                let skill_lines = if char.skills.is_empty() {
                    "  (none above -1)".to_string()
                } else {
                    char.skills
                        .iter()
                        .map(|(s, l)| format!("{s}:{l}"))
                        .collect::<Vec<_>>()
                        .join("  ")
                };
                use crate::runtime::InventoryItemRecord;
                use serde_json::Value;
                let equipment = char
                    .equipment
                    .iter()
                    .filter_map(|raw| {
                        serde_json::from_value::<InventoryItemRecord>(Value::Object(raw.clone()))
                            .ok()
                    })
                    .map(|i| i.name)
                    .collect::<Vec<_>>()
                    .join(", ");
                let summary = format!(
                    "--- CHARACTER SUMMARY ---\nName:  {}\nClass: {}\nHP:    {}/{}   AC: {}   Attack: {:+}\nAttrs: {attr_lines}\nSkills: {skill_lines}\nEquipment: {equipment}\nSaves: Physical {} / Evasion {} / Mental {}\n-------------------------\nConfirm? (yes / no)",
                    char.name,
                    capitalize(&char.character_class),
                    char.hp,
                    char.max_hp,
                    char.ac,
                    char.attack_bonus,
                    char.physical_save,
                    char.evasion_save,
                    char.mental_save
                );
                vec![narrate_event(summary, tick)]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Load class recommendations (thin wrapper over character_build)
// ---------------------------------------------------------------------------

fn load_class_recommendations(class: &str) -> Vec<RecommendedSkill> {
    use crate::character_build::load_class_options;
    load_class_options()
        .into_iter()
        .find(|opt| opt.id == class)
        .map(|opt| opt.recommended_skills)
        .unwrap_or_default()
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

// ---------------------------------------------------------------------------
// SceneHandler impl
// ---------------------------------------------------------------------------

impl SceneHandler for CharacterCreationScene {
    fn get_valid_commands(&self) -> Vec<String> {
        vec!["input".to_string()]
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        "> ".to_string()
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        self.handle_text(&cmd.raw, db)
    }

    fn check_transitions(&self, events: &[GameEvent]) -> Option<SceneState> {
        for event in events {
            if event.event_type == "gm.scene_change"
                && event.data.get("to").and_then(|v| v.as_str()) == Some("exploration")
            {
                return Some(SceneState::Exploration);
            }
        }
        None
    }

    fn get_suggestions(&self) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;

    fn db() -> WorldDatabase {
        WorldDatabase::open_in_memory().unwrap()
    }

    fn cmd(raw: &str) -> ParsedCommand {
        ParsedCommand {
            raw: raw.to_string(),
            verb: "input".to_string(),
            args: vec![],
            direction: None,
        }
    }

    #[test]
    fn initial_valid_commands() {
        let scene = CharacterCreationScene::new(0);
        assert!(scene.get_valid_commands().contains(&"input".to_string()));
    }

    #[test]
    fn initial_prompt_is_gt() {
        let scene = CharacterCreationScene::new(0);
        let db = db();
        assert_eq!(scene.get_prompt(&db), "> ");
    }

    #[test]
    fn start_returns_welcome_prompt() {
        let scene = CharacterCreationScene::new(0);
        let events = scene.start();
        assert!(!events.is_empty());
        let text = events[0]
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(text.contains("Harsh Realm") || text.contains("name"));
    }

    #[test]
    fn handle_empty_name_returns_error() {
        let mut scene = CharacterCreationScene::new(0);
        let db = db();
        let events = scene.handle_command(&cmd(""), &db).unwrap();
        assert!(!events.is_empty());
    }

    #[test]
    fn handle_name_sets_class_step() {
        let mut scene = CharacterCreationScene::new(0);
        let db = db();
        let events = scene.handle_command(&cmd("Hero"), &db).unwrap();
        assert!(events
            .iter()
            .any(|e| e.data.get("text").and_then(|v| v.as_str()).unwrap_or("").contains("Hero")));
        assert_eq!(scene.step, Step::Class);
    }

    #[test]
    fn handle_invalid_class_stays_on_class_step() {
        let mut scene = CharacterCreationScene::new(0);
        let db = db();
        let _ = scene.handle_command(&cmd("Hero"), &db);
        let events = scene.handle_command(&cmd("wizard"), &db).unwrap();
        let text = events[0]
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(text.contains("not a valid class"));
        assert_eq!(scene.step, Step::Class);
    }

    #[test]
    fn check_transitions_scene_change() {
        let scene = CharacterCreationScene::new(0);
        let mut data = serde_json::Map::new();
        data.insert("to".to_string(), json!("exploration"));
        let ev = GameEvent::new(0, "gm.scene_change", data).with_source("gm");
        assert_eq!(scene.check_transitions(&[ev]), Some(SceneState::Exploration));
    }

    #[test]
    fn check_transitions_no_match() {
        let scene = CharacterCreationScene::new(0);
        let ev =
            GameEvent::new(0, "gm.narrate", serde_json::Map::new()).with_source("gm");
        assert_eq!(scene.check_transitions(&[ev]), None);
    }

    #[test]
    fn review_attrs_unknown_command() {
        let mut scene = CharacterCreationScene::new(0);
        scene.step = Step::ReviewAttrs;
        let mut char = Character::new("Hero", "warrior");
        for (attr, val) in
            crate::character_build::ATTRIBUTES
                .iter()
                .zip(crate::character_build::STANDARD_ARRAY)
        {
            char.attributes.insert(attr.to_string(), *val);
            char.attr_mods.insert(attr.to_string(), attr_modifier(*val));
        }
        scene.character = Some(char);
        let events = scene.handle_review_attrs("jump");
        let text = events[0]
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(text.contains("swap") || text.contains("done"));
    }
}
