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
use outpost_core::modifier::ModifiableQuantity;
use outpost_core::needs::NeedsConfig;
use outpost_core::system::SystemCommand;
use outpost_core::tech::load_tech_registry;
use outpost_core::{Command, GameEngine, Query};

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
                system_seed,
                habitable_zone_center_au,
                min_inner_planets,
                max_inner_planets,
                abundance_scalar,
            } = command
            {
                let system_seed = system_seed.unwrap_or(planet_seed);
                let gen_defaults = outpost_core::system_gen::SystemGenParams::default();
                let gen_overrides = GenerationOverrides {
                    habitable_zone_center_au: habitable_zone_center_au
                        .unwrap_or(gen_defaults.habitable_zone_center_au),
                    min_inner_planets: min_inner_planets.unwrap_or(gen_defaults.min_inner_planets),
                    max_inner_planets: max_inner_planets.unwrap_or(gen_defaults.max_inner_planets),
                    abundance_scalar,
                };
                handle_new_game(
                    seq,
                    difficulty,
                    planet_seed,
                    system_seed,
                    gen_overrides,
                    state,
                    socket,
                )
                .await;
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
pub(crate) fn client_command_to_core(
    cmd: ClientCommand,
    _state: &AppState,
) -> Result<Command, String> {
    use outpost_core::colony::ColonyId;
    use outpost_core::difficulty::DifficultyPreset;
    use outpost_core::expedition::FieldExpeditionId;
    use outpost_core::map::{HexCoord, InfraType};
    use outpost_core::orbital::OrbitType;
    use std::str::FromStr;

    match cmd {
        ClientCommand::AdvanceSol => Ok(Command::AdvanceColonySol),
        ClientCommand::FastForward {
            max_sols,
            threshold,
        } => {
            use outpost_core::interrupt::Tier;
            let threshold = match threshold.as_str() {
                "ambient" => Tier::Ambient,
                "notable" => Tier::Notable,
                "urgent" => Tier::Urgent,
                "blocking" => Tier::Blocking,
                other => {
                    return Err(format!(
                        "unknown interrupt tier {other:?}; expected one of \
                         ambient, notable, urgent, blocking"
                    ))
                }
            };
            Ok(Command::FastForward {
                max_sols,
                threshold,
            })
        }
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
        ClientCommand::DeployStarterKit {
            colony_id,
            buildings,
        } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            Ok(Command::DeployStarterKit {
                colony_id: id,
                buildings,
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
        ClientCommand::SetActiveRecipe {
            colony_id,
            building_type,
            recipe_id,
        } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            Ok(Command::SetActiveRecipe {
                colony_id: id,
                building_type,
                recipe_id,
            })
        }
        ClientCommand::SetCommodityReserve {
            colony_id,
            commodity_id,
            amount,
        } => Ok(Command::SetCommodityReserve {
            colony_id: parse_colony_id(&colony_id)?,
            commodity_id,
            amount,
        }),
        ClientCommand::SetBuildingPriority {
            colony_id,
            building_id,
            priority,
        } => Ok(Command::SetBuildingPriority {
            colony_id: parse_colony_id(&colony_id)?,
            building_id: parse_building_id(&building_id)?,
            priority,
        }),
        ClientCommand::SetBuildingLabourLock {
            colony_id,
            building_id,
            lock,
        } => Ok(Command::SetBuildingLabourLock {
            colony_id: parse_colony_id(&colony_id)?,
            building_id: parse_building_id(&building_id)?,
            lock,
        }),
        ClientCommand::RenameBuilding {
            colony_id,
            building_id,
            name,
        } => Ok(Command::RenameBuilding {
            colony_id: parse_colony_id(&colony_id)?,
            building_id: parse_building_id(&building_id)?,
            name,
        }),
        ClientCommand::SetBuildingPaused {
            colony_id,
            building_id,
            paused,
        } => Ok(Command::SetBuildingPaused {
            colony_id: parse_colony_id(&colony_id)?,
            building_id: parse_building_id(&building_id)?,
            paused,
        }),
        ClientCommand::FoundColonyAtSite {
            name,
            starting_population,
            site_id,
            focus,
            supplies_id,
            supply_overrides,
            sponsor_colony_id,
            body_id,
        } => {
            let site_id = uuid::Uuid::from_str(&site_id)
                .map(outpost_core::trade::SiteId)
                .map_err(|_| format!("invalid site_id: {site_id}"))?;
            let body_id = body_id
                .map(|b| {
                    uuid::Uuid::from_str(&b)
                        .map(outpost_core::system::BodyId)
                        .map_err(|_| format!("invalid body_id: {b}"))
                })
                .transpose()?;
            let sponsor_colony_id = sponsor_colony_id
                .map(|c| {
                    ColonyId::from_str(&c).map_err(|_| format!("invalid sponsor_colony_id: {c}"))
                })
                .transpose()?;
            Ok(Command::FoundColonyAtSite {
                name,
                starting_population,
                site_id,
                focus,
                supplies_id,
                supply_overrides,
                sponsor_colony_id,
                body_id,
            })
        }
        ClientCommand::AssignColonyHomeBody { colony_id, body_id } => {
            let colony_id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            let body_id = uuid::Uuid::from_str(&body_id)
                .map(outpost_core::system::BodyId)
                .map_err(|_| format!("invalid body_id: {body_id}"))?;
            Ok(Command::AssignColonyHomeBody { colony_id, body_id })
        }
        ClientCommand::SetBalanceScalar { quantity, value } => {
            Ok(Command::SetBalanceScalar { quantity, value })
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
        ClientCommand::ResearchTech { tech_id } => Ok(Command::ResearchTech { tech_id }),
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
        ClientCommand::DemolishInfrastructure {
            from_colony,
            to_colony,
        } => {
            let from = ColonyId::from_str(&from_colony)
                .map_err(|_| format!("invalid from_colony: {from_colony}"))?;
            let to = ColonyId::from_str(&to_colony)
                .map_err(|_| format!("invalid to_colony: {to_colony}"))?;
            Ok(Command::DemolishInfrastructure {
                from_colony: from,
                to_colony: to,
            })
        }
        ClientCommand::BeginOrbitalConstruction {
            blueprint_id,
            colony_id,
            orbit_type,
            body_id,
        } => {
            let id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            let ot = match orbit_type.to_lowercase().as_str() {
                "low" => OrbitType::Low,
                "geostationary" => OrbitType::Geostationary,
                "lagrange" => OrbitType::Lagrange,
                _ => return Err(format!("unknown orbit_type: {orbit_type}")),
            };
            let body = body_id
                .map(|b| {
                    uuid::Uuid::parse_str(&b)
                        .map(outpost_core::system::BodyId)
                        .map_err(|_| format!("invalid body_id: {b}"))
                })
                .transpose()?;
            Ok(Command::BeginOrbitalConstruction {
                blueprint_id,
                colony_id: id,
                orbit_type: ot,
                body_id: body,
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
        ClientCommand::EstablishOutpost {
            name,
            colony_id,
            body_id,
        } => {
            let colony_id = ColonyId::from_str(&colony_id)
                .map_err(|_| format!("invalid colony_id: {colony_id}"))?;
            let body_id = uuid::Uuid::from_str(&body_id)
                .map(outpost_core::system::BodyId)
                .map_err(|_| format!("invalid body_id: {body_id}"))?;
            Ok(Command::EstablishOutpost {
                name,
                colony_id,
                body_id,
            })
        }
        ClientCommand::DecommissionOutpost { outpost_id } => {
            let outpost_id = uuid::Uuid::from_str(&outpost_id)
                .map_err(|_| format!("invalid outpost_id: {outpost_id}"))?;
            Ok(Command::DecommissionOutpost { outpost_id })
        }
        ClientCommand::QueueOutpostConstruction {
            outpost_id,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        } => {
            let outpost_id = uuid::Uuid::from_str(&outpost_id)
                .map_err(|_| format!("invalid outpost_id: {outpost_id}"))?;
            Ok(Command::QueueOutpostConstruction {
                outpost_id,
                building_type,
                slot_cost,
                labor_per_turn,
                construction_cost,
                construction_turns,
            })
        }
        ClientCommand::PromoteOutpostToColony {
            outpost_id,
            name,
            starting_population,
        } => {
            let outpost_id = uuid::Uuid::from_str(&outpost_id)
                .map_err(|_| format!("invalid outpost_id: {outpost_id}"))?;
            Ok(Command::PromoteOutpostToColony {
                outpost_id,
                name,
                starting_population,
            })
        }
    }
}

/// Convert a [`ClientQuery`] into a core [`Query`].
/// Parse a colony UUID from the wire, or describe why it failed.
fn parse_colony_id(id: &str) -> Result<outpost_core::colony::ColonyId, String> {
    use std::str::FromStr;
    outpost_core::colony::ColonyId::from_str(id).map_err(|_| format!("invalid colony_id: {id}"))
}

/// Parse a placed-building instance UUID from the wire (issue #307).
fn parse_building_id(id: &str) -> Result<uuid::Uuid, String> {
    use std::str::FromStr;
    uuid::Uuid::from_str(id).map_err(|_| format!("invalid building_id: {id}"))
}

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

/// Player-tunable star-system generation overrides, resolved from
/// `ClientCommand::NewGame`'s optional fields against
/// [`outpost_core::system_gen::SystemGenParams::default`] (playtest
/// feedback: New Game sliders).
struct GenerationOverrides {
    habitable_zone_center_au: f32,
    min_inner_planets: u32,
    max_inner_planets: u32,
    /// `None` means "resolve from the difficulty scalar as before"; `Some`
    /// overrides that resolution entirely.
    abundance_scalar: Option<f32>,
}

/// Execute the new-game initialisation sequence and respond with a full snapshot.
///
/// Steps: load content pack → apply difficulty → set needs config → generate
/// the star system → seed planet → found initial colony → emit `NewGameSnapshot`.
async fn handle_new_game(
    seq: u64,
    difficulty: DifficultyPreset,
    planet_seed: u64,
    system_seed: u64,
    gen_overrides: GenerationOverrides,
    state: &AppState,
    socket: &mut WebSocket,
) {
    let content_dir = state.config.content_dir.clone();

    // 1. Load the base content pack from disk — the full game pack (buildings,
    // recipes, supply packages, star systems), not the legacy `core` pack
    // (commodities/recipes only, no buildings/systems/supplies).
    let registry = match load_content_pack_from_dir(&content_dir.join("base")) {
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

        // Load the tech DAG (issue #250) — mirrors
        // `outpost_tauri::commands::load_embedded_tech`, reading from disk
        // instead of the embedded pack. Non-fatal when missing/unparsable so
        // a base pack without a tech tree still boots.
        if let Ok(tech_yaml) = std::fs::read_to_string(content_dir.join("base/tech.yaml")) {
            if let Ok(tech_registry) = load_tech_registry(&tech_yaml) {
                engine.state.tech_registry = Some(tech_registry);
            }
        }

        // Apply difficulty preset and scalar.
        engine.state.difficulty_preset = difficulty;
        engine.state.difficulty_scalar = difficulty_scalar;

        // Build and install a default survival NeedsConfig.
        engine.state.needs_config = Some(NeedsConfig::default_survival());

        let mut events: Vec<outpost_core::Event> = Vec::new();

        // Procedurally generate the star system (issue #199) — the bootstrap
        // default, replacing the content pack's authored systems.yaml
        // scenarios (still available separately via `seed_system_from_content`
        // for a future hand-authored-scenario picker). Independent seed from
        // the planet map so the player can reroll one without the other.
        let abundance_scalar = gen_overrides.abundance_scalar.unwrap_or_else(|| {
            engine
                .state
                .difficulty_scalar
                .scalar_for(&ModifiableQuantity::DepositAbundance)
        });
        // Gas giant/asteroid-belt/cometary-belt/moon counts (issue #318)
        // have no browser-mode New Game sliders yet — always resolve to
        // `SystemGenParams::default()` here, matching the Tauri-only slider
        // scope decision on the issue.
        let gen_defaults = outpost_core::system_gen::SystemGenParams::default();
        let system_evs = engine
            .apply(&Command::System(SystemCommand::GenerateSystem {
                seed: system_seed,
                abundance_scalar,
                habitable_zone_center_au: gen_overrides.habitable_zone_center_au,
                min_inner_planets: gen_overrides.min_inner_planets,
                max_inner_planets: gen_overrides.max_inner_planets,
                min_gas_giants: gen_defaults.min_gas_giants,
                max_gas_giants: gen_defaults.max_gas_giants,
                min_asteroid_belts: gen_defaults.min_asteroid_belts,
                max_asteroid_belts: gen_defaults.max_asteroid_belts,
                min_cometary_belts: gen_defaults.min_cometary_belts,
                max_cometary_belts: gen_defaults.max_cometary_belts,
                min_giant_moons: gen_defaults.min_giant_moons,
                max_giant_moons: gen_defaults.max_giant_moons,
                max_rocky_moons: gen_defaults.max_rocky_moons,
            }))
            .map_err(|e| format!("GenerateSystem failed: {e}"))?;
        events.extend(system_evs);

        // Seed the planet map. `width`/`height` here are only the no-system
        // fallback — `GenerateSystem` just ran above, so the handler derives
        // the real dimensions from the designated home body's rolled size
        // instead (issue #314).
        let seed_evs = engine
            .apply(&Command::SeedPlanet {
                seed: planet_seed,
                width: 10,
                height: 6,
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
        // Colony-local resources — a separate table since issue #304.
        "resources.yaml",
        "buildings.yaml",
        "recipes.yaml",
        "default_directives.yaml",
        "supplies.yaml",
        "systems.yaml",
        "anomalies.yaml",
        "expedition_failures.yaml",
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

/// Populate `engine.state.system_state` from the content pack's first
/// authored star system, if any. No-op when the pack ships no `systems.yaml`.
///
/// Mirrors `outpost_tauri::commands::seed_system_from_content`.
///
/// Not called from the live `handle_new_game` bootstrap path as of issue
/// #199 — procedural generation (`Command::System(SystemCommand::GenerateSystem)`)
/// is the default there now. Kept (not deleted) because #199 explicitly
/// demotes `content/base/systems.yaml`'s authored scenarios to "optional,
/// selectable later" rather than removing them — this is the loader a
/// future scenario-picker command would call. Exercised directly by
/// `seed_system_from_content_populates_bodies_from_real_pack` below.
#[allow(dead_code)]
fn seed_system_from_content(engine: &mut GameEngine) {
    let Some(system) = engine
        .state
        .registry
        .as_ref()
        .and_then(|r| r.star_systems().next().cloned())
    else {
        return;
    };

    // Pass 1: add every body and its authored attributes/role. `parent_body`
    // is deferred to pass 2 (below) since it may name a body that hasn't
    // been added yet — content validation only guarantees the name resolves
    // *somewhere* in this system, not that it comes earlier.
    let mut pending_parents: Vec<(outpost_core::system::BodyId, String)> = Vec::new();
    for body in &system.bodies {
        let _ = engine.apply(&Command::System(SystemCommand::AddBody {
            name: body.name.clone(),
            kind: body.kind.clone(),
            distance_au: body.distance_au,
        }));
        // `AddBody` doesn't carry the resulting body id back; look it up by name.
        let body_id = engine
            .state
            .system_state
            .node_map
            .bodies
            .iter()
            .find(|(_, b)| b.name == body.name)
            .map(|(id, _)| id.clone());
        let Some(id) = body_id else { continue };
        let _ = engine.apply(&Command::System(SystemCommand::SetBodyAttributes {
            body_id: id.clone(),
            atmosphere_density: body.atmosphere_density,
            atmosphere_hazard: body.atmosphere_hazard,
            temperature: body.temperature,
            gravity_g: body.gravity_g,
            radiation: body.radiation,
            subtype: body.subtype,
            tidally_locked: body.tidally_locked,
            axial_tilt_deg: body.axial_tilt_deg,
            rotation_period_hours: body.rotation_period_hours,
            moon_count: body.moon_count,
        }));
        if !matches!(body.role, outpost_core::system::SystemRole::Unassigned) {
            let _ = engine.apply(&Command::System(SystemCommand::AssignRole {
                body_id: id.clone(),
                role: body.role.clone(),
            }));
        }
        if !body.modifiers.is_empty() {
            let _ = engine.apply(&Command::System(SystemCommand::SetBodyModifiers {
                body_id: id.clone(),
                modifiers: body.modifiers.clone(),
            }));
        }
        if let Some(parent_name) = &body.parent_body {
            pending_parents.push((id, parent_name.clone()));
        }
    }

    // Pass 2: resolve authored parent-body names now that every body in the
    // system has a live BodyId.
    for (body_id, parent_name) in pending_parents {
        let parent_id = engine
            .state
            .system_state
            .node_map
            .bodies
            .iter()
            .find(|(_, b)| b.name == parent_name)
            .map(|(id, _)| id.clone());
        let Some(parent_body) = parent_id else {
            continue;
        };
        let _ = engine.apply(&Command::System(SystemCommand::SetBodyParent {
            body_id,
            parent_body,
        }));
    }
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

    /// `load_content_pack_from_dir` also picks up `supplies.yaml` and
    /// `systems.yaml` from the real `content/base` pack (issue #220 — the
    /// browser-mode `new_game` bootstrap needs both for the founding wizard).
    #[test]
    fn load_base_content_pack_includes_supplies_and_systems() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let base_dir = root.join("content").join("base");
        if !base_dir.is_dir() {
            return;
        }
        let registry = load_content_pack_from_dir(&base_dir).expect("base pack must load");
        assert!(
            registry.supply_packages().count() > 0,
            "expected at least one supply package from supplies.yaml"
        );
        assert!(
            registry.star_systems().count() > 0,
            "expected at least one star system from systems.yaml"
        );
        assert!(
            registry.anomalies().count() > 0,
            "expected at least one anomaly from anomalies.yaml (issue #235)"
        );
    }

    /// The shipped base pack must lint clean (issue #272), or the warning
    /// channel is noise nobody reads.
    ///
    /// Lives here rather than in `outpost_core` because the lint runs over a
    /// loaded registry and core cannot touch the filesystem (CLAUDE.md rule 1).
    #[test]
    fn the_base_content_pack_lints_clean() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let base_dir = root.join("content").join("base");
        if !base_dir.is_dir() {
            return;
        }
        let registry = load_content_pack_from_dir(&base_dir).expect("base pack must load");
        let warnings = registry.lint();
        assert!(
            warnings.is_empty(),
            "content/base should lint clean; found:\n{}",
            warnings
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// `handle_new_game`'s tech-loading step (issue #250) reads
    /// `content/base/tech.yaml` from disk the same way it's authored, so the
    /// browser-mode tech tree isn't permanently empty like it was before
    /// this fix — confirms the file path and `load_tech_registry` parsing
    /// both work against the real pack, not just a fixture.
    #[test]
    fn base_tech_yaml_loads_into_a_non_empty_registry() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let tech_path = root.join("content").join("base").join("tech.yaml");
        if !tech_path.is_file() {
            return;
        }
        let yaml = std::fs::read_to_string(&tech_path).expect("read tech.yaml");
        let registry = load_tech_registry(&yaml).expect("tech.yaml must parse");
        assert!(
            registry.all().count() > 0,
            "expected at least one tech definition"
        );
    }

    /// `seed_system_from_content` populates `system_state.node_map.bodies`
    /// from the real `content/base` pack's `systems.yaml`, so the founding
    /// wizard's colonize-targets list isn't empty after `NewGame` (issue #220).
    #[test]
    fn seed_system_from_content_populates_bodies_from_real_pack() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let base_dir = root.join("content").join("base");
        if !base_dir.is_dir() {
            return;
        }
        let registry = load_content_pack_from_dir(&base_dir).expect("base pack must load");

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        seed_system_from_content(&mut engine);

        assert!(
            !engine.state.system_state.node_map.bodies.is_empty(),
            "expected at least one system body after seeding"
        );
    }

    /// Real-engine proof that `handle_new_game`'s actual bootstrap sequence
    /// — `GenerateSystem` → `SeedPlanet` → `FoundColony` — produces a system
    /// with at least one founding-viable body (issue #199), across a small
    /// seed sweep rather than just one lucky seed.
    #[test]
    fn generate_system_bootstrap_sequence_produces_a_founding_viable_body() {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let base_dir = root.join("content").join("base");
        if !base_dir.is_dir() {
            return;
        }

        for seed in 0..8u64 {
            let registry = load_content_pack_from_dir(&base_dir).expect("base pack must load");
            let mut engine = GameEngine::new();
            engine.state.registry = Some(registry);

            let gen_defaults = outpost_core::system_gen::SystemGenParams::default();
            engine
                .apply(&Command::System(SystemCommand::GenerateSystem {
                    seed,
                    abundance_scalar: 1.0,
                    habitable_zone_center_au: gen_defaults.habitable_zone_center_au,
                    min_inner_planets: gen_defaults.min_inner_planets,
                    max_inner_planets: gen_defaults.max_inner_planets,
                    min_gas_giants: gen_defaults.min_gas_giants,
                    max_gas_giants: gen_defaults.max_gas_giants,
                    min_asteroid_belts: gen_defaults.min_asteroid_belts,
                    max_asteroid_belts: gen_defaults.max_asteroid_belts,
                    min_cometary_belts: gen_defaults.min_cometary_belts,
                    max_cometary_belts: gen_defaults.max_cometary_belts,
                    min_giant_moons: gen_defaults.min_giant_moons,
                    max_giant_moons: gen_defaults.max_giant_moons,
                    max_rocky_moons: gen_defaults.max_rocky_moons,
                }))
                .expect("generate system");
            engine
                .apply(&Command::SeedPlanet {
                    seed,
                    width: 10,
                    height: 6,
                })
                .expect("seed planet");
            engine
                .apply(&Command::FoundColony {
                    name: "Alpha Base".into(),
                    starting_population: 200,
                })
                .expect("found colony");

            assert!(
                !engine.state.system_state.node_map.bodies.is_empty(),
                "seed {seed}: expected at least one system body"
            );
            assert!(
                engine
                    .state
                    .system_state
                    .node_map
                    .bodies
                    .values()
                    .any(outpost_core::system::Body::meets_founding_threshold),
                "seed {seed}: expected at least one founding-viable body"
            );
        }
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

        // Fill build slots with the landing-kit starter buildings that
        // cover the 5 survival needs (colony_hq: power+water+oxygen;
        // greenhouse_dome: food; habitat_pod: housing) — bypasses the
        // construction queue's multi-sol build time, matching the
        // established test pattern for isolating production/needs behavior
        // from construction timing.
        for building_type in ["colony_hq", "greenhouse_dome", "habitat_pod"] {
            engine.state.colonies[idx]
                .buildings
                .push(PlacedBuilding::new(building_type, 1));
        }
        assert_eq!(engine.state.colonies[idx].slots_used(), 3);

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

    /// Real-engine proof that water's crating round trip (issue #380) works
    /// against the real content pack: a `water_bottling_plant` bottles
    /// banked water into shippable `water_container` cargo, then — switched
    /// to the other side of the same pick-one pair — unpacks it straight
    /// back into usable water.
    #[test]
    fn water_bottling_plant_crates_and_uncrates_water_from_real_pack() {
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
                name: "Bottling Test".into(),
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
            .push(PlacedBuilding::new("water_tank", 0));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("water_bottling_plant", 1));
        // Power source — water_bottling_plant draws 3kW and isn't exempt
        // from brownout throttling (only Power-category buildings are).
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));
        // Seed banked water directly so bottle_water has something to
        // consume on the first sol, independent of any production chain.
        engine.state.colonies[idx].resources.deposit("water", 200.0);

        // Default (no active_recipes entry) should be bottle_water: with no
        // active_recipes entry, the pick-one default is the lexicographically
        // smallest recipe id for the building (production.rs), and
        // "bottle_water" < "unbottle_water".
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("water_container") > 0.0,
            "water_bottling_plant should default to bottle_water and produce water_container"
        );

        // Switch to unbottle_water and confirm the round trip: crated cargo
        // converts back into usable, banked water.
        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "water_bottling_plant".into(),
                recipe_id: "unbottle_water".into(),
            })
            .unwrap();
        let water_container_before = engine.state.colonies[idx].pool.amount("water_container");
        let water_before = engine.state.colonies[idx].resources.amount("water");

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("water_container") < water_container_before,
            "unbottle_water should consume water_container cargo"
        );
        assert!(
            engine.state.colonies[idx].resources.amount("water") > water_before,
            "unbottle_water should deposit usable water back into the resource pool"
        );
    }

    /// Real-engine proof that refining recipes now emit `waste` as a
    /// byproduct alongside their main output (issue #386).
    #[test]
    fn refinery_recipes_emit_waste_byproduct_from_real_pack() {
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
                name: "Waste Byproduct Test".into(),
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
            .push(PlacedBuilding::new("refinery", 2));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));
        engine.state.colonies[idx]
            .pool
            .deposit("structural_ore", 100.0);

        // Default recipe is refine_ore_to_plate; it should now also emit waste.
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("structural_metal") > 0.0,
            "refine_ore_to_plate should still produce structural_metal"
        );
        assert!(
            engine.state.colonies[idx].resources.amount("waste") > 0.0,
            "refine_ore_to_plate should emit waste as a byproduct (issue #386)"
        );

        // Switch to the conductive recipe and confirm it also emits waste.
        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "refinery".into(),
                recipe_id: "smelt_conductive_ore".into(),
            })
            .unwrap();
        engine.state.colonies[idx]
            .pool
            .deposit("conductive_ore", 100.0);
        let waste_before = engine.state.colonies[idx].resources.amount("waste");

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("conductive_metal") > 0.0,
            "smelt_conductive_ore should still produce conductive_metal"
        );
        assert!(
            engine.state.colonies[idx].resources.amount("waste") >= waste_before,
            "smelt_conductive_ore should also emit waste as a byproduct (issue #386)"
        );
    }

    /// Real-engine proof that a `waste_bunker` banks waste across sols up to
    /// its built capacity, while unbanked waste still evaporates (issue #386,
    /// mirroring the #348 `battery_bank` storage-carryover proof).
    #[test]
    fn waste_bunker_carries_waste_across_sols_up_to_its_capacity() {
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
                name: "Waste Bunker Test".into(),
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
            .push(PlacedBuilding::new("waste_bunker", 0));
        // Seed a large amount of waste directly — well beyond the bunker's
        // 40kg capacity — so this proof is independent of any recipe chain.
        engine.state.colonies[idx].resources.deposit("waste", 500.0);

        engine.apply(&Command::AdvanceColonySol).unwrap();
        let waste_after = engine.state.colonies[idx].resources.amount("waste");
        assert!(
            (waste_after - 40.0).abs() < 1e-6,
            "waste_bunker should bank waste up to its 40kg capacity, got {waste_after}"
        );

        // A second sol with no further waste production should hold steady
        // at capacity rather than draining or growing further.
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            (engine.state.colonies[idx].resources.amount("waste") - 40.0).abs() < 1e-6,
            "banked waste should carry over unchanged with no new production"
        );
    }

    /// Real-engine proof that unbanked waste evaporates every sol exactly
    /// like unbanked power (issue #386) — with no storage building present,
    /// a seeded amount does not survive `bank_and_clear`.
    #[test]
    fn waste_evaporates_every_sol_without_a_storage_building() {
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
                name: "Waste Evaporation Test".into(),
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

        // No waste_bunker placed — this colony has zero waste storage capacity.
        engine.state.colonies[idx].resources.deposit("waste", 500.0);

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(
            engine.state.colonies[idx].resources.amount("waste"),
            0.0,
            "waste should evaporate every sol with no storage building present"
        );
    }

    /// Real-engine proof that `hex_remediation` is tech-gated behind
    /// `bioremediation` and, once researched, a completed project lowers the
    /// colony's own hex's contamination (issue #388).
    #[test]
    fn hex_remediation_requires_bioremediation_and_lowers_contamination_from_real_pack() {
        use outpost_core::{Command, EngineError, Event, GameEngine};

        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let base_dir = root.join("content").join("base");
        let registry = load_content_pack_from_dir(&base_dir).expect("base pack must load");

        let mut engine = GameEngine::with_seed(0);
        engine.state.registry = Some(registry);
        engine
            .apply(&Command::SeedPlanet {
                seed: 7,
                width: 3,
                height: 3,
            })
            .unwrap();
        let pm = engine.state.home_map().unwrap();
        let coord = pm.best_landing_site().unwrap();
        let site_id = *pm
            .sites
            .iter()
            .find(|(_, &c)| c == coord)
            .map(|(id, _)| id)
            .unwrap();
        drop(pm);

        let events = engine
            .apply(&Command::FoundColonyAtSite {
                name: "Fouled Real Pack".into(),
                starting_population: 100,
                site_id,
                focus: None,
                supplies_id: None,
                supply_overrides: None,
                sponsor_colony_id: None,
                body_id: None,
            })
            .unwrap();
        let Event::ColonyFoundedAtSite { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFoundedAtSite, got {events:?}")
        };
        let colony_id = *colony_id;
        let idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == colony_id)
            .unwrap();
        engine.state.colonies[idx]
            .pool
            .deposit("structural_metal", 100.0);

        engine.state.home_map_mut().unwrap().contaminate(coord, 0.5);

        // Before `bioremediation` is researched, queuing hex_remediation is refused.
        let refused = engine.apply(&Command::QueueConstruction {
            colony_id,
            building_type: "hex_remediation".into(),
            slot_cost: 0,
            labor_per_turn: 2,
            construction_cost: vec![("structural_metal".to_string(), 25.0)],
            construction_turns: 3,
        });
        assert!(
            matches!(refused, Err(EngineError::TechLocked { .. })),
            "expected TechLocked before bioremediation is researched, got {refused:?}"
        );

        engine
            .state
            .tech_state
            .researched
            .insert("bioremediation".into());

        engine
            .apply(&Command::QueueConstruction {
                colony_id,
                building_type: "hex_remediation".into(),
                slot_cost: 0,
                labor_per_turn: 2,
                construction_cost: vec![("structural_metal".to_string(), 25.0)],
                construction_turns: 3,
            })
            .expect("hex_remediation should be queueable once bioremediation is researched");

        engine.apply(&Command::AdvanceColonySol).unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        let evs = engine.apply(&Command::AdvanceColonySol).unwrap();

        let remediated = evs
            .iter()
            .any(|e| matches!(e, Event::ContaminationRemediated { .. }));
        assert!(
            remediated,
            "expected ContaminationRemediated once the project completes, got {evs:?}"
        );
        let contamination_after = engine
            .state
            .home_map()
            .unwrap()
            .cell(coord)
            .unwrap()
            .contamination;
        assert!(
            contamination_after < 0.5,
            "the hex's contamination should have dropped, got {contamination_after}"
        );
    }

    /// Real-engine proof that `waste_processing` capacity behaves like
    /// `housing`: a standing capacity re-established every sol by the
    /// `recycling_plant`, not a stock that accumulates or drains (issue #386).
    #[test]
    fn recycling_plant_reestablishes_waste_processing_capacity_each_sol() {
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
                name: "Recycling Plant Test".into(),
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
            .push(PlacedBuilding::new("recycling_plant", 0));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();
        let capacity_sol1 = engine.state.colonies[idx]
            .resources
            .amount("waste_processing");
        assert!(
            capacity_sol1 > 0.0,
            "recycling_plant should provide waste_processing capacity"
        );

        // A second sol should re-establish the same standing capacity, not
        // accumulate it (unlike a stock) or drain it (unlike a draw).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        let capacity_sol2 = engine.state.colonies[idx]
            .resources
            .amount("waste_processing");
        assert!(
            (capacity_sol2 - capacity_sol1).abs() < 1e-6,
            "waste_processing capacity should be re-established each sol, not accumulated: sol1={capacity_sol1}, sol2={capacity_sol2}"
        );
    }

    /// Real-engine proof that the fabricator's new component recipes (#208)
    /// actually work against the real content pack: seed processed metals,
    /// switch `fabricator` through each of the three new recipes, and
    /// confirm each one produces its corresponding component commodity.
    #[test]
    fn fabricator_recipe_selection_produces_each_component_from_real_pack() {
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
                name: "Fabricator Test".into(),
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
            .push(PlacedBuilding::new("fabricator", 2));
        // Power source — fabricator alone demands 14kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        // Seed generous processed-metal (and #210 plastics, #216
        // semiconductors) stock for all three new recipes.
        engine.state.colonies[idx]
            .pool
            .deposit("structural_metal", 100.0);
        engine.state.colonies[idx]
            .pool
            .deposit("conductive_metal", 100.0);
        engine.state.colonies[idx]
            .pool
            .deposit("precious_metal", 100.0);
        engine.state.colonies[idx]
            .pool
            .deposit("refractory_metal", 100.0);
        engine.state.colonies[idx].pool.deposit("plastics", 100.0);
        engine.state.colonies[idx]
            .pool
            .deposit("semiconductors", 100.0);

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "fabricator".into(),
                recipe_id: "fabricate_mechanical_components".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("mechanical_components") > 0.0,
            "fabricator should produce mechanical_components after switching to fabricate_mechanical_components"
        );

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "fabricator".into(),
                recipe_id: "fabricate_electronic_components".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("electronic_components") > 0.0,
            "fabricator should produce electronic_components after switching to fabricate_electronic_components"
        );

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "fabricator".into(),
                recipe_id: "fabricate_alloys".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("alloys") > 0.0,
            "fabricator should produce alloys after switching to fabricate_alloys"
        );
    }

    /// Real-engine proof that the manufactory's end-item recipes (#209)
    /// actually work against the real content pack: seed components and
    /// precious metal, switch `manufactory` through both new recipes, and
    /// confirm each one produces its corresponding end-item commodity.
    #[test]
    fn manufactory_recipe_selection_produces_each_end_item_from_real_pack() {
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
                name: "Manufactory Test".into(),
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
            .push(PlacedBuilding::new("manufactory", 2));
        // Power source — manufactory alone demands 16kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        // Seed generous component/metal stock for both new recipes.
        engine.state.colonies[idx]
            .pool
            .deposit("mechanical_components", 100.0);
        engine.state.colonies[idx]
            .pool
            .deposit("electronic_components", 100.0);
        engine.state.colonies[idx]
            .pool
            .deposit("precious_metal", 100.0);

        // Default (no active_recipes entry) should be assemble_consumer_goods
        // (sorts first alphabetically among the manufactory's recipes).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("consumer_goods") > 0.0,
            "manufactory should default to assemble_consumer_goods with no active_recipes entry"
        );
        assert_eq!(engine.state.colonies[idx].pool.amount("luxury_goods"), 0.0);

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "manufactory".into(),
                recipe_id: "craft_luxury_goods".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("luxury_goods") > 0.0,
            "manufactory should produce luxury_goods after switching to craft_luxury_goods"
        );
    }

    /// Real-engine proof that the petrochemical plant's hydrocarbon-refining
    /// recipes (#210) actually work against the real content pack: seed
    /// hydrocarbons, switch `petrochemical_plant` through all three recipes,
    /// and confirm each one produces its corresponding processed commodity.
    #[test]
    fn petrochemical_plant_recipe_selection_produces_each_output_from_real_pack() {
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
                name: "Petrochemical Test".into(),
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
            .push(PlacedBuilding::new("petrochemical_plant", 2));
        // Power source — petrochemical_plant alone demands 13kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        engine.state.colonies[idx]
            .pool
            .deposit("hydrocarbons", 100.0);

        // Default (no active_recipes entry) should be refine_chemicals
        // (sorts first alphabetically among the plant's three recipes).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("chemicals") > 0.0,
            "petrochemical_plant should default to refine_chemicals with no active_recipes entry"
        );
        assert_eq!(engine.state.colonies[idx].pool.amount("fuel"), 0.0);
        assert_eq!(engine.state.colonies[idx].pool.amount("plastics"), 0.0);

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "petrochemical_plant".into(),
                recipe_id: "refine_fuel".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("fuel") > 0.0,
            "petrochemical_plant should produce fuel after switching to refine_fuel"
        );

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "petrochemical_plant".into(),
                recipe_id: "refine_plastics".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("plastics") > 0.0,
            "petrochemical_plant should produce plastics after switching to refine_plastics"
        );
    }

    /// Real-engine proof that the food processing plant's distinct-foodstuff
    /// recipes (#211) actually work against the real content pack: seed
    /// biomass, confirm the default recipe produces protein_rations (not
    /// produce_rations), then switch via SetActiveRecipe and confirm
    /// produce_rations comes out instead.
    #[test]
    fn food_processing_plant_recipe_selection_produces_each_foodstuff_from_real_pack() {
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
                name: "Food Processing Test".into(),
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
            .push(PlacedBuilding::new("food_processing_plant", 1));
        // Power source — food_processing_plant alone demands 7kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        engine.state.colonies[idx].pool.deposit("biomass", 100.0);

        // Default (no active_recipes entry) should be ferment_protein_rations
        // (sorts first alphabetically among the plant's two recipes).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("protein_rations") > 0.0,
            "food_processing_plant should default to ferment_protein_rations with no active_recipes entry"
        );
        assert_eq!(
            engine.state.colonies[idx].pool.amount("produce_rations"),
            0.0
        );

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "food_processing_plant".into(),
                recipe_id: "press_produce_rations".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("produce_rations") > 0.0,
            "food_processing_plant should produce produce_rations after switching to press_produce_rations"
        );
    }

    /// Real-engine proof that the gas extractor's two recipes (#214)
    /// actually work against the real content pack: confirm the default
    /// recipe produces fusion_fuel (not noble_gases), then switch via
    /// SetActiveRecipe and confirm noble_gases comes out instead.
    #[test]
    fn gas_extractor_recipe_selection_produces_each_gas_from_real_pack() {
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
                name: "Gas Extractor Test".into(),
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
            .push(PlacedBuilding::new("gas_extractor", 1));
        // Power source — gas_extractor alone demands 6kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        // Default (no active_recipes entry) should be extract_fusion_fuel
        // (sorts first alphabetically among the extractor's two recipes).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("fusion_fuel") > 0.0,
            "gas_extractor should default to extract_fusion_fuel with no active_recipes entry"
        );
        assert_eq!(engine.state.colonies[idx].pool.amount("noble_gases"), 0.0);

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "gas_extractor".into(),
                recipe_id: "extract_noble_gases".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("noble_gases") > 0.0,
            "gas_extractor should produce noble_gases after switching to extract_noble_gases"
        );
    }

    /// Real-engine proof that `fusion_reactor_prototype` (#166) no longer
    /// runs on zero inputs (#214): without fusion_fuel in the pool it
    /// produces no power, and once fusion_fuel is seeded it does.
    #[test]
    fn fusion_reactor_requires_fusion_fuel_from_real_pack() {
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
                name: "Fusion Reactor Test".into(),
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
            .push(PlacedBuilding::new("fusion_reactor_prototype", 2));

        // No fusion_fuel seeded — the reactor should produce zero power.
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(
            engine.state.colonies[idx].resources.amount("power"),
            0.0,
            "fusion_reactor_prototype should no longer produce power for free"
        );

        engine.state.colonies[idx]
            .pool
            .deposit("fusion_fuel", 100.0);
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].resources.amount("power") > 0.0,
            "fusion_reactor_prototype should produce power once fusion_fuel is available"
        );
    }

    /// Real-engine proof that `colony_hq` — the consolidated multi-function
    /// starter building (playtest feedback: "multi-function starter
    /// buildings") — actually runs all four of its `concurrent: true`
    /// recipes (power, water, oxygen, research) simultaneously every turn
    /// against the real content pack. The first three match the combined
    /// output of building solar_array_mk1 + water_well +
    /// life_support_module standalone; the fourth is the administrative
    /// research trickle from issue #310, which this test follows all the way
    /// into the system-wide research pool the tech tree spends from.
    #[test]
    fn colony_hq_runs_all_concurrent_recipes_from_real_pack() {
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
                name: "Colony HQ Test".into(),
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
            .push(PlacedBuilding::new("colony_hq", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();

        let pool = &engine.state.colonies[idx].pool;
        // `power` is a colony resource, not cargo, so it lives in `resources`
        // rather than the commodity pool (issue #304).
        assert!(
            engine.state.colonies[idx].resources.amount("power") > 0.0,
            "colony_hq should produce power (hq_generate_power)"
        );
        // `water` is also a colony resource, not cargo, since issue #380.
        assert!(
            engine.state.colonies[idx].resources.amount("water") > 0.0,
            "colony_hq should produce water (hq_pump_water)"
        );
        assert!(
            pool.amount("oxygen") > 0.0,
            "colony_hq should produce oxygen (hq_scrub_oxygen)"
        );

        // Research (issue #310) is checked at the system pool rather than the
        // colony pool: `AdvanceColonySol`'s research-aggregation step drains
        // every colony's `research` into `state.research_pool`, so a non-zero
        // total here proves the HQ's trickle reaches what the tech tree
        // actually spends — not merely that a recipe fired.
        assert!(
            engine.state.research_pool.total() > 0.0,
            "colony_hq's administrative trickle (hq_conduct_research) should \
             reach the system-wide research pool"
        );
        assert!(
            engine.state.colonies[idx].resources.amount("research") < 1e-6,
            "research should be drained out of the colony pool, not stockpiled"
        );

        // Keyed by placed-instance id since #307 stage 4, not by building type.
        let hq_result = engine.state.colonies[idx]
            .last_production_by_building
            .values()
            .find(|r| r.building_type == "colony_hq")
            .expect("colony_hq should have a recorded production result");
        let mut ids = hq_result.concurrent_recipe_ids.clone();
        ids.sort();
        assert_eq!(
            ids,
            vec![
                "hq_conduct_research",
                "hq_generate_power",
                "hq_pump_water",
                "hq_scrub_oxygen"
            ],
            "all four concurrent recipes should have run"
        );
    }

    /// Real-engine proof that the other 5 landing-kit buildings from the
    /// starter-roster redesign (`ice_miner`, `excavation_rig`,
    /// `fabrication_complex`, `air_miner`, `chem_plant`) actually produce
    /// against the real content pack, and that `fabrication_complex`'s
    /// ore→metal recipe (its sorted-first default) and `chem_plant`'s
    /// carbon+water recipe pick up `excavation_rig`/`air_miner`/
    /// `ice_miner`'s outputs on the following sol (inputs are drawn from
    /// start-of-turn pool state, so a producer's own first-sol output isn't
    /// available to a consumer until the next sol — matching the
    /// established pattern in
    /// `semiconductor_chain_produces_semiconductors_from_real_pack`).
    #[test]
    fn landing_kit_extraction_and_processing_buildings_produce_from_real_pack() {
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
                name: "Landing Kit Test".into(),
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

        for building_type in [
            "ice_miner",
            "excavation_rig",
            "fabrication_complex",
            "air_miner",
            "chem_plant",
            // Every real landing kit includes a free water_tank (issue
            // #380) — without it, sol 1's water would evaporate before
            // sol 2's chem_plant could consume it, since water is now a
            // per-sol colony resource rather than a persistent commodity.
            "water_tank",
        ] {
            engine.state.colonies[idx]
                .buildings
                .push(PlacedBuilding::new(building_type, 1));
        }

        // Sol 1: each extraction building deposits its raw output into the
        // pool (fabrication_complex/chem_plant have no input yet, so their
        // recipes don't run this sol — that's expected).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        {
            let pool = &engine.state.colonies[idx].pool;
            // `water` is a colony resource, not cargo, since issue #380.
            assert!(
                engine.state.colonies[idx].resources.amount("water") > 0.0,
                "ice_miner should produce water"
            );
            assert!(
                pool.amount("structural_ore") > 0.0,
                "excavation_rig should produce ore"
            );
            assert!(
                pool.amount("oxygen") > 0.0,
                "air_miner should produce oxygen"
            );
            assert!(
                pool.amount("carbon") > 0.0,
                "air_miner should produce carbon"
            );
        }

        // Sol 2: fabrication_complex/chem_plant now have start-of-turn
        // inputs available from sol 1's production.
        engine.apply(&Command::AdvanceColonySol).unwrap();
        let pool = &engine.state.colonies[idx].pool;
        assert!(
            pool.amount("structural_metal") > 0.0,
            "fabrication_complex's default recipe (foundry_smelt_ore) should consume \
             excavation_rig's sol-1 ore and produce structural_metal"
        );
        assert!(
            pool.amount("chemicals") > 0.0,
            "chem_plant should synthesize chemicals from air_miner's carbon + ice_miner's water"
        );
    }

    /// Real-engine proof that the fission_reactor (#215) both produces
    /// power from nuclear_fuel and emits radioactive_waste as a byproduct
    /// of running — the resolved design decision that waste comes from
    /// reactor operation, not from refining, against the real content pack.
    #[test]
    fn fission_reactor_produces_power_and_radioactive_waste_from_real_pack() {
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
                name: "Fission Reactor Test".into(),
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
            .push(PlacedBuilding::new("fission_reactor", 2));

        // No nuclear_fuel seeded — the reactor should produce nothing.
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(engine.state.colonies[idx].resources.amount("power"), 0.0);
        assert_eq!(
            engine.state.colonies[idx].pool.amount("radioactive_waste"),
            0.0
        );

        engine.state.colonies[idx]
            .pool
            .deposit("nuclear_fuel", 100.0);
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].resources.amount("power") > 0.0,
            "fission_reactor should produce power once nuclear_fuel is available"
        );
        assert!(
            engine.state.colonies[idx].pool.amount("radioactive_waste") > 0.0,
            "fission_reactor should emit radioactive_waste as a byproduct of running"
        );
    }

    /// Real-engine proof that the fissile_mine -> nuclear_refinery chain
    /// (#215) actually works against the real content pack.
    #[test]
    fn nuclear_fuel_chain_produces_nuclear_fuel_from_real_pack() {
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
                name: "Nuclear Fuel Chain Test".into(),
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
            .push(PlacedBuilding::new("fissile_mine", 1));
        // Power source — fissile_mine alone demands 7kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("fissile_ore") > 0.0,
            "fissile_mine should produce fissile_ore"
        );

        engine.state.colonies[idx]
            .pool
            .deposit("fissile_ore", 100.0);
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("nuclear_refinery", 2));
        // nuclear_refinery alone demands 14kW; swap to a bigger array.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk2", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("nuclear_fuel") > 0.0,
            "nuclear_refinery should produce nuclear_fuel from fissile_ore"
        );
    }

    /// Real-engine proof that `atmospheric_harvester`'s carbon byproduct
    /// (#216) and `refractory_foundry`'s new `synthesize_carbon_composites`
    /// recipe actually work together against the real content pack: the
    /// foundry's default recipe stays `refine_refractory_ore` (#207's
    /// invariant, untouched by this issue), and switching it produces
    /// carbon_composites from harvested carbon.
    #[test]
    fn carbon_composites_chain_produces_carbon_composites_from_real_pack() {
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
                name: "Carbon Composites Test".into(),
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
            .push(PlacedBuilding::new("atmospheric_harvester", 1));
        // Power source — atmospheric_harvester alone demands 8kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("carbon") > 0.0,
            "atmospheric_harvester should produce carbon alongside oxygen/trace_minerals"
        );

        engine.state.colonies[idx].pool.deposit("carbon", 100.0);
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("refractory_foundry", 2));
        // refractory_foundry alone demands 16kW; swap to a bigger array.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk2", 1));

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "refractory_foundry".into(),
                recipe_id: "synthesize_carbon_composites".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("carbon_composites") > 0.0,
            "refractory_foundry should produce carbon_composites after switching to synthesize_carbon_composites"
        );
    }

    /// Real-engine proof that the semiconductor_mine -> semiconductor_fab
    /// chain (#216) actually works against the real content pack, closing
    /// the gap #208 left open for electronic_components.
    #[test]
    fn semiconductor_chain_produces_semiconductors_from_real_pack() {
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
                name: "Semiconductor Chain Test".into(),
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
            .push(PlacedBuilding::new("semiconductor_mine", 1));
        // Power source — semiconductor_mine alone demands 7kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("semiconductor_ore") > 0.0,
            "semiconductor_mine should produce semiconductor_ore"
        );

        engine.state.colonies[idx]
            .pool
            .deposit("semiconductor_ore", 100.0);
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("semiconductor_fab", 2));
        // semiconductor_fab alone demands 18kW; swap to a bigger array.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk2", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("semiconductors") > 0.0,
            "semiconductor_fab should produce semiconductors from semiconductor_ore"
        );
    }

    /// Real-engine proof that the silicates -> ceramics/glass/crystals chain
    /// (issue #225) actually works against the real content pack: one
    /// silicate_quarry feeds a ceramics_kiln (switchable between
    /// fire_ceramics and smelt_glass, issue #166 recipe selection) and a
    /// crystal_grower.
    #[test]
    fn silicates_chain_produces_ceramics_glass_crystals_from_real_pack() {
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
                name: "Silicates Chain Test".into(),
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
            .push(PlacedBuilding::new("silicate_quarry", 1));
        // Power source — silicate_quarry alone demands 6kW.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("silicates") > 0.0,
            "silicate_quarry should produce silicates"
        );

        engine.state.colonies[idx].pool.deposit("silicates", 200.0);
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("ceramics_kiln", 2));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("crystal_grower", 3));
        // Cumulative demand now exceeds a single mk1 array; add a mk2.
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk2", 1));

        // ceramics_kiln defaults to fire_ceramics (sorts before smelt_glass).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("ceramics") > 0.0,
            "ceramics_kiln should produce ceramics via its default fire_ceramics recipe"
        );
        assert!(
            engine.state.colonies[idx].pool.amount("crystals") > 0.0,
            "crystal_grower should produce crystals"
        );

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "ceramics_kiln".into(),
                recipe_id: "smelt_glass".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("glass") > 0.0,
            "ceramics_kiln should produce glass after switching to smelt_glass"
        );
    }

    /// Real-engine proof that the fabricator's `fabricate_composites` and
    /// `fabricate_fiber_optics` recipes (issue #225) actually work against
    /// the real content pack.
    #[test]
    fn fabricator_composites_and_fiber_optics_produce_from_real_pack() {
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
                name: "Composites Fiber Optics Test".into(),
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
            .push(PlacedBuilding::new("fabricator", 2));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk1", 1));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("solar_array_mk2", 1));

        engine.state.colonies[idx].pool.deposit("ceramics", 100.0);
        engine.state.colonies[idx].pool.deposit("plastics", 100.0);
        engine.state.colonies[idx].pool.deposit("glass", 100.0);

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "fabricator".into(),
                recipe_id: "fabricate_composites".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("composites") > 0.0,
            "fabricator should produce composites after switching to fabricate_composites"
        );

        engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "fabricator".into(),
                recipe_id: "fabricate_fiber_optics".into(),
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("fiber_optics") > 0.0,
            "fabricator should produce fiber_optics after switching to fabricate_fiber_optics"
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

    /// The per-building staffing commands translate off the wire (issue #307).
    ///
    /// These are the commands that make per-building labour steerable at all, so
    /// a mapping mistake would leave the mechanism live but unreachable.
    #[test]
    fn client_command_per_building_staffing_translates() {
        use crate::config::RuntimeConfig;
        use crate::state::new_state;
        use outpost_core::colony::ColonyId;
        use outpost_core::Command;
        use uuid::Uuid;

        let colony_id = ColonyId::new_v4();
        let building_id = Uuid::new_v4();
        let state = new_state(RuntimeConfig::default());

        let priority = client_command_to_core(
            ClientCommand::SetBuildingPriority {
                colony_id: colony_id.to_string(),
                building_id: building_id.to_string(),
                priority: 2,
            },
            &state,
        )
        .expect("priority translates");
        match priority {
            Command::SetBuildingPriority {
                colony_id: got_colony,
                building_id: got_building,
                priority: got,
            } => {
                assert_eq!(got_colony, colony_id);
                assert_eq!(got_building, building_id);
                assert_eq!(got, 2);
            }
            other => panic!("expected SetBuildingPriority, got {other:?}"),
        }

        let lock = client_command_to_core(
            ClientCommand::SetBuildingLabourLock {
                colony_id: colony_id.to_string(),
                building_id: building_id.to_string(),
                lock: Some(3),
            },
            &state,
        )
        .expect("lock translates");
        match lock {
            Command::SetBuildingLabourLock {
                building_id: got_building,
                lock: got,
                ..
            } => {
                assert_eq!(got_building, building_id);
                assert_eq!(got, Some(3));
            }
            other => panic!("expected SetBuildingLabourLock, got {other:?}"),
        }

        // `null` must survive as an unlock, not collapse into a zero-worker lock.
        let unlock = client_command_to_core(
            ClientCommand::SetBuildingLabourLock {
                colony_id: colony_id.to_string(),
                building_id: building_id.to_string(),
                lock: None,
            },
            &state,
        )
        .expect("unlock translates");
        match unlock {
            Command::SetBuildingLabourLock { lock, .. } => {
                assert_eq!(lock, None, "None is an unlock, not Some(0)");
            }
            other => panic!("expected SetBuildingLabourLock, got {other:?}"),
        }

        let rename = client_command_to_core(
            ClientCommand::RenameBuilding {
                colony_id: colony_id.to_string(),
                building_id: building_id.to_string(),
                name: Some("North Vein".into()),
            },
            &state,
        )
        .expect("rename translates");
        match rename {
            Command::RenameBuilding { name, .. } => {
                assert_eq!(name.as_deref(), Some("North Vein"));
            }
            other => panic!("expected RenameBuilding, got {other:?}"),
        }

        let paused = client_command_to_core(
            ClientCommand::SetBuildingPaused {
                colony_id: colony_id.to_string(),
                building_id: building_id.to_string(),
                paused: true,
            },
            &state,
        )
        .expect("pause translates");
        match paused {
            Command::SetBuildingPaused {
                building_id: got_building,
                paused: got,
                ..
            } => {
                assert_eq!(got_building, building_id);
                assert!(got);
            }
            other => panic!("expected SetBuildingPaused, got {other:?}"),
        }
    }

    /// A bad building UUID is reported, not panicked on.
    #[test]
    fn client_command_per_building_staffing_rejects_a_bad_building_id() {
        use crate::config::RuntimeConfig;
        use crate::state::new_state;
        use outpost_core::colony::ColonyId;

        let state = new_state(RuntimeConfig::default());
        let result = client_command_to_core(
            ClientCommand::SetBuildingPriority {
                colony_id: ColonyId::new_v4().to_string(),
                building_id: "not-a-uuid".into(),
                priority: 1,
            },
            &state,
        );
        assert!(result.is_err(), "a malformed building id must be rejected");
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
                width: 10,
                height: 6,
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
            .apply(&Command::SeedPlanet {
                seed: 1,
                width: 10,
                height: 6,
            })
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
    /// Colony resources are per-sol, not stockpiled (issue #304).
    ///
    /// Before resources existed, `power` and `housing` were ordinary
    /// commodities and both accumulated without bound: power netted a surplus
    /// every sol and banked it forever, and housing — a capacity check that
    /// consumes nothing — gained a whole habitat's worth every sol, so the
    /// housing need became trivially satisfied after a few turns. This pins the
    /// steady state that replaced it, and that neither leaks into the tradeable
    /// commodity pool.
    #[test]
    fn colony_resources_hold_steady_each_sol_instead_of_accumulating() {
        use outpost_core::colony::PlacedBuilding;
        use outpost_core::{Command, Event, GameEngine};

        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let registry =
            load_content_pack_from_dir(&root.join("content").join("base")).expect("pack loads");
        let mut engine = GameEngine::with_seed(0);
        engine.state.registry = Some(registry);

        let events = engine
            .apply(&Command::FoundColony {
                name: "Steady".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let cid = *colony_id;
        let idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == cid)
            .unwrap();
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("habitat_pod", 1));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("colony_hq", 2));

        let mut first: Option<(f64, f64)> = None;
        for sol in 1..=8 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
            let c = &engine.state.colonies[idx];
            let housing = c.resources.amount("housing");
            let power = c.resources.amount("power");

            assert!(power > 0.0, "sol {sol}: colony_hq should have made power");
            assert!(housing > 0.0, "sol {sol}: habitat_pod should offer housing");

            match first {
                None => first = Some((housing, power)),
                Some((h0, p0)) => {
                    assert!(
                        (housing - h0).abs() < 1e-6,
                        "sol {sol}: housing drifted from {h0} to {housing} — capacity is \
                         re-established each sol, it must not accumulate"
                    );
                    assert!(
                        (power - p0).abs() < 1e-6,
                        "sol {sol}: power drifted from {p0} to {power} — unused power is \
                         lost each sol, it must not bank"
                    );
                }
            }

            // And none of it leaked into the tradeable pool, where trade could
            // have shipped it.
            for id in ["power", "housing", "research"] {
                assert_eq!(
                    c.pool.amount(id),
                    0.0,
                    "sol {sol}: {id} must not appear in the commodity pool"
                );
            }
        }

        // Research is the exception that proves the drain works: it leaves the
        // colony each sol and banks in the system-wide pool the tech tree spends.
        assert!(
            (engine.state.research_pool.total() - 8.0).abs() < 1e-4,
            "8 sols of colony_hq's 1 RP/sol trickle should have banked 8 RP, got {}",
            engine.state.research_pool.total()
        );
    }
    /// Housing must reach the migration model through the real content pack.
    ///
    /// Regression test for a bug this refactor introduced and a reviewer caught:
    /// production began depositing `housing` into `Colony.resources`, but four
    /// call sites in the migration/attractiveness path still read
    /// `colony.pool.amount("housing")`. That is permanently `0.0` once the pack
    /// is loaded, so the housing term of every colony's attractiveness score
    /// silently evaluated to zero and arrival overcrowding penalties stopped
    /// firing entirely.
    ///
    /// The whole class of bug was invisible because every migration test builds
    /// a bare `GameEngine` with **no registry**, deposits `housing` into the
    /// commodity pool by hand, and therefore exercises the pre-#304 path. This
    /// test loads `content/base` so the registry-driven dispatch is real.
    #[test]
    fn housing_reaches_the_migration_model_with_the_real_pack() {
        use outpost_core::colony::PlacedBuilding;
        use outpost_core::{Command, Event, GameEngine};

        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let root = std::path::Path::new(&manifest)
            .parent()
            .unwrap_or(std::path::Path::new("."));
        let registry =
            load_content_pack_from_dir(&root.join("content").join("base")).expect("pack loads");
        let mut engine = GameEngine::with_seed(0);
        engine.state.registry = Some(registry);

        let events = engine
            .apply(&Command::FoundColony {
                name: "Housed".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == *colony_id)
            .unwrap();
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("habitat_pod", 1));
        engine.state.colonies[idx]
            .buildings
            .push(PlacedBuilding::new("colony_hq", 2));

        engine.apply(&Command::AdvanceColonySol).unwrap();

        // The habitat's capacity landed in the resource store, not the pool.
        assert!(
            engine.state.colonies[idx].resources.amount("housing") > 0.0,
            "habitat_pod should have established housing capacity"
        );
        assert_eq!(
            engine.state.colonies[idx].pool.amount("housing"),
            0.0,
            "housing is a colony resource, not commodity stock"
        );

        // And the migration path can see it: `RunAutoMigration` reads housing to
        // compute attractiveness. Before the fix this read the commodity pool
        // and so saw 0.0 no matter how many habitats the colony had.
        engine
            .apply(&Command::RunAutoMigration)
            .expect("auto migration should run against the real pack");
    }
}
