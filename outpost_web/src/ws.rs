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
#[allow(clippy::too_many_lines)]
fn client_command_to_core(cmd: ClientCommand, _state: &AppState) -> Result<Command, String> {
    use outpost_core::colony::ColonyId;
    use outpost_core::difficulty::DifficultyPreset;
    use outpost_core::expedition::FieldExpeditionId;
    use outpost_core::map::{HexCoord, InfraType};
    use outpost_core::orbital::OrbitType;
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
        ClientCommand::CancelConstruction {
            colony_id,
            project_id,
        } => {
            let colony_id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            let project_id = uuid::Uuid::from_str(&project_id)
                .map_err(|_| format!("invalid project_id: {project_id}"))?;
            Ok(Command::CancelConstruction {
                colony_id,
                project_id,
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
        ClientCommand::SetDifficulty { grade } => {
            let preset = match grade.to_lowercase().as_str() {
                "sandbox" => DifficultyPreset::Sandbox,
                "easy" => DifficultyPreset::Easy,
                "normal" => DifficultyPreset::Normal,
                "hard" => DifficultyPreset::Hard,
                "brutal" => DifficultyPreset::Brutal,
                _ => return Err(format!("unknown difficulty grade: {grade}")),
            };
            Ok(Command::SetDifficulty { preset })
        }
        ClientCommand::EnqueueResearch { tech_id } => Ok(Command::EnqueueResearch { tech_id }),
        ClientCommand::CancelResearch => Ok(Command::CancelResearch),
        ClientCommand::OpenEmigrationGate {
            from_colony,
            to_colony,
            rate,
        } => {
            let from = ColonyId::from_str(&from_colony)
                .map_err(|_| format!("invalid from_colony: {from_colony}"))?;
            let to = ColonyId::from_str(&to_colony)
                .map_err(|_| format!("invalid to_colony: {to_colony}"))?;
            Ok(Command::OpenEmigrationGate {
                from_colony: from,
                to_colony: to,
                rate,
            })
        }
        ClientCommand::BuildInfrastructure {
            from_colony,
            to_colony,
            infra_type,
        } => {
            let from = ColonyId::from_str(&from_colony)
                .map_err(|_| format!("invalid from_colony: {from_colony}"))?;
            let to = ColonyId::from_str(&to_colony)
                .map_err(|_| format!("invalid to_colony: {to_colony}"))?;
            let it = match infra_type.to_lowercase().as_str() {
                "road" => InfraType::Road,
                "rail" => InfraType::Rail,
                "pipeline" => InfraType::Pipeline,
                _ => return Err(format!("unknown infra_type: {infra_type}")),
            };
            Ok(Command::BuildInfrastructure {
                from_colony: from,
                to_colony: to,
                infra_type: it,
            })
        }
        ClientCommand::BeginOrbitalConstruction {
            blueprint_id,
            colony_id,
            orbit_type,
        } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            let ot = match orbit_type.to_lowercase().as_str() {
                "low" => OrbitType::Low,
                "geostationary" => OrbitType::Geostationary,
                "lagrange" => OrbitType::Lagrange,
                _ => return Err(format!("unknown orbit_type: {orbit_type}")),
            };
            Ok(Command::BeginOrbitalConstruction {
                blueprint_id,
                colony_id: id,
                orbit_type: ot,
            })
        }
        ClientCommand::LaunchFieldExpedition {
            colony_id,
            target_hex_q,
            target_hex_r,
            crew,
            supplies,
            transit_sols,
            is_deep_space,
        } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            Ok(Command::LaunchFieldExpedition {
                colony_id: id,
                target_hex: HexCoord {
                    q: target_hex_q,
                    r: target_hex_r,
                },
                crew_count: crew,
                supplies,
                transit_sols,
                is_deep_space,
            })
        }
        ClientCommand::RecallExpedition { expedition_id } => {
            let uuid = uuid::Uuid::from_str(&expedition_id)
                .map_err(|_| format!("invalid expedition_id: {expedition_id}"))?;
            Ok(Command::RecallExpedition {
                expedition_id: FieldExpeditionId(uuid),
            })
        }
        ClientCommand::ContinueSandbox => Ok(Command::ContinueSandbox),
        ClientCommand::SaveGame => {
            // SaveGame is an infrastructure-layer operation, not a core Command.
            // Return a descriptive error — the web host does not have a configured
            // snapshot backend in this session.
            Err("SaveGame is not supported in WebSocket sessions; use the REST snapshot API".into())
        }
        ClientCommand::LoadGame => {
            // LoadGame is an infrastructure-layer operation.
            Err("LoadGame is not supported in WebSocket sessions; use the REST snapshot API".into())
        }
        ClientCommand::SetDirective {
            colony_id,
            directive_json,
        } => {
            let _colony = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            let directive: outpost_core::directive::Directive =
                serde_json::from_str(&directive_json)
                    .map_err(|e| format!("invalid directive JSON: {e}"))?;
            Ok(Command::SetDirective {
                directive: Box::new(directive),
            })
        }
        ClientCommand::RemoveDirective { directive_id } => {
            let id = outpost_core::directive::DirectiveId::from_str(&directive_id)
                .map_err(|_| format!("invalid directive_id: {directive_id}"))?;
            Ok(Command::RemoveDirective { directive_id: id })
        }
        ClientCommand::SetManualOverride { colony_id, enabled } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            Ok(Command::SetManualOverride {
                colony_id: id,
                enabled,
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

    /// Real-engine proof that the starter (no-tech-prerequisite) building set
    /// sustains a freshly-founded colony at the wizard's default starting
    /// population, using only `Colony::slot_capacity`'s default 5 build
    /// slots (issue #166). Exercises `GameEngine::apply` end-to-end —
    /// production, the power grid, and needs resolution — rather than the
    /// static flow-balance harness, since only the live engine models
    /// housing capacity and stability dynamics.
    #[test]
    fn bootstrap_starter_buildings_sustain_default_starting_population() {
        use outpost_core::colony::PlacedBuilding;
        use outpost_core::needs::NeedsConfig;
        use outpost_core::{Command, Event, GameEngine};

        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let base_dir = root.join("content").join("base");
        let registry = load_content_pack_from_dir(&base_dir).expect("base pack must load");

        let mut engine = GameEngine::with_seed(0);
        engine.state.registry = Some(registry);
        engine.state.needs_config = Some(NeedsConfig::default_survival());

        let events = engine
            .apply(&Command::FoundColony {
                name: "Bootstrap Test".into(),
                starting_population: 100, // matches FoundColonyWizardView's default
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;
        let idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == colony_id)
            .unwrap();

        // Fill all 5 default build slots with the starter buildings that
        // cover the 5 survival needs — bypasses the construction queue's
        // multi-sol build time, matching the established test pattern for
        // isolating production/needs behavior from construction timing.
        for building_type in [
            "water_well",
            "hydroponic_bay",
            "life_support_module",
            "solar_array_mk1",
            "basic_habitat",
        ] {
            engine.state.colonies[idx]
                .buildings
                .push(PlacedBuilding::new(building_type, 1));
        }
        assert_eq!(engine.state.colonies[idx].slots_used(), 5);

        // Advance 20 sols and confirm the colony doesn't spiral: stability
        // stays healthy and population never declines below its starting
        // count — the actual bar for "the starter set bootstraps the
        // colony," not just "doesn't hit exactly zero."
        for _ in 0..20 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
        }

        let final_stability = engine.state.populations[idx].stability;
        let final_population = engine.state.populations[idx].count;
        assert!(
            final_stability > 0.6,
            "starter set should keep stability healthy, got {final_stability}"
        );
        assert!(
            final_population >= 100.0,
            "starter set should not let population decline, got {final_population}"
        );
    }

    /// Real-engine proof that the recipe-selection mechanic (#166) and the
    /// new conductive-metal category (#207) actually work together against
    /// the real content pack: mine conductive ore, switch `refinery` off
    /// its default structural recipe onto `smelt_conductive_ore`, and
    /// confirm conductive_metal — not structural_metal — comes out.
    #[test]
    fn refinery_recipe_selection_produces_conductive_metal_from_real_pack() {
        use outpost_core::colony::PlacedBuilding;
        use outpost_core::{Command, Event, GameEngine};

        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let base_dir = root.join("content").join("base");
        let registry = load_content_pack_from_dir(&base_dir).expect("base pack must load");

        let mut engine = GameEngine::with_seed(0);
        engine.state.registry = Some(registry);

        let events = engine
            .apply(&Command::FoundColony {
                name: "Refinery Test".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;
        let idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == colony_id)
            .unwrap();

        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("conductive_mine", 1));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("refinery", 2));
        // Power source — conductive_mine + refinery together demand 18kW,
        // and neither is exempt from brownout throttling (only Power-
        // category buildings are).
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));
        // Seed structural_ore so the default recipe (refine_ore_to_plate)
        // has something to consume on the first sol.
        engine.state.colonies[idx]
            .pool
            .deposit("structural_ore", 100.0);

        // Default (no active_recipes entry) should be the structural recipe.
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("structural_metal") > 0.0,
            "refinery should default to refine_ore_to_plate with no active_recipes entry"
        );
        assert_eq!(
            engine.state.colonies[idx].pool.amount("conductive_metal"),
            0.0
        );

        // Switch to the conductive recipe.
        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "refinery".into(),
                recipe_id: "smelt_conductive_ore".into(),
            })
            .unwrap();

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("conductive_metal") > 0.0,
            "refinery should produce conductive_metal after switching its active recipe"
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

    /// `ClientCommand::CancelConstruction` translates to the matching core
    /// `Command` variant with parsed UUIDs (issue #169).
    #[test]
    fn client_command_cancel_construction_translates_to_core_command() {
        use crate::config::RuntimeConfig;
        use crate::state::new_state;
        use outpost_core::colony::ColonyId;
        use outpost_core::Command;
        use uuid::Uuid;

        let colony_id = ColonyId::new_v4();
        let project_id = Uuid::new_v4();
        let state = new_state(RuntimeConfig::default());

        let core_cmd = client_command_to_core(
            ClientCommand::CancelConstruction {
                colony_id: colony_id.to_string(),
                project_id: project_id.to_string(),
            },
            &state,
        )
        .expect("translation should succeed");

        match core_cmd {
            Command::CancelConstruction {
                colony_id: got_colony,
                project_id: got_project,
            } => {
                assert_eq!(got_colony, colony_id);
                assert_eq!(got_project, project_id);
            }
            other => panic!("expected Command::CancelConstruction, got {other:?}"),
        }
    }

    /// Malformed UUIDs are rejected with a descriptive error rather than panicking.
    #[test]
    fn client_command_cancel_construction_rejects_invalid_ids() {
        use crate::config::RuntimeConfig;
        use crate::state::new_state;

        let state = new_state(RuntimeConfig::default());
        let result = client_command_to_core(
            ClientCommand::CancelConstruction {
                colony_id: "not-a-uuid".into(),
                project_id: "also-not-a-uuid".into(),
            },
            &state,
        );
        assert!(result.is_err());
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
