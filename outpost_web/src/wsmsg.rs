//! Typed WebSocket message contract — server→client and client→server.
//!
//! All messages are JSON objects with a `"type"` discriminant so the TypeScript
//! client can exhaustively switch on `msg.type` without `any` or runtime casts.
//!
//! # Server → Client
//!
//! | `type`        | Payload                         | Meaning                          |
//! |---------------|---------------------------------|----------------------------------|
//! | `"snapshot"`  | `{ state: WorldSnapshot }`      | Full world state on connect      |
//! | `"event"`     | `{ event: ServerEvent }`        | Incremental engine event         |
//! | `"error"`     | `{ message: string }`           | Command rejected                 |
//! | `"ack"`       | `{ seq: number }`               | Command acknowledged             |
//!
//! # Client → Server
//!
//! | `"type"`      | Payload                         | Meaning                          |
//! |---------------|---------------------------------|----------------------------------|
//! | `"command"`   | `{ seq, command: ClientCmd }`   | Drive the engine                 |
//! | `"query"`     | `{ seq, query: ClientQuery }`   | Read-only state query            |

use outpost_core::{ColonyStatus, ColonySummary, Event, QueryResult};
use serde::{Deserialize, Serialize};

// ─── Client → Server ─────────────────────────────────────────────────────────

/// An inbound message from the frontend client.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Drive the engine with a command.
    Command {
        /// Client-assigned sequence number echoed back in the ack.
        seq: u64,
        /// The command payload.
        command: ClientCommand,
    },
    /// Read-only query.
    Query {
        /// Client-assigned sequence number.
        seq: u64,
        /// The query variant.
        query: ClientQuery,
    },
}

/// Commands the client can submit to the engine.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientCommand {
    /// Advance one colony-sol turn.
    AdvanceSol,
    /// Found a new colony.
    FoundColony {
        /// Colony display name.
        name: String,
        /// Starting colonist head-count.
        starting_population: u64,
    },
    /// Queue a construction project.
    QueueConstruction {
        /// Target colony UUID.
        colony_id: String,
        /// Content-pack key of the building type.
        building_type: String,
        /// Number of build slots the building consumes.
        slot_cost: u32,
        /// Labour units consumed per construction turn.
        labor_per_turn: u32,
        /// Commodity costs (commodity id, quantity) pairs.
        construction_cost: Vec<(String, f64)>,
        /// Colony-sol turns required to complete.
        construction_turns: u32,
    },
    /// Assign labour to a production slot.
    AssignLabour {
        /// Target colony UUID.
        colony_id: String,
        /// Slot name.
        slot: String,
        /// Labour units to assign.
        labour: u64,
    },
    /// Set the active difficulty preset.
    SetDifficulty {
        /// Difficulty grade: "sandbox", "easy", "normal", "hard", or "brutal".
        grade: String,
    },
    /// Append a tech to the research queue.
    EnqueueResearch {
        /// Content-pack tech identifier.
        tech_id: String,
    },
    /// Cancel the active research project and clear the queue.
    CancelResearch,
    /// Open a voluntary emigration gate between two colonies.
    OpenEmigrationGate {
        /// Source colony UUID.
        from_colony: String,
        /// Destination colony UUID.
        to_colony: String,
        /// Fraction of source population that departs per strategic month.
        rate: f32,
    },
    /// Build infrastructure between two colonies.
    BuildInfrastructure {
        /// Source colony UUID.
        from_colony: String,
        /// Destination colony UUID.
        to_colony: String,
        /// Infrastructure type: "road", "rail", or "pipeline".
        infra_type: String,
    },
    /// Begin construction of an orbital station using a blueprint.
    BeginOrbitalConstruction {
        /// Content-pack blueprint identifier.
        blueprint_id: String,
        /// Colony that funds and operates the station.
        colony_id: String,
        /// Orbit band: "low", "geostationary", or "lagrange".
        orbit_type: String,
    },
    /// Launch a field expedition from a colony to a hex tile.
    LaunchFieldExpedition {
        /// Launching colony UUID.
        colony_id: String,
        /// Target hex column coordinate.
        target_hex_q: i32,
        /// Target hex row coordinate.
        target_hex_r: i32,
        /// Number of crew assigned.
        crew: u32,
        /// Supplies loaded for the mission.
        supplies: f32,
        /// Sols required for transit.
        transit_sols: u64,
        /// Whether this is a deep-space expedition.
        is_deep_space: bool,
    },
    /// Recall an active field expedition back to its origin colony.
    RecallExpedition {
        /// Stable UUID of the expedition to recall.
        expedition_id: String,
    },
    /// Activate sandbox-continue mode after a victory.
    ContinueSandbox,
    /// Snapshot current engine state to the configured SQLite database.
    SaveGame,
    /// Restore engine state from the configured SQLite database.
    LoadGame,
    /// Register or replace a directive for a colony.
    SetDirective {
        /// Colony UUID the directive targets.
        colony_id: String,
        /// Serialised directive payload (JSON).
        directive_json: String,
    },
    /// Remove a directive by its UUID.
    RemoveDirective {
        /// UUID of the directive to remove.
        directive_id: String,
    },
    /// Enable or disable manual override for a colony.
    SetManualOverride {
        /// Target colony UUID.
        colony_id: String,
        /// `true` to enable manual override; `false` to resume automation.
        enabled: bool,
    },
}

/// Queries the client can issue (read-only, no state mutation).
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientQuery {
    /// Current colony-sol counter.
    CurrentSol,
    /// Current strategic-month counter.
    CurrentMonth,
    /// List of all colonies.
    ListColonies,
    /// Detailed status for a colony.
    ColonyStatus {
        /// Target colony UUID.
        colony_id: String,
    },
}

// ─── Server → Client ─────────────────────────────────────────────────────────

/// An outbound message from the server to the client.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Full world snapshot sent to a new client on connection.
    Snapshot {
        /// Current world snapshot.
        state: WorldSnapshot,
    },
    /// An incremental engine event.
    Event {
        /// The event payload.
        event: ServerEvent,
    },
    /// A command was rejected.
    Error {
        /// Human-readable rejection reason.
        message: String,
    },
    /// A command was accepted and executed.
    Ack {
        /// The sequence number from the originating [`ClientMessage::Command`].
        seq: u64,
    },
    /// Response to a client query.
    QueryResult {
        /// The sequence number from the originating [`ClientMessage::Query`].
        seq: u64,
        /// The result data.
        result: QueryResultPayload,
    },
}

/// Payload variants for [`ServerMessage::QueryResult`].
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueryResultPayload {
    /// Counter value (sol or month).
    Counter {
        /// The counter value.
        value: u64,
    },
    /// List of colony summaries.
    Colonies {
        /// The colonies.
        colonies: Vec<ColonySummary>,
    },
    /// Detailed colony status.
    ColonyStatus {
        /// The status data.
        status: ColonyStatus,
    },
    /// System-wide research pool.
    ResearchTotal {
        /// Accumulated research points.
        total: f32,
    },
    /// Available labour for a colony.
    Labour {
        /// Labour units available.
        labour: f32,
    },
}

/// A typed engine event forwarded to the client.
///
/// Wraps [`outpost_core::Event`] so the TypeScript layer can depend on a
/// stable JSON shape rather than the Rust enum's internal representation.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerEvent {
    /// A colony-sol turn completed.
    ColonySolAdvanced {
        /// New sol counter value.
        sol: u64,
    },
    /// A strategic-month turn completed.
    StrategicMonthAdvanced {
        /// New month counter value.
        month: u64,
    },
    /// A colony was founded.
    ColonyFounded {
        /// Colony UUID.
        colony_id: String,
        /// Colony display name.
        name: String,
        /// Starting population.
        starting_population: u64,
    },
    /// A construction project was queued.
    ConstructionQueued {
        /// Colony UUID.
        colony_id: String,
        /// Building type key.
        building_type: String,
        /// Project UUID.
        project_id: String,
    },
    /// A construction project was cancelled.
    ConstructionCancelled {
        /// Colony UUID.
        colony_id: String,
        /// Project UUID.
        project_id: String,
        /// Refunded commodities.
        refund: Vec<(String, f64)>,
    },
    /// A building completed construction.
    BuildingConstructed {
        /// Colony UUID.
        colony_id: String,
        /// Building type key.
        building_type: String,
    },
    /// Labour was assigned.
    LabourAssigned {
        /// Colony UUID.
        colony_id: String,
        /// Slot name.
        slot: String,
        /// Labour units assigned.
        labour: u64,
    },
    /// Needs resolved for a colony this sol.
    NeedsResolved {
        /// Colony UUID.
        colony_id: String,
        /// Satisfaction score in `[0, 1]`.
        composite_satisfaction: f32,
        /// Stability change.
        stability_delta: f32,
        /// Population change.
        population_delta: f32,
    },
    /// Research produced by a colony.
    ResearchProduced {
        /// Colony UUID.
        colony_id: String,
        /// Amount drained to the system pool.
        amount: f32,
    },
    /// A directive was set.
    DirectiveSet {
        /// Colony UUID.
        colony_id: String,
        /// Directive UUID.
        directive_id: String,
    },
    /// A directive was removed.
    DirectiveRemoved {
        /// Directive UUID.
        directive_id: String,
    },
    /// Manual override changed.
    ManualOverrideChanged {
        /// Colony UUID.
        colony_id: String,
        /// New override state.
        enabled: bool,
    },
    /// A directive fired its action.
    DirectiveFired {
        /// Colony UUID.
        colony_id: String,
        /// Directive UUID.
        directive_id: String,
    },
    /// A building ran at reduced capacity.
    ProductionShortfall {
        /// Colony UUID.
        colony_id: String,
        /// Building type.
        building_type: String,
        /// Effective scale in `[0, 1]`.
        scale: f64,
        /// Shortfall category.
        reason: String,
    },
}

impl ServerEvent {
    /// Convert a core [`Event`] into the stable [`ServerEvent`] wire format.
    #[must_use]
    pub fn from_core(event: &Event) -> Self {
        match event {
            Event::ColonySolAdvanced { sol } => Self::ColonySolAdvanced { sol: *sol },
            Event::StrategicMonthAdvanced { month } => {
                Self::StrategicMonthAdvanced { month: *month }
            }
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
            Event::DirectiveSet {
                colony_id,
                directive_id,
            } => Self::DirectiveSet {
                colony_id: colony_id.to_string(),
                directive_id: directive_id.to_string(),
            },
            Event::DirectiveRemoved { directive_id } => Self::DirectiveRemoved {
                directive_id: directive_id.to_string(),
            },
            Event::ManualOverrideChanged { colony_id, enabled } => Self::ManualOverrideChanged {
                colony_id: colony_id.to_string(),
                enabled: *enabled,
            },
            Event::DirectiveFired {
                colony_id,
                directive_id,
            } => Self::DirectiveFired {
                colony_id: colony_id.to_string(),
                directive_id: directive_id.to_string(),
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
            // Non-exhaustive guard: new core events default to a sol-advanced echo
            // so the frontend still receives something without a compile break.
            _ => Self::ColonySolAdvanced { sol: 0 },
        }
    }
}

/// A full world snapshot serialised on WebSocket connect.
#[derive(Debug, Serialize)]
pub struct WorldSnapshot {
    /// Current colony-sol counter.
    pub sol: u64,
    /// Current strategic-month counter.
    pub month: u64,
    /// Summaries of all existing colonies.
    pub colonies: Vec<ColonySummary>,
}

/// Build a [`ServerMessage::QueryResult`] from a core [`QueryResult`].
#[must_use]
pub fn query_result_message(seq: u64, result: QueryResult) -> ServerMessage {
    let payload = match result {
        QueryResult::Counter(v) => QueryResultPayload::Counter { value: v },
        QueryResult::Colonies(c) => QueryResultPayload::Colonies { colonies: c },
        QueryResult::ColonyStatus(s) => QueryResultPayload::ColonyStatus { status: s },
        QueryResult::ResearchTotal(t) => QueryResultPayload::ResearchTotal { total: t },
        QueryResult::Labour(l) => QueryResultPayload::Labour { labour: l },
        // Non-exhaustive arm — fall back to counter 0 for unknown variants.
        _ => QueryResultPayload::Counter { value: 0 },
    };
    ServerMessage::QueryResult {
        seq,
        result: payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use outpost_core::Event;

    #[test]
    fn server_event_sol_advanced_round_trips() {
        let core_event = Event::ColonySolAdvanced { sol: 42 };
        let se = ServerEvent::from_core(&core_event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(json.contains("\"kind\":\"colony_sol_advanced\""));
        assert!(json.contains("\"sol\":42"));
    }

    #[test]
    fn server_event_colony_founded_serialises() {
        use uuid::Uuid;
        let id = Uuid::new_v4();
        let core_event = Event::ColonyFounded {
            colony_id: id,
            name: "Alpha Base".into(),
            starting_population: 100,
        };
        let se = ServerEvent::from_core(&core_event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(json.contains("\"kind\":\"colony_founded\""));
        assert!(json.contains("Alpha Base"));
        assert!(json.contains("100"));
    }

    #[test]
    fn server_message_error_serialises() {
        let msg = ServerMessage::Error {
            message: "colony not found".into(),
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"type\":\"error\""));
        assert!(json.contains("colony not found"));
    }

    #[test]
    fn server_message_ack_serialises() {
        let msg = ServerMessage::Ack { seq: 7 };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"type\":\"ack\""));
        assert!(json.contains("\"seq\":7"));
    }

    #[test]
    fn client_command_deserialises() {
        let raw = r#"{"type":"command","seq":1,"command":{"kind":"advance_sol"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::AdvanceSol,
            } => assert_eq!(seq, 1),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_set_difficulty_deserialises() {
        let raw = r#"{"type":"command","seq":3,"command":{"kind":"set_difficulty","grade":"hard"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::SetDifficulty { grade },
            } => {
                assert_eq!(seq, 3);
                assert_eq!(grade, "hard");
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_enqueue_research_deserialises() {
        let raw = r#"{"type":"command","seq":4,"command":{"kind":"enqueue_research","tech_id":"fusion_power"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::EnqueueResearch { tech_id },
            } => {
                assert_eq!(seq, 4);
                assert_eq!(tech_id, "fusion_power");
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_cancel_research_deserialises() {
        let raw = r#"{"type":"command","seq":5,"command":{"kind":"cancel_research"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::CancelResearch,
            } => assert_eq!(seq, 5),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_open_emigration_gate_deserialises() {
        let raw = r#"{"type":"command","seq":6,"command":{"kind":"open_emigration_gate","from_colony":"00000000-0000-0000-0000-000000000001","to_colony":"00000000-0000-0000-0000-000000000002","rate":0.1}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::OpenEmigrationGate {
                        from_colony,
                        to_colony,
                        rate,
                    },
            } => {
                assert_eq!(seq, 6);
                assert!(from_colony.contains("0001"));
                assert!(to_colony.contains("0002"));
                assert!((rate - 0.1).abs() < 1e-5);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_build_infrastructure_deserialises() {
        let raw = r#"{"type":"command","seq":7,"command":{"kind":"build_infrastructure","from_colony":"00000000-0000-0000-0000-000000000001","to_colony":"00000000-0000-0000-0000-000000000002","infra_type":"road"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::BuildInfrastructure {
                        from_colony: _,
                        to_colony: _,
                        infra_type,
                    },
            } => {
                assert_eq!(seq, 7);
                assert_eq!(infra_type, "road");
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_begin_orbital_construction_deserialises() {
        let raw = r#"{"type":"command","seq":8,"command":{"kind":"begin_orbital_construction","blueprint_id":"relay_station","colony_id":"00000000-0000-0000-0000-000000000001","orbit_type":"low"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::BeginOrbitalConstruction {
                        blueprint_id,
                        colony_id: _,
                        orbit_type,
                    },
            } => {
                assert_eq!(seq, 8);
                assert_eq!(blueprint_id, "relay_station");
                assert_eq!(orbit_type, "low");
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_launch_field_expedition_deserialises() {
        let raw = r#"{"type":"command","seq":9,"command":{"kind":"launch_field_expedition","colony_id":"00000000-0000-0000-0000-000000000001","target_hex_q":3,"target_hex_r":-1,"crew":5,"supplies":100.0,"transit_sols":10,"is_deep_space":false}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::LaunchFieldExpedition {
                        colony_id: _,
                        target_hex_q,
                        target_hex_r,
                        crew,
                        supplies: _,
                        transit_sols,
                        is_deep_space,
                    },
            } => {
                assert_eq!(seq, 9);
                assert_eq!(target_hex_q, 3);
                assert_eq!(target_hex_r, -1);
                assert_eq!(crew, 5);
                assert_eq!(transit_sols, 10);
                assert!(!is_deep_space);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_recall_expedition_deserialises() {
        let raw = r#"{"type":"command","seq":10,"command":{"kind":"recall_expedition","expedition_id":"00000000-0000-0000-0000-000000000099"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::RecallExpedition { expedition_id },
            } => {
                assert_eq!(seq, 10);
                assert!(expedition_id.contains("0099"));
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_continue_sandbox_deserialises() {
        let raw = r#"{"type":"command","seq":11,"command":{"kind":"continue_sandbox"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::ContinueSandbox,
            } => assert_eq!(seq, 11),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_save_game_deserialises() {
        let raw = r#"{"type":"command","seq":12,"command":{"kind":"save_game"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::SaveGame,
            } => assert_eq!(seq, 12),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_load_game_deserialises() {
        let raw = r#"{"type":"command","seq":13,"command":{"kind":"load_game"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::LoadGame,
            } => assert_eq!(seq, 13),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_remove_directive_deserialises() {
        let raw = r#"{"type":"command","seq":14,"command":{"kind":"remove_directive","directive_id":"00000000-0000-0000-0000-000000000042"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::RemoveDirective { directive_id },
            } => {
                assert_eq!(seq, 14);
                assert!(directive_id.contains("0042"));
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_set_manual_override_deserialises() {
        let raw = r#"{"type":"command","seq":15,"command":{"kind":"set_manual_override","colony_id":"00000000-0000-0000-0000-000000000001","enabled":true}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::SetManualOverride { colony_id: _, enabled },
            } => {
                assert_eq!(seq, 15);
                assert!(enabled);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_query_deserialises() {
        let raw = r#"{"type":"query","seq":2,"query":{"kind":"current_sol"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Query {
                seq,
                query: ClientQuery::CurrentSol,
            } => assert_eq!(seq, 2),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn world_snapshot_serialises() {
        let snap = WorldSnapshot {
            sol: 1,
            month: 0,
            colonies: vec![],
        };
        let msg = ServerMessage::Snapshot { state: snap };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"type\":\"snapshot\""));
        assert!(json.contains("\"sol\":1"));
    }
}
