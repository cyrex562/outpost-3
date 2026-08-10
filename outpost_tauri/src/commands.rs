//! Tauri command handlers.
//!
//! Uses shim `ClientCommand` / `ClientQuery` / `ServerEvent` types that mirror
//! the wire format the Vue frontend already speaks. Each handler translates
//! the shim type into a core [`Command`] / [`Query`], applies it, and returns
//! the results in the same wire shape.

use include_dir::{include_dir, Dir};
use outpost_core::colony::ColonyId;
use outpost_core::content::loader::PackLoader;
use outpost_core::difficulty::DifficultyPreset;
use outpost_core::modifier::{DifficultyScalar, ModifiableQuantity};
use outpost_core::morale::MoraleConfig;
use outpost_core::needs::NeedsConfig;
use outpost_core::snapshot::Snapshot as SnapshotDb;
use outpost_core::system::{BodyKind, SystemCommand, SystemRole};
use outpost_core::tech::load_tech_registry;
use outpost_core::trade::SiteId;
use outpost_core::{Command, Event, GameEngine, Query, QueryResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::state::EngineState;

/// Embedded content pack shipped inside the binary. Path is relative to this crate's Cargo.toml.
static EMBEDDED_PACK: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../content/base");

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CmdError {
    #[error("engine not initialised — call bootstrap first")]
    NotInitialised,
    #[error("engine error: {0}")]
    Engine(String),
    #[error("content load error: {0}")]
    Content(String),
    #[error("snapshot error: {0}")]
    Snapshot(String),
    #[error("invalid argument: {0}")]
    InvalidArg(String),
}

// Serialise as the Display string. Deriving `Serialize` on the enum makes
// tauri reject `invoke` with a structured object (e.g. `{"Snapshot": "..."}`)
// which the frontend then stringifies to `[object Object]`. Emitting the
// human-readable form keeps every reject path legible in the UI.
impl Serialize for CmdError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl From<outpost_core::EngineError> for CmdError {
    fn from(e: outpost_core::EngineError) -> Self {
        Self::Engine(e.to_string())
    }
}

type CmdResult<T> = Result<T, CmdError>;

// ── Wire types (mirror the frontend's `Command` / `GameEvent`) ────────────────

/// Client → engine command. Matches the frontend's `Command` discriminated union.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientCommand {
    AdvanceSol,
    /// Advance up to `max_sols` sols, halting on an interrupt at or above
    /// `threshold` (issue #332). `threshold` is a tier slug: `ambient`,
    /// `notable`, `urgent`, or `blocking`.
    FastForward {
        max_sols: u32,
        threshold: String,
    },
    FoundColony {
        name: String,
        starting_population: u64,
    },
    QueueConstruction {
        colony_id: String,
        building_type: String,
        slot_cost: u32,
        labor_per_turn: u32,
        construction_cost: Vec<(String, f64)>,
        construction_turns: u32,
    },
    AssignLabour {
        colony_id: String,
        slot: String,
        labour: u64,
    },
    /// Select which recipe a building type runs in this colony (issue #166).
    SetActiveRecipe {
        colony_id: String,
        building_type: String,
        recipe_id: String,
    },
    /// Withhold a quantity of one commodity from industry (issue #308).
    SetCommodityReserve {
        colony_id: String,
        commodity_id: String,
        amount: f64,
    },
    /// Set one placed building's staffing priority (issue #307).
    ///
    /// Instance-scoped, unlike `SetActiveRecipe` above, which applies to every
    /// building of a type.
    SetBuildingPriority {
        colony_id: String,
        building_id: String,
        priority: u8,
    },
    /// Pin (`Some`) or release (`None`) a placed building's labour (issue #307).
    SetBuildingLabourLock {
        colony_id: String,
        building_id: String,
        lock: Option<u32>,
    },
    /// Name a placed building, or clear it back to the auto-numbered default
    /// (issue #307).
    RenameBuilding {
        colony_id: String,
        building_id: String,
        name: Option<String>,
    },
    /// Pause or resume a placed building (issue #309).
    ///
    /// A paused building is excluded from production entirely (no labour, power,
    /// or commodity draw) but keeps its build slot — pausing is not demolition.
    SetBuildingPaused {
        colony_id: String,
        building_id: String,
        paused: bool,
    },
    /// Cancel a queued construction project and receive a 50% partial refund.
    CancelConstruction {
        colony_id: String,
        project_id: String,
    },
    /// Place a batch of starter buildings instantly at colony founding,
    /// bypassing the normal construction queue (issue: playtest feedback
    /// round 2 — "lander" mechanic).
    DeployStarterKit {
        colony_id: String,
        buildings: Vec<(String, u32)>,
    },
    ResearchTech {
        tech_id: String,
    },
    EnqueueResearch {
        tech_id: String,
    },
    CancelResearch,
    /// Retune one balance scalar live (playtesting).
    SetBalanceScalar {
        /// Stable slug of the quantity.
        quantity: String,
        /// New multiplier.
        value: f32,
    },
    FoundColonyAtSite {
        name: String,
        starting_population: u64,
        site_id: String,
        focus: Option<String>,
        #[serde(default)]
        supplies_id: Option<String>,
        /// Explicit per-commodity starter-supply amounts (issue #167).
        /// When present, overrides the `supplies_id` package scaling.
        #[serde(default)]
        supply_overrides: Option<Vec<(String, f64)>>,
        /// Colony funding this settlement (issue #359). The sponsor pays the
        /// `colony` cost profile and gives up the colonists. `None` means
        /// nobody pays — correct only for a game's first colony.
        #[serde(default)]
        sponsor_colony_id: Option<String>,
        /// Star-system body this site belongs to (issue #183). `None` skips
        /// the habitability gate, preserving pre-#183 client behaviour.
        #[serde(default)]
        body_id: Option<String>,
    },
    /// Set an active preset (built-in) mid-game.
    SetDifficulty {
        preset: String,
    },
    /// Install a custom scalar map + toggles atomically (issue #161, #180).
    SetCustomDifficulty {
        /// Slider values keyed by knob id (see [`get_difficulty_knobs`]).
        scalars: std::collections::HashMap<String, f32>,
        menace_enabled: bool,
        hazards_enabled: bool,
        /// Master maintenance toggle (issue #180). Pre-#180 clients that omit
        /// this default to `true` so authored building upkeep still applies.
        #[serde(default)]
        maintenance_enabled: Option<bool>,
        /// Master deposit-depletion toggle (issue #317). Clients that omit
        /// this default to `false` (deposits stay infinite).
        #[serde(default)]
        deposit_depletion_enabled: Option<bool>,
    },
    /// Master hazards toggle (issue #161).
    SetHazardsEnabled {
        enabled: bool,
    },
    /// Master maintenance toggle (issue #180).
    SetMaintenanceEnabled {
        enabled: bool,
    },
    /// Master deposit-depletion toggle (issue #317).
    SetDepositDepletionEnabled {
        enabled: bool,
    },
    /// Link a colony to its home star-system body (issue #163).
    AssignColonyHomeBody {
        colony_id: String,
        body_id: String,
    },
    /// Establish a new outpost anchored to a body (issue #233/#243).
    EstablishOutpost {
        name: String,
        colony_id: String,
        body_id: String,
    },
    /// Decommission an outpost, removing it from play (issue #233/#243).
    DecommissionOutpost { outpost_id: String },
    /// Queue a construction project at an outpost (issue #233/#243).
    QueueOutpostConstruction {
        outpost_id: String,
        building_type: String,
        slot_cost: u32,
        labor_per_turn: u32,
        construction_cost: Vec<(String, f64)>,
        construction_turns: u32,
    },
    /// Promote an established outpost into a full colony (issue #242/#243).
    PromoteOutpostToColony {
        outpost_id: String,
        name: String,
        starting_population: u64,
    },
    /// Select which recipe a building type runs at an outpost (navigation
    /// rework #7 phase 4 — mirrors `SetActiveRecipe` for colonies).
    SetOutpostActiveRecipe {
        outpost_id: String,
        building_type: String,
        recipe_id: String,
    },
    /// Build an infrastructure edge between two colonies (map/nav plan phase
    /// A3b). `infra_type` is `"road"`, `"rail"`, `"pipeline"`, or
    /// `"powerline"` (issue #383).
    BuildInfrastructure {
        from_colony: String,
        to_colony: String,
        infra_type: String,
    },
    /// Remove the infrastructure edge between two colonies (map/nav plan
    /// phase A3b).
    DemolishInfrastructure {
        from_colony: String,
        to_colony: String,
    },
    /// Add a manually-created trade route between two colonies (issue #363).
    ///
    /// Unlike infrastructure-linked routes, this works between colonies on
    /// different bodies — the engine derives transit time from body
    /// separation the same way either kind of route is created.
    AddTradeRoute {
        colony_a: String,
        colony_b: String,
        throughput_cap: f64,
    },
    /// Remove a trade route by its id (issue #363).
    RemoveTradeRoute { route_id: String },
}

/// Read-only query. Matches the frontend's query message.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientQuery {
    CurrentSol,
    CurrentMonth,
    ListColonies,
    ColonyStatus {
        colony_id: String,
    },
    ColonyScreen {
        colony_id: String,
    },
    SystemResearchTotal,
    /// Full detail for one building type within a colony (issue #182).
    BuildingDetail {
        colony_id: String,
        building_type: String,
    },
    /// Full detail for one building type within an outpost (navigation
    /// rework #7 phase 4).
    OutpostBuildingDetail {
        outpost_id: String,
        building_type: String,
    },
}

/// Wire event returned to the frontend. Matches `GameEvent` (snake_case, `kind`-tagged).
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerEvent {
    ColonySolAdvanced {
        sol: u64,
    },
    StrategicMonthAdvanced {
        month: u64,
    },
    ColonyFounded {
        colony_id: String,
        name: String,
        starting_population: u64,
    },
    ConstructionQueued {
        colony_id: String,
        building_type: String,
        project_id: String,
    },
    ConstructionCancelled {
        colony_id: String,
        project_id: String,
        refund: Vec<(String, f64)>,
    },
    BuildingConstructed {
        colony_id: String,
        building_type: String,
    },
    /// A site-preparation project widened build-slot capacity (issue #306).
    SlotCapacityExpanded {
        colony_id: String,
        building_type: String,
        added: u32,
        slot_capacity: u32,
    },
    /// An outpost's site preparation widened its build-slot capacity.
    OutpostSlotCapacityExpanded {
        outpost_id: String,
        building_type: String,
        added: u32,
        slot_capacity: u32,
    },
    /// Construction blocked by the player's own commodity reserve (issue #355).
    ConstructionStalledByReserve {
        colony_id: String,
        project_id: String,
        building_type: String,
        withheld: Vec<(String, f64)>,
    },
    /// Construction made no progress for want of materials (issue #306).
    ConstructionStalled {
        colony_id: String,
        project_id: String,
        building_type: String,
        missing: Vec<(String, f64)>,
    },
    /// An outpost's construction made no progress for want of materials.
    OutpostConstructionStalled {
        outpost_id: String,
        project_id: String,
        building_type: String,
        missing: Vec<(String, f64)>,
    },
    LabourAssigned {
        colony_id: String,
        slot: String,
        labour: u64,
    },
    /// A building type's active recipe was set (issue #166).
    ActiveRecipeSet {
        colony_id: String,
        building_type: String,
        recipe_id: String,
    },
    NeedsResolved {
        colony_id: String,
        composite_satisfaction: f32,
        stability_delta: f32,
        population_delta: f32,
    },
    ResearchProduced {
        colony_id: String,
        amount: f32,
    },
    ProductionShortfall {
        colony_id: String,
        building_type: String,
        scale: f64,
        reason: String,
    },
    /// The active difficulty preset changed (issue #161).
    DifficultyChanged {
        preset: String,
    },
    /// A colony's home body was recorded (issue #163).
    ColonyHomeBodySet {
        colony_id: String,
        body_id: String,
        habitability_modifier: f32,
    },
    /// A new outpost was established (issue #233/#243).
    OutpostEstablished {
        outpost_id: String,
        colony_id: String,
        body_id: String,
    },
    /// An outpost was decommissioned (issue #233/#243).
    OutpostDecommissioned { outpost_id: String },
    /// An outpost queued a construction project (issue #233/#243).
    OutpostConstructionQueued {
        outpost_id: String,
        building_type: String,
        project_id: String,
    },
    /// An outpost was promoted into a full colony (issue #242/#243).
    OutpostPromoted {
        outpost_id: String,
        colony_id: String,
        name: String,
    },
    /// An outpost building type's active recipe was set (navigation rework
    /// #7 phase 4 — mirrors `ActiveRecipeSet` for colonies).
    OutpostActiveRecipeSet {
        outpost_id: String,
        building_type: String,
        recipe_id: String,
    },
    /// An inter-colony trade convoy landed and was credited to a colony pool
    /// (issue #332).
    TradeConvoyArrived {
        convoy_id: String,
        from_colony: String,
        to_colony: String,
        commodity_id: String,
        amount: f64,
    },
    /// A fast-forward run finished (issue #332).
    ///
    /// The UI's cue to stop its play timer and, when `halted`, open the digest
    /// panel.
    FastForwardEnded {
        sol: u64,
        sols_advanced: u32,
        halted: bool,
        halting_reason: Option<String>,
    },
    /// A trade route was added to the planetary trade network (issue #363).
    TradeRouteAdded {
        route_id: String,
        colony_a: String,
        colony_b: String,
        throughput_cap: f64,
    },
    /// A trade route was removed from the planetary trade network (issue #363).
    TradeRouteRemoved { route_id: String },
    /// Fallback for events we don't have a typed variant for yet.
    Unknown {
        core_kind: String,
    },
}

impl ServerEvent {
    fn from_core(event: &Event) -> Self {
        match event {
            Event::ColonySolAdvanced { sol } => Self::ColonySolAdvanced { sol: *sol },
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
            // Semantically equivalent for UI purposes — surface the same
            // `colony_founded` wire shape so the frontend reducer + wizard
            // handler don't need a parallel code path.
            Event::ColonyFoundedAtSite {
                colony_id,
                name,
                starting_population,
                ..
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
            Event::SlotCapacityExpanded {
                colony_id,
                building_type,
                added,
                slot_capacity,
            } => Self::SlotCapacityExpanded {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
                added: *added,
                slot_capacity: *slot_capacity,
            },
            Event::OutpostSlotCapacityExpanded {
                outpost_id,
                building_type,
                added,
                slot_capacity,
            } => Self::OutpostSlotCapacityExpanded {
                outpost_id: outpost_id.to_string(),
                building_type: building_type.clone(),
                added: *added,
                slot_capacity: *slot_capacity,
            },
            Event::ConstructionStalledByReserve {
                colony_id,
                project_id,
                building_type,
                withheld,
            } => Self::ConstructionStalledByReserve {
                colony_id: colony_id.to_string(),
                project_id: project_id.to_string(),
                building_type: building_type.clone(),
                withheld: withheld.clone(),
            },
            Event::ConstructionStalled {
                colony_id,
                project_id,
                building_type,
                missing,
            } => Self::ConstructionStalled {
                colony_id: colony_id.to_string(),
                project_id: project_id.to_string(),
                building_type: building_type.clone(),
                missing: missing.clone(),
            },
            Event::OutpostConstructionStalled {
                outpost_id,
                project_id,
                building_type,
                missing,
            } => Self::OutpostConstructionStalled {
                outpost_id: outpost_id.to_string(),
                project_id: project_id.to_string(),
                building_type: building_type.clone(),
                missing: missing.clone(),
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
            Event::ActiveRecipeSet {
                colony_id,
                building_type,
                recipe_id,
            } => Self::ActiveRecipeSet {
                colony_id: colony_id.to_string(),
                building_type: building_type.clone(),
                recipe_id: recipe_id.clone(),
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
            Event::DifficultyChanged { preset } => Self::DifficultyChanged {
                preset: preset.label().to_owned(),
            },
            Event::ColonyHomeBodySet {
                colony_id,
                body_id,
                habitability_modifier,
            } => Self::ColonyHomeBodySet {
                colony_id: colony_id.to_string(),
                body_id: body_id.0.to_string(),
                habitability_modifier: *habitability_modifier,
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
            Event::OutpostActiveRecipeSet {
                outpost_id,
                building_type,
                recipe_id,
            } => Self::OutpostActiveRecipeSet {
                outpost_id: outpost_id.to_string(),
                building_type: building_type.clone(),
                recipe_id: recipe_id.clone(),
            },
            Event::TradeRouteAdded {
                route_id,
                colony_a,
                colony_b,
                throughput_cap,
            } => Self::TradeRouteAdded {
                route_id: route_id.to_string(),
                colony_a: colony_a.to_string(),
                colony_b: colony_b.to_string(),
                throughput_cap: *throughput_cap,
            },
            Event::TradeRouteRemoved { route_id } => Self::TradeRouteRemoved {
                route_id: route_id.to_string(),
            },
            other => Self::Unknown {
                core_kind: format!("{other:?}")
                    .split_whitespace()
                    .next()
                    .unwrap_or("event")
                    .to_owned(),
            },
        }
    }
}

/// Initial snapshot returned by `bootstrap` or `snapshot`.
#[derive(Debug, Serialize)]
pub struct SnapshotPayload {
    pub sol: u64,
    pub month: u64,
    pub colonies: Vec<ColonyWire>,
    pub research_total: f32,
}

#[derive(Debug, Serialize)]
pub struct ColonyWire {
    pub id: String,
    pub name: String,
    pub population: f32,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_colony(id: &str) -> Result<ColonyId, CmdError> {
    ColonyId::from_str(id).map_err(|_| CmdError::InvalidArg(format!("bad colony id: {id}")))
}

/// Parse a placed-building instance id (issue #307).
fn parse_building(id: &str) -> Result<uuid::Uuid, CmdError> {
    uuid::Uuid::from_str(id).map_err(|_| CmdError::InvalidArg(format!("bad building id: {id}")))
}

/// Parse an interrupt-tier slug into [`outpost_core::interrupt::Tier`]
/// (issue #332). Mirrors the `outpost_web` WS mapping.
///
/// Rejects an unknown slug rather than defaulting: silently falling back would
/// turn a typo'd cautious fast-forward into one that runs through a crisis.
fn parse_tier(s: &str) -> Result<outpost_core::interrupt::Tier, CmdError> {
    use outpost_core::interrupt::Tier;
    match s.to_lowercase().as_str() {
        "ambient" => Ok(Tier::Ambient),
        "notable" => Ok(Tier::Notable),
        "urgent" => Ok(Tier::Urgent),
        "blocking" => Ok(Tier::Blocking),
        other => Err(CmdError::InvalidArg(format!(
            "unknown interrupt tier: {other}; expected ambient, notable, urgent, or blocking"
        ))),
    }
}

/// Parse an infrastructure-type string (`road`/`rail`/`pipeline`/`powerline`)
/// into [`outpost_core::map::InfraType`] (map/nav plan phase A3b; powerline
/// added by issue #383). Mirrors the `outpost_web` WS mapping.
fn parse_infra_type(s: &str) -> Result<outpost_core::map::InfraType, CmdError> {
    use outpost_core::map::InfraType;
    match s.to_lowercase().as_str() {
        "road" => Ok(InfraType::Road),
        "rail" => Ok(InfraType::Rail),
        "pipeline" => Ok(InfraType::Pipeline),
        "powerline" => Ok(InfraType::Powerline),
        _ => Err(CmdError::InvalidArg(format!("unknown infra_type: {s}"))),
    }
}

fn build_snapshot(engine: &GameEngine) -> SnapshotPayload {
    let state = &engine.state;
    let colonies = state
        .colonies
        .iter()
        .zip(state.populations.iter())
        .map(|(c, p)| ColonyWire {
            id: c.id.to_string(),
            name: c.name.clone(),
            population: p.count,
        })
        .collect();
    SnapshotPayload {
        sol: state.sol,
        month: state.month,
        colonies,
        research_total: state.research_pool.total,
    }
}

fn load_embedded_content() -> Result<outpost_core::content::registry::ContentRegistry, CmdError> {
    let mut raw: Vec<(&str, &str)> = Vec::new();
    for file in EMBEDDED_PACK.files() {
        if file.path().extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let name = file
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let text = file
            .contents_utf8()
            .ok_or_else(|| CmdError::Content(format!("non-utf8 embedded file: {name}")))?;
        raw.push((name, text));
    }
    PackLoader::load(&raw).map_err(|e| CmdError::Content(e.to_string()))
}

fn load_content(
    content_dir: &Path,
) -> Result<outpost_core::content::registry::ContentRegistry, CmdError> {
    let mut raw_owned: Vec<(String, String)> = Vec::new();
    let entries = std::fs::read_dir(content_dir).map_err(|e| CmdError::Content(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| CmdError::Content(e.to_string()))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_owned();
            let text =
                std::fs::read_to_string(&path).map_err(|e| CmdError::Content(e.to_string()))?;
            raw_owned.push((name, text));
        }
    }
    let raw: Vec<(&str, &str)> = raw_owned
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    PackLoader::load(&raw).map_err(|e| CmdError::Content(e.to_string()))
}

fn parse_preset(difficulty: &str) -> DifficultyPreset {
    match difficulty {
        "Sandbox" | "sandbox" => DifficultyPreset::Sandbox,
        "Easy" | "easy" => DifficultyPreset::Easy,
        "Hard" | "hard" => DifficultyPreset::Hard,
        "Brutal" | "brutal" => DifficultyPreset::Brutal,
        "Custom" | "custom" => DifficultyPreset::Custom,
        _ => DifficultyPreset::Normal,
    }
}

// ── #161: knob spec + custom-preset persistence ─────────────────────────────

/// One row in the custom-difficulty panel. Sliders and toggles are unified so
/// the frontend can render every knob from a single list.
///
/// `preset_default_at_current_preset` seeds the panel from the currently
/// selected built-in preset so "Custom" starts as a copy of that preset.
#[derive(Debug, Serialize)]
pub struct KnobSpec {
    pub id: String,
    pub label: String,
    /// `"slider"` or `"toggle"`.
    pub kind: &'static str,
    /// Slider step values (empty for toggles).
    pub step: Vec<f32>,
    pub min: f32,
    pub max: f32,
    /// Value at the currently active preset (baseline reference).
    pub preset_default_at_current_preset: f32,
    /// The engine's current effective value (post-Custom / post-toggle).
    pub current_value: f32,
}

/// Canonical order of the 11 slider knobs + 2 toggles surfaced to the UI.
///
/// The `.0` is the wire id used both in [`KnobSpec::id`] and in the
/// [`ClientCommand::SetCustomDifficulty::scalars`] map keys.
const SLIDER_KNOBS: &[(&str, &str, ModifiableQuantityKey)] = &[
    (
        "production_rate",
        "Production Rate",
        ModifiableQuantityKey::ProductionRateAll,
    ),
    (
        "labor_efficiency",
        "Labor Efficiency",
        ModifiableQuantityKey::LaborEfficiency,
    ),
    (
        "research_rate",
        "Research Rate",
        ModifiableQuantityKey::ResearchRate,
    ),
    (
        "storage_capacity",
        "Storage Capacity",
        ModifiableQuantityKey::StorageCapacity,
    ),
    (
        "population_growth",
        "Population Growth",
        ModifiableQuantityKey::PopulationGrowth,
    ),
    (
        "stability_decay",
        "Stability Decay",
        ModifiableQuantityKey::StabilityRate,
    ),
    (
        "hazard_probability",
        "Hazard Probability",
        ModifiableQuantityKey::HazardProbability,
    ),
    (
        "construction_cost",
        "Construction Cost",
        ModifiableQuantityKey::ConstructionCost,
    ),
    (
        "resource_consumption",
        "Resource Consumption",
        ModifiableQuantityKey::ResourceConsumption,
    ),
    (
        "research_cost",
        "Research Cost",
        ModifiableQuantityKey::ResearchCost,
    ),
    (
        "power_requirement",
        "Power Requirement",
        ModifiableQuantityKey::PowerRequirement,
    ),
    (
        "maintenance_consumption",
        "Maintenance Consumption",
        ModifiableQuantityKey::MaintenanceConsumption,
    ),
];

/// Fixed anchor slider steps shared by every knob (issue #161 design decision).
const SLIDER_STEPS: [f32; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

#[derive(Debug, Clone, Copy)]
enum ModifiableQuantityKey {
    ProductionRateAll,
    LaborEfficiency,
    ResearchRate,
    StorageCapacity,
    PopulationGrowth,
    StabilityRate,
    HazardProbability,
    ConstructionCost,
    ResourceConsumption,
    ResearchCost,
    PowerRequirement,
    MaintenanceConsumption,
}

impl ModifiableQuantityKey {
    fn to_quantity(self) -> ModifiableQuantity {
        match self {
            Self::ProductionRateAll => ModifiableQuantity::ProductionRate("*".into()),
            Self::LaborEfficiency => ModifiableQuantity::LaborEfficiency,
            Self::ResearchRate => ModifiableQuantity::ResearchRate,
            Self::StorageCapacity => ModifiableQuantity::StorageCapacity,
            Self::PopulationGrowth => ModifiableQuantity::PopulationGrowth,
            Self::StabilityRate => ModifiableQuantity::StabilityRate,
            Self::HazardProbability => ModifiableQuantity::HazardProbability,
            Self::ConstructionCost => ModifiableQuantity::ConstructionCost,
            Self::ResourceConsumption => ModifiableQuantity::ResourceConsumption,
            Self::ResearchCost => ModifiableQuantity::ResearchCost,
            Self::PowerRequirement => ModifiableQuantity::PowerRequirement,
            Self::MaintenanceConsumption => ModifiableQuantity::MaintenanceConsumption,
        }
    }
}

/// Assemble a [`DifficultyScalar`] from a knob-id → value map submitted by the UI.
fn scalars_from_map(map: &std::collections::HashMap<String, f32>) -> DifficultyScalar {
    let mut out = DifficultyScalar::new();
    for (id, _, key) in SLIDER_KNOBS {
        if let Some(v) = map.get(*id) {
            out.set(key.to_quantity(), *v);
        }
    }
    out
}

/// Wire form of a saved custom-difficulty preset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPreset {
    pub name: String,
    /// Knob-id → value.
    pub scalars: std::collections::HashMap<String, f32>,
    pub menace_enabled: bool,
    pub hazards_enabled: bool,
    /// Master maintenance toggle (issue #180). Pre-#180 preset YAMLs default
    /// to `true` so authored building upkeep still applies after upgrade.
    #[serde(default = "yes")]
    pub maintenance_enabled: bool,
}

fn yes() -> bool {
    true
}

/// Directory containing one .yaml per saved preset.
fn custom_preset_dir(app: &AppHandle) -> Result<PathBuf, CmdError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| CmdError::Snapshot(format!("no app data dir: {e}")))?
        .join("custom_difficulty");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    }
    Ok(dir)
}

fn sanitize_preset_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Boot a fresh engine: load content, set difficulty, seed the planet map.
///
/// If `content_dir` is empty (or literally `"embedded"`), the pack embedded
/// in the binary at build time is used. Otherwise `content_dir` is read
/// from the filesystem.
#[tauri::command]
// Tauri IPC commands take flat scalar arguments (no struct-of-options
// aggregation across the JS/Rust boundary in this codebase's convention);
// issue #318 added 9 more New Game generation overrides on top of an
// already-large parameter list inherited from issue #199/#161.
#[allow(clippy::too_many_arguments)]
pub fn bootstrap(
    content_dir: String,
    planet_seed: u64,
    difficulty: String,
    #[allow(non_snake_case)] custom_scalars: Option<std::collections::HashMap<String, f32>>,
    custom_menace_enabled: Option<bool>,
    custom_hazards_enabled: Option<bool>,
    custom_maintenance_enabled: Option<bool>,
    system_seed: Option<u64>,
    habitable_zone_center_au: Option<f32>,
    min_inner_planets: Option<u32>,
    max_inner_planets: Option<u32>,
    abundance_scalar_override: Option<f32>,
    // Gas-giant/asteroid-belt/cometary-belt/moon count overrides (issue #318).
    min_gas_giants: Option<u32>,
    max_gas_giants: Option<u32>,
    min_asteroid_belts: Option<u32>,
    max_asteroid_belts: Option<u32>,
    min_cometary_belts: Option<u32>,
    max_cometary_belts: Option<u32>,
    min_giant_moons: Option<u32>,
    max_giant_moons: Option<u32>,
    max_rocky_moons: Option<u32>,
    engine_state: State<'_, EngineState>,
) -> CmdResult<SnapshotPayload> {
    log::info!(
        "bootstrap: content_dir={content_dir:?} planet_seed={planet_seed} difficulty={difficulty:?} system_seed={system_seed:?}"
    );
    let registry = match (if content_dir.is_empty() || content_dir == "embedded" {
        load_embedded_content()
    } else {
        load_content(Path::new(&content_dir))
    }) {
        Ok(r) => r,
        Err(e) => {
            log::error!("bootstrap FAILED loading content: {e}");
            return Err(e);
        }
    };

    let mut engine = GameEngine::new();
    attach_content(&mut engine, registry);

    let _ = engine.apply(&Command::SetDifficulty {
        preset: parse_preset(&difficulty),
    });

    // Overlay a custom-difficulty payload if one was supplied (issue #161).
    if let Some(map) = custom_scalars {
        // No dedicated bootstrap parameter for this yet — preserve whatever
        // `SetDifficulty` above just derived from the preset (issue #317)
        // rather than silently resetting it to infinite.
        let deposit_depletion_enabled = engine.state.deposit_depletion_enabled;
        let _ = engine.apply(&Command::SetCustomDifficulty {
            scalars: scalars_from_map(&map),
            menace_enabled: custom_menace_enabled.unwrap_or(true),
            hazards_enabled: custom_hazards_enabled.unwrap_or(true),
            maintenance_enabled: custom_maintenance_enabled.unwrap_or(true),
            deposit_depletion_enabled,
        });
    }

    // Procedurally generate the star system (issue #199) — the bootstrap
    // default, replacing the content pack's authored systems.yaml scenarios
    // (still available separately via `seed_system_from_content` for a
    // future hand-authored-scenario picker). Independent seed from the
    // planet map by default; falls back to `planet_seed` when the caller
    // doesn't supply one, matching the only behavior a single-seed caller
    // could have meant.
    let abundance_scalar = abundance_scalar_override.unwrap_or_else(|| {
        engine
            .state
            .difficulty_scalar
            .scalar_for(&ModifiableQuantity::DepositAbundance)
    });
    let gen_defaults = outpost_core::system_gen::SystemGenParams::default();
    let _ = engine.apply(&Command::System(SystemCommand::GenerateSystem {
        seed: system_seed.unwrap_or(planet_seed),
        abundance_scalar,
        habitable_zone_center_au: habitable_zone_center_au
            .unwrap_or(gen_defaults.habitable_zone_center_au),
        min_inner_planets: min_inner_planets.unwrap_or(gen_defaults.min_inner_planets),
        max_inner_planets: max_inner_planets.unwrap_or(gen_defaults.max_inner_planets),
        min_gas_giants: min_gas_giants.unwrap_or(gen_defaults.min_gas_giants),
        max_gas_giants: max_gas_giants.unwrap_or(gen_defaults.max_gas_giants),
        min_asteroid_belts: min_asteroid_belts.unwrap_or(gen_defaults.min_asteroid_belts),
        max_asteroid_belts: max_asteroid_belts.unwrap_or(gen_defaults.max_asteroid_belts),
        min_cometary_belts: min_cometary_belts.unwrap_or(gen_defaults.min_cometary_belts),
        max_cometary_belts: max_cometary_belts.unwrap_or(gen_defaults.max_cometary_belts),
        min_giant_moons: min_giant_moons.unwrap_or(gen_defaults.min_giant_moons),
        max_giant_moons: max_giant_moons.unwrap_or(gen_defaults.max_giant_moons),
        max_rocky_moons: max_rocky_moons.unwrap_or(gen_defaults.max_rocky_moons),
    }));

    // `width`/`height` here are only the no-system fallback — since
    // `GenerateSystem` just ran above, the handler derives the real
    // dimensions from the designated home body's rolled size instead
    // (issue #314).
    let _ = engine.apply(&Command::SeedPlanet {
        seed: planet_seed,
        width: 22,
        height: 12,
    });

    log::info!(
        "bootstrap: tech_registry loaded={}",
        engine.state.tech_registry.is_some()
    );

    let snap = build_snapshot(&engine);
    log::info!(
        "bootstrap OK: sol={} colonies={}",
        snap.sol,
        snap.colonies.len()
    );
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}

/// Attach the runtime-only content fields to `engine`.
///
/// `GameState` deliberately does not persist these — they are content, not
/// state, and `SnapshotBlob::into_state` restores them as `None` with the
/// comment "reloaded from content packs after load". That contract makes every
/// caller that produces a `GameState` responsible for reattaching them, which
/// is exactly what `load_game` was failing to do: a loaded game came back with
/// no registry at all, which left `list_buildings` erroring (an empty build
/// dialog), `get_tech_tree` erroring, and — worse — the production step a
/// silent no-op, since it skips entirely when `registry` is `None`.
///
/// Bootstrap and load now share this so the two cannot drift again.
fn attach_content(
    engine: &mut GameEngine,
    registry: outpost_core::content::registry::ContentRegistry,
) {
    engine.state.registry = Some(registry);
    engine.state.needs_config = Some(NeedsConfig::default_survival());
    engine.state.morale_config = Some(MoraleConfig::default_config());
    load_embedded_tech(engine);
}

fn load_embedded_tech(engine: &mut GameEngine) {
    let Some(tech_file) = EMBEDDED_PACK.get_file("tech.yaml") else {
        return;
    };
    let Some(yaml) = tech_file.contents_utf8() else {
        return;
    };
    if let Ok(registry) = load_tech_registry(yaml) {
        engine.state.tech_registry = Some(registry);
    }
}

/// Seed `system_state.node_map` from the loaded content pack.
///
/// Picks the first `StarSystemDef` in the registry and applies one
/// [`SystemCommand::AddBody`] per authored body. No-op when the pack ships
/// no systems. Selection UI (letting the player pick which scenario)
/// can layer on later without changing this seeding path.
///
/// Assigned roles land on the body via a follow-up
/// [`SystemCommand::AssignRole`] since `AddBody` sets `Unassigned` by default.
///
/// Not called from the live `bootstrap` command as of issue #199 —
/// procedural generation (`Command::System(SystemCommand::GenerateSystem)`)
/// is the default there now. Kept (not deleted) because #199 explicitly
/// demotes `content/base/systems.yaml`'s authored scenarios to "optional,
/// selectable later" rather than removing them — this is the loader a
/// future scenario-picker command would call.
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
    // been added yet — content validation (issue #196) only guarantees the
    // name resolves *somewhere* in this system, not that it comes earlier.
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
        if !matches!(body.role, SystemRole::Unassigned) {
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

/// Return whether an engine has been bootstrapped.
#[tauri::command]
pub fn is_ready(engine_state: State<'_, EngineState>) -> bool {
    engine_state.engine.lock().unwrap().is_some()
}

/// Record a frontend (Vue/JS) error into the same log file the Rust side
/// writes to (`tauri_plugin_log`'s file target, see `lib.rs`).
///
/// The webview has no error boundary of its own, so an uncaught exception
/// during a component's render/setup can leave the screen blank with
/// nothing in the OS-level log to explain it. `main.ts` wires a global
/// `app.config.errorHandler` plus `window.onerror`/`unhandledrejection`
/// listeners that call this command so those failures land here instead of
/// vanishing silently.
#[tauri::command]
pub fn log_frontend_error(source: String, message: String, stack: Option<String>) {
    match stack {
        Some(stack) => log::error!("FRONTEND ERROR [{source}]: {message}\n{stack}"),
        None => log::error!("FRONTEND ERROR [{source}]: {message}"),
    }
}

/// Terminate the app process cleanly.
///
/// Using `AppHandle::exit(0)` instead of `WebviewWindow::close()` avoids the
/// v2 quirk where closing the main window doesn't necessarily reap the
/// process, and doesn't depend on the `core:window:close` capability.
#[tauri::command]
pub fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Return a fresh full snapshot of engine state.
#[tauri::command]
pub fn snapshot(engine_state: State<'_, EngineState>) -> CmdResult<SnapshotPayload> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    Ok(build_snapshot(engine))
}

/// Apply a single command (mirrors the WebSocket "command" message).
#[tauri::command]
pub fn apply_command(
    command: ClientCommand,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<ServerEvent>> {
    let command_debug = format!("{command:?}");
    log::debug!("apply_command: received {command_debug}");

    let mut guard = match engine_state.engine.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::error!(
                "apply_command: engine mutex POISONED (a prior command panicked mid-mutation) — recovering with the pre-panic state; command={command_debug}"
            );
            poisoned.into_inner()
        }
    };
    let engine = match guard.as_mut().ok_or(CmdError::NotInitialised) {
        Ok(e) => e,
        Err(e) => {
            log::error!("apply_command FAILED (not initialised): command={command_debug}");
            return Err(e);
        }
    };

    let core_cmd = match command {
        ClientCommand::AdvanceSol => Command::AdvanceColonySol,
        ClientCommand::FastForward {
            max_sols,
            threshold,
        } => Command::FastForward {
            max_sols,
            threshold: parse_tier(&threshold)?,
        },
        ClientCommand::FoundColony {
            name,
            starting_population,
        } => Command::FoundColony {
            name,
            starting_population,
        },
        ClientCommand::QueueConstruction {
            colony_id,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        } => Command::QueueConstruction {
            colony_id: parse_colony(&colony_id)?,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        },
        ClientCommand::AssignLabour {
            colony_id,
            slot,
            labour,
        } => Command::AssignLabour {
            colony_id: parse_colony(&colony_id)?,
            slot,
            labour,
        },
        ClientCommand::SetActiveRecipe {
            colony_id,
            building_type,
            recipe_id,
        } => Command::SetActiveRecipe {
            colony_id: parse_colony(&colony_id)?,
            building_type,
            recipe_id,
        },
        ClientCommand::SetCommodityReserve {
            colony_id,
            commodity_id,
            amount,
        } => Command::SetCommodityReserve {
            colony_id: parse_colony(&colony_id)?,
            commodity_id,
            amount,
        },
        ClientCommand::SetBuildingPriority {
            colony_id,
            building_id,
            priority,
        } => Command::SetBuildingPriority {
            colony_id: parse_colony(&colony_id)?,
            building_id: parse_building(&building_id)?,
            priority,
        },
        ClientCommand::SetBuildingLabourLock {
            colony_id,
            building_id,
            lock,
        } => Command::SetBuildingLabourLock {
            colony_id: parse_colony(&colony_id)?,
            building_id: parse_building(&building_id)?,
            lock,
        },
        ClientCommand::RenameBuilding {
            colony_id,
            building_id,
            name,
        } => Command::RenameBuilding {
            colony_id: parse_colony(&colony_id)?,
            building_id: parse_building(&building_id)?,
            name,
        },
        ClientCommand::SetBuildingPaused {
            colony_id,
            building_id,
            paused,
        } => Command::SetBuildingPaused {
            colony_id: parse_colony(&colony_id)?,
            building_id: parse_building(&building_id)?,
            paused,
        },
        ClientCommand::CancelConstruction {
            colony_id,
            project_id,
        } => Command::CancelConstruction {
            colony_id: parse_colony(&colony_id)?,
            project_id: Uuid::parse_str(&project_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad project_id: {project_id}")))?,
        },
        ClientCommand::DeployStarterKit {
            colony_id,
            buildings,
        } => Command::DeployStarterKit {
            colony_id: parse_colony(&colony_id)?,
            buildings,
        },
        ClientCommand::ResearchTech { tech_id } => Command::ResearchTech { tech_id },
        ClientCommand::EnqueueResearch { tech_id } => Command::EnqueueResearch { tech_id },
        ClientCommand::CancelResearch => Command::CancelResearch,
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
            let uuid = Uuid::parse_str(&site_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad site_id: {site_id}")))?;
            let body_id = body_id
                .map(|b| {
                    Uuid::parse_str(&b)
                        .map(outpost_core::system::BodyId)
                        .map_err(|_| CmdError::InvalidArg(format!("bad body_id: {b}")))
                })
                .transpose()?;
            let sponsor_colony_id = sponsor_colony_id
                .map(|c| {
                    ColonyId::from_str(&c)
                        .map_err(|_| CmdError::InvalidArg(format!("bad sponsor_colony_id: {c}")))
                })
                .transpose()?;
            Command::FoundColonyAtSite {
                name,
                starting_population,
                site_id: SiteId(uuid),
                focus,
                supplies_id,
                supply_overrides,
                sponsor_colony_id,
                body_id,
            }
        }
        ClientCommand::SetBalanceScalar { quantity, value } => {
            Command::SetBalanceScalar { quantity, value }
        }
        ClientCommand::SetDifficulty { preset } => Command::SetDifficulty {
            preset: parse_preset(&preset),
        },
        ClientCommand::SetCustomDifficulty {
            scalars,
            menace_enabled,
            hazards_enabled,
            maintenance_enabled,
            deposit_depletion_enabled,
        } => Command::SetCustomDifficulty {
            scalars: scalars_from_map(&scalars),
            menace_enabled,
            hazards_enabled,
            maintenance_enabled: maintenance_enabled.unwrap_or(true),
            deposit_depletion_enabled: deposit_depletion_enabled.unwrap_or(false),
        },
        ClientCommand::SetHazardsEnabled { enabled } => Command::SetHazardsEnabled { enabled },
        ClientCommand::SetMaintenanceEnabled { enabled } => {
            Command::SetMaintenanceEnabled { enabled }
        }
        ClientCommand::SetDepositDepletionEnabled { enabled } => {
            Command::SetDepositDepletionEnabled { enabled }
        }
        ClientCommand::AssignColonyHomeBody { colony_id, body_id } => {
            let cid = parse_colony(&colony_id)?;
            let bid = Uuid::parse_str(&body_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad body_id: {body_id}")))?;
            Command::AssignColonyHomeBody {
                colony_id: cid,
                body_id: outpost_core::system::BodyId(bid),
            }
        }
        ClientCommand::EstablishOutpost {
            name,
            colony_id,
            body_id,
        } => {
            let cid = parse_colony(&colony_id)?;
            let bid = Uuid::parse_str(&body_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad body_id: {body_id}")))?;
            Command::EstablishOutpost {
                name,
                colony_id: cid,
                body_id: outpost_core::system::BodyId(bid),
            }
        }
        ClientCommand::DecommissionOutpost { outpost_id } => Command::DecommissionOutpost {
            outpost_id: Uuid::parse_str(&outpost_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad outpost_id: {outpost_id}")))?,
        },
        ClientCommand::QueueOutpostConstruction {
            outpost_id,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        } => Command::QueueOutpostConstruction {
            outpost_id: Uuid::parse_str(&outpost_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad outpost_id: {outpost_id}")))?,
            building_type,
            slot_cost,
            labor_per_turn,
            construction_cost,
            construction_turns,
        },
        ClientCommand::PromoteOutpostToColony {
            outpost_id,
            name,
            starting_population,
        } => Command::PromoteOutpostToColony {
            outpost_id: Uuid::parse_str(&outpost_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad outpost_id: {outpost_id}")))?,
            name,
            starting_population,
        },
        ClientCommand::SetOutpostActiveRecipe {
            outpost_id,
            building_type,
            recipe_id,
        } => Command::SetOutpostActiveRecipe {
            outpost_id: Uuid::parse_str(&outpost_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad outpost_id: {outpost_id}")))?,
            building_type,
            recipe_id,
        },
        ClientCommand::BuildInfrastructure {
            from_colony,
            to_colony,
            infra_type,
        } => Command::BuildInfrastructure {
            from_colony: parse_colony(&from_colony)?,
            to_colony: parse_colony(&to_colony)?,
            infra_type: parse_infra_type(&infra_type)?,
        },
        ClientCommand::DemolishInfrastructure {
            from_colony,
            to_colony,
        } => Command::DemolishInfrastructure {
            from_colony: parse_colony(&from_colony)?,
            to_colony: parse_colony(&to_colony)?,
        },
        ClientCommand::AddTradeRoute {
            colony_a,
            colony_b,
            throughput_cap,
        } => Command::AddTradeRoute {
            colony_a: parse_colony(&colony_a)?,
            colony_b: parse_colony(&colony_b)?,
            throughput_cap,
        },
        ClientCommand::RemoveTradeRoute { route_id } => Command::RemoveTradeRoute {
            route_id: Uuid::from_str(&route_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad route_id: {route_id}")))?,
        },
    };

    match engine.apply(&core_cmd) {
        Ok(events) => {
            log::info!(
                "apply_command OK: {} event(s) from {command_debug} — {:?}",
                events.len(),
                events.iter().map(|e| format!("{e:?}")).collect::<Vec<_>>()
            );
            Ok(events.iter().map(ServerEvent::from_core).collect())
        }
        Err(e) => {
            log::error!("apply_command FAILED: {e} — command={command_debug}");
            Err(CmdError::from(e))
        }
    }
}

/// Run a read-only query. Returns raw JSON to accommodate heterogeneous result shapes.
#[tauri::command]
pub fn run_query(
    query: ClientQuery,
    engine_state: State<'_, EngineState>,
) -> CmdResult<serde_json::Value> {
    let query_debug = format!("{query:?}");
    log::debug!("run_query: received {query_debug}");

    let guard = match engine_state.engine.lock() {
        Ok(g) => g,
        Err(poisoned) => {
            log::error!(
                "run_query: engine mutex POISONED (a prior command panicked mid-mutation) — recovering with the pre-panic state; query={query_debug}"
            );
            poisoned.into_inner()
        }
    };
    let engine = match guard.as_ref().ok_or(CmdError::NotInitialised) {
        Ok(e) => e,
        Err(e) => {
            log::error!("run_query FAILED (not initialised): query={query_debug}");
            return Err(e);
        }
    };

    let core_query = match query {
        ClientQuery::CurrentSol => Query::CurrentSol,
        ClientQuery::CurrentMonth => Query::CurrentMonth,
        ClientQuery::ListColonies => Query::ListColonies,
        ClientQuery::ColonyStatus { colony_id } => Query::ColonyStatus {
            colony_id: parse_colony(&colony_id)?,
        },
        ClientQuery::ColonyScreen { colony_id } => Query::ColonyScreen {
            colony_id: parse_colony(&colony_id)?,
        },
        ClientQuery::SystemResearchTotal => Query::SystemResearchTotal,
        ClientQuery::BuildingDetail {
            colony_id,
            building_type,
        } => Query::BuildingDetail {
            colony_id: parse_colony(&colony_id)?,
            building_type,
        },
        ClientQuery::OutpostBuildingDetail {
            outpost_id,
            building_type,
        } => Query::OutpostBuildingDetail {
            outpost_id: Uuid::parse_str(&outpost_id)
                .map_err(|_| CmdError::InvalidArg(format!("bad outpost_id: {outpost_id}")))?,
            building_type,
        },
    };

    let result = match engine.query(&core_query) {
        Ok(r) => r,
        Err(e) => {
            log::error!("run_query FAILED: {e} — query={query_debug}");
            return Err(CmdError::from(e));
        }
    };
    log::debug!("run_query OK: query={query_debug}");
    let value = match result {
        QueryResult::Counter(v) => serde_json::json!({ "kind": "counter", "value": v }),
        QueryResult::Colonies(list) => serde_json::json!({ "kind": "colonies", "colonies": list }),
        QueryResult::ColonyStatus(s) => serde_json::json!({ "kind": "colony_status", "status": s }),
        QueryResult::Labour(l) => serde_json::json!({ "kind": "labour", "labour": l }),
        QueryResult::ResearchTotal(t) => {
            serde_json::json!({ "kind": "research_total", "total": t })
        }
        QueryResult::ColonyScreen(d) => serde_json::json!({ "kind": "colony_screen", "data": d }),
        QueryResult::BuildingDetail(d) => {
            serde_json::json!({ "kind": "building_detail", "data": d })
        }
        other => serde_json::json!({ "kind": "other", "debug": format!("{other:?}") }),
    };
    Ok(value)
}

/// Discard the current engine and return to a pre-bootstrap state.
#[tauri::command]
pub fn reset_engine(engine_state: State<'_, EngineState>) {
    *engine_state.engine.lock().unwrap() = None;
}

/// Persist the current game state to a SQLite file.
#[tauri::command]
pub fn save_game(path: String, engine_state: State<'_, EngineState>) -> CmdResult<()> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let mut db =
        SnapshotDb::open(Path::new(&path)).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    db.save(&engine.state)
        .map_err(|e| CmdError::Snapshot(e.to_string()))
}

/// Load a previously saved game from a SQLite file.
#[tauri::command]
pub fn load_game(path: String, engine_state: State<'_, EngineState>) -> CmdResult<SnapshotPayload> {
    let db = SnapshotDb::open(Path::new(&path)).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let game_state = db.load().map_err(|e| CmdError::Snapshot(e.to_string()))?;
    let mut engine = GameEngine::new();
    engine.state = game_state;

    // The save holds state, not content — see `attach_content`. Without this
    // a loaded game has no registry, so the build dialog is empty, the tech
    // tree fails to load, and production silently stops happening.
    //
    // Always the embedded pack: `bootstrap` can be pointed at a directory,
    // but the save records no content source to reproduce that choice, and
    // embedded is what the shipped app uses.
    let registry = load_embedded_content().inspect_err(|e| {
        log::error!("load_game FAILED loading content: {e}");
    })?;
    attach_content(&mut engine, registry);
    log::info!(
        "load_game: content reattached, registry={} tech_registry={}",
        engine.state.registry.is_some(),
        engine.state.tech_registry.is_some()
    );

    let snap = build_snapshot(&engine);
    *engine_state.engine.lock().unwrap() = Some(engine);
    Ok(snap)
}

// ── Custom, high-level queries used by dedicated UI views ────────────────────

/// A body on the system map, positioned for rendering.
#[derive(Debug, Serialize)]
pub struct SystemBodyWire {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub role: String,
    pub distance_au: f32,
    pub colonizable: bool,
    /// Atmospheric thickness/density band (issue #197).
    pub atmosphere_density: String,
    /// Atmospheric chemical hazard band (issue #197).
    pub atmosphere_hazard: String,
    pub temperature: String,
    pub gravity_g: f32,
    pub radiation: String,
    pub habitability: u8,
    pub habitability_modifier: f32,
    /// Habitability score after tech-driven mitigations are applied (issue
    /// #185). Equal to `habitability` when no mitigation applies.
    pub habitability_effective: u8,
    /// Habitability modifier after tech-driven mitigations are applied
    /// (issue #185). Equal to `habitability_modifier` when no mitigation applies.
    pub habitability_modifier_effective: f32,
    /// Starlight reaching this body, in units where Sol at 1 AU is `1.0`
    /// (issue #413). A moon reports its parent's.
    pub insolation: f32,
    /// Surface/composition archetype (issue #196). Flavor/authoring
    /// guidance — not a habitability input.
    pub subtype: String,
    pub tidally_locked: bool,
    pub axial_tilt_deg: f32,
    pub rotation_period_hours: f32,
    pub moon_count: u32,
    /// Display name of the body this one orbits, if any.
    pub parent_body_name: Option<String>,
    /// Per-category production modifiers (issue #184) — category name to
    /// multiplier, e.g. `("power_output", 1.3)`. Empty when unauthored.
    pub category_modifiers: Vec<(String, f32)>,
    /// Density-zoned annulus profile for belt-kind bodies (system-screen fix
    /// B2). `None` for non-belt bodies.
    pub belt_profile: Option<BeltProfileWire>,
}

/// One angular zone of a belt's annulus (system-screen fix B2).
#[derive(Debug, Serialize)]
pub struct BeltZoneWire {
    /// Angular start of the zone, in degrees `[0, 360)`.
    pub start_deg: f32,
    /// Angular sweep of the zone, in degrees.
    pub sweep_deg: f32,
    /// Fill density `[0, 1]` — drives the annulus fill opacity.
    pub density: f32,
}

/// Radial/angular density profile of a belt, for annulus rendering
/// (system-screen fix B2).
#[derive(Debug, Serialize)]
pub struct BeltProfileWire {
    /// Inner radius of the annulus, in AU.
    pub inner_au: f32,
    /// Outer radius of the annulus, in AU.
    pub outer_au: f32,
    /// Angular zones subdividing the annulus.
    pub zones: Vec<BeltZoneWire>,
}

/// Return the generated star system's display name.
///
/// Mirrors `outpost_web`'s `GET /api/system-name`. Empty for a system that
/// predates seed-derived naming; the caller falls back to a generic label.
#[tauri::command]
pub fn get_system_name(engine_state: State<'_, EngineState>) -> CmdResult<String> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    Ok(engine.state.system_state.node_map.system_name.clone())
}

/// Return the current list of system bodies with rendering hints.
#[tauri::command]
pub fn get_system_bodies(engine_state: State<'_, EngineState>) -> CmdResult<Vec<SystemBodyWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let node_bodies = &engine.state.system_state.node_map.bodies;
    let bodies: Vec<SystemBodyWire> = node_bodies
        .values()
        .map(|b| SystemBodyWire {
            id: b.id.0.to_string(),
            name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            role: format!("{:?}", b.role),
            distance_au: b.distance_au,
            insolation: engine
                .state
                .system_state
                .node_map
                .insolation_for(&b.id)
                .unwrap_or(0.0),
            colonizable: matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            ),
            atmosphere_density: format!("{:?}", b.atmosphere_density),
            atmosphere_hazard: format!("{:?}", b.atmosphere_hazard),
            temperature: format!("{:?}", b.temperature),
            gravity_g: b.gravity_g,
            radiation: format!("{:?}", b.radiation),
            habitability: b.habitability(),
            habitability_modifier: b.habitability_modifier(),
            habitability_effective: b
                .habitability_with_mitigations(&engine.state.habitability_mitigations),
            habitability_modifier_effective: b
                .habitability_modifier_with_mitigations(&engine.state.habitability_mitigations),
            subtype: format!("{:?}", b.subtype),
            tidally_locked: b.tidally_locked,
            axial_tilt_deg: b.axial_tilt_deg,
            rotation_period_hours: b.rotation_period_hours,
            moon_count: b.moon_count,
            parent_body_name: b
                .parent_body
                .as_ref()
                .and_then(|pid| node_bodies.get(pid))
                .map(|p| p.name.clone()),
            category_modifiers: b
                .modifiers
                .iter()
                .map(|m| (format!("{:?}", m.category), m.multiplier))
                .collect(),
            belt_profile: b.belt_profile.as_ref().map(|p| BeltProfileWire {
                inner_au: p.inner_au,
                outer_au: p.outer_au,
                zones: p
                    .zones
                    .iter()
                    .map(|z| BeltZoneWire {
                        start_deg: z.start_deg,
                        sweep_deg: z.sweep_deg,
                        density: z.density,
                    })
                    .collect(),
            }),
        })
        .collect();
    Ok(bodies)
}

/// Tech definition + player state (researched / `in_progress` / queued / available / locked).
#[derive(Debug, Serialize)]
pub struct TechNodeWire {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub tier: u32,
    pub cost: f32,
    pub prerequisites: Vec<String>,
    pub state: String, // researched | in_progress | queued | available | locked
    pub progress: f32,
    pub effects: Vec<outpost_core::tech::TechEffect>,
    /// Zero-based position in `TechState.research_queue` (the actual FIFO
    /// drain order), only meaningful when `state == "queued"`.
    pub queue_position: Option<usize>,
}

/// One accumulated interrupt in the fast-forward digest (issue #332).
#[derive(Debug, Serialize)]
pub struct DigestItemWire {
    pub tier: String,
    pub message: String,
    pub colony_id: Option<String>,
    pub acknowledged: bool,
}

/// The return-from-fast-forward triage payload (issue #332).
#[derive(Debug, Serialize)]
pub struct InterruptDigestWire {
    pub stopped_at_sol: u64,
    pub sols_requested: u32,
    pub halting_message: Option<String>,
    pub halting_tier: Option<String>,
    pub items: Vec<DigestItemWire>,
}

/// Render an interrupt tier as the slug `ClientCommand::FastForward` accepts,
/// so the UI can feed a tier it read back in as a threshold.
fn tier_slug(tier: outpost_core::interrupt::Tier) -> &'static str {
    use outpost_core::interrupt::Tier;
    match tier {
        Tier::Ambient => "ambient",
        Tier::Notable => "notable",
        Tier::Urgent => "urgent",
        Tier::Blocking => "blocking",
    }
}

/// Return what happened during the last fast-forward run (issue #332).
///
/// `ClientCommand::FastForward` reports only that a run ended and why; this is
/// where the accumulated below-threshold interrupts are read for the digest
/// panel. Empty before any fast-forward has run.
#[tauri::command]
pub fn get_interrupt_digest(
    engine_state: State<'_, EngineState>,
) -> CmdResult<InterruptDigestWire> {
    use outpost_core::{Query, QueryResult};

    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let QueryResult::InterruptDigest(digest) = engine
        .query(&Query::InterruptDigest)
        .map_err(|e| CmdError::Engine(e.to_string()))?
    else {
        return Err(CmdError::Engine(
            "InterruptDigest query returned the wrong result variant".into(),
        ));
    };

    Ok(InterruptDigestWire {
        stopped_at_sol: digest.stopped_at_turn,
        sols_requested: digest.turns_advanced,
        halting_message: digest.halting_interrupt.as_ref().map(|i| i.message.clone()),
        halting_tier: digest
            .halting_interrupt
            .as_ref()
            .map(|i| tier_slug(i.tier).to_owned()),
        items: digest
            .digest_items
            .iter()
            .map(|item| DigestItemWire {
                tier: tier_slug(item.interrupt.tier).to_owned(),
                message: item.interrupt.message.clone(),
                colony_id: item.interrupt.colony_id.map(|id| id.to_string()),
                acknowledged: item.acknowledged,
            })
            .collect(),
    })
}

/// Return the full tech tree with per-node state.
#[tauri::command]
pub fn get_tech_tree(engine_state: State<'_, EngineState>) -> CmdResult<Vec<TechNodeWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let tech_registry = engine
        .state
        .tech_registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no tech registry loaded".into()))?;

    let tech_state = &engine.state.tech_state;
    let current = tech_state.current_project.as_deref();

    let mut nodes = Vec::new();
    for tech in tech_registry.all() {
        let is_done = tech_state.is_researched(&tech.id);
        let is_active = current == Some(tech.id.as_str());
        let queue_position = tech_state
            .research_queue
            .iter()
            .position(|t| t == &tech.id);
        let is_queued = queue_position.is_some();
        let prereqs_met = tech_state.prerequisites_met(tech);

        let state = if is_done {
            "researched"
        } else if is_active {
            "in_progress"
        } else if is_queued {
            "queued"
        } else if prereqs_met {
            "available"
        } else {
            "locked"
        };
        let progress = if is_active {
            tech_state.progress / tech.research_cost.max(0.001)
        } else {
            0.0
        };
        nodes.push(TechNodeWire {
            id: tech.id.clone(),
            name: tech.display_name.clone(),
            category: tech.category.clone(),
            description: tech.description.clone(),
            tier: tech.tier,
            cost: tech.research_cost,
            prerequisites: tech.prerequisites.clone(),
            state: state.to_owned(),
            progress,
            effects: tech.effects.clone(),
            queue_position,
        });
    }
    nodes.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(nodes)
}

/// Colonizable target for the founding wizard.
#[derive(Debug, Serialize)]
pub struct ColonizeTargetWire {
    pub body_id: String,
    pub body_name: String,
    pub kind: String,
    pub distance_au: f32,
    /// Body habitability score (0-100), issue #183.
    pub habitability: u8,
    /// Whether founding on this body is currently allowed — either the
    /// habitability score clears `HABITABILITY_FOUNDING_THRESHOLD`, or the
    /// player has unlocked `HARSH_WORLD_CAPABILITY_ID`. The wizard still
    /// shows bodies below this so the player understands what's blocking
    /// them (issue #183 UX decision), rather than hiding them outright.
    pub can_found: bool,
}

#[tauri::command]
pub fn get_colonize_targets(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<ColonizeTargetWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let harsh_world_unlocked = engine
        .state
        .unlocked_capabilities
        .contains(outpost_core::system::HARSH_WORLD_CAPABILITY_ID);
    let list = engine
        .state
        .system_state
        .node_map
        .bodies
        .values()
        .filter(|b| {
            matches!(
                b.kind,
                BodyKind::InnerPlanet | BodyKind::Moon | BodyKind::AsteroidBelt
            )
        })
        .map(|b| ColonizeTargetWire {
            body_id: b.id.0.to_string(),
            body_name: b.name.clone(),
            kind: format!("{:?}", b.kind),
            distance_au: b.distance_au,
            habitability: b.habitability(),
            can_found: b.meets_founding_threshold() || harsh_world_unlocked,
        })
        .collect();
    Ok(list)
}

/// An established outpost, for the outposts management view (issue #243).
#[derive(Debug, Serialize)]
pub struct OutpostWire {
    pub id: String,
    pub name: String,
    pub parent_colony_id: String,
    pub body_id: String,
    pub body_name: String,
    pub slot_capacity: u32,
    pub slots_used: u32,
    pub buildings: Vec<String>,
    /// Pooled commodity stockpile as `(commodity_id, amount)` pairs — every
    /// commodity that has ever had a non-zero amount, mirroring
    /// `Query::ColonyScreen`'s `stockpile` convention.
    pub pool: Vec<(String, f64)>,
}

/// List every established outpost across all colonies (issue #233/#243).
///
/// The frontend filters by `parent_colony_id` client-side to scope the
/// list to the currently selected colony — mirrors how `ListColonies`
/// returns everything and callers narrow as needed.
#[tauri::command]
pub fn list_outposts(engine_state: State<'_, EngineState>) -> CmdResult<Vec<OutpostWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let list = engine
        .state
        .outposts
        .iter()
        .map(|o| {
            let body_name = engine
                .state
                .system_state
                .node_map
                .bodies
                .get(&o.body_id)
                .map_or_else(|| "Unknown".to_string(), |b| b.name.clone());
            OutpostWire {
                id: o.id.to_string(),
                name: o.name.clone(),
                parent_colony_id: o.parent_colony_id.to_string(),
                body_id: o.body_id.0.to_string(),
                body_name,
                slot_capacity: o.slot_capacity,
                slots_used: o.slots_used(),
                buildings: o.buildings.iter().map(|b| b.building_type.clone()).collect(),
                pool: o
                    .pool
                    .commodity_ids()
                    .map(|cid| (cid.to_string(), o.pool.amount(cid)))
                    .collect(),
            }
        })
        .collect();
    Ok(list)
}

/// A body evaluated as a possible `EstablishOutpost` target for a given
/// parent colony (issue #241/#243).
#[derive(Debug, Serialize)]
pub struct OutpostTargetWire {
    pub body_id: String,
    pub body_name: String,
    pub kind: String,
    pub distance_au: f32,
    /// Distance from the parent colony's home body, in AU. `None` when the
    /// parent colony has no `home_body_id` (range gating is inert for it —
    /// see `outpost::max_outpost_range_au`'s doc comment) or when computing
    /// it would require a body no longer in the system.
    pub distance_from_home_au: Option<f32>,
    /// Whether `EstablishOutpost` would currently accept this body — always
    /// `true` when `distance_from_home_au` is `None` (the range gate is
    /// inert for an unplaced colony).
    pub in_range: bool,
}

/// List bodies a given colony could establish an outpost on, annotated with
/// range-gate status (issue #241) so the placement flow can filter/explain
/// out-of-range bodies instead of only discovering it on rejection.
///
/// # Errors
///
/// Returns [`CmdError::InvalidArg`] if `colony_id` isn't a valid UUID.
#[tauri::command]
pub fn get_outpost_targets(
    colony_id: String,
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<OutpostTargetWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let cid = parse_colony(&colony_id)?;
    let home_body = engine
        .state
        .colonies
        .iter()
        .find(|c| c.id == cid)
        .and_then(|c| c.home_body_id.as_ref())
        .and_then(|bid| engine.state.system_state.node_map.bodies.get(bid));
    let max_range_au = home_body.map(|_| {
        outpost_core::outpost::max_outpost_range_au(
            engine.state.system_state.node_map.propulsion_level,
            engine.state.outpost_range_bonus_au,
        )
    });
    let list = engine
        .state
        .system_state
        .node_map
        .bodies
        .values()
        .map(|b| {
            let distance_from_home_au = home_body.map(|h| (h.distance_au - b.distance_au).abs());
            let in_range = match (distance_from_home_au, max_range_au) {
                (Some(d), Some(max)) => d <= max,
                _ => true,
            };
            OutpostTargetWire {
                body_id: b.id.0.to_string(),
                body_name: b.name.clone(),
                kind: format!("{:?}", b.kind),
                distance_au: b.distance_au,
                distance_from_home_au,
                in_range,
            }
        })
        .collect();
    Ok(list)
}

/// A building option in the founding wizard.
///
/// Carries the canonical construction data from `BuildingDef` so the UI
/// never has to invent slot cost / labor / cost / turns.
#[derive(Debug, Serialize)]
pub struct BuildingOptionWire {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub slot_cost: u32,
    pub labor_per_turn: u32,
    pub construction_turns: u32,
    pub construction_cost: Vec<(String, f64)>,
    pub tech_prerequisite: Option<String>,
    /// Whether this building is part of the default landing kit (issue #317) —
    /// lets the founding wizard pre-select it as the recommended loadout.
    pub starter_kit: bool,
    /// Most instances one colony may have, or `None` for unlimited. Lets the
    /// build UI grey out an option the engine would reject rather than
    /// offering a button that always errors.
    pub max_instances: Option<u32>,
}

#[tauri::command]
pub fn list_buildings(engine_state: State<'_, EngineState>) -> CmdResult<Vec<BuildingOptionWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let registry = engine
        .state
        .registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no content registry loaded".into()))?;
    let mut out: Vec<BuildingOptionWire> = registry
        .buildings()
        .map(|b| BuildingOptionWire {
            id: b.id.clone(),
            name: b.name.clone(),
            description: b.description.clone(),
            category: format!("{:?}", b.category),
            slot_cost: b.slot_cost,
            labor_per_turn: b.labor_required,
            construction_turns: b.construction_turns,
            construction_cost: b
                .construction_cost
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
            tech_prerequisite: b.tech_prerequisite.clone(),
            starter_kit: b.starter_kit,
            max_instances: b.max_instances,
        })
        .collect();
    out.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(out)
}

/// One hex on the planet surface, enriched with UI-relevant fields.
#[derive(Debug, Serialize)]
pub struct PlanetHexWire {
    pub q: i32,
    pub r: i32,
    pub site_id: String,
    pub terrain: String,
    pub biome: String,
    pub elevation: f32,
    /// Geothermal gradient in `[0.0, 1.0]` (issue #412) — how shallow magma
    /// sits beneath this hex. Surfaced so it can inform a landing-site
    /// choice, which is exactly where it matters.
    pub geothermal_gradient: f32,
    pub temperature: String,
    /// Fraction of this cell covered by water/ice, in `[0.0, 1.0]` (issue #316).
    pub water_coverage: f32,
    /// Vegetation density in this cell, in `[0.0, 1.0]` (issue #316).
    pub vegetation_density: f32,
    /// Contamination severity in `[0.0, 1.0]` from waste overflow (issue
    /// #387). `0.0` is pristine.
    pub contamination: f32,
    pub deposits: Vec<DepositWire>,
    pub habitable: bool,
    pub suitability: f32,
    pub occupied_by: Option<String>,
    /// Id of the colony occupying this cell, if any (persistent planet map,
    /// phase A1) — lets a map node link through to `/colony/:id`.
    pub occupant_colony_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DepositWire {
    pub commodity_id: String,
    pub richness: f32,
}

#[derive(Debug, Serialize)]
pub struct PlanetMapWire {
    pub seed: u64,
    pub width: u32,
    pub height: u32,
    pub hexes: Vec<PlanetHexWire>,
    /// Infrastructure edges connecting colony nodes (map/nav plan phase A3).
    pub edges: Vec<InfraEdgeWire>,
}

/// An infrastructure edge between two colonies, for planet-map rendering
/// (map/nav plan phase A3). Endpoints resolve to hex positions on the
/// frontend via each colony's `occupant_colony_id`.
#[derive(Debug, Serialize)]
pub struct InfraEdgeWire {
    /// Id of the colony the edge runs from.
    pub from_colony_id: String,
    /// Id of the colony the edge runs to.
    pub to_colony_id: String,
    /// Infrastructure type (`road`, `rail`, `pipeline`, `powerline`).
    pub infra_type: String,
    /// Cargo (or, for a powerline, power) throughput per turn, before tech
    /// modifiers.
    pub throughput: f32,
    /// Construction cost (abstract resource units).
    pub cost: f32,
    /// Fraction of throughput lost in transit, in `[0.0, 1.0]` (issue #383).
    pub loss_pct: f32,
}

/// A starter supply package option surfaced in the founding wizard.
#[derive(Debug, Serialize)]
pub struct SupplyPackageWire {
    pub id: String,
    pub name: String,
    pub description: String,
    pub commodities: Vec<(String, f64)>,
}

/// Return all `SupplyPackage` records from the loaded content pack, sorted by name.
#[tauri::command]
pub fn list_supply_packages(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<SupplyPackageWire>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let registry = engine
        .state
        .registry
        .as_ref()
        .ok_or_else(|| CmdError::Content("no content registry loaded".into()))?;
    let mut out: Vec<SupplyPackageWire> = registry
        .supply_packages()
        .map(|p| SupplyPackageWire {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            commodities: p
                .commodities
                .iter()
                .map(|i| (i.id.clone(), i.quantity))
                .collect(),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Return every tunable balance scalar and its current value (playtesting).
#[tauri::command]
pub fn get_balance_scalars(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<outpost_core::ui::BalanceScalarRow>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    match engine.query(&outpost_core::Query::BalanceScalars) {
        Ok(outpost_core::QueryResult::BalanceScalars(rows)) => Ok(rows),
        Ok(_) => Err(CmdError::Engine("unexpected query result".into())),
        Err(e) => Err(CmdError::Engine(e.to_string())),
    }
}

/// Return every trade route in the planetary trade network (issue #363),
/// infrastructure-linked or manually added alike.
#[tauri::command]
pub fn get_trade_routes(
    engine_state: State<'_, EngineState>,
) -> CmdResult<Vec<outpost_core::trade::TradeRoute>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    match engine.query(&outpost_core::Query::TradeRoutes) {
        Ok(outpost_core::QueryResult::TradeRoutes(routes)) => Ok(routes),
        Ok(_) => Err(CmdError::Engine("unexpected query result".into())),
        Err(e) => Err(CmdError::Engine(e.to_string())),
    }
}

/// Return the current planet hex map with per-cell metadata.
#[tauri::command]
pub fn get_planet_map(engine_state: State<'_, EngineState>) -> CmdResult<PlanetMapWire> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let pm = engine
        .state
        .home_map()
        .ok_or_else(|| CmdError::Engine("no planet map — bootstrap first".into()))?;

    Ok(build_planet_map_wire(pm, engine))
}

/// Return the surface of any body in the system (planet or moon) — the live
/// stored map if it has been settled, a generated preview if it has not. Used
/// by the system map's "View Surface" action (map/nav plan).
///
/// Preferring the stored surface makes this the single surface view for every
/// world (issue #300): a settled body shows its colonies and infrastructure, an
/// unvisited one still renders. The two agree cell-for-cell, since a body's
/// stored map is generated by `body_surface_preview` when it is first settled.
#[tauri::command]
pub fn get_body_surface(
    engine_state: State<'_, EngineState>,
    body_id: String,
) -> CmdResult<PlanetMapWire> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;

    let uuid = uuid::Uuid::parse_str(&body_id)
        .map_err(|e| CmdError::Engine(format!("invalid body id: {e}")))?;
    let id = outpost_core::system::BodyId(uuid);
    if let Some(pm) = engine.state.map_for_body(&id) {
        return Ok(build_planet_map_wire(pm, engine));
    }
    let pm = engine
        .body_surface_preview(&id)
        .map_err(|e| CmdError::Engine(e.to_string()))?;

    Ok(build_planet_map_wire(&pm, engine))
}

/// Convert a [`map::PlanetMap`] into its wire form, resolving per-cell site
/// ids, colony occupancy, and infrastructure edges from the engine state.
/// Shared by [`get_planet_map`] (the live founding map) and
/// [`get_body_surface`] (a generated read-only preview).
fn build_planet_map_wire(
    pm: &outpost_core::map::PlanetMap,
    engine: &outpost_core::GameEngine,
) -> PlanetMapWire {
    // Build reverse coord → site_id lookup so hex rows carry a site_id.
    let coord_to_site: std::collections::HashMap<_, _> =
        pm.sites.iter().map(|(sid, coord)| (*coord, *sid)).collect();

    // Reverse coord → colony_name lookup for the `occupied_by` field.
    let colony_names: std::collections::HashMap<_, _> = engine
        .state
        .colonies
        .iter()
        .map(|c| (c.id, c.name.clone()))
        .collect();
    let coord_to_colony: std::collections::HashMap<_, _> = pm
        .colonies
        .iter()
        .filter_map(|node| {
            colony_names
                .get(&node.colony_id)
                .map(|name| (node.coord, (node.colony_id, name.clone())))
        })
        .collect();

    let mut hexes: Vec<PlanetHexWire> = pm
        .cells
        .values()
        .map(|cell| {
            let site_id = coord_to_site
                .get(&cell.coord)
                .map(|sid| sid.0.to_string())
                .unwrap_or_default();
            PlanetHexWire {
                q: cell.coord.q,
                r: cell.coord.r,
                site_id,
                terrain: format!("{:?}", cell.terrain),
                biome: format!("{:?}", cell.biome),
                elevation: cell.elevation,
            geothermal_gradient: cell.geothermal_gradient,
                temperature: format!("{:?}", cell.temperature),
                water_coverage: cell.water_coverage,
                vegetation_density: cell.vegetation_density,
                contamination: cell.contamination,
                deposits: cell
                    .deposits
                    .iter()
                    .map(|d| DepositWire {
                        commodity_id: d.commodity_id.clone(),
                        richness: d.richness,
                    })
                    .collect(),
                habitable: cell.is_habitable(),
                suitability: cell.suitability(),
                occupied_by: coord_to_colony
                    .get(&cell.coord)
                    .map(|(_, name)| name.clone()),
                occupant_colony_id: coord_to_colony
                    .get(&cell.coord)
                    .map(|(id, _)| id.to_string()),
            }
        })
        .collect();

    // Stable ordering so the frontend doesn't churn cell z-order between calls.
    hexes.sort_by_key(|h| (h.r, h.q));

    let edges: Vec<InfraEdgeWire> = pm
        .edges
        .iter()
        .map(|e| InfraEdgeWire {
            from_colony_id: e.from.to_string(),
            to_colony_id: e.to.to_string(),
            infra_type: format!("{:?}", e.infra_type).to_lowercase(),
            throughput: e.throughput,
            cost: e.cost,
            loss_pct: e.loss_pct,
        })
        .collect();

    PlanetMapWire {
        seed: pm.seed,
        width: pm.width,
        height: pm.height,
        hexes,
        edges,
    }
}

/// List `*.o3save` files in the given directory. Returns filenames only.
#[tauri::command]
pub fn list_saves(dir: String) -> CmdResult<Vec<String>> {
    let path = Path::new(&dir);
    if !path.exists() {
        return Ok(vec![]);
    }
    let mut saves = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|e| CmdError::Snapshot(e.to_string()))? {
        let entry = entry.map_err(|e| CmdError::Snapshot(e.to_string()))?;
        let name = entry.file_name();
        let s = name.to_string_lossy();
        if s.ends_with(".o3save") || s.ends_with(".sqlite") {
            saves.push(s.into_owned());
        }
    }
    saves.sort();
    Ok(saves)
}

// ── #161: difficulty knob discovery + preset persistence ────────────────────

/// Return one [`KnobSpec`] per slider / toggle so the panel is data-driven.
///
/// Slider defaults + current values are read from the engine's current
/// difficulty scalar; toggles read `hazards_enabled` and the presence of an
/// active `menace_state`.
#[tauri::command]
pub fn get_difficulty_knobs(engine_state: State<'_, EngineState>) -> CmdResult<Vec<KnobSpec>> {
    let guard = engine_state.engine.lock().unwrap();
    let engine = guard.as_ref().ok_or(CmdError::NotInitialised)?;
    let state = &engine.state;

    // Reference row from the current preset (or Normal for Custom).
    let ref_preset = if matches!(state.difficulty_preset, DifficultyPreset::Custom) {
        DifficultyPreset::Normal
    } else {
        state.difficulty_preset
    };
    let ref_scalar = state.difficulty_grade_table.build_scalar(ref_preset);

    let mut specs: Vec<KnobSpec> = Vec::new();
    for (id, label, key) in SLIDER_KNOBS {
        let q = key.to_quantity();
        specs.push(KnobSpec {
            id: (*id).to_owned(),
            label: (*label).to_owned(),
            kind: "slider",
            step: SLIDER_STEPS.to_vec(),
            min: SLIDER_STEPS[0],
            max: SLIDER_STEPS[SLIDER_STEPS.len() - 1],
            preset_default_at_current_preset: ref_scalar.scalar_for(&q),
            current_value: state.difficulty_scalar.scalar_for(&q),
        });
    }

    specs.push(KnobSpec {
        id: "menace_enabled".into(),
        label: "Menace Enabled".into(),
        kind: "toggle",
        step: vec![],
        min: 0.0,
        max: 1.0,
        preset_default_at_current_preset: 1.0,
        current_value: if state.menace_state.is_some() {
            1.0
        } else {
            0.0
        },
    });
    specs.push(KnobSpec {
        id: "hazards_enabled".into(),
        label: "Hazards Enabled".into(),
        kind: "toggle",
        step: vec![],
        min: 0.0,
        max: 1.0,
        preset_default_at_current_preset: 1.0,
        current_value: if state.hazards_enabled { 1.0 } else { 0.0 },
    });
    specs.push(KnobSpec {
        id: "maintenance_enabled".into(),
        label: "Maintenance Enabled".into(),
        kind: "toggle",
        step: vec![],
        min: 0.0,
        max: 1.0,
        preset_default_at_current_preset: 1.0,
        current_value: if state.maintenance_enabled { 1.0 } else { 0.0 },
    });
    Ok(specs)
}

/// Enumerate every saved preset in the app-data directory.
#[tauri::command]
pub fn list_custom_presets(app: AppHandle) -> CmdResult<Vec<CustomPreset>> {
    let dir = custom_preset_dir(&app)?;
    let mut out: Vec<CustomPreset> = Vec::new();
    let entries = std::fs::read_dir(&dir).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| CmdError::Snapshot(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if let Ok(preset) = serde_yaml::from_str::<CustomPreset>(&text) {
            out.push(preset);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Persist a named preset (replaces on collision).
#[tauri::command]
pub fn save_custom_preset(
    app: AppHandle,
    name: String,
    scalars: std::collections::HashMap<String, f32>,
    menace_enabled: bool,
    hazards_enabled: bool,
    maintenance_enabled: Option<bool>,
) -> CmdResult<()> {
    let sanitized = sanitize_preset_name(&name);
    if sanitized.is_empty() {
        return Err(CmdError::InvalidArg("preset name is empty".into()));
    }
    let dir = custom_preset_dir(&app)?;
    let preset = CustomPreset {
        name: sanitized.clone(),
        scalars,
        menace_enabled,
        hazards_enabled,
        maintenance_enabled: maintenance_enabled.unwrap_or(true),
    };
    let text =
        serde_yaml::to_string(&preset).map_err(|e| CmdError::Snapshot(format!("yaml: {e}")))?;
    let path = dir.join(format!("{sanitized}.yaml"));
    std::fs::write(&path, text).map_err(|e| CmdError::Snapshot(e.to_string()))
}

/// Delete a named preset. Missing files are treated as success.
#[tauri::command]
pub fn delete_custom_preset(app: AppHandle, name: String) -> CmdResult<()> {
    let sanitized = sanitize_preset_name(&name);
    let dir = custom_preset_dir(&app)?;
    let path = dir.join(format!("{sanitized}.yaml"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| CmdError::Snapshot(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod embedded_content_tests {
    use super::*;

    /// The desktop shell serves content compiled into the binary by
    /// `include_dir!("$CARGO_MANIFEST_DIR/../content/base")`, not read from
    /// disk — so a content change only reaches players if the embed is
    /// actually picking it up. Nothing else in the test suite exercises that
    /// path, which is why a roster change can look correct in browser mode
    /// (which reads `content/base` live over HTTP) and be missing in the app.
    #[test]
    fn embedded_pack_contains_the_authored_building_roster() {
        let registry = load_embedded_content().expect("embedded pack must load");
        let ids: Vec<&str> = registry.buildings().map(|b| b.id.as_str()).collect();

        assert!(
            ids.len() >= 40,
            "expected the full authored roster in the embedded pack, got {}",
            ids.len()
        );
        assert!(
            ids.contains(&"colony_hq"),
            "embedded pack is missing colony_hq; got {ids:?}"
        );
    }

    /// A colony must be able to build power with nothing researched — the
    /// pre-tech generator has to survive the embed, not just exist in
    /// `content/base`.
    #[test]
    fn embedded_pack_offers_a_power_source_with_no_tech() {
        let registry = load_embedded_content().expect("embedded pack must load");
        let no_tech_power: Vec<&str> = registry
            .buildings()
            .filter(|b| b.tech_prerequisite.is_none() && b.power_delta < 0.0)
            .filter(|b| {
                matches!(
                    b.category,
                    outpost_core::content::types::BuildingCategory::Power
                )
            })
            .map(|b| b.id.as_str())
            .collect();

        assert!(
            !no_tech_power.is_empty(),
            "embedded pack exposes no tech-0 power building; buildings present: {:?}",
            registry.buildings().map(|b| &b.id).collect::<Vec<_>>()
        );
    }

    /// A save records *state*, not content — `SnapshotBlob::into_state`
    /// deliberately restores `registry`/`tech_registry`/`needs_config`/
    /// `morale_config` as `None`, leaving the caller to reattach them.
    /// `load_game` did not, so a loaded game came back with no content at all:
    /// the build dialog was empty (`list_buildings` errors without a
    /// registry), the tech tree failed to load, and the production step
    /// silently did nothing, since it no-ops when `registry` is `None`.
    ///
    /// Exercises the real round trip through SQLite rather than calling
    /// `attach_content` directly, so a future `load_game` that forgets it
    /// fails here.
    #[test]
    fn a_loaded_game_has_its_content_reattached() {
        let dir = std::env::temp_dir().join(format!("outpost3-loadtest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("roundtrip.o3save");

        // Build a game the way bootstrap does, save it, and drop it.
        let mut engine = GameEngine::new();
        attach_content(
            &mut engine,
            load_embedded_content().expect("embedded pack must load"),
        );
        engine
            .apply(&Command::FoundColony {
                name: "Roundtrip".into(),
                starting_population: 50,
            })
            .expect("found colony");
        {
            let mut db = SnapshotDb::open(&path).expect("open save");
            db.save(&engine.state).expect("save");
        }

        // Reload exactly as `load_game` does.
        let db = SnapshotDb::open(&path).expect("reopen save");
        let game_state = db.load().expect("load");
        let mut restored = GameEngine::new();
        restored.state = game_state;

        // Precondition: this is the gap being closed. If the snapshot ever
        // starts persisting content, this assertion tells us the fix is
        // redundant rather than silently passing.
        assert!(
            restored.state.registry.is_none(),
            "snapshot unexpectedly restored a registry; attach_content may no longer be needed"
        );

        attach_content(
            &mut restored,
            load_embedded_content().expect("embedded pack must load"),
        );

        assert!(restored.state.registry.is_some(), "registry not reattached");
        assert!(
            restored.state.tech_registry.is_some(),
            "tech registry not reattached"
        );
        assert!(
            restored.state.needs_config.is_some(),
            "needs config not reattached"
        );
        assert!(
            restored.state.morale_config.is_some(),
            "morale config not reattached"
        );

        // And the thing the player actually noticed: buildings are listable.
        let count = restored
            .state
            .registry
            .as_ref()
            .expect("registry")
            .buildings()
            .count();
        assert!(count >= 40, "expected the full roster after load, got {count}");

        // The colony survived the round trip too, so this isn't passing on an
        // empty game.
        assert_eq!(restored.state.colonies.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
