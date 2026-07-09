//! Exploration scene handler.
//!
//! Ported from `src/harsh_realm/gm/scenes/exploration*.py` (14 Python modules).

use crate::cell::TerrainRegistry;
use crate::combat::awareness::awareness_check;
use crate::loot_source;
use crate::combat_runtime::AwarenessResult;
use crate::command::ParsedCommand;
use crate::content::IRRecordRepository;
use crate::creature::{CreatureData, CreatureRegistry};
use crate::runtime_content::{ir_creature_to_data, RuntimeContentStore};
use crate::db::WorldDatabase;
use crate::encounters::EncounterSystem;
use crate::engine_results::{DiscoveryResult, EncounterResult};
use crate::events::GameEvent;
use crate::gm::narrator::Narrator;
use crate::gm::scenes::base::{SceneHandler, SceneState};
use crate::gm::scenes::time_support::{format_time, get_time_description};
use crate::grid::{Grid, GridCoord, SquareGrid};
use crate::gm_runtime::PendingEncounterState;
use crate::oracle::FateChecker;
use crate::payloads::contexts::{
    DungeonSceneContext, ShoppingSceneContext, SocialSceneContext, TownSceneContext,
};
use crate::healing::HealingSystem;
use crate::payloads::notices_combat::{
    CharacterHpChangedNotice, CombatActionButton, CombatActionsNotice, ExplorationActionsNotice,
    NarrationNotice,
};
use crate::payloads::notices_world::{
    ExplorationEnterHexNotice, InventoryItemGivenNotice, LootSourceRevealedNotice,
    OracleFateCheckNotice,
};
use crate::payloads::requests::ExplorationMoved;
use crate::repositories::cell::CellRepository;
use crate::repositories::entity::EntityRepository;
use crate::repositories::gm_state::GMStateRepository;
use crate::runtime::{InventoryItemRecord, JsonObject, JsonValue, SceneNpcRecord};
use crate::table_engine::TableEngine;
use crate::threads::ThreadTracker;
use crate::travel::{
    find_travel_path, TravelAction, TravelGoal, TravelStatus, TRAVEL_GOAL_KEY,
};
use crate::weather::WeatherService;

fn to_json_object<T: serde::Serialize>(value: &T) -> JsonObject {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

fn narrate(tick: i64, text: impl Into<String>) -> GameEvent {
    let notice = NarrationNotice { text: text.into() };
    GameEvent::new(tick, "gm.narrate", to_json_object(&notice))
}

/// Pre-combat exploration scene handler.
pub struct ExplorationScene {
    terrain_registry: TerrainRegistry,
    narrator: Narrator,
    pub tick: i64,
    current_terrain: Option<String>,
    current_features: Vec<String>,
    current_char_hp: i32,
    current_char_max_hp: i32,
    pending_combat: Option<PendingEncounterState>,
    pending_combat_transition: bool,
    pending_social_scene: Option<SocialSceneContext>,
    pending_social_transition: bool,
    pending_shopping_scene: Option<ShoppingSceneContext>,
    pending_shopping_transition: bool,
    pending_dungeon_scene: Option<DungeonSceneContext>,
    pending_dungeon_transition: bool,
    pending_town_scene: Option<TownSceneContext>,
    pending_town_transition: bool,
    /// Lazily-loaded creature catalog for spawning encounter combatants.
    creature_registry: Option<CreatureRegistry>,
}

impl ExplorationScene {
    /// HR-19: `quests` lists active/completed quests with objective progress;
    /// `quests accept <id>` / `quests abandon <id>` emit the request events that
    /// the controller resolves through the QuestService.
    fn handle_quests(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        use crate::quest::{QuestCatalog, QuestContent, QuestRepository};
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };

        let sub = cmd.args.first().map(|s| s.to_lowercase());
        if matches!(sub.as_deref(), Some("accept") | Some("abandon")) {
            let action = sub.as_deref().unwrap_or("accept");
            let Some(quest_id) = cmd.args.get(1) else {
                return Ok(vec![
                    self.narrate_msg(format!("Usage: quests {action} <quest_id>"))
                ]);
            };
            let event_type = if action == "accept" {
                "quest.accept_requested"
            } else {
                "quest.fail_requested"
            };
            let mut data = JsonObject::new();
            data.insert("entity_id".into(), JsonValue::String(char.id.clone()));
            data.insert("quest_id".into(), JsonValue::String(quest_id.clone()));
            let mut ev = GameEvent::new(self.tick, event_type, data);
            ev.source = "exploration".to_string();
            let verb = if action == "accept" { "accept" } else { "abandon" };
            return Ok(vec![
                self.narrate_msg(format!("You {verb} the quest '{quest_id}'.")),
                ev,
            ]);
        }

        let repo = QuestRepository::new(db);
        let catalog = QuestCatalog::load_default();
        let active = repo.list_active(&char.id)?;
        let completed = repo.list_completed(&char.id)?;

        let mut lines = vec!["=== Quests ===".to_string(), "Active:".to_string()];
        if active.is_empty() {
            lines.push("  (none)".to_string());
        }
        for q in &active {
            let content = catalog.get(&q.quest_id);
            let name = content
                .as_ref()
                .map(|c| c.name.clone())
                .unwrap_or_else(|| q.quest_id.clone());
            lines.push(format!("  • {name}"));
            if let Some(content) = content {
                for obj in &content.objectives {
                    let prog = q.progress.get(&obj.key).and_then(|v| v.as_i64()).unwrap_or(0);
                    let target = obj.target.map(|t| format!("/{t}")).unwrap_or_default();
                    lines.push(format!("      - {} [{prog}{target}]", obj.description));
                }
            }
        }
        lines.push("Completed:".to_string());
        if completed.is_empty() {
            lines.push("  (none)".to_string());
        }
        for q in &completed {
            let name = catalog
                .get(&q.quest_id)
                .map(|c| c.name)
                .unwrap_or_else(|| q.quest_id.clone());
            lines.push(format!("  ✓ {name}"));
        }
        Ok(vec![self.narrate_msg(lines.join("\n"))])
    }

    /// Create a new exploration scene.
    pub fn new(terrain_registry: TerrainRegistry, narrator: Narrator, tick: i64) -> Self {
        ExplorationScene {
            terrain_registry,
            narrator,
            tick,
            current_terrain: None,
            current_features: Vec::new(),
            current_char_hp: 0,
            current_char_max_hp: 0,
            pending_combat: None,
            pending_combat_transition: false,
            pending_social_scene: None,
            pending_social_transition: false,
            pending_shopping_scene: None,
            pending_shopping_transition: false,
            pending_dungeon_scene: None,
            pending_dungeon_transition: false,
            pending_town_scene: None,
            pending_town_transition: false,
            creature_registry: None,
        }
    }

    /// Pending combat context for social-scene callers.
    pub fn pending_social_scene(&self) -> Option<&SocialSceneContext> {
        self.pending_social_scene.as_ref()
    }

    /// Pending shopping context.
    pub fn pending_shopping_scene(&self) -> Option<&ShoppingSceneContext> {
        self.pending_shopping_scene.as_ref()
    }

    /// Pending dungeon context.
    pub fn pending_dungeon_scene(&self) -> Option<&DungeonSceneContext> {
        self.pending_dungeon_scene.as_ref()
    }

    /// Pending town context.
    pub fn pending_town_scene(&self) -> Option<&TownSceneContext> {
        self.pending_town_scene.as_ref()
    }

    /// Take and clear the pending combat state (for controller wiring).
    pub fn take_pending_combat(&mut self) -> Option<crate::gm_runtime::PendingEncounterState> {
        self.pending_combat.take()
    }

    /// Take and clear the pending social context (for controller wiring).
    pub fn take_pending_social(&mut self) -> Option<SocialSceneContext> {
        self.pending_social_scene.take()
    }

    /// Take and clear the pending shopping context (for controller wiring).
    pub fn take_pending_shopping(&mut self) -> Option<ShoppingSceneContext> {
        self.pending_shopping_scene.take()
    }

    /// Take and clear the pending dungeon context (for controller wiring).
    pub fn take_pending_dungeon(&mut self) -> Option<DungeonSceneContext> {
        self.pending_dungeon_scene.take()
    }

    /// Take and clear the pending town context (for controller wiring).
    pub fn take_pending_town(&mut self) -> Option<TownSceneContext> {
        self.pending_town_scene.take()
    }

    /// Update the scene's internal tick counter.
    pub fn set_tick(&mut self, tick: i64) {
        self.tick = tick;
    }

    // ---- narration helper --------------------------------------------------

    fn narrate_msg(&self, text: impl Into<String>) -> GameEvent {
        narrate(self.tick, text)
    }

    /// Build a `combat.actions` event with `phase: "cleared"` and no actions.
    ///
    /// Appended by flee/avoid/sneak/talk resolutions so the frontend immediately
    /// hides the pre-combat encounter menu when the encounter is dismissed.
    fn cleared_actions_event(&self) -> GameEvent {
        let notice = CombatActionsNotice {
            phase: "cleared".to_string(),
            actions: vec![],
        };
        let mut ev = GameEvent::new(self.tick, "combat.actions", to_json_object(&notice));
        ev.source = "exploration".to_string();
        ev
    }

    // ---- weather -----------------------------------------------------------

    fn get_weather(&self, db: &WorldDatabase, q: i32, r: i32) -> crate::runtime::WeatherState {
        let seed = GMStateRepository::new(db)
            .get_value("seed")
            .ok()
            .flatten()
            .unwrap_or_else(|| "default".to_string());
        let service = WeatherService::new(seed);
        service.at(
            q as i64,
            r as i64,
            self.tick,
            self.current_terrain.as_deref(),
        )
    }

    fn on_tick(&self, db: &WorldDatabase, q: i32, r: i32) -> Vec<GameEvent> {
        let mut events = Vec::new();
        let seed = GMStateRepository::new(db)
            .get_value("seed")
            .ok()
            .flatten()
            .unwrap_or_else(|| "default".to_string());
        let service = WeatherService::new(seed);
        if service.did_weather_change(q as i64, r as i64, self.tick.saturating_sub(1), self.tick) {
            let new_weather = service.at(
                q as i64,
                r as i64,
                self.tick,
                self.current_terrain.as_deref(),
            );
            events.push(self.narrate_msg(format!(
                "The weather shifts: {}",
                new_weather.description
            )));
        }
        events
    }

    // ---- persistence helpers -----------------------------------------------

    fn load_character(
        &mut self,
        db: &WorldDatabase,
    ) -> Result<Option<crate::character::Character>, String> {
        let repo = EntityRepository::new(db);
        let (char, _) = repo.load_character_with_record()?;
        if let Some(ref c) = char {
            self.current_char_hp = c.hp;
            self.current_char_max_hp = c.max_hp;
        }
        Ok(char)
    }

    fn fetch_cell(
        &self,
        db: &WorldDatabase,
        coord: GridCoord,
    ) -> Result<Option<crate::cell::CellData>, String> {
        CellRepository::new(db).fetch_cell(coord)
    }

    fn list_npcs(
        &self,
        db: &WorldDatabase,
        q: i32,
        r: i32,
    ) -> Result<Vec<SceneNpcRecord>, String> {
        EntityRepository::new(db).list_npcs_at_location(q as i64, r as i64)
    }

    fn find_npc_at(
        &self,
        db: &WorldDatabase,
        name_query: &str,
        q: i32,
        r: i32,
    ) -> Result<Option<SceneNpcRecord>, String> {
        let npcs = self.list_npcs(db, q, r)?;
        let query_lower = name_query.to_lowercase();
        if let Some(npc) = npcs
            .iter()
            .find(|n| n.name.to_lowercase().starts_with(&query_lower))
        {
            return Ok(Some(npc.clone()));
        }
        Ok(npcs
            .into_iter()
            .find(|n| n.name.to_lowercase().contains(&query_lower)))
    }

    fn load_travel_goal(&self, db: &WorldDatabase) -> Option<TravelGoal> {
        let raw = GMStateRepository::new(db)
            .get_value(TRAVEL_GOAL_KEY)
            .ok()
            .flatten()?;
        if raw.is_empty() {
            None
        } else {
            TravelGoal::from_json(&raw)
        }
    }

    fn save_travel_goal(&self, db: &WorldDatabase, goal: &TravelGoal) {
        let _ = GMStateRepository::new(db).set_value(TRAVEL_GOAL_KEY, &goal.to_json());
    }

    fn clear_travel_goal(&self, db: &WorldDatabase) {
        let _ = GMStateRepository::new(db).set_value(TRAVEL_GOAL_KEY, "");
    }

    // ---- command handlers --------------------------------------------------

    fn handle_move(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let grid = SquareGrid::default();
        let direction = match &cmd.direction {
            Some(d) => d.clone(),
            None => {
                return Ok(vec![self.narrate_msg(format!(
                    "Which direction? ({})",
                    grid.directions().join(", ")
                ))])
            }
        };
        if !grid.directions().contains(&direction.as_str()) {
            return Ok(vec![
                self.narrate_msg(format!("\"{}\" is not a valid direction.", direction))
            ]);
        }

        let char = match self.load_character(db)? {
            Some(c) => c,
            None => {
                return Ok(vec![self.narrate_msg(
                    "[ERROR] No character found. Something has gone wrong.",
                )])
            }
        };

        let current_coord = GridCoord {
            q: char.position_q,
            r: char.position_r,
        };
        let target_coord = match grid.neighbor(current_coord, &direction) {
            Ok(c) => c,
            Err(_) => {
                return Ok(vec![self.narrate_msg(
                    "You have reached the edge of the known world. You cannot go that way.",
                )])
            }
        };

        let current_cell = self.fetch_cell(db, current_coord)?;
        let target_cell = match self.fetch_cell(db, target_coord)? {
            Some(c) => c,
            None => {
                return Ok(vec![self.narrate_msg(
                    "You have reached the edge of the known world. You cannot go that way.",
                )])
            }
        };

        // Check passability
        if let Some(terrain_type) = self.terrain_registry.get(&target_cell.terrain) {
            if !terrain_type.passable {
                use crate::gm::scenes::exploration_support::blocked_message;
                use crate::payloads::notices_world::{CellPreview, ExplorationRevealedNotice};
                // #108: the player can't ENTER impassable terrain, but they can SEE it.
                // Reveal the cell (persist explored=2) and push it live so the map shows
                // the mountains / water / wall they walked up to, instead of leaving it
                // in fog. The `moved` handler only pushes cells you enter, so a blocked
                // move must emit its own reveal or the obstacle stays invisible.
                let _ = CellRepository::new(db)
                    .reveal_adjacent(target_coord.q as i64, target_coord.r as i64);
                let notice = ExplorationRevealedNotice {
                    cell: CellPreview {
                        q: target_coord.q,
                        r: target_coord.r,
                        terrain: target_cell.terrain.clone(),
                        features: target_cell.features.clone(),
                        explored: JsonValue::from(2),
                    },
                };
                let mut ev =
                    GameEvent::new(self.tick, "exploration.revealed", to_json_object(&notice));
                ev.source = "exploration".to_string();
                return Ok(vec![ev, self.narrate_msg(blocked_message(&target_cell.terrain))]);
            }
        }

        let first_visit = !target_cell.explored;
        let weather = self.get_weather(db, target_coord.q, target_coord.r);
        let mut events = self.on_tick(db, target_coord.q, target_coord.r);

        let move_desc = match &current_cell {
            Some(cc) => self
                .narrator
                .describe_movement(Some(&direction), cc, &target_cell),
            None => self
                .narrator
                .describe_movement(Some(&direction), &target_cell, &target_cell),
        };
        events.push(self.narrate_msg(&move_desc));

        // Build move event
        use crate::payloads::notices_combat::CharacterSnapshot;
        use crate::payloads::notices_world::CellPreview;
        let char_snapshot =
            serde_json::to_value(&char)
                .ok()
                .and_then(|v| if let JsonValue::Object(m) = v { Some(m) } else { None })
                .map(|m| serde_json::from_value::<CharacterSnapshot>(JsonValue::Object(m)).ok())
                .flatten();
        if let Some(snap) = char_snapshot {
            let adjacent: Vec<CellPreview> = grid
                .neighbors(target_coord)
                .into_iter()
                .filter_map(|nb| self.fetch_cell(db, nb).ok().flatten())
                .map(|c| CellPreview {
                    q: c.q,
                    r: c.r,
                    terrain: c.terrain.clone(),
                    features: c.features.clone(),
                    explored: JsonValue::Bool(c.explored),
                })
                .collect();
            let move_payload = ExplorationMoved {
                character_id: char.id.clone(),
                character_data: snap,
                from_q: current_coord.q,
                from_r: current_coord.r,
                to_q: target_coord.q,
                to_r: target_coord.r,
                direction: direction.clone(),
                first_visit,
                target_cell: CellPreview {
                    q: target_coord.q,
                    r: target_coord.r,
                    terrain: target_cell.terrain.clone(),
                    features: target_cell.features.clone(),
                    explored: JsonValue::Bool(true),
                },
                adjacent_cells: adjacent,
                weather: Some(to_json_object(&weather)),
            };
            // HR-791: this is a frontend NOTICE (marker/LOCATION/cell/weather update),
            // not a request — it must be named without the `_requested` suffix, or
            // `GMController::resolve_domain_events` filters it out and it never reaches
            // the client (which is why movement didn't visibly update). Position is
            // persisted directly above; this event just tells the UI where the PC is now.
            let mut ev = GameEvent::new(self.tick, "exploration.moved", to_json_object(&move_payload));
            ev.source = "exploration".to_string();
            events.push(ev);
        }

        self.current_terrain = Some(target_cell.terrain.clone());
        self.current_features = target_cell.features.clone();

        // Persist the player's new world position. `handle_move` is the single
        // entry point for exploration movement, and the emitted
        // `exploration.move_requested` event has no persistence subscriber — so
        // without this write the character never actually moves (the DB keeps
        // reporting the old cell, which reads to the player as "look, not move").
        {
            let mut moved = char.clone();
            moved.position_q = target_coord.q;
            moved.position_r = target_coord.r;
            let _ = EntityRepository::new(db).save_character(&moved, true, false, false);
        }

        // Mark cell explored and narrate
        let _ = CellRepository::new(db).mark_explored(target_coord.q as i64, target_coord.r as i64, 1);
        if first_visit {
            events.extend(self.look_internal(db, target_coord));
        } else {
            let weather_str = format!("The weather is {} and {}.", weather.condition, weather.temperature);
            events.push(self.narrate_msg(weather_str));
        }

        // Emit a structured hex-entry event so IR triggers the player carries
        // (statuses / equipped items / intrinsic traits) can react to terrain,
        // features, or position. `self_id` lets the trigger runtime source them.
        events.push(enter_hex_event(
            self.tick,
            &char.id,
            &target_cell.terrain,
            &target_cell.features,
            target_coord,
            first_visit,
        ));

        // Roll for a random encounter on entering the cell.
        let already_explored = !first_visit;
        events.extend(self.maybe_spawn_encounter(db, &char, &target_cell, target_coord, already_explored));

        Ok(events)
    }

    /// Roll the encounter system on hex entry. A hostile result stages
    /// `pending_combat` (the player then chooses attack/flee/talk); neutral and
    /// environmental results are narrated. Returns the narration events.
    fn maybe_spawn_encounter(
        &mut self,
        db: &WorldDatabase,
        character: &crate::character::Character,
        target_cell: &crate::cell::CellData,
        coord: GridCoord,
        already_explored: bool,
    ) -> Vec<GameEvent> {
        // Make sure the creature catalog includes the world's IR creatures before
        // selecting an encounter (HR-762).
        self.ensure_full_catalog(db);

        let mut hex = JsonObject::new();
        hex.insert("terrain".into(), JsonValue::String(target_cell.terrain.clone()));
        hex.insert("explored".into(), JsonValue::Bool(already_explored));
        hex.insert(
            "features".into(),
            JsonValue::Array(
                target_cell
                    .features
                    .iter()
                    .map(|f| JsonValue::String(f.clone()))
                    .collect(),
            ),
        );
        hex.insert("q".into(), JsonValue::from(coord.q));
        hex.insert("r".into(), JsonValue::from(coord.r));

        let engine = TableEngine::new(db, rand::thread_rng());
        let mut system = EncounterSystem::new(engine, rand::thread_rng());
        let result = match system.check_encounter(&hex, character, db) {
            Ok(Some(result)) => result,
            Ok(None) => return Vec::new(),
            Err(e) => {
                eprintln!("encounter check failed: {e}");
                return Vec::new();
            }
        };

        if result.encounter_type != "hostile" {
            // Neutral NPCs are already spawned as entities by the encounter system;
            // environmental results are pure narration.
            return vec![self.narrate_msg(&result.description)];
        }

        let creatures = self.select_encounter_creatures(&result, &target_cell.terrain);
        if creatures.is_empty() {
            // No creature catalog match — narrate the flavour but don't stage combat.
            return vec![self.narrate_msg(&result.description)];
        }

        let creature_objs: Vec<JsonObject> = creatures
            .iter()
            .filter_map(|c| match serde_json::to_value(c) {
                Ok(JsonValue::Object(m)) => Some(m),
                _ => None,
            })
            .collect();

        // Roll a real awareness check against the lead creature instead of
        // assuming mutual awareness, so surprise (either side) shapes the
        // available choices and the opening narration.
        let mut rng = rand::thread_rng();
        let check = awareness_check(character, &creatures[0], &target_cell.terrain, &mut rng);
        let awareness_key = awareness_result_key(check.result);

        let can_talk = creatures.iter().any(|c| {
            c.tags.iter().any(|t| t == "humanoid")
        });

        self.pending_combat = Some(PendingEncounterState {
            creatures: creature_objs,
            awareness_result: awareness_key.to_string(),
            terrain: target_cell.terrain.clone(),
            features: target_cell.features.clone(),
            cell_q: coord.q,
            cell_r: coord.r,
        });

        // Emit the pre-combat action bar immediately so the frontend shows
        // the encounter menu buttons as soon as the encounter is staged.
        let notice = pre_combat_actions_notice(awareness_key, can_talk);
        let mut actions_ev = GameEvent::new(self.tick, "combat.actions", to_json_object(&notice));
        actions_ev.source = "exploration".to_string();

        vec![
            self.narrate_msg(&result.description),
            self.narrate_msg(&check.narration),
            self.narrate_msg(format!(
                "Choose: {}.",
                encounter_choices_for(awareness_key).join(", ")
            )),
            actions_ev,
        ]
    }

    /// Pick the creatures for a hostile encounter. Preference order: explicit
    /// `creatures` ids on the encounter record, then `creature_tags`, then
    /// creatures tagged with the current terrain, then any creature. Spawns a
    /// small pack (1–3) of a single chosen species.
    fn select_encounter_creatures(
        &mut self,
        result: &EncounterResult,
        terrain: &str,
    ) -> Vec<CreatureData> {
        let registry = self.creature_registry();

        // 1. Explicit creature ids on the encounter entry.
        let extra = &result.data.extra;
        if let Some(ids) = extra.get("creatures").and_then(|v| v.as_array()) {
            let picked: Vec<CreatureData> = ids
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|id| registry.get(id).cloned())
                .collect();
            if !picked.is_empty() {
                return picked; // explicit list is taken verbatim (no pack sizing)
            }
        }

        // 2. creature_tags, else 3. terrain tag, else 4. anything.
        let tag_filter: Vec<String> = extra
            .get("creature_tags")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default();

        let candidates: Vec<CreatureData> = if !tag_filter.is_empty() {
            registry.by_tags(&tag_filter).into_iter().cloned().collect()
        } else {
            let by_terrain: Vec<CreatureData> = registry
                .by_tags(&[terrain.to_string()])
                .into_iter()
                .cloned()
                .collect();
            if by_terrain.is_empty() {
                registry.all().into_iter().cloned().collect()
            } else {
                by_terrain
            }
        };
        if candidates.is_empty() {
            return Vec::new();
        }

        use rand::Rng;
        let mut rng = rand::thread_rng();
        let species = &candidates[rng.gen_range(0..candidates.len())];
        let pack_size = if species.tags.iter().any(|t| t == "pack") {
            rng.gen_range(2..=3)
        } else {
            rng.gen_range(1..=2)
        };
        std::iter::repeat(species.clone()).take(pack_size).collect()
    }

    /// Lazily load and cache the creature catalog from the content directory.
    fn creature_registry(&mut self) -> &CreatureRegistry {
        if self.creature_registry.is_none() {
            let dir = self.narrator.content_dir().to_path_buf();
            self.creature_registry = Some(CreatureRegistry::load(&dir));
        }
        self.creature_registry.as_ref().expect("just loaded")
    }

    /// Ensure the creature catalog is loaded, folding the world's IR-authored
    /// creatures into the legacy file catalog (HR-762 — one content model). IR
    /// creatures are last-wins on id, so authored IR can augment or override the
    /// legacy `creatures/*.yaml` entries and become encounterable in live play.
    fn ensure_full_catalog(&mut self, db: &WorldDatabase) {
        if self.creature_registry.is_some() {
            return;
        }
        let dir = self.narrator.content_dir().to_path_buf();
        let mut registry = CreatureRegistry::load(&dir);
        match RuntimeContentStore::from_repository(&IRRecordRepository::new(db)) {
            Ok((store, _errors)) => {
                registry.extend(store.creatures().map(|c| ir_creature_to_data(c).0));
            }
            Err(e) => eprintln!("ir catalog load failed: {e}"),
        }
        self.creature_registry = Some(registry);
    }

    fn handle_look(
        &mut self,
        _cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let coord = GridCoord { q: char.position_q, r: char.position_r };
        Ok(self.look_internal(db, coord))
    }

    fn look_internal(&mut self, db: &WorldDatabase, coord: GridCoord) -> Vec<GameEvent> {
        let cell = match self.fetch_cell(db, coord) {
            Ok(Some(c)) => c,
            _ => return vec![self.narrate_msg("You see nothing but an empty void.")],
        };
        self.current_terrain = Some(cell.terrain.clone());
        self.current_features = cell.features.clone();
        let move_desc = self.narrator.describe_movement(None, &cell, &cell);
        let weather = self.get_weather(db, coord.q, coord.r);
        let weather_str = format!("{} It is {}.", weather.description, weather.temperature);
        let mut events = vec![self.narrate_msg(format!("{}\n{}", move_desc, weather_str))];
        if let Ok(npcs) = self.list_npcs(db, coord.q, coord.r) {
            if !npcs.is_empty() {
                let labels: Vec<String> = npcs
                    .iter()
                    .map(|n| {
                        let occ = if n.occupation.is_empty() {
                            "resident"
                        } else {
                            &n.occupation
                        };
                        format!("{} ({})", n.name, occ)
                    })
                    .collect();
                events.push(self.narrate_msg(format!("Present: {}", labels.join(", "))));
            }
        }
        // Reveal adjacent cells
        let grid = SquareGrid::default();
        let terrain_type = self.terrain_registry.get(&cell.terrain).cloned();
        let should_reveal = terrain_type.map_or(true, |t| !t.blocks_vision);
        if should_reveal {
            let repo = CellRepository::new(db);
            for nb in grid.neighbors(coord) {
                let _ = repo.reveal_adjacent(nb.q as i64, nb.r as i64);
            }
        }
        events
    }

    fn handle_status(
        &mut self,
        _cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let attrs = &char.attributes;
        let mods = &char.attr_mods;
        let attr_line = ["str", "dex", "con", "int", "wis", "cha"]
            .iter()
            .map(|a| {
                let val = attrs.get(*a).copied().unwrap_or(10) as i64;
                let m = mods.get(*a).copied().unwrap_or(0);
                format!("{} {} ({:+})", a.to_uppercase(), val, m as i64)
            })
            .collect::<Vec<_>>()
            .join("  ");
        let skills_str: String = {
            let mut pairs: Vec<String> = char
                .skills
                .iter()
                .filter(|(_, lvl)| **lvl >= 0)
                .map(|(sk, lvl)| format!("{}:{}", sk, lvl))
                .collect();
            pairs.sort();
            if pairs.is_empty() {
                "(all untrained)".to_string()
            } else {
                pairs.join("  ")
            }
        };
        let equip_str = if char.equipment.is_empty() {
            "(none)".to_string()
        } else {
            char.equipment
                .iter()
                .filter_map(|e| {
                    serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(e.clone()))
                        .ok()
                        .map(|i| i.name)
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        let time_str = format!("{} ({})", format_time(self.tick), get_time_description(self.tick));
        let weather = self.get_weather(db, char.position_q, char.position_r);
        let weather_str = format!("{}, {}", weather.condition, weather.temperature);
        let sheet = format!(
            "--- {} ---\nTime: {}  Weather: {}\nClass: {}  Level: {}  XP: {}/{}\n\
             HP: {}/{}  AC: {}  Attack: {:+}\nAttrs: {}\nSkills: {}\n\
             Equipment: {}\nPosition: ({}, {})\n\
             Saves: PHY {} / EVA {} / MEN {}",
            char.name,
            time_str,
            weather_str,
            char.character_class.to_uppercase(),
            char.level,
            char.xp,
            char.xp_next,
            char.hp,
            char.max_hp,
            char.ac,
            char.attack_bonus,
            attr_line,
            skills_str,
            equip_str,
            char.position_q,
            char.position_r,
            char.physical_save,
            char.evasion_save,
            char.mental_save,
        );
        Ok(vec![self.narrate_msg(sheet)])
    }

    fn handle_inventory(
        &mut self,
        _cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        if char.equipment.is_empty() {
            return Ok(vec![self.narrate_msg("You are carrying nothing.")]);
        }
        let mut lines = vec!["Inventory:".to_string()];
        for raw in &char.equipment {
            if let Ok(item) = serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone())) {
                let enc = item.enc.map_or("?".to_string(), |e| e.to_string());
                lines.push(format!("  {}  (enc {})", item.name, enc));
            }
        }
        Ok(vec![self.narrate_msg(lines.join("\n"))])
    }

    fn handle_help(&self) -> Vec<GameEvent> {
        vec![self.narrate_msg(
            "Available commands:\n\
             go <direction>      Move (n/ne/e/se/s/sw/w/nw)\n\
             look / l            Describe your current location\n\
             search              Search for hidden items or features\n\
             enter               Enter a settlement or dungeon\n\
             shop                Browse shops at a settlement\n\
             examine <npc>       Inspect a nearby NPC\n\
             talk [to] <npc>     Speak with a nearby NPC\n\
             forage              Gather food/materials from the wilderness\n\
             oracle <question>   Consult the oracle (fate check)\n\
             add thread <title>  Track a new story thread\n\
             list threads        Show active threads\n\
             resolve thread <id> Mark a thread as resolved\n\
             status / stats      Show character sheet\n\
             inventory / inv     List carried equipment\n\
             help / ?            Show this message\n\
             rest / wait         Rest to recover HP\n\
             save                Save the world state\n\
             quit / exit         Quit the game\n\
             travel <q> <r>      Pathfind to a target cell\n\
             Tip: Try 'oracle help' for more oracle & adventure tools.",
        )]
    }

    fn handle_enter(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let coord = GridCoord { q: char.position_q, r: char.position_r };
        let cell = match self.fetch_cell(db, coord)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("There is nothing here to enter.")]),
        };

        let features = &cell.features;
        let target = cmd.args.first().map(|a| a.to_lowercase());

        // Check for dungeon entry
        if features.iter().any(|f| f == "dungeon" || f == "lair")
            || target.as_deref() == Some("dungeon")
        {
            let dungeon_repo = crate::repositories::dungeon::DungeonRepository::new(db);
            if let Ok(Some(dungeon)) = dungeon_repo.get_dungeon_at_location(char.position_q as i64, char.position_r as i64)
            {
                let rooms: Vec<JsonObject> = dungeon
                    .rooms
                    .iter()
                    .filter_map(|r| {
                        serde_json::to_value(r)
                            .ok()
                            .and_then(|v| if let JsonValue::Object(m) = v { Some(m) } else { None })
                    })
                    .collect();
                let conns: Vec<JsonObject> = dungeon
                    .connections
                    .iter()
                    .filter_map(|c| {
                        serde_json::to_value(c)
                            .ok()
                            .and_then(|v| if let JsonValue::Object(m) = v { Some(m) } else { None })
                    })
                    .collect();
                self.pending_dungeon_scene = Some(DungeonSceneContext {
                    dungeon_id: dungeon.id.clone(),
                    rooms,
                    connections: conns,
                    entry_q: char.position_q,
                    entry_r: char.position_r,
                });
                self.pending_dungeon_transition = true;
                return Ok(vec![self.narrate_msg("You descend into the darkness...")]);
            }
            return Ok(vec![self.narrate_msg("You find no entrance here.")]);
        }

        // Check for settlement/town entry
        if features.iter().any(|f| f == "settlement")
            || cell.terrain == "settlement"
            || target.as_deref() == Some("town")
        {
            return self.handle_enter_town(db, coord, &cell);
        }

        Ok(vec![self.narrate_msg("There is nothing here to enter.")])
    }

    fn handle_enter_town(
        &mut self,
        db: &WorldDatabase,
        coord: GridCoord,
        cell: &crate::cell::CellData,
    ) -> Result<Vec<GameEvent>, String> {
        // Load settlement and town cells
        let cell_data = CellRepository::new(db)
            .load_cell_data(coord.q as i64, coord.r as i64)?
            .unwrap_or_default();
        let settlement_data = cell_data
            .get("settlement")
            .and_then(|v| if let JsonValue::Object(m) = v { Some(m.clone()) } else { None })
            .unwrap_or_default();
        let name = settlement_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("the settlement");
        let town_cells: Vec<JsonObject> = cell
            .data
            .get("town_cells")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| if let JsonValue::Object(m) = v { Some(m.clone()) } else { None })
                    .collect()
            })
            .unwrap_or_default();
        self.pending_town_scene = Some(TownSceneContext {
            settlement_data: settlement_data.clone(),
            q: coord.q,
            r: coord.r,
            town_cells: town_cells.clone(),
        });
        self.pending_town_transition = true;
        Ok(vec![self.narrate_msg(format!("You enter {}.", name))])
    }

    fn handle_explore(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let coord = GridCoord { q: char.position_q, r: char.position_r };
        let cell = match self.fetch_cell(db, coord)? {
            Some(c) => c,
            None => {
                return Ok(vec![
                    self.narrate_msg("You see nothing here worth exploring.")
                ])
            }
        };
        let target = cmd.args.first().map(|a| a.to_lowercase());
        if target.as_deref() == Some("town") || target.as_deref() == Some("settlement")
            || cell.features.iter().any(|f| f == "settlement")
            || cell.terrain == "settlement"
        {
            return self.describe_settlement(db, coord);
        }
        Ok(vec![self.narrate_msg(
            "There's nothing here to explore. Try 'search' to look for hidden things.",
        )])
    }

    fn describe_settlement(
        &self,
        db: &WorldDatabase,
        coord: GridCoord,
    ) -> Result<Vec<GameEvent>, String> {
        let cell_data = CellRepository::new(db)
            .load_cell_data(coord.q as i64, coord.r as i64)?
            .unwrap_or_default();
        let settlement = cell_data
            .get("settlement")
            .and_then(|v| if let JsonValue::Object(m) = v { Some(m.clone()) } else { None })
            .unwrap_or_default();
        let name = settlement.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Settlement");
        let size = settlement.get("size").and_then(|v| v.as_str()).unwrap_or("unknown");
        let desc = settlement.get("description").and_then(|v| v.as_str()).unwrap_or("");
        let msg = if desc.is_empty() {
            format!("{} — a {} settlement.", name, size)
        } else {
            format!("{} ({}): {}", name, size, desc)
        };
        let npcs = self.list_npcs(db, coord.q, coord.r)?;
        let mut events = vec![self.narrate_msg(msg)];
        if !npcs.is_empty() {
            let labels: Vec<String> = npcs
                .iter()
                .map(|n| format!("{} ({})", n.name, if n.occupation.is_empty() { "resident" } else { &n.occupation }))
                .collect();
            events.push(self.narrate_msg(format!("Residents: {}", labels.join(", "))));
        }
        Ok(events)
    }

    fn handle_shop(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let coord = GridCoord { q: char.position_q, r: char.position_r };
        let cell = self.fetch_cell(db, coord)?;
        let has_settlement = cell.as_ref().map_or(false, |c| {
            c.features.iter().any(|f| f == "settlement") || c.terrain == "settlement"
        });
        if !has_settlement {
            return Ok(vec![self.narrate_msg(
                "There's nothing to buy out here. Find a settlement first.",
            )]);
        }
        let cell_data = CellRepository::new(db)
            .load_cell_data(coord.q as i64, coord.r as i64)?
            .unwrap_or_default();
        let settlement_data = cell_data
            .get("settlement")
            .and_then(|v| if let JsonValue::Object(m) = v { Some(m.clone()) } else { None })
            .unwrap_or_default();
        let settlement_name = settlement_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("the settlement")
            .to_string();
        let buildings = EntityRepository::new(db)
            .list_entity_records_at_location("building", coord.q as i64, coord.r as i64, true)?;
        if buildings.is_empty() || cmd.args.is_empty() {
            if !buildings.is_empty() {
                let mut lines = vec![format!("--- Shops in {} ---", settlement_name)];
                for b in &buildings {
                    let bt = b.entity_data.get("building_type").and_then(|v| v.as_str()).unwrap_or("shop");
                    lines.push(format!("  {} ({})", b.name, bt));
                }
                lines.push("Use 'shop <name>' to enter a shop.".to_string());
                return Ok(vec![self.narrate_msg(lines.join("\n"))]);
            }
            self.pending_shopping_scene = Some(ShoppingSceneContext {
                settlement_data,
                building_type: None,
                building_tier: None,
                building_name: None,
            });
            self.pending_shopping_transition = true;
            return Ok(vec![self.narrate_msg(format!(
                "You enter the shops of {}.",
                settlement_name
            ))]);
        }
        let query = cmd.args.join(" ").to_lowercase();
        for b in &buildings {
            let bt = b.entity_data.get("building_type").and_then(|v| v.as_str()).unwrap_or("");
            let tier = b.entity_data.get("tier").and_then(|v| v.as_str()).unwrap_or("small");
            if bt == query.replace(' ', "_") || b.name.to_lowercase().starts_with(&query) {
                self.pending_shopping_scene = Some(ShoppingSceneContext {
                    settlement_data,
                    building_type: Some(bt.to_string()),
                    building_tier: Some(tier.to_string()),
                    building_name: Some(b.name.clone()),
                });
                self.pending_shopping_transition = true;
                return Ok(vec![self.narrate_msg(format!("You enter {}.", b.name))]);
            }
        }
        Ok(vec![self.narrate_msg(format!(
            "No '{}' found here. Use 'shop' to see available shops.",
            query
        ))])
    }

    fn handle_talk(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let mut args = cmd.args.clone();
        if args.first().map(|a| a.to_lowercase().as_str() == "to").unwrap_or(false) && args.len() > 1 {
            args.remove(0);
        }
        let npc = if args.is_empty() {
            let npcs = self.list_npcs(db, char.position_q, char.position_r)?;
            if npcs.is_empty() {
                return Ok(vec![self.narrate_msg("There is no one here to talk to.")]);
            }
            if npcs.len() == 1 {
                npcs.into_iter().next().unwrap()
            } else {
                let names: Vec<String> = npcs.iter().map(|n| n.name.clone()).collect();
                return Ok(vec![self.narrate_msg(format!(
                    "Talk to whom? People here: {}.",
                    names.join(", ")
                ))]);
            }
        } else {
            let query = args.join(" ");
            match self.find_npc_at(db, &query, char.position_q, char.position_r)? {
                Some(n) => n,
                None => {
                    return Ok(vec![self.narrate_msg(format!(
                        "You don't see anyone named '{}' here.",
                        query
                    ))])
                }
            }
        };

        let disposition = npc.disposition.as_i64().unwrap_or(0);
        if disposition <= -3 {
            return Ok(vec![self.narrate_msg(format!(
                "{} glares at you with open hostility. They refuse to speak.",
                npc.name
            ))]);
        }
        let greeting = if npc.greeting.is_empty() {
            "They acknowledge your presence.".to_string()
        } else {
            npc.greeting.clone()
        };
        let entity_id = npc.entity_id.clone().unwrap_or_default();
        let npc_data = to_json_object(&npc);
        let narration = if npc.occupation.is_empty() {
            format!("You approach {}. {}", npc.name, greeting)
        } else {
            format!("You approach {}, the {}. {}", npc.name, npc.occupation, greeting)
        };
        self.pending_social_scene = Some(SocialSceneContext {
            npc_entity_id: entity_id,
            npc_name: npc.name.clone(),
            npc_data,
        });
        self.pending_social_transition = true;
        Ok(vec![self.narrate_msg(narration)])
    }

    fn handle_examine(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let coord = GridCoord { q: char.position_q, r: char.position_r };
        let cell = self.fetch_cell(db, coord)?;

        // Check for landmark
        if let Some(cell) = &cell {
            if let Some(landmark) = cell.data.get("landmark").and_then(|v| v.as_object()) {
                let name_query = cmd.args.join(" ").to_lowercase();
                let lname = landmark.get("name").and_then(|v| v.as_str()).unwrap_or("landmark").to_lowercase();
                if cmd.args.is_empty() || name_query.contains(&lname) || lname.contains(&name_query) {
                    let desc = landmark.get("description").and_then(|v| v.as_str()).unwrap_or("A notable landmark.");
                    return Ok(vec![self.narrate_msg(desc)]);
                }
            }
        }

        let npc = if cmd.args.is_empty() {
            let npcs = self.list_npcs(db, char.position_q, char.position_r)?;
            if npcs.is_empty() {
                return Ok(vec![self.narrate_msg("There is no one here to examine.")]);
            }
            if npcs.len() == 1 {
                npcs.into_iter().next().unwrap()
            } else {
                let names: Vec<String> = npcs.iter().map(|n| n.name.clone()).collect();
                return Ok(vec![self.narrate_msg(format!(
                    "Examine whom? People here: {}.",
                    names.join(", ")
                ))]);
            }
        } else {
            let query = cmd.args.join(" ");
            match self.find_npc_at(db, &query, char.position_q, char.position_r)? {
                Some(n) => n,
                None => {
                    return Ok(vec![self.narrate_msg(format!(
                        "You don't see anyone named '{}' here.",
                        query
                    ))])
                }
            }
        };

        let disp_key = npc.disposition.as_str().unwrap_or("neutral");
        let demeanor = match disp_key {
            "hostile" => "They regard you with open hostility.",
            "wary" => "They watch you with guarded eyes.",
            "neutral" => "They pay you little mind.",
            "friendly" => "They seem pleased to see you.",
            _ => "",
        };
        let mut desc_parts = if npc.occupation.is_empty() {
            vec![format!("{} stands nearby.", npc.name)]
        } else {
            vec![format!("{} is a {}.", npc.name, npc.occupation)]
        };
        if !npc.appearance.is_empty() { desc_parts.push(npc.appearance.clone()); }
        if !demeanor.is_empty() { desc_parts.push(demeanor.to_string()); }
        Ok(vec![self.narrate_msg(desc_parts.join(" "))])
    }

    /// Push discovered items into a character's inventory and persist the
    /// change to the DB (inventory-only flags: no position/hp/gold sync).
    ///
    /// Called from `handle_search` exclusively for item/relic finds.
    /// Environmental feature writes are already handled by `search_hex`; this
    /// helper must NOT be called for those to avoid a spurious character save.
    fn grant_items(
        &self,
        db: &WorldDatabase,
        character: &mut crate::character::Character,
        items: &[JsonObject],
    ) -> Result<(), String> {
        // No items → no-op: never issue a spurious character write. This keeps
        // the helper safe to call in isolation (e.g. an environmental find).
        if items.is_empty() {
            return Ok(());
        }
        for item in items {
            character.equipment.push(item.clone());
        }
        EntityRepository::new(db).save_character(character, false, false, false)
    }

    /// Build client-facing `inventory.item_given` events for granted/taken items
    /// (HR-793). Reuses the same notice combat loot uses so items reach the
    /// inventory panel — unlike the old `exploration.{search,take}_requested`
    /// events, which ended in `_requested` and were filtered before the client.
    fn item_given_events(&self, character_id: &str, items: &[JsonObject]) -> Vec<GameEvent> {
        items
            .iter()
            .map(|item| {
                let notice = InventoryItemGivenNotice {
                    character_id: character_id.to_string(),
                    item: item.clone(),
                };
                let mut ev =
                    GameEvent::new(self.tick, "inventory.item_given", to_json_object(&notice));
                ev.source = "exploration".to_string();
                ev
            })
            .collect()
    }

    fn handle_search(
        &mut self,
        _cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        use crate::discovery::DiscoverySystem;
        let mut char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let coord = GridCoord { q: char.position_q, r: char.position_r };
        let cell = self.fetch_cell(db, coord)?;
        if cell.is_none() {
            return Ok(vec![self.narrate_msg("You find nothing in the void.")]);
        }
        let cell = cell.unwrap();
        let mut hex_data = JsonObject::new();
        hex_data.insert("q".into(), JsonValue::from(coord.q));
        hex_data.insert("r".into(), JsonValue::from(coord.r));
        hex_data.insert("terrain".into(), JsonValue::from(cell.terrain.clone()));
        hex_data.insert("explored".into(), JsonValue::from(cell.explored));
        hex_data.insert("data".into(), JsonValue::Object(cell.data.clone()));

        let mut sys = DiscoverySystem::new(
            TableEngine::new(db, rand::thread_rng()),
            rand::thread_rng(),
        );
        let result: DiscoveryResult = match sys.search_hex(&hex_data, &char, db, self.tick, true) {
            Ok(r) => r,
            Err(_) => {
                return Ok(vec![self.narrate_msg("You search the area but find nothing unusual.")])
            }
        };

        // HR-775 INVARIANT: a non-empty `discovered_items` (category item/relic)
        // MUST be granted + persisted here — do not drop this in a refactor.
        // Before this fix the items were packed into the event payload only, and
        // that event has no subscriber, so discovered items were narrated but
        // never saved. Environmental/feature finds (cooldown + cell features) are
        // already persisted inside `search_hex`; we only act when there are items.
        if !result.discovered_items.is_empty() {
            self.grant_items(db, &mut char, &result.discovered_items)?;
        }

        // HR-793: surface discovered items to the client as structured
        // `inventory.item_given` events — the same client-facing notice combat
        // loot uses — so found items appear in the inventory panel, not only in
        // narration. The old `exploration.search_requested` event ended in
        // `_requested`, so `resolve_domain_events` filtered it out and the
        // structured result never reached the client (HR-791 failure mode).
        let mut events = vec![self.narrate_msg(&result.message)];
        events.extend(self.item_given_events(&char.id, &result.discovered_items));
        // HR-786: reveal any unsearched loot sources (containers/corpses) on this
        // cell via a skill check, with a luck fallback and empty-handed results.
        events.extend(self.reveal_loot_sources(db, coord, &char, &cell.terrain)?);
        Ok(events)
    }

    /// Terrain → (skill, attribute) for loot-source reveal checks (HR-786),
    /// mirroring the discovery system's terrain skill selection.
    fn reveal_terrain_skill(terrain: &str) -> (&'static str, &'static str) {
        match terrain {
            "ruins" | "settlement" | "dungeon" => ("notice", "int"),
            _ => ("survive", "wis"),
        }
    }

    /// Narration for a revealed loot source, flavored by the skill margin.
    fn reveal_message(source: &loot_source::LootSource, outcome_key: &str) -> String {
        let mut parts: Vec<String> = source
            .contents
            .iter()
            .filter_map(|i| i.get("name").and_then(|v| v.as_str()).map(str::to_string))
            .collect();
        if source.gold > 0 {
            parts.push(format!("{} coins", source.gold));
        }
        if parts.is_empty() {
            return format!("You search the {} — it holds nothing.", source.name);
        }
        let prefix = if outcome_key == "exceptional_success" {
            "You expertly search"
        } else {
            "You search"
        };
        format!(
            "{} the {} and find: {}. Use 'take <item>'.",
            prefix,
            source.name,
            parts.join(", ")
        )
    }

    /// Build the `loot.source_revealed` client event for a source at `coord`
    /// (HR-786). `items`/`gold` reflect the source's CURRENT remaining contents;
    /// an exhausted source emits empty `items` + 0 `gold` so the panel drops it.
    fn loot_source_revealed_event(
        &self,
        coord: GridCoord,
        id: &str,
        kind: &str,
        name: &str,
        items: Vec<JsonObject>,
        gold: i32,
    ) -> GameEvent {
        let notice = LootSourceRevealedNotice {
            q: coord.q,
            r: coord.r,
            id: id.to_string(),
            kind: kind.to_string(),
            name: name.to_string(),
            items,
            gold,
        };
        let mut ev = GameEvent::new(self.tick, "loot.source_revealed", to_json_object(&notice));
        ev.source = "exploration".to_string();
        ev
    }

    /// HR-786: reveal unsearched loot sources on the current cell. A terrain
    /// skill check vs each source's `difficulty` decides success; a luck save
    /// (d20 vs 15) is a fallback, and failing both is an empty-handed result
    /// (the source stays unrevealed to retry). Persists the updated sources.
    fn reveal_loot_sources(
        &self,
        db: &WorldDatabase,
        coord: GridCoord,
        character: &crate::character::Character,
        terrain: &str,
    ) -> Result<Vec<GameEvent>, String> {
        use crate::resolvers::skill_check::resolve_skill_check;
        use crate::saves::resolve_save;
        use rand::Rng;

        let cell = match self.fetch_cell(db, coord)? {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };
        let mut cell_data = cell.data.clone();
        let mut sources = loot_source::read_sources(&cell_data);
        if sources.iter().all(|s| s.searched) {
            return Ok(Vec::new());
        }

        let (skill, attr) = Self::reveal_terrain_skill(terrain);
        let skill_level = character.skills.get(skill).copied().unwrap_or(-1) as i64;
        let attr_mod = character.attr_mods.get(attr).copied().unwrap_or(0) as i64;
        let mut rng = rand::thread_rng();
        let mut events = Vec::new();
        let mut changed = false;

        for source in sources.iter_mut() {
            if source.searched {
                continue;
            }
            let roll = (rng.gen_range(1..=6) + rng.gen_range(1..=6)) as i64;
            let outcome =
                resolve_skill_check(roll, skill_level, attr_mod, source.difficulty as i64);
            if outcome.success {
                source.searched = true;
                changed = true;
                events.push(self.narrate_msg(Self::reveal_message(source, &outcome.outcome_key)));
                events.push(self.loot_source_revealed_event(
                    coord,
                    &source.id,
                    &source.kind,
                    &source.name,
                    source.contents.clone(),
                    source.gold,
                ));
            } else {
                let luck = resolve_save(character, "luck", 0, 0, &mut rng);
                if luck.passed {
                    source.searched = true;
                    changed = true;
                    events.push(self.narrate_msg(format!(
                        "Luck guides your hand — you get into the {} despite yourself.",
                        source.name
                    )));
                    events.push(self.loot_source_revealed_event(
                        coord,
                        &source.id,
                        &source.kind,
                        &source.name,
                        source.contents.clone(),
                        source.gold,
                    ));
                } else {
                    events.push(self.narrate_msg(format!(
                        "You search the {} but come up empty-handed.",
                        source.name
                    )));
                }
            }
        }

        if changed {
            loot_source::write_sources(&mut cell_data, &sources);
            CellRepository::new(db).save_cell_data(coord.q as i64, coord.r as i64, &cell_data)?;
        }
        Ok(events)
    }

    fn handle_take(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let mut char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let coord = GridCoord { q: char.position_q, r: char.position_r };
        let cell = match self.fetch_cell(db, coord)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("There's nothing here to pick up.")]),
        };
        let markers: Vec<JsonObject> = cell
            .data
            .get("death_markers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| if let JsonValue::Object(m) = v { Some(m.clone()) } else { None })
                    .collect()
            })
            .unwrap_or_default();
        // HR-786: revealed loot sources (containers/corpses/ground) are takeable
        // alongside death markers. Only `searched` sources contribute.
        let mut sources = loot_source::read_sources(&cell.data);

        // A takeable item, tagged with where it lives so removal touches the right
        // structure. `raw` is the faithful stored JSON carried into inventory.
        #[derive(Clone, Copy)]
        enum Loc {
            Marker(usize, usize),
            Source(usize, usize),
        }
        let mut available: Vec<(Loc, InventoryItemRecord, JsonObject)> = Vec::new();
        for (mi, marker) in markers.iter().enumerate() {
            if let Some(items) = marker.get("items").and_then(|v| v.as_array()) {
                for (ii, raw) in items.iter().enumerate() {
                    if let JsonValue::Object(obj) = raw {
                        if let Ok(item) = serde_json::from_value::<InventoryItemRecord>(raw.clone()) {
                            available.push((Loc::Marker(mi, ii), item, obj.clone()));
                        }
                    }
                }
            }
        }
        for (si, source) in sources.iter().enumerate() {
            if !source.searched {
                continue;
            }
            for (ii, raw) in source.contents.iter().enumerate() {
                if let Ok(item) =
                    serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone()))
                {
                    available.push((Loc::Source(si, ii), item, raw.clone()));
                }
            }
        }

        if available.is_empty() {
            return Ok(vec![self.narrate_msg("There's nothing here to pick up.")]);
        }
        if cmd.args.is_empty() {
            let names: Vec<String> = available.iter().map(|(_, i, _)| i.name.clone()).collect();
            return Ok(vec![self.narrate_msg(format!(
                "Items here: {}. Use 'take <item name>'.",
                names.join(", ")
            ))]);
        }

        let item_name = cmd.args.join(" ").to_lowercase();
        let idx = available
            .iter()
            .position(|(_, i, _)| i.name.to_lowercase() == item_name)
            .or_else(|| {
                available
                    .iter()
                    .position(|(_, i, _)| i.name.to_lowercase().contains(&item_name))
            });
        let idx = match idx {
            Some(i) => i,
            None => {
                return Ok(vec![
                    self.narrate_msg(format!("You don't see '{}' here.", item_name))
                ])
            }
        };
        let (loc, found_name, taken_item) = {
            let (l, item, raw) = &available[idx];
            (*l, item.name.clone(), raw.clone())
        };

        // Clone the full cell payload so writing back does not clobber other cell
        // state; only the affected loot structure changes.
        let mut cell_data_update = cell.data.clone();
        // HR-786: when taking from a loot source, re-emit its remaining contents
        // so the loot panel updates (empty items ⇒ the panel drops the source).
        let mut source_update: Option<GameEvent> = None;
        match loc {
            Loc::Marker(found_mi, found_ii) => {
                let updated_markers: Vec<JsonValue> = markers
                    .iter()
                    .enumerate()
                    .filter_map(|(mi, marker)| {
                        let items: Vec<JsonValue> = marker
                            .get("items")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .enumerate()
                                    .filter(|(ii, _)| !(mi == found_mi && *ii == found_ii))
                                    .map(|(_, v)| v.clone())
                                    .collect()
                            })
                            .unwrap_or_default();
                        let gold = marker.get("gold").and_then(|v| v.as_i64()).unwrap_or(0);
                        if items.is_empty() && gold <= 0 {
                            None
                        } else {
                            let mut m = marker.clone();
                            m.insert("items".into(), JsonValue::Array(items));
                            Some(JsonValue::Object(m))
                        }
                    })
                    .collect();
                cell_data_update.insert("death_markers".into(), JsonValue::Array(updated_markers));
            }
            Loc::Source(si, ii) => {
                let (src_id, src_kind, src_name) = {
                    let s = &sources[si];
                    (s.id.clone(), s.kind.clone(), s.name.clone())
                };
                if ii < sources[si].contents.len() {
                    sources[si].contents.remove(ii);
                }
                // The post-take view: remaining contents, or empty if exhausted.
                let (items, gold) = if sources[si].has_loot() {
                    (sources[si].contents.clone(), sources[si].gold)
                } else {
                    sources.remove(si); // drop a source once it holds nothing
                    (Vec::new(), 0)
                };
                source_update = Some(self.loot_source_revealed_event(
                    coord, &src_id, &src_kind, &src_name, items, gold,
                ));
                loot_source::write_sources(&mut cell_data_update, &sources);
            }
        }

        // HR-772/784: a successful take must PERSIST — add to inventory and clear
        // from the ground in a single transaction so a mid-way failure cannot
        // leave the item in both places.
        char.equipment.push(taken_item.clone());
        db.transaction(|| {
            EntityRepository::new(db).save_character(&char, false, false, false)?;
            CellRepository::new(db).save_cell_data(coord.q as i64, coord.r as i64, &cell_data_update)
        })?;

        // HR-793: emit the client-facing `inventory.item_given` notice.
        let mut events = vec![self.narrate_msg(format!("You pick up {}.", found_name))];
        events.extend(self.item_given_events(&char.id, &[taken_item]));
        // HR-786: refresh the loot panel for the affected source.
        if let Some(ev) = source_update {
            events.push(ev);
        }
        Ok(events)
    }

    fn handle_rest(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let mut events = self.on_tick(db, char.position_q, char.position_r);
        let until_healed = cmd.raw.to_lowercase().contains("until healed");

        if until_healed && char.hp >= char.max_hp {
            events.push(self.narrate_msg("You are already at full health."));
            return Ok(events);
        }

        // Apply healing rules from HealingSystem:
        //   plain `rest`        → full rest → level + CON-mod HP (min 1)
        //   `rest until healed` → restore to max_hp
        let mut healed = char.clone();
        let healing_result = if until_healed {
            let hp_restored = healed.max_hp - healed.hp;
            healed.hp = healed.max_hp;
            let narration = format!(
                "You rest until your wounds close. HP fully restored ({}/{}).",
                healed.hp, healed.max_hp
            );
            crate::engine_results::HealingResult {
                healed: hp_restored > 0,
                hp_restored,
                new_hp: healed.hp,
                max_hp: healed.max_hp,
                method: "rest".into(),
                cost: 0,
                skill_check_roll: None,
                skill_check_total: None,
                skill_check_difficulty: None,
                narration,
            }
        } else {
            // Full-rest tick value gives `level + CON-mod` HP, per healing.rs.
            HealingSystem.rest(&mut healed, crate::healing::FULL_REST_TICKS)
        };

        // Only persist and notify when HP actually changed. A plain `rest` at full
        // HP heals nothing (`healed = false`), so writing + emitting would be noise.
        if healing_result.healed {
            // Persist the healed HP so it survives the next `load_character`.
            // `handle_rest` is the single entry point for rest-based recovery.
            let _ = EntityRepository::new(db).save_character(&healed, false, true, false);

            // HR-793: rest's structured result reaches the client via the
            // client-facing `character.hp_changed` notice below (HP bar update) +
            // the narration. The old `exploration.rest_requested` event ended in
            // `_requested`, was filtered by `resolve_domain_events`, had no
            // subscriber, and carried nothing `character.hp_changed` did not —
            // so it was removed as dead weight.
            let hp_notice = CharacterHpChangedNotice {
                hp: healed.hp,
                max_hp: healed.max_hp,
            };
            let mut hp_ev = GameEvent::new(
                self.tick,
                "character.hp_changed",
                to_json_object(&hp_notice),
            );
            hp_ev.source = "exploration".to_string();
            events.push(hp_ev);
        }

        events.push(self.narrate_msg(&healing_result.narration));
        Ok(events)
    }

    fn handle_wait(&self) -> Vec<GameEvent> {
        vec![self.narrate_msg("Time passes...")]
    }

    fn handle_save(&self) -> Vec<GameEvent> {
        let mut ev = GameEvent::new(self.tick, "world.save_requested", JsonObject::new());
        ev.source = "exploration".to_string();
        vec![ev, self.narrate_msg("World saved.")]
    }

    fn handle_quit(&self) -> Vec<GameEvent> {
        let mut ev = GameEvent::new(self.tick, "world.quit_requested", JsonObject::new());
        ev.source = "exploration".to_string();
        vec![ev, self.narrate_msg("Goodbye.")]
    }

    fn handle_oracle(
        &self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let raw = cmd.args.join(" ");
        if raw.is_empty() || raw.to_lowercase() == "help" {
            return Ok(vec![self.narrate_msg(
                "Usage: oracle <question> (<likelihood>)\n\
                 Likelihoods: impossible, no_way, very_unlikely, unlikely, \
                 even_odds, somewhat_likely, likely, very_likely, certain\n\
                 Example: oracle is there a guard? (likely)",
            )]);
        }
        let mut likelihood = "even_odds".to_string();
        let mut question = raw.clone();
        if let (Some(s), Some(e)) = (raw.rfind('('), raw.rfind(')')) {
            if s < e {
                let candidate = raw[s + 1..e].trim().to_lowercase().replace(' ', "_");
                question = raw[..s].trim().to_string();
                let valid = [
                    "impossible", "no_way", "very_unlikely", "unlikely", "even_odds",
                    "somewhat_likely", "likely", "very_likely", "certain",
                ];
                if valid.contains(&candidate.as_str()) {
                    likelihood = candidate;
                }
            }
        }
        let chaos = GMStateRepository::new(db)
            .get_int("oracle_chaos_factor", 5)? as i32;
        let mut rng = rand::thread_rng();
        let mut checker = FateChecker::new();
        let result = checker.check(&likelihood, chaos, &mut rng)
            .map_err(|e| e.to_string())?;
        let narr = format!("**{}**\n{}", question, result.narration);
        let fate_notice = OracleFateCheckNotice {
            question: question.to_string(),
            likelihood: result.likelihood.clone(),
            chaos_factor: result.chaos_factor,
            roll: result.roll,
        };
        let mut oracle_ev = GameEvent::new(self.tick, "oracle.fate_check", to_json_object(&fate_notice));
        oracle_ev.source = "oracle".to_string();
        Ok(vec![self.narrate_msg(narr), oracle_ev])
    }

    fn handle_oracle_management(
        &self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let mut args = cmd.args.clone();
        let usage = "Oracle thread & plotline commands:\n\
                     add thread <title>      — track a new story thread\n\
                     list threads            — show active threads\n\
                     resolve thread <id>     — mark a thread as resolved\n\
                     add npc <name>          — add an NPC to the oracle cast\n\
                     list npcs               — show tracked NPCs\n\
                     remove npc <id>         — remove an NPC from the list\n\
                     create plotline <title> <theme> — start a new plotline\n\
                     list plotlines          — show active plotlines\n\
                     advance plotline <id>   — draw the next plotline scene\n\
                     resolve plotline <id>   — mark a plotline complete";
        if args.is_empty() {
            return Ok(vec![self.narrate_msg(usage)]);
        }
        let mut target = args.remove(0).to_lowercase();
        if target == "thread" { target = "threads".to_string(); }
        if target == "npc" { target = "npcs".to_string(); }
        if target == "plotline" { target = "plotlines".to_string(); }

        let sub = args.first().map(|a| a.to_lowercase());
        let tracker = ThreadTracker::new(db);
        match target.as_str() {
            "threads" => match cmd.verb.as_str() {
                "add" => {
                    let title = args.join(" ");
                    if title.is_empty() {
                        return Ok(vec![self.narrate_msg("Usage: add thread <title>")]);
                    }
                    let thread = tracker.add_thread(&title, "story").map_err(|e| e)?;
                    Ok(vec![self.narrate_msg(format!("Thread added: {} (id: {})", thread.title, thread.id))])
                }
                "list" => {
                    let threads = tracker.list_threads(Some("active")).map_err(|e| e)?;
                    if threads.is_empty() {
                        Ok(vec![self.narrate_msg("No active threads.")])
                    } else {
                        let lines: Vec<String> = threads.iter().map(|t| format!("  [{}] {}", t.id, t.title)).collect();
                        Ok(vec![self.narrate_msg(format!("Active threads:\n{}", lines.join("\n")))])
                    }
                }
                "resolve" => {
                    let id = args.join(" ");
                    match tracker.resolve_thread(&id).map_err(|e| e)? {
                        Some(t) => Ok(vec![self.narrate_msg(format!("Thread resolved: {}", t.title))]),
                        None => Ok(vec![self.narrate_msg(format!("Thread '{}' not found.", id))]),
                    }
                }
                _ => Ok(vec![self.narrate_msg(usage)]),
            },
            "npcs" => match cmd.verb.as_str() {
                "add" => {
                    let name = args.join(" ");
                    if name.is_empty() {
                        return Ok(vec![self.narrate_msg("Usage: add npc <name>")]);
                    }
                    let npc = tracker.add_npc(&name, "", None).map_err(|e| e)?;
                    Ok(vec![self.narrate_msg(format!("NPC added: {} (id: {})", npc.name, npc.id))])
                }
                "list" => {
                    let npcs = tracker.list_npcs(Some("active")).map_err(|e| e)?;
                    if npcs.is_empty() {
                        Ok(vec![self.narrate_msg("No tracked NPCs.")])
                    } else {
                        let lines: Vec<String> = npcs.iter().map(|n| format!("  [{}] {}", n.id, n.name)).collect();
                        Ok(vec![self.narrate_msg(format!("Tracked NPCs:\n{}", lines.join("\n")))])
                    }
                }
                "remove" => {
                    let id = args.join(" ");
                    let removed = tracker.remove_npc(&id).map_err(|e| e)?;
                    if removed {
                        Ok(vec![self.narrate_msg(format!("NPC '{}' removed.", id))])
                    } else {
                        Ok(vec![self.narrate_msg(format!("NPC '{}' not found.", id))])
                    }
                }
                _ => Ok(vec![self.narrate_msg(usage)]),
            },
            "plotlines" => {
                use crate::adventure_crafter::AdventureCrafter;
                let mut rng = rand::thread_rng();
                let mut crafter = AdventureCrafter::new(db, &mut rng);
                match cmd.verb.as_str() {
                    "create" => {
                        if args.len() < 2 {
                            return Ok(vec![self.narrate_msg("Usage: create plotline <title> <theme>")]);
                        }
                        let theme = args.pop().unwrap_or_default();
                        let title = args.join(" ");
                        let pl = crafter.create_plotline(&title, &theme).map_err(|e| e)?;
                        Ok(vec![self.narrate_msg(format!("Plotline created: {} [{}] (id: {})", pl.title, pl.theme, pl.id))])
                    }
                    "list" => {
                        let pls = crafter.list_plotlines(Some("active")).map_err(|e| e)?;
                        if pls.is_empty() {
                            Ok(vec![self.narrate_msg("No active plotlines.")])
                        } else {
                            let lines: Vec<String> = pls.iter().map(|p| format!("  [{}] {} [{}]", p.id, p.title, p.theme)).collect();
                            Ok(vec![self.narrate_msg(format!("Active plotlines:\n{}", lines.join("\n")))])
                        }
                    }
                    "advance" => {
                        let id = args.join(" ");
                        match crafter.advance_plotline(&id).map_err(|e| e)? {
                            Some(scene) => Ok(vec![self.narrate_msg(format!("Plotline scene: {}", scene.narration))]),
                            None => Ok(vec![self.narrate_msg(format!("Plotline '{}' not found or complete.", id))]),
                        }
                    }
                    "resolve" => {
                        let id = args.join(" ");
                        let resolved = crafter.resolve_plotline(&id).map_err(|e| e)?;
                        if resolved {
                            Ok(vec![self.narrate_msg(format!("Plotline '{}' resolved.", id))])
                        } else {
                            Ok(vec![self.narrate_msg(format!("Plotline '{}' not found.", id))])
                        }
                    }
                    _ => Ok(vec![self.narrate_msg(usage)]),
                }
            }
            _ => Ok(vec![self.narrate_msg(usage)]),
        }
    }

    fn handle_travel(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let (tq, tr, action, skill) = match parse_travel_args(&cmd.args) {
            Ok(v) => v,
            Err(msg) => return Ok(vec![self.narrate_msg(msg)]),
        };
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let start = GridCoord { q: char.position_q, r: char.position_r };
        let goal_coord = GridCoord { q: tq, r: tr };
        if start == goal_coord {
            return Ok(vec![self.narrate_msg("You are already there.")]);
        }
        let passable = CellRepository::new(db)
            .list_passable_coords(&self.terrain_registry)
            .unwrap_or_default();
        let path = find_travel_path(start, goal_coord, &|c| passable.contains(&(c.q as i64, c.r as i64)), 20000);
        match path {
            None => Ok(vec![self.narrate_msg("No passable route found to that location.")]),
            Some(steps) if steps.len() <= 1 => Ok(vec![self.narrate_msg("You are already there.")]),
            Some(steps) => {
                // HR-408: walk the whole path this turn (one `travel.step` per cell),
                // stopping on arrival, a staged encounter, or a blocked cell. The
                // goal is persisted so an encounter-interrupted trip can be resumed.
                let goal = TravelGoal {
                    target_q: tq,
                    target_r: tr,
                    action,
                    skill,
                    status: TravelStatus::Active,
                };
                self.save_travel_goal(db, &goal);
                self.walk_journey(db, steps, &goal, false)
            }
        }
    }

    fn handle_resume_travel(
        &mut self,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let goal = match self.load_travel_goal(db) {
            Some(g) if g.status == TravelStatus::Interrupted => g,
            _ => return Ok(vec![self.narrate_msg("No interrupted journey to resume.")]),
        };
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let start = GridCoord { q: char.position_q, r: char.position_r };
        let target = GridCoord { q: goal.target_q, r: goal.target_r };
        if start == target {
            // Already standing on the target (e.g. the interrupting encounter spawned
            // on the destination cell). Complete the journey AND clear the banner.
            self.clear_travel_goal(db);
            return Ok(vec![
                self.travel_end_event("travel.completed", goal.target_q, goal.target_r),
                self.narrate_msg("You have arrived."),
            ]);
        }
        let passable = CellRepository::new(db)
            .list_passable_coords(&self.terrain_registry)
            .unwrap_or_default();
        let path = find_travel_path(start, target, &|c| passable.contains(&(c.q as i64, c.r as i64)), 20000);
        match path {
            None => {
                self.clear_travel_goal(db);
                Ok(vec![
                    self.travel_end_event("travel.cancelled", goal.target_q, goal.target_r),
                    self.narrate_msg("No route to destination — journey cancelled."),
                ])
            }
            Some(steps) if steps.len() <= 1 => {
                self.clear_travel_goal(db);
                Ok(vec![
                    self.travel_end_event("travel.completed", goal.target_q, goal.target_r),
                    self.narrate_msg("You have arrived."),
                ])
            }
            Some(steps) => {
                let mut updated = goal.clone();
                updated.status = TravelStatus::Active;
                self.save_travel_goal(db, &updated);
                self.walk_journey(db, steps, &updated, true)
            }
        }
    }

    fn handle_cancel_travel(&self, db: &WorldDatabase) -> Vec<GameEvent> {
        let Some(goal) = self.load_travel_goal(db) else {
            return vec![self.narrate_msg("No journey to cancel.")];
        };
        self.clear_travel_goal(db);
        let mut ev = GameEvent::new(
            self.tick,
            "travel.cancelled",
            to_json_object(&crate::payloads::notices_world::TravelEndNotice {
                target_q: goal.target_q,
                target_r: goal.target_r,
            }),
        );
        ev.source = "exploration".to_string();
        vec![ev, self.narrate_msg("Journey cancelled.")]
    }

    /// Build a journey-ending notice (`travel.completed` / `travel.cancelled` /
    /// `travel.blocked`). The frontend clears its resume banner on any of these, so
    /// every path that ends a journey must emit one — including the "already there"
    /// short-circuits, or the banner lingers on a target the player is standing on.
    fn travel_end_event(&self, event_type: &'static str, tq: i32, tr: i32) -> GameEvent {
        let mut ev = GameEvent::new(
            self.tick,
            event_type,
            to_json_object(&crate::payloads::notices_world::TravelEndNotice {
                target_q: tq,
                target_r: tr,
            }),
        );
        ev.source = "exploration".to_string();
        ev
    }

    /// Advance the player one cell along a journey: persist the new position,
    /// reveal the cell, emit a compact `travel.step` notice, then run the same
    /// cell-entry side effects a manual move does (weather tick, IR `enter_hex`
    /// triggers, and an encounter roll). A hostile encounter sets
    /// `self.pending_combat`, which `walk_journey` reads to interrupt the trip.
    ///
    /// CONSTRAINT (HR-408): `char` is a start-of-journey snapshot passed down by
    /// `walk_journey`; this saves it with only the position mutated. The current
    /// per-cell side effects never mutate the player's persisted HP/inventory
    /// *during* the loop (IR triggers run post-scene on the fresh DB row), so the
    /// snapshot is safe. If you add per-step HP/inventory effects here (terrain
    /// damage, fatigue, poison ticks), reload the character or mutate `walker`
    /// in `walk_journey` between steps — otherwise a later step's save clobbers them.
    fn travel_step(
        &mut self,
        db: &WorldDatabase,
        char: &crate::character::Character,
        from: GridCoord,
        to: GridCoord,
    ) -> Result<Vec<GameEvent>, String> {
        let grid = SquareGrid::default();
        let target_cell = match self.fetch_cell(db, to)? {
            Some(c) => c,
            None => return Ok(Vec::new()),
        };
        let first_visit = !target_cell.explored;
        let direction = grid
            .directions()
            .into_iter()
            .find(|d| grid.neighbor(from, d).ok() == Some(to))
            .map(|d| d.to_string())
            .unwrap_or_else(|| "northeast".to_string());

        // Persist the new position + reveal the cell.
        {
            let mut moved = char.clone();
            moved.position_q = to.q;
            moved.position_r = to.r;
            let _ = EntityRepository::new(db).save_character(&moved, true, false, false);
        }
        let _ = CellRepository::new(db).mark_explored(to.q as i64, to.r as i64, 1);
        self.current_terrain = Some(target_cell.terrain.clone());
        self.current_features = target_cell.features.clone();

        use crate::payloads::notices_world::{CellPreview, TravelStepNotice};
        let payload = TravelStepNotice {
            character_id: char.id.clone(),
            from_q: from.q,
            from_r: from.r,
            to_q: to.q,
            to_r: to.r,
            direction,
            first_visit,
            target_cell: CellPreview {
                q: to.q,
                r: to.r,
                terrain: target_cell.terrain.clone(),
                features: target_cell.features.clone(),
                explored: JsonValue::Bool(true),
            },
        };
        let mut events = Vec::new();
        let mut ev = GameEvent::new(self.tick, "travel.step", to_json_object(&payload));
        ev.source = "exploration".to_string();
        events.push(ev);

        events.extend(self.on_tick(db, to.q, to.r));
        events.push(enter_hex_event(
            self.tick,
            &char.id,
            &target_cell.terrain,
            &target_cell.features,
            to,
            first_visit,
        ));
        events.extend(self.maybe_spawn_encounter(db, char, &target_cell, to, !first_visit));
        Ok(events)
    }

    /// Walk the full path, emitting one `travel.step` per cell, until arrival
    /// (`travel.completed` + arrival action), a staged encounter
    /// (`travel.interrupted`; goal marked `Interrupted` for resume), or an
    /// impassable cell (`travel.blocked`). `resumed` selects the opening event.
    fn walk_journey(
        &mut self,
        db: &WorldDatabase,
        steps: Vec<GridCoord>,
        goal: &TravelGoal,
        resumed: bool,
    ) -> Result<Vec<GameEvent>, String> {
        use crate::payloads::notices_world::{
            TravelEndNotice, TravelInterruptedNotice, TravelJourneyNotice,
        };
        let char = match self.load_character(db)? {
            Some(c) => c,
            None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
        };
        let mut events = Vec::new();

        let start_type = if resumed { "travel.resumed" } else { "travel.started" };
        let mut sev = GameEvent::new(
            self.tick,
            start_type,
            to_json_object(&TravelJourneyNotice {
                target_q: goal.target_q,
                target_r: goal.target_r,
                steps_remaining: steps.len().saturating_sub(1) as i32,
            }),
        );
        sev.source = "exploration".to_string();
        events.push(sev);
        events.push(self.narrate_msg(format!(
            "You {} toward ({}, {}).",
            if resumed { "resume your journey" } else { "set out" },
            goal.target_q,
            goal.target_r
        )));

        let passable = CellRepository::new(db)
            .list_passable_coords(&self.terrain_registry)
            .unwrap_or_default();
        let target = GridCoord { q: goal.target_q, r: goal.target_r };
        let mut walker = char.clone();
        for window in steps.windows(2) {
            let (from, to) = (window[0], window[1]);
            if !passable.contains(&(to.q as i64, to.r as i64)) {
                self.clear_travel_goal(db);
                let mut bev = GameEvent::new(
                    self.tick,
                    "travel.blocked",
                    to_json_object(&TravelEndNotice { target_q: goal.target_q, target_r: goal.target_r }),
                );
                bev.source = "exploration".to_string();
                events.push(bev);
                events.push(self.narrate_msg("The way ahead is blocked. Your journey ends here."));
                return Ok(events);
            }
            walker.position_q = from.q;
            walker.position_r = from.r;
            events.extend(self.travel_step(db, &walker, from, to)?);
            walker.position_q = to.q;
            walker.position_r = to.r;

            // Arrival takes precedence over interruption: if the encounter spawned on
            // the destination cell, the journey is still complete (the player reached
            // the target) — the encounter becomes a normal in-place exploration
            // encounter. Interrupting here would leave a "resume" banner for a target
            // the player is already standing on, which never clears.
            if to == target {
                break;
            }
            if self.pending_combat.is_some() {
                let mut interrupted = goal.clone();
                interrupted.status = TravelStatus::Interrupted;
                self.save_travel_goal(db, &interrupted);
                let mut iev = GameEvent::new(
                    self.tick,
                    "travel.interrupted",
                    to_json_object(&TravelInterruptedNotice {
                        target_q: goal.target_q,
                        target_r: goal.target_r,
                        reason: "encounter".to_string(),
                    }),
                );
                iev.source = "exploration".to_string();
                events.push(iev);
                events.push(self.narrate_msg(
                    "Your journey is interrupted! (Resume once the way is clear.)",
                ));
                return Ok(events);
            }
        }

        // Arrived.
        self.clear_travel_goal(db);
        let mut cev = GameEvent::new(
            self.tick,
            "travel.completed",
            to_json_object(&TravelEndNotice { target_q: goal.target_q, target_r: goal.target_r }),
        );
        cev.source = "exploration".to_string();
        events.push(cev);
        events.push(self.narrate_msg(format!("You arrive at ({}, {}).", goal.target_q, goal.target_r)));
        let dest = GridCoord { q: goal.target_q, r: goal.target_r };
        events.extend(self.arrival_action(db, goal, dest)?);
        Ok(events)
    }

    /// Perform the journey's arrival action at the destination cell.
    fn arrival_action(
        &mut self,
        db: &WorldDatabase,
        goal: &TravelGoal,
        dest: GridCoord,
    ) -> Result<Vec<GameEvent>, String> {
        match goal.action {
            TravelAction::Enter => {
                let cmd = ParsedCommand {
                    verb: "enter".to_string(),
                    args: Vec::new(),
                    raw: "enter".to_string(),
                    direction: None,
                };
                self.handle_enter(&cmd, db)
            }
            // Skill-modified travel (right-click "use skill") is a niche follow-up;
            // arrival falls back to a look. The skill id is preserved on the goal.
            TravelAction::Skill | TravelAction::Move | TravelAction::Look => {
                Ok(self.look_internal(db, dest))
            }
        }
    }

    /// Build the `exploration.actions` notice for the player's current tile.
    ///
    /// Returns typed action buttons based on terrain/features (mirrors
    /// `get_suggestions` logic but excludes directions and meta commands).
    fn exploration_actions_notice(&self) -> ExplorationActionsNotice {
        let terrain = self.current_terrain.as_deref().unwrap_or("");
        let features = &self.current_features;

        let mut actions: Vec<CombatActionButton> =
            if terrain == "settlement" || features.iter().any(|f| f == "settlement") {
                vec![
                    CombatActionButton {
                        command: "enter".into(),
                        label: "Enter".into(),
                        style: "primary".into(),
                    },
                    CombatActionButton {
                        command: "talk".into(),
                        label: "Talk".into(),
                        style: "default".into(),
                    },
                    CombatActionButton {
                        command: "shop".into(),
                        label: "Shop".into(),
                        style: "default".into(),
                    },
                    CombatActionButton {
                        command: "search".into(),
                        label: "Search".into(),
                        style: "default".into(),
                    },
                    CombatActionButton {
                        command: "look".into(),
                        label: "Look".into(),
                        style: "default".into(),
                    },
                ]
            } else if terrain == "ruins" || features.iter().any(|f| f == "ruins") {
                vec![
                    CombatActionButton {
                        command: "search".into(),
                        label: "Search".into(),
                        style: "default".into(),
                    },
                    CombatActionButton {
                        command: "look".into(),
                        label: "Look".into(),
                        style: "default".into(),
                    },
                ]
            } else if features.iter().any(|f| f == "lair" || f == "dungeon") {
                vec![
                    CombatActionButton {
                        command: "enter".into(),
                        label: "Enter".into(),
                        style: "primary".into(),
                    },
                    CombatActionButton {
                        command: "search".into(),
                        label: "Search".into(),
                        style: "default".into(),
                    },
                    CombatActionButton {
                        command: "look".into(),
                        label: "Look".into(),
                        style: "default".into(),
                    },
                ]
            } else {
                // Wilderness / default
                vec![
                    CombatActionButton {
                        command: "look".into(),
                        label: "Look".into(),
                        style: "default".into(),
                    },
                    CombatActionButton {
                        command: "search".into(),
                        label: "Search".into(),
                        style: "default".into(),
                    },
                ]
            };

        // Always append Rest when the character is injured.
        if self.current_char_hp > 0
            && self.current_char_max_hp > 0
            && self.current_char_hp < self.current_char_max_hp
        {
            actions.push(CombatActionButton {
                command: "rest".into(),
                label: "Rest".into(),
                style: "default".into(),
            });
        }

        ExplorationActionsNotice { actions }
    }

    fn handle_pre_combat_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        let pending = match self.pending_combat.clone() {
            Some(p) => p,
            None => return Ok(vec![self.narrate_msg("No pending encounter.")]),
        };
        let awareness = &pending.awareness_result;
        let verb = cmd.verb.as_str();
        let raw = cmd.raw.trim().to_lowercase();

        if verb == "attack" || raw == "attack" {
            self.pending_combat_transition = true;
            return Ok(Vec::new());
        }
        if verb == "flee" || raw.starts_with("flee") {
            self.pending_combat = None;
            let cleared = self.cleared_actions_event();
            return Ok(vec![
                self.narrate_msg("You back away carefully and avoid the encounter."),
                cleared,
            ]);
        }
        if verb == "avoid" || raw.starts_with("avoid") {
            self.pending_combat = None;
            let cleared = self.cleared_actions_event();
            return Ok(vec![
                self.narrate_msg("You slip away unnoticed."),
                cleared,
            ]);
        }
        if verb == "sneak" || raw.starts_with("sneak") {
            self.pending_combat = None;
            let cleared = self.cleared_actions_event();
            return Ok(vec![
                self.narrate_msg("You attempt to sneak past... and succeed."),
                cleared,
            ]);
        }
        if verb == "talk" || raw.starts_with("talk") {
            let can_talk = pending.creatures.iter().any(|c| {
                c.get("tags")
                    .and_then(|v| v.as_array())
                    .map_or(false, |arr| arr.iter().any(|t| t.as_str() == Some("humanoid")))
            });
            if !can_talk {
                return Ok(vec![self.narrate_msg("You can't reason with these creatures!")]);
            }
            self.pending_combat = None;
            let cleared = self.cleared_actions_event();
            return Ok(vec![
                self.narrate_msg(
                    "You hold up your hands in a gesture of peace. The encounter backs off — for now.",
                ),
                cleared,
            ]);
        }
        if verb == "status" {
            let char = match self.load_character(db)? {
                Some(c) => c,
                None => return Ok(vec![self.narrate_msg("[ERROR] No character found.")]),
            };
            let suffix = match awareness.as_str() {
                "player_surprise" => ", avoid, sneak",
                "mutual_awareness" => ", talk",
                _ => "",
            };
            return Ok(vec![self.narrate_msg(format!(
                "HP: {}/{}  AC: {}\nPending encounter — choose: attack, flee{}",
                char.hp, char.max_hp, char.ac, suffix
            ))]);
        }
        if verb == "help" {
            let help = match awareness.as_str() {
                "player_surprise" => {
                    "You have the element of surprise!\n  attack — Launch your assault\n  avoid  — Slip away quietly\n  sneak  — Attempt to sneak past"
                }
                "mutual_awareness" => {
                    "Both sides are aware. Choose quickly:\n  attack — Fight!\n  flee   — Back away\n  talk   — Attempt to de-escalate"
                }
                _ => "You were caught off guard!\n  attack — Fight back!\n  flee   — Run!",
            };
            return Ok(vec![self.narrate_msg(help)]);
        }
        Ok(vec![self.narrate_msg(format!(
            "Unknown command in encounter state. Options: {}",
            self.get_valid_commands().join(", ")
        ))])
    }
}

fn parse_travel_args(
    args: &[String],
) -> Result<(i32, i32, TravelAction, Option<String>), &'static str> {
    if args.len() < 2 {
        return Err("Usage: travel <q> <r> [action]");
    }
    let tq = args[0].parse::<i32>().map_err(|_| "Invalid column (q) — must be a number.")?;
    let tr = args[1].parse::<i32>().map_err(|_| "Invalid row (r) — must be a number.")?;
    let action = match args.get(2).map(|a| a.to_lowercase()).as_deref() {
        Some("look") => TravelAction::Look,
        Some("enter") => TravelAction::Enter,
        Some("skill") => TravelAction::Skill,
        _ => TravelAction::Move,
    };
    let skill = if action == TravelAction::Skill {
        args.get(3).cloned()
    } else {
        None
    };
    Ok((tq, tr, action, skill))
}

/// Build the `exploration.enter_hex` event for a hex entry. `self_id` is the
/// player's entity id so the trigger runtime sources the player's IR triggers;
/// `terrain`/`features`/coords are exposed for trigger `when` conditions.
fn enter_hex_event(
    tick: i64,
    char_id: &str,
    terrain: &str,
    features: &[String],
    coord: GridCoord,
    first_visit: bool,
) -> GameEvent {
    let notice = ExplorationEnterHexNotice {
        self_id: char_id.to_string(),
        terrain: terrain.to_string(),
        features: features.to_vec(),
        q: coord.q,
        r: coord.r,
        first_visit,
    };
    let mut ev = GameEvent::new(tick, "exploration.enter_hex", to_json_object(&notice));
    ev.source = "exploration".to_string();
    ev
}

/// The string key persisted in [`PendingEncounterState`] for an awareness outcome.
fn awareness_result_key(result: AwarenessResult) -> &'static str {
    match result {
        AwarenessResult::PlayerSurprise => "player_surprise",
        AwarenessResult::EnemySurprise => "enemy_surprise",
        AwarenessResult::MutualAwareness => "mutual_awareness",
    }
}

/// Player action choices offered for a staged encounter, keyed by awareness.
/// Player surprise enables avoid/sneak; mutual awareness enables talk; enemy
/// surprise leaves only attack/flee.
fn encounter_choices_for(awareness_key: &str) -> Vec<&'static str> {
    match awareness_key {
        "player_surprise" => vec!["attack", "avoid", "sneak"],
        "mutual_awareness" => vec!["attack", "flee", "talk"],
        _ => vec!["attack", "flee"],
    }
}

/// Build the `CombatActionsNotice` for the pre-combat menu shown as soon as a
/// hostile encounter is staged (before the player picks attack/flee/talk).
///
/// The button list mirrors `encounter_choices_for` but as typed action buttons.
/// `can_talk` suppresses the talk button when the creatures have no "humanoid" tag.
fn pre_combat_actions_notice(awareness: &str, can_talk: bool) -> CombatActionsNotice {
    let choices = encounter_choices_for(awareness);
    let mut actions: Vec<CombatActionButton> = choices
        .into_iter()
        .filter_map(|choice| {
            let (command, label, style) = match choice {
                "attack" => ("attack", "Attack", "primary"),
                "flee"   => ("flee",   "Flee",   "default"),
                "avoid"  => ("avoid",  "Avoid",  "default"),
                "sneak"  => ("sneak",  "Sneak Past", "default"),
                "talk"   => ("talk",   "Talk",   "default"),
                _        => return None,
            };
            if choice == "talk" && !can_talk {
                return None;
            }
            Some(CombatActionButton {
                command: command.to_string(),
                label:   label.to_string(),
                style:   style.to_string(),
            })
        })
        .collect();
    // Always ensure the attack button is present even if encounter_choices_for
    // somehow omits it (defensive guard; current data never triggers this).
    if !actions.iter().any(|a| a.command == "attack") {
        actions.insert(0, CombatActionButton {
            command: "attack".to_string(),
            label:   "Attack".to_string(),
            style:   "primary".to_string(),
        });
    }
    CombatActionsNotice {
        phase: "pre_combat".to_string(),
        actions,
    }
}

impl SceneHandler for ExplorationScene {
    fn get_valid_commands(&self) -> Vec<String> {
        if self.pending_combat.is_some() {
            let awareness = self.pending_combat.as_ref().map(|p| p.awareness_result.as_str()).unwrap_or("");
            let can_talk = self.pending_combat.as_ref().map_or(false, |p| {
                p.creatures.iter().any(|c| {
                    c.get("tags")
                        .and_then(|v| v.as_array())
                        .map_or(false, |arr| arr.iter().any(|t| t.as_str() == Some("humanoid")))
                })
            });
            if awareness == "player_surprise" {
                return vec!["attack", "avoid", "sneak", "status", "help"]
                    .into_iter().map(String::from).collect();
            }
            let mut cmds = vec!["attack", "flee", "status", "help"];
            if awareness == "mutual_awareness" && can_talk {
                cmds.push("talk");
            }
            return cmds.into_iter().map(String::from).collect();
        }
        vec![
            "go", "n", "s", "e", "w", "ne", "nw", "se", "sw",
            "look", "l", "search", "enter", "shop", "explore",
            "examine", "talk", "forage", "oracle", "status", "stats",
            "inventory", "inv", "help", "?", "rest", "wait", "save", "quit", "exit",
            "add", "list", "resolve", "create", "advance", "remove",
            "travel", "resume", "cancel",
        ].into_iter().map(String::from).collect()
    }

    fn get_prompt(&self, _db: &WorldDatabase) -> String {
        let terrain = self.current_terrain.as_deref().unwrap_or("wilderness");
        format!("[Exploration — {}] > ", terrain)
    }

    fn handle_command(
        &mut self,
        cmd: &ParsedCommand,
        db: &WorldDatabase,
    ) -> Result<Vec<GameEvent>, String> {
        // Pre-combat state takes precedence over any travel prompt. HR-408: a
        // hostile encounter that interrupts a journey sets BOTH `pending_combat`
        // and an `Interrupted` goal at once; the encounter must be resolved first,
        // otherwise a raw "yes" would resume travel and let the player walk past
        // the encounter one cell at a time (the resume banner only appears after
        // combat, once `pending_combat` is clear).
        //
        // NOTE: exploration.actions is NOT appended during pre-combat — the combat
        // encounter menu takes over. All other paths (including yes/no travel
        // resume) receive the action-bar refresh.
        if self.pending_combat.is_some() {
            return self.handle_pre_combat_command(cmd, db);
        }

        // Intercept yes/no for an interrupted journey (post-combat: no encounter staged).
        if matches!(cmd.verb.as_str(), "yes" | "no") {
            if let Some(goal) = self.load_travel_goal(db) {
                if goal.status == TravelStatus::Interrupted {
                    let mut evs = if cmd.verb == "yes" {
                        self.handle_resume_travel(db)?
                    } else {
                        self.handle_cancel_travel(db)
                    };
                    let notice = self.exploration_actions_notice();
                    let mut act_ev = GameEvent::new(
                        self.tick,
                        "exploration.actions",
                        to_json_object(&notice),
                    );
                    act_ev.source = "exploration".to_string();
                    evs.push(act_ev);
                    return Ok(evs);
                }
            }
        }

        let verb = cmd.verb.as_str();
        let mut result = (match verb {
            "go" | "move" | "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw"
            | "north" | "south" | "east" | "west"
            | "northeast" | "northwest" | "southeast" | "southwest" => {
                let mut move_cmd = cmd.clone();
                if move_cmd.direction.is_none() {
                    // Map verb to direction for single-letter commands
                    let dir = match verb {
                        "n" | "north" => Some("north"),
                        "s" | "south" => Some("south"),
                        "e" | "east" => Some("east"),
                        "w" | "west" => Some("west"),
                        "ne" | "northeast" => Some("northeast"),
                        "nw" | "northwest" => Some("northwest"),
                        "se" | "southeast" => Some("southeast"),
                        "sw" | "southwest" => Some("southwest"),
                        _ => None,
                    };
                    move_cmd.direction = dir.map(String::from);
                }
                self.handle_move(&move_cmd, db)
            }
            "look" | "l" => self.handle_look(cmd, db),
            "status" | "stats" => self.handle_status(cmd, db),
            "inventory" | "inv" => self.handle_inventory(cmd, db),
            "quests" => self.handle_quests(cmd, db),
            "help" | "?" => Ok(self.handle_help()),
            "enter" => self.handle_enter(cmd, db),
            "shop" => self.handle_shop(cmd, db),
            "explore" => self.handle_explore(cmd, db),
            "talk" => self.handle_talk(cmd, db),
            "examine" => self.handle_examine(cmd, db),
            "search" => self.handle_search(cmd, db),
            "take" => self.handle_take(cmd, db),
            "rest" => self.handle_rest(cmd, db),
            "wait" => Ok(self.handle_wait()),
            "save" => Ok(self.handle_save()),
            "quit" | "exit" => Ok(self.handle_quit()),
            "oracle" => self.handle_oracle(cmd, db),
            "add" | "list" | "resolve" | "create" | "advance" | "remove" => {
                self.handle_oracle_management(cmd, db)
            }
            "travel" => self.handle_travel(cmd, db),
            "resume" => self.handle_resume_travel(db),
            "cancel" => Ok(self.handle_cancel_travel(db)),
            other => Ok(vec![self.narrate_msg(format!(
                "Unknown command: '{}'. Type 'help' for a list of commands.",
                other
            ))]),
        })?;

        // Append exploration.actions after every normal (non-pre-combat) command
        // so the action bar refreshes with the current tile's available actions.
        let notice = self.exploration_actions_notice();
        let mut act_ev =
            GameEvent::new(self.tick, "exploration.actions", to_json_object(&notice));
        act_ev.source = "exploration".to_string();
        result.push(act_ev);
        Ok(result)
    }

    fn check_transitions(&self, _events: &[GameEvent]) -> Option<SceneState> {
        if self.pending_combat_transition {
            return Some(SceneState::Combat);
        }
        if self.pending_social_transition {
            return Some(SceneState::Social);
        }
        if self.pending_shopping_transition {
            return Some(SceneState::Shopping);
        }
        if self.pending_dungeon_transition {
            return Some(SceneState::Dungeon);
        }
        if self.pending_town_transition {
            return Some(SceneState::Town);
        }
        None
    }

    fn get_suggestions(&self) -> Vec<String> {
        if let Some(p) = &self.pending_combat {
            return encounter_choices_for(&p.awareness_result)
                .into_iter()
                .map(String::from)
                .collect();
        }

        let dir_abbrevs = [
            ("north", "n"), ("south", "s"), ("east", "e"), ("west", "w"),
            ("northeast", "ne"), ("northwest", "nw"), ("southeast", "se"), ("southwest", "sw"),
        ];
        let dirs: Vec<String> = dir_abbrevs.iter().map(|(_, abbr)| abbr.to_string()).collect();
        let terrain = self.current_terrain.as_deref().unwrap_or("");
        let features = &self.current_features;

        let mut suggestions = if terrain == "settlement" || features.iter().any(|f| f == "settlement") {
            vec!["enter", "talk", "shop", "search", "look"]
        } else if terrain == "ruins" || features.iter().any(|f| f == "ruins") {
            vec!["search", "look"]
        } else if features.iter().any(|f| f == "lair" || f == "dungeon") {
            vec!["enter", "search", "look"]
        } else {
            vec!["look", "search"]
        };
        let mut result: Vec<String> = suggestions.into_iter().map(String::from).collect();
        result.extend(dirs);
        result.extend(["status", "inventory", "help", "oracle"].iter().map(|s| s.to_string()));
        if self.current_char_hp > 0
            && self.current_char_max_hp > 0
            && self.current_char_hp < self.current_char_max_hp
        {
            result.insert(0, "rest until healed".to_string());
            result.insert(1, "rest".to_string());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{TerrainRegistry, TerrainType};
    use crate::db::WorldDatabase;
    use crate::gm::narrator::Narrator;

    fn make_scene() -> ExplorationScene {
        let registry = TerrainRegistry::default();
        let narrator = Narrator::new(TerrainRegistry::default(), std::path::Path::new(""));
        ExplorationScene::new(registry, narrator, 0)
    }

    #[test]
    fn ir_creatures_fold_into_the_live_catalog() {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        // Seed an IR-only creature into the world's ir_records.
        let rec: JsonObject = serde_json::from_value(serde_json::json!({
            "component_type": "creature", "id": "ir_wraith", "name": "IR Wraith",
            "hd": 2, "ac": 13, "attack_bonus": 2, "damage": "1d6", "tags": ["ruins"]
        }))
        .unwrap();
        IRRecordRepository::new(&db).upsert_many(&[rec], "test").unwrap();

        // The scene's narrator content dir is empty, so the legacy catalog is
        // empty; the IR creature must still appear after folding.
        let mut scene = make_scene();
        scene.ensure_full_catalog(&db);
        let reg = scene.creature_registry.as_ref().expect("catalog loaded");
        assert!(reg.get("ir_wraith").is_some(), "IR creature should be encounterable");
    }

    fn db() -> WorldDatabase {
        WorldDatabase::open_in_memory().unwrap()
    }

    #[test]
    fn get_valid_commands_normal() {
        let scene = make_scene();
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"look".to_string()));
        assert!(cmds.contains(&"status".to_string()));
        assert!(cmds.contains(&"help".to_string()));
    }

    fn creature(id: &str, tags: &[&str]) -> CreatureData {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "hd": 1, "ac": 12, "attack_bonus": 1, "damage": "1d6",
            "tags": tags,
        }))
        .unwrap()
    }

    fn hostile(extra: serde_json::Value) -> EncounterResult {
        let data: crate::engine_runtime::EncounterRecord = serde_json::from_value(serde_json::json!({
            "type": "hostile", "name": "Ambush", "description": "Foes close in.",
        }))
        .unwrap();
        let mut data = data;
        if let serde_json::Value::Object(m) = extra {
            data.extra = m;
        }
        EncounterResult {
            encounter_type: "hostile".into(),
            name: "Ambush".into(),
            description: "Foes close in.".into(),
            entity_id: None,
            data,
        }
    }

    #[test]
    fn select_creatures_prefers_explicit_ids() {
        let mut scene = make_scene();
        scene.creature_registry = Some(CreatureRegistry::new(vec![
            creature("wolf", &["forest", "pack"]),
            creature("rat", &["sewer"]),
        ]));
        let picked = scene.select_encounter_creatures(
            &hostile(serde_json::json!({ "creatures": ["rat"] })),
            "forest",
        );
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].id, "rat");
    }

    #[test]
    fn select_creatures_falls_back_to_terrain_tag() {
        let mut scene = make_scene();
        scene.creature_registry = Some(CreatureRegistry::new(vec![
            creature("wolf", &["forest", "pack"]),
            creature("rat", &["sewer"]),
        ]));
        let picked = scene.select_encounter_creatures(&hostile(serde_json::json!({})), "forest");
        assert!(!picked.is_empty());
        // Only the forest-tagged species is eligible; a pack spawns 2–3.
        assert!(picked.iter().all(|c| c.id == "wolf"));
        assert!((2..=3).contains(&picked.len()));
    }

    #[test]
    fn select_creatures_empty_registry_yields_none() {
        let mut scene = make_scene();
        scene.creature_registry = Some(CreatureRegistry::new(vec![]));
        let picked = scene.select_encounter_creatures(&hostile(serde_json::json!({})), "forest");
        assert!(picked.is_empty());
    }

    #[test]
    fn move_into_cell_can_stage_hostile_combat() {
        use crate::db_schema::SCHEMA_SQL;
        use std::path::PathBuf;

        // Seed an in-memory world with the repo's encounter tables loaded.
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        let content = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
            .join("xwn-core")
            .join("content");
        let mut engine = TableEngine::new(&db, rand::thread_rng());
        engine.load_tables(&content).unwrap();

        let mut scene = make_scene();
        // Inject a forest-tagged creature so a hostile roll can stage combat.
        scene.creature_registry =
            Some(CreatureRegistry::new(vec![creature("feral_dog", &["forest", "pack"])]));

        let character = crate::character::Character::new("Scout", "expert");
        let cell: crate::cell::CellData = serde_json::from_value(serde_json::json!({
            "q": 1, "r": 1, "terrain": "forest", "features": [], "explored": false
        }))
        .unwrap();
        let coord = GridCoord { q: 1, r: 1 };

        // The encounter roll is probabilistic (thread_rng); loop until it stages.
        let mut staged = false;
        for _ in 0..400 {
            let _ = scene.maybe_spawn_encounter(&db, &character, &cell, coord, false);
            if scene.pending_combat.is_some() {
                staged = true;
                break;
            }
        }
        assert!(staged, "a hostile encounter should stage combat within 400 entries");
        let pending = scene.pending_combat.as_ref().unwrap();
        assert!(!pending.creatures.is_empty(), "staged combat has creatures");
        // Awareness is now rolled, so it is one of the three valid outcomes.
        assert!(
            ["player_surprise", "mutual_awareness", "enemy_surprise"]
                .contains(&pending.awareness_result.as_str()),
            "unexpected awareness key: {}",
            pending.awareness_result
        );

        // The staged encounter transitions to combat when the player attacks.
        let attack = ParsedCommand {
            verb: "attack".into(),
            args: vec![],
            raw: "attack".into(),
            direction: None,
        };
        scene.handle_pre_combat_command(&attack, &db).unwrap();
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Combat));
    }

    #[test]
    fn get_valid_commands_pre_combat_player_surprise() {
        let mut scene = make_scene();
        scene.pending_combat = Some(PendingEncounterState {
            creatures: vec![],
            awareness_result: "player_surprise".to_string(),
            terrain: "plains".to_string(),
            features: vec![],
            cell_q: 0,
            cell_r: 0,
        });
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"attack".to_string()));
        assert!(cmds.contains(&"avoid".to_string()));
        assert!(cmds.contains(&"sneak".to_string()));
        assert!(!cmds.contains(&"flee".to_string()));
    }

    #[test]
    fn get_valid_commands_pre_combat_enemy_surprise() {
        let mut scene = make_scene();
        scene.pending_combat = Some(PendingEncounterState {
            creatures: vec![],
            awareness_result: "enemy_surprise".to_string(),
            terrain: "forest".to_string(),
            features: vec![],
            cell_q: 0,
            cell_r: 0,
        });
        let cmds = scene.get_valid_commands();
        assert!(cmds.contains(&"attack".to_string()));
        assert!(cmds.contains(&"flee".to_string()));
        assert!(!cmds.contains(&"avoid".to_string()));
    }

    #[test]
    fn check_transitions_none_by_default() {
        let scene = make_scene();
        let result = scene.check_transitions(&[]);
        assert_eq!(result, None);
    }

    #[test]
    fn check_transitions_combat() {
        let mut scene = make_scene();
        scene.pending_combat_transition = true;
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Combat));
    }

    #[test]
    fn check_transitions_social() {
        let mut scene = make_scene();
        scene.pending_social_transition = true;
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Social));
    }

    #[test]
    fn check_transitions_shopping() {
        let mut scene = make_scene();
        scene.pending_shopping_transition = true;
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Shopping));
    }

    #[test]
    fn check_transitions_dungeon() {
        let mut scene = make_scene();
        scene.pending_dungeon_transition = true;
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Dungeon));
    }

    #[test]
    fn check_transitions_town() {
        let mut scene = make_scene();
        scene.pending_town_transition = true;
        assert_eq!(scene.check_transitions(&[]), Some(SceneState::Town));
    }

    #[test]
    fn get_suggestions_settlement_terrain() {
        let mut scene = make_scene();
        scene.current_terrain = Some("settlement".to_string());
        let s = scene.get_suggestions();
        assert!(s.contains(&"enter".to_string()));
        assert!(s.contains(&"shop".to_string()));
    }

    #[test]
    fn get_suggestions_rest_when_injured() {
        let mut scene = make_scene();
        scene.current_char_hp = 5;
        scene.current_char_max_hp = 20;
        let s = scene.get_suggestions();
        assert!(s.contains(&"rest".to_string()));
        assert_eq!(s[0], "rest until healed");
    }

    // ---- exploration.actions notice tests (HR-112) -------------------------

    #[test]
    fn exploration_actions_settlement_lists_enter_talk_shop() {
        let mut scene = make_scene();
        scene.current_terrain = Some("settlement".to_string());
        let notice = scene.exploration_actions_notice();
        let cmds: Vec<&str> = notice.actions.iter().map(|b| b.command.as_str()).collect();
        assert!(cmds.contains(&"enter"), "expected enter in {cmds:?}");
        assert!(cmds.contains(&"talk"), "expected talk in {cmds:?}");
        assert!(cmds.contains(&"shop"), "expected shop in {cmds:?}");
        assert!(cmds.contains(&"search"), "expected search in {cmds:?}");
        assert!(cmds.contains(&"look"), "expected look in {cmds:?}");
        // Enter must be styled primary
        let enter = notice.actions.iter().find(|b| b.command == "enter").unwrap();
        assert_eq!(enter.style, "primary", "Enter must be styled primary");
    }

    #[test]
    fn exploration_actions_wilderness_is_look_search() {
        let scene = make_scene();
        let notice = scene.exploration_actions_notice();
        let cmds: Vec<&str> = notice.actions.iter().map(|b| b.command.as_str()).collect();
        assert_eq!(
            cmds,
            vec!["look", "search"],
            "wilderness must have only look + search, got {cmds:?}"
        );
    }

    #[test]
    fn exploration_actions_include_rest_when_injured() {
        let mut scene = make_scene();
        scene.current_char_hp = 1;
        scene.current_char_max_hp = 10;
        let notice = scene.exploration_actions_notice();
        let cmds: Vec<&str> = notice.actions.iter().map(|b| b.command.as_str()).collect();
        assert!(
            cmds.contains(&"rest"),
            "expected rest when injured, got {cmds:?}"
        );
    }

    #[test]
    fn handle_help_returns_narration() {
        let scene = make_scene();
        let db = db();
        let cmd = ParsedCommand { verb: "help".to_string(), args: vec![], raw: "help".to_string(), direction: None };
        let events = scene.handle_command_test(&cmd, &db);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn handle_unknown_command() {
        let mut scene = make_scene();
        let db = db();
        let cmd = ParsedCommand { verb: "xyzzy".to_string(), args: vec![], raw: "xyzzy".to_string(), direction: None };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert!(!events.is_empty());
        // Should be a narration about unknown command
        let data = &events[0].data;
        let text = data.get("text").and_then(|v| v.as_str()).unwrap_or("");
        assert!(text.contains("Unknown command"));
    }

    #[test]
    fn handle_move_no_direction() {
        let mut scene = make_scene();
        let db = db();
        let cmd = ParsedCommand { verb: "go".to_string(), args: vec![], raw: "go".to_string(), direction: None };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert!(!events.is_empty());
        let text = events[0].data.get("text").and_then(|v| v.as_str()).unwrap_or("");
        assert!(text.contains("direction") || text.contains("Direction"));
    }

    #[test]
    fn moving_into_impassable_terrain_reveals_it() {
        // #108: the player can't ENTER impassable terrain, but must be able to SEE it.
        // A blocked move emits `exploration.revealed` (explored=2) and persists the
        // reveal, so the obstacle appears on the map instead of staying in fog.
        use crate::cell::{TerrainRegistry, TerrainType};
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        let character = crate::character::Character::new("Scout", "expert");
        EntityRepository::new(&db).create_character(&character).unwrap();
        db.execute(
            "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
             VALUES (0, 0, 'testland', '[]', 1, '{}')",
            &[],
        )
        .unwrap();
        db.execute(
            "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
             VALUES (1, 0, 'mountains', '[]', 0, '{}')",
            &[],
        )
        .unwrap();

        let registry = TerrainRegistry::from_types(vec![TerrainType {
            id: "mountains".into(),
            name: "Mountains".into(),
            passable: false,
            blocks_vision: true,
            description_pool: String::new(),
        }]);
        let narrator = Narrator::new(TerrainRegistry::default(), std::path::Path::new(""));
        let mut scene = ExplorationScene::new(registry, narrator, 0);

        let cmd = ParsedCommand {
            verb: "go".to_string(),
            args: vec![],
            raw: "go east".to_string(),
            direction: Some("east".to_string()),
        };
        let events = scene.handle_move(&cmd, &db).unwrap();

        // Emits exploration.revealed for the mountains cell, marked "seen" (explored=2).
        let revealed = events
            .iter()
            .find(|e| e.event_type == "exploration.revealed")
            .expect("a blocked move must reveal the impassable cell");
        let cell = revealed.data.get("cell").and_then(|v| v.as_object()).unwrap();
        assert_eq!(cell.get("q").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(cell.get("r").and_then(|v| v.as_i64()), Some(0));
        assert_eq!(cell.get("terrain").and_then(|v| v.as_str()), Some("mountains"));
        assert_eq!(
            cell.get("explored").and_then(|v| v.as_i64()),
            Some(2),
            "revealed as 'seen' (2), not fully explored"
        );

        // The player did NOT move into the impassable cell.
        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let ch = loaded.unwrap();
        assert_eq!((ch.position_q, ch.position_r), (0, 0));

        // And the reveal is persisted (fog → seen) so it survives a reload.
        let after = CellRepository::new(&db)
            .fetch_cell(GridCoord { q: 1, r: 0 })
            .unwrap()
            .unwrap();
        assert!(after.explored, "the impassable cell must persist as revealed");
    }

    #[test]
    fn move_persists_player_position() {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        // Persist a character at the origin.
        let character = crate::character::Character::new("Scout", "expert");
        assert_eq!((character.position_q, character.position_r), (0, 0));
        EntityRepository::new(&db).create_character(&character).unwrap();

        // Seed the current cell and its east neighbour. "testland" is not in the
        // default terrain registry, so the passability check treats it as passable.
        for (q, r) in [(0_i64, 0_i64), (1, 0)] {
            let sql = format!(
                "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
                 VALUES ({q}, {r}, 'testland', '[]', 1, '{{}}')"
            );
            db.execute(sql.as_str(), &[]).unwrap();
        }

        // Move east.
        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "go".to_string(),
            args: vec![],
            raw: "go east".to_string(),
            direction: Some("east".to_string()),
        };
        let _ = scene.handle_move(&cmd, &db).unwrap();

        // Regression: exploration moves must PERSIST the new world position, not
        // just narrate it. Before the fix the character stayed at (0, 0), so the
        // move read to the player as a "look".
        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let loaded = loaded.expect("character present");
        assert_eq!(
            (loaded.position_q, loaded.position_r),
            (1, 0),
            "handle_move must persist the new world position"
        );
    }

    // ---- HR-408: multi-step travel / journey -------------------------------

    /// Seed an in-memory world with a horizontal line of passable `testland`
    /// cells (0,0)..=(len-1,0) and a character at the origin.
    fn travel_world(len: i64) -> (WorldDatabase, ExplorationScene) {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        let character = crate::character::Character::new("Scout", "expert");
        EntityRepository::new(&db).create_character(&character).unwrap();
        for q in 0..len {
            let sql = format!(
                "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
                 VALUES ({q}, 0, 'testland', '[]', 1, '{{}}')"
            );
            db.execute(sql.as_str(), &[]).unwrap();
        }
        (db, make_scene())
    }

    fn travel_cmd(q: i32, r: i32) -> ParsedCommand {
        ParsedCommand {
            verb: "travel".to_string(),
            args: vec![q.to_string(), r.to_string()],
            raw: format!("travel {q} {r}"),
            direction: None,
        }
    }

    fn ev_types(events: &[GameEvent]) -> Vec<&str> {
        events.iter().map(|e| e.event_type.as_str()).collect()
    }

    fn loaded_pos(db: &WorldDatabase) -> (i32, i32) {
        let (loaded, _) = EntityRepository::new(db).load_character_with_record().unwrap();
        let loaded = loaded.expect("character present");
        (loaded.position_q, loaded.position_r)
    }

    fn first_text(events: &[GameEvent]) -> String {
        events
            .iter()
            .find_map(|e| e.data.get("text").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn travel_walks_full_path_and_completes() {
        let (db, mut scene) = travel_world(4);
        let events = scene.handle_travel(&travel_cmd(3, 0), &db).unwrap();
        let types = ev_types(&events);
        assert!(types.contains(&"travel.started"), "got {types:?}");
        assert_eq!(
            events.iter().filter(|e| e.event_type == "travel.step").count(),
            3,
            "one travel.step per cell (0->1->2->3)"
        );
        assert!(types.contains(&"travel.completed"), "got {types:?}");
        assert!(!types.contains(&"travel.interrupted"));
        assert_eq!(loaded_pos(&db), (3, 0), "character must arrive at the destination");
        assert!(scene.load_travel_goal(&db).is_none(), "goal cleared on arrival");
    }

    #[test]
    fn travel_interrupted_by_encounter_marks_goal_and_stops() {
        let (db, mut scene) = travel_world(4);
        // Stage a pending encounter; the empty test world spawns none of its own,
        // so this survives the first step and trips the interruption check.
        scene.pending_combat = Some(crate::gm_runtime::PendingEncounterState {
            creatures: vec![],
            awareness_result: "mutual_awareness".to_string(),
            terrain: "testland".to_string(),
            features: vec![],
            cell_q: 1,
            cell_r: 0,
        });
        let events = scene.handle_travel(&travel_cmd(3, 0), &db).unwrap();
        let types = ev_types(&events);
        assert!(types.contains(&"travel.interrupted"), "got {types:?}");
        assert!(!types.contains(&"travel.completed"));
        assert_eq!(
            events.iter().filter(|e| e.event_type == "travel.step").count(),
            1,
            "journey stops after the interrupting step"
        );
        assert_eq!(loaded_pos(&db), (1, 0), "character stops where interrupted");
        let goal = scene.load_travel_goal(&db).expect("interrupted goal persists for resume");
        assert_eq!(goal.status, TravelStatus::Interrupted);
        assert_eq!((goal.target_q, goal.target_r), (3, 0));
    }

    #[test]
    fn resume_continues_interrupted_journey_to_completion() {
        let (db, mut scene) = travel_world(4);
        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let mut ch = loaded.unwrap();
        ch.position_q = 1;
        ch.position_r = 0;
        EntityRepository::new(&db).save_character(&ch, true, false, false).unwrap();
        scene.save_travel_goal(
            &db,
            &TravelGoal {
                target_q: 3,
                target_r: 0,
                action: TravelAction::Move,
                skill: None,
                status: TravelStatus::Interrupted,
            },
        );
        let events = scene.handle_resume_travel(&db).unwrap();
        let types = ev_types(&events);
        assert!(types.contains(&"travel.resumed"), "got {types:?}");
        assert!(types.contains(&"travel.completed"), "got {types:?}");
        assert_eq!(loaded_pos(&db), (3, 0));
        assert!(scene.load_travel_goal(&db).is_none());
    }

    #[test]
    fn travel_to_current_cell_is_noop() {
        let (db, mut scene) = travel_world(4);
        let events = scene.handle_travel(&travel_cmd(0, 0), &db).unwrap();
        assert!(!ev_types(&events).contains(&"travel.started"));
        assert!(first_text(&events).contains("already there"));
    }

    #[test]
    fn travel_with_no_route_narrates() {
        let (db, mut scene) = travel_world(4);
        // (9,9) has no cell -> not passable -> no path.
        let events = scene.handle_travel(&travel_cmd(9, 9), &db).unwrap();
        assert!(!ev_types(&events).contains(&"travel.started"));
        assert!(first_text(&events).contains("No passable route"), "got: {}", first_text(&events));
    }

    #[test]
    fn cancel_travel_emits_event_and_clears_goal() {
        let (db, scene) = travel_world(4);
        scene.save_travel_goal(
            &db,
            &TravelGoal {
                target_q: 3,
                target_r: 0,
                action: TravelAction::Move,
                skill: None,
                status: TravelStatus::Interrupted,
            },
        );
        let events = scene.handle_cancel_travel(&db);
        assert!(ev_types(&events).contains(&"travel.cancelled"));
        assert!(scene.load_travel_goal(&db).is_none());
    }

    #[test]
    fn rest_command_routes_through_parser_and_heals() {
        // HR-798 end-to-end: a `rest` command parsed by the real CommandParser must
        // reach handle_rest (not handle_wait) and restore HP. Guards the parser-alias
        // regression where `rest` was mapped to `wait` and healed nothing.
        let (db, mut scene) = travel_world(1);
        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let mut ch = loaded.unwrap();
        ch.hp = 1;
        ch.max_hp = 5;
        EntityRepository::new(&db).save_character(&ch, false, true, false).unwrap();

        let cmd = crate::parser::CommandParser.parse("rest");
        assert_eq!(cmd.verb, "rest", "parser must route `rest` to its own verb");
        let events = scene.handle_command(&cmd, &db).unwrap();

        let (after, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        assert!(
            after.unwrap().hp > 1,
            "the parsed `rest` command must restore HP"
        );
        let text: String = events
            .iter()
            .filter_map(|e| e.data.get("text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!text.contains("Time passes"), "`rest` must not fall through to `wait`");
    }

    #[test]
    fn encounter_on_destination_cell_completes_not_interrupts() {
        // HR-799: an encounter on the FINAL cell must complete the journey (which
        // clears the resume banner), not interrupt it — otherwise the player ends up
        // standing on the target with a "resume to (target)" banner that never clears.
        let (db, mut scene) = travel_world(2); // cells (0,0),(1,0)
        scene.pending_combat = Some(crate::gm_runtime::PendingEncounterState {
            creatures: vec![],
            awareness_result: "mutual_awareness".to_string(),
            terrain: "testland".to_string(),
            features: vec![],
            cell_q: 1,
            cell_r: 0,
        });
        // A 1-step journey whose destination (1,0) is the cell with the encounter.
        let events = scene.handle_travel(&travel_cmd(1, 0), &db).unwrap();
        let types = ev_types(&events);
        assert!(types.contains(&"travel.completed"), "arrival completes even with an encounter on the cell; got {types:?}");
        assert!(!types.contains(&"travel.interrupted"));
        assert!(scene.load_travel_goal(&db).is_none(), "goal cleared on arrival");
        assert_eq!(loaded_pos(&db), (1, 0));
    }

    #[test]
    fn resume_at_destination_emits_completed_to_clear_banner() {
        // HR-799: resuming while already on the target must emit a banner-clearing
        // event (travel.completed), not just clear the goal server-side — else the
        // frontend resume banner lingers on a target the player is standing on.
        let (db, mut scene) = travel_world(2);
        scene.save_travel_goal(
            &db,
            &TravelGoal {
                target_q: 0,
                target_r: 0,
                action: TravelAction::Move,
                skill: None,
                status: TravelStatus::Interrupted,
            },
        );
        let events = scene.handle_resume_travel(&db).unwrap();
        assert!(
            ev_types(&events).contains(&"travel.completed"),
            "must emit a banner-clearing event; got {:?}",
            ev_types(&events)
        );
        assert!(scene.load_travel_goal(&db).is_none());
    }

    #[test]
    fn resume_without_interrupted_goal_says_none() {
        let (db, mut scene) = travel_world(4);
        let events = scene.handle_resume_travel(&db).unwrap();
        assert!(first_text(&events).contains("No interrupted journey"), "got: {}", first_text(&events));
    }

    #[test]
    fn staged_encounter_takes_precedence_over_resume_yes() {
        // HR-408 review (MED): after a travel interrupt, both pending_combat and an
        // Interrupted goal are set. A raw "yes" must route to combat, not resume the
        // journey — otherwise the player walks past the encounter one cell at a time.
        let (db, mut scene) = travel_world(4);
        scene.pending_combat = Some(crate::gm_runtime::PendingEncounterState {
            creatures: vec![],
            awareness_result: "mutual_awareness".to_string(),
            terrain: "testland".to_string(),
            features: vec![],
            cell_q: 1,
            cell_r: 0,
        });
        scene.save_travel_goal(
            &db,
            &TravelGoal {
                target_q: 3,
                target_r: 0,
                action: TravelAction::Move,
                skill: None,
                status: TravelStatus::Interrupted,
            },
        );
        let yes = ParsedCommand {
            verb: "yes".to_string(),
            args: vec![],
            raw: "yes".to_string(),
            direction: None,
        };
        let events = scene.handle_command(&yes, &db).unwrap();
        let types = ev_types(&events);
        assert!(!types.contains(&"travel.resumed"), "yes must not resume travel while combat is staged");
        assert!(!types.contains(&"travel.step"), "character must not move while combat is staged");
        assert_eq!(loaded_pos(&db), (0, 0), "character stays put until the encounter is resolved");
        assert_eq!(
            scene.load_travel_goal(&db).map(|g| g.status),
            Some(TravelStatus::Interrupted),
            "goal stays Interrupted for post-combat resume"
        );
    }

    #[test]
    fn enter_hex_event_carries_self_id_and_terrain() {
        let ev = enter_hex_event(
            3,
            "pc",
            "ruins",
            &["lair".to_string()],
            GridCoord { q: 2, r: -1 },
            true,
        );
        assert_eq!(ev.event_type, "exploration.enter_hex");
        assert_eq!(ev.source, "exploration");
        // self_id drives IR trigger sourcing; terrain/features gate `when`.
        assert_eq!(ev.data.get("self_id").and_then(|v| v.as_str()), Some("pc"));
        assert_eq!(ev.data.get("terrain").and_then(|v| v.as_str()), Some("ruins"));
        assert_eq!(ev.data.get("q").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(ev.data.get("r").and_then(|v| v.as_i64()), Some(-1));
        let features = ev.data.get("features").and_then(|v| v.as_array()).unwrap();
        assert_eq!(features[0].as_str(), Some("lair"));
    }

    #[test]
    fn handle_wait() {
        let scene = make_scene();
        let events = scene.handle_wait();
        assert_eq!(events.len(), 1);
        let text = events[0].data.get("text").and_then(|v| v.as_str()).unwrap_or("");
        assert!(text.contains("Time passes"));
    }

    #[test]
    fn pre_combat_flee() {
        let mut scene = make_scene();
        let db = db();
        scene.pending_combat = Some(PendingEncounterState {
            creatures: vec![],
            awareness_result: "mutual_awareness".to_string(),
            terrain: "plains".to_string(),
            features: vec![],
            cell_q: 0,
            cell_r: 0,
        });
        let cmd = ParsedCommand { verb: "flee".to_string(), args: vec![], raw: "flee".to_string(), direction: None };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert!(scene.pending_combat.is_none());
        assert!(!events.is_empty());
    }

    #[test]
    fn pre_combat_avoid() {
        let mut scene = make_scene();
        let db = db();
        scene.pending_combat = Some(PendingEncounterState {
            creatures: vec![],
            awareness_result: "player_surprise".to_string(),
            terrain: "plains".to_string(),
            features: vec![],
            cell_q: 0,
            cell_r: 0,
        });
        let cmd = ParsedCommand { verb: "avoid".to_string(), args: vec![], raw: "avoid".to_string(), direction: None };
        let events = scene.handle_command(&cmd, &db).unwrap();
        assert!(scene.pending_combat.is_none());
        assert!(!events.is_empty());
    }

    // ---------------------------------------------------------------------------
    // #113: pre-combat action bar notice tests
    // ---------------------------------------------------------------------------

    #[test]
    fn pre_combat_actions_mutual_awareness_lists_attack_flee_talk() {
        // With can_talk=true, mutual_awareness exposes attack + flee + talk.
        let notice = pre_combat_actions_notice("mutual_awareness", true);
        assert_eq!(notice.phase, "pre_combat");
        let cmds: Vec<&str> = notice.actions.iter().map(|a| a.command.as_str()).collect();
        assert!(cmds.contains(&"attack"), "attack must be present");
        assert!(cmds.contains(&"flee"),   "flee must be present");
        assert!(cmds.contains(&"talk"),   "talk must be present for humanoid encounters");
        assert!(!cmds.contains(&"avoid"), "avoid must not appear in mutual_awareness");
        assert!(!cmds.contains(&"sneak"), "sneak must not appear in mutual_awareness");

        // With can_talk=false, talk button must be suppressed.
        let no_talk = pre_combat_actions_notice("mutual_awareness", false);
        let no_cmds: Vec<&str> = no_talk.actions.iter().map(|a| a.command.as_str()).collect();
        assert!(no_cmds.contains(&"attack"));
        assert!(no_cmds.contains(&"flee"));
        assert!(!no_cmds.contains(&"talk"), "talk suppressed when can_talk=false");
    }

    #[test]
    fn pre_combat_actions_player_surprise_lists_avoid_sneak() {
        // Player surprise: attack, avoid, sneak (no flee, no talk regardless of can_talk).
        let notice = pre_combat_actions_notice("player_surprise", false);
        assert_eq!(notice.phase, "pre_combat");
        let cmds: Vec<&str> = notice.actions.iter().map(|a| a.command.as_str()).collect();
        assert!(cmds.contains(&"attack"), "attack must be present");
        assert!(cmds.contains(&"avoid"),  "avoid must be present in player_surprise");
        assert!(cmds.contains(&"sneak"),  "sneak must be present in player_surprise");
        assert!(!cmds.contains(&"flee"),  "flee must not appear in player_surprise");
    }

    #[test]
    fn pre_combat_flee_emits_cleared_actions() {
        // Fleeing from a staged encounter should produce a combat.actions event
        // with phase "cleared" so the frontend hides the pre-combat menu.
        let mut scene = make_scene();
        let db = db();
        scene.pending_combat = Some(PendingEncounterState {
            creatures: vec![],
            awareness_result: "mutual_awareness".to_string(),
            terrain: "plains".to_string(),
            features: vec![],
            cell_q: 0,
            cell_r: 0,
        });
        let flee = ParsedCommand {
            verb: "flee".to_string(),
            args: vec![],
            raw: "flee".to_string(),
            direction: None,
        };
        let events = scene.handle_pre_combat_command(&flee, &db).unwrap();
        let cleared = events
            .iter()
            .find(|e| e.event_type == "combat.actions")
            .expect("handle_pre_combat_command(flee) must emit a combat.actions event");
        let phase = cleared.data.get("phase").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(phase, "cleared", "the emitted combat.actions must carry phase=\"cleared\"");
        let actions = cleared.data.get("actions").and_then(|v| v.as_array()).unwrap();
        assert!(actions.is_empty(), "cleared actions list must be empty");
    }

    // ---------------------------------------------------------------------------
    // HR-771 regression: `rest` must restore HP and persist to DB
    // ---------------------------------------------------------------------------

    fn setup_rest_db() -> WorldDatabase {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    #[test]
    fn rest_restores_hp_and_persists() {
        // Regression for HR-771: `rest` must actually heal the character and
        // write the new HP to the DB.  Before the fix, handle_rest only emitted
        // an event and narrated "You rest for a while." — HP never changed.
        let db = setup_rest_db();
        let mut character = crate::character::Character::new("RestHero", "warrior");
        character.max_hp = 20;
        character.hp = 5; // deliberately injured
        // Pin the healing math so the expected value is unambiguous: a full rest
        // restores `(level + CON-mod).max(1)`. Level 1, CON-mod 0 → exactly 1 HP.
        character.level = 1;
        character.attr_mods.insert("con".to_string(), 0);
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "rest".to_string(),
            args: vec![],
            raw: "rest".to_string(),
            direction: None,
        };
        let events = scene.handle_rest(&cmd, &db).unwrap();

        // The events must include a `character.hp_changed` notification so the
        // frontend HP bar updates.
        let has_hp_event = events
            .iter()
            .any(|ev| ev.event_type == "character.hp_changed");
        assert!(has_hp_event, "handle_rest must emit character.hp_changed");

        // HR-793: the vestigial, filtered `exploration.rest_requested` event must
        // be gone — rest's structured result reaches the client via hp_changed.
        assert!(
            !events
                .iter()
                .any(|ev| ev.event_type == "exploration.rest_requested"),
            "handle_rest must not emit the filtered _requested event"
        );

        // Most importantly: the DB must reflect the EXACT healed HP (5 + 1 = 6).
        let (reloaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let reloaded = reloaded.expect("character must exist after rest");
        assert_eq!(
            reloaded.hp, 6,
            "full rest restores (level + CON-mod).max(1) = 1 HP; 5 → 6 (got {})",
            reloaded.hp
        );
    }

    #[test]
    fn rest_at_full_hp_is_a_noop() {
        // A plain `rest` at full HP heals nothing: no HP change, and — critically —
        // no `character.hp_changed` event (pins the review's gating fix).
        let db = setup_rest_db();
        let mut character = crate::character::Character::new("FullHero", "warrior");
        character.max_hp = 20;
        character.hp = 20; // already at full health
        character.level = 1;
        character.attr_mods.insert("con".to_string(), 0);
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "rest".to_string(),
            args: vec![],
            raw: "rest".to_string(),
            direction: None,
        };
        let events = scene.handle_rest(&cmd, &db).unwrap();

        // No hp_changed event when nothing healed.
        let has_hp_event = events
            .iter()
            .any(|ev| ev.event_type == "character.hp_changed");
        assert!(
            !has_hp_event,
            "rest at full HP must NOT emit character.hp_changed"
        );

        // HP stays put in the DB.
        let (reloaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let reloaded = reloaded.expect("character must exist after no-op rest");
        assert_eq!(reloaded.hp, 20, "rest at full HP must not change HP");
    }

    #[test]
    fn rest_until_healed_restores_to_full_hp() {
        // `rest until healed` must restore HP to max_hp and persist it.
        let db = setup_rest_db();
        let mut character = crate::character::Character::new("HealHero", "warrior");
        character.max_hp = 20;
        character.hp = 3; // badly injured
        character.level = 1;
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "rest".to_string(),
            args: vec!["until".to_string(), "healed".to_string()],
            raw: "rest until healed".to_string(),
            direction: None,
        };
        let _events = scene.handle_rest(&cmd, &db).unwrap();

        // The DB must reflect full HP.
        let (reloaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let reloaded = reloaded.expect("character must exist after rest until healed");
        assert_eq!(
            reloaded.hp,
            reloaded.max_hp,
            "rest until healed must restore HP to max_hp (expected {}, got {})",
            reloaded.max_hp,
            reloaded.hp
        );
    }

    // ---------------------------------------------------------------------------
    // HR-772 regression: `take` must move the item into inventory AND clear the
    // cell's death marker in the DB.  Before the fix, handle_take only emitted an
    // `exploration.take_requested` event that nothing subscribed to, so the item
    // was never added and the marker was never removed.
    // ---------------------------------------------------------------------------

    #[test]
    fn take_persists_item_to_inventory_and_clears_marker() {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }

        // A character standing on the origin, starting with an empty pack.
        let character = crate::character::Character::new("Looter", "expert");
        assert_eq!((character.position_q, character.position_r), (0, 0));
        assert!(character.equipment.is_empty());
        EntityRepository::new(&db).create_character(&character).unwrap();

        // Seed the current cell, then attach a death marker holding one item via
        // the repository (mirrors how loot on a corpse is stored).
        db.execute(
            "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
             VALUES (0, 0, 'testland', '[]', 1, '{}')",
            &[],
        )
        .unwrap();
        let marker: JsonObject = serde_json::from_value(serde_json::json!({
            "items": [
                { "name": "Iron Sword", "type": "weapon", "value": 25, "weapon_damage": "1d8" }
            ]
        }))
        .unwrap();
        CellRepository::new(&db)
            .save_cell_data(0, 0, &{
                let mut o = JsonObject::new();
                o.insert("death_markers".into(), JsonValue::Array(vec![JsonValue::Object(marker)]));
                o
            })
            .unwrap();

        // Take the seeded item.
        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "take".to_string(),
            args: vec!["iron".to_string(), "sword".to_string()],
            raw: "take iron sword".to_string(),
            direction: None,
        };
        let events = scene.handle_take(&cmd, &db).unwrap();

        // HR-793: the structured result must reach the client as a client-facing
        // `inventory.item_given` event (not the filtered `exploration.take_requested`).
        assert!(
            !events
                .iter()
                .any(|ev| ev.event_type == "exploration.take_requested"),
            "take must not emit the filtered _requested event"
        );
        let given: Vec<&GameEvent> = events
            .iter()
            .filter(|ev| ev.event_type == "inventory.item_given")
            .collect();
        assert_eq!(given.len(), 1, "take must emit one inventory.item_given");
        assert_eq!(
            given[0].data.get("item").and_then(|i| i.get("name")).and_then(|v| v.as_str()),
            Some("Iron Sword"),
            "the item_given event must carry the taken item's data"
        );

        // Regression: the item must now live in the character's persisted inventory.
        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let loaded = loaded.expect("character present");
        assert_eq!(
            loaded.equipment.len(),
            1,
            "take must persist the item into the character's inventory"
        );
        assert_eq!(
            loaded.equipment[0].get("name").and_then(|v| v.as_str()),
            Some("Iron Sword"),
            "the taken item's identity must round-trip into inventory"
        );

        // Regression: the death marker must be cleared from the cell in the DB.
        let cell = CellRepository::new(&db)
            .fetch_cell(GridCoord { q: 0, r: 0 })
            .unwrap()
            .expect("cell present");
        let remaining = cell
            .data
            .get("death_markers")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        assert_eq!(
            remaining, 0,
            "take must remove the emptied death marker from the cell (found {remaining})"
        );
    }

    #[test]
    fn take_from_multi_item_marker_leaves_the_rest() {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }

        let character = crate::character::Character::new("Looter", "expert");
        EntityRepository::new(&db).create_character(&character).unwrap();
        db.execute(
            "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
             VALUES (0, 0, 'testland', '[]', 1, '{}')",
            &[],
        )
        .unwrap();
        // One marker holding two distinct items — taking one must leave the other.
        let marker: JsonObject = serde_json::from_value(serde_json::json!({
            "items": [
                { "name": "Iron Sword", "type": "weapon", "value": 25 },
                { "name": "Leather Pouch", "type": "misc", "value": 3 }
            ]
        }))
        .unwrap();
        CellRepository::new(&db)
            .save_cell_data(0, 0, &{
                let mut o = JsonObject::new();
                o.insert("death_markers".into(), JsonValue::Array(vec![JsonValue::Object(marker)]));
                o
            })
            .unwrap();

        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "take".to_string(),
            args: vec!["iron".to_string(), "sword".to_string()],
            raw: "take iron sword".to_string(),
            direction: None,
        };
        let _ = scene.handle_take(&cmd, &db).unwrap();

        // Exactly the requested item entered inventory.
        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        let loaded = loaded.expect("character present");
        assert_eq!(loaded.equipment.len(), 1);
        assert_eq!(
            loaded.equipment[0].get("name").and_then(|v| v.as_str()),
            Some("Iron Sword")
        );

        // The other item survives in the still-present marker.
        let cell = CellRepository::new(&db)
            .fetch_cell(GridCoord { q: 0, r: 0 })
            .unwrap()
            .expect("cell present");
        let markers = cell
            .data
            .get("death_markers")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert_eq!(markers.len(), 1, "the marker must remain (it still has an item)");
        let remaining_items = markers[0]
            .get("items")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        assert_eq!(remaining_items, 1, "only the taken item is removed");
        assert_eq!(
            markers[0]["items"][0].get("name").and_then(|v| v.as_str()),
            Some("Leather Pouch"),
            "the untaken item must be the one that remains"
        );
    }

    // ---------------------------------------------------------------------------
    // HR-793 regression: search/take surface discovered items to the client via
    // the client-facing `inventory.item_given` notice, not the filtered
    // `exploration.{search,take}_requested` events.
    // ---------------------------------------------------------------------------

    #[test]
    fn item_given_events_are_client_facing_per_item() {
        let scene = make_scene();
        let items: Vec<JsonObject> = vec![
            serde_json::from_value(serde_json::json!({ "name": "Data Chip", "type": "relic" }))
                .unwrap(),
            serde_json::from_value(serde_json::json!({ "name": "Old Coin", "type": "currency", "value": 5 }))
                .unwrap(),
        ];

        let events = scene.item_given_events("char-1", &items);

        assert_eq!(events.len(), 2, "one event per granted item");
        for ev in &events {
            // Not a `_requested` event → survives `resolve_domain_events`.
            assert_eq!(ev.event_type, "inventory.item_given");
            assert!(!ev.event_type.ends_with("_requested"));
            assert_eq!(ev.source, "exploration");
            assert_eq!(
                ev.data.get("character_id").and_then(|v| v.as_str()),
                Some("char-1")
            );
        }
        assert_eq!(
            events[0].data.get("item").and_then(|i| i.get("name")).and_then(|v| v.as_str()),
            Some("Data Chip")
        );
        assert_eq!(
            events[1].data.get("item").and_then(|i| i.get("name")).and_then(|v| v.as_str()),
            Some("Old Coin")
        );

        // No items → no events (a fruitless search narrates only).
        assert!(scene.item_given_events("char-1", &[]).is_empty());
    }

    // ---------------------------------------------------------------------------
    // HR-775 regression: discovered items from `search` must enter inventory
    // ---------------------------------------------------------------------------

    // ---------------------------------------------------------------------------
    // HR-786 regression: searchable loot sources (reveal → take)
    // ---------------------------------------------------------------------------

    fn loot_db() -> WorldDatabase {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db.execute(
            "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
             VALUES (0, 0, 'plains', '[]', 1, '{}')",
            &[],
        )
        .unwrap();
        db
    }

    fn loot_item(name: &str, value: i64) -> JsonObject {
        serde_json::from_value(serde_json::json!({
            "name": name, "type": "misc", "value": value
        }))
        .unwrap()
    }

    fn seed_source(db: &WorldDatabase, source: loot_source::LootSource) {
        let mut data = JsonObject::new();
        loot_source::push_source(&mut data, source);
        CellRepository::new(db).save_cell_data(0, 0, &data).unwrap();
    }

    #[test]
    fn take_from_revealed_loot_source_moves_item_and_empties_source() {
        let db = loot_db();
        let character = crate::character::Character::new("Looter", "expert");
        EntityRepository::new(&db).create_character(&character).unwrap();
        seed_source(
            &db,
            loot_source::LootSource {
                id: "crate_1".into(),
                kind: loot_source::KIND_CONTAINER.into(),
                name: "crate".into(),
                contents: vec![loot_item("Iron Sword", 25)],
                gold: 0,
                difficulty: 8,
                searched: true,
                empty: false,
            },
        );

        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "take".to_string(),
            args: vec!["iron".to_string(), "sword".to_string()],
            raw: "take iron sword".to_string(),
            direction: None,
        };
        let events = scene.handle_take(&cmd, &db).unwrap();

        // Item entered inventory + emitted the client-facing notice.
        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        assert_eq!(loaded.unwrap().equipment.len(), 1);
        assert!(events.iter().any(|e| e.event_type == "inventory.item_given"));
        // HR-786: the loot panel is refreshed — the now-exhausted source is
        // re-emitted with empty contents so the panel drops it.
        let revealed = events
            .iter()
            .find(|e| e.event_type == "loot.source_revealed")
            .expect("take from a source emits loot.source_revealed");
        assert_eq!(
            revealed.data.get("items").and_then(|v| v.as_array()).map(|a| a.len()),
            Some(0),
            "the exhausted source carries no remaining items"
        );
        // The emptied source is dropped from the cell.
        let cell = CellRepository::new(&db)
            .fetch_cell(GridCoord { q: 0, r: 0 })
            .unwrap()
            .unwrap();
        assert!(
            loot_source::read_sources(&cell.data).is_empty(),
            "an emptied source is removed"
        );
    }

    #[test]
    fn take_ignores_an_unsearched_loot_source() {
        let db = loot_db();
        let character = crate::character::Character::new("Looter", "expert");
        EntityRepository::new(&db).create_character(&character).unwrap();
        seed_source(
            &db,
            loot_source::LootSource {
                id: "chest_1".into(),
                kind: loot_source::KIND_CONTAINER.into(),
                name: "locked chest".into(),
                contents: vec![loot_item("Ruby", 300)],
                gold: 0,
                difficulty: 10,
                searched: false, // NOT yet revealed
                empty: false,
            },
        );

        let mut scene = make_scene();
        let cmd = ParsedCommand {
            verb: "take".to_string(),
            args: vec!["ruby".to_string()],
            raw: "take ruby".to_string(),
            direction: None,
        };
        let _ = scene.handle_take(&cmd, &db).unwrap();

        let (loaded, _) = EntityRepository::new(&db).load_character_with_record().unwrap();
        assert!(
            loaded.unwrap().equipment.is_empty(),
            "an unsearched source's items are not takeable until revealed"
        );
    }

    #[test]
    fn reveal_reveals_a_trivial_source_and_persists() {
        let db = loot_db();
        let character = crate::character::Character::new("Seeker", "expert");
        EntityRepository::new(&db).create_character(&character).unwrap();
        seed_source(
            &db,
            loot_source::LootSource {
                id: "crate_1".into(),
                kind: loot_source::KIND_CONTAINER.into(),
                name: "open crate".into(),
                contents: vec![loot_item("Rope", 5)],
                gold: 3,
                difficulty: 0, // trivially searchable → skill check always succeeds
                searched: false,
                empty: false,
            },
        );

        let scene = make_scene();
        let events = scene
            .reveal_loot_sources(&db, GridCoord { q: 0, r: 0 }, &character, "plains")
            .unwrap();
        assert!(!events.is_empty(), "revealing a source narrates its contents");
        // HR-786: reveal emits a client-facing loot.source_revealed for the panel.
        let revealed = events
            .iter()
            .find(|e| e.event_type == "loot.source_revealed")
            .expect("reveal emits loot.source_revealed");
        assert_eq!(
            revealed.data.get("id").and_then(|v| v.as_str()),
            Some("crate_1")
        );
        assert_eq!(
            revealed.data.get("items").and_then(|v| v.as_array()).map(|a| a.len()),
            Some(1),
            "the revealed source carries its contents"
        );

        let cell = CellRepository::new(&db)
            .fetch_cell(GridCoord { q: 0, r: 0 })
            .unwrap()
            .unwrap();
        let sources = loot_source::read_sources(&cell.data);
        assert!(
            sources[0].searched,
            "a difficulty-0 source is always revealed and the state persists"
        );
    }

    fn setup_search_db() -> WorldDatabase {
        use crate::db_schema::SCHEMA_SQL;
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    #[test]
    fn grant_items_persists_discovered_items_to_inventory() {
        // Primary regression for HR-775.  Before the fix, `handle_search` only
        // packed items into the event payload; `grant_items` (and the call-site
        // inside `handle_search`) did not exist, so this test would have failed
        // to compile.  Running it without the persistence call in `handle_search`
        // would yield equipment.len() == 0 instead of 1.
        let db = setup_search_db();

        let mut character = crate::character::Character::new("Searcher", "expert");
        assert!(character.equipment.is_empty());
        EntityRepository::new(&db).create_character(&character).unwrap();

        let scene = make_scene();
        let item: JsonObject = serde_json::from_value(serde_json::json!({
            "name": "Data Chip",
            "type": "relic",
            "value": 150,
            "category": "relic"
        }))
        .unwrap();

        // Call the extracted helper directly — deterministic, no RNG involved.
        scene
            .grant_items(&db, &mut character, &[item])
            .expect("grant_items must not fail");

        // Reload from DB and verify the item survived the round-trip.
        let (reloaded, _) = EntityRepository::new(&db)
            .load_character_with_record()
            .unwrap();
        let reloaded = reloaded.expect("character must still exist");
        assert_eq!(
            reloaded.equipment.len(),
            1,
            "grant_items must persist exactly one item to inventory (got {})",
            reloaded.equipment.len()
        );
        assert_eq!(
            reloaded.equipment[0].get("name").and_then(|v| v.as_str()),
            Some("Data Chip"),
            "the item identity must round-trip through the DB"
        );
    }

    #[test]
    fn grant_items_with_empty_slice_leaves_inventory_unchanged() {
        // Searching a hex with zero discovered items (environmental find, cooldown,
        // or nothing at all) must NOT modify the character's inventory — no spurious
        // save, no data corruption.
        let db = setup_search_db();

        let mut character = crate::character::Character::new("SearcherB", "warrior");
        let sentinel: JsonObject = serde_json::from_value(serde_json::json!({
            "name": "Preexisting Item",
            "type": "misc",
            "value": 5
        }))
        .unwrap();
        character.equipment.push(sentinel);
        EntityRepository::new(&db).create_character(&character).unwrap();

        let scene = make_scene();
        scene
            .grant_items(&db, &mut character, &[])
            .expect("grant_items with empty slice must not fail");

        let (reloaded, _) = EntityRepository::new(&db)
            .load_character_with_record()
            .unwrap();
        let reloaded = reloaded.expect("character present");
        assert_eq!(
            reloaded.equipment.len(),
            1,
            "empty grant must not alter existing inventory (got {} items)",
            reloaded.equipment.len()
        );
        assert_eq!(
            reloaded.equipment[0].get("name").and_then(|v| v.as_str()),
            Some("Preexisting Item"),
            "the pre-existing item must be unchanged"
        );
    }

    #[test]
    fn grant_items_appends_multiple_items_to_inventory() {
        // Verify that multiple simultaneously discovered items are all granted.
        let db = setup_search_db();

        let mut character = crate::character::Character::new("SearcherC", "expert");
        EntityRepository::new(&db).create_character(&character).unwrap();

        let scene = make_scene();
        let items: Vec<JsonObject> = vec![
            serde_json::from_value(serde_json::json!({"name": "Old Coin", "type": "misc", "value": 2})).unwrap(),
            serde_json::from_value(serde_json::json!({"name": "Pretech Shard", "type": "relic", "value": 300})).unwrap(),
        ];

        scene
            .grant_items(&db, &mut character, &items)
            .expect("grant_items must not fail");

        let (reloaded, _) = EntityRepository::new(&db)
            .load_character_with_record()
            .unwrap();
        let reloaded = reloaded.expect("character present");
        assert_eq!(
            reloaded.equipment.len(),
            2,
            "both items must land in inventory (got {})",
            reloaded.equipment.len()
        );
        let names: Vec<&str> = reloaded
            .equipment
            .iter()
            .filter_map(|e| e.get("name").and_then(|v| v.as_str()))
            .collect();
        assert!(names.contains(&"Old Coin"), "Old Coin must be in inventory");
        assert!(names.contains(&"Pretech Shard"), "Pretech Shard must be in inventory");
    }
}

// Test-only helper to call handle_command without mutability in test assertions
impl ExplorationScene {
    #[cfg(test)]
    fn handle_command_test(&self, cmd: &ParsedCommand, _db: &WorldDatabase) -> Vec<GameEvent> {
        self.handle_help()
    }
}
