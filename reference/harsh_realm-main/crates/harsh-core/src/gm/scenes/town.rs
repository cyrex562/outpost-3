//! Town scene handler.
//!
//! Ported from `src/harsh_realm/gm/scenes/town.py` and its three mixin files
//! (_town_commands_mixin.py, _town_interaction_mixin.py, _town_movement_mixin.py).
//! All Python classes are collapsed into a single `TownScene` struct.

use std::collections::HashMap;

use crate::command::ParsedCommand;
use crate::db::WorldDatabase;
use crate::events::GameEvent;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::gm::scenes::time_support::{format_time, get_time_description};
use crate::gm::scenes::town_support::is_impassable_town;
use crate::grid::{Grid, GridCoord, SquareGrid};
use crate::payloads::contexts::{ShoppingSceneContext, SocialSceneContext};
use crate::payloads::notices_combat::NarrationNotice;
use crate::payloads::notices_world::{GmSuggestionsNotice, TownMapNotice, TownMoveNotice};
use crate::repositories::entity::EntityRepository;
use crate::runtime::{JsonObject, JsonValue, SceneNpcRecord};
use crate::scene_data::{SettlementBuilding, TownCell};
use crate::settlement::SettlementSummary;
use crate::weather::WeatherService;

/// Scene handler for town exploration on a square grid.
pub struct TownScene {
    settlement: SettlementSummary,
    q: i32,
    r: i32,
    tick: i64,
    terrain: HashMap<(i32, i32), String>,
    town_cells: Vec<TownCell>,
    buildings: Vec<SettlementBuilding>,
    building_map: HashMap<(i32, i32), usize>,
    player_q: i32,
    player_r: i32,
    grid: SquareGrid,
    pending_leave: bool,
    pub pending_social_scene: Option<SocialSceneContext>,
    pub pending_social_transition: bool,
    pub pending_shopping_scene: Option<ShoppingSceneContext>,
    pub pending_shopping_transition: bool,
}

impl TownScene {
    /// Create a new town scene.
    pub fn new(
        settlement: SettlementSummary,
        q: i32,
        r: i32,
        town_cells: Vec<TownCell>,
        tick: i64,
    ) -> Self {
        let terrain: HashMap<(i32, i32), String> = town_cells
            .iter()
            .map(|c| ((c.q, c.r), c.terrain.clone()))
            .collect();

        let (player_q, player_r) = if !town_cells.is_empty() {
            let min_q = town_cells.iter().map(|c| c.q).min().unwrap_or(0);
            let max_q = town_cells.iter().map(|c| c.q).max().unwrap_or(0);
            let min_r = town_cells.iter().map(|c| c.r).min().unwrap_or(0);
            let max_r = town_cells.iter().map(|c| c.r).max().unwrap_or(0);
            ((min_q + max_q) / 2, (min_r + max_r) / 2)
        } else {
            (0, 0)
        };

        let mut buildings: Vec<SettlementBuilding> = Vec::new();
        let mut building_map: HashMap<(i32, i32), usize> = HashMap::new();
        for b in &settlement.establishments {
            let idx = buildings.len();
            if let (Some(bq), Some(br), Some(bw), Some(bh)) = (b.q, b.r, b.w, b.h) {
                for dq in 0..bw {
                    for dr in 0..bh {
                        building_map.insert((bq + dq, br + dr), idx);
                    }
                }
            }
            buildings.push(b.clone());
        }

        Self {
            settlement,
            q,
            r,
            tick,
            terrain,
            town_cells,
            buildings,
            building_map,
            player_q,
            player_r,
            grid: SquareGrid::new(),
            pending_leave: false,
            pending_social_scene: None,
            pending_social_transition: false,
            pending_shopping_scene: None,
            pending_shopping_transition: false,
        }
    }

    /// Take and clear the pending social context (for controller wiring).
    pub fn take_pending_social(&mut self) -> Option<crate::payloads::contexts::SocialSceneContext> {
        self.pending_social_scene.take()
    }

    /// Take and clear the pending shopping context (for controller wiring).
    pub fn take_pending_shopping(
        &mut self,
    ) -> Option<crate::payloads::contexts::ShoppingSceneContext> {
        self.pending_shopping_scene.take()
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn narrate(&self, text: impl Into<String>) -> GameEvent {
        let notice = NarrationNotice { text: text.into() };
        let data = to_json_object(&notice);
        GameEvent::new(self.tick, "gm.narrate", data).with_source("town")
    }

    fn describe_location(&self) -> String {
        let pos = (self.player_q, self.player_r);
        if let Some(&idx) = self.building_map.get(&pos) {
            if let Some(b) = self.buildings.get(idx) {
                let btype = b
                    .r#type
                    .as_deref()
                    .unwrap_or("building")
                    .replace('_', " ");
                let btype_cap = capitalize(&btype);
                return format!("You are in {} ({}).", b.name, btype_cap);
            }
        }
        let terrain = self.terrain.get(&pos).map_or("open_ground", |s| s.as_str());
        match terrain {
            "road" => "You're on a dusty road.".to_string(),
            "plaza" => "You stand in the town plaza.".to_string(),
            "open_ground" => "You're in an open area.".to_string(),
            "shop" => "A shop stands before you.".to_string(),
            "tavern" => "The warm glow of a tavern beckons.".to_string(),
            "temple" => "A temple's spires rise above.".to_string(),
            "building" => "A building looms nearby.".to_string(),
            _ => "You look around.".to_string(),
        }
    }

    fn building_at_pos(&self) -> Option<&SettlementBuilding> {
        self.building_map
            .get(&(self.player_q, self.player_r))
            .and_then(|&idx| self.buildings.get(idx))
    }

    fn get_town_map_event(&self) -> GameEvent {
        let cells: Vec<JsonObject> = self
            .town_cells
            .iter()
            .filter_map(|c| {
                serde_json::to_value(c)
                    .ok()
                    .and_then(|v| if let JsonValue::Object(m) = v { Some(m) } else { None })
            })
            .collect();
        let buildings: Vec<JsonObject> = self
            .buildings
            .iter()
            .filter_map(|b| {
                serde_json::to_value(b)
                    .ok()
                    .and_then(|v| if let JsonValue::Object(m) = v { Some(m) } else { None })
            })
            .collect();
        let notice = TownMapNotice {
            cells,
            buildings,
            player_q: self.player_q,
            player_r: self.player_r,
            settlement_name: self.settlement.name.clone(),
        };
        GameEvent::new(self.tick, "town.map", to_json_object(&notice)).with_source("town")
    }

    // ------------------------------------------------------------------
    // Command handlers
    // ------------------------------------------------------------------

    fn handle_move(&mut self, cmd: &ParsedCommand, _db: &WorldDatabase) -> Vec<GameEvent> {
        let direction = match cmd.direction.as_deref() {
            Some(d) => d.to_string(),
            None => {
                let dirs = self.grid.directions().join(", ");
                return vec![self.narrate(format!("Which direction? ({dirs})"))];
            }
        };
        if !self.grid.directions().contains(&direction.as_str()) {
            return vec![self.narrate(format!("\"{direction}\" is not a valid direction."))];
        }
        let origin = GridCoord { q: self.player_q, r: self.player_r };
        let target = match self.grid.neighbor(origin, &direction) {
            Ok(t) => t,
            Err(_) => return vec![self.narrate("That direction is blocked.")],
        };
        let tq = target.q as i32;
        let tr = target.r as i32;
        // Check bounds
        if !self.terrain.is_empty() && !self.terrain.contains_key(&(tq, tr)) {
            return vec![self.narrate("You've reached the edge of town. Use 'leave' to exit.")];
        }
        let target_terrain = self.terrain.get(&(tq, tr)).map_or("open_ground", |s| s.as_str()).to_string();
        if is_impassable_town(&target_terrain) {
            return vec![self.narrate("A building blocks your path. You can't go that way.")];
        }
        // Corner-blocking for diagonals
        if let Some((card_a, card_b)) = SquareGrid::diagonal_cardinals(&direction) {
            let na = self.grid.neighbor(origin, card_a).unwrap_or(origin);
            let nb = self.grid.neighbor(origin, card_b).unwrap_or(origin);
            let ta = self.terrain.get(&(na.q as i32, na.r as i32)).map_or("open_ground", |s| s.as_str());
            let tb = self.terrain.get(&(nb.q as i32, nb.r as i32)).map_or("open_ground", |s| s.as_str());
            if is_impassable_town(ta) && is_impassable_town(tb) {
                return vec![self.narrate("Buildings block your diagonal path.")];
            }
        }
        self.player_q = tq;
        self.player_r = tr;
        let mut events = Vec::new();
        let desc = self.describe_location();
        let mut narration = format!("You walk {direction}. {desc}");
        let at_shop = target_terrain == "shop" || target_terrain == "tavern" || target_terrain == "temple"
            || self.building_map.contains_key(&(self.player_q, self.player_r));
        if at_shop {
            narration.push_str(" Type 'shop' to browse wares or interact.");
            let suggestions = GmSuggestionsNotice {
                commands: vec!["shop".to_string(), "look".to_string(), "talk".to_string()],
            };
            events.push(GameEvent::new(self.tick, "gm.suggestions", to_json_object(&suggestions)).with_source("town"));
        }
        events.push(self.narrate(narration));
        let move_notice = TownMoveNotice {
            direction: direction.clone(),
            player_q: self.player_q,
            player_r: self.player_r,
            terrain: target_terrain,
        };
        events.push(GameEvent::new(self.tick, "town.move", to_json_object(&move_notice)).with_source("town"));
        events
    }

    fn handle_look(&self, db: &WorldDatabase) -> Vec<GameEvent> {
        let name = &self.settlement.name;
        let desc = self.describe_location();
        let npcs = EntityRepository::new(db)
            .list_npcs_at_location(self.q as i64, self.r as i64)
            .unwrap_or_default();
        let filtered = self.filter_npcs_for_position(&npcs);
        let seed = db.get_meta("seed").ok().flatten().unwrap_or_else(|| "default".to_string());
        let weather_svc = WeatherService::new(&seed);
        let weather = weather_svc.at(self.q as i64, self.r as i64, self.tick, Some("settlement"));
        let weather_desc = format!("{} It is {}.", weather.description, weather.temperature);
        let mut text = format!("--- {name} ---\n{desc}\n{weather_desc}");
        if !filtered.is_empty() {
            let npc_names: Vec<&str> = filtered.iter().map(|n| n.name.as_str()).collect();
            text.push_str(&format!("\nPeople here: {}", npc_names.join(", ")));
        }
        vec![self.narrate(text), self.get_town_map_event()]
    }

    fn handle_talk(&mut self, cmd: &ParsedCommand, db: &WorldDatabase) -> Vec<GameEvent> {
        let mut args = cmd.args.clone();
        if args.first().map_or(false, |a| a.to_lowercase() == "to") && args.len() > 1 {
            args.remove(0);
        }
        let name_query = args.join(" ").to_lowercase();
        let npcs = EntityRepository::new(db)
            .list_npcs_at_location(self.q as i64, self.r as i64)
            .unwrap_or_default();
        if npcs.is_empty() {
            return vec![self.narrate("There's no one here to talk to.")];
        }
        let candidates = self.filter_npcs_for_position(&npcs);
        if candidates.is_empty() {
            return vec![self.narrate("There's no one here to talk to.")];
        }
        let npc = if name_query.is_empty() {
            if candidates.len() == 1 {
                &candidates[0]
            } else {
                let names: Vec<&str> = candidates.iter().map(|n| n.name.as_str()).collect();
                return vec![self.narrate(format!("Talk to whom? People here: {}", names.join(", ")))];
            }
        } else {
            let found = candidates.iter().find(|n| n.name.to_lowercase().starts_with(&name_query))
                .or_else(|| candidates.iter().find(|n| n.name.to_lowercase().contains(&name_query)));
            match found {
                Some(n) => n,
                None => return vec![self.narrate(format!("You don't see anyone named '{name_query}' here."))],
            }
        };
        let npc_name = npc.name.clone();
        let npc_data = serde_json::to_value(npc)
            .ok()
            .and_then(|v| if let JsonValue::Object(m) = v { Some(m) } else { None })
            .unwrap_or_default();
        self.pending_social_scene = Some(SocialSceneContext {
            npc_entity_id: npc.entity_id.clone().unwrap_or_default(),
            npc_name: npc_name.clone(),
            npc_data,
        });
        self.pending_social_transition = true;
        vec![self.narrate(format!("You approach {npc_name}."))]
    }

    fn handle_shop(&mut self, _cmd: &ParsedCommand) -> Vec<GameEvent> {
        let building = self.building_at_pos().cloned();
        let settlement_json = serde_json::to_value(&self.settlement)
            .ok()
            .and_then(|v| if let JsonValue::Object(m) = v { Some(m) } else { None })
            .unwrap_or_default();
        let building_type = building.as_ref().and_then(|b| b.r#type.clone());
        let building_name = building.as_ref().map(|b| b.name.clone());
        self.pending_shopping_scene = Some(ShoppingSceneContext {
            settlement_data: settlement_json,
            building_type,
            building_tier: None,
            building_name: building_name.clone(),
        });
        self.pending_shopping_transition = true;
        let name = building_name.unwrap_or_else(|| self.settlement.name.clone());
        vec![self.narrate(format!("You enter {name}."))]
    }

    fn handle_leave(&mut self) -> Vec<GameEvent> {
        self.pending_leave = true;
        let name = &self.settlement.name;
        vec![self.narrate(format!("You leave {name} and return to the wilderness."))]
    }

    fn handle_status(&self, db: &WorldDatabase) -> Vec<GameEvent> {
        let seed = db.get_meta("seed").ok().flatten().unwrap_or_else(|| "default".to_string());
        let weather_svc = WeatherService::new(&seed);
        let weather = weather_svc.at(self.q as i64, self.r as i64, self.tick, Some("settlement"));
        let time_str = format!("{} ({})", format_time(self.tick), get_time_description(self.tick));
        let text = format!(
            "Town: {} ({})\nTime: {}  Weather: {}, {}\nPosition: ({}, {})",
            self.settlement.name,
            self.settlement.size,
            time_str,
            weather.condition,
            weather.temperature,
            self.player_q,
            self.player_r,
        );
        vec![self.narrate(text)]
    }

    fn handle_help(&self) -> Vec<GameEvent> {
        vec![self.narrate(
            "Town commands:\n\
             \x20 go <direction>   Move within town (8 directions)\n\
             \x20 look / l         Describe your surroundings\n\
             \x20 talk [to] <npc>  Speak with an NPC\n\
             \x20 shop             Enter the shops\n\
             \x20 leave / exit     Leave the settlement\n\
             \x20 status           Show town info\n\
             \x20 help             Show this message",
        )]
    }

    fn handle_inventory(&self) -> Vec<GameEvent> {
        vec![self.narrate("Inventory is managed through the exploration scene.")]
    }

    fn filter_npcs_for_position<'a>(&self, npcs: &'a [SceneNpcRecord]) -> Vec<&'a SceneNpcRecord> {
        let pos = (self.player_q, self.player_r);
        if let Some(&idx) = self.building_map.get(&pos) {
            if let Some(b) = self.buildings.get(idx) {
                return npcs.iter().filter(|n| {
                    (b.npc_id.is_some() && n.entity_id == b.npc_id)
                        || (b.npc_name.is_some() && Some(n.name.as_str()) == b.npc_name.as_deref())
                }).collect();
            }
        }
        let terrain = self.terrain.get(&pos).map_or("open_ground", |s| s.as_str());
        if matches!(terrain, "plaza" | "road" | "open_ground") {
            let assigned_names: std::collections::HashSet<&str> = self.buildings.iter()
                .filter_map(|b| b.npc_name.as_deref())
                .collect();
            let assigned_ids: std::collections::HashSet<&str> = self.buildings.iter()
                .filter_map(|b| b.npc_id.as_deref())
                .collect();
            return npcs.iter().filter(|n| {
                let id_ok = n.entity_id.as_deref().map_or(true, |id| !assigned_ids.contains(id));
                let name_ok = !assigned_names.contains(n.name.as_str());
                id_ok && name_ok
            }).collect();
        }
        vec![]
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn to_json_object<T: serde::Serialize>(value: &T) -> JsonObject {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

impl SceneHandler for TownScene {
    fn get_valid_commands(&self) -> Vec<String> {
        ["move", "look", "talk", "shop", "leave", "status", "help", "inventory", "search"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        format!("[{}]", self.settlement.name)
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let raw_verb = cmd.raw.trim().to_lowercase();
        let raw_first = raw_verb.split_whitespace().next().unwrap_or("");
        let events = match cmd.verb.as_str() {
            "move" => self.handle_move(cmd, db),
            "look" | "l" | "search" => self.handle_look(db),
            "talk" => self.handle_talk(cmd, db),
            "shop" => self.handle_shop(cmd),
            "leave" | "exit" => self.handle_leave(),
            "status" => self.handle_status(db),
            "help" => self.handle_help(),
            "inventory" => self.handle_inventory(),
            _ if raw_first == "leave" || raw_first == "exit" => self.handle_leave(),
            _ => vec![self.narrate(format!(
                "Unknown command \"{}\". Type \"help\" for available commands.",
                cmd.raw
            ))],
        };
        Ok(events)
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        if self.pending_leave {
            return Some(SceneState::Exploration);
        }
        if self.pending_social_transition {
            return Some(SceneState::Social);
        }
        if self.pending_shopping_transition {
            return Some(SceneState::Shopping);
        }
        None
    }

    fn get_suggestions(&self) -> Vec<String> {
        let pos = (self.player_q, self.player_r);
        let terrain = self.terrain.get(&pos).map_or("open_ground", |s| s.as_str());
        let at_shop = matches!(terrain, "shop" | "tavern" | "temple")
            || self.building_map.contains_key(&pos);
        let abbr: HashMap<&str, &str> = [
            ("north", "n"), ("south", "s"), ("east", "e"), ("west", "w"),
            ("northeast", "ne"), ("northwest", "nw"), ("southeast", "se"), ("southwest", "sw"),
        ].into_iter().collect();
        let origin = GridCoord { q: self.player_q, r: self.player_r };
        let mut dirs: Vec<String> = Vec::new();
        for d in self.grid.directions() {
            if let Ok(target) = self.grid.neighbor(origin, d) {
                let tt = self.terrain.get(&(target.q as i32, target.r as i32));
                if tt.is_some() && !is_impassable_town(tt.unwrap()) {
                    dirs.push(abbr.get(d).unwrap_or(&d).to_string());
                }
            }
        }
        let mut s = Vec::new();
        if at_shop {
            s.push("shop".to_string());
        }
        s.push("talk".to_string());
        s.push("look".to_string());
        s.extend(dirs);
        s.push("leave".to_string());
        s.push("help".to_string());
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
    use crate::settlement::SettlementSummary;

    fn make_settlement(name: &str) -> SettlementSummary {
        SettlementSummary {
            name: name.to_string(),
            size: "village".to_string(),
            description: String::new(),
            settlement_id: "test-settlement".to_string(),
            establishments: vec![],
            resident_npc_ids: vec![],
        }
    }

    fn three_cell_town() -> Vec<TownCell> {
        vec![
            TownCell { q: 0, r: 0, terrain: "road".to_string(), features: vec![], data: JsonObject::new() },
            TownCell { q: 1, r: 0, terrain: "road".to_string(), features: vec![], data: JsonObject::new() },
            TownCell { q: 0, r: 1, terrain: "road".to_string(), features: vec![], data: JsonObject::new() },
        ]
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

    #[test]
    fn prompt_shows_settlement_name() {
        let scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, vec![], 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        assert!(scene.get_prompt(&db).contains("Dusthaven"));
    }

    #[test]
    fn help_returns_narrate_event() {
        let mut scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, vec![], 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("help", "help"), &db).unwrap();
        assert_eq!(events[0].event_type, "gm.narrate");
        assert!(events[0].data["text"].as_str().unwrap_or("").contains("Town commands"));
    }

    #[test]
    fn leave_transitions_to_exploration() {
        let mut scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, vec![], 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        scene.handle_command(&cmd("leave", "leave"), &db).unwrap();
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Exploration));
    }

    #[test]
    fn move_updates_player_position() {
        let mut scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, three_cell_town(), 0);
        assert_eq!((scene.player_q, scene.player_r), (0, 0));
        let db = WorldDatabase::open_in_memory().unwrap();
        scene.handle_command(&cmd_dir("move", "move east", "east"), &db).unwrap();
        assert_eq!((scene.player_q, scene.player_r), (1, 0));
    }

    #[test]
    fn move_out_of_bounds_refused() {
        let mut scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, three_cell_town(), 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd_dir("move", "move west", "west"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("edge of town"), "text: {text}");
    }

    #[test]
    fn move_into_impassable_refused() {
        let cells = vec![
            TownCell { q: 0, r: 0, terrain: "road".to_string(), features: vec![], data: JsonObject::new() },
            TownCell { q: 1, r: 0, terrain: "building".to_string(), features: vec![], data: JsonObject::new() },
        ];
        let mut scene = TownScene::new(make_settlement("Town"), 0, 0, cells, 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd_dir("move", "move east", "east"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("blocks your path"), "text: {text}");
    }

    #[test]
    fn shop_sets_pending_shopping_transition() {
        let mut scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, vec![], 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        scene.handle_command(&cmd("shop", "shop"), &db).unwrap();
        assert!(scene.pending_shopping_transition);
    }

    #[test]
    fn look_returns_narrate_and_map_event() {
        let scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, vec![], 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut scene = scene;
        let events = scene.handle_command(&cmd("look", "look"), &db).unwrap();
        assert!(events.iter().any(|e| e.event_type == "gm.narrate"));
        assert!(events.iter().any(|e| e.event_type == "town.map"));
    }

    #[test]
    fn no_transition_before_action() {
        let scene = TownScene::new(make_settlement("Dusthaven"), 0, 0, vec![], 0);
        assert_eq!(scene.check_transitions(&[]), None);
    }

    #[test]
    fn status_contains_settlement_name() {
        let mut scene = TownScene::new(make_settlement("Riverton"), 0, 0, vec![], 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene.handle_command(&cmd("status", "status"), &db).unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("Riverton"), "text: {text}");
    }

    #[test]
    fn corner_blocking_when_both_cardinals_impassable() {
        // Player at (0,0), building at (0,1) and (1,0), diag target (1,1) open
        let cells = vec![
            TownCell { q: 0, r: 0, terrain: "road".to_string(), features: vec![], data: JsonObject::new() },
            TownCell { q: 1, r: 0, terrain: "building".to_string(), features: vec![], data: JsonObject::new() },
            TownCell { q: 0, r: 1, terrain: "building".to_string(), features: vec![], data: JsonObject::new() },
            TownCell { q: 1, r: 1, terrain: "road".to_string(), features: vec![], data: JsonObject::new() },
        ];
        let mut scene = TownScene::new(make_settlement("Town"), 0, 0, cells, 0);
        let db = WorldDatabase::open_in_memory().unwrap();
        let events = scene
            .handle_command(&cmd_dir("move", "move southeast", "southeast"), &db)
            .unwrap();
        let text = events[0].data["text"].as_str().unwrap_or("");
        assert!(text.contains("block"), "text: {text}");
    }
}
