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

use outpost_core::{difficulty::DifficultyPreset, ColonyStatus, ColonySummary, Event, QueryResult};
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
    /// Advance up to `max_sols` sols, halting early on an interrupt at or above
    /// `threshold` (issue #332).
    ///
    /// `threshold` is a tier slug — `"ambient"`, `"notable"`, `"urgent"`, or
    /// `"blocking"`. An unrecognised value is rejected rather than silently
    /// defaulting, so a typo can't turn a cautious fast-forward into one that
    /// runs straight through a crisis.
    FastForward {
        /// Upper bound on sols to advance.
        max_sols: u32,
        /// Lowest interrupt tier that halts the run.
        threshold: String,
    },
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
    /// Cancel a queued construction project and receive a 50% partial refund.
    CancelConstruction {
        /// Target colony UUID.
        colony_id: String,
        /// UUID of the construction project to cancel.
        project_id: String,
    },
    /// Place a batch of starter buildings instantly at colony founding,
    /// bypassing the normal construction queue (issue: playtest feedback
    /// round 2 — "lander" mechanic).
    DeployStarterKit {
        /// Target colony UUID.
        colony_id: String,
        /// `(building_type, slot_cost)` pairs to place, in order.
        buildings: Vec<(String, u32)>,
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
    /// Select which recipe a building type runs in a colony (issue #166).
    SetActiveRecipe {
        /// Target colony UUID.
        colony_id: String,
        /// Building type to set the active recipe for.
        building_type: String,
        /// Recipe id to activate.
        recipe_id: String,
    },
    /// Set one placed building's staffing priority (issue #307).
    ///
    /// Instance-scoped, unlike `SetActiveRecipe` above, which applies to every
    /// building of a type.
    SetBuildingPriority {
        /// Target colony UUID.
        colony_id: String,
        /// Placed-building instance UUID.
        building_id: String,
        /// New priority; `1` is staffed first.
        priority: u8,
    },
    /// Pin (`Some`) or release (`None`) a placed building's labour (issue #307).
    SetBuildingLabourLock {
        /// Target colony UUID.
        colony_id: String,
        /// Placed-building instance UUID.
        building_id: String,
        /// Workers to pin, or `null` to unlock.
        lock: Option<u32>,
    },
    /// Name a placed building, or clear it back to the auto-numbered default
    /// (issue #307).
    RenameBuilding {
        /// Target colony UUID.
        colony_id: String,
        /// Placed-building instance UUID.
        building_id: String,
        /// New name, or `null` to revert to the default.
        name: Option<String>,
    },
    /// Found a new colony at a surveyed planet-map site (issue #220 — the
    /// founding wizard's primary command in browser mode).
    FoundColonyAtSite {
        /// Display name for the new colony.
        name: String,
        /// Starting colonist head-count.
        starting_population: u64,
        /// Surveyed site UUID where the colony is being placed.
        site_id: String,
        /// Optional economic focus description.
        #[serde(default)]
        focus: Option<String>,
        /// Optional `SupplyPackage` id used to seed starting commodities.
        #[serde(default)]
        supplies_id: Option<String>,
        /// Explicit per-commodity starter-supply amounts (issue #167).
        #[serde(default)]
        supply_overrides: Option<Vec<(String, f64)>>,
        /// Star-system body this site belongs to, if known (issue #183).
        #[serde(default)]
        body_id: Option<String>,
    },
    /// Link a colony to its home system body (issue #163).
    AssignColonyHomeBody {
        /// Colony to update.
        colony_id: String,
        /// Star-system body UUID.
        body_id: String,
    },
    /// Set the active difficulty preset.
    SetDifficulty {
        /// Difficulty grade: "sandbox", "easy", "normal", "hard", or "brutal".
        grade: String,
    },
    /// Set the current research project, replacing any active one.
    ResearchTech {
        /// Content-pack tech identifier.
        tech_id: String,
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
    /// Remove the infrastructure edge between two colonies.
    DemolishInfrastructure {
        /// Source colony UUID.
        from_colony: String,
        /// Destination colony UUID.
        to_colony: String,
    },
    /// Begin construction of an orbital station using a blueprint.
    BeginOrbitalConstruction {
        /// Content-pack blueprint identifier.
        blueprint_id: String,
        /// Colony that funds and operates the station.
        colony_id: String,
        /// Orbit band: "low", "geostationary", or "lagrange".
        orbit_type: String,
        /// System body the finished station will orbit (issue #234), as a
        /// UUID string. Omitted/`None` for a Lagrange-band, system-wide
        /// station.
        #[serde(default)]
        body_id: Option<String>,
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
    /// Snapshot current engine state to the configured `SQLite` database.
    SaveGame,
    /// Restore engine state from the configured `SQLite` database.
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
    /// Initialise a new game: load content, apply difficulty, generate the
    /// star system, seed planet, found colony.
    NewGame {
        /// Difficulty preset to apply.
        difficulty: DifficultyPreset,
        /// Deterministic seed used for planet map generation.
        planet_seed: u64,
        /// Deterministic seed used for star-system generation (issue #199).
        ///
        /// Independent of `planet_seed` by design — the player can reroll
        /// the system without rerolling the founding planet's hex map, or
        /// vice versa. Optional for backward compatibility with older
        /// clients; defaults to `planet_seed` when omitted, matching the
        /// only behavior a caller with a single seed field could have meant.
        #[serde(default)]
        system_seed: Option<u64>,
        /// Optional star-system generation tuning (playtest feedback: New
        /// Game sliders). Omitted/`None` fields fall back to
        /// `outpost_core::system_gen::SystemGenParams::default`'s values.
        #[serde(default)]
        habitable_zone_center_au: Option<f32>,
        /// See `habitable_zone_center_au`.
        #[serde(default)]
        min_inner_planets: Option<u32>,
        /// See `habitable_zone_center_au`.
        #[serde(default)]
        max_inner_planets: Option<u32>,
        /// Overrides the difficulty-derived deposit-abundance scalar when
        /// present; otherwise the scalar is resolved from `difficulty` as
        /// before.
        #[serde(default)]
        abundance_scalar: Option<f32>,
    },
    /// Establish a new outpost anchored to a body (issue #233/#243).
    EstablishOutpost {
        /// Outpost display name.
        name: String,
        /// Colony establishing (and owning) the outpost.
        colony_id: String,
        /// System body UUID to anchor the outpost to.
        body_id: String,
    },
    /// Decommission an outpost, removing it from play (issue #233/#243).
    DecommissionOutpost {
        /// Outpost UUID.
        outpost_id: String,
    },
    /// Queue a construction project at an outpost (issue #233/#243).
    QueueOutpostConstruction {
        /// Target outpost UUID.
        outpost_id: String,
        /// Content-pack key of the building type.
        building_type: String,
        /// Build-slot cost.
        slot_cost: u32,
        /// Labour units consumed per construction turn.
        labor_per_turn: u32,
        /// Commodity costs (commodity id, quantity) pairs.
        construction_cost: Vec<(String, f64)>,
        /// Colony-sol turns required to complete.
        construction_turns: u32,
    },
    /// Promote an established outpost into a full colony (issue #242/#243).
    PromoteOutpostToColony {
        /// Outpost UUID to promote.
        outpost_id: String,
        /// Name for the resulting colony.
        name: String,
        /// Starting population for the new colony.
        starting_population: u64,
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
    /// Full snapshot returned after `NewGame` initialisation completes.
    NewGameSnapshot {
        /// Sequence number from the originating command.
        seq: u64,
        /// Current world snapshot (post-init).
        state: WorldSnapshot,
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
    /// An environmental hazard struck a colony.
    HazardOccurred {
        /// Colony UUID.
        colony_id: String,
        /// Hazard category (`snake_case` string).
        hazard_kind: String,
        /// Sampled severity in `[0, 1]`.
        severity: f32,
        /// Stability change applied (negative).
        stability_delta: f32,
        /// Commodity losses: `(commodity_id, amount_lost)`.
        commodity_losses: Vec<(String, f64)>,
        /// Population lost this hazard.
        population_lost: f32,
    },
    /// Colonists arrived at a destination colony.
    MigrationArrived {
        /// Source colony UUID (`null` for off-map waves).
        from_colony: Option<String>,
        /// Destination colony UUID.
        to_colony: String,
        /// Number of colonists who arrived.
        count: f32,
        /// Stability penalty to receiving colony for overcrowding.
        overcrowding_stability_penalty: f32,
        /// Stability penalty to sending colony for forced departure.
        forced_departure_stability_penalty: f32,
    },
    /// Voluntary emigration was auto-triggered due to low stability.
    VoluntaryEmigrationTriggered {
        /// Colony colonists departed from.
        from_colony: String,
        /// Colony colonists were directed toward.
        to_colony: String,
        /// Number of colonists that departed.
        count: f32,
    },
    /// A field expedition was launched from a colony.
    ExpeditionLaunched {
        /// Expedition UUID.
        expedition_id: String,
        /// Origin colony UUID.
        colony_id: String,
        /// Target hex (axial q, r).
        target_hex_q: i32,
        /// Target hex axial r.
        target_hex_r: i32,
    },
    /// A field expedition arrived at its target hex.
    ExpeditionArrived {
        /// Expedition UUID.
        expedition_id: String,
    },
    /// A field expedition completed its return and deposited resources.
    ExpeditionReturned {
        /// Expedition UUID.
        expedition_id: String,
        /// Colony UUID that received deposits.
        colony_id: String,
        /// Resources deposited: `(commodity_id, amount)`.
        deposits: Vec<(String, f64)>,
    },
    /// A field expedition was lost due to supply depletion.
    ExpeditionLost {
        /// Expedition UUID.
        expedition_id: String,
    },
    /// A technology node completed research.
    TechUnlocked {
        /// Content-pack id of the tech that finished.
        tech_id: String,
    },
    /// A victory condition was satisfied.
    VictoryAchieved {
        /// The condition that was satisfied (debug string).
        condition: String,
    },
    /// The menace level crossed its critical threshold.
    MenaceCritical {
        /// Menace category (`snake_case` string).
        menace_kind: String,
        /// Menace level at the moment it went critical.
        level: f32,
        /// Strategic months before game-over if unmitigated.
        countdown_months: u32,
    },
    /// A cargo shipment was credited to a colony pool.
    CargoDelivered {
        /// Shipment UUID.
        shipment_id: String,
        /// Colony UUID.
        colony_id: String,
        /// Commodity identifier.
        commodity_id: String,
        /// Quantity deposited.
        amount: f64,
    },
    /// An inter-colony trade convoy landed and was credited to a colony pool
    /// (issue #332).
    TradeConvoyArrived {
        /// Convoy UUID.
        convoy_id: String,
        /// Sending colony UUID.
        from_colony: String,
        /// Receiving colony UUID.
        to_colony: String,
        /// Commodity identifier.
        commodity_id: String,
        /// Quantity deposited.
        amount: f64,
    },
    /// A fast-forward run finished (issue #332).
    ///
    /// The UI's cue to stop its play timer and, when `halted`, open the digest
    /// panel — `GET /api/interrupt-digest` has the accumulated detail.
    FastForwardEnded {
        /// Sol the run stopped on.
        sol: u64,
        /// Sols actually advanced.
        sols_advanced: u32,
        /// Whether an interrupt stopped the run early.
        halted: bool,
        /// Why it halted, when it did.
        halting_reason: Option<String>,
    },
    /// An orbital station construction project finished.
    OrbitalStationCompleted {
        /// Station UUID.
        station_id: String,
        /// Colony UUID.
        colony_id: String,
        /// Station specialisation type (`snake_case` string).
        station_type: String,
        /// Orbit band (`snake_case` string).
        orbit_type: String,
        /// Blueprint id that produced this station.
        blueprint_id: String,
        /// System body the station orbits (issue #234), as a UUID string;
        /// `None` for a Lagrange-band, system-wide station.
        body_id: Option<String>,
    },
    /// A new outpost was established (issue #233/#243).
    OutpostEstablished {
        /// New outpost's UUID.
        outpost_id: String,
        /// Colony that established it.
        colony_id: String,
        /// Body it's anchored to.
        body_id: String,
    },
    /// An outpost was decommissioned (issue #233/#243).
    OutpostDecommissioned {
        /// Removed outpost's UUID.
        outpost_id: String,
    },
    /// An outpost queued a construction project (issue #233/#243).
    OutpostConstructionQueued {
        /// Target outpost UUID.
        outpost_id: String,
        /// Building type queued.
        building_type: String,
        /// Project UUID.
        project_id: String,
    },
    /// An outpost was promoted into a full colony (issue #242/#243).
    OutpostPromoted {
        /// Promoted outpost's former UUID.
        outpost_id: String,
        /// Newly created colony's UUID.
        colony_id: String,
        /// New colony's name.
        name: String,
    },
    /// A core event with no frontend representation; safely ignored.
    Ignored,
}

impl ServerEvent {
    /// Convert a core [`Event`] into the stable [`ServerEvent`] wire format.
    #[must_use]
    #[allow(clippy::too_many_lines)]
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
            Event::HazardOccurred {
                colony_id,
                kind,
                severity,
                stability_delta,
                commodity_losses,
                population_lost,
            } => Self::HazardOccurred {
                colony_id: colony_id.to_string(),
                hazard_kind: format!("{kind:?}"),
                severity: *severity,
                stability_delta: *stability_delta,
                commodity_losses: commodity_losses.clone(),
                population_lost: *population_lost,
            },
            Event::MigrationArrived {
                from_colony,
                to_colony,
                count,
                overcrowding_stability_penalty,
                forced_departure_stability_penalty,
            } => Self::MigrationArrived {
                from_colony: from_colony.as_ref().map(ToString::to_string),
                to_colony: to_colony.to_string(),
                count: *count,
                overcrowding_stability_penalty: *overcrowding_stability_penalty,
                forced_departure_stability_penalty: *forced_departure_stability_penalty,
            },
            Event::VoluntaryEmigrationTriggered {
                from_colony,
                to_colony,
                count,
            } => Self::VoluntaryEmigrationTriggered {
                from_colony: from_colony.to_string(),
                to_colony: to_colony.to_string(),
                count: *count,
            },
            Event::ExpeditionLaunched {
                expedition_id,
                colony_id,
                target_hex,
            } => Self::ExpeditionLaunched {
                expedition_id: expedition_id.0.to_string(),
                colony_id: colony_id.to_string(),
                target_hex_q: target_hex.q,
                target_hex_r: target_hex.r,
            },
            Event::ExpeditionArrived { expedition_id } => Self::ExpeditionArrived {
                expedition_id: expedition_id.0.to_string(),
            },
            Event::ExpeditionReturned {
                expedition_id,
                colony_id,
                deposits,
            } => Self::ExpeditionReturned {
                expedition_id: expedition_id.0.to_string(),
                colony_id: colony_id.to_string(),
                deposits: deposits.clone(),
            },
            Event::ExpeditionLost { expedition_id } => Self::ExpeditionLost {
                expedition_id: expedition_id.0.to_string(),
            },
            Event::TechUnlocked { tech_id } => Self::TechUnlocked {
                tech_id: tech_id.clone(),
            },
            Event::VictoryAchieved { condition } => Self::VictoryAchieved {
                condition: format!("{condition:?}"),
            },
            Event::MenaceCritical {
                kind,
                level,
                countdown_months,
            } => Self::MenaceCritical {
                menace_kind: format!("{kind:?}"),
                level: *level,
                countdown_months: *countdown_months,
            },
            Event::CargoDelivered {
                shipment_id,
                colony_id,
                commodity_id,
                amount,
            } => Self::CargoDelivered {
                shipment_id: shipment_id.to_string(),
                colony_id: colony_id.to_string(),
                commodity_id: commodity_id.clone(),
                amount: *amount,
            },
            Event::TradeConvoyArrived {
                convoy_id,
                from_colony,
                to_colony,
                commodity_id,
                amount,
            } => Self::TradeConvoyArrived {
                convoy_id: convoy_id.to_string(),
                from_colony: from_colony.to_string(),
                to_colony: to_colony.to_string(),
                commodity_id: commodity_id.clone(),
                amount: *amount,
            },
            Event::FastForwardEnded {
                sol,
                sols_advanced,
                halted,
                halting_reason,
            } => Self::FastForwardEnded {
                sol: *sol,
                sols_advanced: *sols_advanced,
                halted: *halted,
                halting_reason: halting_reason.clone(),
            },
            Event::OrbitalStationCompleted {
                station_id,
                colony_id,
                station_type,
                orbit_type,
                blueprint_id,
                body_id,
            } => Self::OrbitalStationCompleted {
                station_id: station_id.to_string(),
                colony_id: colony_id.to_string(),
                station_type: format!("{station_type:?}"),
                orbit_type: format!("{orbit_type:?}"),
                blueprint_id: blueprint_id.clone(),
                body_id: body_id.as_ref().map(|b| b.0.to_string()),
            },
            Event::OutpostEstablished {
                outpost_id,
                colony_id,
                body_id,
            } => Self::OutpostEstablished {
                outpost_id: outpost_id.to_string(),
                colony_id: colony_id.to_string(),
                body_id: body_id.0.to_string(),
            },
            Event::OutpostDecommissioned { outpost_id } => Self::OutpostDecommissioned {
                outpost_id: outpost_id.to_string(),
            },
            Event::OutpostConstructionQueued {
                outpost_id,
                building_type,
                project_id,
            } => Self::OutpostConstructionQueued {
                outpost_id: outpost_id.to_string(),
                building_type: building_type.clone(),
                project_id: project_id.to_string(),
            },
            Event::OutpostPromoted {
                outpost_id,
                colony_id,
                name,
            } => Self::OutpostPromoted {
                outpost_id: outpost_id.to_string(),
                colony_id: colony_id.to_string(),
                name: name.clone(),
            },
            // All remaining core events have no frontend representation.
            _ => Self::Ignored,
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
        let raw =
            r#"{"type":"command","seq":3,"command":{"kind":"set_difficulty","grade":"hard"}}"#;
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
    fn client_command_research_tech_deserialises() {
        let raw = r#"{"type":"command","seq":6,"command":{"kind":"research_tech","tech_id":"fusion_power"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::ResearchTech { tech_id },
            } => {
                assert_eq!(seq, 6);
                assert_eq!(tech_id, "fusion_power");
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
    fn client_command_found_colony_at_site_deserialises() {
        let raw = r#"{"type":"command","seq":7,"command":{"kind":"found_colony_at_site","name":"Alpha","starting_population":100,"site_id":"00000000-0000-0000-0000-000000000003"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::FoundColonyAtSite {
                        name,
                        starting_population,
                        site_id,
                        focus,
                        supplies_id,
                        supply_overrides,
                        body_id,
                    },
            } => {
                assert_eq!(seq, 7);
                assert_eq!(name, "Alpha");
                assert_eq!(starting_population, 100);
                assert_eq!(site_id, "00000000-0000-0000-0000-000000000003");
                assert_eq!(focus, None);
                assert_eq!(supplies_id, None);
                assert_eq!(supply_overrides, None);
                assert_eq!(body_id, None);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_assign_colony_home_body_deserialises() {
        let raw = r#"{"type":"command","seq":8,"command":{"kind":"assign_colony_home_body","colony_id":"00000000-0000-0000-0000-000000000001","body_id":"00000000-0000-0000-0000-000000000002"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command: ClientCommand::AssignColonyHomeBody { colony_id, body_id },
            } => {
                assert_eq!(seq, 8);
                assert_eq!(colony_id, "00000000-0000-0000-0000-000000000001");
                assert_eq!(body_id, "00000000-0000-0000-0000-000000000002");
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn client_command_set_active_recipe_deserialises() {
        let raw = r#"{"type":"command","seq":9,"command":{"kind":"set_active_recipe","colony_id":"00000000-0000-0000-0000-000000000001","building_type":"refinery","recipe_id":"refine_conductive_ore"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::SetActiveRecipe {
                        colony_id,
                        building_type,
                        recipe_id,
                    },
            } => {
                assert_eq!(seq, 9);
                assert_eq!(colony_id, "00000000-0000-0000-0000-000000000001");
                assert_eq!(building_type, "refinery");
                assert_eq!(recipe_id, "refine_conductive_ore");
            }
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
    fn client_command_demolish_infrastructure_deserialises() {
        let raw = r#"{"type":"command","seq":9,"command":{"kind":"demolish_infrastructure","from_colony":"00000000-0000-0000-0000-000000000001","to_colony":"00000000-0000-0000-0000-000000000002"}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::DemolishInfrastructure {
                        from_colony,
                        to_colony,
                    },
            } => {
                assert_eq!(seq, 9);
                assert_eq!(from_colony, "00000000-0000-0000-0000-000000000001");
                assert_eq!(to_colony, "00000000-0000-0000-0000-000000000002");
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
                        body_id,
                    },
            } => {
                assert_eq!(seq, 8);
                assert_eq!(blueprint_id, "relay_station");
                assert_eq!(orbit_type, "low");
                assert_eq!(body_id, None);
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
                command:
                    ClientCommand::SetManualOverride {
                        colony_id: _,
                        enabled,
                    },
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
    fn new_game_command_deserialises() {
        let raw = r#"{"type":"command","seq":10,"command":{"kind":"new_game","difficulty":"Normal","planet_seed":42}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                seq,
                command:
                    ClientCommand::NewGame {
                        difficulty,
                        planet_seed,
                        system_seed,
                        ..
                    },
            } => {
                assert_eq!(seq, 10);
                assert_eq!(difficulty, DifficultyPreset::Normal);
                assert_eq!(planet_seed, 42);
                // Omitted in the payload — defaults to None, resolved to
                // planet_seed by the caller (issue #199).
                assert_eq!(system_seed, None);
            }
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn new_game_command_with_explicit_system_seed_deserialises() {
        let raw = r#"{"type":"command","seq":11,"command":{"kind":"new_game","difficulty":"Hard","planet_seed":1,"system_seed":99}}"#;
        let msg: ClientMessage = serde_json::from_str(raw).expect("parse");
        match msg {
            ClientMessage::Command {
                command: ClientCommand::NewGame { system_seed, .. },
                ..
            } => assert_eq!(system_seed, Some(99)),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn new_game_snapshot_serialises() {
        let snap = WorldSnapshot {
            sol: 0,
            month: 0,
            colonies: vec![],
        };
        let msg = ServerMessage::NewGameSnapshot {
            seq: 10,
            state: snap,
        };
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"type\":\"new_game_snapshot\""));
        assert!(json.contains("\"seq\":10"));
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

    /// Verify that core events which previously fell through to the wildcard
    /// `ColonySolAdvanced { sol: 0 }` arm now map to `Ignored` and do NOT
    /// produce a `colony_sol_advanced` payload that would corrupt the sol counter.
    #[test]
    fn wildcard_events_do_not_corrupt_sol_counter() {
        use outpost_core::hazard::HazardKind;

        // HazardOccurred must NOT become ColonySolAdvanced
        let hazard_event = Event::HazardOccurred {
            colony_id: uuid::Uuid::new_v4(),
            kind: HazardKind::DustStorm,
            severity: 0.5,
            stability_delta: -0.1,
            commodity_losses: vec![],
            population_lost: 0.0,
        };
        let se = ServerEvent::from_core(&hazard_event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(
            !json.contains("\"kind\":\"colony_sol_advanced\""),
            "HazardOccurred must not map to colony_sol_advanced: {json}"
        );
        assert!(
            json.contains("\"kind\":\"hazard_occurred\""),
            "expected hazard_occurred: {json}"
        );

        // MigrationArrived must NOT become ColonySolAdvanced
        let migration_event = Event::MigrationArrived {
            from_colony: None,
            to_colony: uuid::Uuid::new_v4(),
            count: 50.0,
            overcrowding_stability_penalty: -0.05,
            forced_departure_stability_penalty: 0.0,
        };
        let se = ServerEvent::from_core(&migration_event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(
            !json.contains("\"kind\":\"colony_sol_advanced\""),
            "MigrationArrived must not map to colony_sol_advanced: {json}"
        );

        // VictoryAchieved must NOT become ColonySolAdvanced
        let victory_event = Event::VictoryAchieved {
            condition: outpost_core::victory::VictoryCondition::InterstellarExpeditionLaunched,
        };
        let se = ServerEvent::from_core(&victory_event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(
            !json.contains("\"kind\":\"colony_sol_advanced\""),
            "VictoryAchieved must not map to colony_sol_advanced: {json}"
        );
        assert!(
            json.contains("\"kind\":\"victory_achieved\""),
            "expected victory_achieved: {json}"
        );

        // ExpeditionLost must NOT become ColonySolAdvanced
        let expedition_lost_event = Event::ExpeditionLost {
            expedition_id: outpost_core::expedition::FieldExpeditionId::new(),
        };
        let se = ServerEvent::from_core(&expedition_lost_event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(
            !json.contains("\"kind\":\"colony_sol_advanced\""),
            "ExpeditionLost must not map to colony_sol_advanced: {json}"
        );

        // MenaceCritical must NOT become ColonySolAdvanced
        let menace_event = Event::MenaceCritical {
            kind: outpost_core::menace::MenaceKind::ResourceDepletion,
            level: 0.9,
            countdown_months: 3,
        };
        let se = ServerEvent::from_core(&menace_event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(
            !json.contains("\"kind\":\"colony_sol_advanced\""),
            "MenaceCritical must not map to colony_sol_advanced: {json}"
        );
        assert!(
            json.contains("\"kind\":\"menace_critical\""),
            "expected menace_critical: {json}"
        );
    }

    /// Events that have no frontend representation map to `Ignored`, not to a
    /// fake `ColonySolAdvanced { sol: 0 }`.
    #[test]
    fn unrepresented_events_become_ignored_not_sol_zero() {
        // ResearchStarted has no dedicated ServerEvent variant; it should be Ignored.
        let event = Event::ResearchStarted {
            tech_id: "tech_fusion".into(),
        };
        let se = ServerEvent::from_core(&event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(
            !json.contains("\"kind\":\"colony_sol_advanced\""),
            "ResearchStarted must not map to colony_sol_advanced: {json}"
        );
        assert!(
            json.contains("\"kind\":\"ignored\""),
            "expected ignored: {json}"
        );
    }

    /// TechUnlocked serialises with the correct kind tag.
    #[test]
    fn tech_unlocked_serialises() {
        let event = Event::TechUnlocked {
            tech_id: "tech_hab_dome".into(),
        };
        let se = ServerEvent::from_core(&event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(json.contains("\"kind\":\"tech_unlocked\""), "json: {json}");
        assert!(json.contains("tech_hab_dome"), "json: {json}");
    }

    /// ExpeditionLaunched serialises with hex coordinates.
    #[test]
    fn expedition_launched_serialises() {
        use outpost_core::{expedition::FieldExpeditionId, map::HexCoord};
        let exp_id = FieldExpeditionId::new();
        let colony_id = uuid::Uuid::new_v4();
        let event = Event::ExpeditionLaunched {
            expedition_id: exp_id.clone(),
            colony_id,
            target_hex: HexCoord { q: 3, r: -2 },
        };
        let se = ServerEvent::from_core(&event);
        let json = serde_json::to_string(&se).expect("serialize");
        assert!(
            json.contains("\"kind\":\"expedition_launched\""),
            "json: {json}"
        );
        assert!(json.contains("\"target_hex_q\":3"), "json: {json}");
        assert!(json.contains("\"target_hex_r\":-2"), "json: {json}");
    }
}
