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
pub mod outpost;
pub mod population;
pub mod predicate;
pub mod research;
pub mod snapshot;
pub mod system;
pub mod system_gen;
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
use needs::{apply_needs_check_scaled, apply_population_dynamics};
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

/// Cargo-capacity stand-in for [`Command::FoundColonyAtSite`]'s
/// `supply_overrides` (issue #167, open design question: "bounded by a
/// cargo capacity?"). Each per-commodity override is capped at this many
/// times the largest per-100-colonist amount authored for that commodity
/// across all [`content::types::SupplyPackage`]s, scaled by
/// `starting_population / 100.0`.
pub const MAX_SUPPLY_OVERRIDE_MULTIPLE: f64 = 3.0;

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
    /// Rejects if the colony lacks sufficient build slots, or with
    /// [`EngineError::TechLocked`] if the registry defines this building
    /// with a `tech_prerequisite` that hasn't been researched yet (issue
    /// #247). The tech gate is inert when no registry is loaded.
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
    /// Place a batch of buildings directly into a colony's `buildings` list,
    /// bypassing `build_queue`/`construction_turns` entirely — a "lander"
    /// mechanic for the founding moment, mirroring how
    /// [`Command::BuildOrbitalStation`] already bypasses its own queued
    /// sibling (`BeginOrbitalConstruction`).
    ///
    /// One-shot per colony (rejects once [`colony::Colony::starter_kit_deployed`]
    /// is set) so this can't become a standing free-and-instant alternative
    /// to `QueueConstruction` later in the game. Validates the whole batch
    /// (non-empty, tech gates, total slot cost) before placing anything, so
    /// a rejected request never leaves a partially-deployed kit and never
    /// consumes the one-shot flag. Still subject to the same
    /// [`EngineError::TechLocked`]/[`EngineError::SlotCapacityExceeded`]
    /// gates `QueueConstruction` enforces.
    ///
    /// Unlike `QueueConstruction`, this skips `labor_per_turn` and
    /// `construction_cost` entirely — starter buildings are meant to arrive
    /// fully paid-for as part of the founding moment, not drip-fed labor
    /// over several sols the colony doesn't have workers for yet.
    DeployStarterKit {
        /// Target colony — must not have deployed a starter kit already.
        colony_id: ColonyId,
        /// `(building_type, slot_cost)` pairs to place, in order. Must not
        /// be empty.
        buildings: Vec<(String, u32)>,
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
    /// Select which recipe a building type runs in this colony (issue #166).
    ///
    /// Applies to every instance of `building_type` in the colony — recipe
    /// selection is colony-wide per type, not per placed instance. Errors if
    /// `recipe_id` doesn't name a recipe belonging to `building_type` in the
    /// loaded content registry.
    SetActiveRecipe {
        /// Target colony.
        colony_id: ColonyId,
        /// Building type to set the active recipe for.
        building_type: String,
        /// Recipe id to activate; must have `recipe.building == building_type`.
        recipe_id: String,
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
        /// Optional `SupplyPackage` id (from the content pack) used to seed
        /// the new colony's commodity pool. Amounts in the package are per-100-
        /// colonist and scale linearly with `starting_population`.
        #[serde(default)]
        supplies_id: Option<String>,
        /// Explicit per-commodity starter-supply amounts (issue #167).
        ///
        /// When present these **replace** the `supplies_id` package scaling
        /// entirely — the wizard pre-fills its per-commodity spinners from a
        /// package's defaults (scaled by `starting_population`), the player
        /// tweaks them, and the final absolute amounts are sent here. Each
        /// commodity id must appear in at least one authored
        /// [`content::types::SupplyPackage`] and each amount is capped at
        /// [`MAX_SUPPLY_OVERRIDE_MULTIPLE`] times the largest per-100-colonist
        /// amount authored for that commodity across all packages (scaled by
        /// `starting_population / 100.0`) — a stand-in "cargo capacity" bound
        /// per issue #167's open design question. `supplies_id` may still be
        /// sent alongside this for display/analytics but is not consulted
        /// for seeding when overrides are present.
        #[serde(default)]
        supply_overrides: Option<Vec<(String, f64)>>,
        /// Star-system body this site belongs to, if known (issue #183).
        ///
        /// When present, founding is gated on
        /// [`system::Body::meets_founding_threshold`] (unless the player has
        /// unlocked [`system::HARSH_WORLD_CAPABILITY_ID`]), and on success the
        /// colony is auto-linked to the body exactly as
        /// [`Command::AssignColonyHomeBody`] would — that command remains
        /// separately callable (e.g. to re-link) but is no longer required
        /// for the gate to apply. `None` skips the gate entirely, preserving
        /// existing callers that don't have body context.
        #[serde(default)]
        body_id: Option<system::BodyId>,
    },
    /// Link a colony to its home system body and inherit the body's
    /// habitability-derived productivity modifier (issue #163).
    ///
    /// Called by the founding wizard right after [`Command::FoundColony`] or
    /// [`Command::FoundColonyAtSite`] to record which star-system body the
    /// colony sits on. The core looks up the body in `system_state.node_map`,
    /// stashes `body_id` on the colony, and copies
    /// [`system::Body::habitability_modifier`] into
    /// [`colony::Colony::habitability_modifier`].
    AssignColonyHomeBody {
        /// Colony to update.
        colony_id: ColonyId,
        /// Body the colony calls home.
        body_id: system::BodyId,
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
        /// System body the finished station will orbit (issue #234). Should
        /// be `Some` for `Low`/`Geostationary` (slot capacity is tracked per
        /// body for those bands) and is conventionally `None` for `Lagrange`
        /// (a system-wide asset).
        #[serde(default)]
        body_id: Option<system::BodyId>,
    },
    /// Build an orbital station in the given orbit band, linked to a colony.
    ///
    /// Builds immediately (no construction-turn cost) — unlike
    /// [`Command::BeginOrbitalConstruction`], this path is not queued.
    /// Fails with [`EngineError::OrbitalSlotExceeded`] if the orbit band is
    /// full, or with [`EngineError::OrbitalError`] wrapping
    /// [`orbital::OrbitalError::InvalidSlotCost`] if `slot_cost` falls
    /// outside `station_type`'s valid range (issue #234).
    BuildOrbitalStation {
        /// Colony that funds and operates the station.
        colony_id: ColonyId,
        /// Station specialization type.
        station_type: orbital::StationType,
        /// Target orbit band.
        orbit_type: orbital::OrbitType,
        /// System body the station orbits (issue #234); see
        /// [`Command::BeginOrbitalConstruction`]'s doc comment for the
        /// `None`-means-Lagrange convention.
        #[serde(default)]
        body_id: Option<system::BodyId>,
        /// Chosen station size, within [`orbital::StationType::slot_range`]
        /// (issue #234). Defaults to [`orbital::StationType::slot_cost`]
        /// (the pre-#234 fixed size) when omitted.
        #[serde(default)]
        slot_cost: Option<u32>,
    },
    /// Demolish (decommission) an orbital station by its stable id.
    DecommissionOrbitalStation {
        /// Stable identifier of the station to remove.
        station_id: uuid::Uuid,
    },
    /// Deploy a satellite constellation in the given orbit band.
    ///
    /// A **probe** (issue #234) is simply a small constellation (typically
    /// `count: 1`) with `body_id` set to a body other than the founding
    /// colony's home body — there is no separate probe command.
    DeployConstellation {
        /// Satellite type (coverage layer).
        satellite_type: orbital::SatelliteType,
        /// Orbit band for the constellation.
        orbit_type: orbital::OrbitType,
        /// Number of satellites to deploy.
        count: u32,
        /// System body this constellation covers (issue #234); see
        /// [`Command::BeginOrbitalConstruction`]'s doc comment for the
        /// `None`-means-Lagrange convention.
        #[serde(default)]
        body_id: Option<system::BodyId>,
    },
    /// Toggle the map-overlay visibility of a satellite constellation.
    ToggleConstellationOverlay {
        /// Stable identifier of the constellation.
        constellation_id: uuid::Uuid,
    },

    // ── Phase 10: Difficulty / Menace / Victory ───────────────────────────
    /// Set the active difficulty preset, rebuilding the difficulty scalar from the grade table.
    ///
    /// Since #161, callable at any sol so mid-game difficulty changes are supported.
    SetDifficulty {
        /// Preset to activate.
        preset: difficulty::DifficultyPreset,
    },
    /// Install a user-authored [`modifier::DifficultyScalar`] atomically alongside
    /// the menace / hazards / maintenance toggles (issue #161 custom-difficulty
    /// menu; maintenance added in #180).
    ///
    /// The preset is set to [`difficulty::DifficultyPreset::Custom`]. When
    /// `menace_enabled` is `false` any active menace is cleared; when it flips
    /// back on the engine re-attaches the last-known menace definition (if any).
    SetCustomDifficulty {
        /// Per-quantity scalar map assembled from the panel sliders.
        scalars: modifier::DifficultyScalar,
        /// Whether the existential-menace clock should be active.
        menace_enabled: bool,
        /// Whether environmental hazards should fire.
        hazards_enabled: bool,
        /// Whether per-building maintenance draws should apply (issue #180).
        maintenance_enabled: bool,
    },
    /// Toggle the master hazard switch (issue #161).
    ///
    /// A cleaner in-game toggle than clamping `HazardProbability` to zero.
    SetHazardsEnabled {
        /// New master hazards-enabled state.
        enabled: bool,
    },
    /// Toggle the master building-maintenance switch (issue #180).
    ///
    /// When `false`, per-building `maintenance` draws are short-circuited
    /// regardless of the `MaintenanceConsumption` scalar.
    SetMaintenanceEnabled {
        /// New master maintenance-enabled state.
        enabled: bool,
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
    /// Apply player mitigation to reduce the menace pressure level.
    ///
    /// Deducts `resource_cost` units of `resource_id` from the specified colony pool,
    /// then reduces `menace.level` by `amount`.  Fails if the colony pool cannot cover
    /// the resource cost.
    MitigateMenace {
        /// Colony whose resource pool pays the mitigation cost.
        colony_id: ColonyId,
        /// Commodity to spend (e.g. `"energy"`, `"materials"`).
        resource_id: String,
        /// Amount of `resource_id` consumed.
        resource_cost: f64,
        /// How much menace level to remove.
        amount: f32,
    },
    /// Record that the interstellar expedition megaproject has been launched.
    ///
    /// This is the primary victory trigger. The engine evaluates all victory conditions
    /// and emits [`Event::VictoryAchieved`] for each newly satisfied condition.
    LaunchExpedition,

    // ── M8: Field Expeditions (issue #103) ────────────────────────────────
    /// Launch a field expedition from a colony to explore a hex tile.
    ///
    /// Creates an [`expedition::Expedition`] in `InTransit` status and emits
    /// [`Event::ExpeditionLaunched`].  The expedition advances each colony-sol.
    LaunchFieldExpedition {
        /// Colony launching the expedition.
        colony_id: ColonyId,
        /// Hex tile target for exploration.
        target_hex: map::HexCoord,
        /// Number of crew assigned to the mission.
        crew_count: u32,
        /// Supplies loaded for the mission.
        supplies: f32,
        /// Sols required to travel from the colony to the target hex.
        transit_sols: u64,
        /// Whether this is a deep-space expedition (contributes to interstellar megaproject).
        is_deep_space: bool,
    },

    /// Recall an active field expedition back to its origin colony.
    ///
    /// Sets the expedition status to `Returning` if it is currently `OnSite`.
    /// Emits no event if the expedition is already returning or terminal.
    RecallExpedition {
        /// Stable identifier of the expedition to recall.
        expedition_id: expedition::FieldExpeditionId,
    },

    // ── Body-scouting survey expeditions (issue #235) ───────────────────────
    /// Launch a probe or manned survey expedition at a system body.
    ///
    /// Unlike [`Command::LaunchFieldExpedition`] (which targets a planet-map
    /// hex), this targets any [`system::BodyId`] and uses the richer
    /// [`expedition::ExpeditionType`] tiers (probe through manned) with
    /// probabilistic full/partial/failed survey outcomes. See
    /// [`expedition::resolve_survey`].
    ///
    /// Fails with [`EngineError::ColonyNotFound`] if `colony_id` is unknown,
    /// or [`EngineError::InvalidArgument`] if `target_body` is not a known
    /// system body.
    LaunchSurveyExpedition {
        /// Colony that launches and funds the expedition.
        colony_id: ColonyId,
        /// Target body to survey.
        target_body: system::BodyId,
        /// Mission profile (probe through manned).
        expedition_type: expedition::ExpeditionType,
    },

    /// Resolve a pending mid-mission decision (typically an anomaly
    /// encounter) on a survey expedition currently in
    /// [`expedition::ExpeditionPhase::AwaitingDecision`].
    ///
    /// Fails with [`EngineError::InvalidArgument`] if the expedition is
    /// unknown, not awaiting a decision, or `choice_id` does not match one
    /// of the pending event's choices.
    ResolveMissionDecision {
        /// Expedition with the pending decision.
        expedition_id: expedition::ExpeditionId,
        /// Id of the chosen [`expedition::MissionChoice`].
        choice_id: String,
    },

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

    // ── M7: Per-colony interrupt configuration ────────────────────────────
    /// Set the interrupt sensitivity configuration for a colony.
    ///
    /// Replaces any existing config for `colony_id`.  Use
    /// [`interrupt::InterruptConfig::silent`] to suppress all interrupts from
    /// a colony, or [`interrupt::InterruptConfig::all_enabled`] to restore the
    /// default.
    SetColonyInterruptConfig {
        /// Target colony.
        colony_id: ColonyId,
        /// New interrupt sensitivity config for this colony.
        config: interrupt::InterruptConfig,
    },

    // ── Debug/testing (issue #232) ─────────────────────────────────────────
    /// Grant an arbitrary quantity of a commodity directly into a colony's
    /// pool, bypassing production/deposits/recipes entirely.
    ///
    /// A testing-mode escape hatch, not a normal gameplay command — the
    /// intended workflow (per issue #232) is a playtester using this to
    /// simulate "what if this colony already had enough of X" while probing
    /// whether the current resource-distribution algorithm gives a founding
    /// site a genuine path to a tech/victory goal, so the algorithm can be
    /// tuned from what the playthrough actually needed. Hosts should gate
    /// this behind an explicit testing/sandbox mode rather than exposing it
    /// in normal play. No content-pack or deposit validation is performed —
    /// `commodity_id` need not correspond to a real [`content::CommodityDef`]
    /// or an existing deposit.
    DebugGrantColonyResources {
        /// Target colony.
        colony_id: ColonyId,
        /// Content-pack commodity id to grant.
        commodity_id: String,
        /// Quantity to add to the colony's pool (capped by pool capacity,
        /// same as any other deposit).
        amount: f64,
    },

    // ── Outposts (issue #233) ───────────────────────────────────────────────
    /// Establish a new [`outpost::Outpost`] anchored to `body_id`, owned by
    /// `colony_id`.
    ///
    /// Outposts extend a colony's reach rather than existing independently.
    /// Rejected with [`EngineError::OutpostOutOfRange`] if `body_id` is
    /// farther than [`outpost::max_outpost_range_au`] from the parent
    /// colony's `home_body_id` (issue #241) — inert for a colony with no
    /// `home_body_id` (never spatially placed). What can actually be *built*
    /// at an established outpost is gated separately, by tech prerequisite,
    /// at [`Command::QueueOutpostConstruction`].
    EstablishOutpost {
        /// Human-readable name.
        name: String,
        /// The colony establishing (and owning) this outpost.
        colony_id: ColonyId,
        /// The system body to anchor the outpost to.
        body_id: system::BodyId,
    },
    /// Tear down an outpost, removing it from play.
    ///
    /// No refund of invested resources — matches [`Command::CancelConstruction`]'s
    /// "in-progress project" refund model existing only for queued (not
    /// completed) work; a completed outpost's buildings/stockpile are simply
    /// discarded. Promotion to a full colony (issue #242) is the alternative
    /// to decommissioning when the outpost's investment should be kept.
    DecommissionOutpost {
        /// Outpost to remove.
        outpost_id: outpost::OutpostId,
    },
    /// Convert an established outpost into a full, independent
    /// [`colony::Colony`] (issue #242).
    ///
    /// Promotion is unconditional — any outpost can be promoted at any time,
    /// matching #233's "establishment is never gated" precedent for the base
    /// mechanism (no minimum stockpile/building/tech is required). The new
    /// colony carries over the outpost's pool, buildings, construction
    /// queue, slot capacity, category modifiers, active recipes, and last
    /// production outcome unchanged; `body_id` becomes the new colony's
    /// `home_body_id`. The outpost is removed and its `parent_colony_id`
    /// link is dropped — the resulting colony is fully independent, with no
    /// retained relationship to the outpost's former parent (there is no
    /// "parent colony" concept for a `Colony` to carry it in). A fresh
    /// [`crate::population::PopulationPool`] is spun up from
    /// `starting_population`, since an outpost has no population of its own
    /// to carry over.
    PromoteOutpostToColony {
        /// Outpost to promote.
        outpost_id: outpost::OutpostId,
        /// Name for the resulting colony.
        name: String,
        /// Starting population for the new colony.
        starting_population: u64,
    },
    /// Queue a construction project at an outpost, reusing the same
    /// [`colony::ConstructionProject`]/[`colony::ConstructionQueue`]
    /// machinery colonies use.
    ///
    /// Unlike colony construction, no `labor` commodity is withdrawn per
    /// turn — outposts have no population to generate a labor supply from;
    /// `labor_per_turn` is retained for API symmetry with
    /// [`Command::QueueConstruction`] and future balancing but is not
    /// currently consumed.
    QueueOutpostConstruction {
        /// Target outpost.
        outpost_id: outpost::OutpostId,
        /// Content-pack building type id.
        building_type: String,
        /// Build-slot cost.
        slot_cost: u32,
        /// Reserved for future use — see doc comment above.
        labor_per_turn: u32,
        /// Material cost, `(commodity_id, quantity)` pairs.
        construction_cost: Vec<(String, f64)>,
        /// Number of sols to complete.
        construction_turns: u32,
    },
    /// Select which authored recipe an outpost's building type runs, for
    /// building types with more than one recipe (mirrors
    /// [`Command::SetActiveRecipe`]).
    SetOutpostActiveRecipe {
        /// Target outpost.
        outpost_id: outpost::OutpostId,
        /// Building type to select a recipe for.
        building_type: String,
        /// Recipe id to activate.
        recipe_id: String,
    },
    /// Withdraw resources/research from an outpost's pool and contribute
    /// them to a system megaproject.
    ///
    /// [`system::SystemCommand::ContributeToMegaproject`] already accepts
    /// raw `resources`/`research` with no colony/source reference — this
    /// command is a thin withdraw-then-forward wrapper so an outpost's
    /// production can fund a megaproject without a full colony backing it
    /// (the megaproject-support use case from issue #233).
    ContributeOutpostToMegaproject {
        /// Source outpost.
        outpost_id: outpost::OutpostId,
        /// Target megaproject.
        project_id: system::MegaprojectId,
        /// Resources to withdraw and contribute, `(commodity_id, quantity)`
        /// pairs. Withdrawal is capped at what the outpost's pool actually
        /// holds — under-supply contributes whatever is available, it does
        /// not error.
        resources: Vec<(String, f64)>,
        /// Research to contribute — currently always `0.0` in practice since
        /// outposts don't produce `research` without a research-capable
        /// building authored for one, but accepted for symmetry with
        /// `ContributeToMegaproject`.
        research: f32,
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
    /// Return full detail for one building type within a colony (issue #182).
    BuildingDetail {
        /// Target colony.
        colony_id: ColonyId,
        /// Content-pack key of the building type.
        building_type: String,
    },
    /// Return full detail for one building type within an outpost (navigation
    /// rework #7 phase 4 — outpost drill-down parity with colonies). Same
    /// response shape as [`Query::BuildingDetail`]; only the owner differs.
    OutpostBuildingDetail {
        /// Target outpost.
        outpost_id: outpost::OutpostId,
        /// Content-pack key of the building type.
        building_type: String,
    },
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
    /// Full detail for one building type within a colony.
    BuildingDetail(ui::BuildingDetailData),
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
    /// Stability scalar in `[0.0, 1.0]`.
    pub stability: f32,
    /// Labour units available this turn.
    pub available_labour: f32,
    /// Commodity pool snapshot: `(commodity_id, amount)` pairs.
    pub commodity_pool: Vec<(String, f32)>,
    /// Placed building type identifiers.
    pub buildings: Vec<String>,
    /// Active construction project identifiers (`building_type`).
    pub active_construction: Vec<String>,
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
    /// A building type's active recipe was set (issue #166).
    ActiveRecipeSet {
        /// Target colony.
        colony_id: ColonyId,
        /// Building type the recipe applies to.
        building_type: String,
        /// The newly-active recipe id.
        recipe_id: String,
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
    /// A colony's home body was recorded (issue #163).
    ColonyHomeBodySet {
        /// Colony that was updated.
        colony_id: ColonyId,
        /// Body the colony was linked to.
        body_id: system::BodyId,
        /// Productivity multiplier copied from the body's habitability.
        habitability_modifier: f32,
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
        /// System body the finished station will orbit (issue #234); `None`
        /// for a Lagrange-band, system-wide station.
        body_id: Option<system::BodyId>,
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
        /// System body the station orbits (issue #234); `None` for Lagrange.
        body_id: Option<system::BodyId>,
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
        /// System body the station orbits (issue #234); `None` for Lagrange.
        body_id: Option<system::BodyId>,
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
        /// System body this constellation covers (issue #234); `None` for a
        /// Lagrange-band, system-wide constellation. A **probe** is simply a
        /// small constellation with `body_id` set to a non-home body.
        body_id: Option<system::BodyId>,
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
    /// The menace level crossed its critical threshold; a countdown to game-over has begun.
    MenaceCritical {
        /// Category of the menace that went critical.
        kind: menace::MenaceKind,
        /// Current level at the moment it went critical.
        level: f32,
        /// Strategic months remaining before game-over if left unmitigated.
        countdown_months: u32,
    },
    /// The menace countdown reached zero; the colony network collapses.
    GameOver {
        /// The menace kind that caused the collapse.
        reason: menace::MenaceKind,
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

    // ── M8: Field expedition events (issue #103) ──────────────────────────
    /// A field expedition was launched from a colony.
    ExpeditionLaunched {
        /// Identifier of the new expedition.
        expedition_id: expedition::FieldExpeditionId,
        /// Colony that launched the expedition.
        colony_id: ColonyId,
        /// Target hex tile.
        target_hex: map::HexCoord,
    },

    /// A field expedition arrived at its target hex and began on-site work.
    ExpeditionArrived {
        /// Expedition that arrived.
        expedition_id: expedition::FieldExpeditionId,
    },

    /// A field expedition discovered a resource deposit on-site.
    ExpeditionDiscovery {
        /// Expedition making the discovery.
        expedition_id: expedition::FieldExpeditionId,
        /// Hex where the discovery was made.
        hex: map::HexCoord,
        /// Discovered commodity identifier.
        resource_id: String,
        /// Quantity discovered.
        amount: f64,
    },

    /// A field expedition completed its return and deposited resources.
    ExpeditionReturned {
        /// Expedition that returned.
        expedition_id: expedition::FieldExpeditionId,
        /// Colony that received the deposits.
        colony_id: ColonyId,
        /// Resources deposited: `(commodity_id, amount)`.
        deposits: Vec<(String, f64)>,
    },

    /// A field expedition was lost due to supply depletion.
    ExpeditionLost {
        /// Expedition that was lost.
        expedition_id: expedition::FieldExpeditionId,
    },

    // ── Body-scouting survey expeditions (issue #235) ───────────────────────
    /// A survey expedition (probe or manned) was launched at a system body.
    SurveyExpeditionLaunched {
        /// Identifier of the new expedition.
        expedition_id: expedition::ExpeditionId,
        /// Colony that launched and is funding the expedition.
        colony_id: ColonyId,
        /// Target body being surveyed.
        target_body: system::BodyId,
        /// Mission profile.
        expedition_type: expedition::ExpeditionType,
    },

    /// A survey expedition finished its transit leg and began surveying.
    SurveyExpeditionArrived {
        /// Expedition that arrived on-station.
        expedition_id: expedition::ExpeditionId,
        /// Body being surveyed.
        target_body: system::BodyId,
    },

    /// A mid-mission event (typically an anomaly encounter) was injected into
    /// an active survey expedition, halting it in
    /// [`expedition::ExpeditionPhase::AwaitingDecision`] until resolved via
    /// [`Command::ResolveMissionDecision`].
    MidMissionEventTriggered {
        /// Expedition the event was injected into.
        expedition_id: expedition::ExpeditionId,
        /// The event itself (title, description, tier, choices).
        event: expedition::MidMissionEvent,
    },

    /// A player decision on a pending mid-mission event was resolved.
    MissionDecisionResolved {
        /// Expedition the decision applied to.
        expedition_id: expedition::ExpeditionId,
        /// Id of the choice that was selected.
        choice_id: String,
    },

    /// An anomaly investigation granted its reward (research/resources/tech).
    AnomalyOutcomeResolved {
        /// Expedition that investigated the anomaly.
        expedition_id: expedition::ExpeditionId,
        /// The resolved outcome and its rewards.
        outcome: expedition::AnomalyOutcome,
    },

    /// A survey expedition completed (successfully or not) and its outcome
    /// was recorded.
    SurveyCompleted {
        /// Expedition that completed.
        expedition_id: expedition::ExpeditionId,
        /// Colony that funded the expedition.
        colony_id: ColonyId,
        /// Body that was surveyed.
        target_body: system::BodyId,
        /// The resolved survey outcome.
        outcome: expedition::SurveyOutcome,
    },

    /// A [`Command::DebugGrantColonyResources`] testing-mode grant landed in
    /// a colony's pool.
    DebugResourcesGranted {
        /// Colony that received the grant.
        colony_id: ColonyId,
        /// Commodity granted.
        commodity_id: String,
        /// Quantity actually added (may be less than requested if pool
        /// capacity capped it — see [`colony::pool::ColonyPool::deposit`]).
        amount: f64,
    },

    // ── Outposts (issue #233) ───────────────────────────────────────────────
    /// A new outpost was established.
    OutpostEstablished {
        /// The new outpost's id.
        outpost_id: outpost::OutpostId,
        /// The colony that established it.
        colony_id: ColonyId,
        /// The body it's anchored to.
        body_id: system::BodyId,
    },
    /// An outpost was decommissioned and removed from play.
    OutpostDecommissioned {
        /// The removed outpost's id.
        outpost_id: outpost::OutpostId,
    },
    /// An outpost was promoted into a full, independent colony (issue #242).
    OutpostPromoted {
        /// The promoted outpost's former id (now removed from play).
        outpost_id: outpost::OutpostId,
        /// The newly created colony's id.
        colony_id: ColonyId,
        /// The new colony's name.
        name: String,
    },
    /// An outpost queued a construction project.
    OutpostConstructionQueued {
        /// Target outpost.
        outpost_id: outpost::OutpostId,
        /// Building type queued.
        building_type: String,
        /// Queued project's id.
        project_id: colony::ProjectId,
    },
    /// An outpost's queued construction project completed.
    OutpostBuildingConstructed {
        /// Target outpost.
        outpost_id: outpost::OutpostId,
        /// Building type completed.
        building_type: String,
    },
    /// An outpost's per-building production shortfall (mirrors
    /// [`Event::ProductionShortfall`]).
    OutpostProductionShortfall {
        /// Target outpost.
        outpost_id: outpost::OutpostId,
        /// Building type that fell short.
        building_type: String,
        /// Scale factor actually achieved (`1.0` = full output), in `[0.0, 1.0]`.
        scale: f64,
        /// Why production fell short.
        reason: colony::ShortfallReason,
    },
    /// An outpost's active recipe selection changed.
    OutpostActiveRecipeSet {
        /// Target outpost.
        outpost_id: outpost::OutpostId,
        /// Building type the selection applies to.
        building_type: String,
        /// Newly active recipe id.
        recipe_id: String,
    },
    /// An outpost contributed resources/research to a megaproject.
    OutpostContributedToMegaproject {
        /// Source outpost.
        outpost_id: outpost::OutpostId,
        /// Target megaproject.
        project_id: system::MegaprojectId,
        /// Resources actually withdrawn and contributed (may be less than
        /// requested if the outpost's pool didn't hold enough).
        resources: Vec<(String, f64)>,
        /// Research contributed.
        research: f32,
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
    /// The referenced outpost does not exist.
    #[error("outpost not found: {0}")]
    OutpostNotFound(outpost::OutpostId),
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
    /// The target body's habitability score is below the founding threshold
    /// and the player hasn't unlocked the harsh-world capability (issue #183).
    #[error(
        "body habitability {score} is below the founding threshold of {threshold} \
         (research a tech that unlocks '{}' to override)",
        system::HARSH_WORLD_CAPABILITY_ID
    )]
    HabitabilityBelowThreshold {
        /// The body's actual habitability score.
        score: u8,
        /// The minimum score required without the capability override.
        threshold: u8,
    },
    /// The target body is farther from the parent colony's home body than
    /// the current max outpost range allows (issue #241).
    #[error(
        "body is {distance_au:.2} AU from the parent colony's home body, exceeding the max \
         outpost range of {max_range_au:.2} AU (research a propulsion or outpost-range tech \
         to extend it)"
    )]
    OutpostOutOfRange {
        /// Actual distance between the parent colony's home body and the target body, in AU.
        distance_au: f32,
        /// The current max allowed range, in AU.
        max_range_au: f32,
    },
    /// The requested building requires a tech prerequisite that hasn't been
    /// researched yet. Enforced on [`Command::QueueConstruction`] (issue
    /// #247) and [`Command::QueueOutpostConstruction`] (issue #241).
    #[error("building '{building_id}' requires tech prerequisite '{tech_id}' to be researched")]
    TechLocked {
        /// The building that was requested.
        building_id: String,
        /// The unresearched tech id it requires.
        tech_id: String,
    },
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
    ///
    /// # Panics
    ///
    /// Panics only on an internal invariant violation: if a colony index
    /// lookup immediately following a successful colony insertion fails to
    /// find that same colony, which should be impossible.
    #[allow(clippy::too_many_lines)]
    pub fn apply(&mut self, cmd: &Command) -> Result<Vec<Event>, EngineError> {
        // Block all commands once victory is recorded, unless sandbox-continue is active.
        if self.state.victory.is_some()
            && !self.state.victory_state.sandbox_continue
            && !matches!(
                cmd,
                Command::ContinueAfterVictory | Command::ContinueSandbox
            )
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
                            let station = OrbitalStation::new_default_size(
                                project.station_type,
                                project.orbit_type,
                                project.colony_id,
                                project.body_id.clone(),
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
                                body_id: project.body_id.clone(),
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

                // ── Step 1b: Outpost construction (issue #233) ──────────────
                // No `labor` withdrawal — outposts have no population to fund
                // one from; see `outpost::Outpost`'s module doc comment.
                for out in &mut self.state.outposts {
                    if let Some(completed) = out.build_queue.tick_active() {
                        let building_type = completed.building_type.clone();
                        out.buildings.push(colony::PlacedBuilding::new(
                            &building_type,
                            completed.slot_cost,
                        ));
                        events.push(Event::OutpostBuildingConstructed {
                            outpost_id: out.id,
                            building_type,
                        });
                    }
                }

                // ── Step 2: Needs resolution ────────────────────────────────
                // Consume bulk commodities, update stability and population.
                if let Some(config) = self.state.needs_config.clone() {
                    let stability_scalar = self
                        .state
                        .difficulty_scalar
                        .scalar_for(&modifier::ModifiableQuantity::StabilityRate);
                    let growth_scalar = self
                        .state
                        .difficulty_scalar
                        .scalar_for(&modifier::ModifiableQuantity::PopulationGrowth);
                    let consumption_scalar = self
                        .state
                        .difficulty_scalar
                        .scalar_for(&modifier::ModifiableQuantity::ResourceConsumption);
                    for (colony, pop) in self
                        .state
                        .colonies
                        .iter_mut()
                        .zip(self.state.populations.iter_mut())
                    {
                        let population_count = f64::from(pop.count);
                        let report = apply_needs_check_scaled(
                            &mut colony.pool,
                            population_count,
                            &config,
                            consumption_scalar,
                        );

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

                        // Apply difficulty scalars: stability_decay is scaled by StabilityRate,
                        // population growth/decline is scaled by PopulationGrowth.
                        let scaled_stability_delta = report.stability_delta * stability_scalar;
                        let scaled_pop_delta = pop_delta * growth_scalar;

                        // Apply stability and population changes.
                        pop.stability = (pop.stability + scaled_stability_delta).clamp(0.0, 1.0);
                        pop.count = (pop.count + scaled_pop_delta).max(0.0);

                        events.push(Event::NeedsResolved {
                            colony_id: colony.id,
                            composite_satisfaction: report.composite_satisfaction,
                            stability_delta: scaled_stability_delta,
                            population_delta: scaled_pop_delta,
                        });
                    }
                }

                // ── Step 3: Production ──────────────────────────────────────
                // Only runs when a content registry is loaded.  Shortfalls are
                // emitted as `ProductionShortfall` events; no crash on partial.
                if let Some(registry) = &self.state.registry.clone() {
                    let power_scalar = self
                        .state
                        .difficulty_scalar
                        .scalar_for(&modifier::ModifiableQuantity::PowerRequirement);
                    let maintenance_scalar = self
                        .state
                        .difficulty_scalar
                        .scalar_for(&modifier::ModifiableQuantity::MaintenanceConsumption);
                    let maintenance_enabled = self.state.maintenance_enabled;
                    // Precompute deposit richness per colony (issue #239)
                    // before the mutable loop below — `colony_deposit_richness`
                    // takes `&self` and can't run once `self.state.colonies`
                    // is borrowed mutably.
                    let colony_deposits: std::collections::HashMap<
                        ColonyId,
                        Option<std::collections::HashMap<String, f32>>,
                    > = self
                        .state
                        .colonies
                        .iter()
                        .map(|c| {
                            (
                                c.id,
                                self.colony_deposit_richness(c.id, c.home_body_id.as_ref()),
                            )
                        })
                        .collect();
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
                        let deposits = colony_deposits.get(&colony.id).and_then(Option::as_ref);
                        let prod_outcome = colony::process_production_scaled(
                            &mut colony.pool,
                            &placed,
                            labor,
                            registry,
                            power_scalar,
                            maintenance_scalar,
                            maintenance_enabled,
                            colony.habitability_modifier,
                            &colony.active_recipes,
                            &colony.category_modifiers,
                            deposits,
                            &self.state.modifier_accumulator,
                            &self.state.difficulty_scalar,
                        );
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
                        colony.last_production = prod_outcome
                            .building_results
                            .into_iter()
                            .map(|r| (r.building_type.clone(), r))
                            .collect();
                    }

                    // ── Step 3b: Outpost production (issue #233) ────────────
                    // Fixed skeleton-crew labor (no population to derive it
                    // from) and a neutral 1.0 habitability modifier — outposts
                    // aren't founded on habitability grounds, just anchored to
                    // a body for its deposits/role. Upkeep shortfalls (power,
                    // maintenance) reduce output via the same scale-factor
                    // logic colonies get, for free, since this reuses
                    // `process_production_scaled` unchanged. Deposit gating
                    // (issue #239) uses the outpost's `body_id` directly —
                    // outposts have no hex placement, only a body link.
                    let outpost_deposits: std::collections::HashMap<
                        outpost::OutpostId,
                        std::collections::HashMap<String, f32>,
                    > = self
                        .state
                        .outposts
                        .iter()
                        .map(|o| (o.id, self.body_deposit_richness(&o.body_id)))
                        .collect();
                    for out in &mut self.state.outposts {
                        let placed: Vec<(String, u32)> = out
                            .buildings
                            .iter()
                            .map(|b| (b.building_type.clone(), b.slot_cost))
                            .collect();
                        out.pool.reset_deltas();
                        let empty_deposits = std::collections::HashMap::new();
                        let deposits = outpost_deposits.get(&out.id).unwrap_or(&empty_deposits);
                        let prod_outcome = colony::process_production_scaled(
                            &mut out.pool,
                            &placed,
                            outpost::OUTPOST_BASE_LABOR,
                            registry,
                            power_scalar,
                            maintenance_scalar,
                            maintenance_enabled,
                            1.0,
                            &out.active_recipes,
                            &out.category_modifiers,
                            Some(deposits),
                            &self.state.modifier_accumulator,
                            &self.state.difficulty_scalar,
                        );
                        for result in &prod_outcome.building_results {
                            for shortfall in &result.shortfalls {
                                events.push(Event::OutpostProductionShortfall {
                                    outpost_id: out.id,
                                    building_type: result.building_type.clone(),
                                    scale: result.scale,
                                    reason: shortfall.reason.clone(),
                                });
                            }
                        }
                        out.last_production = prod_outcome
                            .building_results
                            .into_iter()
                            .map(|r| (r.building_type.clone(), r))
                            .collect();
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

                // ── Step 4e: Field expedition advancement (M8) ───────────
                // Advance each active expedition by one sol: consume supplies,
                // check arrival / on-site survey / return, emit events.
                {
                    let current_sol = self.state.sol;
                    // Collect indices to avoid borrow conflicts.
                    let exp_count = self.state.expeditions.len();
                    let mut expedition_events: Vec<Event> = Vec::new();

                    for i in 0..exp_count {
                        let exp = &mut self.state.expeditions[i];
                        if !exp.is_active() {
                            continue;
                        }
                        // Burn supplies.
                        exp.supplies_remaining -= exp.supply_consumed_per_sol;
                        if exp.supplies_remaining <= expedition::SUPPLY_LOSS_THRESHOLD {
                            // Lost to supply depletion.
                            exp.status = expedition::ExpeditionStatus::Lost;
                            expedition_events.push(Event::ExpeditionLost {
                                expedition_id: exp.id,
                            });
                            continue;
                        }

                        match exp.status {
                            expedition::ExpeditionStatus::InTransit => {
                                if current_sol >= exp.eta_sol {
                                    exp.status = expedition::ExpeditionStatus::OnSite;
                                    exp.sol_arrived = Some(current_sol);
                                    expedition_events.push(Event::ExpeditionArrived {
                                        expedition_id: exp.id,
                                    });
                                }
                            }
                            expedition::ExpeditionStatus::OnSite => {
                                // Simple discovery roll each sol on-site using deterministic
                                // arithmetic (no external RNG dependency in core).
                                let arrived_sol = exp.sol_arrived.unwrap_or(current_sol);
                                let sols_on_site = current_sol.saturating_sub(arrived_sol);

                                // Roll a discovery every other sol on-site (deterministic).
                                if sols_on_site > 0 && sols_on_site.is_multiple_of(2) {
                                    let resource_id = "raw_materials".to_string();
                                    let amount = f64::from(exp.crew_count) * 10.0;
                                    exp.discovered_resources.push((resource_id.clone(), amount));
                                    expedition_events.push(Event::ExpeditionDiscovery {
                                        expedition_id: exp.id,
                                        hex: exp.target_hex,
                                        resource_id,
                                        amount,
                                    });
                                }

                                // Begin return after default on-site period.
                                if sols_on_site >= expedition::DEFAULT_ONSITE_SOLS {
                                    exp.status = expedition::ExpeditionStatus::Returning;
                                }
                            }
                            expedition::ExpeditionStatus::Returning => {
                                // Return transit mirrors outbound; use sol_arrived + transit.
                                let arrived_sol = exp.sol_arrived.unwrap_or(exp.sol_launched);
                                let transit_duration = arrived_sol - exp.sol_launched;
                                let return_eta = arrived_sol
                                    + expedition::DEFAULT_ONSITE_SOLS
                                    + transit_duration;

                                if current_sol >= return_eta {
                                    let deposits = exp.discovered_resources.clone();
                                    exp.status = expedition::ExpeditionStatus::Completed;
                                    let origin = exp.origin_colony;
                                    let eid = exp.id;
                                    let is_deep = exp.is_deep_space;

                                    // Deposit discovered resources into origin colony pool.
                                    if let Some(idx) =
                                        self.state.colonies.iter().position(|c| c.id == origin)
                                    {
                                        for (res_id, amt) in &deposits {
                                            self.state.colonies[idx].pool.deposit(res_id, *amt);
                                        }
                                    }

                                    expedition_events.push(Event::ExpeditionReturned {
                                        expedition_id: eid,
                                        colony_id: origin,
                                        deposits: deposits.clone(),
                                    });

                                    // Deep-space expeditions contribute to the interstellar
                                    // megaproject if one is registered.
                                    if is_deep {
                                        self.state.expedition_launched = true;
                                        if self.state.victory.is_none() {
                                            let snap = self.build_victory_snapshot();
                                            for condition in
                                                self.state.victory_state.evaluate(&snap)
                                            {
                                                self.state.victory = Some(condition.clone());
                                                expedition_events
                                                    .push(Event::VictoryAchieved { condition });
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    events.extend(expedition_events);
                }

                // ── Step 4f: Survey expedition advancement (issue #235) ───
                // Advance each active body-scouting survey expedition by one
                // sol: transit countdown, on-site anomaly checks, and the
                // final `resolve_survey` outcome.
                {
                    let current_sol = self.state.sol;
                    let ids: Vec<expedition::ExpeditionId> = self
                        .state
                        .expedition_registry
                        .iter()
                        .map(|(id, _)| id.clone())
                        .collect();
                    let mut survey_events: Vec<Event> = Vec::new();

                    for id in ids {
                        let Some(state) = self.state.expedition_registry.get_mut(&id) else {
                            continue;
                        };

                        match state.phase {
                            expedition::ExpeditionPhase::InTransit => {
                                state.transit_turns_remaining =
                                    state.transit_turns_remaining.saturating_sub(1);
                                if state.transit_turns_remaining == 0 {
                                    state.phase = expedition::ExpeditionPhase::Surveying;
                                    survey_events.push(Event::SurveyExpeditionArrived {
                                        expedition_id: id.clone(),
                                        target_body: state.target_body.clone(),
                                    });
                                }
                            }
                            expedition::ExpeditionPhase::Surveying => {
                                // Anomaly check first — a triggered anomaly
                                // halts the survey countdown until the player
                                // resolves it via `ResolveMissionDecision`.
                                let mut anomaly_fired = false;
                                if let Some(registry) = self.state.registry.as_ref() {
                                    let mut anomalies: Vec<&expedition::AnomalyDef> =
                                        registry.anomalies().collect();
                                    anomalies.sort_by(|a, b| a.id.cmp(&b.id));

                                    for anomaly in anomalies {
                                        if state.triggered_anomalies.contains(&anomaly.id) {
                                            continue;
                                        }
                                        let salt = expedition::string_salt(&anomaly.id);
                                        let trigger_roll = expedition::deterministic_roll(
                                            id.0,
                                            current_sol ^ 0xA000_0000 ^ salt,
                                        );
                                        if !expedition::check_anomaly_trigger(
                                            anomaly,
                                            state.expedition_type,
                                            trigger_roll,
                                        ) {
                                            continue;
                                        }
                                        let outcome_roll = expedition::deterministic_roll(
                                            id.0,
                                            current_sol ^ 0xB000_0000 ^ salt,
                                        );
                                        let Some(chosen) = expedition::resolve_anomaly_outcome(
                                            anomaly,
                                            outcome_roll,
                                        ) else {
                                            continue;
                                        };

                                        state.triggered_anomalies.push(anomaly.id.clone());
                                        let event = expedition::MidMissionEvent {
                                            id: uuid::Uuid::new_v4(),
                                            title: anomaly.name.clone(),
                                            description: anomaly.description.clone(),
                                            tier: Tier::Blocking,
                                            choices: vec![
                                                expedition::MissionChoice {
                                                    id: "investigate".to_string(),
                                                    label: "Investigate".to_string(),
                                                    effect: expedition::ChoiceEffect::GrantAnomalyOutcome(
                                                        chosen.clone(),
                                                    ),
                                                },
                                                expedition::MissionChoice {
                                                    id: "ignore".to_string(),
                                                    label: "Ignore and continue".to_string(),
                                                    effect: expedition::ChoiceEffect::Narrative,
                                                },
                                            ],
                                        };
                                        state.pending_event = Some(event.clone());
                                        state.phase = expedition::ExpeditionPhase::AwaitingDecision;
                                        survey_events.push(Event::MidMissionEventTriggered {
                                            expedition_id: id.clone(),
                                            event,
                                        });
                                        anomaly_fired = true;
                                        break;
                                    }
                                }

                                if !anomaly_fired {
                                    state.survey_turns_remaining =
                                        state.survey_turns_remaining.saturating_sub(1);
                                    if state.survey_turns_remaining == 0 {
                                        let body = self
                                            .state
                                            .system_state
                                            .node_map
                                            .bodies
                                            .get(&state.target_body);
                                        let (site_name, deposits) = body.map_or_else(
                                            || {
                                                (
                                                    "Unnamed Site".to_string(),
                                                    std::collections::HashMap::new(),
                                                )
                                            },
                                            |b| {
                                                let deposits: std::collections::HashMap<
                                                    String,
                                                    f64,
                                                > = b
                                                    .deposits
                                                    .iter()
                                                    .map(|d| {
                                                        (
                                                            d.commodity_id.clone(),
                                                            f64::from(d.abundance) * 1000.0,
                                                        )
                                                    })
                                                    .collect();
                                                (format!("{} Site", b.name), deposits)
                                            },
                                        );
                                        let roll = expedition::deterministic_roll(
                                            id.0,
                                            current_sol ^ 0xC000_0000,
                                        );
                                        // Combine this mission's own
                                        // mid-mission-choice modifiers with
                                        // the system-wide tech-driven bonus
                                        // (issue #236's sensor_systems/
                                        // deep_space_navigation/
                                        // xenoarchaeology techs).
                                        let combined_modifiers = expedition::SurveyModifiers {
                                            full_reveal_bonus: state.modifiers.full_reveal_bonus
                                                + self
                                                    .state
                                                    .tech_survey_modifiers
                                                    .full_reveal_bonus,
                                            full_reveal_penalty: state
                                                .modifiers
                                                .full_reveal_penalty,
                                            partial_reveal_bonus: state
                                                .modifiers
                                                .partial_reveal_bonus
                                                + self
                                                    .state
                                                    .tech_survey_modifiers
                                                    .partial_reveal_bonus,
                                        };
                                        let outcome = expedition::resolve_survey(
                                            state.expedition_type,
                                            state.target_body.clone(),
                                            &combined_modifiers,
                                            roll,
                                            site_name,
                                            deposits,
                                        );
                                        state.outcome = Some(outcome.clone());
                                        state.phase = expedition::ExpeditionPhase::Completed;
                                        let origin_colony = state.origin_colony;
                                        let target_body = state.target_body.clone();

                                        match &outcome {
                                            expedition::SurveyOutcome::FullReveal {
                                                site_name,
                                                ..
                                            } => {
                                                if let Some(b) = self
                                                    .state
                                                    .system_state
                                                    .node_map
                                                    .bodies
                                                    .get_mut(&target_body)
                                                {
                                                    b.surveyed = true;
                                                    b.candidate_site_name = Some(site_name.clone());
                                                }
                                            }
                                            expedition::SurveyOutcome::PartialReveal { .. } => {
                                                if let Some(b) = self
                                                    .state
                                                    .system_state
                                                    .node_map
                                                    .bodies
                                                    .get_mut(&target_body)
                                                {
                                                    b.surveyed = true;
                                                }
                                            }
                                            expedition::SurveyOutcome::Failed { .. } => {}
                                        }

                                        survey_events.push(Event::SurveyCompleted {
                                            expedition_id: id.clone(),
                                            colony_id: origin_colony,
                                            target_body,
                                            outcome,
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    events.extend(survey_events);
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
                // Insert default directives from the loaded content registry.
                self.insert_default_directives(id);
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
                // Tech gate (issue #247) — only enforced when the registry
                // actually defines this building, mirroring the same
                // None-prerequisite-is-open convention `tech::unlocked_buildings`
                // already applies and the identical check #241 added for
                // `QueueOutpostConstruction`. An unregistered `building_type`
                // string (used by some older tests/harness runs with no
                // registry loaded) is left to fail downstream unchanged.
                if let Some(reg) = self.state.registry.as_ref() {
                    if let Some(def) = reg.building(building_type) {
                        if let Some(tech_id) = &def.tech_prerequisite {
                            if !self.state.tech_state.researched.contains(tech_id) {
                                return Err(EngineError::TechLocked {
                                    building_id: building_type.clone(),
                                    tech_id: tech_id.clone(),
                                });
                            }
                        }
                    }
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

            Command::DeployStarterKit {
                colony_id,
                buildings,
            } => {
                let idx = self.find_colony_index(*colony_id)?;
                if self.state.colonies[idx].starter_kit_deployed {
                    return Err(EngineError::InvalidArgument(
                        "starter kit already deployed for this colony".into(),
                    ));
                }
                if buildings.is_empty() {
                    return Err(EngineError::InvalidArgument(
                        "starter kit batch must not be empty".into(),
                    ));
                }

                // Validate the whole batch before placing anything, so a
                // rejection (bad building_type, tech-locked, over budget)
                // never leaves a partially-deployed kit.
                let mut total_slots: u32 = 0;
                for (building_type, slot_cost) in buildings {
                    if building_type.trim().is_empty() {
                        return Err(EngineError::InvalidArgument(
                            "building_type must not be empty".into(),
                        ));
                    }
                    if let Some(reg) = self.state.registry.as_ref() {
                        if let Some(def) = reg.building(building_type) {
                            if let Some(tech_id) = &def.tech_prerequisite {
                                if !self.state.tech_state.researched.contains(tech_id) {
                                    return Err(EngineError::TechLocked {
                                        building_id: building_type.clone(),
                                        tech_id: tech_id.clone(),
                                    });
                                }
                            }
                        }
                    }
                    total_slots = total_slots.saturating_add(*slot_cost);
                }
                let available = self.state.colonies[idx].slots_available();
                if total_slots > available {
                    return Err(EngineError::SlotCapacityExceeded {
                        needed: total_slots,
                        available,
                    });
                }

                let mut events = Vec::with_capacity(buildings.len());
                for (building_type, slot_cost) in buildings {
                    self.state.colonies[idx]
                        .buildings
                        .push(colony::PlacedBuilding::new(
                            building_type.clone(),
                            *slot_cost,
                        ));
                    events.push(Event::BuildingConstructed {
                        colony_id: *colony_id,
                        building_type: building_type.clone(),
                    });
                }
                self.state.colonies[idx].starter_kit_deployed = true;
                Ok(events)
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

            Command::SetActiveRecipe {
                colony_id,
                building_type,
                recipe_id,
            } => {
                let idx = self.find_colony_index(*colony_id)?;
                let registry = self.state.registry.as_ref().ok_or_else(|| {
                    EngineError::InvalidArgument(
                        "no content registry loaded — cannot validate recipe selection".into(),
                    )
                })?;
                let recipe = registry
                    .recipes()
                    .find(|r| &r.id == recipe_id)
                    .ok_or_else(|| {
                        EngineError::InvalidArgument(format!("unknown recipe: {recipe_id}"))
                    })?;
                if &recipe.building != building_type {
                    return Err(EngineError::InvalidArgument(format!(
                        "recipe '{recipe_id}' belongs to building '{}', not '{building_type}'",
                        recipe.building
                    )));
                }
                self.state.colonies[idx]
                    .active_recipes
                    .insert(building_type.clone(), recipe_id.clone());
                Ok(vec![Event::ActiveRecipeSet {
                    colony_id: *colony_id,
                    building_type: building_type.clone(),
                    recipe_id: recipe_id.clone(),
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
                supplies_id,
                supply_overrides,
                body_id,
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
                // Issue #183: gate on the parent body's habitability score
                // before any mutation, unless the harsh-world capability is
                // unlocked. `home_body_modifier` carries the computed
                // productivity modifier and the body's per-category
                // modifiers (issue #184) forward so the auto-link below
                // doesn't need to look the body up a second time.
                let home_body_modifier = match body_id {
                    Some(bid) => {
                        let body = self
                            .state
                            .system_state
                            .node_map
                            .bodies
                            .get(bid)
                            .ok_or_else(|| {
                                EngineError::InvalidArgument(format!("body not found: {bid}"))
                            })?;
                        if !body.meets_founding_threshold()
                            && !self
                                .state
                                .unlocked_capabilities
                                .contains(system::HARSH_WORLD_CAPABILITY_ID)
                        {
                            return Err(EngineError::HabitabilityBelowThreshold {
                                score: body.habitability(),
                                threshold: system::HABITABILITY_FOUNDING_THRESHOLD,
                            });
                        }
                        Some((
                            body.habitability_modifier_with_mitigations(
                                &self.state.habitability_mitigations,
                            ),
                            body.modifiers.clone(),
                        ))
                    }
                    None => None,
                };
                // Resolve the supply package (if any) up front so an unknown id
                // is reported before any mutation happens. `supply_overrides`
                // (issue #167), when present, takes precedence and is
                // validated against a per-commodity cargo-capacity cap
                // derived from the loaded supply packages.
                let supplies = if let Some(overrides) = supply_overrides.as_deref() {
                    let registry = self.state.registry.as_ref().ok_or_else(|| {
                        EngineError::InvalidArgument(
                            "no content registry loaded — cannot validate supply_overrides".into(),
                        )
                    })?;
                    // Largest authored per-100-colonist amount for each
                    // commodity, across all packages — the cap basis.
                    let mut max_per_100: std::collections::HashMap<&str, f64> =
                        std::collections::HashMap::new();
                    for pkg in registry.supply_packages() {
                        for ing in &pkg.commodities {
                            let slot = max_per_100.entry(ing.id.as_str()).or_insert(0.0);
                            if ing.quantity > *slot {
                                *slot = ing.quantity;
                            }
                        }
                    }
                    // starting_population is a headcount well below 2^52; the
                    // precision loss clippy warns about is unreachable here.
                    #[allow(clippy::cast_precision_loss)]
                    let scale = (*starting_population as f64) / 100.0;
                    let mut resolved = Vec::with_capacity(overrides.len());
                    for (id, qty) in overrides {
                        if *qty < 0.0 {
                            return Err(EngineError::InvalidArgument(format!(
                                "supply override for '{id}' must not be negative"
                            )));
                        }
                        let cap_per_100 =
                            max_per_100.get(id.as_str()).copied().ok_or_else(|| {
                                EngineError::InvalidArgument(format!(
                                    "unknown commodity in supply_overrides: {id}"
                                ))
                            })?;
                        let cap = cap_per_100 * MAX_SUPPLY_OVERRIDE_MULTIPLE * scale;
                        if *qty > cap {
                            return Err(EngineError::InvalidArgument(format!(
                                "supply override for '{id}' ({qty}) exceeds cargo capacity cap ({cap})"
                            )));
                        }
                        resolved.push(content::types::Ingredient {
                            id: id.clone(),
                            quantity: *qty,
                        });
                    }
                    Some((resolved, true))
                } else if let Some(id) = supplies_id.as_deref() {
                    let registry = self.state.registry.as_ref().ok_or_else(|| {
                        EngineError::InvalidArgument(
                            "no content registry loaded — cannot resolve supplies_id".into(),
                        )
                    })?;
                    let pkg = registry.supply_package(id).ok_or_else(|| {
                        EngineError::InvalidArgument(format!("unknown supplies_id: {id}"))
                    })?;
                    Some((pkg.commodities.clone(), false))
                } else {
                    None
                };
                let _ = pm;
                let colony = colony::Colony::new(name.clone());
                let colony_id = colony.id;
                self.state.add_colony(colony, *starting_population);
                // Insert default directives from the loaded content registry.
                self.insert_default_directives(colony_id);
                // Place colony node on the map.
                let pm_mut = self
                    .state
                    .planet_map
                    .as_mut()
                    .ok_or(EngineError::NoPlanetMap)?;
                pm_mut
                    .place_colony(colony_id, coord)
                    .map_err(|e| EngineError::InvalidState(e.to_string()))?;
                // Seed the starter supplies. Package-derived amounts are
                // per-100-colonist and scaled linearly by starting_population;
                // `supply_overrides` amounts (issue #167) are already
                // absolute final quantities and are deposited as-is.
                if let Some((ings, already_absolute)) = supplies {
                    #[allow(clippy::cast_precision_loss)]
                    let scale = (*starting_population as f64) / 100.0;
                    let idx = self
                        .find_colony_index(colony_id)
                        .expect("colony was just inserted");
                    for ing in &ings {
                        let qty = if already_absolute {
                            ing.quantity
                        } else {
                            ing.quantity * scale
                        };
                        if qty > 0.0 {
                            self.state.colonies[idx].pool.deposit(&ing.id, qty);
                        }
                    }
                }
                let mut events = vec![
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
                ];
                // Auto-link the home body now that the gate has passed —
                // equivalent to a follow-up Command::AssignColonyHomeBody,
                // which remains separately callable but is no longer
                // required for this to take effect.
                if let (Some(bid), Some((modifier, category_modifiers))) =
                    (body_id, home_body_modifier)
                {
                    let idx = self
                        .find_colony_index(colony_id)
                        .expect("colony was just inserted");
                    self.state.colonies[idx].home_body_id = Some(bid.clone());
                    self.state.colonies[idx].habitability_modifier = modifier;
                    self.state.colonies[idx].category_modifiers = category_modifiers;
                    events.push(Event::ColonyHomeBodySet {
                        colony_id,
                        body_id: bid.clone(),
                        habitability_modifier: modifier,
                    });
                }
                Ok(events)
            }

            Command::AssignColonyHomeBody { colony_id, body_id } => {
                let idx = self.find_colony_index(*colony_id)?;
                let body = self
                    .state
                    .system_state
                    .node_map
                    .bodies
                    .get(body_id)
                    .ok_or_else(|| {
                        EngineError::InvalidArgument(format!("body not found: {body_id}"))
                    })?;
                let modifier = body
                    .habitability_modifier_with_mitigations(&self.state.habitability_mitigations);
                let category_modifiers = body.modifiers.clone();
                let colony = &mut self.state.colonies[idx];
                colony.home_body_id = Some(body_id.clone());
                colony.habitability_modifier = modifier;
                colony.category_modifiers = category_modifiers;
                Ok(vec![Event::ColonyHomeBodySet {
                    colony_id: *colony_id,
                    body_id: body_id.clone(),
                    habitability_modifier: modifier,
                }])
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

            Command::SetColonyInterruptConfig { colony_id, config } => {
                self.find_colony_index(*colony_id)?;
                self.state
                    .interrupt_configs
                    .insert(*colony_id, config.clone());
                Ok(vec![])
            }

            Command::DebugGrantColonyResources {
                colony_id,
                commodity_id,
                amount,
            } => {
                let idx = self.find_colony_index(*colony_id)?;
                let granted = self.state.colonies[idx]
                    .pool
                    .deposit(commodity_id, amount.max(0.0));
                Ok(vec![Event::DebugResourcesGranted {
                    colony_id: *colony_id,
                    commodity_id: commodity_id.clone(),
                    amount: granted,
                }])
            }

            Command::EstablishOutpost {
                name,
                colony_id,
                body_id,
            } => {
                let colony_idx = self.find_colony_index(*colony_id)?;
                let body = self
                    .state
                    .system_state
                    .node_map
                    .bodies
                    .get(body_id)
                    .ok_or_else(|| {
                        EngineError::InvalidArgument(format!("body not found: {body_id}"))
                    })?;
                // Range gate (issue #241) — only meaningful for a colony that
                // is actually placed on a body (`home_body_id`); a colony
                // founded via the bare `Command::FoundColony` (no spatial
                // placement) has no "distance from home" concept, so the
                // check is inert for it rather than defaulting to a
                // spurious zero distance.
                if let Some(home_body_id) = self.state.colonies[colony_idx].home_body_id.clone() {
                    if let Some(home_body) =
                        self.state.system_state.node_map.bodies.get(&home_body_id)
                    {
                        let distance_au = (home_body.distance_au - body.distance_au).abs();
                        let max_range_au = outpost::max_outpost_range_au(
                            self.state.system_state.node_map.propulsion_level,
                            self.state.outpost_range_bonus_au,
                        );
                        if distance_au > max_range_au {
                            return Err(EngineError::OutpostOutOfRange {
                                distance_au,
                                max_range_au,
                            });
                        }
                    }
                }
                let mut new_outpost =
                    outpost::Outpost::new(name.clone(), *colony_id, body_id.clone());
                new_outpost.category_modifiers.clone_from(&body.modifiers);
                let outpost_id = new_outpost.id;
                self.state.outposts.push(new_outpost);
                Ok(vec![Event::OutpostEstablished {
                    outpost_id,
                    colony_id: *colony_id,
                    body_id: body_id.clone(),
                }])
            }

            Command::DecommissionOutpost { outpost_id } => {
                let idx = self.find_outpost_index(*outpost_id)?;
                self.state.outposts.remove(idx);
                Ok(vec![Event::OutpostDecommissioned {
                    outpost_id: *outpost_id,
                }])
            }

            Command::PromoteOutpostToColony {
                outpost_id,
                name,
                starting_population,
            } => {
                let idx = self.find_outpost_index(*outpost_id)?;
                let old_outpost = self.state.outposts.remove(idx);

                // Prefer the body's current habitability/category modifiers
                // (freshest data) over the outpost's establishment-time
                // cache; fall back to the cache if the body has since been
                // removed from the system.
                let (habitability_modifier, category_modifiers) = self
                    .state
                    .system_state
                    .node_map
                    .bodies
                    .get(&old_outpost.body_id)
                    .map(|body| {
                        (
                            body.habitability_modifier_with_mitigations(
                                &self.state.habitability_mitigations,
                            ),
                            body.modifiers.clone(),
                        )
                    })
                    .unwrap_or((1.0, old_outpost.category_modifiers.clone()));

                let mut colony = colony::Colony::new(name.clone());
                colony.pool = old_outpost.pool;
                colony.buildings = old_outpost.buildings;
                colony.build_queue = old_outpost.build_queue;
                colony.slot_capacity = old_outpost.slot_capacity.max(colony::BASE_SLOT_CAPACITY);
                colony.home_body_id = Some(old_outpost.body_id);
                colony.habitability_modifier = habitability_modifier;
                colony.category_modifiers = category_modifiers;
                colony.active_recipes = old_outpost.active_recipes;
                colony.last_production = old_outpost.last_production;

                let colony_id = colony.id;
                self.state.add_colony(colony, *starting_population);
                self.insert_default_directives(colony_id);

                Ok(vec![Event::OutpostPromoted {
                    outpost_id: *outpost_id,
                    colony_id,
                    name: name.clone(),
                }])
            }

            Command::QueueOutpostConstruction {
                outpost_id,
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
                // Tech gate (issue #241) — only enforced when the registry
                // actually defines this building (content-driven, mirrors
                // `tech::unlocked_buildings`'s None-prerequisite-is-open
                // convention); an unregistered `building_type` string (as
                // used by several older tests) is left to fail downstream
                // rather than being rejected here.
                if let Some(reg) = self.state.registry.as_ref() {
                    if let Some(def) = reg.building(building_type) {
                        if let Some(tech_id) = &def.tech_prerequisite {
                            if !self.state.tech_state.researched.contains(tech_id) {
                                return Err(EngineError::TechLocked {
                                    building_id: building_type.clone(),
                                    tech_id: tech_id.clone(),
                                });
                            }
                        }
                    }
                }
                let idx = self.find_outpost_index(*outpost_id)?;
                let available = self.state.outposts[idx].slots_available();
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
                self.state.outposts[idx].build_queue.enqueue(project);
                Ok(vec![Event::OutpostConstructionQueued {
                    outpost_id: *outpost_id,
                    building_type: building_type.clone(),
                    project_id,
                }])
            }

            Command::SetOutpostActiveRecipe {
                outpost_id,
                building_type,
                recipe_id,
            } => {
                let idx = self.find_outpost_index(*outpost_id)?;
                let registry = self.state.registry.as_ref().ok_or_else(|| {
                    EngineError::InvalidArgument(
                        "no content registry loaded — cannot validate recipe selection".into(),
                    )
                })?;
                let recipe = registry
                    .recipes()
                    .find(|r| &r.id == recipe_id)
                    .ok_or_else(|| {
                        EngineError::InvalidArgument(format!("unknown recipe: {recipe_id}"))
                    })?;
                if &recipe.building != building_type {
                    return Err(EngineError::InvalidArgument(format!(
                        "recipe '{recipe_id}' belongs to building '{}', not '{building_type}'",
                        recipe.building
                    )));
                }
                self.state.outposts[idx]
                    .active_recipes
                    .insert(building_type.clone(), recipe_id.clone());
                Ok(vec![Event::OutpostActiveRecipeSet {
                    outpost_id: *outpost_id,
                    building_type: building_type.clone(),
                    recipe_id: recipe_id.clone(),
                }])
            }

            Command::ContributeOutpostToMegaproject {
                outpost_id,
                project_id,
                resources,
                research,
            } => {
                let idx = self.find_outpost_index(*outpost_id)?;
                // Validate the megaproject can actually accept a contribution
                // *before* touching the outpost's pool — `pool.withdraw` is
                // irreversible (it doesn't just check, it removes), so
                // withdrawing first and validating after would silently
                // destroy resources on a failed contribution (stale
                // `project_id`, already-completed project, etc.).
                let project = self
                    .state
                    .system_state
                    .megaprojects
                    .get(project_id)
                    .ok_or_else(|| {
                        EngineError::InvalidArgument(format!(
                            "megaproject not found: {project_id:?}"
                        ))
                    })?;
                if project.completed {
                    return Err(EngineError::InvalidArgument(format!(
                        "megaproject already complete: {project_id:?}"
                    )));
                }
                if project.next_milestone_index().is_none() {
                    return Err(EngineError::InvalidArgument(format!(
                        "megaproject has no active milestone: {project_id:?}"
                    )));
                }

                let withdrawn: Vec<(String, f64)> = resources
                    .iter()
                    .map(|(commodity_id, amount)| {
                        let actual = self.state.outposts[idx]
                            .pool
                            .withdraw(commodity_id, *amount);
                        (commodity_id.clone(), actual)
                    })
                    .collect();
                let sys_cmd = system::SystemCommand::ContributeToMegaproject {
                    project_id: project_id.clone(),
                    resources: withdrawn.clone(),
                    research: *research,
                };
                let sys_events =
                    system::apply_system_command(&mut self.state.system_state, &sys_cmd)
                        .map_err(|e| EngineError::InvalidArgument(e.to_string()))?;
                let mut events: Vec<Event> = vec![Event::OutpostContributedToMegaproject {
                    outpost_id: *outpost_id,
                    project_id: project_id.clone(),
                    resources: withdrawn,
                    research: *research,
                }];
                events.extend(sys_events.into_iter().map(Event::System));
                Ok(events)
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
                body_id,
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
                    body_id.clone(),
                );
                project.costs_paid = true;
                let build_months = project.months_remaining;
                self.state.orbital_construction_queue.push(project);
                Ok(vec![Event::OrbitalConstructionStarted {
                    blueprint_id: blueprint_id.clone(),
                    colony_id: *colony_id,
                    orbit_type: *orbit_type,
                    build_months,
                    body_id: body_id.clone(),
                }])
            }

            Command::BuildOrbitalStation {
                colony_id,
                station_type,
                orbit_type,
                body_id,
                slot_cost,
            } => {
                self.find_colony_index(*colony_id)?;
                let chosen_slot_cost = slot_cost.unwrap_or_else(|| station_type.slot_cost());
                let station = OrbitalStation::new(
                    *station_type,
                    *orbit_type,
                    *colony_id,
                    body_id.clone(),
                    chosen_slot_cost,
                )?;
                let station_id = station.id;
                let slot_cost = station.slot_cost;
                self.state.orbital_registry.add_station(station)?;
                Ok(vec![Event::OrbitalStationBuilt {
                    station_id,
                    colony_id: *colony_id,
                    station_type: *station_type,
                    orbit_type: *orbit_type,
                    slot_cost,
                    body_id: body_id.clone(),
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
                body_id,
            } => {
                if *count == 0 {
                    return Err(EngineError::InvalidArgument(
                        "constellation count must be > 0".into(),
                    ));
                }
                let constellation = SatelliteConstellation::new(
                    *satellite_type,
                    *orbit_type,
                    *count,
                    body_id.clone(),
                );
                let constellation_id = constellation.id;
                self.state
                    .orbital_registry
                    .deploy_constellation(constellation);
                Ok(vec![Event::ConstellationDeployed {
                    constellation_id,
                    satellite_type: *satellite_type,
                    orbit_type: *orbit_type,
                    count: *count,
                    body_id: body_id.clone(),
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
                // The sol > 0 gate was removed for #161 so the custom-difficulty
                // menu can retune the game mid-campaign.
                self.state.difficulty_preset = *preset;
                self.state.difficulty_scalar =
                    self.state.difficulty_grade_table.build_scalar(*preset);
                Ok(vec![Event::DifficultyChanged { preset: *preset }])
            }

            Command::SetCustomDifficulty {
                scalars,
                menace_enabled,
                hazards_enabled,
                maintenance_enabled,
            } => {
                self.state.difficulty_preset = difficulty::DifficultyPreset::Custom;
                self.state.difficulty_scalar = scalars.clone();
                self.state.hazards_enabled = *hazards_enabled;
                self.state.maintenance_enabled = *maintenance_enabled;
                if *menace_enabled {
                    // Re-attach the last-known definition if the player
                    // toggles menace back on with nothing currently active.
                    if self.state.menace_state.is_none() {
                        if let Some(def) = self.state.last_menace_definition.clone() {
                            self.state.menace_state = Some(menace::MenaceState::new(def));
                        }
                    }
                } else {
                    self.state.menace_state = None;
                }
                Ok(vec![Event::DifficultyChanged {
                    preset: difficulty::DifficultyPreset::Custom,
                }])
            }

            Command::SetHazardsEnabled { enabled } => {
                self.state.hazards_enabled = *enabled;
                Ok(vec![])
            }

            Command::SetMaintenanceEnabled { enabled } => {
                self.state.maintenance_enabled = *enabled;
                Ok(vec![])
            }

            Command::ActivateMenace { definition } => {
                if let Some(def) = definition.as_ref() {
                    self.state.last_menace_definition = Some(def.clone());
                }
                self.state.menace_state = definition
                    .as_ref()
                    .map(|d| menace::MenaceState::new(d.clone()));
                Ok(vec![])
            }

            Command::TickMenace => {
                let mut events = Vec::new();
                if let Some(ms) = &mut self.state.menace_state {
                    let outcome = ms.tick();

                    // ── Phase-based events ────────────────────────────────────
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

                    // ── Continuous-level events ───────────────────────────────
                    if outcome.just_went_critical {
                        let countdown_months = ms
                            .countdown
                            .unwrap_or(menace::MenaceState::DEFAULT_COUNTDOWN);
                        events.push(Event::MenaceCritical {
                            kind: outcome.kind,
                            level: ms.level,
                            countdown_months,
                        });
                    }
                    if outcome.game_over {
                        events.push(Event::GameOver {
                            reason: outcome.kind,
                        });
                    }
                }
                Ok(events)
            }

            Command::MitigateMenace {
                colony_id,
                resource_id,
                resource_cost,
                amount,
            } => {
                let idx = self.find_colony_index(*colony_id)?;
                let available = self.state.colonies[idx].pool.amount(resource_id);
                if available < *resource_cost {
                    return Err(EngineError::InvalidArgument(format!(
                        "insufficient {resource_id}: need {resource_cost} but have {available:.2}"
                    )));
                }
                self.state.colonies[idx]
                    .pool
                    .withdraw(resource_id, *resource_cost);
                let Some(ms) = &mut self.state.menace_state else {
                    return Err(EngineError::InvalidState(
                        "no active menace to mitigate".into(),
                    ));
                };
                ms.mitigate(*amount);
                Ok(vec![])
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

            // ── M8: Field Expeditions ─────────────────────────────────────────
            Command::LaunchFieldExpedition {
                colony_id,
                target_hex,
                crew_count,
                supplies,
                transit_sols,
                is_deep_space,
            } => {
                // Validate colony exists.
                let _colony_idx = self
                    .state
                    .colonies
                    .iter()
                    .position(|c| &c.id == colony_id)
                    .ok_or(EngineError::ColonyNotFound(*colony_id))?;

                #[allow(clippy::cast_precision_loss)]
                let supply_per_sol = *crew_count as f32;
                let expedition = expedition::Expedition::new(
                    *colony_id,
                    *target_hex,
                    *crew_count,
                    *supplies,
                    // Default supply burn: 1 unit/sol per crew member.
                    supply_per_sol,
                    self.state.sol,
                    *transit_sols,
                    *is_deep_space,
                );
                let eid = expedition.id;
                let hex = expedition.target_hex;
                self.state.expeditions.push(expedition);
                Ok(vec![Event::ExpeditionLaunched {
                    expedition_id: eid,
                    colony_id: *colony_id,
                    target_hex: hex,
                }])
            }

            Command::RecallExpedition { expedition_id } => {
                let exp = self
                    .state
                    .expeditions
                    .iter_mut()
                    .find(|e| &e.id == expedition_id)
                    .ok_or_else(|| {
                        EngineError::InvalidArgument(format!(
                            "expedition {:?} not found",
                            expedition_id.0
                        ))
                    })?;

                if exp.status == expedition::ExpeditionStatus::OnSite {
                    exp.status = expedition::ExpeditionStatus::Returning;
                }
                Ok(vec![])
            }

            Command::LaunchSurveyExpedition {
                colony_id,
                target_body,
                expedition_type,
            } => {
                self.find_colony_index(*colony_id)?;
                if !self
                    .state
                    .system_state
                    .node_map
                    .bodies
                    .contains_key(target_body)
                {
                    return Err(EngineError::InvalidArgument(format!(
                        "unknown system body: {:?}",
                        target_body.0
                    )));
                }

                let mut state = expedition::ExpeditionState::new(
                    *expedition_type,
                    target_body.clone(),
                    *colony_id,
                );
                // Apply tech-driven propulsion scaling (issue #236) to the
                // transit leg only — survey duration is a function of
                // mission thoroughness, not travel speed.
                #[allow(
                    clippy::cast_precision_loss,
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss
                )]
                {
                    let scaled = (state.transit_turns_remaining as f32
                        * self.state.propulsion_transit_scalar)
                        .max(1.0);
                    state.transit_turns_remaining = scaled.round() as u32;
                }
                let eid = state.id.clone();
                self.state.expedition_registry.launch(state);
                Ok(vec![Event::SurveyExpeditionLaunched {
                    expedition_id: eid,
                    colony_id: *colony_id,
                    target_body: target_body.clone(),
                    expedition_type: *expedition_type,
                }])
            }

            Command::ResolveMissionDecision {
                expedition_id,
                choice_id,
            } => {
                // Peek at the pending choice before `resolve_decision` consumes
                // it, so a `GrantAnomalyOutcome` reward can be applied to
                // `GameState` afterwards — `ExpeditionRegistry` itself has no
                // access to the research pool, colony pools, or tech registry.
                let reward = self
                    .state
                    .expedition_registry
                    .get(expedition_id)
                    .and_then(|s| s.pending_event.as_ref())
                    .and_then(|ev| ev.choices.iter().find(|c| c.id == *choice_id))
                    .and_then(|c| match &c.effect {
                        expedition::ChoiceEffect::GrantAnomalyOutcome(outcome) => {
                            Some(outcome.clone())
                        }
                        _ => None,
                    });

                self.state
                    .expedition_registry
                    .resolve_decision(expedition_id, choice_id)
                    .map_err(|e| EngineError::InvalidArgument(e.to_string()))?;

                let mut events = vec![Event::MissionDecisionResolved {
                    expedition_id: expedition_id.clone(),
                    choice_id: choice_id.clone(),
                }];

                if let Some(outcome) = reward {
                    if outcome.research_bonus > 0.0 {
                        self.state.research_pool.deposit(outcome.research_bonus);
                    }
                    if !outcome.resource_reward.is_empty() {
                        let origin_colony = self
                            .state
                            .expedition_registry
                            .get(expedition_id)
                            .map(|s| s.origin_colony);
                        if let Some(colony_id) = origin_colony {
                            if let Ok(idx) = self.find_colony_index(colony_id) {
                                for (commodity_id, qty) in &outcome.resource_reward {
                                    self.state.colonies[idx].pool.deposit(commodity_id, *qty);
                                }
                            }
                        }
                    }
                    if let Some(tech_id) = &outcome.unlocks_tech {
                        if !self.state.tech_state.is_researched(tech_id) {
                            let effects = self
                                .state
                                .tech_registry
                                .as_ref()
                                .and_then(|r| r.get(tech_id))
                                .map(|def| def.effects.clone());
                            if let Some(effects) = effects {
                                self.state.tech_state.researched.insert(tech_id.clone());
                                turn::TurnProcessor::apply_tech_effects(&mut self.state, &effects);
                                events.push(Event::TechUnlocked {
                                    tech_id: tech_id.clone(),
                                });
                            }
                        }
                    }
                    events.push(Event::AnomalyOutcomeResolved {
                        expedition_id: expedition_id.clone(),
                        outcome,
                    });
                }

                // An AbortMission choice resolves straight to a terminal
                // `Aborted` phase with a `Failed` outcome already recorded —
                // surface that as a normal survey completion too.
                if let Some(state) = self.state.expedition_registry.get(expedition_id) {
                    if state.phase == expedition::ExpeditionPhase::Aborted {
                        if let Some(outcome) = state.outcome.clone() {
                            events.push(Event::SurveyCompleted {
                                expedition_id: expedition_id.clone(),
                                colony_id: state.origin_colony,
                                target_body: state.target_body.clone(),
                                outcome,
                            });
                        }
                    }
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
                // Drop any queued copy of this tech: promoting it straight to
                // `current_project` here must not leave it in
                // `research_queue` too, or a later drain would complete it
                // (and reapply its effects) a second time (issue #250 review).
                self.state
                    .tech_state
                    .research_queue
                    .retain(|t| t != tech_id);
                self.state.tech_state.set_current_project(tech_id.clone());
                Ok(vec![Event::ResearchStarted {
                    tech_id: tech_id.clone(),
                }])
            }

            Command::EnqueueResearch { tech_id } => {
                let registry = self.state.tech_registry.as_ref().ok_or_else(|| {
                    EngineError::InvalidArgument("no tech registry loaded".into())
                })?;
                let def = registry.get(tech_id).ok_or_else(|| {
                    EngineError::InvalidArgument(format!("unknown tech '{tech_id}'"))
                })?;
                // Same gate as `ResearchTech`: only a directly-available tech
                // (prerequisites already researched) may be queued. Prevents
                // a locked tech from being promoted straight to
                // `current_project` on a later drain with its prerequisites
                // never having been researched (issue #250 review).
                if !self.state.tech_state.prerequisites_met(def) {
                    return Err(EngineError::InvalidArgument(format!(
                        "prerequisites not met for tech '{tech_id}'"
                    )));
                }
                if self.state.tech_state.is_researched(tech_id)
                    || self.state.tech_state.current_project.as_deref() == Some(tech_id.as_str())
                    || self
                        .state
                        .tech_state
                        .research_queue
                        .iter()
                        .any(|t| t == tech_id)
                {
                    return Err(EngineError::InvalidArgument(format!(
                        "tech '{tech_id}' is already researched, active, or queued"
                    )));
                }
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
                    .map(|(c, p)| {
                        #[allow(clippy::cast_possible_truncation)]
                        let commodity_pool: Vec<(String, f32)> = c
                            .pool
                            .commodity_ids()
                            .map(|cid| (cid.to_string(), c.pool.amount(cid) as f32))
                            .collect();
                        let buildings = c
                            .buildings
                            .iter()
                            .map(|b| b.building_type.clone())
                            .collect();
                        let active_construction = c
                            .build_queue
                            .projects
                            .iter()
                            .map(|proj| proj.building_type.clone())
                            .collect();
                        ColonySummary {
                            id: c.id,
                            name: c.name.clone(),
                            population: p.count,
                            stability: p.stability,
                            available_labour: p.available_labor(),
                            commodity_pool,
                            buildings,
                            active_construction,
                        }
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

            Query::BuildingDetail {
                colony_id,
                building_type,
            } => {
                let idx = self.find_colony_index(*colony_id)?;
                let colony = &self.state.colonies[idx];
                let data = self.build_building_detail_data(
                    building_type,
                    &colony.active_recipes,
                    &colony.last_production,
                )?;
                Ok(QueryResult::BuildingDetail(data))
            }

            Query::OutpostBuildingDetail {
                outpost_id,
                building_type,
            } => {
                let idx = self.find_outpost_index(*outpost_id)?;
                let post = &self.state.outposts[idx];
                let data = self.build_building_detail_data(
                    building_type,
                    &post.active_recipes,
                    &post.last_production,
                )?;
                Ok(QueryResult::BuildingDetail(data))
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
                // Per-colony config filtering: if the interrupt is scoped to a
                // colony that has a non-default config, drop sources not in the
                // mask.  System-wide interrupts (colony_id == None) are always
                // surfaced.
                if let Some(cid) = irq.colony_id {
                    if let Some(cfg) = self.state.interrupt_configs.get(&cid) {
                        if !cfg.allows(&irq.source) {
                            continue;
                        }
                    }
                }

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
    #[allow(clippy::too_many_lines)]
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

        // Mid-mission events (typically anomaly encounters on a survey
        // expedition) → reuse EventFired at the event's own authored tier
        // (issue #235's "reuse the interrupt + predicate system").
        for ev in events {
            if let Event::MidMissionEventTriggered { event, .. } = ev {
                interrupts.push(Interrupt::new(
                    event.tier,
                    InterruptSource::EventFired(format!("mid_mission:{}", event.id)),
                    None,
                    format!("{}: {}", event.title, event.description),
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

        // Economic output: base_value-weighted sum of this turn's actual
        // production across every colony (issue #212 — base_value's first
        // mechanical consumer). For each building that produced this turn,
        // look up the recipe it ran and value each output at
        // `quantity * scale * commodity.base_value`. Static/authored
        // base_value only — no supply/demand pricing (deferred per #212's
        // design decision). Zero when no registry is loaded (e.g. tests
        // that construct `GameState` directly).
        let total_output: f64 = self.state.registry.as_ref().map_or(0.0, |registry| {
            self.state
                .colonies
                .iter()
                .flat_map(|colony| colony.last_production.values())
                .filter_map(|result| registry.recipe(&result.recipe_id).map(|r| (result, r)))
                .map(|(result, recipe)| {
                    recipe
                        .outputs
                        .iter()
                        .map(|output| {
                            let base_value =
                                registry.commodity(&output.id).map_or(0.0, |c| c.base_value);
                            output.quantity * result.scale * base_value
                        })
                        .sum::<f64>()
                })
                .sum()
        });

        victory::VictorySnapshot {
            expedition_launched: self.state.expedition_launched,
            total_output: total_output as u64,
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

    /// Deposit richness available at a system body, for deposit-gated
    /// extraction recipes (issue #239) — the max
    /// [`system::BodyDeposit::abundance`] per commodity id. Empty if the
    /// body is unknown or has no deposits.
    fn body_deposit_richness(
        &self,
        body_id: &system::BodyId,
    ) -> std::collections::HashMap<String, f32> {
        let mut out = std::collections::HashMap::new();
        if let Some(body) = self.state.system_state.node_map.bodies.get(body_id) {
            for d in &body.deposits {
                let entry = out.entry(d.commodity_id.clone()).or_insert(0.0_f32);
                *entry = entry.max(d.abundance);
            }
        }
        out
    }

    /// Deposit richness available to a colony, for deposit-gated extraction
    /// recipes (issue #239) — the max of any matching hex
    /// [`map::Deposit::richness`] (for a colony founded via
    /// `Command::FoundColonyAtSite`, keyed off its planet-map hex) and any
    /// matching [`system::BodyDeposit::abundance`] (for a colony linked to a
    /// system body via `home_body_id`).
    ///
    /// Returns `None` — gating inert — only for a colony with **no spatial
    /// placement at all**: not on the planet map (never `FoundColonyAtSite`d,
    /// or no planet map seeded) *and* no `home_body_id` (e.g. one created via
    /// the bare `Command::FoundColony` test/fixture path). A colony that
    /// genuinely has a site or body — even one whose hex/body happens to
    /// carry zero recorded deposits — returns `Some(map)` (possibly empty),
    /// since "placed somewhere with nothing there" must still gate, unlike
    /// "no placement data exists to check against."
    fn colony_deposit_richness(
        &self,
        colony_id: ColonyId,
        home_body_id: Option<&system::BodyId>,
    ) -> Option<std::collections::HashMap<String, f32>> {
        let mut out = std::collections::HashMap::new();
        let mut placed = false;
        if let Some(pm) = &self.state.planet_map {
            if let Some(node) = pm.colonies.iter().find(|n| n.colony_id == colony_id) {
                placed = true;
                if let Some(cell) = pm.cells.get(&node.coord) {
                    for d in &cell.deposits {
                        let entry = out.entry(d.commodity_id.clone()).or_insert(0.0_f32);
                        *entry = entry.max(d.richness);
                    }
                }
            }
        }
        if let Some(body_id) = home_body_id {
            placed = true;
            for (k, v) in self.body_deposit_richness(body_id) {
                let entry = out.entry(k).or_insert(0.0_f32);
                *entry = entry.max(v);
            }
        }
        placed.then_some(out)
    }

    /// Find the index of a colony by ID, or return [`EngineError::ColonyNotFound`].
    fn find_colony_index(&self, id: ColonyId) -> Result<usize, EngineError> {
        self.state
            .colonies
            .iter()
            .position(|c| c.id == id)
            .ok_or(EngineError::ColonyNotFound(id))
    }

    /// Find the index of an outpost by ID, or return [`EngineError::OutpostNotFound`].
    fn find_outpost_index(&self, id: outpost::OutpostId) -> Result<usize, EngineError> {
        self.state
            .outposts
            .iter()
            .position(|o| o.id == id)
            .ok_or(EngineError::OutpostNotFound(id))
    }

    /// Build [`ui::BuildingDetailData`] for one building type, given the
    /// owner's (colony's or outpost's) active-recipe selections and last
    /// production outcomes. Shared by [`Query::BuildingDetail`] and
    /// [`Query::OutpostBuildingDetail`] — the response shape is identical,
    /// only the owner differs.
    fn build_building_detail_data(
        &self,
        building_type: &str,
        active_recipes: &std::collections::HashMap<String, String>,
        last_production: &std::collections::HashMap<String, colony::BuildingProductionResult>,
    ) -> Result<ui::BuildingDetailData, EngineError> {
        let registry = self
            .state
            .registry
            .as_ref()
            .ok_or_else(|| EngineError::InvalidArgument("no content registry loaded".into()))?;
        let def = registry
            .buildings()
            .find(|b| b.id == building_type)
            .ok_or_else(|| {
                EngineError::InvalidArgument(format!("unknown building type: {building_type}"))
            })?;

        let recipe =
            colony::production::recipe_for_building(building_type, active_recipes, registry)
                .map(recipe_to_row);

        let mut available_recipes: Vec<&content::types::RecipeDef> = registry
            .recipes()
            .filter(|r| r.building == building_type && !r.concurrent)
            .collect();
        available_recipes.sort_by(|a, b| a.id.cmp(&b.id));
        let available_recipes: Vec<ui::RecipeRow> = if available_recipes.len() > 1 {
            available_recipes.into_iter().map(recipe_to_row).collect()
        } else {
            Vec::new()
        };

        let last_run = last_production
            .get(building_type)
            .map(|r| ui::BuildingRunRow {
                scale: r.scale,
                is_full_production: r.is_full_production(),
                shortfalls: r
                    .shortfalls
                    .iter()
                    .map(|s| {
                        let (kind, commodity_id) = match &s.reason {
                            colony::ShortfallReason::InputShort { commodity_id } => {
                                ("input_short", Some(commodity_id.clone()))
                            }
                            colony::ShortfallReason::PowerBrownout => ("power_brownout", None),
                            colony::ShortfallReason::LaborShort => ("labor_short", None),
                            colony::ShortfallReason::MaintenanceShort { commodity_id } => {
                                ("maintenance_short", Some(commodity_id.clone()))
                            }
                            colony::ShortfallReason::DepositShort { commodity_id } => {
                                ("deposit_short", Some(commodity_id.clone()))
                            }
                        };
                        ui::ShortfallRow {
                            kind: kind.to_string(),
                            commodity_id,
                            effective_scale: s.effective_scale,
                        }
                    })
                    .collect(),
            });

        Ok(ui::BuildingDetailData {
            building_type: def.id.clone(),
            name: def.name.clone(),
            description: def.description.clone(),
            category: format!("{:?}", def.category),
            slot_cost: def.slot_cost,
            power_delta: def.power_delta,
            maintenance: def
                .maintenance
                .iter()
                .map(|i| ui::IngredientRow {
                    commodity_id: i.id.clone(),
                    quantity: i.quantity,
                })
                .collect(),
            recipe,
            available_recipes,
            concurrent_recipes: colony::production::concurrent_recipes_for_building(
                building_type,
                registry,
            )
            .into_iter()
            .map(recipe_to_row)
            .collect(),
            last_run,
        })
    }

    /// Instantiate default directives from the loaded content registry and
    /// insert them into the `DirectiveStore` keyed by `colony_id`.
    ///
    /// No-op when no content registry is loaded.
    fn insert_default_directives(&mut self, colony_id: ColonyId) {
        use content::types::DefaultAction;

        let Some(registry) = &self.state.registry else {
            return;
        };
        let templates: Vec<_> = registry.default_directives().to_vec();
        for def in templates {
            let action = match def.action {
                DefaultAction::AdvanceColonySol => Command::AdvanceColonySol,
                DefaultAction::AssignLabourFraction { slot, fraction } => {
                    // Snapshot available labour at founding (may be 0 until first sol).
                    let labour = self
                        .state
                        .populations
                        .iter()
                        .zip(self.state.colonies.iter())
                        .find(|(_, c)| c.id == colony_id)
                        .map_or(0, |(pop, _)| {
                            #[allow(
                                clippy::cast_possible_truncation,
                                clippy::cast_sign_loss,
                                clippy::cast_precision_loss
                            )]
                            let units =
                                ((pop.available_labour() as f64) * f64::from(fraction)) as u64;
                            units
                        });
                    Command::AssignLabour {
                        colony_id,
                        slot,
                        labour,
                    }
                }
            };
            let directive =
                directive::Directive::new(colony_id, def.predicate, action, def.priority);
            self.state.directive_store.set_directive(directive);
        }
    }
}

/// Map a content-pack [`content::types::RecipeDef`] onto its wire
/// [`ui::RecipeRow`]. Shared by [`GameEngine::build_building_detail_data`]'s
/// active/available/concurrent recipe projections.
fn recipe_to_row(r: &content::types::RecipeDef) -> ui::RecipeRow {
    ui::RecipeRow {
        recipe_id: r.id.clone(),
        name: r.name.clone(),
        inputs: r
            .inputs
            .iter()
            .map(|i| ui::IngredientRow {
                commodity_id: i.id.clone(),
                quantity: i.quantity,
            })
            .collect(),
        outputs: r
            .outputs
            .iter()
            .map(|i| ui::IngredientRow {
                commodity_id: i.id.clone(),
                quantity: i.quantity,
            })
            .collect(),
        cycle_sols: r.cycle_sols,
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
    fn assign_colony_home_body_copies_habitability_modifier() {
        // Bootstrap: create an inner planet, mark its attributes hostile, found
        // a colony, then link the colony to the body. Expect the colony's
        // habitability_modifier to reflect the body's derived value (issue #163).
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Hellworld".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 0.3,
            }))
            .unwrap();
        let body_id = match &events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        engine
            .apply(&Command::System(system::SystemCommand::SetBodyAttributes {
                body_id: body_id.clone(),
                atmosphere_density: system::AtmosphereDensity::Dense,
                atmosphere_hazard: system::AtmosphereHazard::Toxic,
                temperature: system::TemperatureBand::Extreme,
                gravity_g: 0.0,
                radiation: system::RadiationLevel::High,
                subtype: system::PlanetarySubtype::Molten,
                tidally_locked: false,
                axial_tilt_deg: 23.5,
                rotation_period_hours: 24.0,
                moon_count: 0,
            }))
            .unwrap();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Doomed".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded");
        };
        let colony_id = *colony_id;
        let events = engine
            .apply(&Command::AssignColonyHomeBody {
                colony_id,
                body_id: body_id.clone(),
            })
            .unwrap();
        let Event::ColonyHomeBodySet {
            habitability_modifier,
            ..
        } = &events[0]
        else {
            panic!("expected ColonyHomeBodySet");
        };
        assert!((habitability_modifier - 0.75).abs() < 1e-4);
        let idx = engine.find_colony_index(colony_id).unwrap();
        assert!((engine.state.colonies[idx].habitability_modifier - 0.75).abs() < 1e-4);
        assert_eq!(
            engine.state.colonies[idx].home_body_id.as_ref(),
            Some(&body_id)
        );
    }

    #[test]
    fn assign_colony_home_body_copies_category_modifiers() {
        // Issue #184: AssignColonyHomeBody should also cache the body's
        // authored per-category modifiers onto the colony, alongside the
        // flat habitability_modifier.
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Forgeworld".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 0.3,
            }))
            .unwrap();
        let body_id = match &events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        let authored_modifiers = vec![system::BodyModifier {
            category: system::YieldCategory::IndustryYield,
            multiplier: 1.5,
        }];
        engine
            .apply(&Command::System(system::SystemCommand::SetBodyModifiers {
                body_id: body_id.clone(),
                modifiers: authored_modifiers.clone(),
            }))
            .unwrap();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Forge".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded");
        };
        let colony_id = *colony_id;
        engine
            .apply(&Command::AssignColonyHomeBody { colony_id, body_id })
            .unwrap();
        let idx = engine.find_colony_index(colony_id).unwrap();
        assert_eq!(
            engine.state.colonies[idx].category_modifiers,
            authored_modifiers
        );
    }

    #[test]
    fn assign_colony_home_body_fails_when_body_unknown() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::FoundColony {
                name: "Alpha".into(),
                starting_population: 100,
            })
            .unwrap();
        let colonies = engine.state.colonies.clone();
        let colony_id = colonies[0].id;
        let bogus = system::BodyId::new();
        let err = engine
            .apply(&Command::AssignColonyHomeBody {
                colony_id,
                body_id: bogus,
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
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

    // ── Default directives ────────────────────────────────────────────────

    /// Build a minimal content registry that includes all three default directives
    /// from the embedded YAML string (mirrors `content/core/default_directives.yaml`).
    fn registry_with_defaults() -> crate::content::ContentRegistry {
        use crate::content::PackLoader;
        const MANIFEST: &str = "id: test\nname: Test\nversion: \"0.1.0\"";
        const DIRECTIVES: &str = r#"
- label: maintain_water_stockpile
  predicate:
    kind: StockpileBelow
    commodity_id: water
    threshold: 10.0
  action:
    kind: AdvanceColonySol
  priority: 80

- label: prioritise_food_production
  predicate:
    kind: StockpileBelow
    commodity_id: food
    threshold: 20.0
  action:
    kind: AdvanceColonySol
  priority: 70

- label: allocate_life_support_labour
  predicate:
    kind: Always
  action:
    kind: AssignLabourFraction
    slot: life_support
    fraction: 0.1
  priority: 60
"#;
        PackLoader::load(&[
            ("pack.yaml", MANIFEST),
            ("default_directives.yaml", DIRECTIVES),
        ])
        .expect("test registry loads")
    }

    #[test]
    fn found_colony_with_registry_populates_directive_store() {
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_defaults());
        let events = engine
            .apply(&Command::FoundColony {
                name: "New Haven".into(),
                starting_population: 200,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded");
        };
        let directives: Vec<_> = engine
            .state
            .directive_store
            .directives
            .iter()
            .filter(|d| d.colony_id == *colony_id)
            .collect();
        assert!(
            directives.len() >= 3,
            "expected at least 3 default directives, got {}",
            directives.len()
        );
    }

    #[test]
    fn found_colony_without_registry_has_empty_directive_store() {
        let mut engine = GameEngine::new();
        // No registry loaded — directive store should remain empty.
        engine
            .apply(&Command::FoundColony {
                name: "Ghost Colony".into(),
                starting_population: 50,
            })
            .unwrap();
        assert!(engine.state.directive_store.directives.is_empty());
    }

    #[test]
    fn default_directives_evaluate_correctly_on_first_sol() {
        use crate::predicate::PredicateContext;
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_defaults());
        let events = engine
            .apply(&Command::FoundColony {
                name: "Eden".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded");
        };
        let colony_id = *colony_id;
        // Build a context with water = 5 (below 10 threshold) — water directive should fire.
        let mut commodities = std::collections::HashMap::new();
        commodities.insert(
            "water".to_string(),
            crate::predicate::CommoditySnapshot {
                amount: 5.0,
                delta: 0.0,
            },
        );
        let ctx = PredicateContext {
            colony_id,
            population: 100.0,
            stability: 0.8,
            available_labour: 80.0,
            system_research: 0.0,
            sol: 1,
            month: 0,
            commodities,
        };
        let action = engine
            .state
            .directive_store
            .evaluate_for_colony(colony_id, &ctx);
        assert!(
            action.is_some(),
            "expected a directive to fire when water is below threshold"
        );
    }

    #[test]
    fn player_can_remove_default_directive() {
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_defaults());
        let events = engine
            .apply(&Command::FoundColony {
                name: "Outpost Alpha".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded");
        };
        let colony_id = *colony_id;
        let dir_id = engine
            .state
            .directive_store
            .directives
            .iter()
            .find(|d| d.colony_id == colony_id)
            .map(|d| d.id)
            .expect("at least one directive exists");
        engine
            .apply(&Command::RemoveDirective {
                directive_id: dir_id,
            })
            .unwrap();
        assert!(
            engine
                .state
                .directive_store
                .directives
                .iter()
                .all(|d| d.id != dir_id),
            "removed directive should be gone"
        );
    }

    #[test]
    fn player_can_override_default_directive() {
        use crate::directive::Directive;
        use crate::predicate::Predicate;
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_defaults());
        let events = engine
            .apply(&Command::FoundColony {
                name: "Outpost Beta".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded");
        };
        let colony_id = *colony_id;
        // Replace any existing directive with a custom one.
        let custom = Directive::new(colony_id, Predicate::Never, Command::AdvanceColonySol, 255);
        let custom_id = custom.id;
        engine
            .apply(&Command::SetDirective {
                directive: Box::new(custom),
            })
            .unwrap();
        assert!(
            engine
                .state
                .directive_store
                .directives
                .iter()
                .any(|d| d.id == custom_id && d.priority == 255),
            "custom override directive should be present with priority 255"
        );
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
    fn queue_construction_fails_when_tech_not_researched() {
        use crate::content::{BuildingCategory, BuildingDef, ContentRegistry};

        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Delta".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "advanced_reactor".into(),
            name: "Advanced Reactor".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: Some("fusion_engineering".into()),
            maintenance: vec![],
        });
        engine.state.registry = Some(reg);

        let result = engine.apply(&queue_cmd(colony_id, "advanced_reactor", 1));
        assert!(
            matches!(
                result,
                Err(EngineError::TechLocked { ref tech_id, .. }) if tech_id == "fusion_engineering"
            ),
            "expected TechLocked, got {result:?}"
        );
    }

    #[test]
    fn queue_construction_succeeds_once_tech_researched() {
        use crate::content::{BuildingCategory, BuildingDef, ContentRegistry};

        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Delta".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "advanced_reactor".into(),
            name: "Advanced Reactor".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: Some("fusion_engineering".into()),
            maintenance: vec![],
        });
        engine.state.registry = Some(reg);
        engine
            .state
            .tech_state
            .researched
            .insert("fusion_engineering".into());

        let result = engine.apply(&queue_cmd(colony_id, "advanced_reactor", 1));
        assert!(result.is_ok(), "expected success, got {result:?}");
    }

    #[test]
    fn queue_construction_ignores_tech_gate_when_no_registry_loaded() {
        // Grandfathering: with no content registry loaded (bare-engine
        // tests, some harness runs), the gate must stay fully inert rather
        // than rejecting every building_type outright.
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Delta".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;
        assert!(engine.state.registry.is_none());

        let result = engine.apply(&queue_cmd(colony_id, "anything_unregistered", 1));
        assert!(result.is_ok(), "expected success, got {result:?}");
    }

    // ── DeployStarterKit ──

    #[test]
    fn deploy_starter_kit_places_buildings_instantly() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Founding".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let events = engine
            .apply(&Command::DeployStarterKit {
                colony_id,
                buildings: vec![("colony_hq".into(), 1), ("habitat_dome".into(), 1)],
            })
            .unwrap();

        assert_eq!(events.len(), 2);
        assert!(events.iter().all(|e| matches!(
            e,
            Event::BuildingConstructed { colony_id: cid, .. } if *cid == colony_id
        )));

        let colony = engine
            .state
            .colonies
            .iter()
            .find(|c| c.id == colony_id)
            .unwrap();
        assert_eq!(colony.buildings.len(), 2);
        assert!(colony.build_queue.projects.is_empty());
        assert!(colony.starter_kit_deployed);
    }

    #[test]
    fn deploy_starter_kit_rejects_empty_batch_without_consuming_one_shot() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Founding".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let result = engine.apply(&Command::DeployStarterKit {
            colony_id,
            buildings: vec![],
        });
        assert!(
            matches!(result, Err(EngineError::InvalidArgument(_))),
            "expected InvalidArgument for an empty batch, got {result:?}"
        );

        let colony = engine
            .state
            .colonies
            .iter()
            .find(|c| c.id == colony_id)
            .unwrap();
        assert!(
            !colony.starter_kit_deployed,
            "a rejected empty batch must not consume the one-shot flag"
        );
    }

    #[test]
    fn deploy_starter_kit_fails_when_tech_not_researched() {
        use crate::content::{BuildingCategory, BuildingDef, ContentRegistry};

        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Delta".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "advanced_reactor".into(),
            name: "Advanced Reactor".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: Some("fusion_engineering".into()),
            maintenance: vec![],
        });
        engine.state.registry = Some(reg);

        let result = engine.apply(&Command::DeployStarterKit {
            colony_id,
            buildings: vec![("advanced_reactor".into(), 1)],
        });
        assert!(
            matches!(
                result,
                Err(EngineError::TechLocked { ref tech_id, .. }) if tech_id == "fusion_engineering"
            ),
            "expected TechLocked, got {result:?}"
        );

        let colony = engine
            .state
            .colonies
            .iter()
            .find(|c| c.id == colony_id)
            .unwrap();
        assert!(colony.buildings.is_empty());
        assert!(!colony.starter_kit_deployed);
    }

    #[test]
    fn deploy_starter_kit_succeeds_once_tech_researched() {
        use crate::content::{BuildingCategory, BuildingDef, ContentRegistry};

        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Delta".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "advanced_reactor".into(),
            name: "Advanced Reactor".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: Some("fusion_engineering".into()),
            maintenance: vec![],
        });
        engine.state.registry = Some(reg);
        engine
            .state
            .tech_state
            .researched
            .insert("fusion_engineering".into());

        let result = engine.apply(&Command::DeployStarterKit {
            colony_id,
            buildings: vec![("advanced_reactor".into(), 1)],
        });
        assert!(result.is_ok(), "expected success, got {result:?}");
    }

    #[test]
    fn deploy_starter_kit_rejects_second_call() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Founding".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        engine
            .apply(&Command::DeployStarterKit {
                colony_id,
                buildings: vec![("colony_hq".into(), 1)],
            })
            .unwrap();

        let result = engine.apply(&Command::DeployStarterKit {
            colony_id,
            buildings: vec![("habitat_dome".into(), 1)],
        });
        assert!(
            matches!(result, Err(EngineError::InvalidArgument(_))),
            "expected InvalidArgument on second deploy, got {result:?}"
        );

        let colony = engine
            .state
            .colonies
            .iter()
            .find(|c| c.id == colony_id)
            .unwrap();
        assert_eq!(
            colony.buildings.len(),
            1,
            "second call must not add buildings"
        );
    }

    #[test]
    fn deploy_starter_kit_rejects_over_budget_batch_atomically() {
        let mut engine = GameEngine::new();
        let events = engine
            .apply(&Command::FoundColony {
                name: "Founding".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // BASE_SLOT_CAPACITY is 5; request a batch totalling 6.
        let result = engine.apply(&Command::DeployStarterKit {
            colony_id,
            buildings: vec![
                ("colony_hq".into(), 1),
                ("habitat_dome".into(), 1),
                ("power_plant".into(), 1),
                ("factory".into(), 1),
                ("mine".into(), 1),
                ("extra".into(), 1),
            ],
        });
        assert!(
            matches!(result, Err(EngineError::SlotCapacityExceeded { .. })),
            "expected SlotCapacityExceeded, got {result:?}"
        );

        let colony = engine
            .state
            .colonies
            .iter()
            .find(|c| c.id == colony_id)
            .unwrap();
        assert!(
            colony.buildings.is_empty(),
            "over-budget batch must not partially deploy"
        );
        assert!(!colony.starter_kit_deployed);
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

    // ── SetActiveRecipe (issue #166) ──

    #[test]
    fn set_active_recipe_succeeds_and_persists_on_colony() {
        let mut engine = GameEngine::with_seed(0);
        let colony_id = setup_science_colony(&mut engine);

        let events = engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "research_lab".into(),
                recipe_id: "conduct_research".into(),
            })
            .unwrap();
        assert!(matches!(
            &events[0],
            Event::ActiveRecipeSet { building_type, recipe_id, .. }
                if building_type == "research_lab" && recipe_id == "conduct_research"
        ));

        let idx = engine.find_colony_index(colony_id).unwrap();
        assert_eq!(
            engine.state.colonies[idx]
                .active_recipes
                .get("research_lab"),
            Some(&"conduct_research".to_string())
        );
    }

    #[test]
    fn set_active_recipe_rejects_recipe_belonging_to_a_different_building() {
        let mut engine = GameEngine::with_seed(0);
        let colony_id = setup_science_colony(&mut engine);

        let err = engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "solar_array".into(),
                recipe_id: "conduct_research".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn set_active_recipe_rejects_unknown_recipe_id() {
        let mut engine = GameEngine::with_seed(0);
        let colony_id = setup_science_colony(&mut engine);

        let err = engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "research_lab".into(),
                recipe_id: "not_a_real_recipe".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn set_active_recipe_rejects_when_no_registry_loaded() {
        // Guards against silently accepting an unvalidated recipe_id/building_type
        // pair when self.state.registry is None (issue #166 regression).
        let mut engine = GameEngine::with_seed(0);
        let colony_id = found_colony_id(&mut engine, "No Registry", 10);

        let err = engine
            .apply(&Command::SetActiveRecipe {
                colony_id,
                building_type: "research_lab".into(),
                recipe_id: "conduct_research".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));

        let idx = engine.find_colony_index(colony_id).unwrap();
        assert!(engine.state.colonies[idx].active_recipes.is_empty());
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
            maintenance: vec![],
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
            maintenance: vec![],
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
            maintenance: vec![],
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
            concurrent: false,
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
    fn last_production_persisted_on_colony_after_advance_sol() {
        let mut engine = GameEngine::with_seed(42);
        let colony_id = setup_science_colony(&mut engine);

        engine.apply(&Command::AdvanceColonySol).unwrap();

        let idx = engine.find_colony_index(colony_id).unwrap();
        let last_production = &engine.state.colonies[idx].last_production;
        assert!(
            last_production.contains_key("research_lab"),
            "expected last_production to contain an entry for research_lab, got {last_production:?}"
        );
        let result = &last_production["research_lab"];
        assert_eq!(result.building_type, "research_lab");
        assert!(result.scale > 0.0);
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

    // ── M7: Per-colony interrupt config (issue #102) ─────────────────────────

    /// SetColonyInterruptConfig is handled by the engine and stored.
    #[test]
    fn set_colony_interrupt_config_stored() {
        use crate::interrupt::{InterruptConfig, InterruptSourceKind};

        let mut engine = GameEngine::with_seed(0);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Config Colony".into(),
                starting_population: 10,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Set to only StabilityCritical.
        let mut sources = std::collections::HashSet::new();
        sources.insert(InterruptSourceKind::StabilityCritical);
        engine
            .apply(&Command::SetColonyInterruptConfig {
                colony_id,
                config: InterruptConfig { sources },
            })
            .unwrap();

        let cfg = engine
            .state
            .interrupt_configs
            .get(&colony_id)
            .expect("config must be stored");
        assert!(cfg
            .sources
            .contains(&InterruptSourceKind::StabilityCritical));
        assert!(!cfg
            .sources
            .contains(&InterruptSourceKind::PredictiveWarning));
    }

    // ── Debug/testing commands (issue #232) ──────────────────────────────────

    #[test]
    fn debug_grant_colony_resources_lands_in_pool_and_emits_event() {
        let mut engine = GameEngine::with_seed(0);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Debug Colony".into(),
                starting_population: 10,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let before = engine.state.colonies[0].pool.amount("structural_ore");

        let events = engine
            .apply(&Command::DebugGrantColonyResources {
                colony_id,
                commodity_id: "structural_ore".into(),
                amount: 250.0,
            })
            .unwrap();

        let after = engine.state.colonies[0].pool.amount("structural_ore");
        assert!(
            after >= before + 249.9,
            "expected pool to gain ~250 structural_ore, before={before} after={after}"
        );

        match &events[0] {
            Event::DebugResourcesGranted {
                colony_id: evt_colony,
                commodity_id,
                amount,
            } => {
                assert_eq!(*evt_colony, colony_id);
                assert_eq!(commodity_id, "structural_ore");
                assert!(*amount > 0.0);
            }
            other => panic!("expected DebugResourcesGranted, got {other:?}"),
        }
    }

    #[test]
    fn debug_grant_colony_resources_rejects_unknown_colony() {
        let mut engine = GameEngine::with_seed(0);
        let bogus = uuid::Uuid::new_v4();
        let result = engine.apply(&Command::DebugGrantColonyResources {
            colony_id: bogus,
            commodity_id: "structural_ore".into(),
            amount: 100.0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn debug_grant_colony_resources_clamps_negative_amount_to_zero() {
        let mut engine = GameEngine::with_seed(0);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Debug Colony".into(),
                starting_population: 10,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        let before = engine.state.colonies[0].pool.amount("structural_ore");
        let events = engine
            .apply(&Command::DebugGrantColonyResources {
                colony_id,
                commodity_id: "structural_ore".into(),
                amount: -500.0,
            })
            .unwrap();
        let after = engine.state.colonies[0].pool.amount("structural_ore");

        assert!(
            (after - before).abs() < 1e-9,
            "negative amount must not drain the pool: before={before} after={after}"
        );
        match &events[0] {
            Event::DebugResourcesGranted { amount, .. } => {
                assert!((*amount).abs() < 1e-9, "granted amount should clamp to 0");
            }
            other => panic!("expected DebugResourcesGranted, got {other:?}"),
        }
    }

    // ── Outposts (issue #233) ─────────────────────────────────────────────────

    /// Establish a colony + a body for outpost tests to anchor to. Returns
    /// `(engine, colony_id, body_id)`.
    fn setup_colony_and_body(engine: &mut GameEngine) -> (ColonyId, system::BodyId) {
        let events = engine
            .apply(&Command::FoundColony {
                name: "Parent Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!("expected ColonyFounded")
        };
        let colony_id = *colony_id;

        let events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Mining Belt".into(),
                kind: system::BodyKind::AsteroidBelt,
                distance_au: 2.0,
            }))
            .unwrap();
        let body_id = match &events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            other => panic!("expected BodyAdded, got {other:?}"),
        };
        // Give the body a structural_ore deposit (issue #239) so outpost
        // mining tests built on this shared fixture keep working under
        // deposit gating — an asteroid belt plausibly has ore, and no test
        // here asserts on the body's exact deposit list.
        if let Some(body) = engine.state.system_state.node_map.bodies.get_mut(&body_id) {
            body.deposits
                .push(system::BodyDeposit::new("structural_ore", 1.0));
        }
        (colony_id, body_id)
    }

    fn registry_with_mining_outpost_building() -> crate::content::ContentRegistry {
        use crate::content::{
            BuildingCategory, BuildingDef, ContentRegistry, Ingredient, RecipeDef,
        };
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "mining_outpost".into(),
            name: "Mining Outpost".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "mine_structural_ore_outpost".into(),
            name: "Mine Structural Ore".into(),
            building: "mining_outpost".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "structural_ore".into(),
                quantity: 10.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        reg
    }

    #[test]
    fn establish_outpost_links_parent_colony_and_body() {
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);

        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Mining Camp Alpha".into(),
                colony_id,
                body_id: body_id.clone(),
            })
            .unwrap();
        let outpost_id = match &events[0] {
            Event::OutpostEstablished {
                outpost_id,
                colony_id: evt_colony,
                body_id: evt_body,
            } => {
                assert_eq!(*evt_colony, colony_id);
                assert_eq!(*evt_body, body_id);
                *outpost_id
            }
            other => panic!("expected OutpostEstablished, got {other:?}"),
        };

        let outpost = engine
            .state
            .outposts
            .iter()
            .find(|o| o.id == outpost_id)
            .expect("outpost must be stored");
        assert_eq!(outpost.parent_colony_id, colony_id);
        assert_eq!(outpost.body_id, body_id);
        assert_eq!(outpost.name, "Mining Camp Alpha");
    }

    #[test]
    fn establish_outpost_fails_for_unknown_colony() {
        let mut engine = GameEngine::new();
        let (_, body_id) = setup_colony_and_body(&mut engine);
        let bogus = uuid::Uuid::new_v4();
        let result = engine.apply(&Command::EstablishOutpost {
            name: "Ghost Camp".into(),
            colony_id: bogus,
            body_id,
        });
        assert!(result.is_err());
    }

    #[test]
    fn establish_outpost_fails_for_unknown_body() {
        let mut engine = GameEngine::new();
        let (colony_id, _) = setup_colony_and_body(&mut engine);
        let bogus_body = system::BodyId::new();
        let result = engine.apply(&Command::EstablishOutpost {
            name: "Ghost Camp".into(),
            colony_id,
            body_id: bogus_body,
        });
        assert!(result.is_err());
    }

    #[test]
    fn establish_outpost_within_range_succeeds_for_placed_colony() {
        let mut engine = GameEngine::new();
        let (colony_id, _) = setup_colony_and_body(&mut engine);

        let home_events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Homeworld".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 1.0,
            }))
            .unwrap();
        let home_body_id = match &home_events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            other => panic!("expected BodyAdded, got {other:?}"),
        };
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.colonies[idx].home_body_id = Some(home_body_id);

        // Nearby target: within BASE_OUTPOST_RANGE_AU (3.0) * propulsion_level (1).
        let target_events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Nearby Rock".into(),
                kind: system::BodyKind::AsteroidBelt,
                distance_au: 2.5,
            }))
            .unwrap();
        let target_body_id = match &target_events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            other => panic!("expected BodyAdded, got {other:?}"),
        };

        let result = engine.apply(&Command::EstablishOutpost {
            name: "Nearby Camp".into(),
            colony_id,
            body_id: target_body_id,
        });
        assert!(
            result.is_ok(),
            "outpost within range must be established, got {result:?}"
        );
    }

    #[test]
    fn establish_outpost_beyond_range_fails_for_placed_colony() {
        let mut engine = GameEngine::new();
        let (colony_id, _) = setup_colony_and_body(&mut engine);

        let home_events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Homeworld".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 1.0,
            }))
            .unwrap();
        let home_body_id = match &home_events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            other => panic!("expected BodyAdded, got {other:?}"),
        };
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.colonies[idx].home_body_id = Some(home_body_id);

        // Far target: beyond BASE_OUTPOST_RANGE_AU (3.0) * propulsion_level (1).
        let target_events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Distant Rock".into(),
                kind: system::BodyKind::AsteroidBelt,
                distance_au: 20.0,
            }))
            .unwrap();
        let target_body_id = match &target_events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            other => panic!("expected BodyAdded, got {other:?}"),
        };

        let result = engine.apply(&Command::EstablishOutpost {
            name: "Distant Camp".into(),
            colony_id,
            body_id: target_body_id,
        });
        assert!(
            matches!(result, Err(EngineError::OutpostOutOfRange { .. })),
            "expected OutpostOutOfRange, got {result:?}"
        );
    }

    #[test]
    fn establish_outpost_range_extending_tech_permits_a_previously_out_of_range_body() {
        let mut engine = GameEngine::new();
        let (colony_id, _) = setup_colony_and_body(&mut engine);

        let home_events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Homeworld".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 1.0,
            }))
            .unwrap();
        let home_body_id = match &home_events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            other => panic!("expected BodyAdded, got {other:?}"),
        };
        let idx = engine.find_colony_index(colony_id).unwrap();
        engine.state.colonies[idx].home_body_id = Some(home_body_id);

        let target_events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Distant Rock".into(),
                kind: system::BodyKind::AsteroidBelt,
                distance_au: 5.5,
            }))
            .unwrap();
        let target_body_id = match &target_events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            other => panic!("expected BodyAdded, got {other:?}"),
        };

        // Out of range before any tech bonus (distance 4.5 AU > base 3.0 AU).
        let before = engine.apply(&Command::EstablishOutpost {
            name: "Too Far".into(),
            colony_id,
            body_id: target_body_id.clone(),
        });
        assert!(
            matches!(before, Err(EngineError::OutpostOutOfRange { .. })),
            "expected OutpostOutOfRange before tech bonus, got {before:?}"
        );

        engine.state.outpost_range_bonus_au = 5.0;

        let after = engine.apply(&Command::EstablishOutpost {
            name: "Now Reachable".into(),
            colony_id,
            body_id: target_body_id,
        });
        assert!(
            after.is_ok(),
            "expected outpost establishment to succeed after range-extending tech, got {after:?}"
        );
    }

    #[test]
    fn queue_outpost_construction_fails_when_tech_not_researched() {
        use crate::content::{BuildingCategory, BuildingDef, ContentRegistry};

        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Gated Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!("expected OutpostEstablished")
        };
        let outpost_id = *outpost_id;

        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "advanced_outpost_module".into(),
            name: "Advanced Outpost Module".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: Some("advanced_outpost_tech".into()),
            maintenance: vec![],
        });
        engine.state.registry = Some(reg);

        let result = engine.apply(&Command::QueueOutpostConstruction {
            outpost_id,
            building_type: "advanced_outpost_module".into(),
            slot_cost: 1,
            labor_per_turn: 1,
            construction_cost: vec![],
            construction_turns: 1,
        });
        assert!(
            matches!(
                result,
                Err(EngineError::TechLocked { ref tech_id, .. }) if tech_id == "advanced_outpost_tech"
            ),
            "expected TechLocked, got {result:?}"
        );
    }

    #[test]
    fn queue_outpost_construction_succeeds_once_tech_researched() {
        use crate::content::{BuildingCategory, BuildingDef, ContentRegistry};

        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Gated Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!("expected OutpostEstablished")
        };
        let outpost_id = *outpost_id;

        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "advanced_outpost_module".into(),
            name: "Advanced Outpost Module".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: Some("advanced_outpost_tech".into()),
            maintenance: vec![],
        });
        engine.state.registry = Some(reg);
        engine
            .state
            .tech_state
            .researched
            .insert("advanced_outpost_tech".into());

        let result = engine.apply(&Command::QueueOutpostConstruction {
            outpost_id,
            building_type: "advanced_outpost_module".into(),
            slot_cost: 1,
            labor_per_turn: 1,
            construction_cost: vec![],
            construction_turns: 1,
        });
        assert!(result.is_ok(), "expected success, got {result:?}");
    }

    #[test]
    fn decommission_outpost_removes_it() {
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Temp Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;

        assert_eq!(engine.state.outposts.len(), 1);
        engine
            .apply(&Command::DecommissionOutpost { outpost_id })
            .unwrap();
        assert!(engine.state.outposts.is_empty());
    }

    #[test]
    fn decommission_outpost_fails_for_unknown_outpost() {
        let mut engine = GameEngine::new();
        let bogus = uuid::Uuid::new_v4();
        let result = engine.apply(&Command::DecommissionOutpost { outpost_id: bogus });
        assert!(result.is_err());
    }

    #[test]
    fn promote_outpost_to_colony_creates_independent_colony_with_carried_over_state() {
        let mut engine = GameEngine::new();
        let (parent_colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Frontier Camp".into(),
                colony_id: parent_colony_id,
                body_id: body_id.clone(),
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!("expected OutpostEstablished")
        };
        let outpost_id = *outpost_id;

        // Give the outpost some carried-over state to verify against.
        engine.state.outposts[0]
            .pool
            .deposit("structural_ore", 250.0);
        engine.state.outposts[0].slot_capacity = 7;
        let placed = colony::PlacedBuilding {
            id: uuid::Uuid::new_v4(),
            building_type: "mining_outpost".into(),
            slot_cost: 1,
        };
        engine.state.outposts[0].buildings.push(placed);

        let colonies_before = engine.state.colonies.len();
        let populations_before = engine.state.populations.len();

        let events = engine
            .apply(&Command::PromoteOutpostToColony {
                outpost_id,
                name: "New Haven".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::OutpostPromoted {
            outpost_id: evt_outpost,
            colony_id: new_colony_id,
            name: evt_name,
        } = &events[0]
        else {
            panic!("expected OutpostPromoted, got {:?}", events[0]);
        };
        assert_eq!(*evt_outpost, outpost_id);
        assert_eq!(evt_name, "New Haven");
        let new_colony_id = *new_colony_id;

        // Outpost is gone, colony was added.
        assert!(engine.state.outposts.is_empty());
        assert_eq!(engine.state.colonies.len(), colonies_before + 1);
        assert_eq!(engine.state.populations.len(), populations_before + 1);

        let idx = engine.find_colony_index(new_colony_id).unwrap();
        let colony = &engine.state.colonies[idx];
        assert_eq!(colony.name, "New Haven");
        assert_eq!(colony.home_body_id, Some(body_id));
        assert!((colony.pool.amount("structural_ore") - 250.0).abs() < 1e-6);
        assert_eq!(colony.buildings.len(), 1);
        assert_eq!(colony.slot_capacity, 7);

        // colonies/populations stay index-aligned.
        assert!((engine.state.populations[idx].count - 50.0).abs() < 1e-6);

        // The original parent colony is untouched and independent of the
        // promoted colony — no retained relationship either way.
        assert_ne!(new_colony_id, parent_colony_id);
        assert!(engine
            .state
            .colonies
            .iter()
            .any(|c| c.id == parent_colony_id));
    }

    #[test]
    fn promote_outpost_to_colony_carries_over_low_slot_capacity_up_to_colony_base() {
        // An outpost's slot capacity (2) is below BASE_SLOT_CAPACITY (5); the
        // promoted colony must not be stuck below the normal colony floor.
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Tiny Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!("expected OutpostEstablished")
        };
        let outpost_id = *outpost_id;
        assert_eq!(
            engine.state.outposts[0].slot_capacity,
            outpost::OUTPOST_BASE_SLOT_CAPACITY
        );

        let events = engine
            .apply(&Command::PromoteOutpostToColony {
                outpost_id,
                name: "Grown Up".into(),
                starting_population: 10,
            })
            .unwrap();
        let Event::OutpostPromoted { colony_id, .. } = &events[0] else {
            panic!("expected OutpostPromoted")
        };
        let idx = engine.find_colony_index(*colony_id).unwrap();
        assert_eq!(
            engine.state.colonies[idx].slot_capacity,
            colony::BASE_SLOT_CAPACITY,
            "promoted colony must not be stuck below the normal colony slot-capacity floor"
        );
    }

    #[test]
    fn promote_outpost_to_colony_fails_for_unknown_outpost() {
        let mut engine = GameEngine::new();
        let bogus = uuid::Uuid::new_v4();
        let result = engine.apply(&Command::PromoteOutpostToColony {
            outpost_id: bogus,
            name: "Ghost".into(),
            starting_population: 10,
        });
        assert!(
            matches!(result, Err(EngineError::OutpostNotFound(_))),
            "expected OutpostNotFound, got {result:?}"
        );
    }

    #[test]
    fn queue_outpost_construction_respects_slot_capacity() {
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;

        // OUTPOST_BASE_SLOT_CAPACITY is 2 — a slot_cost of 3 must be rejected.
        let result = engine.apply(&Command::QueueOutpostConstruction {
            outpost_id,
            building_type: "mining_outpost".into(),
            slot_cost: 3,
            labor_per_turn: 1,
            construction_cost: vec![],
            construction_turns: 1,
        });
        assert!(matches!(
            result,
            Err(EngineError::SlotCapacityExceeded { .. })
        ));
    }

    #[test]
    fn outpost_construction_completes_via_advance_colony_sol() {
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_mining_outpost_building());
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;

        engine
            .apply(&Command::QueueOutpostConstruction {
                outpost_id,
                building_type: "mining_outpost".into(),
                slot_cost: 1,
                labor_per_turn: 0,
                construction_cost: vec![],
                construction_turns: 1,
            })
            .unwrap();

        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        assert!(events.iter().any(
            |e| matches!(e, Event::OutpostBuildingConstructed { outpost_id: o, building_type } if *o == outpost_id && building_type == "mining_outpost")
        ));
        let outpost = &engine.state.outposts[0];
        assert_eq!(outpost.buildings.len(), 1);
        assert_eq!(outpost.buildings[0].building_type, "mining_outpost");
    }

    #[test]
    fn outpost_mining_building_produces_raw_material_into_pool() {
        // End-to-end: establish an outpost, build a mining building, advance
        // a sol, and confirm the raw material actually lands in the
        // outpost's pool — the core "single-resource mining/refining" use
        // case issue #233's Definition of Done calls out explicitly.
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_mining_outpost_building());
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Ore Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;

        engine
            .apply(&Command::QueueOutpostConstruction {
                outpost_id,
                building_type: "mining_outpost".into(),
                slot_cost: 1,
                labor_per_turn: 0,
                construction_cost: vec![],
                construction_turns: 1,
            })
            .unwrap();
        // Sol 1: construction completes, building placed — no production yet
        // this same sol (matches colony behavior: production runs against
        // buildings present at the *start* of the step).
        engine.apply(&Command::AdvanceColonySol).unwrap();
        // Sol 2: the now-operational building produces ore.
        engine.apply(&Command::AdvanceColonySol).unwrap();

        let outpost = &engine.state.outposts[0];
        assert!(
            outpost.pool.amount("structural_ore") > 0.0,
            "expected structural_ore in outpost pool, got {}",
            outpost.pool.amount("structural_ore")
        );
    }

    #[test]
    fn outpost_production_shortfall_reduces_output_without_power() {
        use crate::content::{
            BuildingCategory, BuildingDef, ContentRegistry, Ingredient, RecipeDef,
        };
        let mut reg = ContentRegistry::default();
        reg.insert_building(BuildingDef {
            id: "power_hungry_outpost".into(),
            name: "Power-hungry Outpost".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 1,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        reg.insert_recipe(RecipeDef {
            id: "mine_needs_power".into(),
            name: "Mine (needs power)".into(),
            building: "power_hungry_outpost".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "structural_ore".into(),
                quantity: 10.0,
            }],
            cycle_sols: 1,
            power_draw: 50.0, // no power source at this outpost — brownout.
            concurrent: false,
        });

        let mut engine = GameEngine::new();
        engine.state.registry = Some(reg);
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Dark Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;
        engine
            .apply(&Command::QueueOutpostConstruction {
                outpost_id,
                building_type: "power_hungry_outpost".into(),
                slot_cost: 1,
                labor_per_turn: 0,
                construction_cost: vec![],
                construction_turns: 1,
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap(); // construction completes
        let events = engine.apply(&Command::AdvanceColonySol).unwrap(); // production runs, no power

        assert!(
            events.iter().any(|e| matches!(
                e,
                Event::OutpostProductionShortfall { outpost_id: o, .. } if *o == outpost_id
            )),
            "expected an OutpostProductionShortfall event when no power is available"
        );
        let outpost = &engine.state.outposts[0];
        assert!(
            outpost.pool.amount("structural_ore") < 10.0,
            "output should be scaled down by the power shortfall, got {}",
            outpost.pool.amount("structural_ore")
        );
    }

    #[test]
    fn set_outpost_active_recipe_validates_building_match() {
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_mining_outpost_building());
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;

        // Correct building match succeeds.
        engine
            .apply(&Command::SetOutpostActiveRecipe {
                outpost_id,
                building_type: "mining_outpost".into(),
                recipe_id: "mine_structural_ore_outpost".into(),
            })
            .unwrap();
        assert_eq!(
            engine.state.outposts[0]
                .active_recipes
                .get("mining_outpost")
                .map(String::as_str),
            Some("mine_structural_ore_outpost")
        );

        // Mismatched building is rejected.
        let result = engine.apply(&Command::SetOutpostActiveRecipe {
            outpost_id,
            building_type: "smelter".into(),
            recipe_id: "mine_structural_ore_outpost".into(),
        });
        assert!(result.is_err());
    }

    /// Query::OutpostBuildingDetail returns the same recipe + last-run shape
    /// as Query::BuildingDetail, but scoped to an outpost (navigation
    /// rework #7 phase 4).
    #[test]
    fn query_outpost_building_detail_returns_recipe_and_last_run() {
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_mining_outpost_building());
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;
        engine
            .apply(&Command::QueueOutpostConstruction {
                outpost_id,
                building_type: "mining_outpost".into(),
                slot_cost: 1,
                labor_per_turn: 0,
                construction_cost: vec![],
                construction_turns: 1,
            })
            .unwrap();
        engine.apply(&Command::AdvanceColonySol).unwrap(); // construction completes
        engine.apply(&Command::AdvanceColonySol).unwrap(); // production runs

        let result = engine
            .query(&Query::OutpostBuildingDetail {
                outpost_id,
                building_type: "mining_outpost".into(),
            })
            .unwrap();
        match result {
            QueryResult::BuildingDetail(data) => {
                assert_eq!(data.building_type, "mining_outpost");
                let recipe = data.recipe.expect("mining_outpost should have a recipe");
                assert_eq!(recipe.recipe_id, "mine_structural_ore_outpost");
                let last_run = data.last_run.expect("mining_outpost should have run once");
                assert!(last_run.scale > 0.0);
            }
            other => panic!("expected BuildingDetail, got {other:?}"),
        }
    }

    /// Query::OutpostBuildingDetail errors on an unknown outpost id.
    #[test]
    fn query_outpost_building_detail_unknown_outpost_returns_error() {
        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry_with_mining_outpost_building());
        let result = engine.query(&Query::OutpostBuildingDetail {
            outpost_id: uuid::Uuid::new_v4(),
            building_type: "mining_outpost".into(),
        });
        assert!(matches!(result, Err(EngineError::OutpostNotFound(_))));
    }

    #[test]
    fn contribute_outpost_to_megaproject_withdraws_and_forwards() {
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Support Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;

        // Stock the outpost pool with more steel than the megaproject needs.
        engine.state.outposts[0].pool.deposit("steel", 800.0);

        let events = engine
            .apply(&Command::System(
                system::SystemCommand::RegisterMegaproject {
                    name: "Interstellar Expedition".into(),
                    kind: system::MegaprojectKind::InterstellarExpedition,
                    milestones: vec![system::MilestoneSpec {
                        label: "Hull Construction".into(),
                        resource_cost: vec![("steel".into(), 500.0)],
                        research_cost: 0.0,
                    }],
                },
            ))
            .unwrap();
        let project_id = match &events[0] {
            Event::System(system::SystemEvent::MegaprojectRegistered { project_id, .. }) => {
                project_id.clone()
            }
            other => panic!("expected MegaprojectRegistered, got {other:?}"),
        };

        let events = engine
            .apply(&Command::ContributeOutpostToMegaproject {
                outpost_id,
                project_id: project_id.clone(),
                resources: vec![("steel".into(), 500.0)],
                research: 0.0,
            })
            .unwrap();

        assert!(events.iter().any(|e| matches!(
            e,
            Event::OutpostContributedToMegaproject { outpost_id: o, .. } if *o == outpost_id
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            Event::System(system::SystemEvent::MilestoneCompleted { .. })
        )));
        // Withdrawn from the outpost pool.
        assert!((engine.state.outposts[0].pool.amount("steel") - 300.0).abs() < 1e-6);
    }

    #[test]
    fn contribute_outpost_to_megaproject_caps_withdrawal_at_pool_amount() {
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Poor Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;
        // Only 50 steel available, but request contributing 500.
        engine.state.outposts[0].pool.deposit("steel", 50.0);

        let events = engine
            .apply(&Command::System(
                system::SystemCommand::RegisterMegaproject {
                    name: "Interstellar Expedition".into(),
                    kind: system::MegaprojectKind::InterstellarExpedition,
                    milestones: vec![system::MilestoneSpec {
                        label: "Hull Construction".into(),
                        resource_cost: vec![("steel".into(), 500.0)],
                        research_cost: 0.0,
                    }],
                },
            ))
            .unwrap();
        let project_id = match &events[0] {
            Event::System(system::SystemEvent::MegaprojectRegistered { project_id, .. }) => {
                project_id.clone()
            }
            other => panic!("expected MegaprojectRegistered, got {other:?}"),
        };

        engine
            .apply(&Command::ContributeOutpostToMegaproject {
                outpost_id,
                project_id,
                resources: vec![("steel".into(), 500.0)],
                research: 0.0,
            })
            .unwrap();

        // Pool can't go negative — only what was actually held gets withdrawn.
        assert!((engine.state.outposts[0].pool.amount("steel")).abs() < 1e-6);
    }

    #[test]
    fn contribute_outpost_to_megaproject_does_not_withdraw_on_unknown_project() {
        // Regression: withdrawing before validating the megaproject exists
        // would silently destroy pool resources on a failed contribution
        // (e.g. a stale project_id) since ColonyPool::withdraw is
        // irreversible. The contribution must fail *before* touching the
        // pool.
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;
        engine.state.outposts[0].pool.deposit("steel", 100.0);

        let bogus_project = system::MegaprojectId::new();
        let result = engine.apply(&Command::ContributeOutpostToMegaproject {
            outpost_id,
            project_id: bogus_project,
            resources: vec![("steel".into(), 50.0)],
            research: 0.0,
        });
        assert!(result.is_err());
        assert!(
            (engine.state.outposts[0].pool.amount("steel") - 100.0).abs() < 1e-6,
            "pool must be untouched when the contribution is rejected, got {}",
            engine.state.outposts[0].pool.amount("steel")
        );
    }

    #[test]
    fn contribute_outpost_to_megaproject_does_not_withdraw_on_completed_project() {
        let mut engine = GameEngine::new();
        let (colony_id, body_id) = setup_colony_and_body(&mut engine);
        let events = engine
            .apply(&Command::EstablishOutpost {
                name: "Camp".into(),
                colony_id,
                body_id,
            })
            .unwrap();
        let Event::OutpostEstablished { outpost_id, .. } = &events[0] else {
            panic!()
        };
        let outpost_id = *outpost_id;
        engine.state.outposts[0].pool.deposit("steel", 500.0);

        let events = engine
            .apply(&Command::System(
                system::SystemCommand::RegisterMegaproject {
                    name: "Interstellar Expedition".into(),
                    kind: system::MegaprojectKind::InterstellarExpedition,
                    milestones: vec![system::MilestoneSpec {
                        label: "Hull Construction".into(),
                        resource_cost: vec![("steel".into(), 500.0)],
                        research_cost: 0.0,
                    }],
                },
            ))
            .unwrap();
        let project_id = match &events[0] {
            Event::System(system::SystemEvent::MegaprojectRegistered { project_id, .. }) => {
                project_id.clone()
            }
            other => panic!("expected MegaprojectRegistered, got {other:?}"),
        };
        // Complete the (only) milestone.
        engine
            .apply(&Command::ContributeOutpostToMegaproject {
                outpost_id,
                project_id: project_id.clone(),
                resources: vec![("steel".into(), 500.0)],
                research: 0.0,
            })
            .unwrap();
        engine.state.outposts[0].pool.deposit("steel", 100.0);

        // Second contribution to the now-completed project must fail and
        // must not touch the pool.
        let result = engine.apply(&Command::ContributeOutpostToMegaproject {
            outpost_id,
            project_id,
            resources: vec![("steel".into(), 50.0)],
            research: 0.0,
        });
        assert!(result.is_err());
        assert!(
            (engine.state.outposts[0].pool.amount("steel") - 100.0).abs() < 1e-6,
            "pool must be untouched when the project is already complete, got {}",
            engine.state.outposts[0].pool.amount("steel")
        );
    }

    /// A colony with an empty mask never causes an interrupt halt.
    #[test]
    fn colony_with_empty_mask_never_halts() {
        use crate::interrupt::{AdvanceResult, InterruptConfig, Tier};

        let mut engine = GameEngine::with_seed(1);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Silent Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Pre-load a steep declining stability trajectory that would normally
        // fire an Urgent PredictiveWarning.
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

        // Disable all interrupts for this colony.
        engine
            .apply(&Command::SetColonyInterruptConfig {
                colony_id,
                config: InterruptConfig::silent(),
            })
            .unwrap();

        let result = engine.advance_until_interrupted(20, Tier::Urgent).unwrap();
        assert!(
            matches!(result, AdvanceResult::Completed { .. }),
            "colony with empty mask must never cause halt; got {result:?}"
        );
    }

    /// A colony with only StabilityCritical only halts on that source.
    #[test]
    fn colony_stability_critical_only_config_filters_predictive_warning() {
        use crate::interrupt::{
            AdvanceResult, InterruptConfig, InterruptSource, InterruptSourceKind, Tier,
        };

        let mut engine = GameEngine::with_seed(2);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Critical-Only Colony".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let colony_id = *colony_id;

        // Steep declining trajectory → would fire PredictiveWarning (Urgent).
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

        // Only allow StabilityCritical interrupts.
        let mut sources = std::collections::HashSet::new();
        sources.insert(InterruptSourceKind::StabilityCritical);
        engine
            .apply(&Command::SetColonyInterruptConfig {
                colony_id,
                config: InterruptConfig { sources },
            })
            .unwrap();

        // PredictiveWarning is suppressed, so advance should complete without halt
        // (unless a StabilityCritical fires, which requires stability ≤ the floor).
        let result = engine.advance_until_interrupted(5, Tier::Urgent).unwrap();
        match result {
            AdvanceResult::Halted { interrupt, .. } => {
                // If it did halt, it must be StabilityCritical, not PredictiveWarning.
                assert!(
                    matches!(interrupt.source, InterruptSource::StabilityCritical(_)),
                    "only StabilityCritical should be able to halt; got {:?}",
                    interrupt.source
                );
            }
            AdvanceResult::Completed { .. } => {
                // Completed is also valid — PredictiveWarning was filtered.
            }
        }
    }

    /// Disable all interrupts for colony A; verify it never halts on stability drop.
    #[test]
    fn disable_all_interrupts_for_colony_a_never_halts_on_stability() {
        use crate::interrupt::{AdvanceResult, InterruptConfig, Tier};

        let mut engine = GameEngine::with_seed(3);
        let events = engine
            .apply(&Command::FoundColony {
                name: "Colony A".into(),
                starting_population: 100,
            })
            .unwrap();
        let Event::ColonyFounded {
            colony_id: colony_a,
            ..
        } = &events[0]
        else {
            panic!()
        };
        let colony_a = *colony_a;

        // Pre-load steep declining stability for colony A.
        let tracker = engine.state.stability_trackers.entry(colony_a).or_default();
        for s in [1.0f32, 0.7, 0.4, 0.2, 0.1] {
            tracker.push(s);
        }
        let idx = engine.find_colony_index(colony_a).unwrap();
        engine.state.populations[idx].stability = 0.1;

        // Disable all interrupts for colony A.
        engine
            .apply(&Command::SetColonyInterruptConfig {
                colony_id: colony_a,
                config: InterruptConfig::silent(),
            })
            .unwrap();

        let result = engine.advance_until_interrupted(10, Tier::Urgent).unwrap();
        assert!(
            matches!(result, AdvanceResult::Completed { .. }),
            "colony A with all interrupts disabled must never halt on stability drop; got {result:?}"
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

    /// Query::BuildingDetail returns recipe + last-run data after production runs.
    #[test]
    fn query_building_detail_returns_recipe_and_last_run() {
        let mut engine = GameEngine::with_seed(42);
        let colony_id = setup_science_colony(&mut engine);

        engine.apply(&Command::AdvanceColonySol).unwrap();

        let result = engine
            .query(&Query::BuildingDetail {
                colony_id,
                building_type: "research_lab".into(),
            })
            .unwrap();
        match result {
            QueryResult::BuildingDetail(data) => {
                assert_eq!(data.building_type, "research_lab");
                let recipe = data.recipe.expect("research_lab should have a recipe");
                assert!(!recipe.outputs.is_empty());
                let last_run = data.last_run.expect("research_lab should have run once");
                assert!(last_run.scale > 0.0);
            }
            other => panic!("expected BuildingDetail, got {other:?}"),
        }
    }

    /// Query::BuildingDetail's `available_recipes` is empty for a
    /// single-recipe building (research_lab) and lists every recipe,
    /// deterministically sorted by id, for a multi-recipe one (issue #166).
    #[test]
    fn query_building_detail_available_recipes_reflects_recipe_count() {
        use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};

        let mut engine = GameEngine::with_seed(0);
        let colony_id = setup_science_colony(&mut engine);

        // Single-recipe building: available_recipes stays empty.
        let result = engine
            .query(&Query::BuildingDetail {
                colony_id,
                building_type: "research_lab".into(),
            })
            .unwrap();
        let QueryResult::BuildingDetail(data) = result else {
            panic!()
        };
        assert!(data.available_recipes.is_empty());
        assert_eq!(data.recipe.unwrap().recipe_id, "conduct_research");

        // Add a second building with two recipes to the same registry.
        let mut registry = engine.state.registry.clone().unwrap();
        registry.insert_building(BuildingDef {
            id: "refinery".into(),
            name: "Refinery".into(),
            description: String::new(),
            category: BuildingCategory::Processing,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        registry.insert_recipe(RecipeDef {
            id: "refine_b".into(),
            name: "Refine B".into(),
            building: "refinery".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "gadget".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        registry.insert_recipe(RecipeDef {
            id: "refine_a".into(),
            name: "Refine A".into(),
            building: "refinery".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "alloy".into(),
                quantity: 1.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        engine.state.registry = Some(registry);

        let result = engine
            .query(&Query::BuildingDetail {
                colony_id,
                building_type: "refinery".into(),
            })
            .unwrap();
        let QueryResult::BuildingDetail(data) = result else {
            panic!()
        };
        let ids: Vec<&str> = data
            .available_recipes
            .iter()
            .map(|r| r.recipe_id.as_str())
            .collect();
        assert_eq!(ids, vec!["refine_a", "refine_b"], "must be sorted by id");
        // No active_recipes entry set yet — falls back to the sorted-first default.
        assert_eq!(data.recipe.unwrap().recipe_id, "refine_a");
    }

    /// A building with only [`content::types::RecipeDef::concurrent`] recipes
    /// (e.g. `colony_hq`) has no pick-one `recipe` and must not surface those
    /// always-on recipes as if they were mutually-exclusive choices in
    /// `available_recipes` — they belong in `concurrent_recipes` instead
    /// (issue #272 follow-up: the `available_recipes` filter previously
    /// missed `!r.concurrent`, so a concurrent-only building's 3 recipes
    /// wrongly populated a working-looking recipe-switcher in the UI).
    #[test]
    fn query_building_detail_concurrent_only_building_has_no_pick_one_available_recipes() {
        use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};

        let mut engine = GameEngine::with_seed(0);
        let colony_id = setup_science_colony(&mut engine);

        let mut registry = engine.state.registry.clone().unwrap();
        registry.insert_building(BuildingDef {
            id: "hq".into(),
            name: "HQ".into(),
            description: String::new(),
            category: BuildingCategory::Services,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        for (id, commodity) in [("hq_power", "power"), ("hq_water", "water")] {
            registry.insert_recipe(RecipeDef {
                id: id.into(),
                name: id.into(),
                building: "hq".into(),
                inputs: vec![],
                outputs: vec![Ingredient {
                    id: commodity.into(),
                    quantity: 1.0,
                }],
                cycle_sols: 1,
                power_draw: 0.0,
                concurrent: true,
            });
        }
        engine.state.registry = Some(registry);

        let result = engine
            .query(&Query::BuildingDetail {
                colony_id,
                building_type: "hq".into(),
            })
            .unwrap();
        let QueryResult::BuildingDetail(data) = result else {
            panic!()
        };
        assert!(data.recipe.is_none());
        assert!(
            data.available_recipes.is_empty(),
            "concurrent recipes must not appear in the pick-one available_recipes list"
        );
        let concurrent_ids: Vec<&str> = data
            .concurrent_recipes
            .iter()
            .map(|r| r.recipe_id.as_str())
            .collect();
        assert_eq!(concurrent_ids, vec!["hq_power", "hq_water"]);
    }

    /// Query::BuildingDetail errors on an unknown building type.
    #[test]
    fn query_building_detail_unknown_building_returns_error() {
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
        engine.state.registry = Some(research_registry());

        let result = engine.query(&Query::BuildingDetail {
            colony_id,
            building_type: "not_a_real_building".into(),
        });
        assert!(matches!(result, Err(EngineError::InvalidArgument(_))));
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
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
            })
            .unwrap();
        engine
            .apply(&Command::FoundColonyAtSite {
                name: "Beta".into(),
                starting_population: 20,
                site_id: site_b,
                focus: None,
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
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
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
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
                ..TechDef::default()
            },
            TechDef {
                id: "beta".into(),
                display_name: "Beta".into(),
                prerequisites: vec!["alpha".into()],
                research_cost: 20.0,
                effects: vec![TechEffect::UnlockCapability {
                    capability_id: "cap_beta".into(),
                }],
                ..TechDef::default()
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
    fn enqueue_research_rejects_unmet_prerequisites() {
        let mut engine = make_tech_engine();
        // beta requires alpha, which is not yet researched
        let err = engine
            .apply(&Command::EnqueueResearch {
                tech_id: "beta".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
        assert!(engine.state.tech_state.research_queue.is_empty());
    }

    #[test]
    fn enqueue_research_rejects_tech_already_active() {
        let mut engine = make_tech_engine();
        engine
            .apply(&Command::ResearchTech {
                tech_id: "alpha".into(),
            })
            .unwrap();
        let err = engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn enqueue_research_rejects_tech_already_queued() {
        let mut engine = make_tech_engine();
        engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap();
        let err = engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
        assert_eq!(engine.state.tech_state.research_queue.len(), 1);
    }

    #[test]
    fn enqueue_research_rejects_tech_already_researched() {
        let mut engine = make_tech_engine();
        engine.state.tech_state.researched.insert("alpha".into());
        let err = engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::InvalidArgument(_)));
    }

    #[test]
    fn cancel_research_clears_queue_and_project() {
        let mut engine = make_tech_engine();
        engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap();
        // `ResearchTech` dedupes `alpha` out of the queue as it promotes it
        // to `current_project` (see `research_tech_dedupes_queued_copy`), so
        // by the time `CancelResearch` runs, the queue is already empty —
        // this test just confirms cancel also clears an empty queue cleanly.
        engine
            .apply(&Command::ResearchTech {
                tech_id: "alpha".into(),
            })
            .unwrap();
        let events = engine.apply(&Command::CancelResearch).unwrap();
        assert!(events.iter().any(|e| matches!(e, Event::ResearchCancelled)));
        assert!(engine.state.tech_state.current_project.is_none());
        assert!(engine.state.tech_state.research_queue.is_empty());
    }

    #[test]
    fn research_tech_dedupes_queued_copy() {
        let mut engine = make_tech_engine();
        engine
            .apply(&Command::EnqueueResearch {
                tech_id: "alpha".into(),
            })
            .unwrap();
        assert_eq!(engine.state.tech_state.research_queue.len(), 1);
        engine
            .apply(&Command::ResearchTech {
                tech_id: "alpha".into(),
            })
            .unwrap();
        assert_eq!(
            engine.state.tech_state.current_project.as_deref(),
            Some("alpha")
        );
        assert!(
            engine.state.tech_state.research_queue.is_empty(),
            "promoting a queued tech directly via ResearchTech must remove \
             it from the queue, or a later drain would complete (and \
             reapply the effects of) the same tech a second time"
        );
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
            supplies_id: None,
            supply_overrides: None,
            body_id: None,
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
            supplies_id: None,
            supply_overrides: None,
            body_id: None,
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
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
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

    /// Done-when (issue #239): a colony founded at the specific hex where
    /// #232's coverage guarantee force-placed a `structural_ore` deposit
    /// can actually mine it under #239's new deposit gating — proving the
    /// coverage guarantee and the gate coexist rather than starving every
    /// founding colony.
    #[test]
    fn founding_site_with_guaranteed_deposit_sustains_gated_extraction() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet {
                seed: 4242,
                radius: 6,
            })
            .unwrap();

        // Find a habitable hex that actually has a structural_ore deposit
        // (guaranteed to exist *somewhere* on the map by #232, though not
        // necessarily at the single best landing site).
        let pm = engine.state.planet_map.as_ref().unwrap();
        let (site_id, coord) = pm
            .sites
            .iter()
            .find_map(|(&id, &coord)| {
                let cell = pm.cells.get(&coord)?;
                if cell.is_habitable()
                    && cell
                        .deposits
                        .iter()
                        .any(|d| d.commodity_id == "structural_ore")
                {
                    Some((id, coord))
                } else {
                    None
                }
            })
            .expect(
                "a habitable hex with a structural_ore deposit must exist \
                 (issue #232's coverage guarantee)",
            );
        let _ = coord;

        let mut registry = content::ContentRegistry::default();
        registry.insert_building(content::types::BuildingDef {
            id: "structural_mine".into(),
            name: "Structural Mine".into(),
            description: String::new(),
            category: content::types::BuildingCategory::Extraction,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        registry.insert_recipe(content::types::RecipeDef {
            id: "mine_structural_ore".into(),
            name: "Mine Structural Ore".into(),
            building: "structural_mine".into(),
            inputs: vec![],
            outputs: vec![content::types::Ingredient {
                id: "structural_ore".into(),
                quantity: 10.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        engine.state.registry = Some(registry);

        engine
            .apply(&Command::FoundColonyAtSite {
                name: "Ore Landing".into(),
                starting_population: 100,
                site_id,
                focus: None,
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
            })
            .unwrap();

        let idx = 0;
        engine.state.colonies[idx]
            .buildings
            .push(colony::PlacedBuilding::new("structural_mine", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();

        assert!(
            engine.state.colonies[idx].pool.amount("structural_ore") > 0.0,
            "colony founded on its own guaranteed structural_ore deposit \
             must be able to mine it under deposit gating"
        );
    }

    /// Done-when (issue #239): a colony founded at a habitable hex with NO
    /// structural_ore deposit gets zero output from a structural_mine —
    /// deposit gating actually blocks extraction when nothing is there,
    /// not just a decorative flag.
    #[test]
    fn founding_site_without_matching_deposit_blocks_gated_extraction() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet {
                seed: 4242,
                radius: 6,
            })
            .unwrap();

        let pm = engine.state.planet_map.as_ref().unwrap();
        let (site_id, _) = pm
            .sites
            .iter()
            .find_map(|(&id, &coord)| {
                let cell = pm.cells.get(&coord)?;
                if cell.is_habitable()
                    && !cell
                        .deposits
                        .iter()
                        .any(|d| d.commodity_id == "structural_ore")
                {
                    Some((id, coord))
                } else {
                    None
                }
            })
            .expect("a habitable hex without a structural_ore deposit must exist");

        let mut registry = content::ContentRegistry::default();
        registry.insert_building(content::types::BuildingDef {
            id: "structural_mine".into(),
            name: "Structural Mine".into(),
            description: String::new(),
            category: content::types::BuildingCategory::Extraction,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 0,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
        });
        registry.insert_recipe(content::types::RecipeDef {
            id: "mine_structural_ore".into(),
            name: "Mine Structural Ore".into(),
            building: "structural_mine".into(),
            inputs: vec![],
            outputs: vec![content::types::Ingredient {
                id: "structural_ore".into(),
                quantity: 10.0,
            }],
            cycle_sols: 1,
            power_draw: 0.0,
            concurrent: false,
        });
        engine.state.registry = Some(registry);

        engine
            .apply(&Command::FoundColonyAtSite {
                name: "Barren Landing".into(),
                starting_population: 100,
                site_id,
                focus: None,
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
            })
            .unwrap();

        let idx = 0;
        engine.state.colonies[idx]
            .buildings
            .push(colony::PlacedBuilding::new("structural_mine", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();

        assert!(
            engine.state.colonies[idx]
                .pool
                .amount("structural_ore")
                .abs()
                < 1e-9,
            "colony with no matching deposit must get zero structural_ore \
             from a structural_mine under deposit gating"
        );
        assert_eq!(
            engine.state.colonies[idx]
                .last_production
                .get("structural_mine")
                .unwrap()
                .scale,
            0.0
        );
    }

    /// Shared setup for the habitability-gate tests (issue #183): seed a
    /// planet map and add a body with the given attributes. Returns
    /// `(engine, site_id, body_id)` for a `FoundColonyAtSite` call.
    fn setup_engine_with_body_and_site(
        atmosphere_density: system::AtmosphereDensity,
        atmosphere_hazard: system::AtmosphereHazard,
        temperature: system::TemperatureBand,
        gravity_g: f32,
        radiation: system::RadiationLevel,
    ) -> (GameEngine, SiteId, system::BodyId) {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet {
                seed: 42,
                radius: 3,
            })
            .unwrap();
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
        let _ = pm;

        let events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: "Target".into(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 1.0,
            }))
            .unwrap();
        let body_id = match &events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        engine
            .apply(&Command::System(system::SystemCommand::SetBodyAttributes {
                body_id: body_id.clone(),
                atmosphere_density,
                atmosphere_hazard,
                temperature,
                gravity_g,
                radiation,
                subtype: system::PlanetarySubtype::Unclassified,
                tidally_locked: false,
                axial_tilt_deg: 23.5,
                rotation_period_hours: 24.0,
                moon_count: 0,
            }))
            .unwrap();

        (engine, site_id, body_id)
    }

    #[test]
    fn found_colony_at_site_rejects_low_habitability_body() {
        let (mut engine, site_id, body_id) = setup_engine_with_body_and_site(
            system::AtmosphereDensity::Dense,
            system::AtmosphereHazard::Toxic,
            system::TemperatureBand::Extreme,
            0.0,
            system::RadiationLevel::High,
        );
        let body = engine
            .state
            .system_state
            .node_map
            .bodies
            .get(&body_id)
            .unwrap();
        assert_eq!(
            body.habitability(),
            0,
            "fixture must be a 0-habitability body"
        );

        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Doomed".into(),
            starting_population: 100,
            site_id,
            focus: None,
            supplies_id: None,
            supply_overrides: None,
            body_id: Some(body_id),
        });
        assert!(
            matches!(
                result,
                Err(EngineError::HabitabilityBelowThreshold { score: 0, .. })
            ),
            "expected HabitabilityBelowThreshold, got {result:?}"
        );
        // No mutation should have happened — the gate fires before any state change.
        assert_eq!(engine.state.colonies.len(), 0);
    }

    #[test]
    fn found_colony_at_site_allows_habitable_body_and_auto_links_it() {
        let (mut engine, site_id, body_id) = setup_engine_with_body_and_site(
            system::AtmosphereDensity::Breathable,
            system::AtmosphereHazard::None,
            system::TemperatureBand::Temperate,
            1.0,
            system::RadiationLevel::Low,
        );
        let expected_modifier = engine
            .state
            .system_state
            .node_map
            .bodies
            .get(&body_id)
            .unwrap()
            .habitability_modifier();

        let events = engine
            .apply(&Command::FoundColonyAtSite {
                name: "Eden".into(),
                starting_population: 100,
                site_id,
                focus: None,
                supplies_id: None,
                supply_overrides: None,
                body_id: Some(body_id.clone()),
            })
            .unwrap();

        assert!(events
            .iter()
            .any(|e| matches!(e, Event::ColonyHomeBodySet { .. })));
        assert_eq!(engine.state.colonies.len(), 1);
        let colony = &engine.state.colonies[0];
        assert_eq!(colony.home_body_id, Some(body_id));
        assert!((colony.habitability_modifier - expected_modifier).abs() < 1e-4);
    }

    #[test]
    fn found_colony_at_site_allows_low_habitability_with_capability_unlocked() {
        let (mut engine, site_id, body_id) = setup_engine_with_body_and_site(
            system::AtmosphereDensity::Dense,
            system::AtmosphereHazard::Toxic,
            system::TemperatureBand::Extreme,
            0.0,
            system::RadiationLevel::High,
        );
        engine
            .state
            .unlocked_capabilities
            .insert(system::HARSH_WORLD_CAPABILITY_ID.to_string());

        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Grit City".into(),
            starting_population: 100,
            site_id,
            focus: None,
            supplies_id: None,
            supply_overrides: None,
            body_id: Some(body_id),
        });
        assert!(
            result.is_ok(),
            "harsh-world capability should override the gate: {result:?}"
        );
        assert_eq!(engine.state.colonies.len(), 1);
    }

    #[test]
    fn found_colony_at_site_rejects_unknown_body_id() {
        let (mut engine, site_id, _) = setup_engine_with_body_and_site(
            system::AtmosphereDensity::Breathable,
            system::AtmosphereHazard::None,
            system::TemperatureBand::Temperate,
            1.0,
            system::RadiationLevel::Low,
        );
        let bogus_body_id = system::BodyId::new();

        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Ghost".into(),
            starting_population: 100,
            site_id,
            focus: None,
            supplies_id: None,
            supply_overrides: None,
            body_id: Some(bogus_body_id),
        });
        assert!(matches!(result, Err(EngineError::InvalidArgument(_))));
        assert_eq!(engine.state.colonies.len(), 0);
    }

    #[test]
    fn found_colony_at_site_without_body_id_skips_gate() {
        // body_id: None must behave exactly as it did before #183 — no gate,
        // no auto-link, no ColonyHomeBodySet event.
        let (mut engine, site_id, _) = setup_engine_with_body_and_site(
            system::AtmosphereDensity::Dense,
            system::AtmosphereHazard::Toxic,
            system::TemperatureBand::Extreme,
            0.0,
            system::RadiationLevel::High,
        );
        let events = engine
            .apply(&Command::FoundColonyAtSite {
                name: "Unlinked".into(),
                starting_population: 100,
                site_id,
                focus: None,
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
            })
            .unwrap();
        assert!(!events
            .iter()
            .any(|e| matches!(e, Event::ColonyHomeBodySet { .. })));
        assert_eq!(engine.state.colonies[0].home_body_id, None);
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
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
            })
            .unwrap();

        // Second colony at the same site must fail.
        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Second".into(),
            starting_population: 100,
            site_id,
            focus: None,
            supplies_id: None,
            supply_overrides: None,
            body_id: None,
        });
        assert!(
            matches!(result, Err(EngineError::SiteOccupied)),
            "expected SiteOccupied, got {result:?}"
        );
    }

    #[test]
    fn found_colony_at_site_seeds_pool_from_supply_package() {
        use crate::content::loader::PackLoader;

        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "\
- id: water
  name: Water
  category: consumable
  base_value: 1.0
- id: food_ration
  name: Food Ration
  category: consumable
  base_value: 1.0
";
        let supplies_yaml = "\
- id: standard
  name: Standard
  commodities:
    - id: water
      quantity: 400.0
    - id: food_ration
      quantity: 300.0
";
        let files: Vec<(&str, &str)> = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("supplies.yaml", supplies_yaml),
        ];
        let registry = PackLoader::load(&files).unwrap();

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        engine
            .apply(&Command::SeedPlanet { seed: 7, radius: 3 })
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
                name: "Alpha".into(),
                starting_population: 200,
                site_id,
                focus: None,
                supplies_id: Some("standard".into()),
                supply_overrides: None,
                body_id: None,
            })
            .unwrap();

        // starting_population 200 → scale 2.0 → 400 * 2 = 800 water, 300 * 2 = 600 food.
        let colony = &engine.state.colonies[0];
        assert!(
            (colony.pool.amount("water") - 800.0).abs() < 0.001,
            "expected 800 water, got {}",
            colony.pool.amount("water")
        );
        assert!(
            (colony.pool.amount("food_ration") - 600.0).abs() < 0.001,
            "expected 600 food_ration, got {}",
            colony.pool.amount("food_ration")
        );
    }

    /// Issue #167: explicit `supply_overrides` deposit the exact absolute
    /// amounts sent, bypassing the per-100-colonist package scaling — this is
    /// what lets the wizard's per-commodity spinners produce arbitrary
    /// player-tweaked quantities rather than only fixed package tiers.
    #[test]
    fn found_colony_at_site_seeds_pool_from_supply_overrides() {
        use crate::content::loader::PackLoader;

        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "\
- id: water
  name: Water
  category: consumable
  base_value: 1.0
- id: food_ration
  name: Food Ration
  category: consumable
  base_value: 1.0
";
        let supplies_yaml = "\
- id: standard
  name: Standard
  commodities:
    - id: water
      quantity: 400.0
    - id: food_ration
      quantity: 300.0
";
        let files: Vec<(&str, &str)> = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("supplies.yaml", supplies_yaml),
        ];
        let registry = PackLoader::load(&files).unwrap();

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        engine
            .apply(&Command::SeedPlanet { seed: 7, radius: 3 })
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

        // Player tweaked the "standard" preset's spinners to arbitrary
        // absolute amounts before founding.
        engine
            .apply(&Command::FoundColonyAtSite {
                name: "Alpha".into(),
                starting_population: 200,
                site_id,
                focus: None,
                supplies_id: Some("standard".into()),
                supply_overrides: Some(vec![("water".into(), 500.0), ("food_ration".into(), 50.0)]),
                body_id: None,
            })
            .unwrap();

        let colony = &engine.state.colonies[0];
        assert!(
            (colony.pool.amount("water") - 500.0).abs() < 0.001,
            "expected exact override 500 water, got {}",
            colony.pool.amount("water")
        );
        assert!(
            (colony.pool.amount("food_ration") - 50.0).abs() < 0.001,
            "expected exact override 50 food_ration, got {}",
            colony.pool.amount("food_ration")
        );
    }

    /// Issue #167: a `supply_overrides` amount beyond the cargo-capacity cap
    /// (`MAX_SUPPLY_OVERRIDE_MULTIPLE` × largest authored per-100 amount,
    /// scaled by population) is rejected before any mutation.
    #[test]
    fn found_colony_at_site_rejects_supply_override_over_cap() {
        use crate::content::loader::PackLoader;

        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "\
- id: water
  name: Water
  category: consumable
  base_value: 1.0
";
        let supplies_yaml = "\
- id: standard
  name: Standard
  commodities:
    - id: water
      quantity: 400.0
";
        let files: Vec<(&str, &str)> = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("supplies.yaml", supplies_yaml),
        ];
        let registry = PackLoader::load(&files).unwrap();

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        engine
            .apply(&Command::SeedPlanet { seed: 7, radius: 3 })
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

        // Cap at pop 100 is 400 * 3.0 * 1.0 = 1200; 1201 must be rejected.
        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Alpha".into(),
            starting_population: 100,
            site_id,
            focus: None,
            supplies_id: None,
            supply_overrides: Some(vec![("water".into(), 1201.0)]),
            body_id: None,
        });
        assert!(
            matches!(result, Err(EngineError::InvalidArgument(_))),
            "expected InvalidArgument for over-cap override, got {result:?}"
        );
        // No colony should have been created — rejection happens before mutation.
        assert!(engine.state.colonies.is_empty());
    }

    /// Issue #167: `supply_overrides` referencing a commodity id absent from
    /// every authored `SupplyPackage` is rejected rather than silently
    /// depositing an unbounded/unknown commodity.
    #[test]
    fn found_colony_at_site_rejects_unknown_supply_override_commodity() {
        use crate::content::loader::PackLoader;

        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "\
- id: water
  name: Water
  category: consumable
  base_value: 1.0
- id: exotic_gas
  name: Exotic Gas
  category: consumable
  base_value: 1.0
";
        let supplies_yaml = "\
- id: standard
  name: Standard
  commodities:
    - id: water
      quantity: 400.0
";
        let files: Vec<(&str, &str)> = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
            ("supplies.yaml", supplies_yaml),
        ];
        let registry = PackLoader::load(&files).unwrap();

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        engine
            .apply(&Command::SeedPlanet { seed: 7, radius: 3 })
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

        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Alpha".into(),
            starting_population: 100,
            site_id,
            focus: None,
            supplies_id: None,
            supply_overrides: Some(vec![("exotic_gas".into(), 10.0)]),
            body_id: None,
        });
        assert!(
            matches!(result, Err(EngineError::InvalidArgument(_))),
            "expected InvalidArgument for unknown supply-override commodity, got {result:?}"
        );
        assert!(engine.state.colonies.is_empty());
    }

    #[test]
    fn found_colony_at_site_rejects_unknown_supplies_id() {
        use crate::content::loader::PackLoader;

        let pack_yaml = "id: t\nname: T\nversion: '0.1.0'\n";
        let commodities_yaml = "\
- id: water
  name: Water
  category: consumable
  base_value: 1.0
";
        let files: Vec<(&str, &str)> = vec![
            ("pack.yaml", pack_yaml),
            ("commodities.yaml", commodities_yaml),
        ];
        let registry = PackLoader::load(&files).unwrap();

        let mut engine = GameEngine::new();
        engine.state.registry = Some(registry);
        engine
            .apply(&Command::SeedPlanet { seed: 8, radius: 3 })
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

        let result = engine.apply(&Command::FoundColonyAtSite {
            name: "Alpha".into(),
            starting_population: 100,
            site_id,
            focus: None,
            supplies_id: Some("nonexistent".into()),
            supply_overrides: None,
            body_id: None,
        });
        assert!(
            matches!(result, Err(EngineError::InvalidArgument(_))),
            "expected InvalidArgument, got {result:?}"
        );
        // Colony must not have been added on failure.
        assert!(engine.state.colonies.is_empty());
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
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
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

    /// A minimal registry with one power-free-to-check building that turns
    /// water into more water-valued output, so `total_output` (issue #212)
    /// has something nonzero to compute: a `water_well` producing `water`
    /// (`base_value: 5.0`) each sol, plus a solar array so it isn't
    /// power-starved.
    fn economic_output_registry() -> crate::content::ContentRegistry {
        use crate::content::types::{BuildingCategory, BuildingDef, Ingredient, RecipeDef};
        let mut reg = crate::content::ContentRegistry::default();

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
            maintenance: vec![],
        });

        reg.insert_building(BuildingDef {
            id: "water_well".into(),
            name: "Water Well".into(),
            description: String::new(),
            category: BuildingCategory::Production,
            construction_cost: vec![],
            power_delta: 4.0,
            worker_slots: 1,
            labor_required: 0,
            slot_cost: 1,
            construction_turns: 1,
            tech_prerequisite: None,
            maintenance: vec![],
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

        reg.insert_recipe(RecipeDef {
            id: "pump_water".into(),
            name: "Pump Water".into(),
            building: "water_well".into(),
            inputs: vec![],
            outputs: vec![Ingredient {
                id: "water".into(),
                quantity: 24.0,
            }],
            cycle_sols: 1,
            power_draw: 4.0,
            concurrent: false,
        });

        reg
    }

    /// Real-engine proof that `base_value` now drives the `EconomicMilestone`
    /// victory condition's `total_output` (issue #212): a colony producing
    /// 24 water/sol at `base_value: 5.0` should report `total_output >= 120`
    /// once `Command::EvaluateVictory` runs, not the old hardcoded zero.
    #[test]
    fn economic_milestone_reflects_base_value_weighted_production() {
        let mut engine = GameEngine::with_seed(0);
        engine.state.registry = Some(economic_output_registry());
        engine.state.victory_state =
            victory::VictoryState::new(vec![victory::VictoryCondition::EconomicMilestone {
                target_output: 100,
            }]);

        let events = engine
            .apply(&Command::FoundColony {
                name: "Economic Test".into(),
                starting_population: 50,
            })
            .unwrap();
        let Event::ColonyFounded { colony_id, .. } = &events[0] else {
            panic!()
        };
        let idx = engine.find_colony_index(*colony_id).unwrap();
        engine.state.colonies[idx]
            .buildings
            .push(colony::PlacedBuilding::new("solar_array", 1));
        engine.state.colonies[idx]
            .buildings
            .push(colony::PlacedBuilding::new("water_well", 1));

        engine.apply(&Command::AdvanceColonySol).unwrap();

        let events = engine.apply(&Command::EvaluateVictory).unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::VictoryAchieved { .. })),
            "24 water/sol * base_value 5.0 = 120 should clear target_output 100, got events: {events:?}"
        );

        let QueryResult::VictoryStatus(progress) = engine.query(&Query::VictoryStatus).unwrap()
        else {
            panic!("expected VictoryStatus result");
        };
        let economic = progress
            .iter()
            .find(|p| {
                matches!(
                    p.condition,
                    victory::VictoryCondition::EconomicMilestone { .. }
                )
            })
            .expect("EconomicMilestone condition must be tracked");
        assert!(
            economic.current >= 120,
            "expected total_output >= 120 (24 water * 5.0 base_value), got {}",
            economic.current
        );
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
        assert!(
            matches!(err, EngineError::GameOver),
            "expected GameOver before sandbox"
        );

        // Activate via the issue-#96 canonical command name.
        let events = engine.apply(&Command::ContinueSandbox).unwrap();
        assert!(
            events.iter().any(|e| matches!(e, Event::SandboxContinued)),
            "expected SandboxContinued event"
        );

        // sandbox_mode top-level flag must be set.
        assert!(
            engine.state.sandbox_mode,
            "GameState::sandbox_mode should be true"
        );

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
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
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
                supplies_id: None,
                supply_overrides: None,
                body_id: None,
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
                body_id: None,
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
                body_id: None,
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
                body_id: None,
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
                body_id: None,
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

    /// `Command::BuildOrbitalStation` builds immediately (no queue) at a
    /// caller-chosen size within the type's slot range (issue #234).
    #[test]
    fn build_orbital_station_command_builds_immediately_at_chosen_size() {
        let mut engine = GameEngine::with_seed(94);
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

        let (min, max) = orbital::StationType::Habitat.slot_range();
        let events = engine
            .apply(&Command::BuildOrbitalStation {
                colony_id,
                station_type: orbital::StationType::Habitat,
                orbit_type: orbital::OrbitType::Low,
                body_id: None,
                slot_cost: Some(max),
            })
            .unwrap();

        assert_eq!(engine.state.orbital_registry.stations.len(), 1);
        assert_eq!(engine.state.orbital_registry.stations[0].slot_cost, max);
        assert!(events.iter().any(|e| matches!(
            e,
            Event::OrbitalStationBuilt { slot_cost, .. } if *slot_cost == max
        )));
        assert!(max > min, "sanity: Habitat has a non-trivial slot range");
    }

    /// `Command::BuildOrbitalStation` rejects a `slot_cost` outside the
    /// station type's valid range (issue #234).
    #[test]
    fn build_orbital_station_command_rejects_out_of_range_slot_cost() {
        let mut engine = GameEngine::with_seed(95);
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

        let (_, max) = orbital::StationType::Habitat.slot_range();
        let result = engine.apply(&Command::BuildOrbitalStation {
            colony_id,
            station_type: orbital::StationType::Habitat,
            orbit_type: orbital::OrbitType::Low,
            body_id: None,
            slot_cost: Some(max + 1),
        });

        assert!(matches!(
            result,
            Err(EngineError::OrbitalError(
                OrbitalError::InvalidSlotCost { .. }
            ))
        ));
        assert!(engine.state.orbital_registry.stations.is_empty());
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

    // ── M8: Field expedition tests (issue #103) ───────────────────────────────

    fn make_expedition_engine() -> GameEngine {
        let mut engine = GameEngine::new();
        let colony = colony::Colony::new("Base Camp".to_string());
        engine.state.add_colony(colony, 100);
        engine
    }

    fn first_colony_id(engine: &GameEngine) -> ColonyId {
        engine.state.colonies[0].id
    }

    #[test]
    fn launch_field_expedition_creates_expedition_and_emits_event() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let hex = map::HexCoord::new(3, -1);

        let events = engine
            .apply(&Command::LaunchFieldExpedition {
                colony_id: cid,
                target_hex: hex,
                crew_count: 4,
                supplies: 200.0,
                transit_sols: 10,
                is_deep_space: false,
            })
            .unwrap();

        assert_eq!(engine.state.expeditions.len(), 1);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, Event::ExpeditionLaunched { .. })),
            "ExpeditionLaunched event must be emitted"
        );
    }

    #[test]
    fn expedition_arrives_after_transit_sols() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let hex = map::HexCoord::new(1, 0);

        engine
            .apply(&Command::LaunchFieldExpedition {
                colony_id: cid,
                target_hex: hex,
                crew_count: 2,
                supplies: 500.0,
                transit_sols: 3,
                is_deep_space: false,
            })
            .unwrap();

        // Advance 3 sols.
        let mut arrived = false;
        for _ in 0..3 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            if evs
                .iter()
                .any(|e| matches!(e, Event::ExpeditionArrived { .. }))
            {
                arrived = true;
            }
        }
        assert!(
            arrived,
            "ExpeditionArrived must be emitted within transit period"
        );
        assert_eq!(
            engine.state.expeditions[0].status,
            expedition::ExpeditionStatus::OnSite
        );
    }

    #[test]
    fn expedition_makes_discovery_on_site() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let hex = map::HexCoord::new(0, 1);

        engine
            .apply(&Command::LaunchFieldExpedition {
                colony_id: cid,
                target_hex: hex,
                crew_count: 5,
                supplies: 1000.0,
                transit_sols: 1,
                is_deep_space: false,
            })
            .unwrap();

        // Advance enough sols to arrive and spend time on-site.
        let mut discovery_found = false;
        for _ in 0..10 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            if evs
                .iter()
                .any(|e| matches!(e, Event::ExpeditionDiscovery { .. }))
            {
                discovery_found = true;
            }
        }
        assert!(discovery_found, "At least one ExpeditionDiscovery expected");
    }

    #[test]
    fn expedition_returns_and_deposits_resources() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let hex = map::HexCoord::new(2, 0);

        engine
            .apply(&Command::LaunchFieldExpedition {
                colony_id: cid,
                target_hex: hex,
                crew_count: 3,
                supplies: 2000.0,
                transit_sols: 1,
                is_deep_space: false,
            })
            .unwrap();

        let mut returned = false;
        // Advance enough sols to complete the full cycle.
        for _ in 0..30 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            if let Some(Event::ExpeditionReturned { deposits, .. }) = evs
                .iter()
                .find(|e| matches!(e, Event::ExpeditionReturned { .. }))
            {
                assert!(
                    !deposits.is_empty(),
                    "Deposits should not be empty on return"
                );
                returned = true;
                break;
            }
        }
        assert!(returned, "ExpeditionReturned must be emitted");
        assert_eq!(
            engine.state.expeditions[0].status,
            expedition::ExpeditionStatus::Completed
        );
    }

    #[test]
    fn supply_depletion_causes_expedition_lost() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let hex = map::HexCoord::new(5, 0);

        // Only 3 supply units — will be exhausted in 1 sol (crew_count=4 burns 4/sol).
        engine
            .apply(&Command::LaunchFieldExpedition {
                colony_id: cid,
                target_hex: hex,
                crew_count: 4,
                supplies: 3.0,
                transit_sols: 10,
                is_deep_space: false,
            })
            .unwrap();

        let mut lost = false;
        for _ in 0..5 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            if evs
                .iter()
                .any(|e| matches!(e, Event::ExpeditionLost { .. }))
            {
                lost = true;
                break;
            }
        }
        assert!(lost, "ExpeditionLost must be emitted when supplies run out");
        assert_eq!(
            engine.state.expeditions[0].status,
            expedition::ExpeditionStatus::Lost
        );
    }

    #[test]
    fn recall_sets_expedition_to_returning() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let hex = map::HexCoord::new(1, 1);

        engine
            .apply(&Command::LaunchFieldExpedition {
                colony_id: cid,
                target_hex: hex,
                crew_count: 2,
                supplies: 1000.0,
                transit_sols: 1,
                is_deep_space: false,
            })
            .unwrap();

        // Arrive on-site (1 sol transit).
        for _ in 0..2 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
        }
        assert_eq!(
            engine.state.expeditions[0].status,
            expedition::ExpeditionStatus::OnSite
        );

        let eid = engine.state.expeditions[0].id.clone();
        engine
            .apply(&Command::RecallExpedition { expedition_id: eid })
            .unwrap();

        assert_eq!(
            engine.state.expeditions[0].status,
            expedition::ExpeditionStatus::Returning
        );
    }

    // ── Body-scouting survey expeditions (issue #235) ─────────────────────────

    fn add_body(engine: &mut GameEngine, name: &str) -> system::BodyId {
        let events = engine
            .apply(&Command::System(system::SystemCommand::AddBody {
                name: name.to_string(),
                kind: system::BodyKind::InnerPlanet,
                distance_au: 1.2,
            }))
            .unwrap();
        match &events[0] {
            Event::System(system::SystemEvent::BodyAdded { body_id, .. }) => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        }
    }

    #[test]
    fn launch_survey_expedition_rejects_unknown_body() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let bogus_body = system::BodyId::new();

        let result = engine.apply(&Command::LaunchSurveyExpedition {
            colony_id: cid,
            target_body: bogus_body,
            expedition_type: expedition::ExpeditionType::FastFlybyProbe,
        });

        assert!(matches!(result, Err(EngineError::InvalidArgument(_))));
    }

    #[test]
    fn launch_survey_expedition_applies_tech_propulsion_scalar() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let body_id = add_body(&mut engine, "Fast Prospect");

        // Simulate several stacked ReduceTransitTime techs already
        // researched (issue #236): a strong scalar makes the reduction
        // deterministically visible against SURVEY_TRANSIT_TURNS = 2.
        engine.state.propulsion_transit_scalar = 0.4;

        engine
            .apply(&Command::LaunchSurveyExpedition {
                colony_id: cid,
                target_body: body_id,
                expedition_type: expedition::ExpeditionType::FastFlybyProbe,
            })
            .unwrap();

        let (_, state) = engine.state.expedition_registry.iter().next().unwrap();
        // 2 * 0.4 = 0.8, floored at 1.0 minimum, rounds to 1.
        assert_eq!(
            state.transit_turns_remaining,
            1,
            "propulsion scalar must reduce transit turns below the unscaled \
             baseline of {}",
            expedition::SURVEY_TRANSIT_TURNS
        );
    }

    #[test]
    fn survey_completion_applies_tech_survey_modifier_bonus() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let body_id = add_body(&mut engine, "Sensor Target");

        // A guaranteed-full-reveal bonus (issue #236): base FastFlybyProbe
        // full-reveal probability is only 0.10, but a +0.95 tech bonus
        // pushes it to a near-certain full reveal regardless of roll.
        engine.state.tech_survey_modifiers.full_reveal_bonus = 0.95;

        engine
            .apply(&Command::LaunchSurveyExpedition {
                colony_id: cid,
                target_body: body_id.clone(),
                expedition_type: expedition::ExpeditionType::FastFlybyProbe,
            })
            .unwrap();

        let mut completed_outcome = None;
        for _ in 0..(expedition::SURVEY_TRANSIT_TURNS
            + expedition::ExpeditionType::FastFlybyProbe.base_duration_turns())
        {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            for e in &evs {
                if let Event::SurveyCompleted { outcome, .. } = e {
                    completed_outcome = Some(outcome.clone());
                }
            }
        }

        assert!(
            matches!(
                completed_outcome,
                Some(expedition::SurveyOutcome::FullReveal { .. })
            ),
            "tech survey-modifier bonus must push a low-odds probe to a full \
             reveal: got {completed_outcome:?}"
        );
    }

    #[test]
    fn survey_expedition_full_lifecycle_launch_transit_survey_completes() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let body_id = add_body(&mut engine, "Prospect");

        let events = engine
            .apply(&Command::LaunchSurveyExpedition {
                colony_id: cid,
                target_body: body_id.clone(),
                expedition_type: expedition::ExpeditionType::FastFlybyProbe,
            })
            .unwrap();
        let eid = events
            .iter()
            .find_map(|e| {
                if let Event::SurveyExpeditionLaunched { expedition_id, .. } = e {
                    Some(expedition_id.clone())
                } else {
                    None
                }
            })
            .expect("SurveyExpeditionLaunched must be emitted");

        assert_eq!(
            engine.state.expedition_registry.get(&eid).unwrap().phase,
            expedition::ExpeditionPhase::InTransit
        );

        // FastFlybyProbe: 2 transit turns + 2 survey turns, no anomalies
        // loaded (no registry set), so it always reaches Completed by sol 4.
        let mut arrived = false;
        let mut completed_outcome = None;
        for _ in 0..expedition::SURVEY_TRANSIT_TURNS
            + expedition::ExpeditionType::FastFlybyProbe.base_duration_turns()
        {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            if evs
                .iter()
                .any(|e| matches!(e, Event::SurveyExpeditionArrived { .. }))
            {
                arrived = true;
            }
            for e in &evs {
                if let Event::SurveyCompleted {
                    expedition_id,
                    outcome,
                    ..
                } = e
                {
                    if *expedition_id == eid {
                        completed_outcome = Some(outcome.clone());
                    }
                }
            }
        }

        assert!(arrived, "SurveyExpeditionArrived must fire after transit");
        let outcome = completed_outcome.expect("SurveyCompleted must fire by end of mission");
        assert_eq!(
            engine.state.expedition_registry.get(&eid).unwrap().phase,
            expedition::ExpeditionPhase::Completed
        );

        // Whatever the roll produced, the outcome's body_id matches the target.
        let outcome_body = match &outcome {
            expedition::SurveyOutcome::FullReveal { body_id, .. }
            | expedition::SurveyOutcome::PartialReveal { body_id, .. }
            | expedition::SurveyOutcome::Failed { body_id, .. } => body_id.clone(),
        };
        assert_eq!(outcome_body, body_id);
    }

    #[test]
    fn survey_expedition_anomaly_triggers_and_investigation_pays_research_and_resources() {
        let mut engine = make_expedition_engine();
        let cid = first_colony_id(&engine);
        let body_id = add_body(&mut engine, "Anomaly World");

        // An anomaly guaranteed to trigger on the first eligible sol
        // (trigger_probability = 1.0), with a single deterministic outcome.
        let mut resource_reward = std::collections::HashMap::new();
        resource_reward.insert("structural_ore".to_string(), 25.0);
        let anomaly = expedition::AnomalyDef {
            id: "test_anomaly".to_string(),
            name: "Test Anomaly".to_string(),
            trigger_probability: 1.0,
            eligible_expedition_types: vec![expedition::ExpeditionType::FastFlybyProbe],
            description: "A guaranteed test anomaly.".to_string(),
            outcomes: vec![expedition::AnomalyOutcome {
                id: "only_outcome".to_string(),
                weight: 1.0,
                description: "The only possible outcome.".to_string(),
                research_bonus: 42.0,
                resource_reward,
                unlocks_tech: None,
            }],
        };
        let mut registry = content::ContentRegistry::default();
        registry.insert_anomaly(anomaly);
        engine.state.registry = Some(registry);

        engine
            .apply(&Command::LaunchSurveyExpedition {
                colony_id: cid,
                target_body: body_id,
                expedition_type: expedition::ExpeditionType::FastFlybyProbe,
            })
            .unwrap();

        // Advance through the transit leg; the anomaly can only fire once
        // Surveying begins.
        let mut mid_mission_event = None;
        for _ in 0..(expedition::SURVEY_TRANSIT_TURNS + 1) {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            for e in &evs {
                if let Event::MidMissionEventTriggered { event, .. } = e {
                    mid_mission_event = Some(event.clone());
                }
            }
            if mid_mission_event.is_some() {
                break;
            }
        }
        let event = mid_mission_event.expect("guaranteed anomaly must trigger while surveying");

        let eid = engine
            .state
            .expedition_registry
            .iter()
            .next()
            .map(|(id, _)| id.clone())
            .unwrap();
        assert_eq!(
            engine.state.expedition_registry.get(&eid).unwrap().phase,
            expedition::ExpeditionPhase::AwaitingDecision
        );

        let research_before = engine.state.research_pool.total();
        let colony_idx = engine.find_colony_index(cid).unwrap();
        let ore_before = engine.state.colonies[colony_idx]
            .pool
            .amount("structural_ore");

        let investigate_choice = event
            .choices
            .iter()
            .find(|c| c.id == "investigate")
            .expect("investigate choice must be present");
        let resolve_events = engine
            .apply(&Command::ResolveMissionDecision {
                expedition_id: eid.clone(),
                choice_id: investigate_choice.id.clone(),
            })
            .unwrap();

        assert!(
            resolve_events
                .iter()
                .any(|e| matches!(e, Event::AnomalyOutcomeResolved { .. })),
            "AnomalyOutcomeResolved must be emitted on investigation"
        );
        assert!((engine.state.research_pool.total() - research_before - 42.0).abs() < 1e-6);
        let ore_after = engine.state.colonies[colony_idx]
            .pool
            .amount("structural_ore");
        assert!((ore_after - ore_before - 25.0).abs() < 1e-6);
        assert_eq!(
            engine.state.expedition_registry.get(&eid).unwrap().phase,
            expedition::ExpeditionPhase::Surveying,
            "expedition should resume surveying after the decision resolves"
        );
    }

    #[test]
    fn deep_space_expedition_return_sets_expedition_launched() {
        let mut engine = make_expedition_engine();
        // Set up capstone victory condition so victory snapshot works.
        engine
            .apply(&Command::InitVictoryConditions {
                conditions: vec![victory::VictoryCondition::InterstellarExpeditionLaunched],
            })
            .unwrap();

        let cid = first_colony_id(&engine);
        let hex = map::HexCoord::new(0, 0);

        engine
            .apply(&Command::LaunchFieldExpedition {
                colony_id: cid,
                target_hex: hex,
                crew_count: 5,
                supplies: 5000.0,
                transit_sols: 1,
                is_deep_space: true,
            })
            .unwrap();

        let mut victory_achieved = false;
        for _ in 0..30 {
            let evs = engine.apply(&Command::AdvanceColonySol).unwrap();
            if evs
                .iter()
                .any(|e| matches!(e, Event::VictoryAchieved { .. }))
            {
                victory_achieved = true;
                break;
            }
        }
        assert!(
            engine.state.expedition_launched,
            "expedition_launched must be set after deep-space return"
        );
        assert!(
            victory_achieved,
            "VictoryAchieved must be emitted for deep-space expedition return"
        );
    }
}
