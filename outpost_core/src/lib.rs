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
//! - [`needs`]      — per-turn needs resolution and stability dynamics

#![warn(missing_docs)]

pub mod balance;
pub mod colony;
pub mod content;
pub mod directive;
pub mod interrupt;
pub mod map;
pub mod modifier;
pub mod needs;
pub mod population;
pub mod predicate;
pub mod research;
pub mod snapshot;
pub mod tech;
pub mod turn;

use thiserror::Error;

use colony::{ColonyId, ProjectId};
use directive::DirectiveId;
use interrupt::{AdvanceResult, Interrupt, InterruptSource, Tier};
use needs::{apply_needs_check, apply_population_dynamics};
use turn::{GameState, TurnProcessor};

/// Stability floor below which a predictive warning is emitted.
const STABILITY_CRISIS_FLOOR: f32 = 0.2;
/// Default ETA horizon (in turns) for predictive warnings.
const PREDICTIVE_WARNING_ETA: u32 = 10;

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
    /// Rejects if the colony lacks sufficient build slots.
    QueueConstruction {
        /// Target colony.
        colony_id: ColonyId,
        /// Content-pack key identifying the building type to queue.
        building_type: String,
        /// Number of build slots the building will consume.
        slot_cost: u32,
        /// Labor units consumed from the colony pool each construction turn.
        labor_per_turn: u32,
        /// Commodity costs for construction (commodity id, quantity pairs).
        construction_cost: Vec<(String, f64)>,
        /// Number of colony-sol turns required to complete construction.
        construction_turns: u32,
    },
    /// Cancel a queued construction project and receive a 50 % partial refund.
    CancelConstruction {
        /// Target colony.
        colony_id: ColonyId,
        /// Identifier of the construction project to cancel.
        project_id: ProjectId,
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
    /// Register or replace a directive for a colony.
    SetDirective {
        /// The directive to register.
        directive: Box<directive::Directive>,
    },
    /// Remove a directive by its stable ID.
    RemoveDirective {
        /// ID of the directive to remove.
        directive_id: directive::DirectiveId,
    },
    /// Enable or disable manual override for a colony.
    ///
    /// When `enabled` is `true`, directive evaluation is suppressed for the
    /// colony.  When `false`, automation resumes.
    SetManualOverride {
        /// Target colony.
        colony_id: ColonyId,
        /// `true` to enable manual override; `false` to resume automation.
        enabled: bool,
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
    /// Return the current system-wide accumulated research total.
    SystemResearchTotal,
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
    /// Available labour (f32 since population count is now fractional).
    Labour(f32),
    /// Total accumulated research in the system-wide pool.
    ResearchTotal(f32),
}

/// Lightweight colony summary returned by [`Query::ListColonies`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColonySummary {
    /// Colony stable identifier.
    pub id: ColonyId,
    /// Colony display name.
    pub name: String,
    /// Current colonist head-count (fractional for growth modelling).
    pub population: f32,
}

/// Detailed colony status returned by [`Query::ColonyStatus`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColonyStatus {
    /// Colony stable identifier.
    pub id: ColonyId,
    /// Colony display name.
    pub name: String,
    /// Current colonist head-count (fractional for growth modelling).
    pub population: f32,
    /// Stability scalar in `[0.0, 1.0]`.
    pub stability: f32,
    /// Labour units available this turn.
    pub available_labour: f32,
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
        /// Stable identifier for the new project (for tracking / cancellation).
        project_id: ProjectId,
    },
    /// A construction project was cancelled; refunded commodities are listed.
    ConstructionCancelled {
        /// Target colony.
        colony_id: ColonyId,
        /// Identifier of the cancelled project.
        project_id: ProjectId,
        /// Commodities returned to the colony pool (50 % of spent costs).
        refund: Vec<(String, f64)>,
    },
    /// A construction project completed and the building became operational.
    BuildingConstructed {
        /// Colony where construction finished.
        colony_id: ColonyId,
        /// Content-pack key of the completed building.
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
    /// Needs resolution completed for a colony this sol.
    NeedsResolved {
        /// Colony where needs were checked.
        colony_id: ColonyId,
        /// Weighted composite satisfaction score, in `[0.0, 1.0]`.
        composite_satisfaction: f32,
        /// Stability change applied this sol.
        stability_delta: f32,
        /// Population change applied this sol (positive = growth, negative = loss).
        population_delta: f32,
    },
    /// Research was produced by a colony and drained into the system-wide pool.
    ResearchProduced {
        /// Colony that contributed research this sol.
        colony_id: colony::ColonyId,
        /// Amount of research drained from the colony pool into the system pool.
        amount: f32,
    },
    /// A directive was registered (or replaced) for a colony.
    DirectiveSet {
        /// Colony the directive governs.
        colony_id: ColonyId,
        /// Stable identifier of the directive.
        directive_id: DirectiveId,
    },
    /// A directive was removed.
    DirectiveRemoved {
        /// Stable identifier of the removed directive.
        directive_id: DirectiveId,
    },
    /// Manual override state changed for a colony.
    ManualOverrideChanged {
        /// Affected colony.
        colony_id: ColonyId,
        /// New override state.
        enabled: bool,
    },
    /// A directive fired its action this turn.
    DirectiveFired {
        /// Colony on which the directive fired.
        colony_id: ColonyId,
        /// Identifier of the directive that fired.
        directive_id: DirectiveId,
    },
    /// A building ran at less than full capacity due to a resource shortfall.
    ProductionShortfall {
        /// Colony where the shortfall occurred.
        colony_id: ColonyId,
        /// Building type that was affected.
        building_type: String,
        /// Effective production scale applied this turn, in `[0.0, 1.0]`.
        scale: f64,
        /// Category of shortfall that caused the reduction.
        reason: colony::ShortfallReason,
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
    /// The colony does not have enough build slots for the requested construction.
    #[error("slot capacity exceeded: need {needed} slots but only {available} available")]
    SlotCapacityExceeded {
        /// Slots required by the new building.
        needed: u32,
        /// Slots actually available in the colony.
        available: u32,
    },
    /// The referenced construction project does not exist.
    #[error("project not found: {0}")]
    ProjectNotFound(ProjectId),
    /// The referenced directive does not exist.
    #[error("directive not found: {0}")]
    DirectiveNotFound(directive::DirectiveId),
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
    #[allow(clippy::too_many_lines)]
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
                // ── Step 1: Construction ────────────────────────────────────
                for colony in &mut self.state.colonies {
                    // Consume labor for the active project.
                    if let Some(active) = colony.build_queue.projects.first() {
                        let labor = f64::from(active.labor_per_turn);
                        colony.pool.withdraw("labor", labor);
                    }
                    if let Some(completed) = colony.build_queue.tick_active() {
                        let building_type = completed.building_type.clone();
                        colony.buildings.push(colony::PlacedBuilding::new(
                            &building_type,
                            completed.slot_cost,
                        ));
                        events.push(Event::BuildingConstructed {
                            colony_id: colony.id,
                            building_type,
                        });
                    }
                }

                // ── Step 2: Needs resolution ────────────────────────────────
                // Consume bulk commodities, update stability and population.
                if let Some(config) = self.state.needs_config.clone() {
                    for (colony, pop) in self
                        .state
                        .colonies
                        .iter_mut()
                        .zip(self.state.populations.iter_mut())
                    {
                        let population_count = f64::from(pop.count);
                        let report = apply_needs_check(&mut colony.pool, population_count, &config);

                        let housing_sat = report
                            .needs
                            .iter()
                            .find(|n| n.commodity_id == "housing")
                            .map_or(1.0, |n| n.satisfaction);

                        let pop_delta = apply_population_dynamics(
                            population_count,
                            pop.stability,
                            housing_sat,
                            &config,
                        );

                        // Apply stability and population changes.
                        pop.stability = (pop.stability + report.stability_delta).clamp(0.0, 1.0);
                        pop.count = (pop.count + pop_delta).max(0.0);

                        events.push(Event::NeedsResolved {
                            colony_id: colony.id,
                            composite_satisfaction: report.composite_satisfaction,
                            stability_delta: report.stability_delta,
                            population_delta: pop_delta,
                        });
                    }
                }

                // ── Step 3: Production ──────────────────────────────────────
                // Only runs when a content registry is loaded.  Shortfalls are
                // emitted as `ProductionShortfall` events; no crash on partial.
                if let Some(registry) = &self.state.registry.clone() {
                    for (colony, pop) in self
                        .state
                        .colonies
                        .iter_mut()
                        .zip(self.state.populations.iter())
                    {
                        let labor: f32 = pop.available_labor();
                        let placed: Vec<(String, u32)> = colony
                            .buildings
                            .iter()
                            .map(|b| (b.building_type.clone(), b.slot_cost))
                            .collect();
                        colony.pool.reset_deltas();
                        let prod_outcome =
                            colony::process_production(&mut colony.pool, &placed, labor, registry);
                        // Emit events for every shortfall so callers can log or react.
                        for result in &prod_outcome.building_results {
                            for shortfall in &result.shortfalls {
                                events.push(Event::ProductionShortfall {
                                    colony_id: colony.id,
                                    building_type: result.building_type.clone(),
                                    scale: result.scale,
                                    reason: shortfall.reason.clone(),
                                });
                            }
                        }
                    }
                }

                // ── Step 4: Research aggregation ────────────────────────────
                // Drain `research` from every colony pool into the system pool.
                // This happens after production so that labs which ran this turn
                // contribute their output immediately.
                for colony in &mut self.state.colonies {
                    let produced = colony.pool.amount("research");
                    if produced > 0.0 {
                        colony.pool.withdraw("research", produced);
                        #[allow(clippy::cast_possible_truncation)]
                        let produced_f32 = produced as f32;
                        self.state.research_pool.deposit(produced_f32);
                        events.push(Event::ResearchProduced {
                            colony_id: colony.id,
                            amount: produced_f32,
                        });
                    }
                }

                // ── Step 4b: Stability tracking ───────────────────────────
                // Record a stability sample per colony so predictive warnings
                // can extrapolate trajectory without full forward simulation.
                let colony_ids_for_tracking: Vec<ColonyId> =
                    self.state.colonies.iter().map(|c| c.id).collect();
                for (i, colony_id) in colony_ids_for_tracking.iter().enumerate() {
                    if let Some(pop) = self.state.populations.get(i) {
                        let stability = pop.stability;
                        self.state
                            .stability_trackers
                            .entry(*colony_id)
                            .or_default()
                            .push(stability);
                    }
                }

                // ── Step 5: Directive evaluation ──────────────────────────
                // Two-pass: collect (id, col_id, action) while holding immutable
                // borrows, then fire via self.apply (needs &mut self).
                let mut to_fire: Vec<(DirectiveId, ColonyId, Command)> = Vec::new();
                {
                    let colony_ids: Vec<ColonyId> =
                        self.state.colonies.iter().map(|c| c.id).collect();
                    for colony_id in colony_ids {
                        if self.state.directive_store.is_manual_override(colony_id) {
                            continue;
                        }
                        let Some(idx) = self.state.colonies.iter().position(|c| c.id == colony_id)
                        else {
                            continue;
                        };
                        let pop = &self.state.populations[idx];
                        let ctx = predicate::PredicateContext {
                            colony_id,
                            population: pop.count,
                            stability: pop.stability,
                            available_labour: pop.available_labor(),
                            system_research: self.state.research_pool.total(),
                            sol: self.state.sol,
                            month: self.state.month,
                        };
                        if let Some((dir_id, action)) = self
                            .state
                            .directive_store
                            .directives
                            .iter()
                            .filter(|d| d.colony_id == colony_id)
                            .filter(|d| d.predicate.evaluate(&ctx))
                            .max_by_key(|d| d.priority)
                            .map(|d| (d.id, d.action.clone()))
                        {
                            to_fire.push((dir_id, colony_id, action));
                        }
                    }
                }
                for (directive_id, colony_id, action) in to_fire {
                    events.push(Event::DirectiveFired {
                        colony_id,
                        directive_id,
                    });
                    let fired_events = self.apply(&action)?;
                    events.extend(fired_events);
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
                slot_cost,
                labor_per_turn,
                construction_cost,
                construction_turns,
            } => {
                if building_type.trim().is_empty() {
                    return Err(EngineError::InvalidArgument(
                        "building_type must not be empty".into(),
                    ));
                }
                let idx = self.find_colony_index(*colony_id)?;
                let available = self.state.colonies[idx].slots_available();
                if *slot_cost > available {
                    return Err(EngineError::SlotCapacityExceeded {
                        needed: *slot_cost,
                        available,
                    });
                }
                let project = colony::ConstructionProject::new(
                    building_type.clone(),
                    *slot_cost,
                    *labor_per_turn,
                    construction_cost.clone(),
                    *construction_turns,
                );
                let project_id = project.id;
                self.state.colonies[idx].build_queue.enqueue(project);
                Ok(vec![Event::ConstructionQueued {
                    colony_id: *colony_id,
                    building_type: building_type.clone(),
                    project_id,
                }])
            }

            Command::CancelConstruction {
                colony_id,
                project_id,
            } => {
                let idx = self.find_colony_index(*colony_id)?;
                let colony = &mut self.state.colonies[idx];
                let project = colony
                    .build_queue
                    .cancel(*project_id)
                    .ok_or(EngineError::ProjectNotFound(*project_id))?;
                let refund = project.cancel_refund();
                // Return refunded commodities to the pool.
                for (commodity_id, qty) in &refund {
                    colony.pool.deposit(commodity_id, *qty);
                }
                Ok(vec![Event::ConstructionCancelled {
                    colony_id: *colony_id,
                    project_id: *project_id,
                    refund,
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

            Command::SetDirective { directive } => {
                let colony_id = directive.colony_id;
                self.find_colony_index(colony_id)?;
                let directive_id = directive.id;
                let d = *directive.clone();
                if let Some(existing) = self
                    .state
                    .directive_store
                    .directives
                    .iter_mut()
                    .find(|x| x.id == d.id)
                {
                    *existing = d;
                } else {
                    self.state.directive_store.directives.push(d);
                }
                Ok(vec![Event::DirectiveSet {
                    colony_id,
                    directive_id,
                }])
            }

            Command::RemoveDirective { directive_id } => {
                if !self
                    .state
                    .directive_store
                    .directives
                    .iter()
                    .any(|d| d.id == *directive_id)
                {
                    return Err(EngineError::DirectiveNotFound(*directive_id));
                }
                self.state
                    .directive_store
                    .directives
                    .retain(|d| d.id != *directive_id);
                Ok(vec![Event::DirectiveRemoved {
                    directive_id: *directive_id,
                }])
            }

            Command::SetManualOverride { colony_id, enabled } => {
                self.find_colony_index(*colony_id)?;
                if *enabled {
                    self.state
                        .directive_store
                        .manual_override
                        .insert(*colony_id);
                } else {
                    self.state.directive_store.manual_override.remove(colony_id);
                }
                Ok(vec![Event::ManualOverrideChanged {
                    colony_id: *colony_id,
                    enabled: *enabled,
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
                    available_labour: p.available_labor(),
                }))
            }

            Query::AvailableLabour { colony_id } => {
                let idx = self.find_colony_index(*colony_id)?;
                Ok(QueryResult::Labour(
                    self.state.populations[idx].available_labor(),
                ))
            }

            Query::SystemResearchTotal => {
                Ok(QueryResult::ResearchTotal(self.state.research_pool.total()))
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

    /// Advance up to `n` colony-sol turns, halting at the first interrupt whose
    /// tier is ≥ `threshold`.
    ///
    /// Each turn:
    /// 1. Applies [`Command::AdvanceColonySol`].
    /// 2. Collects construction-complete and predictive-stability-warning interrupts.
    /// 3. Returns [`AdvanceResult::Halted`] on the first interrupt that meets or
    ///    exceeds `threshold`, carrying accumulated `Notable`/`Ambient` interrupts
    ///    in the digest.
    ///
    /// A clean run returns [`AdvanceResult::Completed`] with the full digest.
    /// Predictive warnings fire *before* the stability crisis, not on it.
    ///
    /// # Errors
    ///
    /// Propagates any [`EngineError`] raised by individual `AdvanceColonySol` turns.
    pub fn advance_until_interrupted(
        &mut self,
        n: u32,
        threshold: Tier,
    ) -> Result<AdvanceResult, EngineError> {
        let mut digest: Vec<Interrupt> = Vec::new();

        for _ in 0..n {
            let events = self.apply(&Command::AdvanceColonySol)?;

            // Update per-colony stability trackers after each sol.
            let colony_ids: Vec<_> = self.state.colonies.iter().map(|c| c.id).collect();
            for (i, colony_id) in colony_ids.iter().enumerate() {
                let stability = self.state.populations[i].stability;
                self.state
                    .stability_trackers
                    .entry(*colony_id)
                    .or_default()
                    .push(stability);
            }

            let turn_interrupts = self.collect_turn_interrupts(&events);

            for irq in turn_interrupts {
                if irq.tier >= threshold {
                    return Ok(AdvanceResult::Halted {
                        at_turn: self.state.sol,
                        interrupt: irq,
                        digest,
                    });
                }
                // Below threshold: accumulate in digest.
                digest.push(irq);
            }
        }

        Ok(AdvanceResult::Completed {
            turns_advanced: n,
            digest,
        })
    }

    /// Collect interrupts generated by the events from one sol turn plus the
    /// current stability trajectory for all colonies.
    ///
    /// Returns `ConstructionComplete` (`Notable`) for each finished building,
    /// and `PredictiveWarning` (`Urgent`) for each colony whose stability is
    /// trending toward crisis within [`PREDICTIVE_WARNING_ETA`] turns.
    fn collect_turn_interrupts(&self, events: &[Event]) -> Vec<Interrupt> {
        let mut interrupts: Vec<Interrupt> = Vec::new();

        // Construction completions → Notable.
        for ev in events {
            if let Event::BuildingConstructed {
                colony_id,
                building_type,
            } = ev
            {
                interrupts.push(Interrupt::new(
                    Tier::Notable,
                    InterruptSource::ConstructionComplete,
                    Some(*colony_id),
                    format!("{building_type} construction complete"),
                ));
            }
        }

        // Predictive stability warnings → Urgent.
        for (colony, pop) in self
            .state
            .colonies
            .iter()
            .zip(self.state.populations.iter())
        {
            if let Some(tracker) = self.state.stability_trackers.get(&colony.id) {
                if let Some(eta) = tracker.eta_to_floor(STABILITY_CRISIS_FLOOR) {
                    if eta <= PREDICTIVE_WARNING_ETA {
                        interrupts.push(Interrupt::new(
                            Tier::Urgent,
                            InterruptSource::PredictiveWarning {
                                quantity: pop.stability,
                                eta_turns: eta,
                            },
                            Some(colony.id),
                            format!(
                                "Colony '{}': stability declining — crisis in ~{eta} turns \
                                 (current: {:.2})",
                                colony.name, pop.stability
                            ),
                        ));
                    }
                }
            }
        }

        interrupts
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
        assert!((cols[0].population - 150.0).abs() < 1.0);
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

    fn queue_cmd(colony_id: ColonyId, building_type: &str, slot_cost: u32) -> Command {
        Command::QueueConstruction {
            colony_id,
            building_type: building_type.into(),
            slot_cost,
            labor_per_turn: 5,
            construction_cost: vec![],
            construction_turns: 2,
        }
    }

    #[test]
    fn queue_construction_unknown_colony_returns_error() {
        let mut engine = GameEngine::new();
        let fake_id = uuid::Uuid::new_v4();
        let err = engine.apply(&queue_cmd(fake_id, "mine", 1)).unwrap_err();
        assert!(matches!(err, EngineError::ColonyNotFound(_)));
    }

    #[test]
    fn queue_construction_emits_event() {
        let mut engine = GameEngine::new();
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
            .apply(&queue_cmd(colony_id, "greenhouse", 1))
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            Event::ConstructionQueued { colony_id: cid, building_type: bt, .. }
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
                slot_cost: 1,
                labor_per_turn: 0,
                construction_cost: vec![],
                construction_turns: 1,
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    // ── Building model tests (Done-when bullets for issue #13) ──

    #[test]
    fn queue_construction_rejected_when_slots_full() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Slot Test".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Fill all 5 base slots.
        for _ in 0..colony::BASE_SLOT_CAPACITY {
            engine.apply(&queue_cmd(colony_id, "mine", 1)).unwrap();
        }
        // One more should be rejected.
        let err = engine.apply(&queue_cmd(colony_id, "mine", 1)).unwrap_err();
        assert!(
            matches!(err, EngineError::SlotCapacityExceeded { .. }),
            "expected SlotCapacityExceeded, got {err:?}"
        );
    }

    #[test]
    fn construction_completes_turn_by_turn() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Build Test".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Queue a 3-turn build.
        engine
            .apply(&Command::QueueConstruction {
                colony_id,
                building_type: "greenhouse".into(),
                slot_cost: 1,
                labor_per_turn: 0,
                construction_cost: vec![],
                construction_turns: 3,
            })
            .unwrap();

        // Turns 1 and 2: not yet complete.
        for _ in 0..2 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            assert!(!evs
                .iter()
                .any(|e| matches!(e, Event::BuildingConstructed { .. })));
        }
        // Turn 3: should complete.
        let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            evs.iter().any(|e| matches!(
                e,
                Event::BuildingConstructed { colony_id: cid, building_type: bt }
                if *cid == colony_id && bt == "greenhouse"
            )),
            "expected BuildingConstructed event"
        );

        // Colony should now have the building placed.
        let idx = engine.find_colony_index(colony_id).unwrap();
        assert_eq!(engine.state.colonies[idx].buildings.len(), 1);
        assert_eq!(
            engine.state.colonies[idx].buildings[0].building_type,
            "greenhouse"
        );
    }

    #[test]
    fn cancel_construction_returns_50_pct_refund() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Cancel Test".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Queue a 4-turn build with steel cost.
        let cost = vec![("steel".to_string(), 100.0)];
        let evs = engine
            .apply(&Command::QueueConstruction {
                colony_id,
                building_type: "smelter".into(),
                slot_cost: 1,
                labor_per_turn: 0,
                construction_cost: cost,
                construction_turns: 4,
            })
            .unwrap();
        let Event::ConstructionQueued { project_id, .. } = &evs[0] else {
            panic!()
        };
        let project_id = *project_id;

        // Advance 2 turns (50 % done).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap();

        // Cancel — expect 25 % refund (50 % spent × 50 % back).
        let evs = engine
            .apply(&Command::CancelConstruction {
                colony_id,
                project_id,
            })
            .unwrap();
        let Event::ConstructionCancelled { refund, .. } = &evs[0] else {
            panic!()
        };
        let steel_refund = refund.iter().find(|(id, _)| id == "steel").unwrap();
        assert!(
            (steel_refund.1 - 25.0).abs() < 1e-9,
            "expected 25.0 steel refund, got {}",
            steel_refund.1
        );
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
        assert!((status.population - 200.0).abs() < 1.0);
        assert!((status.stability - 1.0).abs() < 0.01);
        assert!((status.available_labour - 200.0).abs() < 1.0);
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
        assert!((labour - 80.0).abs() < 1.0, "labour was {labour}");
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
        assert!((labour - 120.0).abs() < 1.0, "labour was {labour}");

        // 3. Queue construction via apply().
        engine
            .apply(&queue_cmd(colony_id, "solar_array", 1))
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
        assert!((status.population - 120.0).abs() < 1.0);

        // 7. Confirm all colonies are listed.
        let QueryResult::Colonies(cols) = engine.query(&Query::ListColonies).unwrap() else {
            panic!()
        };
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, colony_id);
    }

    // ── Research: Done-when tests for issue #17 ──────────────────────────────

    /// Build a minimal `ContentRegistry` with `research_lab` and the `conduct_research` recipe.
    fn research_registry() -> crate::content::ContentRegistry {
        use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};
        let mut reg = crate::content::ContentRegistry::default();

        // A trivial power source so the research lab doesn't brown out.
        reg.insert_building(BuildingDef {
            id: "solar_array".into(),
            name: "Solar Array".into(),
            description: String::new(),
            category: BuildingCategory::Power,
            construction_cost: vec![],
            power_delta: -100.0,
            worker_slots: 0,
            labor_required: 0,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
        });

        // Research lab: consumes 1 water, produces 5 research per sol.
        reg.insert_building(BuildingDef {
            id: "research_lab".into(),
            name: "Research Lab".into(),
            description: String::new(),
            category: BuildingCategory::Research,
            construction_cost: vec![],
            power_delta: 8.0, // draws 8 kW
            worker_slots: 3,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 4,
            tech_prerequisite: None,
        });

        reg.insert_building(BuildingDef {
            id: "water_source".into(),
            name: "Water Source".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 0,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
        });

        reg.insert_commodity(crate::content::types::CommodityDef {
            id: "water".into(),
            name: "Water".into(),
            description: String::new(),
            category: "consumable".into(),
            phase: crate::content::types::Phase::Liquid,
            base_value: 5.0,
            tradeable: true,
            tier: crate::content::types::CommodityTier::Basic,
            weight: 1.0,
        });

        reg.insert_commodity(crate::content::types::CommodityDef {
            id: "research".into(),
            name: "Research".into(),
            description: String::new(),
            category: "virtual".into(),
            phase: crate::content::types::Phase::Solid,
            base_value: 0.0,
            tradeable: false,
            tier: crate::content::types::CommodityTier::Advanced,
            weight: 0.0,
        });

        reg.insert_recipe(RecipeDef {
            id: "conduct_research".into(),
            name: "Conduct Research".into(),
            building: "research_lab".into(),
            inputs: vec![Ingredient {
                id: "water".into(),
                quantity: 1.0,
            }],
            outputs: vec![Ingredient {
                id: "research".into(),
                quantity: 5.0,
            }],
            cycle_sols: 1,
            power_draw: 8.0,
        });

        reg
    }

    /// Found a colony, set up the registry, seed water, place a research_lab + solar_array.
    fn setup_science_colony(engine: &mut GameEngine) -> colony::ColonyId {
        let events = engine
            .apply(&Command::FoundColony {
                name: "Science Base".into(),
                starting_population: 200,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        engine.state.registry = Some(research_registry());

        // Seed water so the recipe can run.
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.colonies[idx].pool.deposit("water", 1000.0);

        // Place buildings directly (bypasses construction queue for test simplicity).
        engine.state.colonies[idx]
            .buildings
            .push(colony::PlacedBuilding::new("solar_array", 1));
        engine.state.colonies[idx]
            .buildings
            .push(colony::PlacedBuilding::new("research_lab", 1));

        colony_id
    }

    #[test]
    fn research_lab_produces_research_each_turn() {
        let mut engine = GameEngine::with_seed(42);
        let colony_id = setup_science_colony(&mut engine);

        let events = engine.apply(&Command::AdvanceColonySol).unwrap();

        // A ResearchProduced event should have been emitted.
        let research_event = events.iter().find(
            |e| matches!(e, Event::ResearchProduced { colony_id: cid, .. } if *cid == colony_id),
        );
        assert!(research_event.is_some(), "expected ResearchProduced event");

        let Event::ResearchProduced { amount, .. } = research_event.unwrap() else {
            panic!()
        };
        assert!(
            *amount > 0.0,
            "research amount should be positive, got {amount}"
        );

        // After draining, colony's research pool should be 0.
        let idx = engine.find_colony_index(colony_id).unwrap();
        assert!(
            engine.state.colonies[idx].pool.amount("research") < 1e-6,
            "colony research pool should be drained after aggregation"
        );

        // System pool should have the same amount.
        let QueryResult::ResearchTotal(total) = engine.query(&Query::SystemResearchTotal).unwrap()
        else {
            panic!()
        };
        assert!(
            (total - amount).abs() < 1e-4,
            "system pool {total} should match produced {amount}"
        );
    }

    #[test]
    fn research_drains_from_colony_into_system_pool_correctly() {
        let mut engine = GameEngine::with_seed(1);
        let _colony_id = setup_science_colony(&mut engine);

        // Advance 3 turns; expect system pool to grow each time.
        let mut prev_total = 0.0f32;
        for turn in 1..=3 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
            let QueryResult::ResearchTotal(total) =
                engine.query(&Query::SystemResearchTotal).unwrap()
            else {
                panic!()
            };
            assert!(
                total > prev_total,
                "system research pool should grow each turn; turn {turn}: {total} <= {prev_total}"
            );
            prev_total = total;
        }
    }

    #[test]
    fn multi_colony_research_accumulates_into_system_pool() {
        let mut engine = GameEngine::with_seed(7);
        let reg = research_registry();
        engine.state.registry = Some(reg);

        // Found two colonies, each with a research lab.
        for name in &["Alpha Science", "Beta Science"] {
            let events = engine
                .apply(&Command::FoundColony {
                    name: (*name).into(),
                    starting_population: 200,
                })
                .unwrap();
            let Event::ColonyFounded { colony_id, .. } = &events[0] else {
                panic!()
            };
            let idx = engine.find_colony_index(*colony_id).unwrap();
            engine.state.colonies[idx].pool.deposit("water", 1000.0);
            engine.state.colonies[idx]
                .buildings
                .push(colony::PlacedBuilding::new("solar_array", 1));
            engine.state.colonies[idx]
                .buildings
                .push(colony::PlacedBuilding::new("research_lab", 1));
        }

        // Advance one turn — both colonies should contribute.
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        let research_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, Event::ResearchProduced { .. }))
            .collect();
        assert_eq!(
            research_events.len(),
            2,
            "expected ResearchProduced event from each of the 2 colonies"
        );

        let total_produced: f32 = research_events
            .iter()
            .map(|e| {
                let Event::ResearchProduced { amount, .. } = e else {
                    panic!()
                };
                *amount
            })
            .sum();

        let QueryResult::ResearchTotal(system_total) =
            engine.query(&Query::SystemResearchTotal).unwrap()
        else {
            panic!()
        };
        assert!(
            (system_total - total_produced).abs() < 1e-4,
            "system total {system_total} should equal sum of colony contributions {total_produced}"
        );
        assert!(system_total > 0.0, "system total must be positive");
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
                slot_cost: 1,
                labor_per_turn: 5,
                construction_cost: vec![],
                construction_turns: 2,
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

    // ── Directive system (Done-when tests for issue #22) ─────────────────────

    /// Helper: found a colony and return its `ColonyId`.
    fn found_colony_id(engine: &mut GameEngine, name: &str, pop: u64) -> colony::ColonyId {
        let events = engine
            .apply(&Command::FoundColony {
                name: name.into(),
                starting_population: pop,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded")
        };
        *colony_id
    }

    /// Done-when: a colony runs unattended across 50 turns under a directive;
    /// actions fire when the predicate is true.
    ///
    /// We set `Predicate::Always` so the directive fires every turn.
    /// After 50 advances, the labour-assignment directive must have fired at
    /// least once (all 50 times, actually).
    #[test]
    fn directive_fires_across_50_unattended_turns() {
        use crate::directive::Directive;
        use crate::predicate::Predicate;

        let mut engine = GameEngine::with_seed(1);
        let colony_id = found_colony_id(&mut engine, "Automation Base", 200);

        // Directive: always assign labour to "mining".
        let directive = Directive::new(
            colony_id,
            Predicate::Always,
            Command::AssignLabour {
                colony_id,
                slot: "mining".into(),
                labour: 10,
            },
            5,
        );
        engine
            .apply(&Command::SetDirective {
                directive: Box::new(directive),
            })
            .unwrap();

        // Advance 50 turns unattended; count how many DirectiveFired events occur.
        let mut fired_count = 0usize;
        for _ in 0..50 {
            let events = engine.apply(&Command::AdvanceColonySol).unwrap();
            fired_count += events
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        Event::DirectiveFired { colony_id: cid, .. } if *cid == colony_id
                    )
                })
                .count();
        }
        assert_eq!(
            fired_count, 50,
            "directive should fire every turn when predicate is Always; got {fired_count}"
        );
    }

    /// Done-when: directive fires on predicate (conditional).
    ///
    /// Use `Predicate::lt(Stability, 0.5)`.  With stability starting at 1.0 the
    /// directive must NOT fire on the first turn.  We then lower stability
    /// directly and verify it fires on the next turn.
    #[test]
    fn directive_fires_only_when_predicate_matches() {
        use crate::directive::Directive;
        use crate::predicate::{Metric, Predicate};

        let mut engine = GameEngine::with_seed(2);
        let colony_id = found_colony_id(&mut engine, "Conditional Base", 100);

        let directive = Directive::new(
            colony_id,
            Predicate::lt(Metric::Stability, 0.5),
            Command::AssignLabour {
                colony_id,
                slot: "repair".into(),
                labour: 5,
            },
            1,
        );
        engine
            .apply(&Command::SetDirective {
                directive: Box::new(directive),
            })
            .unwrap();

        // First turn: stability = 1.0, predicate is false — no firing.
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        let fired = events.iter().any(
            |e| matches!(e, Event::DirectiveFired { colony_id: cid, .. } if *cid == colony_id),
        );
        assert!(!fired, "directive must not fire when stability is 1.0");

        // Artificially drop stability below threshold.
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.populations[idx].stability = 0.3;

        // Next turn: predicate is now true — directive must fire.
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        let fired = events.iter().any(
            |e| matches!(e, Event::DirectiveFired { colony_id: cid, .. } if *cid == colony_id),
        );
        assert!(fired, "directive must fire when stability falls below 0.5");
    }

    /// Done-when: priority ordering — highest-priority directive fires first.
    #[test]
    fn directive_priority_ordering_highest_fires() {
        use crate::directive::Directive;
        use crate::predicate::Predicate;

        let mut engine = GameEngine::with_seed(3);
        let colony_id = found_colony_id(&mut engine, "Priority Base", 200);

        // Low priority: assigns 1 labour unit.
        let low = Directive::new(
            colony_id,
            Predicate::Always,
            Command::AssignLabour {
                colony_id,
                slot: "low_priority_slot".into(),
                labour: 1,
            },
            1,
        );
        // High priority: assigns 5 labour units.
        let high = Directive::new(
            colony_id,
            Predicate::Always,
            Command::AssignLabour {
                colony_id,
                slot: "high_priority_slot".into(),
                labour: 5,
            },
            10,
        );
        engine
            .apply(&Command::SetDirective {
                directive: Box::new(low),
            })
            .unwrap();
        engine
            .apply(&Command::SetDirective {
                directive: Box::new(high),
            })
            .unwrap();

        // Advance one turn; exactly one DirectiveFired event should occur.
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        let fired_events: Vec<_> = events
            .iter()
            .filter(
                |e| matches!(e, Event::DirectiveFired { colony_id: cid, .. } if *cid == colony_id),
            )
            .collect();
        assert_eq!(
            fired_events.len(),
            1,
            "only the highest-priority directive should fire"
        );

        // The LabourAssigned that follows must be for the high-priority slot.
        let labour_event = events
            .iter()
            .find(|e| matches!(e, Event::LabourAssigned { .. }));
        assert!(
            matches!(
                labour_event,
                Some(Event::LabourAssigned { slot, labour: 5, .. }) if slot == "high_priority_slot"
            ),
            "expected high-priority slot to receive labour; got {labour_event:?}"
        );
    }

    /// Done-when: manual override suppresses directive evaluation; re-enabling
    /// resumes automation.
    #[test]
    fn manual_override_suppresses_and_resumes_directives() {
        use crate::directive::Directive;
        use crate::predicate::Predicate;

        let mut engine = GameEngine::with_seed(4);
        let colony_id = found_colony_id(&mut engine, "Override Base", 200);

        let directive = Directive::new(
            colony_id,
            Predicate::Always,
            Command::AssignLabour {
                colony_id,
                slot: "auto_slot".into(),
                labour: 5,
            },
            5,
        );
        engine
            .apply(&Command::SetDirective {
                directive: Box::new(directive),
            })
            .unwrap();

        // Enable manual override.
        engine
            .apply(&Command::SetManualOverride {
                colony_id,
                enabled: true,
            })
            .unwrap();

        // Advance: directive must NOT fire while manual override is active.
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        let fired = events.iter().any(
            |e| matches!(e, Event::DirectiveFired { colony_id: cid, .. } if *cid == colony_id),
        );
        assert!(
            !fired,
            "directive must not fire while manual override is active"
        );

        // Disable manual override — automation resumes.
        engine
            .apply(&Command::SetManualOverride {
                colony_id,
                enabled: false,
            })
            .unwrap();

        // Advance: directive must fire now.
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        let fired = events.iter().any(
            |e| matches!(e, Event::DirectiveFired { colony_id: cid, .. } if *cid == colony_id),
        );
        assert!(
            fired,
            "directive must fire after manual override is disabled"
        );
    }

    /// Done-when: `set_directive` / `remove_directive` API.
    #[test]
    fn set_and_remove_directive_api() {
        use crate::directive::Directive;
        use crate::predicate::Predicate;

        let mut engine = GameEngine::with_seed(5);
        let colony_id = found_colony_id(&mut engine, "API Base", 100);

        let directive = Directive::new(
            colony_id,
            Predicate::Always,
            Command::AssignLabour {
                colony_id,
                slot: "managed_slot".into(),
                labour: 1,
            },
            1,
        );
        let directive_id = directive.id;

        // Register.
        let events = engine
            .apply(&Command::SetDirective {
                directive: Box::new(directive),
            })
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::DirectiveSet { directive_id: did, .. } if *did == directive_id)),
            "SetDirective must emit DirectiveSet event"
        );

        // Remove.
        let events = engine
            .apply(&Command::RemoveDirective { directive_id })
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::DirectiveRemoved { directive_id: did } if *did == directive_id)),
            "RemoveDirective must emit DirectiveRemoved event"
        );

        // After removal, directive must not fire.
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        let fired = events
            .iter()
            .any(|e| matches!(e, Event::DirectiveFired { .. }));
        assert!(!fired, "directive must not fire after removal");
    }

    // ── Interrupt tiers + advance_until_interrupted (Done-when for issue #23) ──

    /// Done-when: advancing halts on first Urgent interrupt when threshold = Urgent.
    #[test]
    fn advance_halts_on_urgent_interrupt() {
        use crate::interrupt::{AdvanceResult, InterruptSource, Tier};

        let mut engine = GameEngine::with_seed(42);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Crisis Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Pre-load a steep declining stability trajectory so the first sol fires Urgent.
        let tracker = engine
            .state
            .stability_trackers
            .entry(colony_id)
            .or_default();
        for s in [1.0f32, 0.7, 0.5, 0.3, 0.22] {
            tracker.push(s);
        }
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.populations[idx].stability = 0.22;

        let result = engine.advance_until_interrupted(20, Tier::Urgent).unwrap();

        assert!(
            matches!(result, AdvanceResult::Halted { .. }),
            "expected Halted, got Completed"
        );
        if let AdvanceResult::Halted { interrupt, .. } = result {
            assert_eq!(interrupt.tier, Tier::Urgent);
            assert!(matches!(
                interrupt.source,
                InterruptSource::PredictiveWarning { .. }
            ));
        }
    }

    /// Done-when: clean advance to N turns returns Completed with no halt.
    #[test]
    fn advance_completes_n_turns_without_interrupts() {
        use crate::interrupt::{AdvanceResult, Tier};

        let mut engine = GameEngine::with_seed(1);
        // No colonies → no interrupts → should complete all 5 turns.
        let result = engine.advance_until_interrupted(5, Tier::Urgent).unwrap();
        assert!(
            matches!(
                result,
                AdvanceResult::Completed {
                    turns_advanced: 5,
                    ..
                }
            ),
            "expected Completed{{5}}, got {result:?}"
        );
    }

    /// Done-when: predictive warning fires *before* the stability crisis, not on it.
    #[test]
    fn predictive_warning_fires_before_crisis() {
        use crate::interrupt::{AdvanceResult, InterruptSource, Tier};

        let mut engine = GameEngine::with_seed(7);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Warning Colony".into(),
                starting_population: 200,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Steep declining trajectory: will project crisis within 10 turns.
        let tracker = engine
            .state
            .stability_trackers
            .entry(colony_id)
            .or_default();
        for s in [0.9f32, 0.75, 0.6, 0.45, 0.3] {
            tracker.push(s);
        }
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.populations[idx].stability = 0.3;

        let result = engine.advance_until_interrupted(20, Tier::Urgent).unwrap();

        match result {
            AdvanceResult::Halted { interrupt, .. } => {
                // The warning fired; stability at warning time must be above the crisis floor.
                if let InterruptSource::PredictiveWarning { quantity, .. } = interrupt.source {
                    assert!(
                        quantity > STABILITY_CRISIS_FLOOR,
                        "predictive warning must fire before stability hits crisis floor \
                         ({STABILITY_CRISIS_FLOOR}); was {quantity}"
                    );
                }
            }
            AdvanceResult::Completed { .. } => {
                panic!("expected Halted (predictive warning) but got Completed");
            }
        }
    }

    /// Done-when: Blocking threshold lets Urgent interrupts accumulate in digest.
    #[test]
    fn urgent_interrupt_accumulates_in_digest_when_threshold_is_blocking() {
        use crate::interrupt::{AdvanceResult, Tier};

        let mut engine = GameEngine::with_seed(99);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Notable Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Steep declining stability: will emit Urgent warnings during advance.
        let tracker = engine
            .state
            .stability_trackers
            .entry(colony_id)
            .or_default();
        for s in [0.9f32, 0.7, 0.5, 0.3, 0.22] {
            tracker.push(s);
        }
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.populations[idx].stability = 0.22;

        // Blocking threshold: Urgent interrupts don't halt, they go to the digest.
        let result = engine.advance_until_interrupted(5, Tier::Blocking).unwrap();

        let digest = match result {
            AdvanceResult::Completed { digest, .. } => digest,
            AdvanceResult::Halted { .. } => panic!("expected Completed, got Halted"),
        };
        assert!(
            !digest.is_empty(),
            "digest should contain accumulated Urgent interrupts"
        );
    }
}
