//! Dungeon scene handler.
//!
//! Ported from `src/harsh_realm/gm/scenes/dungeon.py` and its three mixin files
//! (_dungeon_commands_mixin.py, _dungeon_movement_mixin.py, _dungeon_search_mixin.py).
//! All Python classes are collapsed into a single `DungeonScene` struct.

use std::collections::{HashMap, HashSet};

use rand::thread_rng;
use serde_json::Value as JsonValue;

use crate::character::Character;
use crate::command::ParsedCommand;
use crate::damage::parse_damage_expr;
use crate::db::WorldDatabase;
use crate::dice::DiceRoller;
use crate::engine_results::SkillCheckResult;
use crate::events::GameEvent;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::grid::SquareGrid;
use crate::payloads::notices_combat::{CharacterSnapshot, CombatSaveNotice, NarrationNotice};
use crate::saves::resolve_save;
use crate::payloads::notices_world::{ActionSkillCheckNotice, DungeonEnterRoomNotice};
use crate::payloads::requests::CombatTakeDamageRequested;
use crate::payloads::transport::StatusApplyRequested;
use crate::repositories::entity::EntityRepository;
use crate::runtime::{InventoryItemRecord, JsonObject};
use crate::scene_data::{DungeonConnection, DungeonRoom};
use crate::skill_checks::SkillCheckResolver;
use crate::status_effects::repository::StatusEffectRepository;

/// Scene handler for dungeon exploration.
pub struct DungeonScene {
    dungeon_id: String,
    rooms: HashMap<String, DungeonRoom>,
    connections: Vec<DungeonConnection>,
    room_exits: HashMap<String, HashMap<String, String>>,
    entry_q: i32,
    entry_r: i32,
    tick: i64,
    current_room_id: String,
    visited_rooms: HashSet<String>,
    found_hidden_loot: HashSet<String>,
    spotted_traps: HashSet<String>,
    disarmed_traps: HashSet<String>,
    triggered_traps: HashSet<String>,
    room_terrain: HashMap<String, String>,
    pending_exit: bool,
    pending_combat: bool,
}

impl DungeonScene {
    /// Create a new dungeon scene.
    pub fn new(
        dungeon_id: impl Into<String>,
        rooms: Vec<DungeonRoom>,
        connections: Vec<DungeonConnection>,
        entry_q: i32,
        entry_r: i32,
        tick: i64,
    ) -> Self {
        let dungeon_id = dungeon_id.into();
        let room_map: HashMap<String, DungeonRoom> =
            rooms.iter().cloned().map(|r| (r.id.clone(), r)).collect();
        let room_terrain: HashMap<String, String> = room_map
            .values()
            .map(|r| (r.id.clone(), r.r#type.clone()))
            .collect();
        let room_exits = build_room_exits(&connections);

        let entrance_id = room_map
            .values()
            .find(|r| r.r#type == "entrance")
            .map(|r| r.id.clone())
            .or_else(|| rooms.first().map(|r| r.id.clone()))
            .unwrap_or_default();

        let mut visited = HashSet::new();
        visited.insert(entrance_id.clone());

        Self {
            dungeon_id,
            rooms: room_map,
            connections,
            room_exits,
            entry_q,
            entry_r,
            tick,
            current_room_id: entrance_id,
            visited_rooms: visited,
            found_hidden_loot: HashSet::new(),
            spotted_traps: HashSet::new(),
            disarmed_traps: HashSet::new(),
            triggered_traps: HashSet::new(),
            room_terrain,
            pending_exit: false,
            pending_combat: false,
        }
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn narrate(&self, text: impl Into<String>) -> GameEvent {
        let notice = NarrationNotice { text: text.into() };
        let data = to_json_object(&notice);
        GameEvent::new(self.tick, "gm.narrate", data).with_source("dungeon")
    }

    fn get_room_exits(&self, room_id: &str) -> HashMap<String, String> {
        self.room_exits.get(room_id).cloned().unwrap_or_default()
    }

    fn is_direction_blocked(&self, room_id: &str, direction: &str) -> bool {
        let exits = self.get_room_exits(room_id);
        match exits.get(direction) {
            None => true,
            Some(target) => self.room_terrain.get(target).map_or(false, |t| t == "wall"),
        }
    }

    fn find_entrance(&self) -> Option<&DungeonRoom> {
        self.rooms.values().find(|r| r.r#type == "entrance")
    }

    fn is_illuminated(&self, char_id: &str, db: &WorldDatabase) -> bool {
        StatusEffectRepository::new(db)
            .list_for_entity_effect(char_id, "illuminated")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    // ------------------------------------------------------------------
    // Command handlers
    // ------------------------------------------------------------------

    fn handle_look(&self, db: &WorldDatabase) -> Vec<GameEvent> {
        let room = match self.rooms.get(&self.current_room_id) {
            Some(r) => r,
            None => return vec![self.narrate("You are in a void.")],
        };
        let char = EntityRepository::new(db).load_first_character().ok().flatten();
        let is_lit = char.as_ref().map_or(false, |c| self.is_illuminated(&c.id, db));
        let text = if is_lit {
            let desc = room
                .description
                .as_deref()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "A standard area with unremarkable features.");
            let exits: Vec<String> = self.get_room_exits(&self.current_room_id).keys().cloned().collect();
            let exit_str = if exits.is_empty() { "none".to_string() } else { exits.join(", ") };
            format!(
                "--- {} ({}) ---\n{}\nVisible Exits: {}",
                room.name,
                room.r#type.to_uppercase(),
                desc,
                exit_str
            )
        } else {
            format!(
                "--- {} (DARKNESS) ---\nIt is pitch black. You can't see more than a few feet in front of you.\nYou can just barely make out some possible exits.",
                room.name
            )
        };
        vec![self.narrate(text)]
    }

    fn handle_exit(&mut self) -> Vec<GameEvent> {
        let at_entrance = self
            .find_entrance()
            .map_or(true, |e| e.id == self.current_room_id);
        if !at_entrance {
            return vec![self.narrate("You must return to the entrance to leave the dungeon.")];
        }
        self.pending_exit = true;
        vec![self.narrate("You carefully make your way back to the surface.")]
    }

    fn handle_status(&self, db: &WorldDatabase) -> Vec<GameEvent> {
        let room = self.rooms.get(&self.current_room_id);
        let char = EntityRepository::new(db).load_first_character().ok().flatten();
        let is_lit = char.as_ref().map_or(false, |c| self.is_illuminated(&c.id, db));
        let text = format!(
            "Dungeon Exploration: {}\nCurrent Location: {}\nConditions: {}\nProgress: {}/{} rooms visited",
            self.dungeon_id,
            room.map_or("Unknown", |r| r.name.as_str()),
            if is_lit { "Illuminated" } else { "Dark" },
            self.visited_rooms.len(),
            self.rooms.len(),
        );
        vec![self.narrate(text)]
    }

    fn handle_help(&self) -> Vec<GameEvent> {
        vec![self.narrate(
            "Dungeon commands:\n\
             \x20 go <direction>   Move to an adjacent room\n\
             \x20 look / l         Describe current room\n\
             \x20 search           Search for items (uses Notice skill)\n\
             \x20 exit / leave     Leave dungeon (at entrance only)\n\
             \x20 status           Show dungeon progress\n\
             \x20 help             Show this message",
        )]
    }

    fn handle_inventory(&self, db: &WorldDatabase) -> Vec<GameEvent> {
        let char = match EntityRepository::new(db).load_first_character().ok().flatten() {
            Some(c) => c,
            None => return vec![self.narrate("[ERROR] No character found.")],
        };
        if char.equipment.is_empty() {
            return vec![self.narrate("You are carrying nothing.")];
        }
        let mut lines = vec!["--- Inventory ---".to_string()];
        for item in &char.equipment {
            let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            lines.push(format!("  {name}"));
        }
        vec![self.narrate(lines.join("\n"))]
    }

    fn handle_use(&self, cmd: &ParsedCommand, db: &WorldDatabase) -> Vec<GameEvent> {
        let item_name = if cmd.args.is_empty() {
            return vec![self.narrate("Use what? (e.g. 'use torch')")];
        } else {
            cmd.args.join(" ").to_lowercase()
        };
        let mut char = match EntityRepository::new(db).load_first_character().ok().flatten() {
            Some(c) => c,
            None => return vec![self.narrate("[ERROR] No character found.")],
        };
        let item_index = char
            .equipment
            .iter()
            .position(|item| item.get("name").and_then(|v| v.as_str()).map_or(false, |n| n.to_lowercase() == item_name));
        let item_index = match item_index {
            Some(i) => i,
            None => return vec![self.narrate(format!("You aren't carrying a '{item_name}'."))],
        };
        let item_display_name = char.equipment[item_index]
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&item_name)
            .to_string();
        if item_name == "torch" || item_name == "lantern" {
            let apply = StatusApplyRequested {
                entity_id: char.id.clone(),
                effect_id: "illuminated".to_string(),
                duration_ticks: Some(60),
                source: Some("dungeon".to_string()),
                data: JsonObject::new(),
            };
            let mut events = vec![
                self.narrate(format!("You light the {item_display_name}. The darkness recedes.")),
                GameEvent::new(self.tick, "status.apply_requested", to_json_object(&apply))
                    .with_source("dungeon"),
            ];
            if item_name == "torch" {
                char.equipment.remove(item_index);
                let _ = EntityRepository::new(db).save_character_state(&mut char);
            }
            return events;
        }
        vec![self.narrate(format!("You can't use the {item_display_name} that way."))]
    }

    fn handle_move(&mut self, cmd: &ParsedCommand, db: &WorldDatabase) -> Vec<GameEvent> {
        let direction = match cmd.direction.as_deref() {
            Some(d) => d.to_string(),
            None => {
                let exits: Vec<String> = self.get_room_exits(&self.current_room_id).keys().cloned().collect();
                if exits.is_empty() {
                    return vec![self.narrate("There are no exits from this room.")];
                }
                return vec![self.narrate(format!("Which direction? Exits: {}", exits.join(", ")))];
            }
        };
        let adj = self.get_room_exits(&self.current_room_id);
        let target_room_id = match adj.get(&direction) {
            Some(id) => id.clone(),
            None => {
                let exits: Vec<String> = adj.keys().cloned().collect();
                return vec![self.narrate(format!(
                    "You can't go {direction}. Exits: {}.",
                    if exits.is_empty() { "none".to_string() } else { exits.join(", ") }
                ))];
            }
        };
        // Corner-blocking check for diagonals
        if let Some((card_a, card_b)) = SquareGrid::diagonal_cardinals(&direction) {
            let blocked_a = self.is_direction_blocked(&self.current_room_id, card_a);
            let blocked_b = self.is_direction_blocked(&self.current_room_id, card_b);
            if blocked_a && blocked_b {
                return vec![self.narrate(format!(
                    "The walls block diagonal movement to the {direction}."
                ))];
            }
        }
        let first_visit = !self.visited_rooms.contains(&target_room_id);
        self.current_room_id = target_room_id.clone();
        self.visited_rooms.insert(target_room_id.clone());

        let target_room = self.rooms.get(&target_room_id).cloned();
        let room_name = target_room.as_ref().map_or("a room", |r| r.name.as_str()).to_string();
        let room_type = target_room.as_ref().map_or("room", |r| r.r#type.as_str()).to_string();
        let description = target_room
            .as_ref()
            .and_then(|r| r.description.as_deref())
            .unwrap_or("")
            .to_string();

        let mut narration = format!("You move {direction} into {room_name}.");
        if !description.is_empty() {
            narration.push(' ');
            narration.push_str(&description);
        }
        if first_visit {
            narration.push_str(" (first visit)");
        }
        let mut events = vec![self.narrate(narration)];

        // Trap detection & trigger
        if let Some(ref room) = target_room {
            if !room.traps.is_empty() {
                let char = EntityRepository::new(db).load_first_character().ok().flatten();
                if let Some(ref c) = char {
                    let is_lit = self.is_illuminated(&c.id, db);
                    let diff_mod = if is_lit { 0 } else { 4 };
                    let mut resolver = SkillCheckResolver::new(db);
                    let mut spotted = false;
                    for (i, trap) in room.traps.iter().enumerate() {
                        let uid = format!("{}:{i}", room.id);
                        if self.disarmed_traps.contains(&uid) {
                            continue;
                        }
                        let notice_diff = trap.get("notice_diff").and_then(|v| v.as_i64()).unwrap_or(8) as i32;
                        if let Ok(res) = resolver.resolve("notice", c, None, 0, 0, &mut thread_rng()) {
                            if (res.total - diff_mod) >= notice_diff {
                                self.spotted_traps.insert(uid);
                                let trap_type = trap.get("type").and_then(|v| v.as_str()).unwrap_or("trap");
                                events.push(self.narrate(format!("Wait! You spot a hidden {trap_type}!")));
                                spotted = true;
                            }
                        }
                    }
                    if !spotted {
                        let trap_events = self.trigger_traps(c, room);
                        events.extend(trap_events);
                    }
                }
            }
        }

        // dungeon.enter_room event
        let enter_notice = DungeonEnterRoomNotice {
            dungeon_id: self.dungeon_id.clone(),
            room_id: target_room_id.clone(),
            room_name: room_name.clone(),
            room_type,
            direction,
            first_visit,
        };
        events.push(GameEvent::new(self.tick, "dungeon.enter_room", to_json_object(&enter_notice)).with_source("dungeon"));

        // Encounter on first visit
        if first_visit {
            if let Some(ref room) = target_room {
                if let Some(ref enc) = room.encounter {
                    let desc = enc.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    events.push(self.narrate(format!("Something stirs in the darkness! {desc}")));
                }
            }
        }
        events
    }

    fn trigger_traps(&mut self, character: &Character, room: &DungeonRoom) -> Vec<GameEvent> {
        let mut events = Vec::new();
        for (i, trap) in room.traps.iter().enumerate() {
            let uid = format!("{}:{i}", room.id);
            if self.disarmed_traps.contains(&uid) || self.triggered_traps.contains(&uid) {
                continue;
            }
            self.triggered_traps.insert(uid.clone());
            let hazard_type = trap.get("type").and_then(|v| v.as_str()).unwrap_or("pit").to_string();
            events.push(self.narrate(format!("CLICK! You triggered a {hazard_type}!")));

            // Determine whether this trap carries a saving throw. A trap needs
            // only a `save_type` to trigger a save; `avoid_diff`/`save_diff` is
            // an OPTIONAL difficulty modifier. When absent it defaults to 0
            // (standard difficulty = the character's base save target), matching
            // the disarm path which also defaults a missing `avoid_diff`.
            let save_type = trap.get("save_type").and_then(|v| v.as_str());
            // `avoid_diff` (preferred) or `save_diff` is the difficulty modifier
            // added on top of the character's base save target. Positive values
            // make the save harder; negative values make it easier.
            let avoid_diff_val = trap
                .get("avoid_diff")
                .or_else(|| trap.get("save_diff"))
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
                .unwrap_or(0);
            let save_present = save_type.is_some();

            // HR-774: Resolve the save INLINE and gate damage on the result.
            // The old code emitted `action.save_requested` here, but nothing
            // subscribed to that event, so saves were orphaned.  The inline
            // approach matches the HR-773 pattern of rolling damage inline.
            let save_passed = if let Some(st) = save_type {
                let save = resolve_save(character, st, avoid_diff_val, 0, &mut thread_rng());
                let passed = save.passed;
                let notice = CombatSaveNotice {
                    character: character.name.clone(),
                    save_type: save.save_type.clone(),
                    roll: save.roll,
                    modifier: save.modifier,
                    total: save.total,
                    target: save.target,
                    passed: save.passed,
                };
                events.push(
                    GameEvent::new(self.tick, "combat.save", to_json_object(&notice))
                        .with_source("dungeon"),
                );
                if passed {
                    events.push(self.narrate(format!(
                        "You react in time and avoid the {hazard_type}!"
                    )));
                }
                passed
            } else {
                // No save on this trap; treat as failed (damage proceeds).
                false
            };

            // Emit damage only when:
            //   • the trap has a damage expression, AND
            //   • there is no save OR the save was failed.
            // A passed save fully negates the trap damage.
            if let Some(damage_expr) = trap.get("damage").and_then(|v| v.as_str()) {
                if !save_present || !save_passed {
                    // Roll the damage expression now so the handler receives a
                    // concrete i32.  The old code emitted {damage_expr, hazard_id,
                    // hazard_type} which did not match CombatTakeDamageRequested,
                    // causing serde to fail and damage to be silently discarded
                    // (HR-773).
                    let damage = match parse_damage_expr(damage_expr) {
                        Ok((num, sides, flat)) if sides >= 2 => {
                            DiceRoller::new(thread_rng())
                                .roll(num as u32, sides as u32, flat)
                                .map(|r| r.final_total.max(1))
                                .unwrap_or_else(|_| flat.max(1))
                        }
                        Ok((num, _sides, flat)) => {
                            // sides < 2: treat whole expression as a flat value.
                            (num + flat).max(1)
                        }
                        // `parse_damage_expr` requires a 'd'; a bare integer like
                        // "3" errors, so fall back to parsing it as flat damage
                        // before defaulting to 1.
                        Err(_) => damage_expr.trim().parse::<i32>().unwrap_or(1).max(1),
                    };
                    let req = CombatTakeDamageRequested {
                        character_id: character.id.clone(),
                        character_data: character_snapshot(character),
                        damage,
                        source: hazard_type.clone(),
                    };
                    events.push(
                        GameEvent::new(
                            self.tick,
                            "combat.take_damage_requested",
                            to_json_object(&req),
                        )
                        .with_source("dungeon"),
                    );
                }
            }
        }
        events
    }

    fn handle_search(&mut self, db: &WorldDatabase) -> Vec<GameEvent> {
        let room = match self.rooms.get(&self.current_room_id).cloned() {
            Some(r) => r,
            None => return vec![self.narrate("You find nothing in the void.")],
        };
        let mut events = Vec::new();
        let char = EntityRepository::new(db).load_first_character().ok().flatten();
        let is_lit = char.as_ref().map_or(false, |c| self.is_illuminated(&c.id, db));
        let diff_mod = if is_lit { 0 } else { 4 };
        if !is_lit {
            events.push(self.narrate("Searching in the dark is difficult..."));
        }
        // Standard loot (always visible)
        if !room.loot.is_empty() {
            let names: Vec<String> = room.loot.iter().map(|item_data| {
                serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(item_data.clone()))
                    .map(|i| i.name.clone())
                    .unwrap_or_else(|_| "an unknown object".to_string())
            }).collect();
            events.push(self.narrate(format!("You find: {}", names.join(", "))));
        }
        // Hidden loot (Notice check)
        if !room.hidden_loot.is_empty() {
            if let Some(ref c) = char {
                let mut resolver = SkillCheckResolver::new(db);
                if let Ok(result) = resolver.resolve("search", c, None, 0, 0, &mut thread_rng()) {
                    let effective_total = result.total - diff_mod;
                    events.push(self.narrate(result.narration.clone()));
                    // skill check event
                    let check_notice = ActionSkillCheckNotice {
                        verb: "search".to_string(),
                        skill: result.skill.clone(),
                        attribute: result.attribute.clone(),
                        roll: result.roll,
                        total: result.total,
                        difficulty: result.difficulty + diff_mod,
                        margin: effective_total - result.difficulty,
                        success: effective_total >= result.difficulty,
                        outcome_key: result.outcome_key.clone(),
                        disposition_delta: 0,
                        npc_entity_id: None,
                    };
                    let check_data = to_json_object(&check_notice);
                    events.push(GameEvent::new(self.tick, "action.skill_check", check_data).with_source("dungeon"));
                    let mut discovered = Vec::new();
                    for (i, entry) in room.hidden_loot.iter().enumerate() {
                        let uid = format!("{}:{i}", room.id);
                        if self.found_hidden_loot.contains(&uid) {
                            continue;
                        }
                        let difficulty = entry.get("difficulty").and_then(|v| v.as_i64()).unwrap_or(8) as i32;
                        if effective_total >= difficulty {
                            if let Some(item_data) = entry.get("item") {
                                let name = serde_json::from_value::<InventoryItemRecord>(item_data.clone())
                                    .map(|i| i.name.clone())
                                    .unwrap_or_else(|_| "a hidden object".to_string());
                                discovered.push(name);
                                self.found_hidden_loot.insert(uid);
                            }
                        }
                    }
                    if !discovered.is_empty() {
                        events.push(self.narrate(format!("Your eyes spot something hidden: {}", discovered.join(", "))));
                    } else if room.loot.is_empty() {
                        events.push(self.narrate("You search the room but find nothing of interest."));
                    }
                }
            } else {
                events.push(self.narrate("You find nothing else."));
            }
        } else if room.loot.is_empty() {
            events.push(self.narrate("You search the room but find nothing of interest."));
        }
        events
    }

    fn handle_disarm(&mut self, db: &WorldDatabase) -> Vec<GameEvent> {
        let room = match self.rooms.get(&self.current_room_id).cloned() {
            Some(r) => r,
            None => return vec![self.narrate("You find nothing to disarm.")],
        };
        let found = room.traps.iter().enumerate().find(|(i, _)| {
            let uid = format!("{}:{i}", room.id);
            self.spotted_traps.contains(&uid) && !self.disarmed_traps.contains(&uid)
        });
        let (trap_index, trap) = match found {
            Some((i, t)) => (i, t.clone()),
            None => return vec![self.narrate("You don't see any active traps to disarm.")],
        };
        let char = match EntityRepository::new(db).load_first_character().ok().flatten() {
            Some(c) => c,
            None => return vec![self.narrate("[ERROR] No character found.")],
        };
        let uid = format!("{}:{trap_index}", room.id);
        let mut resolver = SkillCheckResolver::new(db);
        let diff = trap.get("avoid_diff").and_then(|v| v.as_i64()).unwrap_or(10) as i32;
        let result = match resolver.resolve("fix", &char, Some(diff), 0, 0, &mut thread_rng()) {
            Ok(r) => r,
            Err(e) => return vec![self.narrate(format!("Skill check error: {e}"))],
        };
        let trap_type = trap.get("type").and_then(|v| v.as_str()).unwrap_or("trap").to_string();
        let damage_str = trap.get("damage").and_then(|v| v.as_str()).unwrap_or("1d6").to_string();
        let mut events = vec![self.narrate(result.narration)];
        if result.success {
            self.disarmed_traps.insert(uid);
            events.push(self.narrate(format!("Success! You safely disarmed the {trap_type}.")));
        } else {
            events.push(self.narrate(format!("Failure! The {trap_type} goes off!")));
            events.push(self.narrate(format!("You take damage from the trap! ({damage_str})")));
            self.disarmed_traps.insert(uid);
        }
        events
    }
}

fn build_room_exits(connections: &[DungeonConnection]) -> HashMap<String, HashMap<String, String>> {
    let mut exits: HashMap<String, HashMap<String, String>> = HashMap::new();
    for c in connections {
        if c.direction.is_empty() {
            continue;
        }
        exits
            .entry(c.from_room.clone())
            .or_default()
            .insert(c.direction.clone(), c.to_room.clone());
        if let Some(rev) = reverse_direction(&c.direction) {
            exits
                .entry(c.to_room.clone())
                .or_default()
                .insert(rev.to_string(), c.from_room.clone());
        }
    }
    exits
}

fn reverse_direction(d: &str) -> Option<&'static str> {
    match d {
        "north" => Some("south"),
        "south" => Some("north"),
        "east" => Some("west"),
        "west" => Some("east"),
        "northeast" => Some("southwest"),
        "southwest" => Some("northeast"),
        "northwest" => Some("southeast"),
        "southeast" => Some("northwest"),
        _ => None,
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

fn to_json_object<T: serde::Serialize>(value: &T) -> JsonObject {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

impl SceneHandler for DungeonScene {
    fn get_valid_commands(&self) -> Vec<String> {
        ["move", "look", "search", "disarm", "exit", "status", "help", "inventory", "use"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        let name = self
            .rooms
            .get(&self.current_room_id)
            .map(|r| r.name.as_str())
            .unwrap_or("Unknown");
        format!("[Dungeon — {name}]")
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let raw_verb = cmd.raw.trim().to_lowercase();
        let raw_verb_first = raw_verb.split_whitespace().next().unwrap_or("");
        let verb = cmd.verb.as_str();
        let events = match verb {
            "move" => self.handle_move(cmd, db),
            "look" | "l" => self.handle_look(db),
            "search" => self.handle_search(db),
            "disarm" => self.handle_disarm(db),
            "use" => self.handle_use(cmd, db),
            "exit" | "leave" => self.handle_exit(),
            "status" => self.handle_status(db),
            "help" => self.handle_help(),
            "inventory" => self.handle_inventory(db),
            _ if raw_verb_first == "exit" || raw_verb_first == "leave" => self.handle_exit(),
            _ => vec![self.narrate(format!(
                "Unknown command \"{}\". Type \"help\" for available commands.",
                cmd.raw
            ))],
        };
        Ok(events)
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        if self.pending_exit {
            return Some(SceneState::Exploration);
        }
        if self.pending_combat {
            return Some(SceneState::Combat);
        }
        None
    }

    fn get_suggestions(&self) -> Vec<String> {
        let mut s: Vec<String> = self
            .get_room_exits(&self.current_room_id)
            .keys()
            .cloned()
            .collect();
        s.push("look".to_string());
        s.push("search".to_string());
        // Suggest disarm if a spotted trap exists
        let has_trap = self.rooms.get(&self.current_room_id).map_or(false, |room| {
            room.traps.iter().enumerate().any(|(i, _)| {
                let uid = format!("{}:{i}", room.id);
                self.spotted_traps.contains(&uid) && !self.disarmed_traps.contains(&uid)
            })
        });
        if has_trap {
            s.push("disarm".to_string());
        }
        if self.find_entrance().map_or(false, |e| e.id == self.current_room_id) {
            s.push("exit".to_string());
        }
        s
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;

    fn make_room(id: &str, name: &str, rtype: &str) -> DungeonRoom {
        DungeonRoom {
            id: id.to_string(),
            name: name.to_string(),
            r#type: rtype.to_string(),
            x: None,
            y: None,
            w: None,
            h: None,
            center_q: None,
            center_r: None,
            editor_pos: None,
            description: None,
            loot: vec![],
            hidden_loot: vec![],
            traps: vec![],
            encounter: None,
            data: JsonObject::new(),
        }
    }

    fn conn(from: &str, to: &str, dir: &str) -> DungeonConnection {
        DungeonConnection {
            from_room: from.to_string(),
            to_room: to.to_string(),
            direction: dir.to_string(),
        }
    }

    fn cmd(verb: &str, raw: &str) -> ParsedCommand {
        ParsedCommand {
            verb: verb.to_string(),
            args: vec![],
            raw: raw.to_string(),
            direction: None,
        }
    }

    fn cmd_dir(verb: &str, raw: &str, direction: &str) -> ParsedCommand {
        ParsedCommand {
            verb: verb.to_string(),
            args: vec![],
            raw: raw.to_string(),
            direction: Some(direction.to_string()),
        }
    }

    fn two_room_dungeon() -> DungeonScene {
        let rooms = vec![
            make_room("entrance", "Entrance Hall", "entrance"),
            make_room("room1", "Dusty Room", "room_floor"),
        ];
        let connections = vec![conn("entrance", "room1", "north")];
        DungeonScene::new("test_dungeon", rooms, connections, 0, 0, 0)
    }

    #[test]
    fn prompt_shows_room_name() {
        let scene = two_room_dungeon();
        let db = WorldDatabase::open_in_memory().unwrap();
        assert!(scene.get_prompt(&db).contains("Entrance Hall"));
    }

    #[test]
    fn get_valid_commands_includes_move_and_look() {
        let scene = two_room_dungeon();
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"move".to_string()));
        assert!(cmds.contains(&"look".to_string()));
    }

    #[test]
    fn help_returns_narrate_event() {
        let mut scene = two_room_dungeon();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("help", "help"), &db).unwrap();
        assert_eq!(events[0].event_type, "gm.narrate");
    }

    #[test]
    fn move_north_updates_current_room() {
        let mut scene = two_room_dungeon();
        let db = WorldDatabase::open_in_memory().unwrap();
        scene.handle_command(&cmd_dir("move", "move north", "north"), &db).unwrap();
        assert_eq!(scene.current_room_id, "room1");
    }

    #[test]
    fn move_invalid_direction_narrates_error() {
        let mut scene = two_room_dungeon();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd_dir("move", "move south", "south"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("can't go south"), "text: {text}");
    }

    #[test]
    fn exit_at_entrance_transitions() {
        let mut scene = two_room_dungeon();
        assert_eq!(scene.current_room_id, "entrance");
        let db = WorldDatabase::open_in_memory().unwrap();
        scene.handle_command(&cmd("exit", "exit"), &db).unwrap();
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Exploration));
    }

    #[test]
    fn exit_not_at_entrance_refused() {
        let mut scene = two_room_dungeon();
        scene.current_room_id = "room1".to_string();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("exit", "exit"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("return to the entrance"), "text: {text}");
        assert_eq!(scene.check_transitions(&[]), None);
    }

    #[test]
    fn visited_rooms_tracked() {
        let mut scene = two_room_dungeon();
        let db = WorldDatabase::open_in_memory().unwrap();
        assert!(scene.visited_rooms.contains("entrance"));
        scene.handle_command(&cmd_dir("move", "move north", "north"), &db).unwrap();
        assert!(scene.visited_rooms.contains("room1"));
    }

    #[test]
    fn corner_blocking_diagonal_when_both_cardinals_are_walls() {
        let rooms = vec![
            make_room("entrance", "Entrance", "entrance"),
            make_room("wall_n", "North Wall", "wall"),
            make_room("wall_e", "East Wall", "wall"),
            make_room("diag", "Diagonal Room", "room_floor"),
        ];
        let connections = vec![
            conn("entrance", "wall_n", "north"),
            conn("entrance", "wall_e", "east"),
            conn("entrance", "diag", "northeast"),
        ];
        let mut scene = DungeonScene::new("test", rooms, connections, 0, 0, 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene
            .handle_command(&cmd_dir("move", "move northeast", "northeast"), &db)
            .unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("walls block diagonal"), "text: {text}");
    }

    #[test]
    fn disarm_no_trap_spotted_message() {
        let mut scene = two_room_dungeon();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("disarm", "disarm"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("don't see any active traps"), "text: {text}");
    }

    #[test]
    fn reverse_direction_sanity() {
        assert_eq!(reverse_direction("north"), Some("south"));
        assert_eq!(reverse_direction("northeast"), Some("southwest"));
        assert_eq!(reverse_direction("unknown"), None);
    }

    #[test]
    fn no_transition_before_exit() {
        let scene = two_room_dungeon();
        assert_eq!(scene.check_transitions(&[]), None);
    }

    #[test]
    fn suggestions_include_exits_and_look() {
        let scene = two_room_dungeon();
        let sugg = scene.get_suggestions();
        assert!(sugg.contains(&"north".to_string()));
        assert!(sugg.contains(&"look".to_string()));
    }

    #[test]
    fn status_returns_dungeon_id() {
        let mut scene = two_room_dungeon();
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("status", "status"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("test_dungeon"), "text: {text}");
    }

    // ------------------------------------------------------------------
    // HR-773 regression tests
    // ------------------------------------------------------------------

    fn make_trap_room(damage_expr: &str) -> DungeonRoom {
        let mut trap = JsonObject::new();
        trap.insert("type".to_string(), JsonValue::String("pit".to_string()));
        trap.insert("damage".to_string(), JsonValue::String(damage_expr.to_string()));
        DungeonRoom {
            id: "room_trap".to_string(),
            name: "Trap Room".to_string(),
            r#type: "room".to_string(),
            x: None,
            y: None,
            w: None,
            h: None,
            center_q: None,
            center_r: None,
            editor_pos: None,
            description: None,
            loot: vec![],
            hidden_loot: vec![],
            traps: vec![trap],
            encounter: None,
            data: JsonObject::new(),
        }
    }

    /// Proves the root cause of HR-773: the old dungeon payload
    /// (`damage_expr`, `hazard_id`, `hazard_type`) cannot be deserialized
    /// into `CombatTakeDamageRequested`, which expects `damage` (i32) and
    /// `character_data`.  When serde returned `Err`, the handler silently
    /// returned `vec![]` — the player took no damage.
    #[test]
    fn old_dungeon_trap_payload_fails_deserialization() {
        let mut data = JsonObject::new();
        data.insert("character_id".to_string(), JsonValue::String("char_1".to_string()));
        data.insert("damage_expr".to_string(), JsonValue::String("1d6".to_string()));
        data.insert("hazard_id".to_string(), JsonValue::String("room_trap:0".to_string()));
        data.insert("hazard_type".to_string(), JsonValue::String("pit".to_string()));
        let result = serde_json::from_value::<crate::payloads::requests::CombatTakeDamageRequested>(
            serde_json::Value::Object(data),
        );
        assert!(
            result.is_err(),
            "HR-773: old dungeon payload must fail deserialization — this was the root cause of the bug"
        );
    }

    /// After the fix, `trigger_traps` must emit `combat.take_damage_requested`
    /// with a payload that deserializes cleanly into `CombatTakeDamageRequested`
    /// and carries a positive rolled-damage value.
    ///
    /// This test **fails before the fix** because the old emit used wrong field
    /// names (`damage_expr`, `hazard_id`, `hazard_type`) and `serde_json::from_value`
    /// returned `Err`, making the handler a no-op.
    #[test]
    fn trigger_traps_emits_valid_take_damage_payload() {
        use crate::character::Character;
        use crate::payloads::requests::CombatTakeDamageRequested;

        let trap_room = make_trap_room("1d6");
        let character = Character {
            id: "char_test".to_string(),
            name: "Tester".to_string(),
            character_class: "warrior".to_string(),
            hp: 10,
            max_hp: 10,
            ..Character::default()
        };

        let rooms = vec![
            make_room("entrance", "Entrance", "entrance"),
            trap_room.clone(),
        ];
        let mut scene = DungeonScene::new("test_dungeon", rooms, vec![], 0, 0, 1);
        let events = scene.trigger_traps(&character, &trap_room);

        let dmg_event = events
            .iter()
            .find(|e| e.event_type == "combat.take_damage_requested")
            .expect("HR-773: trigger_traps must emit combat.take_damage_requested for a damage trap");

        // The emitted payload MUST deserialize into the struct the handler expects.
        let payload = serde_json::from_value::<CombatTakeDamageRequested>(
            serde_json::Value::Object(dmg_event.data.clone()),
        )
        .expect("HR-773: event payload must deserialize into CombatTakeDamageRequested");

        assert_eq!(payload.character_id, "char_test", "character_id must be forwarded");
        assert!(payload.damage >= 1, "rolled damage must be >= 1, got {}", payload.damage);
        assert_eq!(payload.source, "pit", "source must be the hazard type");
        assert_eq!(payload.character_data.hp, 10, "snapshot hp must match character hp");
    }

    /// A trap authored with a bare-integer `damage` like "3" (no 'd') must deal
    /// exactly that flat amount, not fall through to the `1` default.
    #[test]
    fn trigger_traps_handles_bare_integer_damage() {
        use crate::character::Character;
        use crate::payloads::requests::CombatTakeDamageRequested;

        let trap_room = make_trap_room("3");
        let character = Character {
            id: "char_test".to_string(),
            name: "Tester".to_string(),
            character_class: "warrior".to_string(),
            hp: 10,
            max_hp: 10,
            ..Character::default()
        };
        let rooms = vec![make_room("entrance", "Entrance", "entrance"), trap_room.clone()];
        let mut scene = DungeonScene::new("test_dungeon", rooms, vec![], 0, 0, 1);
        let events = scene.trigger_traps(&character, &trap_room);

        let dmg_event = events
            .iter()
            .find(|e| e.event_type == "combat.take_damage_requested")
            .expect("bare-integer damage trap must still emit a take-damage request");
        let payload = serde_json::from_value::<CombatTakeDamageRequested>(
            serde_json::Value::Object(dmg_event.data.clone()),
        )
        .expect("payload must deserialize");
        assert_eq!(payload.damage, 3, "bare-integer \"3\" must deal 3, not the fallback 1");
    }

    // ------------------------------------------------------------------
    // HR-774 regression tests
    // ------------------------------------------------------------------

    /// Build a trap room where the trap has both a damage expression AND a
    /// saving throw.  `save_type` + `avoid_diff` control the save parameters.
    fn make_save_trap_room(damage_expr: &str, save_type: &str, avoid_diff: i32) -> DungeonRoom {
        let mut trap = JsonObject::new();
        trap.insert("type".to_string(), JsonValue::String("dart".to_string()));
        trap.insert("damage".to_string(), JsonValue::String(damage_expr.to_string()));
        trap.insert("save_type".to_string(), JsonValue::String(save_type.to_string()));
        trap.insert("avoid_diff".to_string(), serde_json::Value::Number(avoid_diff.into()));
        DungeonRoom {
            id: "room_save_trap".to_string(),
            name: "Dart Trap Room".to_string(),
            r#type: "room".to_string(),
            x: None,
            y: None,
            w: None,
            h: None,
            center_q: None,
            center_r: None,
            editor_pos: None,
            description: None,
            loot: vec![],
            hidden_loot: vec![],
            traps: vec![trap],
            encounter: None,
            data: JsonObject::new(),
        }
    }

    /// HR-774: When a character's save auto-passes (physical_save = 1 means
    /// the target is 1 + avoid_diff = 1; any d20 roll clears it), the trap
    /// damage must be fully negated.
    ///
    /// **Fails without the fix:** the old code emitted damage unconditionally
    /// before resolving the save (and emitted an orphan `action.save_requested`
    /// that no handler consumed), so `combat.take_damage_requested` would still
    /// be present in the events and the assertion below would fail.
    #[test]
    fn hr774_passed_save_negates_damage() {
        use crate::character::Character;
        use crate::payloads::notices_combat::CombatSaveNotice;

        // physical_save = 1: target = 1 + 0 (avoid_diff) = 1.
        // Any d20 roll (1-20) + modifier (0) = 1-20 >= 1 → always passes.
        let character = Character {
            id: "char_pass".to_string(),
            name: "Lucky".to_string(),
            character_class: "expert".to_string(),
            hp: 10,
            max_hp: 10,
            physical_save: 1,
            ..Character::default()
        };
        let trap_room = make_save_trap_room("1d6", "physical", 0);
        let rooms = vec![
            make_room("entrance", "Entrance", "entrance"),
            trap_room.clone(),
        ];
        let mut scene = DungeonScene::new("test_dungeon", rooms, vec![], 0, 0, 1);
        let events = scene.trigger_traps(&character, &trap_room);

        // A `combat.save` event must be emitted and must have passed = true.
        let save_event = events
            .iter()
            .find(|e| e.event_type == "combat.save")
            .expect("HR-774: trigger_traps must emit combat.save when the trap has a save");
        let save_notice = serde_json::from_value::<CombatSaveNotice>(
            serde_json::Value::Object(save_event.data.clone()),
        )
        .expect("HR-774: combat.save payload must deserialize into CombatSaveNotice");
        assert_eq!(save_notice.character, "Lucky");
        assert_eq!(save_notice.save_type, "physical");
        // Determinism relies on a zero modifier (Character::default has no CON
        // mod or save bonus). Assert it so drift in Character defaults is caught.
        assert_eq!(
            save_notice.modifier, 0,
            "HR-774: determinism assumes modifier == 0 (no CON mod / save bonus)"
        );
        assert!(
            save_notice.passed,
            "HR-774: save must pass when physical_save = 1 (target = 1)"
        );

        // No damage must be emitted when the save passes.
        assert!(
            events.iter().all(|e| e.event_type != "combat.take_damage_requested"),
            "HR-774: a passed save must fully negate trap damage — \
             combat.take_damage_requested must NOT be emitted"
        );
    }

    /// HR-774: When a character's save auto-fails (physical_save = 21 means
    /// the target is 21; d20 max is 20, so `total` can never reach 21), the
    /// trap damage must still be emitted.
    ///
    /// **Fails without the fix:** the old code never emitted `combat.save` at
    /// all (only the orphan `action.save_requested`), so the assertion that
    /// `combat.save` exists would fail.
    #[test]
    fn hr774_failed_save_triggers_damage() {
        use crate::character::Character;
        use crate::payloads::notices_combat::CombatSaveNotice;
        use crate::payloads::requests::CombatTakeDamageRequested;

        // physical_save = 21: target = 21 + 0 (avoid_diff) = 21.
        // d20 max = 20 + modifier (0) = 20 < 21 → always fails.
        let character = Character {
            id: "char_fail".to_string(),
            name: "Unlucky".to_string(),
            character_class: "warrior".to_string(),
            hp: 10,
            max_hp: 10,
            physical_save: 21,
            ..Character::default()
        };
        let trap_room = make_save_trap_room("1d6", "physical", 0);
        let rooms = vec![
            make_room("entrance", "Entrance", "entrance"),
            trap_room.clone(),
        ];
        let mut scene = DungeonScene::new("test_dungeon", rooms, vec![], 0, 0, 1);
        let events = scene.trigger_traps(&character, &trap_room);

        // A `combat.save` event must be emitted and must have passed = false.
        let save_event = events
            .iter()
            .find(|e| e.event_type == "combat.save")
            .expect("HR-774: trigger_traps must emit combat.save when the trap has a save");
        let save_notice = serde_json::from_value::<CombatSaveNotice>(
            serde_json::Value::Object(save_event.data.clone()),
        )
        .expect("HR-774: combat.save payload must deserialize into CombatSaveNotice");
        assert_eq!(save_notice.character, "Unlucky");
        // Determinism relies on a zero modifier (Character::default has no CON
        // mod or save bonus). Assert it so drift in Character defaults is caught.
        assert_eq!(
            save_notice.modifier, 0,
            "HR-774: determinism assumes modifier == 0 (no CON mod / save bonus)"
        );
        assert!(
            !save_notice.passed,
            "HR-774: save must fail when physical_save = 21 (target = 21, d20 max = 20)"
        );

        // Damage must be emitted because the save failed.
        let dmg_event = events
            .iter()
            .find(|e| e.event_type == "combat.take_damage_requested")
            .expect("HR-774: a failed save must still emit combat.take_damage_requested");
        let payload = serde_json::from_value::<CombatTakeDamageRequested>(
            serde_json::Value::Object(dmg_event.data.clone()),
        )
        .expect("HR-774: damage payload must deserialize into CombatTakeDamageRequested");
        assert_eq!(payload.character_id, "char_fail");
        assert!(payload.damage >= 1, "damage must be >= 1, got {}", payload.damage);
        assert_eq!(payload.source, "dart", "source must be the hazard type");
    }

    /// HR-774 (review fix): a trap with a `save_type` but NO `avoid_diff`/
    /// `save_diff` key must still roll a save at standard difficulty (the
    /// character's base save target) and gate damage on it — NOT silently drop
    /// the save and apply damage unconditionally.
    ///
    /// Uses `physical_save = 21` (target = 21, d20 max = 20) so the save always
    /// fails deterministically, letting us assert both that `combat.save` fired
    /// AND that damage still followed.
    #[test]
    fn hr774_save_without_avoid_diff_still_rolls_save() {
        use crate::character::Character;
        use crate::payloads::notices_combat::CombatSaveNotice;
        use crate::payloads::requests::CombatTakeDamageRequested;

        // Trap has a save_type but deliberately no avoid_diff / save_diff key.
        let mut trap = JsonObject::new();
        trap.insert("type".to_string(), JsonValue::String("dart".to_string()));
        trap.insert("damage".to_string(), JsonValue::String("1d6".to_string()));
        trap.insert("save_type".to_string(), JsonValue::String("physical".to_string()));
        let trap_room = DungeonRoom {
            id: "room_nodiff_trap".to_string(),
            name: "Undefined-Difficulty Trap Room".to_string(),
            r#type: "room".to_string(),
            x: None,
            y: None,
            w: None,
            h: None,
            center_q: None,
            center_r: None,
            editor_pos: None,
            description: None,
            loot: vec![],
            hidden_loot: vec![],
            traps: vec![trap],
            encounter: None,
            data: JsonObject::new(),
        };

        // physical_save = 21 → target = 21 + 0 (defaulted avoid_diff) = 21 →
        // always fails, so we can assert damage follows the save.
        let character = Character {
            id: "char_nodiff".to_string(),
            name: "Standard".to_string(),
            character_class: "warrior".to_string(),
            hp: 10,
            max_hp: 10,
            physical_save: 21,
            ..Character::default()
        };
        let rooms = vec![
            make_room("entrance", "Entrance", "entrance"),
            trap_room.clone(),
        ];
        let mut scene = DungeonScene::new("test_dungeon", rooms, vec![], 0, 0, 1);
        let events = scene.trigger_traps(&character, &trap_room);

        // A `combat.save` event MUST be emitted even without avoid_diff.
        let save_event = events
            .iter()
            .find(|e| e.event_type == "combat.save")
            .expect(
                "HR-774: a trap with save_type but no avoid_diff must still emit combat.save",
            );
        let save_notice = serde_json::from_value::<CombatSaveNotice>(
            serde_json::Value::Object(save_event.data.clone()),
        )
        .expect("HR-774: combat.save payload must deserialize into CombatSaveNotice");
        assert_eq!(save_notice.save_type, "physical");
        assert_eq!(
            save_notice.target, 21,
            "HR-774: missing avoid_diff must default to 0 → target == base save (21)"
        );
        assert!(!save_notice.passed, "save must fail with target 21");

        // Damage must still be gated on (and here follow) the save.
        let dmg_event = events
            .iter()
            .find(|e| e.event_type == "combat.take_damage_requested")
            .expect("HR-774: a failed save must still emit combat.take_damage_requested");
        let payload = serde_json::from_value::<CombatTakeDamageRequested>(
            serde_json::Value::Object(dmg_event.data.clone()),
        )
        .expect("HR-774: damage payload must deserialize into CombatTakeDamageRequested");
        assert_eq!(payload.character_id, "char_nodiff");
        assert!(payload.damage >= 1, "damage must be >= 1, got {}", payload.damage);
    }
}
