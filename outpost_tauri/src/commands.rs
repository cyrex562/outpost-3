//! Tauri command handlers — one per player action.
//!
//! Each handler locks the engine state, dispatches a `Command`, and returns
//! serialized `Event`s to the frontend. No HTTP round-trip needed.

use outpost_core::content::loader::PackLoader;
use outpost_core::snapshot::SnapshotDb;
use outpost_core::{Command, Event, GameEngine, NeedsConfig};
use serde::Serialize;
use tauri::State;

use crate::state::EngineState;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error, Serialize)]
pub enum CmdError {
    #[error("engine not initialised — call new_game first")]
    NotInitialised,
    #[error("engine error: {0}")]
    Engine(String),
    #[error("content load error: {0}")]
    Content(String),
    #[error("snapshot error: {0}")]
    Snapshot(String),
}

impl From<outpost_core::EngineError> for CmdError {
    fn from(e: outpost_core::EngineError) -> Self {
        Self::Engine(e.to_string())
    }
}

type CmdResult<T> = Result<T, CmdError>;

// ── Snapshot DTO ──────────────────────────────────────────────────────────────

/// Lightweight state snapshot returned to the frontend after every command.
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub sol: u64,
    pub month: u64,
    pub colonies: Vec<ColonySummary>,
    pub events: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ColonySummary {
    pub id: String,
    pub name: String,
    pub population: f32,
    pub stability: f32,
    pub commodity_pool: Vec<(String, f32)>,
}

fn make_snapshot(engine: &GameEngine, events: Vec<Event>) -> Snapshot {
    let state = &engine.state;
    Snapshot {
        sol: state.sol,
        month: state.month,
        colonies: state
            .colonies
            .iter()
            .zip(state.populations.iter())
            .map(|(c, p)| ColonySummary {
                id: c.id.to_string(),
                name: c.name.clone(),
                population: p.count,
                stability: state
                    .stability_trackers
                    .get(&c.id)
                    .and_then(|t| t.samples.back().copied())
                    .unwrap_or(1.0),
                commodity_pool: c
                    .pool
                    .commodity_ids()
                    .map(|id| (id.to_owned(), c.pool.amount(id) as f32))
                    .collect(),
            })
            .collect(),
        events: events.iter().map(|e| format!("{e:?}")).collect(),
    }
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Start a new game: load content pack, set difficulty, seed planet, found colony.
#[tauri::command]
pub fn new_game(
    content_dir: String,
    planet_seed: u64,
    difficulty: String,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Snapshot> {
    use std::path::Path;

    // Read all YAML files from the content directory and pass them to the loader.
    let dir = std::path::Path::new(&content_dir);
    let mut raw_files_owned: Vec<(String, String)> = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| CmdError::Content(e.to_string()))? {
        let entry = entry.map_err(|e| CmdError::Content(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned();
            let text = std::fs::read_to_string(&path)
                .map_err(|e| CmdError::Content(e.to_string()))?;
            raw_files_owned.push((name, text));
        }
    }
    let raw_files: Vec<(&str, &str)> = raw_files_owned
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let registry = PackLoader::load(&raw_files)
        .map_err(|e| CmdError::Content(e.to_string()))?;

    let mut engine = GameEngine::new();

    // Wire content registry and needs config.
    engine.state.registry = Some(registry);
    engine.state.needs_config = Some(NeedsConfig::default_survival());

    // Apply difficulty.
    let _ = engine.apply(&Command::SetDifficulty {
        grade: difficulty.parse().unwrap_or_default(),
    });

    // Seed the planet map.
    let _ = engine.apply(&Command::SeedPlanet {
        seed: planet_seed,
        radius: 8,
    });

    // Found the starting colony at the best landing site.
    let events = engine
        .apply(&Command::FoundColony {
            name: "Outpost Alpha".to_owned(),
            hex: None,
        })
        .map_err(CmdError::from)?;

    let snap = make_snapshot(&engine, events);
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}

/// Advance the simulation by one colony-sol.
#[tauri::command]
pub fn advance_sol(engine_state: State<'_, EngineState>) -> CmdResult<Snapshot> {
    let mut guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_mut().ok_or(CmdError::NotInitialised)?;
    let events = engine
        .apply(&Command::AdvanceColonySol)
        .map_err(CmdError::from)?;
    Ok(make_snapshot(engine, events))
}

/// Found an additional colony.
#[tauri::command]
pub fn found_colony(name: String, engine_state: State<'_, EngineState>) -> CmdResult<Snapshot> {
    let mut guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_mut().ok_or(CmdError::NotInitialised)?;
    let events = engine
        .apply(&Command::FoundColony { name, hex: None })
        .map_err(CmdError::from)?;
    Ok(make_snapshot(engine, events))
}

/// Queue a building for construction in a colony.
#[tauri::command]
pub fn queue_construction(
    colony_id: String,
    building_id: String,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Snapshot> {
    let mut guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_mut().ok_or(CmdError::NotInitialised)?;
    let colony = colony_id
        .parse()
        .map_err(|_| CmdError::Engine("invalid colony id".into()))?;
    let events = engine
        .apply(&Command::QueueConstruction {
            colony_id: colony,
            building_id,
        })
        .map_err(CmdError::from)?;
    Ok(make_snapshot(engine, events))
}

/// Assign labour to a building slot.
#[tauri::command]
pub fn assign_labour(
    colony_id: String,
    building_id: String,
    amount: f32,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Snapshot> {
    let mut guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_mut().ok_or(CmdError::NotInitialised)?;
    let colony = colony_id
        .parse()
        .map_err(|_| CmdError::Engine("invalid colony id".into()))?;
    let events = engine
        .apply(&Command::AssignLabour {
            colony_id: colony,
            building_id,
            amount,
        })
        .map_err(CmdError::from)?;
    Ok(make_snapshot(engine, events))
}

/// Enqueue a tech for research.
#[tauri::command]
pub fn enqueue_research(
    tech_id: String,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Snapshot> {
    let mut guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_mut().ok_or(CmdError::NotInitialised)?;
    let events = engine
        .apply(&Command::EnqueueResearch { tech_id })
        .map_err(CmdError::from)?;
    Ok(make_snapshot(engine, events))
}

/// Return current snapshot without advancing the turn.
#[tauri::command]
pub fn get_snapshot(engine_state: State<'_, EngineState>) -> CmdResult<Snapshot> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    Ok(make_snapshot(engine, vec![]))
}

/// Persist the current game state to a SQLite file.
#[tauri::command]
pub fn save_game(path: String, engine_state: State<'_, EngineState>) -> CmdResult<()> {
    use std::path::Path;
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let mut db = SnapshotDb::open(Path::new(&path))
        .map_err(|e| CmdError::Snapshot(e.to_string()))?;
    db.save(&engine.state)
        .map_err(|e| CmdError::Snapshot(e.to_string()))
}

/// Load a previously saved game from a SQLite file.
#[tauri::command]
pub fn load_game(path: String, engine_state: State<'_, EngineState>) -> CmdResult<Snapshot> {
    use std::path::Path;
    let db = SnapshotDb::open(Path::new(&path))
        .map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let game_state = db
        .load()
        .map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let mut engine = GameEngine::new();
    engine.state = game_state;
    let snap = make_snapshot(&engine, vec![]);
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}
