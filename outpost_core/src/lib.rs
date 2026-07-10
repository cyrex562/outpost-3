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
pub mod difficulty;
pub mod directive;
pub mod expedition;
pub mod hazard;
pub mod interrupt;
pub mod map;
pub mod menace;
pub mod migration;
pub mod modifier;
pub mod needs;
pub mod orbital;
pub mod population;
pub mod predicate;
pub mod research;
pub mod snapshot;
pub mod system;
pub mod tech;
pub mod trade;
pub mod turn;
pub mod ui;
pub mod victory;

use thiserror::Error;

use colony::{ColonyId, ProjectId};
use directive::DirectiveId;
use interrupt::{AdvanceResult, Interrupt, InterruptSource, Tier};
use map::PlanetMap;
use migration::{
    compute_attractiveness, compute_auto_flows, resolve_arrival, AutoMigrationParams,
    ColonyAttractiveness, EmigrationGate, PendingMigration,
};
use needs::{apply_needs_check, apply_population_dynamics};
use orbital::{OrbitalError, OrbitalStation, SatelliteConstellation};
use trade::{SiteId, TradeOverride, TradeRoute};
use turn::{GameState, TurnProcessor};

/// Stability floor below which a predictive warning is emitted.
const STABILITY_CRISIS_FLOOR: f32 = 0.2;
/// Default ETA horizon (in turns) for predictive warnings.
const PREDICTIVE_WARNING_ETA: u32 = 10;
/// Population floor (absolute) below which a population-decline warning is emitted.
const POPULATION_CRISIS_FLOOR: f32 = 10.0;
/// ETA horizon (turns) for population-decline predictive warnings.
const POPULATION_WARNING_ETA: u32 = 10;

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
    /// Found a new colony at a specific surveyed site.
    ///
    /// Similar to [`Command::FoundColony`] but records the site identifier so
    /// the hex-map layer can link the colony to its location.
    FoundColonyAtSite {
        /// Display name for the new colony.
        name: String,
        /// Starting colonist head-count.
        starting_population: u64,
        /// Surveyed site where the colony is being placed.
        site_id: SiteId,
        /// Optional economic focus description (e.g. "mining", "agriculture").
        focus: Option<String>,
    },
    /// Add an infrastructure trade route between two colonies.
    ///
    /// Subsequent strategic turns will flow commodity surpluses from the
    /// higher-stockpile side toward the lower-stockpile side up to
    /// `throughput_cap` per commodity per turn.
    AddTradeRoute {
        /// One colony endpoint.
        colony_a: ColonyId,
        /// Other colony endpoint.
        colony_b: ColonyId,
        /// Maximum units per commodity that may transit per strategic turn.
        throughput_cap: f64,
    },
    /// Remove a trade route by its unique identifier.
    RemoveTradeRoute {
        /// Route identifier returned in [`Event::TradeRouteAdded`].
        route_id: uuid::Uuid,
    },
    /// Set or replace a manual trade priority override for a commodity at a colony.
    SetTradeOverride {
        /// Colony the override applies to.
        colony_id: ColonyId,
        /// Commodity identifier.
        commodity_id: String,
        /// When `true`, auto-flow is suppressed entirely for this commodity.
        suppress_auto: bool,
        /// Optional per-turn quantity cap (below the route throughput cap).
        cap: Option<f64>,
    },
    /// Clear a previously set trade override.
    ClearTradeOverride {
        /// Colony the override applied to.
        colony_id: ColonyId,
        /// Commodity identifier whose override should be removed.
        commodity_id: String,
    },
    /// Schedule an immigration wave to arrive at a gateway colony.
    ///
    /// Models external colonist ships landing at a spaceport colony after a
    /// transit delay.  The wave is queued as a [`migration::PendingMigration`]
    /// with `from_colony = None` and arrives after `transit_turns` strategic
    /// months.
    ScheduleImmigrationWave {
        /// Colony where the immigrants will land (must have a spaceport / gateway role).
        colony_id: ColonyId,
        /// Number of colonists in the incoming wave.
        count: f32,
        /// Strategic-month turns until arrival.
        transit_turns: u32,
    },
    /// Direct colonists to migrate from one colony to another.
    ///
    /// A voluntary directed override: colonists depart immediately and arrive
    /// after `transit_turns` strategic months.  No stability penalty.
    DirectMigration {
        /// Source colony.
        from_colony: ColonyId,
        /// Destination colony.
        to_colony: ColonyId,
        /// Number of colonists to move.
        count: f32,
        /// Transit time in strategic months.
        transit_turns: u32,
    },
    /// Force an evacuation from one colony to another.
    ///
    /// A forced move: colonists depart immediately, stability of the sending
    /// colony is penalised, and overcrowding pressure may hit the receiver.
    EvacuateColony {
        /// Colony being evacuated.
        from_colony: ColonyId,
        /// Colony receiving the evacuees.
        to_colony: ColonyId,
        /// Fraction of the source colony's population to evacuate (`[0.0, 1.0]`).
        fraction: f32,
        /// Transit time in strategic months.
        transit_turns: u32,
    },
    /// Open an emigration gate, enabling a voluntary flow from one colony to another.
    ///
    /// Each strategic month the engine will create a [`migration::PendingMigration`]
    /// batch of `rate * source_population` colonists directed from `from_colony` to
    /// `to_colony`.  Opening the same pair again replaces the rate.
    OpenEmigrationGate {
        /// Colony colonists depart from.
        from_colony: ColonyId,
        /// Colony colonists travel toward.
        to_colony: ColonyId,
        /// Fraction of `from_colony` population that departs per strategic month (`[0.0, 1.0]`).
        rate: f32,
    },
    /// Close a previously opened emigration gate between two colonies.
    ///
    /// Future strategic months will no longer create voluntary batches for this
    /// route.  Already in-transit batches are unaffected.
    CloseEmigrationGate {
        /// Colony the gate departs from.
        from_colony: ColonyId,
        /// Colony the gate points toward.
        to_colony: ColonyId,
    },
    /// Run one strategic-month pass of auto migration between colonies.
    ///
    /// Computes attractiveness scores for all colonies and queues voluntary
    /// pull-flow migrations toward the most attractive destinations.
    RunAutoMigration,
    /// Resolve all pending migrations that have reached `turns_remaining == 0`.
    ///
    /// Colonists arrive at their destination colonies; overcrowding and forced-
    /// move stability effects are applied.
    ResolvePendingMigrations,
    /// Begin construction of an orbital station using a content-pack blueprint.
    ///
    /// Deducts commodity costs from the colony pool immediately and adds the
    /// project to `GameState::orbital_construction_queue`.  Fails with
    /// [`EngineError::InsufficientResources`] when the colony pool cannot cover
    /// the blueprint costs, or with [`EngineError::InvalidArgument`] if the
    /// blueprint id is unknown.
    BeginOrbitalConstruction {
        /// Content-pack blueprint identifier.
        blueprint_id: String,
        /// Colony that will fund and operate the station.
        colony_id: ColonyId,
        /// Orbit band the finished station should occupy.
        orbit_type: orbital::OrbitType,
    },
    /// Build an orbital station in the given orbit band, linked to a colony.
    ///
    /// Fails with [`EngineError::OrbitalSlotExceeded`] if the orbit band is full.
    BuildOrbitalStation {
        /// Colony that funds and operates the station.
        colony_id: ColonyId,
        /// Station specialization type.
        station_type: orbital::StationType,
        /// Target orbit band.
        orbit_type: orbital::OrbitType,
    },
    /// Demolish (decommission) an orbital station by its stable id.
    DecommissionOrbitalStation {
        /// Stable identifier of the station to remove.
        station_id: uuid::Uuid,
    },
    /// Deploy a satellite constellation in the given orbit band.
    DeployConstellation {
        /// Satellite type (coverage layer).
        satellite_type: orbital::SatelliteType,
        /// Orbit band for the constellation.
        orbit_type: orbital::OrbitType,
        /// Number of satellites to deploy.
        count: u32,
    },
    /// Toggle the map-overlay visibility of a satellite constellation.
    ToggleConstellationOverlay {
        /// Stable identifier of the constellation.
        constellation_id: uuid::Uuid,
    },

    // ── Phase 10: Difficulty / Menace / Victory ───────────────────────────
    /// Set the active difficulty preset, rebuilding the difficulty scalar from the grade table.
    SetDifficulty {
        /// Preset to activate.
        preset: difficulty::DifficultyPreset,
    },
    /// Activate the existential clock with the given authored menace definition.
    ///
    /// Clears any previously active menace. Pass `None` to deactivate (sandbox off).
    ActivateMenace {
        /// Menace definition to activate, or `None` to deactivate.
        definition: Option<menace::MenaceDefinition>,
    },
    /// Tick the menace clock by one strategic month (called internally by `AdvanceColonySol`
    /// when a strategic month fires).  Exposed as a command for testing.
    TickMenace,
    /// Record that the interstellar expedition megaproject has been launched.
    ///
    /// This is the primary victory trigger. The engine evaluates all victory conditions
    /// and emits [`Event::VictoryAchieved`] for each newly satisfied condition.
    LaunchExpedition,
    /// Evaluate all tracked victory conditions against current game metrics.
    ///
    /// Emits [`Event::VictoryAchieved`] for newly satisfied conditions.
    EvaluateVictory,
    /// Activate sandbox-continue mode after a victory has been achieved.
    ///
    /// Suppresses the victory screen and lets the player keep playing.
    ContinueAfterVictory,
    /// Activate sandbox-continue mode after a victory (canonical issue-#96 name).
    ///
    /// Equivalent to [`Command::ContinueAfterVictory`].  Sets
    /// `GameState::sandbox_mode = true` so further commands are accepted
    /// without `VictoryAchieved` events being re-fired.
    ContinueSandbox,
    /// Initialise victory tracking with a specific set of conditions.
    ///
    /// If not called, the engine defaults to tracking the capstone expedition condition only.
    InitVictoryConditions {
        /// Conditions to track.
        conditions: Vec<victory::VictoryCondition>,
    },

    // ── M1: Planet map ────────────────────────────────────────────────────
    /// Generate and store a planet map from an RNG seed and cell radius.
    ///
    /// Overwrites any previously seeded map. The generated map is stored in
    /// `GameState::planet_map` and subsequent [`Command::FoundColonyAtSite`]
    /// commands can reference hex sites by their [`SiteId`].
    SeedPlanet {
        /// Deterministic RNG seed for map generation.
        seed: u64,
        /// Hex radius of the generated map (cell count = 3r²+3r+1).
        radius: u32,
    },

    // ── M1: Megaproject / Victory ─────────────────────────────────────────
    /// Contribute progress to an active megaproject.
    ///
    /// `progress` units are applied as research contribution to the current
    /// milestone.  When the milestone completes the engine checks whether the
    /// whole project is done; if it is an `InterstellarExpedition` the engine
    /// emits [`Event::VictoryAchieved`] and locks further commands.
    AdvanceMegaproject {
        /// The megaproject to advance.
        project_id: system::MegaprojectId,
        /// Research units to contribute this tick.
        progress: u32,
    },

    // ── M1: System zoom commands ──────────────────────────────────────────
    /// Issue a command directly to the system zoom layer.
    ///
    /// Wraps [`system::SystemCommand`] so all system-scope operations
    /// (body management, shipping routes, haulers, megaprojects, propulsion)
    /// are reachable through the main drive API.
    System(system::SystemCommand),

    // ── M1: Research direction ────────────────────────────────────────────
    /// Set the active research project, replacing any current one.
    ResearchTech {
        /// Tech definition id from the loaded content pack.
        tech_id: String,
    },
    /// Append a tech to the end of the research queue.
    EnqueueResearch {
        /// Tech definition id to enqueue.
        tech_id: String,
    },
    /// Clear the research queue and stop the current project.
    CancelResearch,

    // ── M1: Infrastructure ────────────────────────────────────────────────
    /// Begin construction of an infrastructure edge between two colonies.
    ///
    /// Both colonies must be placed on the planet map. The edge is added
    /// instantly (prototype behaviour) and a [`TradeRoute`](trade::TradeRoute) is wired up so
    /// commodity auto-flow activates on the next strategic turn.
    BuildInfrastructure {
        /// Source colony endpoint.
        from_colony: ColonyId,
        /// Destination colony endpoint.
        to_colony: ColonyId,
        /// Type of infrastructure to construct.
        infra_type: map::InfraType,
    },
    /// Remove an infrastructure edge between two colonies.
    ///
    /// Also removes the corresponding trade route so commodity flow stops.
    DemolishInfrastructure {
        /// Source colony endpoint.
        from_colony: ColonyId,
        /// Destination colony endpoint.
        to_colony: ColonyId,
    },

    // ── M3: Transport capacity for migration batches ──────────────────────
    /// Add passenger haulers to the migration transport fleet.
    ///
    /// Increases [`system::TransportCapacity::haulers`] by `count`.
    AddHauler {
        /// Number of haulers to add.
        count: u32,
    },
    /// Remove passenger haulers from the migration transport fleet.
    ///
    /// Clamps at zero — you cannot remove more haulers than exist.
    RemoveHauler {
        /// Number of haulers to remove.
        count: u32,
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
    /// Return the full colony management screen data bundle for a colony.
    ColonyScreen {
        /// Target colony.
        colony_id: ColonyId,
    },
    /// Return the planet hex map data (all hexes, colony nodes, infrastructure).
    PlanetMap,
    /// Return the interrupt digest from the most recent advance run.
    ///
    /// Returns an empty digest when no advance has been run yet.
    InterruptDigest,
    /// Return the current time-control state (sol, month, threshold, max turns).
    TimeControl,
    /// Return current difficulty preset and the active difficulty scalar.
    DifficultyStatus,
    /// Return the current menace state (if any menace is active).
    MenaceStatus,
    /// Return victory progress for all tracked conditions.
    VictoryStatus,
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
    /// Colony management screen data bundle.
    ColonyScreen(ui::ColonyScreenData),
    /// Planet hex map data bundle.
    PlanetMap(ui::PlanetMapData),
    /// Interrupt digest from the most recent advance run.
    InterruptDigest(ui::InterruptDigestData),
    /// Current time-control state.
    TimeControl(ui::TimeControlState),
    /// Active difficulty preset.
    DifficultyStatus(difficulty::DifficultyPreset),
    /// Current menace state snapshot, or `None` if inactive.
    MenaceStatus(Option<menace::MenaceState>),
    /// Victory progress for all tracked conditions.
    VictoryStatus(Vec<victory::VictoryProgress>),
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
    /// A colony was founded at a specific planetary site.
    ColonyFoundedAtSite {
        /// Stable identifier assigned to the new colony.
        colony_id: ColonyId,
        /// Display name of the new colony.
        name: String,
        /// Starting colonist head-count.
        starting_population: u64,
        /// Site identifier linking the colony to its hex-map location.
        site_id: SiteId,
        /// Optional economic focus.
        focus: Option<String>,
    },
    /// A trade route was added to the planetary trade network.
    TradeRouteAdded {
        /// Stable identifier for the new route.
        route_id: uuid::Uuid,
        /// One colony endpoint.
        colony_a: ColonyId,
        /// Other colony endpoint.
        colony_b: ColonyId,
        /// Per-commodity throughput cap.
        throughput_cap: f64,
    },
    /// A trade route was removed from the planetary trade network.
    TradeRouteRemoved {
        /// Identifier of the removed route.
        route_id: uuid::Uuid,
    },
    /// A manual trade override was set for a colony+commodity pair.
    TradeOverrideSet {
        /// Colony the override applies to.
        colony_id: ColonyId,
        /// Commodity identifier.
        commodity_id: String,
        /// Whether auto-flow is suppressed.
        suppress_auto: bool,
        /// Optional quantity cap.
        cap: Option<f64>,
    },
    /// A manual trade override was cleared.
    TradeOverrideCleared {
        /// Colony the override was on.
        colony_id: ColonyId,
        /// Commodity identifier.
        commodity_id: String,
    },
    /// An immigration wave was scheduled to arrive at a gateway colony.
    ImmigrationWaveScheduled {
        /// Destination colony.
        colony_id: ColonyId,
        /// Number of incoming colonists.
        count: f32,
        /// Turns until arrival.
        transit_turns: u32,
    },
    /// Colonists arrived at their destination colony (wave or directed migration).
    MigrationArrived {
        /// Source colony (`None` for off-map immigration waves).
        from_colony: Option<ColonyId>,
        /// Destination colony.
        to_colony: ColonyId,
        /// Number of colonists who arrived.
        count: f32,
        /// Stability penalty applied to the receiving colony for overcrowding.
        overcrowding_stability_penalty: f32,
        /// Stability penalty applied to the sending colony for forced departure.
        forced_departure_stability_penalty: f32,
    },
    /// Colonists departed a colony as part of a directed migration or evacuation.
    MigrationDeparted {
        /// Source colony.
        from_colony: ColonyId,
        /// Destination colony.
        to_colony: ColonyId,
        /// Number of colonists who departed.
        count: f32,
        /// Whether this was a forced evacuation.
        forced: bool,
    },
    /// Auto migration flows were computed and queued for the strategic month.
    AutoMigrationQueued {
        /// Number of migration legs queued.
        flow_count: usize,
        /// Total colonists in transit across all flows.
        total_in_transit: f32,
    },
    /// A migration batch was capped by transport capacity; excess demand is deferred.
    ///
    /// Emitted when `DirectMigration` or `EvacuateColony` demand exceeds
    /// `haulers × colonists_per_hauler` for the route this month.
    MigrationQueued {
        /// Stable identifier of the pending migration batch that was dispatched.
        batch_id: uuid::Uuid,
        /// Number of colonists whose departure was deferred to the next month.
        deferred_count: f32,
    },
    /// An emigration gate was opened between two colonies.
    EmigrationGateOpened {
        /// Colony colonists depart from.
        from_colony: ColonyId,
        /// Colony colonists travel toward.
        to_colony: ColonyId,
        /// Fraction of source population that departs per strategic month.
        rate: f32,
    },
    /// An emigration gate was closed between two colonies.
    EmigrationGateClosed {
        /// Colony the gate departed from.
        from_colony: ColonyId,
        /// Colony the gate pointed toward.
        to_colony: ColonyId,
    },
    /// Migration batches were created this strategic month for open emigration gates.
    GateMigrationQueued {
        /// Number of migration batches created across all open gates.
        batch_count: usize,
        /// Total colonists placed in transit.
        total_in_transit: f32,
    },
    /// Voluntary emigration was auto-triggered due to low stability.
    VoluntaryEmigrationTriggered {
        /// Colony that triggered the auto-emigration.
        from_colony: ColonyId,
        /// Colony colonists were directed toward (most attractive neighbour).
        to_colony: ColonyId,
        /// Number of colonists that departed.
        count: f32,
    },
    /// An orbital station construction project was started.
    OrbitalConstructionStarted {
        /// Blueprint that was used to start this project.
        blueprint_id: String,
        /// Colony funding the build.
        colony_id: ColonyId,
        /// Orbit band the finished station will occupy.
        orbit_type: orbital::OrbitType,
        /// Strategic months until completion.
        build_months: u32,
    },
    /// An orbital station construction project finished.
    OrbitalStationCompleted {
        /// Stable identifier of the newly placed station.
        station_id: uuid::Uuid,
        /// Colony that owns the station.
        colony_id: ColonyId,
        /// Station specialization type.
        station_type: orbital::StationType,
        /// Orbit band the station now occupies.
        orbit_type: orbital::OrbitType,
        /// Blueprint id that produced this station.
        blueprint_id: String,
    },
    /// An orbital station was built.
    OrbitalStationBuilt {
        /// Stable identifier of the new station.
        station_id: uuid::Uuid,
        /// Colony that owns the station.
        colony_id: ColonyId,
        /// Station specialization type.
        station_type: orbital::StationType,
        /// Orbit band the station occupies.
        orbit_type: orbital::OrbitType,
        /// Slots consumed in the orbit band.
        slot_cost: u32,
    },
    /// An orbital station was decommissioned; its slots are freed.
    OrbitalStationDecommissioned {
        /// Stable identifier of the decommissioned station.
        station_id: uuid::Uuid,
    },
    /// A satellite constellation was deployed.
    ConstellationDeployed {
        /// Stable identifier of the constellation.
        constellation_id: uuid::Uuid,
        /// Coverage type.
        satellite_type: orbital::SatelliteType,
        /// Orbit band.
        orbit_type: orbital::OrbitType,
        /// Number of satellites in the array.
        count: u32,
    },
    /// The map-overlay visibility of a constellation was toggled.
    ConstellationOverlayToggled {
        /// Stable identifier of the constellation.
        constellation_id: uuid::Uuid,
        /// New visibility state.
        visible: bool,
    },
    // ── Phase 10 events ───────────────────────────────────────────────────
    /// The difficulty preset was changed.
    DifficultyChanged {
        /// The new active preset.
        preset: difficulty::DifficultyPreset,
    },
    /// A menace phase was newly activated this strategic month.
    MenacePhaseTriggered {
        /// Content-pack id of the menace definition.
        menace_id: String,
        /// Index of the phase that just activated.
        phase_index: usize,
        /// Telegraph text for the *next* phase (if any), wired to the interrupt system.
        telegraph: Option<String>,
        /// Hazard content-pack key injected on this phase entry, if any.
        hazard_injection: Option<String>,
    },
    /// The menace has reached its final phase; collapse is now emergent.
    MenaceFinalPhaseReached {
        /// Content-pack id of the menace.
        menace_id: String,
    },
    /// A technology node completed research and its effects were applied.
    TechUnlocked {
        /// Content-pack id of the tech that finished.
        tech_id: String,
    },
    /// A victory condition was newly satisfied.
    VictoryAchieved {
        /// The condition that was satisfied.
        condition: victory::VictoryCondition,
    },
    /// The player chose to continue playing after victory (sandbox continue).
    SandboxContinued,

    // ── M1: Research direction events ─────────────────────────────────────
    /// A new tech was set as the active research project.
    ResearchStarted {
        /// The tech that is now being researched.
        tech_id: String,
    },
    /// A tech was appended to the research queue.
    ResearchQueued {
        /// The tech added to the queue.
        tech_id: String,
    },
    /// The research queue and current project were cleared.
    ResearchCancelled,

    /// A planet map was generated and stored in `GameState`.
    PlanetSeeded {
        /// Seed used for generation.
        seed: u64,
        /// Hex radius of the generated map.
        radius: u32,
        /// Total number of hex cells in the map.
        cell_count: usize,
    },

    /// A colony was placed on its hex coordinate in the planet map.
    ColonyPlacedOnMap {
        /// The colony that was placed.
        colony_id: ColonyId,
        /// Axial column of the hex.
        q: i32,
        /// Axial row of the hex.
        r: i32,
    },

    /// A cargo shipment arrived and its contents were credited to the destination colony pool.
    CargoDelivered {
        /// Stable identifier of the shipment that arrived.
        shipment_id: uuid::Uuid,
        /// Colony whose pool was credited.
        colony_id: ColonyId,
        /// Commodity that was deposited.
        commodity_id: String,
        /// Quantity deposited into the colony pool.
        amount: f64,
    },

    /// An event produced by the system zoom layer.
    ///
    /// Wraps [`system::SystemEvent`] so all system-scope events are observable
    /// through the main event stream.
    System(system::SystemEvent),

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

    // ── M1: Infrastructure events ─────────────────────────────────────────
    /// An infrastructure edge was built and the corresponding trade route activated.
    InfrastructureBuilt {
        /// Source colony.
        from_colony: ColonyId,
        /// Destination colony.
        to_colony: ColonyId,
        /// Type of infrastructure constructed.
        infra_type: map::InfraType,
        /// Construction cost in abstract resource units.
        cost: f32,
        /// Stable identifier of the trade route created for this edge.
        route_id: uuid::Uuid,
    },
    /// An infrastructure edge was demolished and its trade route removed.
    InfrastructureDemolished {
        /// Source colony.
        from_colony: ColonyId,
        /// Destination colony.
        to_colony: ColonyId,
        /// Stable identifier of the trade route that was removed.
        route_id: uuid::Uuid,
    },

    // ── M2: Environmental hazard events ───────────────────────────────────
    /// An environmental hazard struck a colony this sol.
    HazardOccurred {
        /// Colony that was hit by the hazard.
        colony_id: ColonyId,
        /// The type of hazard that occurred.
        kind: hazard::HazardKind,
        /// Sampled severity in `[0.0, 1.0]`; higher = more damage.
        severity: f32,
        /// Stability change applied (negative).
        stability_delta: f32,
        /// Commodity losses: `(commodity_id, amount_lost)`.
        commodity_losses: Vec<(String, f64)>,
        /// Population lost this hazard.
        population_lost: f32,
    },

    // ── M2: Interrupt-source events ───────────────────────────────────────
    /// A named hazard or environmental event was fired by the sim.
    HazardFired {
        /// Content-pack identifier of the event that fired.
        event_id: interrupt::EventId,
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
    /// The colony pool does not hold enough of a required commodity to start construction.
    #[error(
        "insufficient resources: need {needed} of '{commodity}' but only {available} available"
    )]
    InsufficientResources {
        /// Commodity that is short.
        commodity: String,
        /// Amount required.
        needed: f32,
        /// Amount currently in the colony pool.
        available: f32,
    },
    /// The referenced directive does not exist.
    #[error("directive not found: {0}")]
    DirectiveNotFound(directive::DirectiveId),
    /// An orbital slot operation failed (orbit band full, station not found, etc.).
    #[error("orbital error: {0}")]
    OrbitalError(#[from] OrbitalError),
    /// No planet map has been seeded yet.
    #[error("no planet map: call SeedPlanet first")]
    NoPlanetMap,
    /// The referenced site identifier is not in the planet map.
    #[error("site not found: {0:?}")]
    SiteNotFound(SiteId),
    /// The hex cell at the requested site is not habitable (ocean or similar).
    #[error("site is not habitable")]
    SiteNotHabitable,
    /// A colony already occupies the hex cell at the requested site.
    #[error("site is already occupied")]
    SiteOccupied,
    /// A command was submitted after the game has been won.
    ///
    /// The player must activate sandbox-continue mode via
    /// [`Command::ContinueAfterVictory`] to resume play.
    #[error("game over: victory already achieved")]
    GameOver,
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
    /// Digest from the most recent `advance_until_interrupted` run (for the UI digest panel).
    last_advance_digest: Option<ui::InterruptDigestData>,
    /// Current interrupt threshold used for time-control display.
    pub interrupt_threshold: interrupt::Tier,
    /// Maximum turns for the next advance-until-interrupted run.
    pub max_advance_turns: u32,
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
            last_advance_digest: None,
            interrupt_threshold: interrupt::Tier::Notable,
            max_advance_turns: 10,
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
        // Block all commands once victory is recorded, unless sandbox-continue is active.
        if self.state.victory.is_some()
            && !self.state.victory_state.sandbox_continue
            && !matches!(cmd, Command::ContinueAfterVictory | Command::ContinueSandbox)
        {
            return Err(EngineError::GameOver);
        }

        match cmd {
            Command::AdvanceColonySol => {
                let outcome = self.processor.advance(&mut self.state);
                let hazard_outcomes = outcome.hazard_outcomes.clone();
                let mut events = vec![Event::ColonySolAdvanced { sol: outcome.sol }];
                if outcome
                    .cadences_fired
                    .contains(&turn::TurnCadence::StrategicMonth)
                {
                    events.push(Event::StrategicMonthAdvanced {
                        month: outcome.month,
                    });
                    // Emit TechUnlocked for each completed tech this month.
                    for tech_id in outcome.completed_techs {
                        events.push(Event::TechUnlocked { tech_id });
                    }
                    // Emit one CargoDelivered event per commodity line per shipment.
                    for record in outcome.cargo_delivered {
                        events.push(Event::CargoDelivered {
                            shipment_id: record.shipment_id,
                            colony_id: record.colony_id,
                            commodity_id: record.commodity_id,
                            amount: record.amount,
                        });
                    }

                    // ── Orbital construction countdown ────────────────────────
                    // Decrement each in-progress project; when months_remaining
                    // hits zero, place the finished station in the registry.
                    {
                        let mut completed_projects: Vec<orbital::OrbitalConstructionProject> =
                            Vec::new();
                        self.state.orbital_construction_queue.retain_mut(|p| {
                            if p.tick() {
                                completed_projects.push(p.clone());
                                false
                            } else {
                                true
                            }
                        });
                        for project in completed_projects {
                            let station = OrbitalStation::new(
                                project.station_type,
                                project.orbit_type,
                                project.colony_id,
                            );
                            let station_id = station.id;
                            // Best-effort: if the orbit band filled while the
                            // project was in-flight, skip placement (rare edge
                            // case).  No rollback of the already-paid costs.
                            let _ = self.state.orbital_registry.add_station(station);
                            events.push(Event::OrbitalStationCompleted {
                                station_id,
                                colony_id: project.colony_id,
                                station_type: project.station_type,
                                orbit_type: project.orbit_type,
                                blueprint_id: project.blueprint_id.clone(),
                            });
                        }
                    }

                    // ── Gate migration ────────────────────────────────────────
                    // For every open emigration gate, compute departures as
                    // rate * source_population and enqueue a PendingMigration.
                    {
                        let gates: Vec<EmigrationGate> = self.state.emigration_gates.clone();
                        let mut batch_count = 0usize;
                        let mut total_in_transit = 0.0_f32;
                        for gate in &gates {
                            let Ok(from_idx) = self.find_colony_index(gate.from_colony) else {
                                continue;
                            };
                            if self.find_colony_index(gate.to_colony).is_err() {
                                continue;
                            }
                            let src_pop = self.state.populations[from_idx].count;
                            let movers = (src_pop * gate.rate).floor();
                            if movers < 1.0 {
                                continue;
                            }
                            self.state.populations[from_idx].count = (src_pop - movers).max(0.0);
                            let mig = PendingMigration::new(
                                Some(gate.from_colony),
                                gate.to_colony,
                                movers,
                                1, // 1 strategic month transit
                                false,
                                movers * 0.1,
                            );
                            self.state.pending_migrations.push(mig);
                            batch_count += 1;
                            total_in_transit += movers;
                        }
                        if batch_count > 0 {
                            events.push(Event::GateMigrationQueued {
                                batch_count,
                                total_in_transit,
                            });
                        }
                    }

                    // ── Voluntary emigration at low stability ─────────────────
                    // When stability ≤ emigration_stability_floor auto-trigger
                    // a small outflow even without an open gate.
                    if let Some(config) = self.state.needs_config.clone() {
                        let colony_ids: Vec<ColonyId> =
                            self.state.colonies.iter().map(|c| c.id).collect();
                        let attractiveness: Vec<ColonyAttractiveness> = self
                            .state
                            .colonies
                            .iter()
                            .zip(self.state.populations.iter())
                            .map(|(colony, pop)| {
                                #[allow(clippy::cast_possible_truncation)]
                                let housing = colony.pool.amount("housing") as f32;
                                compute_attractiveness(
                                    colony.id,
                                    pop.stability,
                                    housing,
                                    pop.count,
                                    1.0,
                                )
                            })
                            .collect();

                        for (i, &src_id) in colony_ids.iter().enumerate() {
                            let stability = self.state.populations[i].stability;
                            if stability > config.emigration_stability_floor {
                                continue;
                            }
                            let src_pop = self.state.populations[i].count;
                            if src_pop < 1.0 {
                                continue;
                            }
                            let src_score = attractiveness
                                .iter()
                                .find(|a| a.colony_id == src_id)
                                .map_or(0.0, |a| a.score);
                            // Find the most attractive other colony.
                            let best_dst = attractiveness
                                .iter()
                                .filter(|a| a.colony_id != src_id && a.score > src_score)
                                .max_by(|a, b| {
                                    a.score
                                        .partial_cmp(&b.score)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                            let Some(dst) = best_dst else { continue };
                            let movers = (src_pop * config.voluntary_emigration_rate)
                                .floor()
                                .max(1.0);
                            let movers = movers.min(src_pop);
                            self.state.populations[i].count = (src_pop - movers).max(0.0);
                            let mig = PendingMigration::new(
                                Some(src_id),
                                dst.colony_id,
                                movers,
                                1,
                                false,
                                movers * 0.1,
                            );
                            self.state.pending_migrations.push(mig);
                            events.push(Event::VoluntaryEmigrationTriggered {
                                from_colony: src_id,
                                to_colony: dst.colony_id,
                                count: movers,
                            });
                        }
                    }

                    // ── Tick and resolve pending migrations ───────────────────
                    {
                        let mut arrived: Vec<PendingMigration> = Vec::new();
                        self.state.pending_migrations.retain_mut(|m| {
                            if m.tick() {
                                arrived.push(m.clone());
                                false
                            } else {
                                true
                            }
                        });
                        for mig in &arrived {
                            let Ok(to_idx) = self.find_colony_index(mig.to_colony) else {
                                continue;
                            };
                            #[allow(clippy::cast_possible_truncation)]
                            let housing = self.state.colonies[to_idx].pool.amount("housing") as f32;
                            let current_pop = self.state.populations[to_idx].count;
                            let outcome = resolve_arrival(mig, housing, current_pop);
                            self.state.populations[to_idx].count += outcome.arrived;
                            self.state.populations[to_idx].stability =
                                (self.state.populations[to_idx].stability
                                    + outcome.overcrowding_stability_penalty)
                                    .clamp(0.0, 1.0);
                            events.push(Event::MigrationArrived {
                                from_colony: mig.from_colony,
                                to_colony: mig.to_colony,
                                count: outcome.arrived,
                                overcrowding_stability_penalty: outcome
                                    .overcrowding_stability_penalty,
                                forced_departure_stability_penalty: outcome
                                    .forced_departure_stability_penalty,
                            });
                        }
                    }
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

                // ── Step 4b: Hazard effects ───────────────────────────────
                // Apply stability, commodity, and population damage from hazard
                // outcomes rolled by the turn processor.
                for h in &hazard_outcomes {
                    // Find colony and population by id.
                    if let Some(idx) = self.state.colonies.iter().position(|c| c.id == h.colony_id)
                    {
                        // Apply commodity losses.
                        for (comm_id, loss) in &h.commodity_losses {
                            self.state.colonies[idx].pool.withdraw(comm_id, *loss);
                        }
                        // Apply stability delta.
                        self.state.populations[idx].stability =
                            (self.state.populations[idx].stability + h.stability_delta)
                                .clamp(0.0, 1.0);
                        // Apply population loss.
                        self.state.populations[idx].count =
                            (self.state.populations[idx].count - h.population_lost).max(0.0);
                    }
                    events.push(Event::HazardOccurred {
                        colony_id: h.colony_id,
                        kind: h.kind,
                        severity: h.severity,
                        stability_delta: h.stability_delta,
                        commodity_losses: h.commodity_losses.clone(),
                        population_lost: h.population_lost,
                    });
                }

                // ── Step 4d: Stability + population tracking ─────────────
                // Record samples per colony so predictive warnings can
                // extrapolate trajectory without full forward simulation.
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
                        let count = pop.count;
                        self.state
                            .population_trackers
                            .entry(*colony_id)
                            .or_default()
                            .push(count);
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
                        // Build per-commodity snapshot for commodity predicates.
                        let commodities = self.state.colonies[idx]
                            .pool
                            .commodity_ids()
                            .map(|id| {
                                let pool = &self.state.colonies[idx].pool;
                                (
                                    id.to_owned(),
                                    predicate::CommoditySnapshot {
                                        amount: pool.amount(id),
                                        delta: pool.delta(id),
                                    },
                                )
                            })
                            .collect();
                        let ctx = predicate::PredicateContext {
                            colony_id,
                            population: pop.count,
                            stability: pop.stability,
                            available_labour: pop.available_labor(),
                            system_research: self.state.research_pool.total(),
                            sol: self.state.sol,
                            month: self.state.month,
                            commodities,
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

            Command::FoundColonyAtSite {
                name,
                starting_population,
                site_id,
                focus,
            } => {
                if name.trim().is_empty() {
                    return Err(EngineError::InvalidArgument(
                        "colony name must not be empty".into(),
                    ));
                }
                // Require a seeded planet map.
                let coord = {
                    let pm = self
                        .state
                        .planet_map
                        .as_ref()
                        .ok_or(EngineError::NoPlanetMap)?;
                    pm.coord_for_site(*site_id)
                        .ok_or(EngineError::SiteNotFound(*site_id))?
                };
                // Validate habitability and occupancy before mutating state.
                // planet_map is guaranteed Some here (we just resolved coord from it).
                let pm = self
                    .state
                    .planet_map
                    .as_ref()
                    .ok_or(EngineError::NoPlanetMap)?;
                let cell = pm
                    .cells
                    .get(&coord)
                    .ok_or(EngineError::SiteNotFound(*site_id))?;
                if !cell.is_habitable() {
                    return Err(EngineError::SiteNotHabitable);
                }
                if pm.colonies.iter().any(|n| n.coord == coord) {
                    return Err(EngineError::SiteOccupied);
                }
                let _ = pm;
                let colony = colony::Colony::new(name.clone());
                let colony_id = colony.id;
                self.state.add_colony(colony, *starting_population);
                // Place colony node on the map.
                let pm_mut = self
                    .state
                    .planet_map
                    .as_mut()
                    .ok_or(EngineError::NoPlanetMap)?;
                pm_mut
                    .place_colony(colony_id, coord)
                    .map_err(|e| EngineError::InvalidState(e.to_string()))?;
                Ok(vec![
                    Event::ColonyFoundedAtSite {
                        colony_id,
                        name: name.clone(),
                        starting_population: *starting_population,
                        site_id: *site_id,
                        focus: focus.clone(),
                    },
                    Event::ColonyPlacedOnMap {
                        colony_id,
                        q: coord.q,
                        r: coord.r,
                    },
                ])
            }

            Command::AddTradeRoute {
                colony_a,
                colony_b,
                throughput_cap,
            } => {
                self.find_colony_index(*colony_a)?;
                self.find_colony_index(*colony_b)?;
                if *throughput_cap < 0.0 {
                    return Err(EngineError::InvalidArgument(
                        "throughput_cap must be >= 0".into(),
                    ));
                }
                let route = TradeRoute::new(*colony_a, *colony_b, *throughput_cap);
                let route_id = route.id;
                self.state.trade_network.add_route(route);
                Ok(vec![Event::TradeRouteAdded {
                    route_id,
                    colony_a: *colony_a,
                    colony_b: *colony_b,
                    throughput_cap: *throughput_cap,
                }])
            }

            Command::RemoveTradeRoute { route_id } => {
                if !self.state.trade_network.remove_route(*route_id) {
                    return Err(EngineError::InvalidArgument(format!(
                        "trade route {route_id} not found"
                    )));
                }
                Ok(vec![Event::TradeRouteRemoved {
                    route_id: *route_id,
                }])
            }

            Command::SetTradeOverride {
                colony_id,
                commodity_id,
                suppress_auto,
                cap,
            } => {
                self.find_colony_index(*colony_id)?;
                if commodity_id.trim().is_empty() {
                    return Err(EngineError::InvalidArgument(
                        "commodity_id must not be empty".into(),
                    ));
                }
                self.state.trade_network.set_override(TradeOverride {
                    colony_id: *colony_id,
                    commodity_id: commodity_id.clone(),
                    suppress_auto: *suppress_auto,
                    cap: *cap,
                });
                Ok(vec![Event::TradeOverrideSet {
                    colony_id: *colony_id,
                    commodity_id: commodity_id.clone(),
                    suppress_auto: *suppress_auto,
                    cap: *cap,
                }])
            }

            Command::ClearTradeOverride {
                colony_id,
                commodity_id,
            } => {
                self.find_colony_index(*colony_id)?;
                self.state
                    .trade_network
                    .clear_override(*colony_id, commodity_id);
                Ok(vec![Event::TradeOverrideCleared {
                    colony_id: *colony_id,
                    commodity_id: commodity_id.clone(),
                }])
            }

            Command::ScheduleImmigrationWave {
                colony_id,
                count,
                transit_turns,
            } => {
                self.find_colony_index(*colony_id)?;
                if *count <= 0.0 {
                    return Err(EngineError::InvalidArgument(
                        "immigration wave count must be > 0".into(),
                    ));
                }
                let wave =
                    PendingMigration::new(None, *colony_id, *count, *transit_turns, false, 0.0);
                self.state.pending_migrations.push(wave);
                Ok(vec![Event::ImmigrationWaveScheduled {
                    colony_id: *colony_id,
                    count: *count,
                    transit_turns: *transit_turns,
                }])
            }

            Command::DirectMigration {
                from_colony,
                to_colony,
                count,
                transit_turns,
            } => {
                let from_idx = self.find_colony_index(*from_colony)?;
                self.find_colony_index(*to_colony)?;
                if *count <= 0.0 {
                    return Err(EngineError::InvalidArgument(
                        "migration count must be > 0".into(),
                    ));
                }
                let available = self.state.populations[from_idx].count;
                if *count > available {
                    return Err(EngineError::InvalidArgument(format!(
                        "requested {count} migrants but only {available:.0} available"
                    )));
                }
                // Cap the batch by transport capacity; defer the remainder.
                #[allow(clippy::cast_precision_loss)]
                let capacity = self.state.system_state.transport_capacity.total() as f32;
                let dispatched = count.min(capacity);
                let deferred = count - dispatched;
                // Deduct from source immediately (they are in transit).
                self.state.populations[from_idx].count -= dispatched;
                let mig = PendingMigration::new(
                    Some(*from_colony),
                    *to_colony,
                    dispatched,
                    *transit_turns,
                    false,
                    dispatched * 0.1,
                );
                let batch_id = mig.id;
                self.state.pending_migrations.push(mig);
                let mut events = vec![Event::MigrationDeparted {
                    from_colony: *from_colony,
                    to_colony: *to_colony,
                    count: dispatched,
                    forced: false,
                }];
                if deferred > 0.0 {
                    events.push(Event::MigrationQueued {
                        batch_id,
                        deferred_count: deferred,
                    });
                }
                Ok(events)
            }

            Command::EvacuateColony {
                from_colony,
                to_colony,
                fraction,
                transit_turns,
            } => {
                let from_idx = self.find_colony_index(*from_colony)?;
                self.find_colony_index(*to_colony)?;
                let fraction = fraction.clamp(0.0, 1.0);
                let count = (self.state.populations[from_idx].count * fraction).floor();
                if count < 1.0 {
                    return Err(EngineError::InvalidArgument(
                        "evacuation fraction too small to move any colonists".into(),
                    ));
                }
                // Cap the batch by transport capacity; defer the remainder.
                #[allow(clippy::cast_precision_loss)]
                let capacity = self.state.system_state.transport_capacity.total() as f32;
                let dispatched = count.min(capacity);
                let deferred = count - dispatched;
                // Deduct from source; apply forced-move stability penalty.
                self.state.populations[from_idx].count -= dispatched;
                self.state.populations[from_idx].stability = (self.state.populations[from_idx]
                    .stability
                    - migration::FORCED_MOVE_STABILITY_COST)
                    .clamp(0.0, 1.0);
                let mig = PendingMigration::new(
                    Some(*from_colony),
                    *to_colony,
                    dispatched,
                    *transit_turns,
                    true,
                    dispatched * 0.2,
                );
                let batch_id = mig.id;
                self.state.pending_migrations.push(mig);
                let mut events = vec![Event::MigrationDeparted {
                    from_colony: *from_colony,
                    to_colony: *to_colony,
                    count: dispatched,
                    forced: true,
                }];
                if deferred > 0.0 {
                    events.push(Event::MigrationQueued {
                        batch_id,
                        deferred_count: deferred,
                    });
                }
                Ok(events)
            }

            Command::AddHauler { count } => {
                self.state.system_state.transport_capacity.haulers = self
                    .state
                    .system_state
                    .transport_capacity
                    .haulers
                    .saturating_add(*count);
                Ok(vec![])
            }

            Command::RemoveHauler { count } => {
                self.state.system_state.transport_capacity.haulers = self
                    .state
                    .system_state
                    .transport_capacity
                    .haulers
                    .saturating_sub(*count);
                Ok(vec![])
            }

            Command::OpenEmigrationGate {
                from_colony,
                to_colony,
                rate,
            } => {
                self.find_colony_index(*from_colony)?;
                self.find_colony_index(*to_colony)?;
                let rate = rate.clamp(0.0, 1.0);
                // Replace existing gate for this pair, or push a new one.
                if let Some(gate) = self
                    .state
                    .emigration_gates
                    .iter_mut()
                    .find(|g| g.from_colony == *from_colony && g.to_colony == *to_colony)
                {
                    gate.rate = rate;
                } else {
                    self.state.emigration_gates.push(EmigrationGate {
                        from_colony: *from_colony,
                        to_colony: *to_colony,
                        rate,
                    });
                }
                Ok(vec![Event::EmigrationGateOpened {
                    from_colony: *from_colony,
                    to_colony: *to_colony,
                    rate,
                }])
            }

            Command::CloseEmigrationGate {
                from_colony,
                to_colony,
            } => {
                self.state
                    .emigration_gates
                    .retain(|g| !(g.from_colony == *from_colony && g.to_colony == *to_colony));
                Ok(vec![Event::EmigrationGateClosed {
                    from_colony: *from_colony,
                    to_colony: *to_colony,
                }])
            }

            Command::RunAutoMigration => {
                // Compute attractiveness for every colony.
                let attractiveness: Vec<ColonyAttractiveness> = self
                    .state
                    .colonies
                    .iter()
                    .zip(self.state.populations.iter())
                    .map(|(colony, pop)| {
                        #[allow(clippy::cast_possible_truncation)]
                        let housing = colony.pool.amount("housing") as f32;
                        compute_attractiveness(
                            colony.id,
                            pop.stability,
                            housing,
                            pop.count,
                            1.0, // conservative default
                        )
                    })
                    .collect();

                let populations: Vec<f32> =
                    self.state.populations.iter().map(|p| p.count).collect();
                let colony_ids: Vec<ColonyId> = self.state.colonies.iter().map(|c| c.id).collect();
                let params = AutoMigrationParams::default();

                let flows = compute_auto_flows(&attractiveness, &populations, &colony_ids, &params);
                let flow_count = flows.len();
                let total_in_transit: f32 = flows.iter().map(|f| f.count).sum();

                // Deduct departing colonists from source colonies.
                for flow in &flows {
                    if let Some(src_id) = flow.from_colony {
                        if let Ok(idx) = self.find_colony_index(src_id) {
                            self.state.populations[idx].count =
                                (self.state.populations[idx].count - flow.count).max(0.0);
                        }
                    }
                }
                self.state.pending_migrations.extend(flows);

                Ok(vec![Event::AutoMigrationQueued {
                    flow_count,
                    total_in_transit,
                }])
            }

            Command::ResolvePendingMigrations => {
                let mut events = Vec::new();

                // Tick all pending migrations; collect those that arrive.
                let mut arrived = Vec::new();
                self.state.pending_migrations.retain_mut(|m| {
                    if m.tick() {
                        arrived.push(m.clone());
                        false
                    } else {
                        true
                    }
                });

                for mig in &arrived {
                    let Ok(to_idx) = self.find_colony_index(mig.to_colony) else {
                        continue; // destination no longer exists; drop
                    };

                    #[allow(clippy::cast_possible_truncation)]
                    let housing = self.state.colonies[to_idx].pool.amount("housing") as f32;
                    let current_pop = self.state.populations[to_idx].count;
                    let outcome = resolve_arrival(mig, housing, current_pop);

                    // Add arrived colonists.
                    self.state.populations[to_idx].count += outcome.arrived;

                    // Apply overcrowding penalty to receiver.
                    self.state.populations[to_idx].stability = (self.state.populations[to_idx]
                        .stability
                        + outcome.overcrowding_stability_penalty)
                        .clamp(0.0, 1.0);

                    events.push(Event::MigrationArrived {
                        from_colony: mig.from_colony,
                        to_colony: mig.to_colony,
                        count: outcome.arrived,
                        overcrowding_stability_penalty: outcome.overcrowding_stability_penalty,
                        forced_departure_stability_penalty: outcome
                            .forced_departure_stability_penalty,
                    });
                }

                Ok(events)
            }

            Command::BeginOrbitalConstruction {
                blueprint_id,
                colony_id,
                orbit_type,
            } => {
                let colony_idx = self.find_colony_index(*colony_id)?;
                // Look up the blueprint in the loaded registry.
                let blueprint = self
                    .state
                    .registry
                    .as_ref()
                    .and_then(|r| r.orbital_blueprints.get(blueprint_id.as_str()).cloned())
                    .ok_or_else(|| {
                        EngineError::InvalidArgument(format!(
                            "unknown orbital blueprint: '{blueprint_id}'"
                        ))
                    })?;
                // Validate that the colony pool can cover all commodity costs.
                for (commodity_id, qty) in &blueprint.commodity_costs {
                    #[allow(clippy::cast_possible_truncation)]
                    let available =
                        self.state.colonies[colony_idx].pool.amount(commodity_id) as f32;
                    if available < *qty {
                        return Err(EngineError::InsufficientResources {
                            commodity: commodity_id.clone(),
                            needed: *qty,
                            available,
                        });
                    }
                }
                // Deduct costs.
                for (commodity_id, qty) in &blueprint.commodity_costs {
                    self.state.colonies[colony_idx]
                        .pool
                        .withdraw(commodity_id, f64::from(*qty));
                }
                // Enqueue the construction project.
                let mut project = orbital::OrbitalConstructionProject::new(
                    blueprint_id.clone(),
                    *colony_id,
                    *orbit_type,
                    blueprint.station_type,
                    blueprint.build_months,
                );
                project.costs_paid = true;
                let build_months = project.months_remaining;
                self.state.orbital_construction_queue.push(project);
                Ok(vec![Event::OrbitalConstructionStarted {
                    blueprint_id: blueprint_id.clone(),
                    colony_id: *colony_id,
                    orbit_type: *orbit_type,
                    build_months,
                }])
            }

            Command::BuildOrbitalStation {
                colony_id,
                station_type,
                orbit_type,
            } => {
                self.find_colony_index(*colony_id)?;
                let station = OrbitalStation::new(*station_type, *orbit_type, *colony_id);
                let station_id = station.id;
                let slot_cost = station.slot_cost;
                self.state.orbital_registry.add_station(station)?;
                Ok(vec![Event::OrbitalStationBuilt {
                    station_id,
                    colony_id: *colony_id,
                    station_type: *station_type,
                    orbit_type: *orbit_type,
                    slot_cost,
                }])
            }

            Command::DecommissionOrbitalStation { station_id } => {
                self.state.orbital_registry.remove_station(*station_id)?;
                Ok(vec![Event::OrbitalStationDecommissioned {
                    station_id: *station_id,
                }])
            }

            Command::DeployConstellation {
                satellite_type,
                orbit_type,
                count,
            } => {
                if *count == 0 {
                    return Err(EngineError::InvalidArgument(
                        "constellation count must be > 0".into(),
                    ));
                }
                let constellation =
                    SatelliteConstellation::new(*satellite_type, *orbit_type, *count);
                let constellation_id = constellation.id;
                self.state
                    .orbital_registry
                    .deploy_constellation(constellation);
                Ok(vec![Event::ConstellationDeployed {
                    constellation_id,
                    satellite_type: *satellite_type,
                    orbit_type: *orbit_type,
                    count: *count,
                }])
            }

            Command::ToggleConstellationOverlay { constellation_id } => {
                let visible = self
                    .state
                    .orbital_registry
                    .toggle_overlay(*constellation_id)?;
                Ok(vec![Event::ConstellationOverlayToggled {
                    constellation_id: *constellation_id,
                    visible,
                }])
            }

            // ── Phase 10: Difficulty / Menace / Victory ───────────────────
            Command::SetDifficulty { preset } => {
                self.state.difficulty_preset = *preset;
                self.state.difficulty_scalar =
                    self.state.difficulty_grade_table.build_scalar(*preset);
                Ok(vec![Event::DifficultyChanged { preset: *preset }])
            }

            Command::ActivateMenace { definition } => {
                self.state.menace_state = definition
                    .as_ref()
                    .map(|d| menace::MenaceState::new(d.clone()));
                Ok(vec![])
            }

            Command::TickMenace => {
                let mut events = Vec::new();
                if let Some(ms) = &mut self.state.menace_state {
                    let outcome = ms.tick();
                    if let Some(phase_index) = outcome.phase_entered {
                        let menace_id = ms.definition.id.clone();
                        events.push(Event::MenacePhaseTriggered {
                            menace_id: menace_id.clone(),
                            phase_index,
                            telegraph: outcome.telegraph.clone(),
                            hazard_injection: outcome.hazard_injection.clone(),
                        });
                        // Emit HazardFired so the interrupt system can surface it.
                        if let Some(hazard_id) = &outcome.hazard_injection {
                            events.push(Event::HazardFired {
                                event_id: hazard_id.clone(),
                            });
                        }
                        if outcome.final_phase_reached {
                            events.push(Event::MenaceFinalPhaseReached { menace_id });
                        }
                    }
                }
                Ok(events)
            }

            Command::LaunchExpedition => {
                self.state.expedition_launched = true;
                let snap = self.build_victory_snapshot();
                let mut events = Vec::new();
                for condition in self.state.victory_state.evaluate(&snap) {
                    events.push(Event::VictoryAchieved { condition });
                }
                Ok(events)
            }

            Command::EvaluateVictory => {
                let snap = self.build_victory_snapshot();
                let mut events = Vec::new();
                for condition in self.state.victory_state.evaluate(&snap) {
                    events.push(Event::VictoryAchieved { condition });
                }
                Ok(events)
            }

            Command::ContinueAfterVictory | Command::ContinueSandbox => {
                self.state.victory_state.activate_sandbox_continue();
                self.state.sandbox_mode = true;
                Ok(vec![Event::SandboxContinued])
            }

            Command::InitVictoryConditions { conditions } => {
                self.state.victory_state = victory::VictoryState::new(conditions.clone());
                Ok(vec![])
            }

            // ── M1: Planet map ────────────────────────────────────────────────
            Command::SeedPlanet { seed, radius } => {
                let map = PlanetMap::generate(*seed, *radius);
                let cell_count = map.cells.len();
                self.state.planet_map = Some(map);
                Ok(vec![Event::PlanetSeeded {
                    seed: *seed,
                    radius: *radius,
                    cell_count,
                }])
            }

            // ── M1: Megaproject / Victory ─────────────────────────────────────
            Command::AdvanceMegaproject {
                project_id,
                progress,
            } => {
                let sys_cmd = system::SystemCommand::ContributeToMegaproject {
                    project_id: project_id.clone(),
                    resources: vec![],
                    #[allow(clippy::cast_precision_loss)]
                    research: *progress as f32,
                };
                let sys_events =
                    system::apply_system_command(&mut self.state.system_state, &sys_cmd)
                        .map_err(|e| EngineError::InvalidArgument(e.to_string()))?;

                let mut events: Vec<Event> = Vec::new();
                for sys_evt in &sys_events {
                    if let system::SystemEvent::MegaprojectCompleted { kind, .. } = sys_evt {
                        if *kind == system::MegaprojectKind::InterstellarExpedition {
                            self.state.expedition_launched = true;
                        }
                    }
                }

                if self.state.expedition_launched && self.state.victory.is_none() {
                    let snap = self.build_victory_snapshot();
                    for condition in self.state.victory_state.evaluate(&snap) {
                        self.state.victory = Some(condition.clone());
                        events.push(Event::VictoryAchieved { condition });
                    }
                }

                Ok(events)
            }

            // ── M1: System zoom dispatch ──────────────────────────────────────
            Command::System(sys_cmd) => {
                let sys_events =
                    system::apply_system_command(&mut self.state.system_state, sys_cmd)
                        .map_err(|e| EngineError::InvalidArgument(e.to_string()))?;

                // Propagate expedition-launch side-effect from system events.
                for sys_evt in &sys_events {
                    if let system::SystemEvent::MegaprojectCompleted { kind, .. } = sys_evt {
                        if *kind == system::MegaprojectKind::InterstellarExpedition {
                            self.state.expedition_launched = true;
                        }
                    }
                }

                let mut events: Vec<Event> = sys_events.into_iter().map(Event::System).collect();

                if self.state.expedition_launched && self.state.victory.is_none() {
                    let snap = self.build_victory_snapshot();
                    for condition in self.state.victory_state.evaluate(&snap) {
                        self.state.victory = Some(condition.clone());
                        events.push(Event::VictoryAchieved { condition });
                    }
                }

                Ok(events)
            }

            // ── M1: Research direction ────────────────────────────────────────
            Command::ResearchTech { tech_id } => {
                let registry = self.state.tech_registry.as_ref().ok_or_else(|| {
                    EngineError::InvalidArgument("no tech registry loaded".into())
                })?;
                let def = registry.get(tech_id).ok_or_else(|| {
                    EngineError::InvalidArgument(format!("unknown tech '{tech_id}'"))
                })?;
                if !self.state.tech_state.prerequisites_met(def) {
                    return Err(EngineError::InvalidArgument(format!(
                        "prerequisites not met for tech '{tech_id}'"
                    )));
                }
                self.state.tech_state.set_current_project(tech_id.clone());
                Ok(vec![Event::ResearchStarted {
                    tech_id: tech_id.clone(),
                }])
            }

            Command::EnqueueResearch { tech_id } => {
                let registry = self.state.tech_registry.as_ref().ok_or_else(|| {
                    EngineError::InvalidArgument("no tech registry loaded".into())
                })?;
                let _ = registry.get(tech_id).ok_or_else(|| {
                    EngineError::InvalidArgument(format!("unknown tech '{tech_id}'"))
                })?;
                self.state.tech_state.enqueue(tech_id.clone());
                Ok(vec![Event::ResearchQueued {
                    tech_id: tech_id.clone(),
                }])
            }

            Command::CancelResearch => {
                self.state.tech_state.current_project = None;
                self.state.tech_state.progress = 0.0;
                self.state.tech_state.research_queue.clear();
                Ok(vec![Event::ResearchCancelled])
            }

            // ── M1: Infrastructure ────────────────────────────────────────
            Command::BuildInfrastructure {
                from_colony,
                to_colony,
                infra_type,
            } => {
                // Validate both colonies exist.
                self.find_colony_index(*from_colony)?;
                self.find_colony_index(*to_colony)?;
                // Require a planet map and that both colonies are placed on it.
                let pm = self
                    .state
                    .planet_map
                    .as_mut()
                    .ok_or(EngineError::NoPlanetMap)?;
                let edge = pm
                    .add_edge(*from_colony, *to_colony, *infra_type)
                    .map_err(|e| EngineError::InvalidState(e.to_string()))?;
                let cost = edge.cost;
                let throughput = f64::from(edge.throughput);
                // Wire up a trade route so auto-flow activates.
                let route = TradeRoute::new(*from_colony, *to_colony, throughput);
                let route_id = route.id;
                self.state.trade_network.add_route(route);
                // Track the route so DemolishInfrastructure can remove it.
                let key = canonical_infra_key(*from_colony, *to_colony);
                self.state.infra_routes.insert(key, route_id);
                Ok(vec![Event::InfrastructureBuilt {
                    from_colony: *from_colony,
                    to_colony: *to_colony,
                    infra_type: *infra_type,
                    cost,
                    route_id,
                }])
            }

            Command::DemolishInfrastructure {
                from_colony,
                to_colony,
            } => {
                // Validate both colonies exist.
                self.find_colony_index(*from_colony)?;
                self.find_colony_index(*to_colony)?;
                let key = canonical_infra_key(*from_colony, *to_colony);
                let route_id = self.state.infra_routes.remove(&key).ok_or_else(|| {
                    EngineError::InvalidArgument(format!(
                        "no infrastructure edge between colonies {from_colony} and {to_colony}"
                    ))
                })?;
                // Remove the edge from the planet map if one is seeded.
                if let Some(pm) = self.state.planet_map.as_mut() {
                    pm.edges.retain(|e| {
                        !(e.from == *from_colony && e.to == *to_colony
                            || e.from == *to_colony && e.to == *from_colony)
                    });
                }
                // Remove the backing trade route.
                self.state.trade_network.remove_route(route_id);
                Ok(vec![Event::InfrastructureDemolished {
                    from_colony: *from_colony,
                    to_colony: *to_colony,
                    route_id,
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
    #[allow(clippy::too_many_lines)]
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

            Query::ColonyScreen { colony_id } => {
                let idx = self.find_colony_index(*colony_id)?;
                let c = &self.state.colonies[idx];
                let p = &self.state.populations[idx];
                let buildings = c
                    .buildings
                    .iter()
                    .map(|b| ui::BuildingRow {
                        building_type: b.building_type.clone(),
                        labour_assigned: 0,
                        slot_cost: b.slot_cost,
                        full_capacity: true,
                    })
                    .collect();
                let stockpile = c
                    .pool
                    .commodity_ids()
                    .map(|cid| {
                        let cap = c.pool.capacity(cid);
                        ui::StockpileRow {
                            commodity_id: cid.to_string(),
                            amount: c.pool.amount(cid),
                            capacity: if cap.is_finite() { Some(cap) } else { None },
                            net_per_turn: c.pool.delta(cid),
                        }
                    })
                    .collect();
                let construction_queue = c
                    .build_queue
                    .projects
                    .iter()
                    .map(|proj| ui::ConstructionQueueRow {
                        project_id: proj.id,
                        building_type: proj.building_type.clone(),
                        turns_completed: proj.turns_completed,
                        turns_total: proj.total_turns,
                        slot_cost: proj.slot_cost,
                    })
                    .collect();
                let manual_override = self.state.directive_store.is_manual_override(*colony_id);
                Ok(QueryResult::ColonyScreen(ui::ColonyScreenData {
                    colony_id: c.id,
                    name: c.name.clone(),
                    population: p.count,
                    stability: p.stability,
                    slots_used: c.slots_used(),
                    slot_capacity: c.slot_capacity,
                    labour_available: p.available_labor(),
                    labour_total: p.count * 0.5,
                    buildings,
                    stockpile,
                    construction_queue,
                    manual_override,
                }))
            }

            Query::PlanetMap => {
                let Some(pm) = self.state.planet_map.as_ref() else {
                    // No planet seeded yet — return an empty map.
                    return Ok(QueryResult::PlanetMap(ui::PlanetMapData {
                        planet_name: "Unknown Planet".to_string(),
                        hexes: Vec::new(),
                        colony_nodes: Vec::new(),
                        infrastructure: Vec::new(),
                    }));
                };

                // Map map::HexCell → ui::HexCell.
                let hexes: Vec<ui::HexCell> = pm
                    .cells
                    .values()
                    .map(|cell| ui::HexCell {
                        q: cell.coord.q,
                        r: cell.coord.r,
                        biome: format!("{:?}", cell.biome).to_lowercase(),
                        deposits: cell
                            .deposits
                            .iter()
                            .map(|d| d.commodity_id.clone())
                            .collect(),
                    })
                    .collect();

                // Map map::ColonyNode → ui::ColonyNode, joining with colony/population data.
                let colony_nodes: Vec<ui::ColonyNode> = pm
                    .colonies
                    .iter()
                    .filter_map(|node| {
                        let idx = self
                            .state
                            .colonies
                            .iter()
                            .position(|c| c.id == node.colony_id)?;
                        let c = &self.state.colonies[idx];
                        let p = &self.state.populations[idx];
                        Some(ui::ColonyNode {
                            colony_id: node.colony_id,
                            name: c.name.clone(),
                            q: node.coord.q,
                            r: node.coord.r,
                            population: p.count,
                        })
                    })
                    .collect();

                // Map map::InfraEdge → ui::InfraEdge.
                let infrastructure: Vec<ui::InfraEdge> = pm
                    .edges
                    .iter()
                    .map(|edge| ui::InfraEdge {
                        from_colony_id: edge.from,
                        to_colony_id: edge.to,
                        kind: format!("{:?}", edge.infra_type).to_lowercase(),
                        throughput: edge.throughput / edge.infra_type.base_throughput(),
                    })
                    .collect();

                Ok(QueryResult::PlanetMap(ui::PlanetMapData {
                    planet_name: "Unknown Planet".to_string(),
                    hexes,
                    colony_nodes,
                    infrastructure,
                }))
            }

            Query::InterruptDigest => {
                let digest =
                    self.last_advance_digest
                        .clone()
                        .unwrap_or_else(|| ui::InterruptDigestData {
                            stopped_at_turn: self.state.sol,
                            turns_advanced: 0,
                            halting_interrupt: None,
                            digest_items: Vec::new(),
                            active_filter: ui::DigestFilter::new(),
                        });
                Ok(QueryResult::InterruptDigest(digest))
            }

            Query::TimeControl => Ok(QueryResult::TimeControl(ui::TimeControlState {
                current_sol: self.state.sol,
                current_month: self.state.month,
                threshold: self.interrupt_threshold,
                max_advance_turns: self.max_advance_turns,
            })),

            Query::DifficultyStatus => {
                Ok(QueryResult::DifficultyStatus(self.state.difficulty_preset))
            }

            Query::MenaceStatus => Ok(QueryResult::MenaceStatus(self.state.menace_state.clone())),

            Query::VictoryStatus => Ok(QueryResult::VictoryStatus(
                self.state.victory_state.conditions.clone(),
            )),
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

            // Update per-colony stability + population trackers after each sol.
            let colony_ids: Vec<_> = self.state.colonies.iter().map(|c| c.id).collect();
            for (i, colony_id) in colony_ids.iter().enumerate() {
                let stability = self.state.populations[i].stability;
                let count = self.state.populations[i].count;
                self.state
                    .stability_trackers
                    .entry(*colony_id)
                    .or_default()
                    .push(stability);
                self.state
                    .population_trackers
                    .entry(*colony_id)
                    .or_default()
                    .push(count);
            }

            let turn_interrupts = self.collect_turn_interrupts(&events);

            for irq in turn_interrupts {
                if irq.tier >= threshold {
                    let digest_items: Vec<ui::DigestItem> = digest
                        .iter()
                        .map(|i| ui::DigestItem {
                            interrupt: i.clone(),
                            acknowledged: false,
                        })
                        .collect();
                    self.last_advance_digest = Some(ui::InterruptDigestData {
                        stopped_at_turn: self.state.sol,
                        turns_advanced: n,
                        halting_interrupt: Some(irq.clone()),
                        digest_items,
                        active_filter: ui::DigestFilter::new(),
                    });
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

        let digest_items: Vec<ui::DigestItem> = digest
            .iter()
            .map(|i| ui::DigestItem {
                interrupt: i.clone(),
                acknowledged: false,
            })
            .collect();
        self.last_advance_digest = Some(ui::InterruptDigestData {
            stopped_at_turn: self.state.sol,
            turns_advanced: n,
            halting_interrupt: None,
            digest_items,
            active_filter: ui::DigestFilter::new(),
        });
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

        // Tech completions → Notable.
        for ev in events {
            if let Event::TechUnlocked { tech_id } = ev {
                interrupts.push(Interrupt::new(
                    Tier::Notable,
                    InterruptSource::TechUnlocked,
                    None,
                    format!("Tech researched: {tech_id}"),
                ));
            }
        }

        // StabilityCritical — stability already at or below the crisis floor → Urgent.
        for (colony, pop) in self
            .state
            .colonies
            .iter()
            .zip(self.state.populations.iter())
        {
            if pop.stability <= STABILITY_CRISIS_FLOOR {
                interrupts.push(Interrupt::new(
                    Tier::Urgent,
                    InterruptSource::StabilityCritical(colony.id),
                    Some(colony.id),
                    format!(
                        "Colony '{}': stability critical ({:.0}%)",
                        colony.name,
                        pop.stability * 100.0
                    ),
                ));
            }
        }

        // EventFired — named hazard / environmental events → Urgent.
        for ev in events {
            if let Event::HazardFired { event_id } = ev {
                interrupts.push(Interrupt::new(
                    Tier::Urgent,
                    InterruptSource::EventFired(event_id.clone()),
                    None,
                    format!("Hazard event fired: {event_id}"),
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

            // Predictive population warnings → Urgent.
            if let Some(pop_tracker) = self.state.population_trackers.get(&colony.id) {
                if let Some(eta) = pop_tracker.eta_to_floor(POPULATION_CRISIS_FLOOR) {
                    if eta <= POPULATION_WARNING_ETA {
                        interrupts.push(Interrupt::new(
                            Tier::Urgent,
                            InterruptSource::PredictiveWarning {
                                quantity: pop.count,
                                eta_turns: eta,
                            },
                            Some(colony.id),
                            format!(
                                "Colony '{}': population declining — critical low in ~{eta} turns \
                                 (current: {:.0})",
                                colony.name, pop.count
                            ),
                        ));
                    }
                }
            }
        }

        interrupts
    }

    /// Build a [`victory::VictorySnapshot`] from the current engine state.
    ///
    /// Computes tech-tree completion and trade dominance volumes in addition to
    /// the basic metrics already tracked on [`GameState`].
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn build_victory_snapshot(&self) -> victory::VictorySnapshot {
        // Tech tree completion: all techs in registry must be researched.
        let tech_tree_complete = self.state.tech_registry.as_ref().is_some_and(|reg| {
            reg.all()
                .all(|def| self.state.tech_state.is_researched(&def.id))
        });

        // Trade dominance volumes: every colony is considered player-controlled.
        // Total traded volume = sum of all TradeTransfer amounts recorded this turn.
        // We recompute by inspecting trade routes — all colonies are player colonies.
        // Because we don't store last turn's transfers here, we compute a snapshot
        // of current pool balances as a proxy: total across all pools per commodity.
        // The real measure comes from TradeTransfer amounts produced by run_trade_flow;
        // since we don't persist those, we sum the pool amounts as current holdings.
        // For the victory check we compute volumes from the live pools:
        // total_traded_volume tracks total held per commodity system-wide;
        // player_traded_volume equals total (all colonies are player-owned for now).
        let mut total_traded: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for colony in &self.state.colonies {
            for commodity_id in colony.pool.commodity_ids() {
                let amt = colony.pool.amount(commodity_id);
                *total_traded.entry(commodity_id.to_owned()).or_insert(0.0) += amt;
            }
        }
        // All colonies are player-controlled — player volume equals total.
        let player_traded = total_traded.clone();

        victory::VictorySnapshot {
            expedition_launched: self.state.expedition_launched,
            total_output: 0,
            total_population: self
                .state
                .populations
                .iter()
                .map(|p| p.count.max(0.0) as u64)
                .sum(),
            cumulative_research: self.state.cumulative_research,
            tech_tree_complete,
            total_traded_volume: total_traded,
            player_traded_volume: player_traded,
        }
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

/// Return a canonical `(from, to)` key for the infra-route map.
///
/// Ensures that `(a, b)` and `(b, a)` map to the same slot so bidirectional
/// lookup works without storing both orderings.
fn canonical_infra_key(a: ColonyId, b: ColonyId) -> (ColonyId, ColonyId) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
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

    // ─── UI query tests ──────────────────────────────────────────────────────

    /// Query::ColonyScreen returns correct data for a founded colony.
    #[test]
    fn query_colony_screen_returns_colony_data() {
        let mut engine = GameEngine::with_seed(0);
        let events = engine
            .apply(&Command::FoundColony {
                name: "UI Test Colony".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let result = engine.query(&Query::ColonyScreen { colony_id }).unwrap();
        match result {
            QueryResult::ColonyScreen(data) => {
                assert_eq!(data.name, "UI Test Colony");
                assert_eq!(data.colony_id, colony_id);
                assert!(data.population > 0.0);
                assert_eq!(data.slot_capacity, colony::BASE_SLOT_CAPACITY);
            }
            other => panic!("expected ColonyScreen, got {other:?}"),
        }
    }

    /// Query::ColonyScreen returns error for unknown colony.
    #[test]
    fn query_colony_screen_unknown_colony_returns_error() {
        let engine = GameEngine::with_seed(0);
        let result = engine.query(&Query::ColonyScreen {
            colony_id: uuid::Uuid::new_v4(),
        });
        assert!(matches!(result, Err(EngineError::ColonyNotFound(_))));
    }

    /// Query::PlanetMap returns colony nodes for colonies founded at sites.
    #[test]
    fn query_planet_map_returns_colony_nodes() {
        let mut engine = GameEngine::with_seed(0);
        // Seed a planet so FoundColonyAtSite has a map to look up.
        engine
            .apply(&Command::SeedPlanet {
                seed: 10,
                radius: 5,
            })
            .unwrap();

        // Pick two distinct habitable sites.
        let pm = engine.state.planet_map.as_ref().unwrap();
        let mut habitable_sites: Vec<trade::SiteId> = pm
            .sites
            .iter()
            .filter(|(_, &coord)| pm.cells.get(&coord).map_or(false, |c| c.is_habitable()))
            .map(|(&sid, _)| sid)
            .take(2)
            .collect();
        assert!(
            habitable_sites.len() >= 2,
            "need at least 2 habitable sites for this test"
        );
        let site_a = habitable_sites.pop().unwrap();
        let site_b = habitable_sites.pop().unwrap();
        drop(pm);

        engine
            .apply(&Command::FoundColonyAtSite {
                name: "Alpha".into(),
                starting_population: 10,
                site_id: site_a,
                focus: None,
            })
            .unwrap();
        engine
            .apply(&Command::FoundColonyAtSite {
                name: "Beta".into(),
                starting_population: 20,
                site_id: site_b,
                focus: None,
            })
            .unwrap();

        let result = engine.query(&Query::PlanetMap).unwrap();
        match result {
            QueryResult::PlanetMap(map) => {
                assert_eq!(map.colony_nodes.len(), 2);
                // Names may appear in any order — just check both are present.
                let names: std::collections::HashSet<_> =
                    map.colony_nodes.iter().map(|n| n.name.as_str()).collect();
                assert!(names.contains("Alpha"));
                assert!(names.contains("Beta"));
            }
            other => panic!("expected PlanetMap, got {other:?}"),
        }
    }

    /// Query::InterruptDigest returns empty digest before any advance.
    #[test]
    fn query_interrupt_digest_empty_before_advance() {
        let engine = GameEngine::with_seed(0);
        let result = engine.query(&Query::InterruptDigest).unwrap();
        match result {
            QueryResult::InterruptDigest(d) => {
                assert_eq!(d.turns_advanced, 0);
                assert!(d.digest_items.is_empty());
                assert!(d.halting_interrupt.is_none());
            }
            other => panic!("expected InterruptDigest, got {other:?}"),
        }
    }

    /// Query::InterruptDigest returns populated digest after advance.
    #[test]
    fn query_interrupt_digest_populated_after_advance() {
        let mut engine = GameEngine::with_seed(0);
        engine
            .advance_until_interrupted(3, interrupt::Tier::Blocking)
            .unwrap();
        let result = engine.query(&Query::InterruptDigest).unwrap();
        assert!(matches!(result, QueryResult::InterruptDigest(_)));
    }

    /// Query::TimeControl returns correct current sol and threshold.
    #[test]
    fn query_time_control_returns_current_state() {
        let mut engine = GameEngine::with_seed(0);
        engine.interrupt_threshold = interrupt::Tier::Urgent;
        engine.max_advance_turns = 5;
        engine.apply(&Command::AdvanceColonySol).unwrap();

        let result = engine.query(&Query::TimeControl).unwrap();
        match result {
            QueryResult::TimeControl(tc) => {
                assert_eq!(tc.current_sol, 1);
                assert!(matches!(tc.threshold, interrupt::Tier::Urgent));
                assert_eq!(tc.max_advance_turns, 5);
            }
            other => panic!("expected TimeControl, got {other:?}"),
        }
    }

    // ── inter-colony trade (engine-level) ──────────────────────────────────

    fn found_two_colonies(engine: &mut GameEngine) -> (ColonyId, ColonyId) {
        let evs_a = engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: 100,
            })
            .unwrap();
        let evs_b = engine
            .apply(&Command::FoundColony {
                name: "Beta".into(),
                starting_population: 100,
            })
            .unwrap();
        let id_a = match &evs_a[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!("expected ColonyFounded"),
        };
        let id_b = match &evs_b[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!("expected ColonyFounded"),
        };
        (id_a, id_b)
    }

    #[test]
    fn add_trade_route_emits_event() {
        let mut engine = GameEngine::new();
        let (a, b) = found_two_colonies(&mut engine);

        let evs = engine
            .apply(&Command::AddTradeRoute {
                colony_a: a,
                colony_b: b,
                throughput_cap: 50.0,
            })
            .unwrap();

        assert!(matches!(evs[0], Event::TradeRouteAdded { .. }));
        assert_eq!(engine.state.trade_network.routes.len(), 1);
    }

    #[test]
    fn remove_trade_route_emits_event() {
        let mut engine = GameEngine::new();
        let (a, b) = found_two_colonies(&mut engine);

        let evs = engine
            .apply(&Command::AddTradeRoute {
                colony_a: a,
                colony_b: b,
                throughput_cap: 50.0,
            })
            .unwrap();
        let route_id = match &evs[0] {
            Event::TradeRouteAdded { route_id, .. } => *route_id,
            _ => panic!(),
        };

        let rm_evs = engine
            .apply(&Command::RemoveTradeRoute { route_id })
            .unwrap();
        assert!(matches!(rm_evs[0], Event::TradeRouteRemoved { .. }));
        assert!(engine.state.trade_network.routes.is_empty());
    }

    #[test]
    fn set_and_clear_trade_override_emits_events() {
        let mut engine = GameEngine::new();
        let (a, _b) = found_two_colonies(&mut engine);

        let set_evs = engine
            .apply(&Command::SetTradeOverride {
                colony_id: a,
                commodity_id: "food".into(),
                suppress_auto: true,
                cap: None,
            })
            .unwrap();
        assert!(matches!(set_evs[0], Event::TradeOverrideSet { .. }));

        let clr_evs = engine
            .apply(&Command::ClearTradeOverride {
                colony_id: a,
                commodity_id: "food".into(),
            })
            .unwrap();
        assert!(matches!(clr_evs[0], Event::TradeOverrideCleared { .. }));
    }

    #[test]
    fn found_colony_at_site_emits_event_with_site_id() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet {
                seed: 77,
                radius: 3,
            })
            .unwrap();
        let pm = engine.state.planet_map.as_ref().unwrap();
        let best = pm.best_landing_site().unwrap();
        let site = *pm
            .sites
            .iter()
            .find(|(_, &c)| c == best)
            .map(|(id, _)| id)
            .unwrap();
        drop(pm);

        let evs = engine
            .apply(&Command::FoundColonyAtSite {
                name: "Nova Camp".into(),
                starting_population: 50,
                site_id: site,
                focus: Some("mining".into()),
            })
            .unwrap();

        assert!(
            matches!(
                &evs[0],
                Event::ColonyFoundedAtSite { site_id, focus, .. }
                    if *site_id == site && focus.as_deref() == Some("mining")
            ),
            "expected ColonyFoundedAtSite with correct site_id and focus"
        );
        assert_eq!(engine.state.colonies.len(), 1);
    }

    // ── Phase 7: Population dynamics, migration, immigration waves ────────────

    /// Done-when: growth under good conditions (needs fully met, high stability).
    #[test]
    fn growth_under_good_conditions() {
        use crate::needs::{NeedDef, NeedScaling, NeedsConfig};

        let mut engine = GameEngine::with_seed(42);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Thriving Base".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Configure minimal needs that we can easily satisfy.
        engine.state.needs_config = Some(NeedsConfig {
            needs: vec![
                NeedDef {
                    commodity_id: "food".into(),
                    scaling: NeedScaling::PerCapita { rate: 0.1 },
                    weight: 1.0,
                },
                NeedDef {
                    commodity_id: "housing".into(),
                    scaling: NeedScaling::Housing,
                    weight: 0.8,
                },
            ],
            stability_recovery_rate: 0.05,
            stability_decay_rate: 0.10,
            growth_stability_threshold: 0.70,
            growth_rate: 0.005,
            decline_stability_threshold: 0.30,
            decline_rate: 0.001,
            emigration_stability_floor: 0.25,
            voluntary_emigration_rate: 0.03,
        });

        let idx = engine.find_colony_index(colony_id).unwrap();
        let initial_pop = engine.state.populations[idx].count;

        // Seed abundant food and housing for 10 turns.
        for _ in 0..10 {
            engine.state.colonies[idx].pool.deposit("food", 1000.0);
            engine.state.colonies[idx].pool.deposit("housing", 500.0);
            engine.apply(&Command::AdvanceColonySol).unwrap();
        }

        let final_pop = engine.state.populations[idx].count;
        assert!(
            final_pop > initial_pop,
            "population should grow under good conditions: initial={initial_pop}, final={final_pop}"
        );
    }

    /// Done-when: stability decline under starvation.
    #[test]
    fn stability_declines_under_starvation() {
        use crate::needs::NeedsConfig;

        let mut engine = GameEngine::with_seed(42);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Starved Base".into(),
                starting_population: 200,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Enable needs with no supplies.
        engine.state.needs_config = Some(NeedsConfig::default_survival());
        let idx = engine.find_colony_index(colony_id).unwrap();
        let initial_stability = engine.state.populations[idx].stability;

        // Advance 20 turns with no food, water, etc.
        for _ in 0..20 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
        }

        let final_stability = engine.state.populations[idx].stability;
        assert!(
            final_stability < initial_stability,
            "stability should decline under starvation: initial={initial_stability}, final={final_stability}"
        );
    }

    /// Done-when: migration flow — auto pull-flow moves colonists toward more attractive colony.
    #[test]
    fn auto_migration_flow_moves_colonists_toward_attractive_colony() {
        let mut engine = GameEngine::with_seed(42);

        // Found two colonies: one stable with housing room, one unstable.
        let e1 = engine
            .apply(&Command::FoundColony {
                name: "Thriving".into(),
                starting_population: 300,
            })
            .unwrap();
        let e2 = engine
            .apply(&Command::FoundColony {
                name: "Struggling".into(),
                starting_population: 300,
            })
            .unwrap();
        let id_thriving = match &e1[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!(),
        };
        let id_struggling = match &e2[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!(),
        };

        // Make "Thriving" very attractive: high housing headroom, full stability.
        let idx_t = engine.find_colony_index(id_thriving).unwrap();
        engine.state.colonies[idx_t].pool.deposit("housing", 1000.0);
        engine.state.populations[idx_t].stability = 1.0;

        // Make "Struggling" unattractive: low stability, no housing.
        let idx_s = engine.find_colony_index(id_struggling).unwrap();
        engine.state.populations[idx_s].stability = 0.1;
        let pop_before_struggling = engine.state.populations[idx_s].count;
        let pop_before_thriving = engine.state.populations[idx_t].count;

        // Run auto migration.
        let events = engine.apply(&Command::RunAutoMigration).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::AutoMigrationQueued { .. })),
            "RunAutoMigration should emit AutoMigrationQueued event"
        );

        // At least one flow should have been queued.
        let &Event::AutoMigrationQueued {
            flow_count,
            total_in_transit,
        } = events
            .iter()
            .find(|e| matches!(e, Event::AutoMigrationQueued { .. }))
            .unwrap()
        else {
            panic!()
        };
        assert!(flow_count > 0, "should queue at least one migration flow");
        assert!(total_in_transit >= 1.0, "should have colonists in transit");

        // Struggling colony's population should have been reduced (colonists departed).
        let pop_after_struggling = engine.state.populations[idx_s].count;
        assert!(
            pop_after_struggling < pop_before_struggling,
            "struggling colony should have lost colonists to migration"
        );

        // Resolve the migrations (transit_turns = 1, so one tick arrives them).
        let arrive_events = engine.apply(&Command::ResolvePendingMigrations).unwrap();
        assert!(
            arrive_events
                .iter()
                .any(|e| matches!(e, Event::MigrationArrived { .. })),
            "ResolvePendingMigrations should emit MigrationArrived event"
        );

        let pop_after_thriving = engine.state.populations[idx_t].count;
        assert!(
            pop_after_thriving > pop_before_thriving,
            "thriving colony should have gained colonists: before={pop_before_thriving}, after={pop_after_thriving}"
        );
    }

    /// Done-when: predictive warning timing — fires before crash, not on it.
    /// (Already tested in advance_halts_on_urgent_interrupt; this exercises
    ///  population-decline path specifically.)
    #[test]
    fn predictive_population_warning_fires_before_colony_empties() {
        use crate::interrupt::{AdvanceResult, Tier};

        let mut engine = GameEngine::with_seed(99);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Dying Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Pre-load a steep declining population trajectory; current value must
        // be *above* POPULATION_CRISIS_FLOOR (10) so the ETA can be computed.
        let tracker = engine
            .state
            .population_trackers
            .entry(colony_id)
            .or_default();
        for count in [100.0f32, 70.0, 45.0, 25.0, 15.0] {
            tracker.push(count);
        }
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.populations[idx].count = 15.0;

        // Also set up a stability tracker so the advance doesn't halt on stability.
        // Keep stability stable to isolate the population warning.
        let stab_tracker = engine
            .state
            .stability_trackers
            .entry(colony_id)
            .or_default();
        for s in [0.9f32, 0.9, 0.9, 0.9, 0.9] {
            stab_tracker.push(s);
        }

        let result = engine.advance_until_interrupted(20, Tier::Urgent).unwrap();

        // The advance should halt due to the population predictive warning.
        assert!(
            matches!(result, AdvanceResult::Halted { .. }),
            "expected Halted on population warning"
        );
    }

    /// Done-when: immigration wave — off-map colonists arrive at gateway colony.
    #[test]
    fn immigration_wave_arrives_at_gateway_colony() {
        let mut engine = GameEngine::with_seed(42);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Gateway".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;
        let idx = engine.find_colony_index(colony_id).unwrap();
        let pop_before = engine.state.populations[idx].count;

        // Schedule a wave of 50 arriving in 1 turn.
        engine
            .apply(&Command::ScheduleImmigrationWave {
                colony_id,
                count: 50.0,
                transit_turns: 1,
            })
            .unwrap();

        // One tick of resolve — wave arrives.
        let arrive_events = engine.apply(&Command::ResolvePendingMigrations).unwrap();
        let arrived = arrive_events.iter().find(|e| {
            matches!(
                e,
                Event::MigrationArrived {
                    from_colony: None,
                    ..
                }
            )
        });
        assert!(
            arrived.is_some(),
            "immigration wave should produce MigrationArrived with from_colony=None"
        );

        let pop_after = engine.state.populations[idx].count;
        assert!(
            (pop_after - (pop_before + 50.0)).abs() < 1.0,
            "population should have increased by 50 from immigration wave; before={pop_before}, after={pop_after}"
        );
    }

    /// Done-when: evacuation displaces problem to receiver (overcrowding → stability hit).
    #[test]
    fn evacuation_displaces_to_receiver_with_overcrowding_penalty() {
        let mut engine = GameEngine::with_seed(42);

        let e1 = engine
            .apply(&Command::FoundColony {
                name: "Crisis Colony".into(),
                starting_population: 200,
            })
            .unwrap();
        let e2 = engine
            .apply(&Command::FoundColony {
                name: "Receiver".into(),
                starting_population: 50,
            })
            .unwrap();
        let id_crisis = match &e1[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!(),
        };
        let id_recv = match &e2[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!(),
        };

        let idx_c = engine.find_colony_index(id_crisis).unwrap();
        let idx_r = engine.find_colony_index(id_recv).unwrap();

        // Give receiver very limited housing so evacuation causes overcrowding.
        engine.state.colonies[idx_r].pool.deposit("housing", 30.0);
        let recv_stability_before = engine.state.populations[idx_r].stability;
        let crisis_stability_before = engine.state.populations[idx_c].stability;

        // Evacuate 50 % of crisis colony.
        engine
            .apply(&Command::EvacuateColony {
                from_colony: id_crisis,
                to_colony: id_recv,
                fraction: 0.5,
                transit_turns: 1,
            })
            .unwrap();

        // Sending colony gets stability penalty immediately.
        let crisis_stability_after_departure = engine.state.populations[idx_c].stability;
        assert!(
            crisis_stability_after_departure < crisis_stability_before,
            "evacuation departure should cost stability at source"
        );

        // Resolve arrivals.
        engine.apply(&Command::ResolvePendingMigrations).unwrap();

        let recv_stability_after = engine.state.populations[idx_r].stability;
        assert!(
            recv_stability_after < recv_stability_before,
            "overcrowded receiver should lose stability: before={recv_stability_before}, after={recv_stability_after}"
        );
    }

    #[test]
    fn trade_flow_runs_on_strategic_month_via_engine() {
        // Advance 30 sols with a trade route; goods should move once the
        // strategic-month sub-pipeline fires.
        let mut engine = GameEngine::new();
        let (a, b) = found_two_colonies(&mut engine);

        // Seed food into colony A's pool.
        {
            let idx = engine.find_colony_index(a).unwrap();
            engine.state.colonies[idx].pool.deposit("food", 100.0);
        }

        engine
            .apply(&Command::AddTradeRoute {
                colony_a: a,
                colony_b: b,
                throughput_cap: 20.0,
            })
            .unwrap();

        // Advance 30 sols so the strategic-month fires.
        for _ in 0..30 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
        }

        let idx_b = engine.find_colony_index(b).unwrap();
        let food_at_b = engine.state.colonies[idx_b].pool.amount("food");
        assert!(
            food_at_b > 0.0,
            "colony B should have received some food via trade; got {food_at_b}"
        );
    }

    // ── M1: ResearchTech / EnqueueResearch / CancelResearch tests ────────────

    /// Build a minimal TechRegistry with two techs: "alpha" (no prereqs) and
    /// "beta" (requires alpha).
    fn make_tech_engine() -> GameEngine {
        use crate::tech::{TechDef, TechEffect, TechRegistry};
        let defs = vec![
            TechDef {
                id: "alpha".into(),
                display_name: "Alpha".into(),
                prerequisites: vec![],
                research_cost: 10.0,
                effects: vec![TechEffect::UnlockCapability {
                    capability_id: "cap_alpha".into(),
                }],
            },
            TechDef {
                id: "beta".into(),
                display_name: "Beta".into(),
                prerequisites: vec!["alpha".into()],
                research_cost: 20.0,
                effects: vec![TechEffect::UnlockCapability {
                    capability_id: "cap_beta".into(),
                }],
            },
        ];
        let registry = TechRegistry::build(defs).unwrap();
        let mut engine = GameEngine::new();
        engine.state.tech_registry = Some(registry);
        engine
    }

    #[test]
    fn research_tech_sets_current_project_and_emits_event() {
        let mut engine = make_tech_engine();
        let events = engine
            .apply(&Command::ResearchTech {
                tech_id: "alpha".into(),
            })
            .unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ResearchStarted { tech_id } if tech_id == "alpha")));
        assert_eq!(
            engine.state.tech_state.current_project.as_deref(),
            Some("alpha")
        );
    }

    #[test]
    fn research_tech_rejects_unknown_tech() {
        let mut engine = make_tech_engine();
        let err = engine
            .apply(&Command::ResearchTech {
                tech_id: "nonexistent".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn research_tech_rejects_unmet_prerequisites() {
        let mut engine = make_tech_engine();
        // beta requires alpha, which is not yet researched
        let err = engine
            .apply(&Command::ResearchTech {
                tech_id: "beta".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn research_tech_allows_tech_after_prereq_met() {
        let mut engine = make_tech_engine();
        engine.state.tech_state.researched.insert("alpha".into());
        let events = engine
            .apply(&Command::ResearchTech {
                tech_id: "beta".into(),
            })
            .unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ResearchStarted { tech_id } if tech_id == "beta")));
    }

    #[test]
    fn enqueue_research_pushes_to_queue_and_emits_event() {
        let mut engine = make_tech_engine();
        let events = engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap();
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ResearchQueued { tech_id } if tech_id == "alpha")));
        assert_eq!(engine.state.tech_state.research_queue.len(), 1);
    }

    #[test]
    fn enqueue_research_rejects_unknown_tech() {
        let mut engine = make_tech_engine();
        let err = engine
            .apply(&Command::EnqueueResearch {
                tech_id: "ghost".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn cancel_research_clears_queue_and_project() {
        let mut engine = make_tech_engine();
        engine
            .apply(&Command::ResearchTech {
                tech_id: "alpha".into(),
            })
            .unwrap();
        engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap();
        let events = engine.apply(&Command::CancelResearch).unwrap();
        assert!(events.iter().any(|e| matches!(e, Event::ResearchCancelled)));
        assert!(engine.state.tech_state.current_project.is_none());
        assert!(engine.state.tech_state.research_queue.is_empty());
    }

    // ── M1: Planet map integration tests ─────────────────────────────────

    #[test]
    fn seed_planet_stores_map_in_game_state() {
        let mut engine = GameEngine::new();
        assert!(engine.state.planet_map.is_none());
        let events = engine
            .apply(&Command::SeedPlanet {
                seed: 42,
                radius: 3,
            })
            .unwrap();
        assert!(
            engine.state.planet_map.is_some(),
            "planet_map must be Some after SeedPlanet"
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::PlanetSeeded { seed: 42, .. })),
            "PlanetSeeded event must be emitted"
        );
        // Cell count = 3r²+3r+1 = 3*9+9+1 = 37
        if let Event::PlanetSeeded { cell_count, .. } = &events[0] {
            assert_eq!(*cell_count, 37);
        }
    }

    #[test]
    fn found_colony_at_site_requires_planet_map() {
        let mut engine = GameEngine::new();
        // No planet seeded — must return NoPlanetMap.
        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Alpha".into(),
            starting_population: 100,
            site_id: trade::SiteId::new(),
            focus: None,
        });
        assert!(
            matches!(result, Err(EngineError::NoPlanetMap)),
            "expected NoPlanetMap, got {result:?}"
        );
    }

    // ── M1: Megaproject / Victory tests ──────────────────────────────────────

    /// Register an `InterstellarExpedition` megaproject directly on system state
    /// and return the [`system::MegaprojectId`] assigned to it.
    fn register_interstellar_expedition(
        engine: &mut GameEngine,
        research_cost: f32,
    ) -> system::MegaprojectId {
        let register_cmd = system::SystemCommand::RegisterMegaproject {
            name: "Interstellar Expedition".to_string(),
            kind: system::MegaprojectKind::InterstellarExpedition,
            milestones: vec![system::MilestoneSpec {
                label: "Phase 1".to_string(),
                resource_cost: vec![],
                research_cost,
            }],
        };
        let events =
            system::apply_system_command(&mut engine.state.system_state, &register_cmd).unwrap();
        for evt in events {
            if let system::SystemEvent::MegaprojectRegistered { project_id, .. } = evt {
                return project_id;
            }
        }
        panic!("no MegaprojectRegistered event returned");
    }

    #[test]
    fn advance_megaproject_to_completion_emits_victory_achieved() {
        let mut engine = GameEngine::new();
        let project_id = register_interstellar_expedition(&mut engine, 10.0);

        let events = engine
            .apply(&Command::AdvanceMegaproject {
                project_id: project_id.clone(),
                progress: 100,
            })
            .unwrap();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::VictoryAchieved { .. })),
            "expected VictoryAchieved event, got: {events:?}"
        );
    }

    #[test]
    fn found_colony_at_site_rejects_unknown_site() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet { seed: 7, radius: 3 })
            .unwrap();
        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Beta".into(),
            starting_population: 50,
            site_id: trade::SiteId::new(), // random UUID — not in map
            focus: None,
        });
        assert!(
            matches!(result, Err(EngineError::SiteNotFound(_))),
            "expected SiteNotFound, got {result:?}"
        );
    }

    #[test]
    fn found_colony_at_site_valid_site_registers_colony_on_map() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet {
                seed: 99,
                radius: 3,
            })
            .unwrap();
        // Pick the best landing site and find its SiteId.
        let pm = engine.state.planet_map.as_ref().unwrap();
        let best_coord = pm
            .best_landing_site()
            .expect("map must have habitable cells");
        let site_id = *pm
            .sites
            .iter()
            .find(|(_, &c)| c == best_coord)
            .map(|(id, _)| id)
            .expect("best landing site must have a SiteId");
        drop(pm);

        let events = engine
            .apply(&Command::FoundColonyAtSite {
                name: "Outpost Alpha".into(),
                starting_population: 200,
                site_id,
                focus: Some("mining".into()),
            })
            .unwrap();

        // Colony must be registered in GameState.
        assert_eq!(engine.state.colonies.len(), 1);
        // Colony must appear in planet_map.colonies.
        let pm = engine.state.planet_map.as_ref().unwrap();
        assert_eq!(pm.colonies.len(), 1);
        assert_eq!(pm.colonies[0].coord, best_coord);
        // Events must include ColonyFoundedAtSite and ColonyPlacedOnMap.
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ColonyFoundedAtSite { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ColonyPlacedOnMap { q, r, .. } if *q == best_coord.q && *r == best_coord.r)));
    }

    #[test]
    fn found_colony_at_site_rejects_duplicate_placement() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet {
                seed: 55,
                radius: 3,
            })
            .unwrap();
        let pm = engine.state.planet_map.as_ref().unwrap();
        let best_coord = pm.best_landing_site().unwrap();
        let site_id = *pm
            .sites
            .iter()
            .find(|(_, &c)| c == best_coord)
            .map(|(id, _)| id)
            .unwrap();
        drop(pm);

        engine
            .apply(&Command::FoundColonyAtSite {
                name: "First".into(),
                starting_population: 100,
                site_id,
                focus: None,
            })
            .unwrap();

        // Second colony at the same site must fail.
        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Second".into(),
            starting_population: 100,
            site_id,
            focus: None,
        });
        assert!(
            matches!(result, Err(EngineError::SiteOccupied)),
            "expected SiteOccupied, got {result:?}"
        );
    }

    #[test]
    fn planet_map_query_returns_real_hex_data_after_seed() {
        let mut engine = GameEngine::new();
        // Before seeding: empty result.
        let QueryResult::PlanetMap(empty) = engine.query(&Query::PlanetMap).unwrap() else {
            panic!("expected PlanetMap result");
        };
        assert!(empty.hexes.is_empty());
        assert!(empty.colony_nodes.is_empty());

        engine
            .apply(&Command::SeedPlanet { seed: 1, radius: 2 })
            .unwrap();

        let QueryResult::PlanetMap(data) = engine.query(&Query::PlanetMap).unwrap() else {
            panic!("expected PlanetMap result");
        };
        // radius 2: 3*4+6+1 = 19 cells
        assert_eq!(data.hexes.len(), 19, "radius 2 must yield 19 hex cells");
        assert!(data.colony_nodes.is_empty(), "no colonies placed yet");
    }

    #[test]
    fn planet_map_query_includes_colony_node_after_founding() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet { seed: 2, radius: 3 })
            .unwrap();
        let pm = engine.state.planet_map.as_ref().unwrap();
        let coord = pm.best_landing_site().unwrap();
        let site_id = *pm
            .sites
            .iter()
            .find(|(_, &c)| c == coord)
            .map(|(id, _)| id)
            .unwrap();
        drop(pm);
        engine
            .apply(&Command::FoundColonyAtSite {
                name: "Node Colony".into(),
                starting_population: 150,
                site_id,
                focus: None,
            })
            .unwrap();

        let QueryResult::PlanetMap(data) = engine.query(&Query::PlanetMap).unwrap() else {
            panic!("expected PlanetMap result");
        };
        assert_eq!(data.colony_nodes.len(), 1);
        let node = &data.colony_nodes[0];
        assert_eq!(node.q, coord.q);
        assert_eq!(node.r, coord.r);
        assert_eq!(node.name, "Node Colony");
    }

    #[test]
    fn victory_field_set_after_expedition_completes() {
        let mut engine = GameEngine::new();
        let project_id = register_interstellar_expedition(&mut engine, 10.0);

        engine
            .apply(&Command::AdvanceMegaproject {
                project_id,
                progress: 100,
            })
            .unwrap();

        assert!(
            engine.state.victory.is_some(),
            "GameState::victory should be Some after expedition completes"
        );
    }

    #[test]
    fn engine_returns_game_over_after_victory() {
        let mut engine = GameEngine::new();
        let project_id = register_interstellar_expedition(&mut engine, 10.0);

        engine
            .apply(&Command::AdvanceMegaproject {
                project_id,
                progress: 100,
            })
            .unwrap();

        let err = engine.apply(&Command::AdvanceColonySol).unwrap_err();
        assert!(
            matches!(err, EngineError::GameOver),
            "expected GameOver, got: {err:?}"
        );
    }

    #[test]
    fn sandbox_continue_allows_commands_after_victory() {
        let mut engine = GameEngine::new();
        let project_id = register_interstellar_expedition(&mut engine, 10.0);

        engine
            .apply(&Command::AdvanceMegaproject {
                project_id,
                progress: 100,
            })
            .unwrap();

        // Activate sandbox-continue.
        engine.apply(&Command::ContinueAfterVictory).unwrap();

        // Commands should now succeed.
        let result = engine.apply(&Command::AdvanceColonySol);
        assert!(
            result.is_ok(),
            "should be able to advance after sandbox continue"
        );
    }

    #[test]
    fn continue_sandbox_command_allows_commands_after_victory() {
        // Issue #96: Command::ContinueSandbox is the canonical name; verify it works.
        let mut engine = GameEngine::new();
        let project_id = register_interstellar_expedition(&mut engine, 10.0);

        engine
            .apply(&Command::AdvanceMegaproject {
                project_id,
                progress: 100,
            })
            .unwrap();

        // Engine blocks commands before sandbox mode.
        let err = engine.apply(&Command::AdvanceColonySol).unwrap_err();
        assert!(matches!(err, EngineError::GameOver), "expected GameOver before sandbox");

        // Activate via the issue-#96 canonical command name.
        let events = engine.apply(&Command::ContinueSandbox).unwrap();
        assert!(
            events.iter().any(|e| matches!(e, Event::SandboxContinued)),
            "expected SandboxContinued event"
        );

        // sandbox_mode top-level flag must be set.
        assert!(engine.state.sandbox_mode, "GameState::sandbox_mode should be true");

        // Commands now succeed without GameOver.
        let result = engine.apply(&Command::AdvanceColonySol);
        assert!(result.is_ok(), "commands should succeed in sandbox mode");

        // No further VictoryAchieved events emitted in sandbox mode.
        let advance_events = result.unwrap();
        assert!(
            !advance_events
                .iter()
                .any(|e| matches!(e, Event::VictoryAchieved { .. })),
            "VictoryAchieved must not be re-fired in sandbox mode"
        );
    }

    #[test]
    fn partial_megaproject_progress_does_not_emit_victory() {
        let mut engine = GameEngine::new();
        let project_id = register_interstellar_expedition(&mut engine, 100.0);

        let events = engine
            .apply(&Command::AdvanceMegaproject {
                project_id,
                progress: 10, // only 10 of 100 required
            })
            .unwrap();

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::VictoryAchieved { .. })),
            "should not emit VictoryAchieved for partial progress"
        );
        assert!(engine.state.victory.is_none());
    }

    // ── Issue #84: Infrastructure build/demolish ──────────────────────────────

    /// Helper: spin up an engine with a seeded planet and two colonies placed on it.
    fn setup_two_colony_map() -> (GameEngine, colony::ColonyId, colony::ColonyId) {
        let mut engine = GameEngine::new();
        // Seed a small map so we can find two habitable plains cells.
        engine
            .apply(&Command::SeedPlanet {
                seed: 42,
                radius: 3,
            })
            .unwrap();
        let pm = engine.state.planet_map.as_ref().unwrap();
        // Collect two habitable coords and their site IDs.
        let habitable: Vec<(trade::SiteId, map::HexCoord)> = pm
            .sites
            .iter()
            .filter(|(_, coord)| pm.cells.get(coord).map_or(false, |c| c.is_habitable()))
            .map(|(sid, coord)| (*sid, *coord))
            .take(2)
            .collect();
        assert!(
            habitable.len() >= 2,
            "map must have at least 2 habitable cells"
        );
        let (site_a, _coord_a) = habitable[0];
        let (site_b, _coord_b) = habitable[1];
        drop(pm);

        let events_a = engine
            .apply(&Command::FoundColonyAtSite {
                name: "Alpha".into(),
                starting_population: 100,
                site_id: site_a,
                focus: None,
            })
            .unwrap();
        let colony_a = events_a
            .iter()
            .find_map(|e| {
                if let Event::ColonyFoundedAtSite { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        let events_b = engine
            .apply(&Command::FoundColonyAtSite {
                name: "Beta".into(),
                starting_population: 80,
                site_id: site_b,
                focus: None,
            })
            .unwrap();
        let colony_b = events_b
            .iter()
            .find_map(|e| {
                if let Event::ColonyFoundedAtSite { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        (engine, colony_a, colony_b)
    }

    #[test]
    fn build_infrastructure_creates_trade_route() {
        let (mut engine, colony_a, colony_b) = setup_two_colony_map();

        let events = engine
            .apply(&Command::BuildInfrastructure {
                from_colony: colony_a,
                to_colony: colony_b,
                infra_type: map::InfraType::Road,
            })
            .unwrap();

        // Event emitted.
        let built = events.iter().find_map(|e| {
            if let Event::InfrastructureBuilt {
                from_colony,
                to_colony,
                infra_type,
                route_id,
                ..
            } = e
            {
                Some((*from_colony, *to_colony, *infra_type, *route_id))
            } else {
                None
            }
        });
        assert!(built.is_some(), "InfrastructureBuilt event must be emitted");
        let (fc, tc, it, route_id) = built.unwrap();
        assert_eq!(fc, colony_a);
        assert_eq!(tc, colony_b);
        assert_eq!(it, map::InfraType::Road);

        // Edge stored on planet map.
        let pm = engine.state.planet_map.as_ref().unwrap();
        assert!(pm
            .edges
            .iter()
            .any(|e| e.from == colony_a && e.to == colony_b));

        // Trade route present in network.
        let route = engine
            .state
            .trade_network
            .routes
            .iter()
            .find(|r| r.id == route_id);
        assert!(route.is_some(), "trade route must be wired up");
        let route = route.unwrap();
        assert!(route.throughput_cap > 0.0);
    }

    #[test]
    fn build_infrastructure_throughput_matches_infra_type() {
        let (mut engine, colony_a, colony_b) = setup_two_colony_map();

        let events = engine
            .apply(&Command::BuildInfrastructure {
                from_colony: colony_a,
                to_colony: colony_b,
                infra_type: map::InfraType::Rail,
            })
            .unwrap();

        let route_id = events
            .iter()
            .find_map(|e| {
                if let Event::InfrastructureBuilt { route_id, .. } = e {
                    Some(*route_id)
                } else {
                    None
                }
            })
            .unwrap();
        let route = engine
            .state
            .trade_network
            .routes
            .iter()
            .find(|r| r.id == route_id)
            .unwrap();
        // Rail throughput should be the Rail base throughput.
        assert!(
            (route.throughput_cap - f64::from(map::InfraType::Rail.base_throughput())).abs() < 1.0,
            "throughput cap should match Rail base_throughput"
        );
    }

    #[test]
    fn demolish_infrastructure_removes_edge_and_route() {
        let (mut engine, colony_a, colony_b) = setup_two_colony_map();

        // Build first.
        let build_events = engine
            .apply(&Command::BuildInfrastructure {
                from_colony: colony_a,
                to_colony: colony_b,
                infra_type: map::InfraType::Pipeline,
            })
            .unwrap();
        let route_id = build_events
            .iter()
            .find_map(|e| {
                if let Event::InfrastructureBuilt { route_id, .. } = e {
                    Some(*route_id)
                } else {
                    None
                }
            })
            .unwrap();

        // Demolish.
        let demolish_events = engine
            .apply(&Command::DemolishInfrastructure {
                from_colony: colony_a,
                to_colony: colony_b,
            })
            .unwrap();

        // Event emitted with correct route_id.
        let demolished = demolish_events.iter().find_map(|e| {
            if let Event::InfrastructureDemolished { route_id, .. } = e {
                Some(*route_id)
            } else {
                None
            }
        });
        assert!(
            demolished.is_some(),
            "InfrastructureDemolished event must be emitted"
        );
        assert_eq!(demolished.unwrap(), route_id);

        // Edge removed from planet map.
        let pm = engine.state.planet_map.as_ref().unwrap();
        assert!(
            pm.edges.is_empty(),
            "edge should be removed from planet map"
        );

        // Trade route removed from network.
        assert!(
            engine
                .state
                .trade_network
                .routes
                .iter()
                .all(|r| r.id != route_id),
            "trade route should be removed from network"
        );
    }

    #[test]
    fn demolish_nonexistent_infrastructure_returns_error() {
        let (mut engine, colony_a, colony_b) = setup_two_colony_map();

        let result = engine.apply(&Command::DemolishInfrastructure {
            from_colony: colony_a,
            to_colony: colony_b,
        });
        assert!(result.is_err(), "demolish without prior build must fail");
    }

    #[test]
    fn build_infrastructure_requires_planet_map() {
        let mut engine = GameEngine::new();
        // Found two colonies without a planet map.
        let ev_a = engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: 50,
            })
            .unwrap();
        let colony_a = ev_a
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();
        let ev_b = engine
            .apply(&Command::FoundColony {
                name: "Beta".into(),
                starting_population: 50,
            })
            .unwrap();
        let colony_b = ev_b
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        let result = engine.apply(&Command::BuildInfrastructure {
            from_colony: colony_a,
            to_colony: colony_b,
            infra_type: map::InfraType::Road,
        });
        assert!(matches!(result, Err(EngineError::NoPlanetMap)));
    }

    // ── M1 #85: Command::System dispatch tests ────────────────────────────

    /// Build a minimal SystemState: two bodies, a route, and a hauler.
    fn setup_system(engine: &mut GameEngine) -> (system::BodyId, system::BodyId) {
        let events_a = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Inner".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 1.0,
            }))
            .unwrap();
        let body_a = events_a.iter().find_map(|e| {
            if let Event::System(system::SystemEvent::BodyAdded { body_id, .. }) = e {
                Some(body_id.clone())
            } else {
                None
            }
        });

        let events_b = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Outer".into(),
                kind: system::BodyKind::GasGiant,
                distance_au: 5.0,
            }))
            .unwrap();
        let body_b = events_b.iter().find_map(|e| {
            if let Event::System(system::SystemEvent::BodyAdded { body_id, .. }) = e {
                Some(body_id.clone())
            } else {
                None
            }
        });

        let body_a = body_a.unwrap();
        let body_b = body_b.unwrap();

        engine
            .apply(&Command::System(system::SystemCommand::AddShippingRoute {
                from: body_a.clone(),
                to: body_b.clone(),
            }))
            .unwrap();

        engine
            .apply(&Command::System(system::SystemCommand::AddHauler {
                capacity: 1000.0,
            }))
            .unwrap();

        (body_a, body_b)
    }

    #[test]
    fn system_command_add_body_emits_body_added_event() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "TestBody".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 1.5,
            }))
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::System(system::SystemEvent::BodyAdded { .. }))),
            "expected BodyAdded event"
        );
        assert!(!engine.state.system_state.node_map.bodies.is_empty());
    }

    #[test]
    fn system_command_add_shipping_route_emits_route_added_event() {
        let mut engine = GameEngine::new();
        let (body_a, body_b) = setup_system(&mut engine);
        // Route was added inside setup_system; verify it is stored.
        assert!(
            engine
                .state
                .system_state
                .node_map
                .routes
                .values()
                .any(|r| r.from == body_a && r.to == body_b || r.from == body_b && r.to == body_a),
            "expected shipping route between the two bodies"
        );
    }

    #[test]
    fn dispatch_cargo_arrives_after_n_months_and_deposits_in_colony_pool() {
        let mut engine = GameEngine::new();
        let (body_a, body_b) = setup_system(&mut engine);

        // Found a colony linked to body_b.
        let col_events = engine
            .apply(&Command::FoundColony {
                name: "Outer Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let colony_id = col_events
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        // Dispatch food cargo destined for the colony.
        let dispatch_events = engine
            .apply(&Command::System(system::SystemCommand::DispatchShipment {
                from: body_a,
                to: body_b,
                cargo: vec![("food".into(), 50.0)],
                destination_colony: Some(colony_id),
            }))
            .unwrap();
        assert!(
            dispatch_events.iter().any(|e| matches!(
                e,
                Event::System(system::SystemEvent::ShipmentDispatched { .. })
            )),
            "expected ShipmentDispatched event"
        );

        // The route travel time is 1 month (inner → outer at 5 AU defaults to
        // at least 1 month).  Advance enough strategic months for delivery.
        let food_before = engine
            .state
            .colonies
            .iter()
            .find(|c| c.id == colony_id)
            .unwrap()
            .pool
            .amount("food");

        // Advance colony-sols in 30-sol batches (each batch = one strategic month).
        // After each strategic month, check if food arrived in the colony pool.
        for _month in 0..20 {
            for _ in 0..30 {
                engine.apply(&Command::AdvanceColonySol).unwrap();
            }
            let food_now = engine
                .state
                .colonies
                .iter()
                .find(|c| c.id == colony_id)
                .unwrap()
                .pool
                .amount("food");
            if food_now > food_before {
                return; // cargo arrived and was deposited — test passes
            }
        }
        panic!("cargo never arrived in colony pool after 20 strategic months");
    }

    #[test]
    fn system_command_register_and_contribute_megaproject_via_apply() {
        let mut engine = GameEngine::new();

        // Register a generic megaproject.
        let reg_events = engine
            .apply(&Command::System(
                system::SystemCommand::RegisterMegaproject {
                    name: "Wormhole Gate".into(),
                    kind: system::MegaprojectKind::Custom("test".into()),
                    milestones: vec![system::MilestoneSpec {
                        label: "Phase 1".into(),
                        resource_cost: vec![],
                        research_cost: 10.0,
                    }],
                },
            ))
            .unwrap();

        let project_id = reg_events
            .iter()
            .find_map(|e| {
                if let Event::System(system::SystemEvent::MegaprojectRegistered {
                    project_id,
                    ..
                }) = e
                {
                    Some(project_id.clone())
                } else {
                    None
                }
            })
            .unwrap();

        // Contribute enough to complete the milestone.
        let contrib_events = engine
            .apply(&Command::System(
                system::SystemCommand::ContributeToMegaproject {
                    project_id,
                    resources: vec![],
                    research: 10.0,
                },
            ))
            .unwrap();

        assert!(
            contrib_events.iter().any(|e| matches!(
                e,
                Event::System(system::SystemEvent::MilestoneCompleted { .. })
            )),
            "expected MilestoneCompleted event after contributing full research"
        );
        assert!(
            contrib_events.iter().any(|e| matches!(
                e,
                Event::System(system::SystemEvent::MegaprojectCompleted { .. })
            )),
            "expected MegaprojectCompleted event"
        );
    }

    #[test]
    fn strategic_month_advances_cargo_shipments_automatically() {
        let mut engine = GameEngine::new();
        let (body_a, body_b) = setup_system(&mut engine);

        // Dispatch a shipment (no colony destination — just checking transit).
        engine
            .apply(&Command::System(system::SystemCommand::DispatchShipment {
                from: body_a,
                to: body_b,
                cargo: vec![("ore".into(), 100.0)],
                destination_colony: None,
            }))
            .unwrap();

        assert_eq!(
            engine.state.system_state.shipments.len(),
            1,
            "one shipment should be in transit"
        );

        // Advance colony-sols in 30-sol batches; each batch fires one strategic month.
        // Shipment should arrive within a few months.
        for _month in 0..20 {
            for _ in 0..30 {
                engine.apply(&Command::AdvanceColonySol).unwrap();
            }
            if engine.state.system_state.shipments.is_empty() {
                return; // arrived — test passes
            }
        }
        panic!("shipment never arrived after 20 strategic months");
    }

    // ── Issue #91: emigration gates and voluntary migration ──────────────────

    fn two_colony_engine() -> (GameEngine, ColonyId, ColonyId) {
        let mut engine = GameEngine::new();
        let evs_a = engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: 1000,
            })
            .unwrap();
        let evs_b = engine
            .apply(&Command::FoundColony {
                name: "Beta".into(),
                starting_population: 200,
            })
            .unwrap();
        let id_a = match &evs_a[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!("expected ColonyFounded"),
        };
        let id_b = match &evs_b[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!("expected ColonyFounded"),
        };
        (engine, id_a, id_b)
    }

    #[test]
    fn open_emigration_gate_stored_in_state() {
        let (mut engine, a, b) = two_colony_engine();
        let evs = engine
            .apply(&Command::OpenEmigrationGate {
                from_colony: a,
                to_colony: b,
                rate: 0.10,
            })
            .unwrap();
        assert!(
            matches!(evs[0], Event::EmigrationGateOpened { .. }),
            "should emit EmigrationGateOpened"
        );
        assert_eq!(
            engine.state.emigration_gates.len(),
            1,
            "gate should be stored in state"
        );
        assert_eq!(engine.state.emigration_gates[0].from_colony, a);
        assert_eq!(engine.state.emigration_gates[0].to_colony, b);
    }

    #[test]
    fn close_emigration_gate_removes_from_state() {
        let (mut engine, a, b) = two_colony_engine();
        engine
            .apply(&Command::OpenEmigrationGate {
                from_colony: a,
                to_colony: b,
                rate: 0.05,
            })
            .unwrap();
        assert_eq!(engine.state.emigration_gates.len(), 1);
        let evs = engine
            .apply(&Command::CloseEmigrationGate {
                from_colony: a,
                to_colony: b,
            })
            .unwrap();
        assert!(
            matches!(evs[0], Event::EmigrationGateClosed { .. }),
            "should emit EmigrationGateClosed"
        );
        assert!(
            engine.state.emigration_gates.is_empty(),
            "gate should be removed from state"
        );
    }

    #[test]
    fn open_gate_creates_batch_on_strategic_month() {
        let (mut engine, a, b) = two_colony_engine();
        engine
            .apply(&Command::OpenEmigrationGate {
                from_colony: a,
                to_colony: b,
                rate: 0.10,
            })
            .unwrap();

        let pop_before = engine
            .state
            .populations
            .iter()
            .find(|_| true)
            .map(|p| p.count)
            .unwrap_or(0.0);

        // Advance one strategic month (30 sols by default).
        let mut gate_queued = false;
        for _ in 0..30 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            if evs
                .iter()
                .any(|e| matches!(e, Event::GateMigrationQueued { .. }))
            {
                gate_queued = true;
            }
        }
        assert!(
            gate_queued,
            "GateMigrationQueued event should fire on strategic month"
        );
        // Source pop should have decreased (migrants in transit).
        let pop_after = engine.state.populations[0].count;
        assert!(
            pop_after < pop_before,
            "source population should decrease when gate batch departs; before={pop_before}, after={pop_after}"
        );
    }

    #[test]
    fn gate_batch_arrives_and_transfers_population() {
        let (mut engine, a, b) = two_colony_engine();
        let pop_b_before = engine.state.populations[1].count;

        engine
            .apply(&Command::OpenEmigrationGate {
                from_colony: a,
                to_colony: b,
                rate: 0.10,
            })
            .unwrap();

        // Advance two strategic months so batch from month 1 has time to arrive
        // (transit = 1 month → arrives on the next strategic month).
        let mut arrivals = 0usize;
        for _ in 0..60 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            arrivals += evs
                .iter()
                .filter(|e| matches!(e, Event::MigrationArrived { .. }))
                .count();
        }
        let pop_b_after = engine.state.populations[1].count;
        assert!(
            arrivals > 0,
            "at least one MigrationArrived event should have fired"
        );
        assert!(
            pop_b_after > pop_b_before,
            "destination population should grow after arrival; before={pop_b_before}, after={pop_b_after}"
        );
    }

    #[test]
    fn low_stability_triggers_voluntary_emigration() {
        let (mut engine, a, b) = two_colony_engine();

        // Set needs config with emigration_stability_floor = 0.5 to make it easy
        // to trigger.  Disable commodity-based needs so stability doesn't change
        // during the test (by using an empty needs list).
        engine.state.needs_config = Some(crate::needs::NeedsConfig {
            needs: vec![],
            stability_recovery_rate: 0.0,
            stability_decay_rate: 0.0,
            growth_stability_threshold: 0.70,
            growth_rate: 0.0,
            decline_stability_threshold: 0.0,
            decline_rate: 0.0,
            emigration_stability_floor: 0.5,
            voluntary_emigration_rate: 0.05,
        });

        // Force colony A stability below the floor.
        engine.state.populations[0].stability = 0.2;

        // Give colony B much higher attractiveness via housing and stability.
        engine.state.populations[1].stability = 0.9;
        // Add housing to colony B so it has headroom, boosting attractiveness score.
        let b_idx = engine
            .state
            .colonies
            .iter()
            .position(|c| c.id == b)
            .unwrap();
        engine.state.colonies[b_idx].pool.deposit("housing", 1000.0);

        let pop_a_before = engine.state.populations[0].count;

        // Advance one strategic month and collect events.
        let mut voluntary_triggered = false;
        for _ in 0..30 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            for e in &evs {
                if let Event::VoluntaryEmigrationTriggered { from_colony, .. } = e {
                    if *from_colony == a {
                        voluntary_triggered = true;
                    }
                }
            }
        }
        assert!(
            voluntary_triggered,
            "VoluntaryEmigrationTriggered should fire for colony A when stability < floor"
        );
        // Population should have decreased due to voluntary departure.
        let pop_a_after = engine.state.populations[0].count;
        assert!(
            pop_a_after < pop_a_before,
            "colony A population should decrease; before={pop_a_before}, after={pop_a_after}"
        );
    }

    // ── Issue #89: StabilityCritical, TechUnlocked, EventFired interrupt sources ──

    /// Done-when: StabilityCritical fires when stability drops to or below 20%.
    #[test]
    fn stability_critical_interrupt_fires_at_floor() {
        use crate::interrupt::{AdvanceResult, InterruptSource, Tier};

        let mut engine = GameEngine::with_seed(89);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Crisis Base".into(),
                starting_population: 100,
            })
            .unwrap();
        let colony_id = events
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        // Force stability to the critical floor.
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.populations[idx].stability = STABILITY_CRISIS_FLOOR;

        let result = engine.advance_until_interrupted(10, Tier::Urgent).unwrap();

        assert!(
            matches!(result, AdvanceResult::Halted { .. }),
            "expected Halted on StabilityCritical, got: {result:?}"
        );
        if let AdvanceResult::Halted { interrupt, .. } = result {
            assert!(
                matches!(
                    interrupt.source,
                    InterruptSource::StabilityCritical(cid) if cid == colony_id
                ),
                "expected StabilityCritical interrupt, got: {:?}",
                interrupt.source
            );
            assert_eq!(interrupt.colony_id, Some(colony_id));
        }
    }

    /// Done-when: StabilityCritical does NOT fire when stability is above the floor.
    #[test]
    fn stability_critical_does_not_fire_above_floor() {
        use crate::interrupt::{AdvanceResult, InterruptSource, Tier};

        let mut engine = GameEngine::with_seed(89);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Healthy Base".into(),
                starting_population: 100,
            })
            .unwrap();
        let colony_id = events
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        // Stability well above critical floor — no StabilityCritical interrupt.
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.populations[idx].stability = 0.80;

        let result = engine.advance_until_interrupted(3, Tier::Urgent).unwrap();

        if let AdvanceResult::Halted { interrupt, .. } = &result {
            assert!(
                !matches!(interrupt.source, InterruptSource::StabilityCritical(_)),
                "StabilityCritical must not fire when stability is above the floor"
            );
        }
        // Completed is also fine (no halting interrupt at all).
    }

    /// Done-when: EventFired interrupt fires when a HazardFired event is emitted.
    #[test]
    fn event_fired_interrupt_on_hazard_event() {
        use crate::interrupt::{AdvanceResult, InterruptSource, Tier};

        let mut engine = GameEngine::with_seed(89);
        engine
            .apply(&Command::FoundColony {
                name: "Hazard Base".into(),
                starting_population: 100,
            })
            .unwrap();

        use crate::menace::{FinalSemantics, MenaceDefinition, MenacePhase};

        let menace = MenaceDefinition {
            id: "dust_storm".into(),
            name: "Dust Storm".into(),
            phases: vec![MenacePhase {
                trigger_time: 0, // fires immediately on first tick
                telegraph: "Dust storm incoming.".into(),
                effects: vec![],
                hazard_injection: Some("dust_storm_phase1".into()),
            }],
            final_semantics: FinalSemantics::ProductionCollapse,
        };
        engine.state.menace_state = Some(crate::menace::MenaceState::new(menace));

        // Tick the menace — should emit MenacePhaseTriggered + HazardFired.
        let tick_events = engine.apply(&Command::TickMenace).unwrap();
        assert!(
            tick_events.iter().any(
                |e| matches!(e, Event::HazardFired { event_id } if event_id == "dust_storm_phase1")
            ),
            "TickMenace with hazard_injection must emit HazardFired"
        );

        // Verify collect_turn_interrupts directly:
        let interrupts = engine.collect_turn_interrupts(&tick_events);
        assert!(
            interrupts
                .iter()
                .any(|i| matches!(&i.source, InterruptSource::EventFired(id) if id == "dust_storm_phase1")),
            "collect_turn_interrupts must surface EventFired for HazardFired events; interrupts: {interrupts:?}"
        );
    }

    // ── Issue #93: Orbital station construction cost and build-turn tracking ──

    /// Helper: build a registry with a simple habitat blueprint costing 10 steel.
    fn make_registry_with_habitat_bp() -> content::ContentRegistry {
        use content::types::OrbitalStationBlueprint;
        let mut reg = content::ContentRegistry::default();
        reg.insert_orbital_blueprint(OrbitalStationBlueprint {
            id: "habitat_bp".into(),
            name: "Orbital Habitat".into(),
            station_type: orbital::StationType::Habitat,
            default_orbit: orbital::OrbitType::Low,
            commodity_costs: vec![("steel".into(), 10.0)],
            build_months: 2,
        });
        reg
    }

    /// Done-when: sufficient resources → costs deducted, project enqueued.
    #[test]
    fn begin_orbital_construction_deducts_costs_and_enqueues() {
        let mut engine = GameEngine::with_seed(93);
        engine.state.registry = Some(make_registry_with_habitat_bp());

        let events = engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: 100,
            })
            .unwrap();
        let colony_id = events
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        // Fund the colony with 20 steel.
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.colonies[idx].pool.deposit("steel", 20.0);

        let events = engine
            .apply(&Command::BeginOrbitalConstruction {
                blueprint_id: "habitat_bp".into(),
                colony_id,
                orbit_type: orbital::OrbitType::Low,
            })
            .unwrap();

        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::OrbitalConstructionStarted {
                    blueprint_id,
                    ..
                } if blueprint_id == "habitat_bp"
            )),
            "OrbitalConstructionStarted must be emitted"
        );

        // Costs must be deducted.
        let steel_left = engine.state.colonies[idx].pool.amount("steel");
        assert!(
            (steel_left - 10.0).abs() < 1e-6,
            "10 steel should be deducted; remaining={steel_left}"
        );

        // Project must be in queue.
        assert_eq!(engine.state.orbital_construction_queue.len(), 1);
        assert!(engine.state.orbital_construction_queue[0].costs_paid);
    }

    /// Done-when: insufficient resources → InsufficientResources error.
    #[test]
    fn begin_orbital_construction_fails_when_broke() {
        let mut engine = GameEngine::with_seed(93);
        engine.state.registry = Some(make_registry_with_habitat_bp());

        let events = engine
            .apply(&Command::FoundColony {
                name: "Broke Colony".into(),
                starting_population: 50,
            })
            .unwrap();
        let colony_id = events
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        // Colony has only 5 steel; needs 10.
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.colonies[idx].pool.deposit("steel", 5.0);

        let err = engine
            .apply(&Command::BeginOrbitalConstruction {
                blueprint_id: "habitat_bp".into(),
                colony_id,
                orbit_type: orbital::OrbitType::Low,
            })
            .unwrap_err();

        assert!(
            matches!(err, EngineError::InsufficientResources { .. }),
            "expected InsufficientResources, got {err:?}"
        );
    }

    /// Done-when: unknown blueprint → InvalidArgument error.
    #[test]
    fn begin_orbital_construction_fails_on_unknown_blueprint() {
        let mut engine = GameEngine::with_seed(93);
        engine.state.registry = Some(make_registry_with_habitat_bp());

        let events = engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: 50,
            })
            .unwrap();
        let colony_id = events
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        let err = engine
            .apply(&Command::BeginOrbitalConstruction {
                blueprint_id: "nonexistent_bp".into(),
                colony_id,
                orbit_type: orbital::OrbitType::Low,
            })
            .unwrap_err();

        assert!(
            matches!(err, EngineError::InvalidArgument(_)),
            "expected InvalidArgument for unknown blueprint"
        );
    }

    /// Done-when: after build_months strategic months the station is completed
    /// and OrbitalStationCompleted is emitted.
    #[test]
    fn orbital_construction_completes_after_configured_months() {
        use turn::TurnProcessor;
        let mut engine = GameEngine::with_seed(93);
        engine.state.registry = Some(make_registry_with_habitat_bp());

        let events = engine
            .apply(&Command::FoundColony {
                name: "Builder".into(),
                starting_population: 100,
            })
            .unwrap();
        let colony_id = events
            .iter()
            .find_map(|e| {
                if let Event::ColonyFounded { colony_id, .. } = e {
                    Some(*colony_id)
                } else {
                    None
                }
            })
            .unwrap();

        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.colonies[idx].pool.deposit("steel", 20.0);

        engine
            .apply(&Command::BeginOrbitalConstruction {
                blueprint_id: "habitat_bp".into(),
                colony_id,
                orbit_type: orbital::OrbitType::Low,
            })
            .unwrap();

        assert_eq!(engine.state.orbital_construction_queue.len(), 1);
        assert_eq!(
            engine.state.orbital_construction_queue[0].months_remaining,
            2
        );

        // Replace processor with 1-sol-per-month cadence for fast testing.
        engine.processor = TurnProcessor::with_cadence(0, 1);

        // Advance sol 1 → strategic month 1 → months_remaining decrements to 1.
        let ev1 = engine.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(
            engine.state.orbital_construction_queue.len(),
            1,
            "still in queue after month 1"
        );
        assert!(!ev1
            .iter()
            .any(|e| matches!(e, Event::OrbitalStationCompleted { .. })));

        // Advance sol 2 → strategic month 2 → months_remaining hits 0 → station placed.
        let ev2 = engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(
            engine.state.orbital_construction_queue.is_empty(),
            "queue should be empty after completion"
        );
        assert!(
            ev2.iter().any(|e| matches!(
                e,
                Event::OrbitalStationCompleted { blueprint_id, .. }
                if blueprint_id == "habitat_bp"
            )),
            "OrbitalStationCompleted must be emitted on the finishing month"
        );
        // Station must be in the registry.
        assert_eq!(engine.state.orbital_registry.stations.len(), 1);
    }

    // ── M3: Transport capacity for migration batches ──────────────────────────

    /// Helper: found two colonies with custom populations, return (engine, colony_a_id, colony_b_id).
    fn two_colony_engine_with_pop(pop_a: u64, pop_b: u64) -> (GameEngine, ColonyId, ColonyId) {
        let mut engine = GameEngine::with_seed(0);
        let ev_a = engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: pop_a,
            })
            .unwrap();
        let id_a = match &ev_a[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!("unexpected event"),
        };
        let ev_b = engine
            .apply(&Command::FoundColony {
                name: "Beta".into(),
                starting_population: pop_b,
            })
            .unwrap();
        let id_b = match &ev_b[0] {
            Event::ColonyFounded { colony_id, .. } => *colony_id,
            _ => panic!("unexpected event"),
        };
        (engine, id_a, id_b)
    }

    #[test]
    fn transport_capacity_default_is_nonzero() {
        let engine = GameEngine::new();
        let cap = engine.state.system_state.transport_capacity.total();
        assert!(cap > 0, "default transport capacity should be > 0");
    }

    #[test]
    fn add_hauler_increases_capacity() {
        let mut engine = GameEngine::new();
        let before = engine.state.system_state.transport_capacity.haulers;
        engine.apply(&Command::AddHauler { count: 3 }).unwrap();
        let after = engine.state.system_state.transport_capacity.haulers;
        assert_eq!(
            after,
            before + 3,
            "AddHauler should increase fleet by count"
        );
    }

    #[test]
    fn remove_hauler_decreases_capacity() {
        let mut engine = GameEngine::new();
        engine.apply(&Command::AddHauler { count: 5 }).unwrap();
        let before = engine.state.system_state.transport_capacity.haulers;
        engine.apply(&Command::RemoveHauler { count: 2 }).unwrap();
        let after = engine.state.system_state.transport_capacity.haulers;
        assert_eq!(
            after,
            before - 2,
            "RemoveHauler should decrease fleet by count"
        );
    }

    #[test]
    fn remove_hauler_clamps_at_zero() {
        let mut engine = GameEngine::new();
        // Remove more haulers than exist — should not underflow.
        let current = engine.state.system_state.transport_capacity.haulers;
        engine
            .apply(&Command::RemoveHauler {
                count: current + 100,
            })
            .unwrap();
        assert_eq!(
            engine.state.system_state.transport_capacity.haulers, 0,
            "RemoveHauler must clamp at zero"
        );
    }

    /// Acceptance criterion: capacity=10 with demand=25 → batch of 10, remainder queued.
    #[test]
    fn direct_migration_capped_by_transport_capacity() {
        let (mut engine, id_a, id_b) = two_colony_engine_with_pop(100, 50);

        // Set capacity = 1 hauler × 10 colonists = 10 total.
        engine.state.system_state.transport_capacity.haulers = 1;
        engine
            .state
            .system_state
            .transport_capacity
            .colonists_per_hauler = 10;

        // Try to move 25 colonists — only 10 fit.
        let events = engine
            .apply(&Command::DirectMigration {
                from_colony: id_a,
                to_colony: id_b,
                count: 25.0,
                transit_turns: 1,
            })
            .unwrap();

        // Should emit MigrationDeparted for 10 and MigrationQueued for 15.
        let departed = events
            .iter()
            .find_map(|e| {
                if let Event::MigrationDeparted { count, .. } = e {
                    Some(*count)
                } else {
                    None
                }
            })
            .expect("MigrationDeparted event expected");
        assert!(
            (departed - 10.0).abs() < 1e-3,
            "dispatched count should be 10, got {departed}"
        );

        let queued = events
            .iter()
            .find_map(|e| {
                if let Event::MigrationQueued { deferred_count, .. } = e {
                    Some(*deferred_count)
                } else {
                    None
                }
            })
            .expect("MigrationQueued event expected for overflow");
        assert!(
            (queued - 15.0).abs() < 1e-3,
            "deferred count should be 15, got {queued}"
        );

        // Exactly one pending migration should be in the queue (the dispatched batch).
        assert_eq!(
            engine.state.pending_migrations.len(),
            1,
            "only the dispatched batch enters pending_migrations"
        );
    }

    #[test]
    fn direct_migration_within_capacity_emits_no_queued_event() {
        let (mut engine, id_a, id_b) = two_colony_engine_with_pop(100, 50);
        // Capacity 100 — enough for 10 colonists.
        engine.state.system_state.transport_capacity.haulers = 10;
        engine
            .state
            .system_state
            .transport_capacity
            .colonists_per_hauler = 10;

        let events = engine
            .apply(&Command::DirectMigration {
                from_colony: id_a,
                to_colony: id_b,
                count: 10.0,
                transit_turns: 1,
            })
            .unwrap();

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, Event::MigrationQueued { .. })),
            "no MigrationQueued when demand fits within capacity"
        );
    }

    #[test]
    fn evacuate_colony_capped_by_transport_capacity() {
        let (mut engine, id_a, id_b) = two_colony_engine_with_pop(100, 10);

        // 1 hauler × 10 per hauler = capacity 10.
        engine.state.system_state.transport_capacity.haulers = 1;
        engine
            .state
            .system_state
            .transport_capacity
            .colonists_per_hauler = 10;

        // Evacuate 50 % of 100 = 50 colonists; only 10 fit.
        let events = engine
            .apply(&Command::EvacuateColony {
                from_colony: id_a,
                to_colony: id_b,
                fraction: 0.5,
                transit_turns: 1,
            })
            .unwrap();

        let queued = events
            .iter()
            .find_map(|e| {
                if let Event::MigrationQueued { deferred_count, .. } = e {
                    Some(*deferred_count)
                } else {
                    None
                }
            })
            .expect("MigrationQueued expected for overflow evacuation");
        assert!(
            queued > 0.0,
            "deferred count should be > 0 for capped evacuation, got {queued}"
        );
    }
}
