//! Tauri command handlers.
//!
//! Uses shim `ClientCommand` / `ClientQuery` / `ServerEvent` types that mirror
//! the wire format the Vue frontend already speaks. Each handler translates
//! the shim type into a core [`Command`] / [`Query`], applies it, and returns
//! the results in the same wire shape.

use include_dir::{include_dir, Dir};
use outpost_core::colony::ColonyId;
use outpost_core::content::loader::PackLoader;
use outpost_core::difficulty::DifficultyPreset;
use outpost_core::needs::NeedsConfig;
use outpost_core::snapshot::Snapshot as SnapshotDb;
use outpost_core::system::{BodyKind, SystemCommand, SystemRole};
use outpost_core::tech::{TechDef, TechRegistry};
use outpost_core::trade::SiteId;
use outpost_core::{Command, Event, GameEngine, Query, QueryResult};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;
use tauri::State;

use crate::state::EngineState;

/// Embedded content pack shipped inside the binary. Path is relative to this crate's Cargo.toml.
static EMBEDDED_PACK: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../content/base");

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error, Serialize)]
pub enum CmdError {
    #[error("engine not initialised — call bootstrap first")]
    NotInitialised,
    #[error("engine error: {0}")]
    Engine(String),
    #[error("content load error: {0}")]
    Content(String),
    #[error("snapshot error: {0}")]
    Snapshot(String),
    #[error("invalid argument: {0}")]
    InvalidArg(String),
}

impl From<outpost_core::EngineError> for CmdError {
    fn from(e: outpost_core::EngineError) -> Self {
        Self::Engine(e.to_string())
    }
}

type CmdResult<T> = Result<T, CmdError>;

// ── Wire types (mirror the frontend's `Command` / `GameEvent`) ────────────────

/// Client → engine command. Matches the frontend's `Command` discriminated union.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientCommand {
    AdvanceSol,
    FoundColony {
        name: String,
        starting_population: u64,
    },
    QueueConstruction {
        colony_id: String,
        building_type: String,
        slot_cost: u32,
        labor_per_turn: u32,
        construction_cost: Vec<(String, f64)>,
        construction_turns: u32,
    },
    AssignLabour {
        colony_id: String,
        slot: String,
        labour: u64,
    },
    ResearchTech {
        tech_id: String,
    },
    EnqueueResearch {
        tech_id: String,
    },
    FoundColonyAtSite {
        name: String,
        starting_population: u64,
        site_id: String,
        focus: Option<String>,
        #[serde(default)]
        supplies_id: Option<String>,
    },
}

/// Read-only query. Matches the frontend's query message.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientQuery {
    CurrentSol,
    CurrentMonth,
    ListColonies,
    ColonyStatus { colony_id: String },
    ColonyScreen { colony_id: String },
    SystemResearchTotal,
}

/// Wire event returned to the frontend. Matches `GameEvent` (snake_case, `kind`-tagged).
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerEvent {
    ColonySolAdvanced {
        sol: u64,
    },
    StrategicMonthAdvanced {
        month: u64,
    },
    ColonyFounded {
        colony_id: String,
        name: String,
        starting_population: u64,
    },
    ConstructionQueued {
        colony_id: String,
        building_type: String,
        project_id: String,
    },
    ConstructionCancelled {
        colony_id: String,
        project_id: String,
        refund: Vec<(String, f64)>,
    },
    BuildingConstructed {
        colony_id: String,
        building_type: String,
    },
    LabourAssigned {
        colony_id: String,
        slot: String,
        labour: u64,
    },
    NeedsResolved {
        colony_id: String,
        composite_satisfaction: f32,
        stability_delta: f32,
        population_delta: f32,
    },
    ResearchProduced {
        colony_id: String,
        amount: f32,
    },
    ProductionShortfall {
        colony_id: String,
        building_type: String,
        scale: f64,
        reason: String,
    },
    /// Fallback for events we don't have a typed variant for yet.
    Unknown {
        core_kind: String,
    },
}

impl ServerEvent {
    fn from_core(event: &Event) -> Self {
        match event {
            Event::ColonySolAdvanced { sol } => Self::ColonySolAdvanced { sol: *sol },
            Event::StrategicMonthAdvanced { month } => Self::StrategicMonthAdvanced { month: *month },
            Event::ColonyFounded {
                colony_id,
                name,
                starting_population,
            } => Self::ColonyFounded {
                colony_id: colony_id.to_string(),
                name: name.clone(),
                starting_population: *starting_population,
            },
            Event::ConstructionQueued {
                colony_id,
                building_type,
                project_id,
            } => Self::ConstructionQueued {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
                project_id: project_id.to_string(),
            },
            Event::ConstructionCancelled {
                colony_id,
                project_id,
                refund,
            } => Self::ConstructionCancelled {
                colony_id: colony_id.to_string(),
                project_id: project_id.to_string(),
                refund: refund.clone(),
            },
            Event::BuildingConstructed {
                colony_id,
                building_type,
            } => Self::BuildingConstructed {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
            },
            Event::LabourAssigned {
                colony_id,
                slot,
                labour,
            } => Self::LabourAssigned {
                colony_id: colony_id.to_string(),
                slot: slot.clone(),
                labour: *labour,
            },
            Event::NeedsResolved {
                colony_id,
                composite_satisfaction,
                stability_delta,
                population_delta,
            } => Self::NeedsResolved {
                colony_id: colony_id.to_string(),
                composite_satisfaction: *composite_satisfaction,
                stability_delta: *stability_delta,
                population_delta: *population_delta,
            },
            Event::ResearchProduced { colony_id, amount } => Self::ResearchProduced {
                colony_id: colony_id.to_string(),
                amount: *amount,
            },
            Event::ProductionShortfall {
                colony_id,
                building_type,
                scale,
                reason,
            } => Self::ProductionShortfall {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
                scale: *scale,
                reason: format!("{reason:?}"),
            },
            other => Self::Unknown {
                core_kind: format!("{other:?}").split_whitespace().next().unwrap_or("event").to_owned(),
            },
        }
    }
}

/// Initial snapshot returned by `bootstrap` or `snapshot`.
#[derive(Debug, Serialize)]
pub struct SnapshotPayload {
    pub sol: u64,
    pub month: u64,
    pub colonies: Vec<ColonyWire>,
    pub research_total: f32,
}

#[derive(Debug, Serialize)]
pub struct ColonyWire {
    pub id: String,
    pub name: String,
    pub population: f32,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_colony(id: &str) -> Result<ColonyId, CmdError> {
    ColonyId::from_str(id).map_err(|_| CmdError::InvalidArg(format!("bad colony id: {id}")))
}

fn build_snapshot(engine: &GameEngine) -> SnapshotPayload {
    let state = &engine.state;
    let colonies = state
        .colonies
        .iter()
        .zip(state.populations.iter())
        .map(|(c, p)| ColonyWire {
            id: c.id.to_string(),
            name: c.name.clone(),
            population: p.count,
        })
        .collect();
    SnapshotPayload {
        sol: state.sol,
        month: state.month,
        colonies,
        research_total: state.research_pool.total,
    }
}

fn load_embedded_content() -> Result<outpost_core::content::registry::ContentRegistry, CmdError> {
    let mut raw: Vec<(&str, &str)> = Vec::new();
    for file in EMBEDDED_PACK.files() {
        if file.path().extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let name = file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let text = file
            .contents_utf8()
            .ok_or_else(|| CmdError::Content(format!("non-utf8 embedded file: {name}")))?;
        raw.push((name, text));
    }
    PackLoader::load(&raw).map_err(|e| CmdError::Content(e.to_string()))
}

fn load_content(content_dir: &Path) -> Result<outpost_core::content::registry::ContentRegistry, CmdError> {
    let mut raw_owned: Vec<(String, String)> = Vec::new();
    let entries =
        std::fs::read_dir(content_dir).map_err(|e| CmdError::Content(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| CmdError::Content(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned();
            let text = std::fs::read_to_string(&path)
                .map_err(|e| CmdError::Content(e.to_string()))?;
            raw_owned.push((name, text));
        }
    }
    let raw: Vec<(&str, &str)> = raw_owned
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    PackLoader::load(&raw).map_err(|e| CmdError::Content(e.to_string()))
}

fn parse_preset(difficulty: &str) -> DifficultyPreset {
    match difficulty {
        "Sandbox" | "sandbox" => DifficultyPreset::Sandbox,
        "Easy" | "easy" => DifficultyPreset::Easy,
        "Hard" | "hard" => DifficultyPreset::Hard,
        "Brutal" | "brutal" => DifficultyPreset::Brutal,
        _ => DifficultyPreset::Normal,
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Boot a fresh engine: load content, set difficulty, seed the planet map.
///
/// If `content_dir` is empty (or literally `"embedded"`), the pack embedded
/// in the binary at build time is used. Otherwise `content_dir` is read
/// from the filesystem.
#[tauri::command]
pub fn bootstrap(
    content_dir: String,
    planet_seed: u64,
    difficulty: String,
    engine_state: State<'_, EngineState>,
) -> CmdResult<SnapshotPayload> {
    let registry = if content_dir.is_empty() || content_dir == "embedded" {
        load_embedded_content()?
    } else {
        load_content(Path::new(&content_dir))?
    };

    let mut engine = GameEngine::new();
    engine.state.registry = Some(registry);
    engine.state.needs_config = Some(NeedsConfig::default_survival());

    let _ = engine.apply(&Command::SetDifficulty {
        preset: parse_preset(&difficulty),
    });

    let _ = engine.apply(&Command::SeedPlanet {
        seed: planet_seed,
        radius: 8,
    });

    // Seed a placeholder star system. Content-driven system generation is a
    // future refinement; this gives the UI something to render immediately.
    seed_default_system(&mut engine);
    seed_default_tech_tree(&mut engine);

    let snap = build_snapshot(&engine);
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}

fn seed_default_tech_tree(engine: &mut GameEngine) {
    fn tech(id: &str, name: &str, cost: f32, prereqs: &[&str]) -> TechDef {
        TechDef {
            id: id.to_owned(),
            display_name: name.to_owned(),
            prerequisites: prereqs.iter().map(|s| (*s).to_owned()).collect(),
            research_cost: cost,
            effects: Vec::new(),
        }
    }
    let defs = vec![
        tech("basic_construction", "Basic Construction", 100.0, &[]),
        tech("basic_agriculture", "Basic Agriculture", 100.0, &[]),
        tech("basic_extraction", "Basic Extraction", 100.0, &[]),
        tech("improved_construction", "Improved Construction", 200.0, &["basic_construction"]),
        tech("hydroponics", "Hydroponics", 200.0, &["basic_agriculture"]),
        tech("smelting", "Smelting", 200.0, &["basic_extraction"]),
        tech("power_grids", "Power Grids", 300.0, &["improved_construction"]),
        tech("advanced_hydroponics", "Advanced Hydroponics", 300.0, &["hydroponics"]),
        tech("advanced_smelting", "Advanced Smelting", 300.0, &["smelting"]),
        tech("orbital_mechanics", "Orbital Mechanics", 500.0, &["power_grids"]),
        tech("industrial_ecology", "Industrial Ecology", 500.0, &["advanced_hydroponics", "advanced_smelting"]),
        tech("propulsion_theory", "Propulsion Theory", 800.0, &["orbital_mechanics"]),
        tech("interstellar_engineering", "Interstellar Engineering", 1600.0, &["propulsion_theory", "industrial_ecology"]),
    ];
    if let Ok(registry) = TechRegistry::build(defs) {
        engine.state.tech_registry = Some(registry);
    }
}

fn seed_default_system(engine: &mut GameEngine) {
    let bodies: &[(&str, BodyKind, f32, SystemRole)] = &[
        ("Kepler-A", BodyKind::InnerPlanet, 0.4, SystemRole::RawExtraction),
        ("Kepler-B", BodyKind::InnerPlanet, 0.7, SystemRole::PopulationHub),
        ("Kepler-C", BodyKind::InnerPlanet, 1.1, SystemRole::Science),
        ("Ceres Belt", BodyKind::AsteroidBelt, 2.4, SystemRole::RawExtraction),
        ("Aurelian", BodyKind::GasGiant, 4.2, SystemRole::FuelProduction),
        ("Aurelian-Moon", BodyKind::Moon, 4.35, SystemRole::Industry),
        ("Selkin", BodyKind::InnerPlanet, 6.8, SystemRole::Unassigned),
    ];
    for (name, kind, distance, _role) in bodies.iter().cloned() {
        let _ = engine.apply(&Command::System(SystemCommand::AddBody {
            name: name.to_owned(),
            kind,
            distance_au: distance,
        }));
    }
}

/// Return whether an engine has been bootstrapped.
#[tauri::command]
pub fn is_ready(engine_state: State<'_, EngineState>) -> bool {
    engine_state.engine.lock().unwrap().is_some()
}

/// Return a fresh full snapshot of engine state.
#[tauri::command]
pub fn snapshot(engine_state: State<'_, EngineState>) -> CmdResult<SnapshotPayload> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    Ok(build_snapshot(engine))
}

/// Apply a single command (mirrors the WebSocket "command" message).
#[tauri::command]
pub fn apply_command(
    command: ClientCommand,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<ServerEvent>> {
    let mut guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_mut().ok_or(CmdError::NotInitialised)?;

    let core_cmd = match command {
        ClientCommand::AdvanceSol => Command::AdvanceColonySol,
        ClientCommand::FoundColony {
            name,
            starting_population,
        } => Command::FoundColony {
            name,
            starting_population,
        },
        ClientCommand::QueueConstruction {
            colony_id,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        } => Command::QueueConstruction {
            colony_id: parse_colony(&colony_id)?,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        },
        ClientCommand::AssignLabour {
            colony_id,
            slot,
            labour,
        } => Command::AssignLabour {
            colony_id: parse_colony(&colony_id)?,
            slot,
            labour,
        },
        ClientCommand::ResearchTech { tech_id } => Command::ResearchTech { tech_id },
        ClientCommand::EnqueueResearch { tech_id } => Command::EnqueueResearch { tech_id },
        ClientCommand::FoundColonyAtSite {
            name,
            starting_population,
            site_id,
            focus,
            supplies_id,
        } => {
            let uuid = Uuid::parse_str(&site_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad site_id: {site_id}")))?;
            Command::FoundColonyAtSite {
                name,
                starting_population,
                site_id: SiteId(uuid),
                focus,
                supplies_id,
            }
        }
    };

    let events = engine.apply(&core_cmd).map_err(CmdError::from)?;
    Ok(events.iter().map(ServerEvent::from_core).collect())
}

/// Run a read-only query. Returns raw JSON to accommodate heterogeneous result shapes.
#[tauri::command]
pub fn run_query(
    query: ClientQuery,
    engine_state: State<'_, EngineState>,
) -> CmdResult<serde_json::Value> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let core_query = match query {
        ClientQuery::CurrentSol => Query::CurrentSol,
        ClientQuery::CurrentMonth => Query::CurrentMonth,
        ClientQuery::ListColonies => Query::ListColonies,
        ClientQuery::ColonyStatus { colony_id } => Query::ColonyStatus {
            colony_id: parse_colony(&colony_id)?,
        },
        ClientQuery::ColonyScreen { colony_id } => Query::ColonyScreen {
            colony_id: parse_colony(&colony_id)?,
        },
        ClientQuery::SystemResearchTotal => Query::SystemResearchTotal,
    };

    let result = engine.query(&core_query).map_err(CmdError::from)?;
    let value = match result {
        QueryResult::Counter(v) => serde_json::json!({ "kind": "counter", "value": v }),
        QueryResult::Colonies(list) => serde_json::json!({ "kind": "colonies", "colonies": list }),
        QueryResult::ColonyStatus(s) => serde_json::json!({ "kind": "colony_status", "status": s }),
        QueryResult::Labour(l) => serde_json::json!({ "kind": "labour", "labour": l }),
        QueryResult::ResearchTotal(t) => serde_json::json!({ "kind": "research_total", "total": t }),
        QueryResult::ColonyScreen(d) => serde_json::json!({ "kind": "colony_screen", "data": d }),
        other => serde_json::json!({ "kind": "other", "debug": format!("{other:?}") }),
    };
    Ok(value)
}

/// Discard the current engine and return to a pre-bootstrap state.
#[tauri::command]
pub fn reset_engine(engine_state: State<'_, EngineState>) {
    *engine_state.engine.lock().unwrap() = None;
}

/// Persist the current game state to a SQLite file.
#[tauri::command]
pub fn save_game(path: String, engine_state: State<'_, EngineState>) -> CmdResult<()> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let mut db =
        SnapshotDb::open(Path::new(&path)).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    db.save(&engine.state)
        .map_err(|e| CmdError::Snapshot(e.to_string()))
}

/// Load a previously saved game from a SQLite file.
#[tauri::command]
pub fn load_game(
    path: String,
    engine_state: State<'_, EngineState>,
) -> CmdResult<SnapshotPayload> {
    let db =
        SnapshotDb::open(Path::new(&path)).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let game_state = db.load().map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let mut engine = GameEngine::new();
    engine.state = game_state;
    let snap = build_snapshot(&engine);
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}

// ── Custom, high-level queries used by dedicated UI views ────────────────────

/// A body on the system map, positioned for rendering.
#[derive(Debug, Serialize)]
pub struct SystemBodyWire {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub role: String,
    pub distance_au: f32,
    pub colonizable: bool,
}

/// Return the current list of system bodies with rendering hints.
#[tauri::command]
pub fn get_system_bodies(engine_state: State<'_, EngineState>) -> CmdResult<Vec<SystemBodyWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let bodies: Vec<SystemBodyWire> = engine
        .state
        .system_state
        .node_map
        .bodies
        .values()
        .map(|b| SystemBodyWire {
            id: b.id.0.to_string(),
            name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            role: format!("{:?}", b.role),
            distance_au: b.distance_au,
            colonizable: matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            ),
        })
        .collect();
    Ok(bodies)
}

/// Tech definition + player state (researched / in_progress / available / locked).
#[derive(Debug, Serialize)]
pub struct TechNodeWire {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub cost: f32,
    pub prerequisites: Vec<String>,
    pub state: String, // researched | in_progress | available | locked
    pub progress: f32,
}

/// Return the full tech tree with per-node state.
#[tauri::command]
pub fn get_tech_tree(engine_state: State<'_, EngineState>) -> CmdResult<Vec<TechNodeWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let tech_registry = engine
        .state
        .tech_registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no tech registry loaded".into()))?;

    let tech_state = &engine.state.tech_state;
    let current = tech_state.current_project.as_deref();

    let mut nodes = Vec::new();
    for tech in tech_registry.all() {
        let is_done = tech_state.is_researched(&tech.id);
        let is_active = current == Some(tech.id.as_str());
        let is_queued = tech_state.research_queue.iter().any(|t| t == &tech.id);
        let prereqs_met = tech_state.prerequisites_met(tech);

        let state = if is_done {
            "researched"
        } else if is_active || is_queued {
            "in_progress"
        } else if prereqs_met {
            "available"
        } else {
            "locked"
        };
        let progress = if is_active {
            tech_state.progress / tech.research_cost.max(0.001)
        } else {
            0.0
        };
        nodes.push(TechNodeWire {
            id: tech.id.clone(),
            name: tech.display_name.clone(),
            category: String::new(),
            description: String::new(),
            cost: tech.research_cost,
            prerequisites: tech.prerequisites.clone(),
            state: state.to_owned(),
            progress,
        });
    }
    nodes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(nodes)
}

/// Colonizable target for the founding wizard.
#[derive(Debug, Serialize)]
pub struct ColonizeTargetWire {
    pub body_id: String,
    pub body_name: String,
    pub kind: String,
    pub distance_au: f32,
}

#[tauri::command]
pub fn get_colonize_targets(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<ColonizeTargetWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let list = engine
        .state
        .system_state
        .node_map
        .bodies
        .values()
        .filter(|b| {
            matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            )
        })
        .map(|b| ColonizeTargetWire {
            body_id: b.id.0.to_string(),
            body_name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            distance_au: b.distance_au,
        })
        .collect();
    Ok(list)
}

/// A building option in the founding wizard.
///
/// Carries the canonical construction data from `BuildingDef` so the UI
/// never has to invent slot cost / labor / cost / turns.
#[derive(Debug, Serialize)]
pub struct BuildingOptionWire {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub slot_cost: u32,
    pub labor_per_turn: u32,
    pub construction_turns: u32,
    pub construction_cost: Vec<(String, f64)>,
    pub tech_prerequisite: Option<String>,
}

#[tauri::command]
pub fn list_buildings(engine_state: State<'_, EngineState>) -> CmdResult<Vec<BuildingOptionWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let registry = engine
        .state
        .registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no content registry loaded".into()))?;
    let mut out: Vec<BuildingOptionWire> = registry
        .buildings()
        .map(|b| BuildingOptionWire {
            id: b.id.clone(),
            name: b.name.clone(),
            description: b.description.clone(),
            category: format!("{:?}", b.category),
            slot_cost: b.slot_cost,
            labor_per_turn: b.labor_required,
            construction_turns: b.construction_turns,
            construction_cost: b
                .construction_cost
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
            tech_prerequisite: b.tech_prerequisite.clone(),
        })
        .collect();
    out.sort_by(|a, b| a.category.cmp(&b.category).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

/// One hex on the planet surface, enriched with UI-relevant fields.
#[derive(Debug, Serialize)]
pub struct PlanetHexWire {
    pub q: i32,
    pub r: i32,
    pub site_id: String,
    pub terrain: String,
    pub biome: String,
    pub deposits: Vec<DepositWire>,
    pub habitable: bool,
    pub suitability: f32,
    pub occupied_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DepositWire {
    pub commodity_id: String,
    pub richness: f32,
}

#[derive(Debug, Serialize)]
pub struct PlanetMapWire {
    pub seed: u64,
    pub radius: u32,
    pub hexes: Vec<PlanetHexWire>,
}

/// A starter supply package option surfaced in the founding wizard.
#[derive(Debug, Serialize)]
pub struct SupplyPackageWire {
    pub id: String,
    pub name: String,
    pub description: String,
    pub commodities: Vec<(String, f64)>,
}

/// Return all `SupplyPackage` records from the loaded content pack, sorted by name.
#[tauri::command]
pub fn list_supply_packages(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<SupplyPackageWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let registry = engine
        .state
        .registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no content registry loaded".into()))?;
    let mut out: Vec<SupplyPackageWire> = registry
        .supply_packages()
        .map(|p| SupplyPackageWire {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            commodities: p
                .commodities
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Return the current planet hex map with per-cell metadata.
#[tauri::command]
pub fn get_planet_map(engine_state: State<'_, EngineState>) -> CmdResult<PlanetMapWire> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let pm = engine
        .state
        .planet_map
        .as_ref()
        .ok_or_else(|| CmdError::Engine("no planet map — bootstrap first".into()))?;

    // Build reverse coord → site_id lookup so hex rows carry a site_id.
    let coord_to_site: std::collections::HashMap<_, _> = pm
        .sites
        .iter()
        .map(|(sid, coord)| (*coord, *sid))
        .collect();

    // Reverse coord → colony_name lookup for the `occupied_by` field.
    let colony_names: std::collections::HashMap<_, _> = engine
        .state
        .colonies
        .iter()
        .map(|c| (c.id, c.name.clone()))
        .collect();
    let coord_to_colony: std::collections::HashMap<_, _> = pm
        .colonies
        .iter()
        .filter_map(|node| {
            colony_names
                .get(&node.colony_id)
                .map(|name| (node.coord, name.clone()))
        })
        .collect();

    let mut hexes: Vec<PlanetHexWire> = pm
        .cells
        .values()
        .map(|cell| {
            let site_id = coord_to_site
                .get(&cell.coord)
                .map(|sid| sid.0.to_string())
                .unwrap_or_default();
            PlanetHexWire {
                q: cell.coord.q,
                r: cell.coord.r,
                site_id,
                terrain: format!("{:?}", cell.terrain),
                biome: format!("{:?}", cell.biome),
                deposits: cell
                    .deposits
                    .iter()
                    .map(|d| DepositWire {
                        commodity_id: d.commodity_id.clone(),
                        richness: d.richness,
                    })
                    .collect(),
                habitable: cell.is_habitable(),
                suitability: cell.suitability(),
                occupied_by: coord_to_colony.get(&cell.coord).cloned(),
            }
        })
        .collect();

    // Stable ordering so the frontend doesn't churn cell z-order between calls.
    hexes.sort_by_key(|h| (h.r, h.q));

    Ok(PlanetMapWire {
        seed: pm.seed,
        radius: pm.radius,
        hexes,
    })
}

/// List `*.o3save` files in the given directory. Returns filenames only.
#[tauri::command]
pub fn list_saves(dir: String) -> CmdResult<Vec<String>> {
    let path = Path::new(&dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut saves = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|e| CmdError::Snapshot(e.to_string()))? {
        let entry = entry.map_err(|e| CmdError::Snapshot(e.to_string()))?;
        let name = entry.file_name();
        let s = name.to_string_lossy();
        if s.ends_with(".o3save") || s.ends_with(".sqlite") {
            saves.push(s.into_owned());
        }
    }
    saves.sort();
    Ok(saves)
}
