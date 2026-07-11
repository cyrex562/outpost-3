//! WebSocket game-loop handler (`/ws`).
//!
//! On connection the client receives a [`WorldSnapshot`] of current state.
//! Subsequent inbound [`ClientMessage`]s drive the engine or query it.
//! Engine events are broadcast to all connected clients as [`ServerMessage::Event`].

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::sync::broadcast::error::RecvError;

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
