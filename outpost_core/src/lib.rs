//! Outpost 3 simulation core.
//!
//! A pure Rust library crate with zero I/O or framework dependencies.
//! All game logic lives here and is driven through the [`GameEngine`] interface.
//!
//! # Architecture
//!
//! The only mutation point from outside this crate is:
//! ```ignore
//! GameEngine::apply(cmd: Command) -> Result<Vec<Event>, EngineError>
//! ```
//!
//! State can be read without mutation through:
//! ```ignore
//! GameEngine::query(q: &Query) -> Result<QueryResult, EngineError>
//! ```
//!
//! Module layout mirrors the major design systems described in `docs/DESIGN.md`:
//! - [`turn`]       — two-cadence turn model (colony-sol, strategic-month)
//! - [`colony`]     — pooled commodities, slots, production chains
//! - [`content`]    — content-pack loading for authored data
//! - [`population`] — aggregate population pool, stability, labour derivation

#![warn(missing_docs)]

pub mod colony;
pub mod content;
pub mod population;
pub mod snapshot;
pub mod turn;

use thiserror::Error;

use colony::ColonyId;
use turn::{GameState, TurnProcessor};

/// Default RNG seed used when constructing a [`GameEngine`] with [`GameEngine::new`].
pub const DEFAULT_SEED: u64 = 0;

// ─── Commands ────────────────────────────────────────────────────────────────

/// A command submitted to the engine from the outside world.
///
/// Commands are the **only** way to mutate game state. All variants must be
/// expressible in test code with no UI or I/O dependencies — enabling bots,
/// the balance harness, and CI survival tests to drive the simulation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Command {
    /// Advance the simulation by one colony-sol turn.
    AdvanceColonySol,
    /// Advance the simulation by one strategic-month turn (manual override).
    AdvanceStrategicMonth,
    /// Found a new colony with the given name and starting population.
    FoundColony {
        /// Display name for the new colony.
        name: String,
        /// Starting colonist head-count.
        starting_population: u64,
    },
    /// Queue a construction project in the named colony.
    ///
    /// The `building_type` key references a record in the loaded content pack.
    /// This is a stub; detailed building mechanics arrive in later issues.
    QueueConstruction {
        /// Target colony.
        colony_id: ColonyId,
        /// Content-pack key identifying the building type to queue.
        building_type: String,
    },
    /// Assign a number of labour units to a named slot in a colony.
    ///
    /// `slot` identifies the production slot; `labour` is the worker-unit count.
    AssignLabour {
        /// Target colony.
        colony_id: ColonyId,
        /// Name of the production slot to assign labour to.
        slot: String,
        /// Number of labour units to assign.
        labour: u64,
    },
}

// ─── Queries ─────────────────────────────────────────────────────────────────

/// A read-only query submitted to the engine.
///
/// Queries never mutate state and return a [`QueryResult`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Query {
    /// Return the current colony-sol counter.
    CurrentSol,
    /// Return the current strategic-month counter.
    CurrentMonth,
    /// Return a summary of all colonies.
    ListColonies,
    /// Return detailed state for a single colony (population, labour).
    ColonyStatus {
        /// Target colony.
        colony_id: ColonyId,
    },
    /// Return available labour units for a colony.
    AvailableLabour {
        /// Target colony.
        colony_id: ColonyId,
    },
}

/// The result returned by [`GameEngine::query`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum QueryResult {
    /// A single `u64` counter value.
    Counter(u64),
    /// A list of colony summaries.
    Colonies(Vec<ColonySummary>),
    /// Status report for a single colony.
    ColonyStatus(ColonyStatus),
    /// Available labour units for a colony.
    Labour(u64),
}

/// Lightweight colony summary returned by [`Query::ListColonies`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColonySummary {
    /// Colony stable identifier.
    pub id: ColonyId,
    /// Colony display name.
    pub name: String,
    /// Current colonist head-count.
    pub population: u64,
}

/// Detailed colony status returned by [`Query::ColonyStatus`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColonyStatus {
    /// Colony stable identifier.
    pub id: ColonyId,
    /// Colony display name.
    pub name: String,
    /// Current colonist head-count.
    pub population: u64,
    /// Stability scalar in `[0.0, 1.0]`.
    pub stability: f64,
    /// Labour units available this turn.
    pub available_labour: u64,
}

// ─── Events ──────────────────────────────────────────────────────────────────

/// An event produced by the engine in response to a [`Command`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Event {
    /// A colony-sol turn completed.
    ColonySolAdvanced {
        /// The new colony-sol counter value after advancement.
        sol: u64,
    },
    /// A strategic-month turn completed.
    StrategicMonthAdvanced {
        /// The new strategic-month counter value after advancement.
        month: u64,
    },
    /// A new colony was founded.
    ColonyFounded {
        /// Stable identifier assigned to the new colony.
        colony_id: ColonyId,
        /// Display name of the new colony.
        name: String,
        /// Starting colonist head-count.
        starting_population: u64,
    },
    /// A construction project was queued in a colony.
    ConstructionQueued {
        /// Target colony.
        colony_id: ColonyId,
        /// Content-pack key of the building type queued.
        building_type: String,
    },
    /// Labour was assigned to a production slot in a colony.
    LabourAssigned {
        /// Target colony.
        colony_id: ColonyId,
        /// Name of the production slot.
        slot: String,
        /// Number of labour units assigned.
        labour: u64,
    },
}

// ─── Errors ──────────────────────────────────────────────────────────────────

/// All errors that the engine can return.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// A command was submitted while the engine was in an invalid state.
    #[error("invalid engine state: {0}")]
    InvalidState(String),
    /// The referenced colony does not exist.
    #[error("colony not found: {0}")]
    ColonyNotFound(ColonyId),
    /// A command argument was out of range or otherwise invalid.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

// ─── Engine ──────────────────────────────────────────────────────────────────

/// The top-level game engine.
///
/// Wraps [`GameState`] and [`TurnProcessor`] behind the single `apply` interface.
/// All external callers (CLI, web host, tests) drive the simulation through
/// [`GameEngine::apply`] and read state through [`GameEngine::query`].
#[derive(Debug)]
pub struct GameEngine {
    /// In-memory live game state.
    pub state: GameState,
    /// Turn processor responsible for cadence bookkeeping.
    processor: TurnProcessor,
}

impl GameEngine {
    /// Create a new engine with the default seed.
    #[must_use]
    pub fn new() -> Self {
        Self::with_seed(DEFAULT_SEED)
    }

    /// Create a new engine with an explicit RNG seed.
    ///
    /// Use a fixed seed in tests to get deterministic results.
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            state: GameState::new(),
            processor: TurnProcessor::new(seed),
        }
    }

    /// Apply a [`Command`] and return the resulting [`Event`]s.
    ///
    /// This is the **only** mutation point from outside `outpost_core`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] when the command cannot be applied in the
    /// current engine state.
    pub fn apply(&mut self, cmd: &Command) -> Result<Vec<Event>, EngineError> {
        match cmd {
            Command::AdvanceColonySol => {
                let outcome = self.processor.advance(&mut self.state);
                let mut events = vec![Event::ColonySolAdvanced { sol: outcome.sol }];
                if outcome
                    .cadences_fired
                    .contains(&turn::TurnCadence::StrategicMonth)
                {
                    events.push(Event::StrategicMonthAdvanced {
                        month: outcome.month,
                    });
                }
                Ok(events)
            }

            Command::AdvanceStrategicMonth => {
                self.state.month += 1;
                Ok(vec![Event::StrategicMonthAdvanced {
                    month: self.state.month,
                }])
            }

            Command::FoundColony {
                name,
                starting_population,
            } => {
                if name.trim().is_empty() {
                    return Err(EngineError::InvalidArgument(
                        "colony name must not be empty".into(),
                    ));
                }
                let colony = colony::Colony::new(name.clone());
                let id = colony.id;
                self.state.add_colony(colony, *starting_population);
                Ok(vec![Event::ColonyFounded {
                    colony_id: id,
                    name: name.clone(),
                    starting_population: *starting_population,
                }])
            }

            Command::QueueConstruction {
                colony_id,
                building_type,
            } => {
                // Validate the colony exists.
                self.find_colony_index(*colony_id)?;
                if building_type.trim().is_empty() {
                    return Err(EngineError::InvalidArgument(
                        "building_type must not be empty".into(),
                    ));
                }
                Ok(vec![Event::ConstructionQueued {
                    colony_id: *colony_id,
                    building_type: building_type.clone(),
                }])
            }

            Command::AssignLabour {
                colony_id,
                slot,
                labour,
            } => {
                let idx = self.find_colony_index(*colony_id)?;
                let available = self.state.populations[idx].available_labour();
                if *labour > available {
                    return Err(EngineError::InvalidArgument(format!(
                        "requested {labour} labour but only {available} available"
                    )));
                }
                if slot.trim().is_empty() {
                    return Err(EngineError::InvalidArgument(
                        "slot name must not be empty".into(),
                    ));
                }
                Ok(vec![Event::LabourAssigned {
                    colony_id: *colony_id,
                    slot: slot.clone(),
                    labour: *labour,
                }])
            }
        }
    }

    /// Evaluate a read-only [`Query`] and return a [`QueryResult`].
    ///
    /// Queries never mutate game state.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] if the query references an entity that does not exist.
    pub fn query(&self, q: &Query) -> Result<QueryResult, EngineError> {
        match q {
            Query::CurrentSol => Ok(QueryResult::Counter(self.state.sol)),
            Query::CurrentMonth => Ok(QueryResult::Counter(self.state.month)),

            Query::ListColonies => {
                let summaries = self
                    .state
                    .colonies
                    .iter()
                    .zip(self.state.populations.iter())
                    .map(|(c, p)| ColonySummary {
                        id: c.id,
                        name: c.name.clone(),
                        population: p.count,
                    })
                    .collect();
                Ok(QueryResult::Colonies(summaries))
            }

            Query::ColonyStatus { colony_id } => {
                let idx = self.find_colony_index(*colony_id)?;
                let c = &self.state.colonies[idx];
                let p = &self.state.populations[idx];
                Ok(QueryResult::ColonyStatus(ColonyStatus {
                    id: c.id,
                    name: c.name.clone(),
                    population: p.count,
                    stability: p.stability,
                    available_labour: p.available_labour(),
                }))
            }

            Query::AvailableLabour { colony_id } => {
                let idx = self.find_colony_index(*colony_id)?;
                Ok(QueryResult::Labour(
                    self.state.populations[idx].available_labour(),
                ))
            }
        }
    }

    /// Return the current colony-sol counter.
    #[must_use]
    pub fn sol(&self) -> u64 {
        self.state.sol
    }

    /// Return the current strategic-month counter.
    #[must_use]
    pub fn month(&self) -> u64 {
        self.state.month
    }

    /// Find the index of a colony by ID, or return [`EngineError::ColonyNotFound`].
    fn find_colony_index(&self, id: ColonyId) -> Result<usize, EngineError> {
        self.state
            .colonies
            .iter()
            .position(|c| c.id == id)
            .ok_or(EngineError::ColonyNotFound(id))
    }
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── basic turn counters ──

    #[test]
    fn engine_starts_at_zero() {
        let engine = GameEngine::new();
        assert_eq!(engine.sol(), 0);
        assert_eq!(engine.month(), 0);
    }

    #[test]
    fn advance_colony_sol_increments_counter() {
        let mut engine = GameEngine::new();
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(engine.sol(), 1);
        assert!(matches!(events[0], Event::ColonySolAdvanced { sol: 1 }));
    }

    #[test]
    fn advance_strategic_month_increments_counter() {
        let mut engine = GameEngine::new();
        let events = engine.apply(&Command::AdvanceStrategicMonth).unwrap();
        assert_eq!(engine.month(), 1);
        assert!(matches!(
            events[0],
            Event::StrategicMonthAdvanced { month: 1 }
        ));
    }

    #[test]
    fn multiple_advances_accumulate() {
        let mut engine = GameEngine::new();
        for _ in 0..5 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
        }
        assert_eq!(engine.sol(), 5);
    }

    #[test]
    fn strategic_month_fires_automatically_after_30_sols() {
        let mut engine = GameEngine::with_seed(42);
        let mut month_events = 0usize;
        for _ in 0..30 {
            let events = engine.apply(&Command::AdvanceColonySol).unwrap();
            month_events += events
                .iter()
                .filter(|e| matches!(e, Event::StrategicMonthAdvanced { .. }))
                .count();
        }
        assert_eq!(engine.sol(), 30);
        assert_eq!(month_events, 1);
        assert_eq!(engine.month(), 1);
    }

    #[test]
    fn engine_is_deterministic_for_fixed_seed() {
        let run = || {
            let mut engine = GameEngine::with_seed(7777);
            for _ in 0..60 {
                engine.apply(&Command::AdvanceColonySol).unwrap();
            }
            (engine.sol(), engine.month())
        };
        assert_eq!(run(), run());
    }

    // ── FoundColony ──

    #[test]
    fn found_colony_adds_colony_and_emits_event() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Alpha Base".into(),
                starting_population: 150,
            })
            .unwrap();
        assert_eq!(events.len(), 1);
        let Event::ColonyFounded {
            colony_id,
            name,
            starting_population,
        } = &events[0]
        else {
            panic!("expected ColonyFounded");
        };
        assert_eq!(name, "Alpha Base");
        assert_eq!(*starting_population, 150);

        // Round-trip: query confirms the colony exists.
        let result = engine.query(&Query::ListColonies).unwrap();
        let QueryResult::Colonies(cols) = result else {
            panic!("expected Colonies");
        };
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, *colony_id);
        assert_eq!(cols[0].name, "Alpha Base");
        assert_eq!(cols[0].population, 150);
    }

    #[test]
    fn found_colony_empty_name_returns_error() {
        let mut engine = GameEngine::new();
        let err = engine
            .apply(&Command::FoundColony {
                name: "   ".into(),
                starting_population: 100,
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    // ── QueueConstruction ──

    #[test]
    fn queue_construction_unknown_colony_returns_error() {
        let mut engine = GameEngine::new();
        let fake_id = uuid::Uuid::new_v4();
        let err = engine
            .apply(&Command::QueueConstruction {
                colony_id: fake_id,
                building_type: "mine".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::ColonyNotFound(_)));
    }

    #[test]
    fn queue_construction_emits_event() {
        let mut engine = GameEngine::new();
        // Found a colony first.
        let events = engine
            .apply(&Command::FoundColony {
                name: "Beta Station".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let events = engine
            .apply(&Command::QueueConstruction {
                colony_id,
                building_type: "greenhouse".into(),
            })
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            Event::ConstructionQueued { colony_id: cid, building_type: bt }
            if *cid == colony_id && bt == "greenhouse"
        ));
    }

    #[test]
    fn queue_construction_empty_type_returns_error() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Gamma".into(),
                starting_population: 10,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let err = engine
            .apply(&Command::QueueConstruction {
                colony_id: *colony_id,
                building_type: "".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    // ── AssignLabour ──

    #[test]
    fn assign_labour_emits_event() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Delta".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let events = engine
            .apply(&Command::AssignLabour {
                colony_id,
                slot: "mining".into(),
                labour: 40,
            })
            .unwrap();
        assert!(matches!(
            &events[0],
            Event::LabourAssigned { colony_id: cid, slot, labour: 40 }
            if *cid == colony_id && slot == "mining"
        ));
    }

    #[test]
    fn assign_labour_exceeds_available_returns_error() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Epsilon".into(),
                starting_population: 10,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let err = engine
            .apply(&Command::AssignLabour {
                colony_id: *colony_id,
                slot: "farming".into(),
                labour: 9999,
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn assign_labour_unknown_colony_returns_error() {
        let mut engine = GameEngine::new();
        let err = engine
            .apply(&Command::AssignLabour {
                colony_id: uuid::Uuid::new_v4(),
                slot: "mining".into(),
                labour: 5,
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::ColonyNotFound(_)));
    }

    // ── Query ──

    #[test]
    fn query_current_sol_and_month() {
        let mut engine = GameEngine::new();
        engine.apply(&Command::AdvanceColonySol).unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();

        let QueryResult::Counter(sol) = engine.query(&Query::CurrentSol).unwrap() else {
            panic!()
        };
        assert_eq!(sol, 2);

        let QueryResult::Counter(month) = engine.query(&Query::CurrentMonth).unwrap() else {
            panic!()
        };
        assert_eq!(month, 0);
    }

    #[test]
    fn query_colony_status() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Zeta".into(),
                starting_population: 200,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let result = engine.query(&Query::ColonyStatus { colony_id }).unwrap();
        let QueryResult::ColonyStatus(status) = result else {
            panic!()
        };
        assert_eq!(status.id, colony_id);
        assert_eq!(status.name, "Zeta");
        assert_eq!(status.population, 200);
        assert!((status.stability - 1.0).abs() < f64::EPSILON);
        assert_eq!(status.available_labour, 200);
    }

    #[test]
    fn query_colony_status_unknown_colony_returns_error() {
        let engine = GameEngine::new();
        let err = engine
            .query(&Query::ColonyStatus {
                colony_id: uuid::Uuid::new_v4(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::ColonyNotFound(_)));
    }

    #[test]
    fn query_available_labour() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Eta".into(),
                starting_population: 80,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let QueryResult::Labour(labour) = engine
            .query(&Query::AvailableLabour {
                colony_id: *colony_id,
            })
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(labour, 80);
    }

    // ── full-turn integration (Done-when bullet) ──

    #[test]
    fn full_turn_driven_purely_through_apply_and_query() {
        // This test satisfies the "Done when" requirement:
        // A test drives a full turn purely through apply() calls and Query reads
        // — no direct struct mutation from outside the engine.
        let mut engine = GameEngine::with_seed(99);

        // 1. Found a colony via apply().
        let found_events = engine
            .apply(&Command::FoundColony {
                name: "Outpost Prime".into(),
                starting_population: 120,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &found_events[0] else {
            panic!("expected ColonyFounded event")
        };
        let colony_id = *colony_id;

        // 2. Read state via Query — no struct access.
        let QueryResult::Counter(sol_before) = engine.query(&Query::CurrentSol).unwrap() else {
            panic!()
        };
        assert_eq!(sol_before, 0);

        let QueryResult::Labour(labour) =
            engine.query(&Query::AvailableLabour { colony_id }).unwrap()
        else {
            panic!()
        };
        assert_eq!(labour, 120);

        // 3. Queue construction via apply().
        engine
            .apply(&Command::QueueConstruction {
                colony_id,
                building_type: "solar_array".into(),
            })
            .unwrap();

        // 4. Assign labour via apply().
        engine
            .apply(&Command::AssignLabour {
                colony_id,
                slot: "power_grid".into(),
                labour: 30,
            })
            .unwrap();

        // 5. Advance one colony-sol turn via apply().
        let advance_events = engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(matches!(
            advance_events[0],
            Event::ColonySolAdvanced { sol: 1 }
        ));

        // 6. Verify new turn state via Query.
        let QueryResult::Counter(sol_after) = engine.query(&Query::CurrentSol).unwrap() else {
            panic!()
        };
        assert_eq!(sol_after, 1);

        let QueryResult::ColonyStatus(status) =
            engine.query(&Query::ColonyStatus { colony_id }).unwrap()
        else {
            panic!()
        };
        assert_eq!(status.name, "Outpost Prime");
        assert_eq!(status.population, 120);

        // 7. Confirm all colonies are listed.
        let QueryResult::Colonies(cols) = engine.query(&Query::ListColonies).unwrap() else {
            panic!()
        };
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, colony_id);
    }

    // ── command round-trip (serde) ──

    #[test]
    fn command_round_trip_serde() {
        let cmds = vec![
            Command::AdvanceColonySol,
            Command::AdvanceStrategicMonth,
            Command::FoundColony {
                name: "RT Colony".into(),
                starting_population: 50,
            },
            Command::QueueConstruction {
                colony_id: uuid::Uuid::new_v4(),
                building_type: "barracks".into(),
            },
            Command::AssignLabour {
                colony_id: uuid::Uuid::new_v4(),
                slot: "defence".into(),
                labour: 5,
            },
        ];
        for cmd in &cmds {
            let json = serde_json::to_string(cmd).expect("serialize");
            let back: Command = serde_json::from_str(&json).expect("deserialize");
            // Re-serialize and compare to confirm round-trip stability.
            let json2 = serde_json::to_string(&back).expect("re-serialize");
            assert_eq!(json, json2, "round-trip mismatch for {cmd:?}");
        }
    }

    #[test]
    fn query_round_trip_serde() {
        let id = uuid::Uuid::new_v4();
        let queries = vec![
            Query::CurrentSol,
            Query::CurrentMonth,
            Query::ListColonies,
            Query::ColonyStatus { colony_id: id },
            Query::AvailableLabour { colony_id: id },
        ];
        for q in &queries {
            let json = serde_json::to_string(q).expect("serialize");
            let back: Query = serde_json::from_str(&json).expect("deserialize");
            let json2 = serde_json::to_string(&back).expect("re-serialize");
            assert_eq!(json, json2);
        }
    }
}
