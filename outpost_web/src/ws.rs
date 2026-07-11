//! WebSocket game-loop handler (`/ws`).
//!
//! On connection the client receives a [`WorldSnapshot`] of current state.
//! Subsequent inbound [`ClientMessage`]s drive the engine or query it.
//! Engine events are broadcast to all connected clients as [`ServerMessage::Event`].

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::sync::broadcast::error::RecvError;

use outpost_core::content::loader::{PackLoader, RawFile};
use outpost_core::content::registry::ContentRegistry;
use outpost_core::difficulty::{default_grade_table, DifficultyGradeTable, DifficultyPreset};
use outpost_core::needs::NeedsConfig;
use outpost_core::{Command, Query};

use crate::state::AppState;
use crate::wsmsg::{
    query_result_message, ClientCommand, ClientMessage, ClientQuery, ServerEvent, ServerMessage,
    WorldSnapshot,
};

/// Upgrade an HTTP GET to the game WebSocket.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    // Broadcast receiver — subscribes before sending the snapshot so no events
    // emitted between snapshot construction and subscription are missed.
    let mut events = state.events.subscribe();

    // Build and send the initial world snapshot.
    let snapshot = build_snapshot(&state);
    if let Ok(json) = serde_json::to_string(&ServerMessage::Snapshot { state: snapshot }) {
        if socket.send(Message::Text(json)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            inbound = socket.recv() => {
                match inbound {
                    Some(Ok(Message::Text(text))) => {
                        handle_client_message(&text, &state, &mut socket).await;
                    }
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    Some(Ok(_)) => {}  // ping/pong/binary: ignore
                }
            }
            broadcast = events.recv() => {
                match broadcast {
                    Ok(event) => {
                        let se = ServerEvent::from_core(&event);
                        if let Ok(json) = serde_json::to_string(&ServerMessage::Event { event: se }) {
                            if socket.send(Message::Text(json)).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => {} // dropped frames under load; continue
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}

/// Parse and dispatch a raw text frame from the client.
async fn handle_client_message(text: &str, state: &AppState, socket: &mut WebSocket) {
    let msg = match serde_json::from_str::<ClientMessage>(text) {
        Ok(m) => m,
        Err(e) => {
            let _ = send_error(socket, &format!("invalid message: {e}")).await;
            return;
        }
    };

    match msg {
        ClientMessage::Command { seq, command } => {
            // NewGame is a multi-step initialisation flow — handle it separately.
            if let ClientCommand::NewGame {
                difficulty,
                planet_seed,
            } = command
            {
                handle_new_game(seq, difficulty, planet_seed, state, socket).await;
                return;
            }

            let core_cmd = client_command_to_core(command, state);
            match core_cmd {
                Ok(cmd) => {
                    let result = {
                        let mut engine = state.engine.lock().expect("engine lock");
                        engine.apply(&cmd)
                    };
                    match result {
                        Ok(events) => {
                            // Fan-out through the broadcast channel so all connected
                            // clients (including this one) receive incremental events.
                            for e in events {
                                let _ = state.events.send(e);
                            }
                            let _ = send_json(socket, &ServerMessage::Ack { seq }).await;
                        }
                        Err(e) => {
                            let _ = send_error(socket, &e.to_string()).await;
                        }
                    }
                }
                Err(e) => {
                    let _ = send_error(socket, &e).await;
                }
            }
        }
        ClientMessage::Query { seq, query } => {
            let core_query = client_query_to_core(query, state);
            match core_query {
                Ok(q) => {
                    let result = {
                        let engine = state.engine.lock().expect("engine lock");
                        engine.query(&q)
                    };
                    match result {
                        Ok(qr) => {
                            let msg = query_result_message(seq, qr);
                            let _ = send_json(socket, &msg).await;
                        }
                        Err(e) => {
                            let _ = send_error(socket, &e.to_string()).await;
                        }
                    }
                }
                Err(e) => {
                    let _ = send_error(socket, &e).await;
                }
            }
        }
    }
}

/// Convert a [`ClientCommand`] into a core [`Command`].
fn client_command_to_core(cmd: ClientCommand, _state: &AppState) -> Result<Command, String> {
    use outpost_core::colony::ColonyId;
    use std::str::FromStr;

    match cmd {
        ClientCommand::AdvanceSol => Ok(Command::AdvanceColonySol),
        ClientCommand::FoundColony {
            name,
            starting_population,
        } => Ok(Command::FoundColony {
            name,
            starting_population,
        }),
        ClientCommand::QueueConstruction {
            colony_id,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            Ok(Command::QueueConstruction {
                colony_id: id,
                building_type,
                slot_cost,
                labor_per_turn,
                construction_cost,
                construction_turns,
            })
        }
        ClientCommand::AssignLabour {
            colony_id,
            slot,
            labour,
        } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            Ok(Command::AssignLabour {
                colony_id: id,
                slot,
                labour,
            })
        }
        // NewGame is handled before this function is called.
        ClientCommand::NewGame { .. } => {
            Err("NewGame must be handled before client_command_to_core".into())
        }
    }
}

/// Convert a [`ClientQuery`] into a core [`Query`].
fn client_query_to_core(query: ClientQuery, _state: &AppState) -> Result<Query, String> {
    use outpost_core::colony::ColonyId;
    use std::str::FromStr;

    match query {
        ClientQuery::CurrentSol => Ok(Query::CurrentSol),
        ClientQuery::CurrentMonth => Ok(Query::CurrentMonth),
        ClientQuery::ListColonies => Ok(Query::ListColonies),
        ClientQuery::ColonyStatus { colony_id } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            Ok(Query::ColonyStatus { colony_id: id })
        }
    }
}

/// Build a [`WorldSnapshot`] from the current engine state.
fn build_snapshot(state: &AppState) -> WorldSnapshot {
    let engine = state.engine.lock().expect("engine lock");
    let sol = engine.sol();
    let month = engine.month();
    let colonies = match engine.query(&outpost_core::Query::ListColonies) {
        Ok(outpost_core::QueryResult::Colonies(c)) => c,
        _ => vec![],
    };
    WorldSnapshot {
        sol,
        month,
        colonies,
    }
}

/// Execute the new-game initialisation sequence and respond with a full snapshot.
///
/// Steps: load content pack → apply difficulty → set needs config → seed planet →
/// found initial colony → emit `NewGameSnapshot`.
async fn handle_new_game(
    seq: u64,
    difficulty: DifficultyPreset,
    planet_seed: u64,
    state: &AppState,
    socket: &mut WebSocket,
) {
    let content_dir = state.config.content_dir.clone();

    // 1. Load core content pack from disk.
    let registry = match load_content_pack_from_dir(&content_dir.join("core")) {
        Ok(r) => r,
        Err(e) => {
            let _ = send_error(socket, &format!("content load failed: {e}")).await;
            return;
        }
    };

    // 2. Load difficulty grade table (fall back to built-in default if file missing).
    let grade_table = load_difficulty_table(&content_dir.join("difficulty.yaml"))
        .unwrap_or_else(|_| default_grade_table());
    let difficulty_scalar = grade_table.build_scalar(difficulty);

    // 3. Apply everything to the engine atomically.
    let init_result: Result<Vec<outpost_core::Event>, String> = (|| {
        let mut engine = state.engine.lock().expect("engine lock");

        // Reset to a clean state so NewGame is idempotent.
        *engine = outpost_core::GameEngine::new();

        // Set content registry.
        engine.state.registry = Some(registry);

        // Apply difficulty preset and scalar.
        engine.state.difficulty_preset = difficulty;
        engine.state.difficulty_scalar = difficulty_scalar;

        // Build and install a default survival NeedsConfig.
        engine.state.needs_config = Some(NeedsConfig::default_survival());

        // Seed the planet map.
        let mut events: Vec<outpost_core::Event> = Vec::new();

        let seed_evs = engine
            .apply(&Command::SeedPlanet {
                seed: planet_seed,
                radius: 4,
            })
            .map_err(|e| format!("SeedPlanet failed: {e}"))?;
        events.extend(seed_evs);

        // Found the initial colony.
        let colony_evs = engine
            .apply(&Command::FoundColony {
                name: "Alpha Base".into(),
                starting_population: 200,
            })
            .map_err(|e| format!("FoundColony failed: {e}"))?;
        events.extend(colony_evs);

        Ok(events)
    })();

    match init_result {
        Ok(events) => {
            for e in events {
                let _ = state.events.send(e);
            }
            let snapshot = build_snapshot(state);
            let msg = ServerMessage::NewGameSnapshot {
                seq,
                state: snapshot,
            };
            let _ = send_json(socket, &msg).await;
        }
        Err(e) => {
            let _ = send_error(socket, &e).await;
        }
    }
}

/// Load a content pack from a directory on disk.
///
/// Reads YAML files expected by [`PackLoader`] and returns a populated [`ContentRegistry`].
fn load_content_pack_from_dir(pack_dir: &std::path::Path) -> Result<ContentRegistry, String> {
    if !pack_dir.is_dir() {
        return Err(format!("pack directory not found: {}", pack_dir.display()));
    }

    let file_names = [
        "pack.yaml",
        "commodities.yaml",
        "buildings.yaml",
        "recipes.yaml",
        "default_directives.yaml",
    ];
    let mut raw_contents: Vec<(String, String)> = Vec::new();

    for name in &file_names {
        let path = pack_dir.join(name);
        if path.exists() {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            raw_contents.push(((*name).to_string(), text));
        }
    }

    let raw_files: Vec<RawFile<'_>> = raw_contents
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();

    PackLoader::load(&raw_files).map_err(|e| format!("content pack error: {e}"))
}

/// Load a difficulty grade table from a YAML file.
///
/// Returns an error if the file is missing or cannot be parsed.
fn load_difficulty_table(path: &std::path::Path) -> Result<DifficultyGradeTable, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read difficulty.yaml: {e}"))?;
    serde_yaml::from_str(&text).map_err(|e| format!("failed to parse difficulty.yaml: {e}"))
}

async fn send_json(socket: &mut WebSocket, msg: &ServerMessage) -> Result<(), axum::Error> {
    match serde_json::to_string(msg) {
        Ok(json) => socket.send(Message::Text(json)).await,
        Err(_) => Ok(()),
    }
}

async fn send_error(socket: &mut WebSocket, message: &str) -> Result<(), axum::Error> {
    send_json(
        socket,
        &ServerMessage::Error {
            message: message.to_owned(),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `load_content_pack_from_dir` succeeds on a non-existent directory by returning an error.
    #[test]
    fn load_content_pack_returns_error_for_missing_dir() {
        let result = load_content_pack_from_dir(std::path::Path::new("/nonexistent/xyz123"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    /// `load_difficulty_table` returns an error for a missing file.
    #[test]
    fn load_difficulty_table_returns_error_for_missing_file() {
        let result = load_difficulty_table(std::path::Path::new("/nonexistent/difficulty.yaml"));
        assert!(result.is_err());
    }

    /// `load_content_pack_from_dir` loads the real `content/core` pack when it exists.
    #[test]
    fn load_core_content_pack_succeeds() {
        // Find content/core relative to the workspace root.
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let core_dir = root.join("content").join("core");
        if !core_dir.is_dir() {
            // Skip if running outside the workspace (CI artefact layout).
            return;
        }
        let result = load_content_pack_from_dir(&core_dir);
        assert!(
            result.is_ok(),
            "core pack failed to load: {:?}",
            result.err()
        );
    }

    /// `load_difficulty_table` parses the real `content/difficulty.yaml`.
    #[test]
    fn load_real_difficulty_table_succeeds() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let path = root.join("content").join("difficulty.yaml");
        if !path.exists() {
            return;
        }
        let result = load_difficulty_table(&path);
        assert!(result.is_ok(), "difficulty.yaml failed: {:?}", result.err());
    }

    /// After a `NewGame` sequence the engine has registry and needs_config loaded.
    #[test]
    fn new_game_sequence_sets_registry_and_needs_config() {
        use outpost_core::difficulty::DifficultyPreset;
        use outpost_core::needs::NeedsConfig;
        use outpost_core::Command;
        use outpost_core::GameEngine;

        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let core_dir = root.join("content").join("core");
        if !core_dir.is_dir() {
            return;
        }

        let registry = load_content_pack_from_dir(&core_dir).expect("core pack load");
        let grade_table = default_grade_table();
        let difficulty = DifficultyPreset::Normal;
        let difficulty_scalar = grade_table.build_scalar(difficulty);

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        engine.state.difficulty_preset = difficulty;
        engine.state.difficulty_scalar = difficulty_scalar;
        engine.state.needs_config = Some(NeedsConfig::default_survival());

        engine
            .apply(&Command::SeedPlanet {
                seed: 99,
                radius: 4,
            })
            .expect("seed planet");
        engine
            .apply(&Command::FoundColony {
                name: "Test Base".into(),
                starting_population: 200,
            })
            .expect("found colony");

        assert!(engine.state.registry.is_some(), "registry should be set");
        assert!(
            engine.state.needs_config.is_some(),
            "needs_config should be set"
        );
        assert_eq!(engine.state.difficulty_preset, DifficultyPreset::Normal);
    }

    /// After NewGame init, advancing a sol produces NeedsResolved events (subsystems active).
    #[test]
    fn advance_sol_after_new_game_produces_needs_resolved() {
        use outpost_core::difficulty::DifficultyPreset;
        use outpost_core::needs::NeedsConfig;
        use outpost_core::Command;
        use outpost_core::Event;
        use outpost_core::GameEngine;

        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let core_dir = root.join("content").join("core");
        if !core_dir.is_dir() {
            return;
        }

        let mut registry = load_content_pack_from_dir(&core_dir).expect("core pack load");
        // Clear default directives to avoid recursive AdvanceColonySol actions.
        registry.default_directives.clear();

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        engine.state.difficulty_preset = DifficultyPreset::Normal;
        engine.state.needs_config = Some(NeedsConfig::default_survival());

        engine
            .apply(&Command::SeedPlanet { seed: 1, radius: 4 })
            .expect("seed");
        engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: 50,
            })
            .expect("found");

        let result = engine.apply(&Command::AdvanceColonySol).expect("advance");

        let has_needs_resolved = result
            .iter()
            .any(|e| matches!(e, Event::NeedsResolved { .. }));
        assert!(
            has_needs_resolved,
            "expected NeedsResolved event after sol advance, got: {result:?}"
        );
    }
}
