//! GM Controller — central state machine for Harsh Realm.
//!
//! Ported from the Python `harsh_realm.gm.controller` package:
//!  - `gm/__init__.py`
//!  - `gm/controller_support.py`
//!  - `gm/runtime_protocols.py`
//!  - `gm/gm_factory.py`
//!  - `gm/admin_handler.py`
//!  - `gm/_controller_commands_mixin.py`
//!  - `gm/_controller_events_mixin.py`
//!  - `gm/_controller_flow_mixin.py`
//!  - `gm/_controller_wiring_mixin.py`
//!  - `gm/controller.py`

use rand::thread_rng;
use serde_json::Value as JsonValue;

use crate::cell::TerrainRegistry;
use crate::combat::creation::create_combat;
use crate::combat_runtime::AwarenessResult;
use crate::command::ParsedCommand;
use crate::creature::CreatureData;
use crate::db::WorldDatabase;
use crate::domain_events::DomainEventDispatcher;
use crate::events::{EventBus, GameEvent};
use crate::gm::event_handlers::register_gm_event_handlers;
use crate::gm::scenes::character_creation_scene::CharacterCreationScene;
use crate::gm::scenes::combat_scene::CombatScene;
use crate::gm::scenes::dungeon::DungeonScene;
use crate::gm::scenes::exploration::ExplorationScene;
use crate::gm::scenes::level_up::LevelUpScene;
use crate::gm::scenes::respawn::RespawnScene;
use crate::gm::scenes::shopping::ShoppingScene;
use crate::gm::scenes::social::SocialScene;
use crate::gm::scenes::town::TownScene;
use crate::scene_data::{DungeonConnection, DungeonRoom, TownCell};
use crate::settlement::SettlementSummary;
use crate::gm::scenes::{SceneHandler, SceneState};
use crate::gm::Narrator;
use crate::oracle::{ChaosTracker, RandomEventGenerator, SceneChecker};
use crate::oracle_models::SceneModification;
use crate::parser::CommandParser;
use crate::payloads::contexts::{
    DungeonSceneContext, ShoppingSceneContext, SocialSceneContext, TownSceneContext,
};
use crate::content::IRRecordRepository;
use crate::repositories::difficulty::DifficultySettingsRepository;
use crate::repositories::entity::EntityRepository;
use crate::repositories::gm_state::GMStateRepository;
use crate::runtime::{JsonObject, SceneNpcRecord};
use crate::character::Character;
use crate::runtime_content::{EntityResolver, EntitySnapshot, RuntimeContentStore, TriggerRuntime};
use crate::status_effects::handlers::{handle_apply_requested, handle_remove_requested};
use crate::status_effects::repository::StatusEffectRepository;
use crate::status_effects::service::{StatusEffectService, WorldClock};

/// Fixed world-clock view for the controller's IR trigger runtime.
struct ControllerClock(i32);
impl WorldClock for ControllerClock {
    fn tick(&self) -> i32 {
        self.0
    }
}
impl crate::quest::WorldClock for ControllerClock {
    fn tick(&self) -> i32 {
        self.0
    }
}

/// Bounded cascade depth for IR-driven trigger chains.
const IR_TRIGGER_CASCADE_DEPTH: usize = 8;

/// [`EntityResolver`] for DB-backed (non-combat) scenes.
///
/// Combat keeps its combatants in memory (`CombatScene` is its own resolver);
/// outside combat the player's entity lives in the DB, so this loads the player
/// character up front, serves snapshots of it, and applies `hp` deltas to an
/// owned copy that the controller flushes back to the database after the pass.
/// Status/modifier intents persist directly via the services and need no flush.
struct WorldEntityResolver {
    character: Character,
    hp_changed: bool,
}

impl WorldEntityResolver {
    fn new(character: Character) -> Self {
        Self {
            character,
            hp_changed: false,
        }
    }
}

impl EntityResolver for WorldEntityResolver {
    fn snapshot(&self, entity_id: &str) -> Option<EntitySnapshot> {
        (entity_id == self.character.id).then(|| EntitySnapshot::from_character(&self.character))
    }

    fn apply_resource_delta(&mut self, entity_id: &str, resource: &str, delta: i64) {
        if entity_id != self.character.id || resource != "hp" {
            return;
        }
        let new_hp =
            (self.character.hp as i64 + delta).clamp(0, self.character.max_hp as i64) as i32;
        self.character.hp = new_hp;
        self.hp_changed = true;
    }
}

// ---------------------------------------------------------------------------
// GMController
// ---------------------------------------------------------------------------

/// Orchestrates GM scene state and routes player input to scene handlers.
pub struct GMController<'db> {
    db: &'db WorldDatabase,
    bus: EventBus,
    domain_events: DomainEventDispatcher<'db>,
    registry: TerrainRegistry,
    narrator: Narrator,
    tick: i64,
    state: SceneState,
    // Active scene per variant — only one is active at a time
    char_creation: Option<CharacterCreationScene>,
    exploration: Option<ExplorationScene>,
    combat: Option<CombatScene>,
    respawn: Option<RespawnScene>,
    social: Option<SocialScene>,
    shopping: Option<ShoppingScene>,
    dungeon: Option<DungeonScene>,
    town: Option<TownScene>,
    level_up: Option<LevelUpScene>,
    chaos_tracker: ChaosTracker,
    event_accumulator: Option<Vec<GameEvent>>,
    /// Compiled IR content projected for the runtime; `None` until loaded.
    runtime_content: Option<RuntimeContentStore>,
    /// Whether IR triggers run during combat (true only when `ir_records` is
    /// non-empty, so existing un-populated worlds are untouched).
    ir_triggers_enabled: bool,
}

impl<'db> GMController<'db> {
    /// Create a new GMController bound to `db` for the session lifetime.
    pub fn new(
        db: &'db WorldDatabase,
        bus: EventBus,
        registry: TerrainRegistry,
        narrator: Narrator,
    ) -> Self {
        let mut domain_events = DomainEventDispatcher::default();
        register_gm_event_handlers(&mut domain_events, db);

        Self {
            db,
            bus,
            domain_events,
            registry,
            narrator,
            tick: 0,
            state: SceneState::CharacterCreation,
            char_creation: Some(CharacterCreationScene::new(0)),
            exploration: None,
            combat: None,
            respawn: None,
            social: None,
            shopping: None,
            dungeon: None,
            town: None,
            level_up: None,
            chaos_tracker: ChaosTracker::new(5),
            event_accumulator: None,
            runtime_content: None,
            ir_triggers_enabled: false,
        }
    }

    /// Restore GM state from the database (scene, tick, chaos factor).
    pub fn initialize(&mut self) {
        self.load_runtime_content();
        let gm_repo = GMStateRepository::new(self.db);
        if let Ok(t) = gm_repo.get_int("tick", 0) {
            self.tick = t;
        }
        if let Ok(Some(cv)) = gm_repo.get_value("oracle_chaos_factor") {
            if let Ok(n) = cv.parse::<i32>() {
                self.chaos_tracker.set(n);
            }
        }
        let entities = EntityRepository::new(self.db);
        let has_character = entities.has_living_character().unwrap_or(false);
        if has_character {
            self.state = SceneState::Exploration;
            self.exploration = Some(self.make_exploration_scene());
            self.char_creation = None;
        } else {
            self.state = SceneState::CharacterCreation;
            self.char_creation = Some(CharacterCreationScene::new(self.tick));
        }
    }

    // -----------------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------------

    pub fn tick(&self) -> i64 {
        self.tick
    }

    pub fn state(&self) -> SceneState {
        self.state
    }

    /// Process a text input from the player and return resulting events.
    pub fn handle_input(&mut self, text: &str) -> Vec<GameEvent> {
        let (start_index, created) = self.begin_event_capture();
        let cmd = CommandParser.parse(text);

        let scene_events: Vec<GameEvent> = if cmd.verb == "admin" {
            self.handle_admin_cmd(&cmd)
        } else {
            self.tick += 1;
            self.dispatch_to_scene(&cmd)
        };

        let public_events = self.resolve_domain_events(scene_events.clone());
        // IR-driven triggers fire on the resolved events (guarded; no-op until a
        // world's `ir_records` is populated). Their emitted events publish too.
        let trigger_events = self.run_ir_triggers(&public_events);
        self.publish_events(public_events.clone());
        if !trigger_events.is_empty() {
            self.publish_events(trigger_events.clone());
        }
        // The world clock advanced this turn (admin commands don't tick), so
        // resolve over-time IR effects: fire `time.tick` for status-bearing
        // entities and expire elapsed statuses.
        if cmd.verb != "admin" {
            let tick_events = self.run_time_tick();
            if !tick_events.is_empty() {
                self.publish_events(tick_events.clone());
            }
        }

        let new_state = self.detect_transition(&scene_events, &public_events);
        if let Some(ns) = new_state {
            let pre_transition_state = self.state;
            self.wire_for_transition(ns, &scene_events);
            self.transition_to(ns);
            // When transitioning from character creation to exploration via the
            // chat flow, emit the opening look narration and exploration.actions
            // notice so the client advances identically to the form flow
            // (which calls get_initial_narration inside broadcast_creation).
            if pre_transition_state == SceneState::CharacterCreation
                && ns == SceneState::Exploration
            {
                let opening = self.get_initial_narration();
                self.publish_events(opening);
            }
        }

        self.save_state();

        self.end_event_capture(start_index, created)
    }

    /// Get the current scene's prompt string.
    pub fn get_prompt(&mut self) -> String {
        match self.state {
            SceneState::CharacterCreation => self
                .char_creation
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::Exploration => self
                .exploration
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::Combat => self
                .combat
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::Respawn => self
                .respawn
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::Social => self
                .social
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::Shopping => self
                .shopping
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::Dungeon => self
                .dungeon
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::Town => self
                .town
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
            SceneState::LevelUp => self
                .level_up
                .as_mut()
                .map(|s| s.get_prompt(self.db))
                .unwrap_or_default(),
        }
    }

    /// Return opening narration for the current scene.
    pub fn get_initial_narration(&mut self) -> Vec<GameEvent> {
        match self.state {
            SceneState::CharacterCreation => {
                if let Some(s) = &self.char_creation {
                    s.start()
                } else {
                    vec![]
                }
            }
            SceneState::Exploration => {
                let look_cmd = ParsedCommand {
                    raw: "look".to_string(),
                    verb: "look".to_string(),
                    args: vec![],
                    direction: None,
                };
                if let Some(s) = &mut self.exploration {
                    s.handle_command(&look_cmd, self.db).unwrap_or_default()
                } else {
                    vec![]
                }
            }
            SceneState::Combat => {
                // Resync a client that (re)connected mid-fight: re-announce the
                // scene so the combat UI mounts, then replay the current roster
                // (`combat.start`), grid (`combat.positions`), and action bar
                // (`combat.actions`) snapshots. The frontend already handles
                // these events, so no client change is needed for reconnect
                // recovery.
                let mut change_data = JsonObject::new();
                change_data.insert("from".to_string(), JsonValue::String("combat".to_string()));
                change_data.insert("to".to_string(), JsonValue::String("combat".to_string()));
                let mut events = vec![GameEvent::new(self.tick, "gm.scene_change", change_data)];
                if let Some(c) = &self.combat {
                    events.push(c.make_start_event(self.tick));
                    events.push(c.make_positions_event(self.tick));
                    events.push(c.actions_event(self.tick));
                }
                events
            }
            _ => vec![self.make_narrate_event("...")],
        }
    }

    /// Switch to a new scene state, emitting a `gm.scene_change` event.
    pub fn transition_to(&mut self, new_state: SceneState) -> Vec<GameEvent> {
        let (start_index, created) = self.begin_event_capture();
        let old_state = self.state;
        self.state = new_state;

        match new_state {
            SceneState::Exploration => {
                self.exploration = Some(self.make_exploration_scene());
                self.combat = None;
                self.respawn = None;
            }
            SceneState::CharacterCreation => {
                self.char_creation = Some(CharacterCreationScene::new(self.tick));
                self.exploration = None;
                self.combat = None;
                self.respawn = None;
            }
            SceneState::Combat => {
                if self.combat.is_none() {
                    eprintln!("transition_to: COMBAT requested but no combat scene set");
                }
            }
            SceneState::Respawn => {
                if self.respawn.is_none() {
                    eprintln!("transition_to: RESPAWN requested but no respawn scene set");
                }
            }
            SceneState::Social => {
                if self.social.is_none() {
                    eprintln!("transition_to: SOCIAL requested but no social scene set");
                }
            }
            SceneState::Shopping => {
                if self.shopping.is_none() {
                    eprintln!("transition_to: SHOPPING requested but no shopping scene set");
                }
            }
            SceneState::Dungeon => {
                if self.dungeon.is_none() {
                    eprintln!("transition_to: DUNGEON requested but no dungeon scene set");
                }
            }
            SceneState::Town => {
                if self.town.is_none() {
                    eprintln!("transition_to: TOWN requested but no town scene set");
                }
            }
            SceneState::LevelUp => {
                if self.level_up.is_none() {
                    eprintln!("transition_to: LEVEL_UP requested but no level-up scene set");
                }
            }
        }

        let mut change_data = JsonObject::new();
        change_data.insert(
            "from".to_string(),
            JsonValue::String(old_state.as_str().to_string()),
        );
        change_data.insert(
            "to".to_string(),
            JsonValue::String(new_state.as_str().to_string()),
        );
        let change_event = GameEvent::new(self.tick, "gm.scene_change", change_data);
        self.publish_event(change_event);

        // Emit combat.start immediately after scene_change so the frontend
        // panel can initialize the enemy roster with HP data.
        // Also emit combat.positions and combat.actions right after so the
        // encounter grid and action bar are ready on the first render.
        if new_state == SceneState::Combat {
            // Build events first (immutable borrow), then publish each (mut borrow).
            // Splitting into separate let-bindings releases the immutable borrow
            // before publish_event takes &mut self.
            let start_ev = self.combat.as_ref().map(|c| c.make_start_event(self.tick));
            let pos_ev = self.combat.as_ref().map(|c| c.make_positions_event(self.tick));
            let actions_ev = self.combat.as_ref().map(|c| c.actions_event(self.tick));
            if let Some(ev) = start_ev {
                self.publish_event(ev);
            }
            if let Some(ev) = pos_ev {
                self.publish_event(ev);
            }
            if let Some(ev) = actions_ev {
                self.publish_event(ev);
            }
        }

        self.run_scene_check(old_state, new_state);

        self.end_event_capture(start_index, created)
    }

    // -----------------------------------------------------------------------
    // Scene dispatch
    // -----------------------------------------------------------------------

    fn dispatch_to_scene(&mut self, cmd: &ParsedCommand) -> Vec<GameEvent> {
        match self.state {
            SceneState::CharacterCreation => self
                .char_creation
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::Exploration => self
                .exploration
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::Combat => self
                .combat
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::Respawn => self
                .respawn
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::Social => self
                .social
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::Shopping => self
                .shopping
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::Dungeon => self
                .dungeon
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::Town => self
                .town
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
            SceneState::LevelUp => self
                .level_up
                .as_mut()
                .and_then(|s| s.handle_command(cmd, self.db).ok())
                .unwrap_or_default(),
        }
    }

    fn check_transitions_current(&self, scene_events: &[GameEvent]) -> Option<SceneState> {
        match self.state {
            SceneState::CharacterCreation => self
                .char_creation
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::Exploration => self
                .exploration
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::Combat => self
                .combat
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::Respawn => self
                .respawn
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::Social => self
                .social
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::Shopping => self
                .shopping
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::Dungeon => self
                .dungeon
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::Town => self
                .town
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
            SceneState::LevelUp => self
                .level_up
                .as_ref()
                .and_then(|s| s.check_transitions(scene_events)),
        }
    }

    fn detect_transition(
        &self,
        scene_events: &[GameEvent],
        public_events: &[GameEvent],
    ) -> Option<SceneState> {
        let mut ns = self.check_transitions_current(scene_events);
        if ns.is_none() {
            for ev in public_events {
                if ev.event_type == "character.level_up" {
                    ns = Some(SceneState::LevelUp);
                    break;
                }
            }
        }
        ns
    }

    // -----------------------------------------------------------------------
    // Scene wiring
    // -----------------------------------------------------------------------

    fn wire_for_transition(&mut self, new_state: SceneState, scene_events: &[GameEvent]) {
        match new_state {
            SceneState::Combat => self.wire_combat_scene(),
            SceneState::Respawn => self.wire_respawn_scene(scene_events),
            SceneState::Social => self.wire_social_scene(),
            SceneState::Shopping => self.wire_shopping_scene(),
            SceneState::Dungeon => self.wire_dungeon_scene(),
            SceneState::Town => self.wire_town_scene(),
            SceneState::LevelUp => self.wire_level_up_scene(),
            _ => {}
        }
    }

    fn wire_combat_scene(&mut self) {
        if let Some(exploration) = &mut self.exploration {
            if let Some(pending) = exploration.take_pending_combat() {
                let entities = EntityRepository::new(self.db);
                let character = match entities.load_first_character() {
                    Ok(Some(c)) => c,
                    _ => return,
                };
                let awareness = match pending.awareness_result.as_str() {
                    "player_surprise" => AwarenessResult::PlayerSurprise,
                    "enemy_surprise" => AwarenessResult::EnemySurprise,
                    _ => AwarenessResult::MutualAwareness,
                };
                let mut rng = thread_rng();
                let creatures: Vec<CreatureData> = pending
                    .creatures
                    .iter()
                    .filter_map(|obj| {
                        serde_json::from_value(JsonValue::Object(obj.clone())).ok()
                    })
                    .collect();
                // Load the current difficulty profile so combat inherits all scalars.
                let difficulty_profile = DifficultySettingsRepository::new(self.db)
                    .load_profile()
                    .unwrap_or_default();
                let combat_state = create_combat(
                    &character,
                    &creatures,
                    awareness,
                    &mut rng,
                    &pending.terrain,
                    &pending.features,
                    difficulty_profile.enemy_hp_mult,
                );
                // Hand combat the world's IR content so enemies with authored
                // `actions` resolve them through the IR pipeline on their turn.
                let store = self.runtime_content.clone();
                self.combat = Some(CombatScene::new(
                    character,
                    combat_state,
                    creatures,
                    None,
                    store,
                    difficulty_profile,
                ));
            }
        }
    }

    fn wire_respawn_scene(&mut self, scene_events: &[GameEvent]) {
        let entities = EntityRepository::new(self.db);
        if let Ok(Some(character)) = entities.load_first_character() {
            let (death_q, death_r) = scene_events
                .iter()
                .find(|ev| ev.event_type == "character.death")
                .and_then(|ev| {
                    let q = ev.data.get("position_q").and_then(|v| v.as_i64())? as i32;
                    let r = ev.data.get("position_r").and_then(|v| v.as_i64())? as i32;
                    Some((q, r))
                })
                .unwrap_or((character.position_q, character.position_r));
            self.respawn = Some(RespawnScene::new(character, death_q, death_r));
        }
    }

    fn wire_social_scene(&mut self) {
        let is_town = self.state == SceneState::Town;
        let ctx = self
            .exploration
            .as_mut()
            .and_then(|s| s.take_pending_social())
            .or_else(|| self.town.as_mut().and_then(|s| s.take_pending_social()));

        if let Some(SocialSceneContext {
            npc_entity_id,
            npc_name,
            npc_data,
        }) = ctx
        {
            let npc_json = JsonValue::Object(npc_data);
            let npc_record: SceneNpcRecord =
                serde_json::from_value(npc_json.clone()).unwrap_or_else(|_| {
                    let mut map = JsonObject::new();
                    map.insert("name".to_string(), JsonValue::String(npc_name.clone()));
                    serde_json::from_value(JsonValue::Object(map)).unwrap_or_else(|_| {
                        serde_json::from_value(JsonValue::Object(JsonObject::new())).unwrap()
                    })
                });
            let return_to = if is_town {
                Some("town".to_string())
            } else {
                None
            };
            self.social =
                Some(SocialScene::new(npc_entity_id, npc_name, npc_record, self.tick, return_to));
        }
    }

    fn wire_shopping_scene(&mut self) {
        let is_town = self.state == SceneState::Town;
        let ctx = self
            .exploration
            .as_mut()
            .and_then(|s| s.take_pending_shopping())
            .or_else(|| self.town.as_mut().and_then(|s| s.take_pending_shopping()));

        if let Some(ShoppingSceneContext { building_name, .. }) = ctx {
            let name = building_name.unwrap_or_else(|| "Shop".to_string());
            let return_to = if is_town {
                Some("town".to_string())
            } else {
                None
            };
            self.shopping = Some(ShoppingScene::with_default_items(name, self.tick, return_to));
        }
    }

    fn wire_dungeon_scene(&mut self) {
        if let Some(ctx) =
            self.exploration.as_mut().and_then(|s| s.take_pending_dungeon())
        {
            let DungeonSceneContext {
                dungeon_id,
                rooms,
                connections,
                entry_q,
                entry_r,
            } = ctx;
            let rooms: Vec<DungeonRoom> = rooms
                .iter()
                .filter_map(|o| serde_json::from_value(JsonValue::Object(o.clone())).ok())
                .collect();
            let connections: Vec<DungeonConnection> = connections
                .iter()
                .filter_map(|o| serde_json::from_value(JsonValue::Object(o.clone())).ok())
                .collect();
            self.dungeon = Some(DungeonScene::new(
                dungeon_id,
                rooms,
                connections,
                entry_q,
                entry_r,
                self.tick,
            ));
        }
    }

    fn wire_town_scene(&mut self) {
        if let Some(ctx) =
            self.exploration.as_mut().and_then(|s| s.take_pending_town())
        {
            let TownSceneContext {
                settlement_data,
                q,
                r,
                town_cells,
            } = ctx;
            let settlement: SettlementSummary =
                serde_json::from_value(JsonValue::Object(settlement_data))
                    .unwrap_or_else(|_| SettlementSummary {
                        name: "Unknown".to_string(),
                        size: "village".to_string(),
                        description: String::new(),
                        settlement_id: String::new(),
                        establishments: vec![],
                        resident_npc_ids: vec![],
                    });
            let cells: Vec<TownCell> = town_cells
                .iter()
                .filter_map(|o| serde_json::from_value(JsonValue::Object(o.clone())).ok())
                .collect();
            self.town = Some(TownScene::new(settlement, q, r, cells, self.tick));
        }
    }

    fn wire_level_up_scene(&mut self) {
        let entities = EntityRepository::new(self.db);
        if let Ok(Some(character)) = entities.load_first_character() {
            self.level_up = Some(LevelUpScene::new(character));
        }
    }

    fn make_exploration_scene(&self) -> ExplorationScene {
        ExplorationScene::new(self.registry.clone(), self.narrator.clone(), self.tick)
    }

    // -----------------------------------------------------------------------
    // Event infrastructure
    // -----------------------------------------------------------------------

    fn begin_event_capture(&mut self) -> (usize, bool) {
        let created = self.event_accumulator.is_none();
        if created {
            self.event_accumulator = Some(Vec::new());
        }
        let start = self.event_accumulator.as_ref().map(|a| a.len()).unwrap_or(0);
        (start, created)
    }

    fn end_event_capture(&mut self, start_index: usize, created: bool) -> Vec<GameEvent> {
        let captured = self
            .event_accumulator
            .as_ref()
            .map(|acc| acc[start_index..].to_vec())
            .unwrap_or_default();
        if created {
            self.event_accumulator = None;
        }
        captured
    }

    fn publish_event(&mut self, event: GameEvent) -> Vec<GameEvent> {
        let published = self.bus.publish(event);
        if let Some(acc) = &mut self.event_accumulator {
            acc.extend(published.clone());
        }
        published
    }

    fn publish_events(&mut self, events: Vec<GameEvent>) {
        for ev in events {
            self.publish_event(ev);
        }
    }

    /// Re-read the world's `ir_records` into the runtime content store.
    ///
    /// Called after content is stored to the loaded world (e.g. via the Content
    /// Studio) so authored IR goes live without a full world reload.
    pub fn reload_runtime_content(&mut self) {
        self.load_runtime_content();
    }

    /// Load compiled IR records from the world's `ir_records` table into the
    /// runtime content store, enabling IR triggers only if any are present.
    fn load_runtime_content(&mut self) {
        let repo = IRRecordRepository::new(self.db);
        match RuntimeContentStore::from_repository(&repo) {
            Ok((store, errors)) => {
                for err in &errors {
                    eprintln!("load_runtime_content: {err}");
                }
                self.ir_triggers_enabled = !store.is_empty();
                self.runtime_content = Some(store);
            }
            Err(e) => {
                eprintln!("load_runtime_content: failed to read ir_records: {e}");
                self.ir_triggers_enabled = false;
                self.runtime_content = None;
            }
        }
    }

    /// Run IR-driven triggers for `events` against the active combat scene.
    ///
    /// Guarded: a no-op unless IR triggers are enabled and we're in combat, so
    /// worlds without authored IR content behave exactly as before. Fail-safe —
    /// errors are logged, never panicked.
    fn run_ir_triggers(&mut self, events: &[GameEvent]) -> Vec<GameEvent> {
        if !self.ir_triggers_enabled || events.is_empty() {
            return Vec::new();
        }
        // Shared borrow of `runtime_content`; the resolver borrows a disjoint field
        // (`combat`) or owns DB-loaded state, so there is no aliasing conflict.
        let store = match self.runtime_content.as_ref() {
            Some(store) => store,
            None => return Vec::new(),
        };
        let db = self.db;
        let tick = self.tick;
        let status = StatusEffectService::new(
            StatusEffectRepository::new(db),
            store,
            Some(ControllerClock(tick as i32)),
        );
        let runtime = TriggerRuntime::new(store, &status, db, tick, IR_TRIGGER_CASCADE_DEPTH);

        // Pick the resolver for the active scene. Combat uses its in-memory
        // combatants; every other scene snapshots the DB-backed player entity.
        let outcome = if self.state == SceneState::Combat {
            match self.combat.as_mut() {
                Some(combat) => runtime.run(combat, events),
                None => return Vec::new(),
            }
        } else {
            let entities = EntityRepository::new(db);
            let character = match entities.load_first_character() {
                Ok(Some(c)) => c,
                _ => return Vec::new(),
            };
            let mut resolver = WorldEntityResolver::new(character);
            let outcome = runtime.run(&mut resolver, events);
            // Flush the (possibly) mutated hp back to the world; status/modifier
            // intents already persisted through the services.
            if resolver.hp_changed {
                if let Err(e) = entities.save_character_state(&mut resolver.character) {
                    eprintln!("ir_trigger: failed to persist hp delta: {e}");
                }
            }
            outcome
        };

        for err in &outcome.errors {
            eprintln!("ir_trigger: {err}");
        }
        outcome.emitted
    }

    /// World-clock heartbeat: fire `on: time.tick` triggers for every entity that
    /// carries an active status (the over-time carriers), then expire statuses
    /// whose duration has elapsed. This is what makes poison tick on the clock and
    /// buffs decay over time, in any scene — independent of who is acting.
    fn run_time_tick(&mut self) -> Vec<GameEvent> {
        if !self.ir_triggers_enabled {
            return Vec::new();
        }
        let tick = self.tick;
        let entity_ids = match StatusEffectRepository::new(self.db).distinct_entity_ids() {
            Ok(ids) => ids,
            Err(e) => {
                eprintln!("ir_trigger: status enumerate failed: {e}");
                return Vec::new();
            }
        };

        // One `time.tick` event per status-bearing entity; `self_id` makes the
        // runtime source that entity's triggers (its statuses/items/traits).
        let emitted = if entity_ids.is_empty() {
            Vec::new()
        } else {
            let events: Vec<GameEvent> = entity_ids
                .iter()
                .map(|id| {
                    let mut data = JsonObject::new();
                    data.insert("self_id".into(), JsonValue::String(id.clone()));
                    data.insert("tick".into(), JsonValue::from(tick));
                    GameEvent::new(tick, "time.tick", data)
                })
                .collect();
            self.run_ir_triggers(&events)
        };

        // Drive status expiry by tick count (after ticking, so a status ticks on
        // its final tick before being removed).
        if let Err(e) = StatusEffectRepository::new(self.db).expire_due(tick as i32) {
            eprintln!("ir_trigger: expire_due failed: {e}");
        }
        emitted
    }

    fn resolve_domain_events(&mut self, events: Vec<GameEvent>) -> Vec<GameEvent> {
        let mut resolved = Vec::new();
        for ev in events {
            // HR-776: Route status lifecycle requests through an on-demand service
            // built from the current runtime content.  The lifetime-bound
            // `register_status_effect_handlers` function could not be wired at
            // construction time (runtime_content is None until world-load); instead
            // we handle these two event types here so the event-driven path is
            // always live once a world is loaded.  Combat statuses use the service
            // directly (not via events) and are unaffected by this routing.
            let is_apply = ev.event_type == "status.apply_requested";
            let is_remove = ev.event_type == "status.remove_requested";
            if is_apply || is_remove {
                if let Some(store) = self.runtime_content.as_ref() {
                    let tick = self.tick;
                    let db = self.db;
                    let status = StatusEffectService::new(
                        StatusEffectRepository::new(db),
                        store,
                        Some(ControllerClock(tick as i32)),
                    );
                    let handler_events = if is_apply {
                        handle_apply_requested(&status, &ev)
                    } else {
                        handle_remove_requested(&status, &ev)
                    };
                    resolved.extend(handler_events);
                }
                // If runtime_content is None the status cannot be resolved yet —
                // skip silently (no subscriber was registered anyway).
                continue;
            }
            // HR-19: resolve quest lifecycle requests on-demand (same pattern as
            // statuses). Quest content loads from the pack content dir.
            if ev.event_type.starts_with("quest.") && ev.event_type.ends_with("_requested") {
                let tick = self.tick;
                let db = self.db;
                let service = crate::quest::QuestService::new(
                    crate::quest::QuestRepository::new(db),
                    crate::quest::QuestCatalog::load_default(),
                    Some(ControllerClock(tick as i32)),
                );
                let handler_events = match ev.event_type.as_str() {
                    "quest.accept_requested" => {
                        crate::quest::handle_accept_requested(&service, &ev)
                    }
                    "quest.progress_requested" => {
                        crate::quest::handle_progress_requested(&service, &ev)
                    }
                    "quest.complete_requested" => {
                        crate::quest::handle_complete_requested(&service, &ev)
                    }
                    "quest.fail_requested" => crate::quest::handle_fail_requested(&service, &ev),
                    _ => vec![],
                };
                resolved.extend(handler_events);
                continue;
            }
            let dispatched = self.domain_events.dispatch(ev);
            for de in dispatched {
                if !de.event_type.ends_with("_requested") {
                    resolved.push(de);
                }
            }
        }
        resolved
    }

    // -----------------------------------------------------------------------
    // Flow helpers
    // -----------------------------------------------------------------------

    fn run_scene_check(&mut self, _old_state: SceneState, new_state: SceneState) {
        if matches!(new_state, SceneState::CharacterCreation | SceneState::Respawn) {
            return;
        }
        let mut rng = thread_rng();
        let checker = SceneChecker;
        let result = checker.check(self.chaos_tracker.value(), &mut rng);

        let mut check_data = JsonObject::new();
        check_data.insert("roll".to_string(), JsonValue::Number(result.roll.into()));
        check_data.insert(
            "chaos_factor".to_string(),
            JsonValue::Number(result.chaos_factor.into()),
        );
        check_data.insert(
            "modification".to_string(),
            JsonValue::String(format!("{:?}", result.modification)),
        );
        self.publish_event(GameEvent::new(self.tick, "oracle.scene_check", check_data));

        let old_chaos = self.chaos_tracker.value();
        match result.modification {
            SceneModification::Interrupt => {
                let generator = RandomEventGenerator;
                if let Ok(event) = generator.generate(&mut rng) {
                    let mut ev_data = JsonObject::new();
                    ev_data.insert("focus".to_string(), JsonValue::String(event.focus));
                    ev_data.insert("action".to_string(), JsonValue::String(event.action));
                    ev_data.insert("subject".to_string(), JsonValue::String(event.subject));
                    self.publish_event(GameEvent::new(self.tick, "oracle.random_event", ev_data));
                    self.publish_event(self.make_narrate_event(&result.narration));
                }
                self.chaos_tracker.increase();
            }
            SceneModification::Altered => {
                self.publish_event(self.make_narrate_event(&result.narration));
                self.chaos_tracker.increase();
            }
            _ => {
                self.chaos_tracker.decrease();
            }
        }
        let new_chaos = self.chaos_tracker.value();
        self.emit_chaos_change(old_chaos, new_chaos);

        let gm_repo = GMStateRepository::new(self.db);
        let _ = gm_repo.set_value(
            "oracle_chaos_factor",
            &self.chaos_tracker.value().to_string(),
        );
    }

    /// Publish an `oracle.chaos_changed` event when the chaos factor moved.
    ///
    /// When `new_chaos == old_chaos` (e.g. an increase/decrease clamped at the
    /// 1..=9 bounds), no event is emitted — the frontend's chaos display must
    /// not flicker on a no-op change.
    fn emit_chaos_change(&mut self, old_chaos: i32, new_chaos: i32) {
        if new_chaos == old_chaos {
            return;
        }
        let mut chaos_data = JsonObject::new();
        chaos_data.insert(
            "old_value".to_string(),
            JsonValue::Number(old_chaos.into()),
        );
        chaos_data.insert(
            "new_value".to_string(),
            JsonValue::Number(new_chaos.into()),
        );
        chaos_data.insert(
            "chaos_factor".to_string(),
            JsonValue::Number(new_chaos.into()),
        );
        self.publish_event(GameEvent::new(
            self.tick,
            "oracle.chaos_changed",
            chaos_data,
        ));
    }

    fn save_state(&mut self) {
        let gm_repo = GMStateRepository::new(self.db);
        let _ = gm_repo.set_value("scene", self.state.as_str());
        let _ = gm_repo.set_value("tick", &self.tick.to_string());
    }

    // -----------------------------------------------------------------------
    // Admin command handler (stub — AdminService not yet ported)
    // -----------------------------------------------------------------------

    fn handle_admin_cmd(&mut self, _cmd: &ParsedCommand) -> Vec<GameEvent> {
        vec![self
            .make_narrate_event("[ADMIN] Admin commands not yet available in this build.")]
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_narrate_event(&self, text: &str) -> GameEvent {
        let mut data = JsonObject::new();
        data.insert("text".to_string(), JsonValue::String(text.to_string()));
        GameEvent::new(self.tick, "gm.narrate", data).with_source("gm")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use std::path::Path;

    fn open_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn make_controller(db: &WorldDatabase) -> GMController<'_> {
        let registry = TerrainRegistry::new();
        let narrator = Narrator::new(registry.clone(), Path::new(""));
        GMController::new(db, EventBus::default(), registry, narrator)
    }

    #[test]
    fn new_controller_starts_in_character_creation() {
        let db = open_db();
        let ctrl = make_controller(&db);
        assert_eq!(ctrl.state(), SceneState::CharacterCreation);
    }

    #[test]
    fn initialize_with_no_character_stays_in_creation() {
        let db = open_db();
        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert_eq!(ctrl.state(), SceneState::CharacterCreation);
    }

    #[test]
    fn ir_triggers_disabled_when_no_ir_records() {
        let db = open_db();
        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert!(!ctrl.ir_triggers_enabled, "empty ir_records must leave triggers off");
        // The guard makes run_ir_triggers a no-op even with a combat-shaped event.
        let mut data = JsonObject::new();
        data.insert("attacker_id".into(), JsonValue::String("pc".into()));
        data.insert("target_id".into(), JsonValue::String("hound".into()));
        let emitted = ctrl.run_ir_triggers(&[GameEvent::new(0, "combat.attack", data)]);
        assert!(emitted.is_empty());
    }

    #[test]
    fn ir_triggers_enabled_when_ir_records_present() {
        let db = open_db();
        let repo = IRRecordRepository::new(&db);
        let record: JsonObject = serde_json::from_value(serde_json::json!({
            "component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
            "provides_tags": ["poisoned"], "stacking": "replace"
        }))
        .unwrap();
        repo.upsert_many(&[record], "content-studio").unwrap();

        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert!(ctrl.ir_triggers_enabled, "populated ir_records must enable triggers");
        // Still guarded by scene: outside combat it does nothing.
        assert_ne!(ctrl.state(), SceneState::Combat);
        let emitted = ctrl.run_ir_triggers(&[GameEvent::new(0, "combat.attack", JsonObject::new())]);
        assert!(emitted.is_empty());
    }

    #[test]
    fn enter_hex_fires_ir_trigger_in_exploration() {
        let db = open_db();
        // Seed IR: a character trait that applies `dread` on entering a ruins hex.
        let repo = IRRecordRepository::new(&db);
        let records: Vec<JsonObject> = [
            serde_json::json!({
                "component_type": "trait", "id": "ruin_dread", "name": "Dread of the Ruins",
                "category": "regional",
                "triggers": [{"id": "dread_on_enter_ruins", "on": "exploration.enter_hex",
                              "when": "event.terrain == 'ruins'",
                              "do": [{"kind": "apply_status",
                                      "params": {"status_id": "dread", "duration_ticks": 6}}]}]
            }),
            serde_json::json!({
                "component_type": "status_effect", "id": "dread", "name": "Dread",
                "provides_tags": ["dread"], "stacking": "replace"
            }),
        ]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        repo.upsert_many(&records, "test").unwrap();

        // A player carrying the trait, persisted so load_first_character finds it.
        let mut character = Character::new("Wanderer", "expert");
        character.id = "pc".into();
        character.traits = vec!["ruin_dread".into()];
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert_eq!(ctrl.state(), SceneState::Exploration);
        assert!(ctrl.ir_triggers_enabled);

        // Entering a ruins hex fires the trait's trigger → applies `dread`.
        let mut data = JsonObject::new();
        data.insert("self_id".into(), JsonValue::String("pc".into()));
        data.insert("terrain".into(), JsonValue::String("ruins".into()));
        ctrl.run_ir_triggers(&[GameEvent::new(0, "exploration.enter_hex", data)]);

        let active = StatusEffectRepository::new(&db).list_for_entity("pc").unwrap();
        assert_eq!(active.len(), 1, "ruins entry should apply one status");
        assert_eq!(active[0].effect_id, "dread");

        // A non-ruins hex must NOT fire (the `when` gates on terrain).
        let mut plains = JsonObject::new();
        plains.insert("self_id".into(), JsonValue::String("pc".into()));
        plains.insert("terrain".into(), JsonValue::String("plains".into()));
        ctrl.run_ir_triggers(&[GameEvent::new(0, "exploration.enter_hex", plains)]);
        let active = StatusEffectRepository::new(&db).list_for_entity("pc").unwrap();
        assert_eq!(active.len(), 1, "plains entry must not add another status");
    }

    #[test]
    fn global_trigger_fires_without_being_carried() {
        let db = open_db();
        // A standalone (global) trigger — not attached to any entity — applies
        // `dread` when entering a crypt hex.
        let records: Vec<JsonObject> = [
            serde_json::json!({
                "component_type": "trigger", "id": "crypt_ward", "on": "exploration.enter_hex",
                "when": "event.terrain == 'crypt'",
                "do": [{"kind": "apply_status",
                        "params": {"status_id": "dread", "duration_ticks": 4}}]
            }),
            serde_json::json!({
                "component_type": "status_effect", "id": "dread", "name": "Dread",
                "provides_tags": ["dread"], "stacking": "replace"
            }),
        ]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        IRRecordRepository::new(&db).upsert_many(&records, "test").unwrap();

        // The player carries NO traits — the global trigger must still fire.
        let mut character = Character::new("Wanderer", "expert");
        character.id = "pc".into();
        assert!(character.traits.is_empty());
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert_eq!(ctrl.state(), SceneState::Exploration);

        let enter = |terrain: &str| {
            let mut d = JsonObject::new();
            d.insert("self_id".into(), JsonValue::String("pc".into()));
            d.insert("terrain".into(), JsonValue::String(terrain.into()));
            GameEvent::new(0, "exploration.enter_hex", d)
        };

        ctrl.run_ir_triggers(&[enter("crypt")]);
        let active = StatusEffectRepository::new(&db).list_for_entity("pc").unwrap();
        assert_eq!(active.len(), 1, "global trigger should apply dread in a crypt");
        assert_eq!(active[0].effect_id, "dread");

        // A non-crypt hex must not fire (the global's `when` gates on terrain).
        StatusEffectRepository::new(&db).remove("pc", "dread").unwrap();
        ctrl.run_ir_triggers(&[enter("plains")]);
        assert!(StatusEffectRepository::new(&db)
            .list_for_entity("pc")
            .unwrap()
            .is_empty());
    }

    #[test]
    fn time_tick_ticks_over_time_status_and_expires_it() {
        let db = open_db();
        // A `bleed` status that loses 1 hp on every world-clock tick, lasting 2.
        let records: Vec<JsonObject> = [serde_json::json!({
            "component_type": "status_effect", "id": "bleed", "name": "Bleed",
            "default_duration_ticks": 2, "provides_tags": ["bleeding"], "stacking": "replace",
            "triggers": [{"id": "bleed_tick", "on": "time.tick",
                          "when": "has_tag(self, 'bleeding')",
                          "do": [{"kind": "change_resource",
                                  "params": {"resource": "hp", "delta": -1}}]}]
        })]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        IRRecordRepository::new(&db).upsert_many(&records, "test").unwrap();

        let mut character = Character::new("Bleeder", "expert");
        character.id = "pc".into();
        character.hp = 10;
        character.max_hp = 10;
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert_eq!(ctrl.state(), SceneState::Exploration);

        // Apply bleed at tick 0 (expires at tick 2) via the controller's content.
        {
            let store = ctrl.runtime_content.as_ref().unwrap();
            let svc = StatusEffectService::new(
                StatusEffectRepository::new(&db),
                store,
                Some(ControllerClock(0)),
            );
            svc.apply("pc", "bleed", None, None, None).unwrap();
        }

        let hp = || {
            EntityRepository::new(&db)
                .load_first_character()
                .unwrap()
                .unwrap()
                .hp
        };
        let active = || StatusEffectRepository::new(&db).list_for_entity("pc").unwrap().len();

        // Turn 1: bleed ticks (hp 10 -> 9), still active (expires at 2).
        ctrl.tick = 1;
        ctrl.run_time_tick();
        assert_eq!(hp(), 9, "bleed should cost 1 hp on tick 1");
        assert_eq!(active(), 1, "bleed still active at tick 1");

        // Turn 2: bleed ticks once more (hp 8) then expires by tick count.
        ctrl.tick = 2;
        ctrl.run_time_tick();
        assert_eq!(hp(), 8, "bleed should cost 1 hp on tick 2");
        assert_eq!(active(), 0, "bleed expires at tick 2");

        // Turn 3: no status left, so the clock ticks with no effect.
        ctrl.tick = 3;
        ctrl.run_time_tick();
        assert_eq!(hp(), 8, "no further hp loss after expiry");
    }

    /// HR-776 regression: `status.apply_requested` events emitted by scenes (e.g.
    /// dungeon torch/lantern) must be resolved through `resolve_domain_events` and
    /// actually persist the status to the database.  Before the fix,
    /// `register_status_effect_handlers` was never called so no subscriber existed
    /// and the event was silently swallowed — `list_for_entity` returned empty.
    #[test]
    fn resolve_domain_events_applies_status_via_apply_requested() {
        let db = open_db();

        // Seed an IR status-effect record so the on-demand service can look it up.
        let records: Vec<JsonObject> = [serde_json::json!({
            "component_type": "status_effect",
            "id": "illuminated",
            "name": "Illuminated",
            "default_duration_ticks": 60,
            "provides_tags": ["illuminated"],
            "stacking": "replace",
        })]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        IRRecordRepository::new(&db).upsert_many(&records, "test").unwrap();

        // A character is required so initialize() finds a living entity.
        let mut character = Character::new("Torchbearer", "expert");
        character.id = "pc".into();
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert_eq!(ctrl.state(), SceneState::Exploration);
        // runtime_content must be populated (IR records were seeded above).
        assert!(ctrl.runtime_content.is_some(), "runtime_content must load on initialize");

        // Build a status.apply_requested event exactly as the dungeon scene does.
        let payload = crate::payloads::transport::StatusApplyRequested {
            entity_id: "pc".into(),
            effect_id: "illuminated".into(),
            duration_ticks: Some(60),
            source: Some("dungeon".into()),
            data: JsonObject::new(),
        };
        let event_data: JsonObject =
            match serde_json::to_value(&payload).unwrap() {
                serde_json::Value::Object(m) => m,
                _ => panic!("payload must serialise to object"),
            };
        let req_event = GameEvent::new(ctrl.tick, "status.apply_requested", event_data)
            .with_source("dungeon");

        // Call the chokepoint under test.
        let result_events = ctrl.resolve_domain_events(vec![req_event]);

        // --- Assertions that FAIL before the fix ---

        // 1. A `status.applied` terminal event must have been returned.
        let applied: Vec<_> = result_events
            .iter()
            .filter(|e| e.event_type == "status.applied")
            .collect();
        assert_eq!(
            applied.len(),
            1,
            "resolve_domain_events must return exactly one status.applied event; got: {:?}",
            result_events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );

        // 2. The effect must actually be persisted in the database.
        let active = StatusEffectRepository::new(&db)
            .list_for_entity("pc")
            .unwrap();
        assert_eq!(
            active.len(),
            1,
            "status effect must be written to DB; list_for_entity returned {:?}",
            active
        );
        assert_eq!(
            active[0].effect_id, "illuminated",
            "persisted effect_id must match the request"
        );
    }

    /// HR-776 regression (remove path): `status.remove_requested` must also be
    /// routed through the on-demand service and return `status.removed`.
    #[test]
    fn resolve_domain_events_removes_status_via_remove_requested() {
        let db = open_db();

        let records: Vec<JsonObject> = [serde_json::json!({
            "component_type": "status_effect",
            "id": "illuminated",
            "name": "Illuminated",
            "default_duration_ticks": 60,
            "provides_tags": ["illuminated"],
            "stacking": "replace",
        })]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        IRRecordRepository::new(&db).upsert_many(&records, "test").unwrap();

        let mut character = Character::new("Torchbearer", "expert");
        character.id = "pc".into();
        EntityRepository::new(&db).create_character(&character).unwrap();

        let mut ctrl = make_controller(&db);
        ctrl.initialize();

        // Pre-apply the effect directly via the service so we have something to remove.
        {
            let store = ctrl.runtime_content.as_ref().unwrap();
            let svc = StatusEffectService::new(
                StatusEffectRepository::new(&db),
                store,
                Some(ControllerClock(ctrl.tick as i32)),
            );
            svc.apply("pc", "illuminated", None, None, None).unwrap();
        }
        assert_eq!(
            StatusEffectRepository::new(&db).list_for_entity("pc").unwrap().len(),
            1,
            "pre-condition: effect must exist before remove"
        );

        // Now fire a status.remove_requested through resolve_domain_events.
        let remove_payload = crate::payloads::transport::StatusRemoveRequested {
            entity_id: "pc".into(),
            effect_id: "illuminated".into(),
        };
        let remove_data: JsonObject =
            match serde_json::to_value(&remove_payload).unwrap() {
                serde_json::Value::Object(m) => m,
                _ => panic!("payload must serialise to object"),
            };
        let rem_event =
            GameEvent::new(ctrl.tick, "status.remove_requested", remove_data).with_source("test");

        let result_events = ctrl.resolve_domain_events(vec![rem_event]);

        let removed: Vec<_> = result_events
            .iter()
            .filter(|e| e.event_type == "status.removed")
            .collect();
        assert_eq!(
            removed.len(),
            1,
            "must return exactly one status.removed event; got: {:?}",
            result_events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );

        assert!(
            StatusEffectRepository::new(&db)
                .list_for_entity("pc")
                .unwrap()
                .is_empty(),
            "effect must be gone from DB after remove"
        );
    }

    /// HR-776 contract: with no runtime content store (fresh controller, before
    /// `initialize`), a `status.apply_requested` event must be safely skipped —
    /// returning an empty vec without panicking.  This pins the "silently skip
    /// when no content" behavior so a future refactor can't turn it into an
    /// `unwrap` panic.
    #[test]
    fn resolve_domain_events_skips_status_when_no_runtime_content() {
        let db = open_db();
        let mut ctrl = make_controller(&db);
        // A fresh controller has not loaded runtime content yet.
        assert!(
            ctrl.runtime_content.is_none(),
            "pre-condition: runtime_content must be None before initialize"
        );

        // Same event construction as the applying-path HR-776 test.
        let payload = crate::payloads::transport::StatusApplyRequested {
            entity_id: "pc".into(),
            effect_id: "illuminated".into(),
            duration_ticks: Some(60),
            source: Some("dungeon".into()),
            data: JsonObject::new(),
        };
        let event_data: JsonObject = match serde_json::to_value(&payload).unwrap() {
            serde_json::Value::Object(m) => m,
            _ => panic!("payload must serialise to object"),
        };
        let req_event = GameEvent::new(0, "status.apply_requested", event_data)
            .with_source("dungeon");

        // Must not panic; the status event is skipped, so nothing is resolved.
        let result_events = ctrl.resolve_domain_events(vec![req_event]);
        assert!(
            result_events.is_empty(),
            "status event must be skipped (no content store) — got: {:?}",
            result_events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );
    }

    #[test]
    fn transition_to_exploration_emits_scene_change() {
        let db = open_db();
        let mut ctrl = make_controller(&db);
        let events = ctrl.transition_to(SceneState::Exploration);
        let has_change = events.iter().any(|e| e.event_type == "gm.scene_change");
        assert!(has_change, "expected gm.scene_change event");
    }

    #[test]
    fn combat_initial_narration_reannounces_scene_for_reconnect() {
        // A client that (re)connects mid-fight calls get_initial_narration. Even
        // with no live combat scene object, it must re-announce the combat scene
        // so the encounter UI mounts (roster/positions replay when a scene is
        // present is covered by the combat-scene event-builder tests).
        let db = open_db();
        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        ctrl.state = SceneState::Combat;
        ctrl.combat = None;
        let events = ctrl.get_initial_narration();
        assert_eq!(events.len(), 1, "no combat scene → just the scene re-announce");
        assert_eq!(events[0].event_type, "gm.scene_change");
        assert_eq!(
            events[0].data.get("to").and_then(|v| v.as_str()),
            Some("combat"),
            "resync must set the scene back to combat",
        );
    }

    #[test]
    fn save_state_persists_tick() {
        let db = open_db();
        let mut ctrl = make_controller(&db);
        ctrl.tick = 42;
        ctrl.save_state();
        let gm_repo = GMStateRepository::new(&db);
        let tick_val = gm_repo.get_value("tick").unwrap();
        assert_eq!(tick_val, Some("42".to_string()));
    }

    #[test]
    fn handle_admin_returns_narrate_event() {
        let db = open_db();
        let mut ctrl = make_controller(&db);
        let events = ctrl.handle_input("admin list mappings");
        assert!(events.iter().any(|e| e.event_type == "gm.narrate"));
    }

    /// HR-777 regression: `run_scene_check` must emit `oracle.chaos_changed`
    /// whenever the chaos factor changes.
    ///
    /// With chaos at 5 (mid-range), a scene check always mutates it (increase→6
    /// on Interrupt/Altered, decrease→4 otherwise), so the event must always be
    /// emitted regardless of the RNG branch.  The test fails without the fix
    /// (no `oracle.chaos_changed` event is published) and passes with it.
    #[test]
    fn run_scene_check_emits_chaos_changed_event() {
        let db = open_db();
        let mut ctrl = make_controller(&db);
        // Put the controller in Exploration so run_scene_check doesn't exit early.
        ctrl.state = SceneState::Exploration;
        // Default chaos is 5 (mid-range): increase→6 or decrease→4, both ≠ 5.
        debug_assert_eq!(ctrl.chaos_tracker.value(), 5);

        // Capture every event published during run_scene_check.
        let (start, created) = ctrl.begin_event_capture();
        ctrl.run_scene_check(SceneState::Exploration, SceneState::Exploration);
        let events = ctrl.end_event_capture(start, created);

        // There must be exactly one oracle.chaos_changed event.
        let chaos_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == "oracle.chaos_changed")
            .collect();
        assert_eq!(
            chaos_events.len(),
            1,
            "expected exactly one oracle.chaos_changed event; all event types: {:?}",
            events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );

        let ev = chaos_events[0];
        let old_value = ev.data["old_value"].as_i64().expect("old_value must be i64");
        let new_value = ev.data["new_value"].as_i64().expect("new_value must be i64");
        let chaos_factor = ev.data["chaos_factor"].as_i64().expect("chaos_factor must be i64");

        assert_eq!(old_value, 5, "old_value must be the seeded chaos of 5");
        assert!(
            new_value == 4 || new_value == 6,
            "new_value must be 4 or 6 (one step from 5); got {new_value}"
        );
        assert_eq!(
            chaos_factor, new_value,
            "chaos_factor must equal new_value; got {chaos_factor}"
        );
    }

    /// HR-777: the emit helper publishes exactly one event when the value moved.
    #[test]
    fn emit_chaos_change_publishes_when_value_moves() {
        let db = open_db();
        let mut ctrl = make_controller(&db);

        let (start, created) = ctrl.begin_event_capture();
        ctrl.emit_chaos_change(5, 6);
        let events = ctrl.end_event_capture(start, created);

        let chaos_events: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == "oracle.chaos_changed")
            .collect();
        assert_eq!(
            chaos_events.len(),
            1,
            "expected exactly one oracle.chaos_changed event"
        );
        let ev = chaos_events[0];
        assert_eq!(ev.data["old_value"].as_i64(), Some(5));
        assert_eq!(ev.data["new_value"].as_i64(), Some(6));
        assert_eq!(ev.data["chaos_factor"].as_i64(), Some(6));
    }

    /// HR-777: the emit helper stays silent when the value is unchanged (the
    /// clamp-guard case, e.g. an increase already pinned at the max of 9).
    #[test]
    fn emit_chaos_change_silent_when_unchanged() {
        let db = open_db();
        let mut ctrl = make_controller(&db);

        let (start, created) = ctrl.begin_event_capture();
        ctrl.emit_chaos_change(9, 9);
        let events = ctrl.end_event_capture(start, created);

        let chaos_count = events
            .iter()
            .filter(|e| e.event_type == "oracle.chaos_changed")
            .count();
        assert_eq!(
            chaos_count, 0,
            "no oracle.chaos_changed event may fire when chaos is unchanged; got: {:?}",
            events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );
    }

    #[test]
    fn move_publishes_exploration_moved_notice_to_frontend() {
        // HR-791 regression: a move must PUBLISH an `exploration.moved` notice that
        // reaches the client. The old `exploration.move_requested` name ended in
        // `_requested`, so `resolve_domain_events` filtered it out before publishing
        // and the marker/LOCATION never updated in the UI.
        let db = open_db();
        let mut character = Character::new("Scout", "expert");
        character.id = "pc".into();
        EntityRepository::new(&db).create_character(&character).unwrap();
        // Seed the current cell + its east neighbour (unknown terrain => passable).
        for (q, r) in [(0_i64, 0_i64), (1, 0)] {
            let sql = format!(
                "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
                 VALUES ({q}, {r}, 'testland', '[]', 1, '{{}}')"
            );
            db.execute(&sql, &[]).unwrap();
        }

        let mut ctrl = make_controller(&db);
        ctrl.initialize();
        assert_eq!(ctrl.state(), SceneState::Exploration);

        // Move east; the returned events are the ones published to the client.
        let events = ctrl.handle_input("e");
        assert!(
            events.iter().any(|e| e.event_type == "exploration.moved"),
            "a move must publish an `exploration.moved` notice (reaches the frontend); got: {:?}",
            events.iter().map(|e| &e.event_type).collect::<Vec<_>>()
        );
        // The old, filtered `_requested` name must not be what the client sees.
        assert!(
            !events.iter().any(|e| e.event_type == "exploration.move_requested"),
            "the movement notice must not use the filtered `_requested` name"
        );
    }
}
