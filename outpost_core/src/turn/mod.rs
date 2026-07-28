//! Turn model — single-cadence turn processing.
//!
//! The **colony-sol** is the game's only cadence (issue #332). Every system runs
//! on it: production, labour, needs, hazards, population, research, trade, and
//! shipment delivery.
//!
//! Outpost 3 previously interleaved a second "strategic-month" cadence that
//! gated research, trade, shipment delivery, and orbital construction behind a
//! 30-sol boundary. That is gone. `sols_per_month` survives only as the divisor
//! for the [`GameState::month`] calendar label — no system branches on it — and
//! the `TurnCadence` enum went with the second cadence it existed to
//! distinguish.
//!
//! RNG is injected as a seeded [`rand_chacha::ChaCha8Rng`] stream so turn
//! resolution is deterministic — which is what makes fast-forwarding N sols
//! identical to advancing N times.

use std::collections::{HashMap, HashSet};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use uuid::Uuid;

use crate::colony::{Colony, ColonyId};
use crate::content::ContentRegistry;
use crate::difficulty::{default_grade_table, DifficultyGradeTable, DifficultyPreset};
use crate::directive::DirectiveStore;
use crate::hazard::{roll_hazard, HazardConfig, HazardKind};
use crate::interrupt::{InterruptConfig, StabilityTracker};
use crate::map::PlanetMap;
use crate::menace::MenaceState;
use crate::migration::{EmigrationGate, PendingMigration, PopulationTracker};
use crate::modifier::{
    DifficultyScalar, ModifiableQuantity, ModifierAccumulator, ModifierDescriptor,
};
use crate::needs::NeedsConfig;
use crate::orbital::OrbitalRegistry;
use crate::population::Population;
use crate::research::SystemResearchPool;
use crate::system::{BodyId, SystemState};
use crate::tech::TechEffect;
use crate::tech::{TechRegistry, TechState};

/// Floor on the site-preparation cost and time scalars (issue #306).
///
/// Compounding tech reductions approach this without crossing it, so expansion
/// stays a real decision however deep the tech tree runs.
pub const MIN_INFRASTRUCTURE_SCALAR: f32 = 0.25;

use crate::trade::TradeNetwork;
use crate::victory::{VictoryCondition, VictoryState};

/// Colony-sols in one calendar month.
///
/// Only used to derive the [`GameState::month`] display label and to convert
/// month-denominated authored durations (shipment travel, orbital build times)
/// into sols. Not a cadence — see the module docs (issue #332).
pub const DEFAULT_SOLS_PER_MONTH: u64 = 30;

/// Sols of its own consumption a colony holds back from trade.
///
/// Commodities like `water`, `oxygen`, and `food_ration` are tradeable cargo
/// *and* survival consumables, so only the stock above this reserve is offered
/// to the auto-trade pass — a colony must not export what its population is
/// about to eat, drink, or breathe.
///
/// Originally 10 sols, sized when trade ran once per 30 sols and the reserve had
/// to bridge that whole gap (#331). Since trade rebalances every sol and cargo is
/// dispatched as convoys (#332), the reserve's only remaining job is covering the
/// round trip if a colony exports and then needs the stock back — a few sols of
/// consumption, not most of a month.
///
/// Kept comfortably above the default one-sol convoy transit so a long route
/// still has slack. Balance dial; expect the harness to retune it.
pub const TRADE_RESERVE_SOLS: u32 = 3;

/// A record of one completed turn produced by [`TurnProcessor::advance`].
#[derive(Debug, Clone)]
pub struct TurnOutcome {
    /// Colony-sol counter after this advance.
    pub sol: u64,
    /// Calendar-month label after this advance, derived from [`Self::sol`].
    pub month: u64,
    /// Tech IDs that completed this sol.
    pub completed_techs: Vec<String>,
    /// Cargo deliveries that arrived this sol.
    pub cargo_delivered: Vec<CargoDeliveryRecord>,
    /// Inter-colony trade convoys that landed this sol (issue #332).
    pub convoy_arrivals: Vec<TradeConvoyArrival>,
    /// Environmental hazard outcomes rolled this colony-sol (empty when no hazards triggered).
    pub hazard_outcomes: Vec<crate::hazard::HazardOutcome>,
}

/// What [`TurnProcessor::run_systemwide_pipeline`] produced this sol.
///
/// A struct rather than a tuple: the pipeline now reports three independent
/// kinds of outcome, and positional returns stop being readable at that point.
#[derive(Debug, Clone, Default)]
struct SystemwideOutcome {
    /// Tech IDs completed by this sol's research application.
    completed_techs: Vec<String>,
    /// Interplanetary cargo shipments that landed.
    cargo_delivered: Vec<CargoDeliveryRecord>,
    /// Inter-colony trade convoys that landed.
    convoy_arrivals: Vec<TradeConvoyArrival>,
}

/// A record of one inter-colony trade convoy landing at its destination.
#[derive(Debug, Clone)]
pub struct TradeConvoyArrival {
    /// Convoy that arrived.
    pub convoy_id: uuid::Uuid,
    /// Colony that dispatched the cargo.
    pub from_colony: ColonyId,
    /// Colony that received it.
    pub to_colony: ColonyId,
    /// Commodity deposited.
    pub commodity_id: String,
    /// Amount deposited.
    pub amount: f64,
}

/// A record of one commodity quantity delivered to a colony pool.
#[derive(Debug, Clone)]
pub struct CargoDeliveryRecord {
    /// Shipment that arrived.
    pub shipment_id: uuid::Uuid,
    /// Colony that received the cargo.
    pub colony_id: ColonyId,
    /// Commodity deposited.
    pub commodity_id: String,
    /// Amount deposited.
    pub amount: f64,
}

/// Top-level in-memory game state.
///
/// Owns all live simulation sub-state. Persistence (`SQLite` snapshots) is
/// handled outside this struct between turns — never written during a turn.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct GameState {
    /// All colonies under player management.
    pub colonies: Vec<Colony>,
    /// Population data parallel to `colonies` (same index = same colony).
    pub populations: Vec<Population>,
    /// Colony-sol turn counter (monotonically increasing).
    pub sol: u64,
    /// Calendar-month label, derived from [`Self::sol`] each advance.
    ///
    /// Display only: no system is gated on it (issue #332).
    pub month: u64,
    /// Optional content registry used by the production step.
    ///
    /// When `None`, the production step is a no-op (used in tests that only
    /// exercise cadence logic). Set this before calling [`TurnProcessor::advance`]
    /// to activate real production resolution.
    pub registry: Option<ContentRegistry>,
    /// Needs configuration used for the per-turn needs resolution step.
    ///
    /// When `None`, the needs step is skipped. Set via [`GameState::with_needs`]
    /// to enable stability dynamics.
    pub needs_config: Option<NeedsConfig>,
    /// System-wide research pool: research drained from all colonies each turn.
    pub research_pool: SystemResearchPool,
    /// System-wide technology research state.
    pub tech_state: TechState,
    /// Optional tech registry (loaded from content pack); enables research-turn processing.
    pub tech_registry: Option<TechRegistry>,
    /// Directive store: active directives and manual-override registry.
    pub directive_store: DirectiveStore,
    /// Per-colony stability history for predictive warning trajectory.
    pub stability_trackers: HashMap<ColonyId, StabilityTracker>,
    /// Per-colony interrupt sensitivity configuration.
    ///
    /// Absent entries default to [`InterruptConfig::all_enabled`] (all sources on).
    pub interrupt_configs: HashMap<ColonyId, InterruptConfig>,
    /// Planetary trade network: infrastructure routes + per-colony overrides.
    pub trade_network: TradeNetwork,
    /// Active emigration gates: player-defined directed voluntary flow routes.
    pub emigration_gates: Vec<EmigrationGate>,
    /// In-transit migration batches (voluntary + forced + immigration waves).
    pub pending_migrations: Vec<PendingMigration>,
    /// Per-colony population history for predictive population warnings.
    pub population_trackers: HashMap<ColonyId, PopulationTracker>,
    /// System-wide orbital infrastructure registry (stations + constellations).
    pub orbital_registry: OrbitalRegistry,
    /// In-progress orbital station construction projects.
    pub orbital_construction_queue: Vec<crate::orbital::OrbitalConstructionProject>,

    // ── Phase 10: Difficulty / Menace / Victory ───────────────────────────
    /// Active difficulty preset.
    pub difficulty_preset: DifficultyPreset,
    /// Grade table used to derive [`DifficultyScalar`] from the active preset.
    pub difficulty_grade_table: DifficultyGradeTable,
    /// Current difficulty scalar (derived from preset + grade table).
    pub difficulty_scalar: DifficultyScalar,
    /// Runtime state of the existential clock, if a menace is active.
    ///
    /// `None` in sandbox mode or when no menace has been loaded.
    pub menace_state: Option<MenaceState>,
    /// Victory tracking state.
    pub victory_state: VictoryState,
    /// Cumulative research accumulated over the whole campaign (for the science victory).
    pub cumulative_research: u64,
    /// Whether the interstellar expedition has been launched (primary victory trigger).
    pub expedition_launched: bool,
    /// The victory condition that has been achieved, if any (set on first win).
    ///
    /// `None` until a victory condition is satisfied.  After it is set the engine
    /// blocks further commands and returns [`EngineError::GameOver`] unless the
    /// player activates sandbox-continue mode.
    pub victory: Option<VictoryCondition>,
    /// Whether the player has opted into sandbox-continue mode (issue #96).
    ///
    /// `true` after [`Command::ContinueSandbox`] or [`Command::ContinueAfterVictory`]
    /// is applied post-victory.  Mirrors `victory_state.sandbox_continue`.
    pub sandbox_mode: bool,
    // ── M1: Planet maps ───────────────────────────────────────────────────
    /// Surface hex maps, keyed by the body they belong to (issue #300).
    ///
    /// Empty until the first `Command::SeedPlanet`. Before #300 this was a
    /// single unkeyed `Option<PlanetMap>` with no body association at all,
    /// which meant every colony — wherever the player aimed it — landed on
    /// the one founding-planet map.
    ///
    /// A body gains an entry the first time its surface is needed, seeded
    /// deterministically from its own id (see [`BodyId::surface_seed`]), so a
    /// body's stored surface always matches the one `body_surface_preview`
    /// showed for it.
    pub planet_maps: HashMap<BodyId, PlanetMap>,
    /// The founding planet — which key in [`Self::planet_maps`] is "home".
    ///
    /// `None` until the first `Command::SeedPlanet`.
    pub home_body_id: Option<BodyId>,

    // ── Phase M1: Tech effects wired to live state ────────────────────────
    /// Building IDs unlocked by completed tech nodes.
    pub unlocked_buildings: HashSet<String>,
    /// Capability slugs unlocked by completed tech nodes.
    pub unlocked_capabilities: HashSet<String>,
    /// Commodity IDs unlocked by completed tech nodes.
    pub unlocked_commodities: HashSet<String>,
    /// Accumulated numeric tech bonuses (additive, per category).
    pub modifier_accumulator: ModifierAccumulator,
    /// Environmental hazard configuration loaded from content YAML.
    ///
    /// When `None`, hazard rolling is skipped.  Populate via the content pack
    /// loader and assign before calling [`TurnProcessor::advance`].
    pub hazard_config: Option<HazardConfig>,
    /// Master hazards toggle (issue #161 custom difficulty).
    ///
    /// When `false`, hazard rolls are short-circuited regardless of the
    /// `HazardProbability` scalar.  Defaults to `true` on new games.
    pub hazards_enabled: bool,
    /// Master building-maintenance toggle (issue #180 custom difficulty).
    ///
    /// When `false`, per-building `maintenance` draws are short-circuited
    /// regardless of the `MaintenanceConsumption` scalar. Defaults to `true`
    /// on new games so authored maintenance costs apply by default.
    pub maintenance_enabled: bool,
    /// Most recently activated menace definition, if any.
    ///
    /// Kept alongside `menace_state` so a menace-disabled → re-enabled
    /// toggle in the custom-difficulty menu can restore the previous
    /// definition without needing to reload the content pack.
    pub last_menace_definition: Option<crate::menace::MenaceDefinition>,

    /// System-zoom layer state: node map, hauler fleet, in-transit shipments, megaprojects.
    pub system_state: SystemState,
    /// Maps `(from_colony, to_colony)` pairs to the trade route UUID created by
    /// `BuildInfrastructure` so `DemolishInfrastructure` can remove the right route.
    ///
    /// Keys are stored in canonical order (smaller id first) to allow bidirectional lookup.
    pub infra_routes: HashMap<(ColonyId, ColonyId), Uuid>,

    // ── M8: Field expeditions (issue #103) ───────────────────────────────
    /// All field expeditions launched this campaign (active and terminal).
    pub expeditions: Vec<crate::expedition::Expedition>,

    // ── Tech-driven habitability mitigation (issue #185) ─────────────────
    /// Habitability attributes mitigated by completed tech, applied globally
    /// (retroactively) to every colony with a linked home body.
    pub habitability_mitigations: HashSet<crate::system::HabitabilityAttribute>,

    // ── Outposts (issue #233) ─────────────────────────────────────────────
    /// Lightweight, colony-anchored off-world presences — see
    /// [`crate::outpost::Outpost`].
    pub outposts: Vec<crate::outpost::Outpost>,

    // ── Body-scouting survey expeditions (issue #235) ─────────────────────
    /// Live state for probe/manned survey expeditions targeting system
    /// bodies — see [`crate::expedition::ExpeditionRegistry`]. Distinct from
    /// the older [`Self::expeditions`] (planet-hex field expeditions).
    pub expedition_registry: crate::expedition::ExpeditionRegistry,

    // ── Tech-driven expedition modifiers (issue #236) ─────────────────────
    /// System-wide survey-modifier bonus accumulated from every researched
    /// tech carrying [`TechEffect::SurveyModifierBonus`]; combined with a
    /// given expedition's own per-mission modifiers when resolving a survey.
    pub tech_survey_modifiers: crate::expedition::SurveyModifiers,
    /// Multiplicative scalar applied to a new survey expedition's transit
    /// turns, driven down from `1.0` by every researched tech carrying
    /// [`TechEffect::ReduceTransitTime`] (floored so transit never reaches
    /// zero turns).
    pub propulsion_transit_scalar: f32,

    // ── Outpost tech/range gating (issue #241) ─────────────────────────────
    /// Additive AU bonus to an outpost's max allowed range from its parent
    /// colony's home body, accumulated from every researched tech carrying
    /// [`TechEffect::ExtendOutpostRange`]. Combined with
    /// [`crate::outpost::BASE_OUTPOST_RANGE_AU`] scaled by
    /// `system_state.node_map.propulsion_level` (see
    /// [`crate::outpost::max_outpost_range_au`]).
    pub outpost_range_bonus_au: f32,

    // ── Site-preparation tech modifiers (issue #306) ───────────────────────
    /// Multiplicative scalar applied to a site-preparation project's material
    /// cost when it is queued, driven down from `1.0` by every researched tech
    /// carrying [`TechEffect::ReduceInfrastructureProject`].
    ///
    /// Floored, so expansion never becomes free however deep the tech tree runs.
    pub infrastructure_cost_scalar: f32,
    /// Multiplicative scalar applied to a site-preparation project's
    /// construction turns, accumulated the same way as
    /// [`Self::infrastructure_cost_scalar`].
    ///
    /// The resulting turn count is floored at `1` separately — a project that
    /// took zero turns would complete before the sol it was queued in.
    pub infrastructure_time_scalar: f32,
}

impl GameState {
    /// The founding planet's hex map, if one has been seeded.
    ///
    /// Shorthand for looking [`Self::home_body_id`] up in
    /// [`Self::planet_maps`] — the pre-#300 `planet_map` field's meaning,
    /// now derived rather than stored separately.
    #[must_use]
    pub fn home_map(&self) -> Option<&PlanetMap> {
        self.planet_maps.get(self.home_body_id.as_ref()?)
    }

    /// Mutable form of [`Self::home_map`].
    #[must_use]
    pub fn home_map_mut(&mut self) -> Option<&mut PlanetMap> {
        let home = self.home_body_id.clone()?;
        self.planet_maps.get_mut(&home)
    }

    /// The surface map for `body_id`, if that body has one.
    #[must_use]
    pub fn map_for_body(&self, body_id: &BodyId) -> Option<&PlanetMap> {
        self.planet_maps.get(body_id)
    }

    /// Mutable form of [`Self::map_for_body`].
    #[must_use]
    pub fn map_for_body_mut(&mut self, body_id: &BodyId) -> Option<&mut PlanetMap> {
        self.planet_maps.get_mut(body_id)
    }

    /// Which body owns `site_id`, searching every seeded surface.
    ///
    /// Site ids are allocated per-map, so the same numeric id can exist on
    /// more than one body. Callers that already know the intended body must
    /// prefer [`Self::map_for_body`]; this is for resolving a bare site id
    /// (and for detecting the ambiguity in tests).
    #[must_use]
    pub fn body_for_site(&self, site_id: crate::trade::SiteId) -> Option<BodyId> {
        self.planet_maps
            .iter()
            .find(|(_, pm)| pm.coord_for_site(site_id).is_some())
            .map(|(body_id, _)| body_id.clone())
    }

    /// Which body's surface a colony is placed on, if it is placed at all.
    ///
    /// A colony node lives in exactly one map, so this is unambiguous — unlike
    /// [`Self::body_for_site`], whose site ids repeat across bodies.
    #[must_use]
    pub fn body_hosting_colony(&self, colony_id: ColonyId) -> Option<BodyId> {
        self.planet_maps
            .iter()
            .find(|(_, pm)| pm.colonies.iter().any(|n| n.colony_id == colony_id))
            .map(|(body_id, _)| body_id.clone())
    }

    /// Construct a fresh `GameState` with no colonies and no content registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            colonies: Vec::new(),
            populations: Vec::new(),
            sol: 0,
            month: 0,
            registry: None,
            needs_config: None,
            research_pool: SystemResearchPool::new(),
            tech_state: TechState::new(),
            tech_registry: None,
            directive_store: DirectiveStore::default(),
            stability_trackers: HashMap::new(),
            interrupt_configs: HashMap::new(),
            trade_network: TradeNetwork::new(),
            emigration_gates: Vec::new(),
            pending_migrations: Vec::new(),
            population_trackers: HashMap::new(),
            orbital_registry: OrbitalRegistry::new(),
            orbital_construction_queue: Vec::new(),
            difficulty_preset: DifficultyPreset::Normal,
            difficulty_grade_table: default_grade_table(),
            difficulty_scalar: DifficultyScalar::new(),
            menace_state: None,
            victory_state: VictoryState::capstone_only(),
            cumulative_research: 0,
            expedition_launched: false,
            planet_maps: HashMap::new(),
            home_body_id: None,
            victory: None,
            sandbox_mode: false,
            unlocked_buildings: HashSet::new(),
            unlocked_capabilities: HashSet::new(),
            unlocked_commodities: HashSet::new(),
            modifier_accumulator: ModifierAccumulator::new(),
            hazard_config: None,
            hazards_enabled: true,
            maintenance_enabled: true,
            last_menace_definition: None,
            system_state: SystemState::new(),
            infra_routes: HashMap::new(),
            expeditions: Vec::new(),
            habitability_mitigations: HashSet::new(),
            outposts: Vec::new(),
            expedition_registry: crate::expedition::ExpeditionRegistry::default(),
            tech_survey_modifiers: crate::expedition::SurveyModifiers::default(),
            propulsion_transit_scalar: 1.0,
            outpost_range_bonus_au: 0.0,
            infrastructure_cost_scalar: 1.0,
            infrastructure_time_scalar: 1.0,
        }
    }

    /// Enable the needs resolution step with the given configuration.
    pub fn with_needs(&mut self, config: NeedsConfig) {
        self.needs_config = Some(config);
    }

    /// Add a colony with the given starting population count.
    pub fn add_colony(&mut self, colony: Colony, starting_pop: u64) {
        #[allow(clippy::cast_precision_loss)]
        self.populations.push(Population::new(starting_pop as f32));
        self.colonies.push(colony);
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

/// Processes turns against a [`GameState`], one sol at a time.
///
/// Each call to [`TurnProcessor::advance`] fires exactly one colony-sol, and
/// every system runs on that one cadence — there is no strategic-month
/// sub-pipeline any more (issue #332). `sols_per_month` survives only to derive
/// the calendar label in [`GameState::month`]; nothing branches on it.
///
/// RNG is a seeded [`ChaCha8Rng`] injected at construction — never a global
/// source — ensuring deterministic, reproducible turn resolution. That is what
/// makes fast-forwarding N sols identical to advancing N times
/// ([`crate::GameEngine::advance_until_interrupted`]).
#[derive(Debug)]
pub struct TurnProcessor {
    /// Colony-sols per calendar month — the divisor for the derived
    /// [`GameState::month`] label. Not a cadence: no system is gated on it
    /// (issue #332).
    sols_per_month: u64,
    /// Seeded RNG stream for deterministic turn resolution.
    rng: ChaCha8Rng,
}

impl TurnProcessor {
    /// Create a processor with the default cadence (30 sols/month) and given seed.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self::with_cadence(seed, DEFAULT_SOLS_PER_MONTH)
    }

    /// Create a processor with a configurable cadence and given seed.
    ///
    /// # Panics
    ///
    /// Panics if `sols_per_month` is zero.
    #[must_use]
    pub fn with_cadence(seed: u64, sols_per_month: u64) -> Self {
        assert!(sols_per_month > 0, "sols_per_month must be non-zero");
        Self {
            sols_per_month,
            rng: ChaCha8Rng::seed_from_u64(seed),
        }
    }

    /// Advance `state` by exactly one colony-sol.
    ///
    /// The sol is the game's **only** cadence (issue #332). Research, trade, and
    /// shipment delivery used to run on a nested 30-sol "strategic month"
    /// pipeline; they now run every sol like everything else, and `state.month`
    /// is a derived calendar label rather than a gate on any system.
    ///
    /// The unification is deliberately balance-neutral:
    ///
    /// - **Research** drains the whole pool toward the current project whenever
    ///   it runs (`tech::apply_research_turn_scaled`), and the pool is *fed*
    ///   per sol, so applying it per sol moves the same total RP — just in finer
    ///   increments, without progress sitting idle for up to 29 sols.
    /// - **Shipments** carry their remaining duration in sols now, converted
    ///   from the route's `travel_time_months` at dispatch, so a voyage takes
    ///   the same elapsed time it always did.
    /// - **Trade** now rebalances every sol instead of every 30. That one *is* a
    ///   behaviour change, and an intended one: the 30-sol gap is the sole
    ///   reason a need reserve sized in sols had to exist (#331).
    pub fn advance(&mut self, state: &mut GameState) -> TurnOutcome {
        state.sol += 1;
        // The month is a label derived from the sol counter, not a separate
        // clock — nothing branches on it.
        state.month = state.sol / self.sols_per_month;
        let hazard_outcomes = self.run_colony_sol_pipeline(state);
        let pipeline = Self::run_systemwide_pipeline(state);

        TurnOutcome {
            sol: state.sol,
            month: state.month,
            completed_techs: pipeline.completed_techs,
            cargo_delivered: pipeline.cargo_delivered,
            convoy_arrivals: pipeline.convoy_arrivals,
            hazard_outcomes,
        }
    }

    /// Colony-sol sub-pipeline: RNG advancement, growth, and hazard rolling.
    ///
    /// Returns hazard outcomes for colonies hit this sol. Effects (stability,
    /// commodity pool, population) are applied by the caller (`GameEngine::apply`).
    fn run_colony_sol_pipeline(
        &mut self,
        state: &mut GameState,
    ) -> Vec<crate::hazard::HazardOutcome> {
        let _tick: u64 = rand::RngCore::next_u64(&mut self.rng);

        if state.needs_config.is_none() {
            let growth_scalar = state
                .difficulty_scalar
                .scalar_for(&ModifiableQuantity::PopulationGrowth);
            for pop in &mut state.populations {
                pop.apply_growth_tick_with_scalar(growth_scalar);
            }
        }

        // ── Environmental hazard rolls ────────────────────────────────────
        let mut hazard_outcomes = Vec::new();
        // Master toggle from the custom-difficulty menu (#161). Skip rolling
        // entirely when hazards are disabled — cleaner than clamping the
        // HazardProbability scalar to zero.
        if state.hazards_enabled {
            if let Some(hazard_cfg) = &state.hazard_config.clone() {
                use rand::Rng as _;
                let hazard_prob_scalar = state
                    .difficulty_scalar
                    .scalar_for(&ModifiableQuantity::HazardProbability);
                for (colony, pop) in state.colonies.iter().zip(state.populations.iter()) {
                    let terrain: Option<String> = colony.terrain_id.clone();
                    let pool_entries: Vec<(String, f64)> = colony
                        .pool
                        .commodity_ids()
                        .map(|id| (id.to_owned(), colony.pool.amount(id)))
                        .collect();

                    for kind in HazardKind::ALL {
                        let rng_prob: f32 = self.rng.gen();
                        let rng_sev: f32 = self.rng.gen();
                        // Use lower 32 bits for commodity index selection (safe on all targets).
                        #[allow(clippy::cast_possible_truncation)]
                        let rng_comm: usize =
                            (rand::RngCore::next_u64(&mut self.rng) as u32) as usize;

                        // Apply difficulty scalar to hazard probability.
                        // Dividing rng_prob by hazard_prob_scalar is equivalent to scaling
                        // the effective probability: `rng_prob / s < p` ⟺ `rng_prob < p × s`.
                        // Clamped at a minimum to avoid division by zero.
                        let adjusted_rng_prob = rng_prob / hazard_prob_scalar.max(f32::EPSILON);

                        if let Some(outcome) = roll_hazard(
                            adjusted_rng_prob,
                            rng_sev,
                            rng_comm,
                            kind,
                            colony.id,
                            terrain.as_deref(),
                            hazard_cfg,
                            pop.count,
                            &pool_entries,
                        ) {
                            hazard_outcomes.push(outcome);
                        }
                    }
                }
            }
        } // end `if state.hazards_enabled`
        hazard_outcomes
    }

    /// Apply a flat list of [`TechEffect`]s to live [`GameState`].
    ///
    /// Called after each research-turn completion to wire unlocks and bonuses.
    pub fn apply_tech_effects(state: &mut GameState, effects: &[TechEffect]) {
        let mut mitigation_added = false;
        for effect in effects {
            match effect {
                TechEffect::UnlockBuilding { building_id } => {
                    state.unlocked_buildings.insert(building_id.clone());
                }
                TechEffect::UnlockCapability { capability_id } => {
                    state.unlocked_capabilities.insert(capability_id.clone());
                }
                TechEffect::UnlockCommodity { commodity_id } => {
                    state.unlocked_commodities.insert(commodity_id.clone());
                }
                TechEffect::Bonus { category, value } => {
                    // Map the generic category string to a ModifiableQuantity.
                    // For now we use ProductionRate with the category string as the
                    // building-id key — a future pass can refine the mapping.
                    let quantity = ModifiableQuantity::ProductionRate(category.clone());
                    state.modifier_accumulator.add(ModifierDescriptor::new(
                        quantity,
                        category.clone(),
                        *value,
                    ));
                }
                TechEffect::MitigateAttribute { attribute } => {
                    mitigation_added |= state.habitability_mitigations.insert(*attribute);
                }
                TechEffect::SurveyModifierBonus {
                    full_reveal_bonus,
                    partial_reveal_bonus,
                } => {
                    state.tech_survey_modifiers.full_reveal_bonus += full_reveal_bonus;
                    state.tech_survey_modifiers.partial_reveal_bonus += partial_reveal_bonus;
                }
                TechEffect::ReduceTransitTime { fraction } => {
                    let retained = (1.0 - fraction.clamp(0.0, 0.95)).max(0.05);
                    state.propulsion_transit_scalar =
                        (state.propulsion_transit_scalar * retained).max(0.05);
                }
                TechEffect::ExtendOutpostRange { bonus_au } => {
                    state.outpost_range_bonus_au += bonus_au.max(0.0);
                }
                TechEffect::ReduceInfrastructureProject {
                    cost_fraction,
                    time_fraction,
                } => {
                    state.infrastructure_cost_scalar = (state.infrastructure_cost_scalar
                        * (1.0 - cost_fraction.clamp(0.0, 0.95)))
                    .max(MIN_INFRASTRUCTURE_SCALAR);
                    state.infrastructure_time_scalar = (state.infrastructure_time_scalar
                        * (1.0 - time_fraction.clamp(0.0, 0.95)))
                    .max(MIN_INFRASTRUCTURE_SCALAR);
                }
            }
        }
        if mitigation_added {
            Self::recompute_habitability_modifiers(state);
        }
    }

    /// Recompute `habitability_modifier` for every colony with a linked home
    /// body, folding in `state.habitability_mitigations` (issue #185).
    ///
    /// Applies retroactively — colonies founded before a mitigating tech
    /// completed are updated immediately, matching how other global tech
    /// unlocks (capabilities, buildings) already take effect for existing
    /// colonies rather than only future ones.
    fn recompute_habitability_modifiers(state: &mut GameState) {
        let mitigations = &state.habitability_mitigations;
        let updates: Vec<(usize, f32)> = state
            .colonies
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let body_id = c.home_body_id.clone()?;
                let body = state.system_state.node_map.bodies.get(&body_id)?;
                Some((i, body.habitability_modifier_with_mitigations(mitigations)))
            })
            .collect();
        for (i, modifier) in updates {
            state.colonies[i].habitability_modifier = modifier;
        }
    }

    /// Strategic-month sub-pipeline.
    /// Per-colony stock that must be held back from trade export (issue #355).
    ///
    /// Combines two independent floors:
    ///
    /// 1. **The automatic need reserve.** `water`, `oxygen`, and `food_ration` are
    ///    tradeable cargo *and* survival consumables. Without a reserve, the flow
    ///    pass — which equalises stock between route endpoints — happily ships
    ///    away the water a colony's own colonists are about to drink. Sized at
    ///    [`TRADE_RESERVE_SOLS`] sols of consumption. Commodities no need consumes
    ///    (ores, components) get no entry from this half.
    ///
    ///    Housing is skipped: it's a colony *resource* now, not pool stock, so it
    ///    is never reachable from trade in the first place (issue #304).
    ///
    /// 2. **The player's `commodity_reserves`** (issue #308). Withholding stock
    ///    from industry but not from export made the control a half-measure — a
    ///    reserve read as "protect this" and protected it from one of three
    ///    discretionary consumers.
    ///
    /// The two combine by **maximum, not sum**. Both are floors on what remains,
    /// so the binding one is whichever is higher; adding them would withhold 800
    /// from a player who asked for 500 on top of a 300 need reserve, which is not
    /// what either party asked for.
    fn compute_trade_reserves(state: &GameState) -> Vec<std::collections::HashMap<String, f64>> {
        state
            .colonies
            .iter()
            .enumerate()
            .map(|(idx, colony)| {
                // Start from what the player asked to keep — present regardless of
                // whether a needs config is loaded, so the control works in a
                // bare-`GameState` game too.
                let mut floors = colony.commodity_reserves.clone();
                let Some(config) = state.needs_config.as_ref() else {
                    return floors;
                };
                // `populations` is index-aligned with `colonies`, the same
                // assumption the trade caller makes when it zips them.
                let Some(pop) = state.populations.get(idx) else {
                    return floors;
                };
                let population = f64::from(pop.count);
                for need in config
                    .needs
                    .iter()
                    .filter(|need| !matches!(need.scaling, crate::needs::NeedScaling::Housing))
                {
                    let needed = need.required_amount(population) * f64::from(TRADE_RESERVE_SOLS);
                    let slot = floors.entry(need.commodity_id.clone()).or_insert(0.0);
                    *slot = slot.max(needed);
                }
                floors
            })
            .collect()
    }

    /// System-wide sub-pipeline, run every sol (issue #332).
    ///
    /// Research progress, inter-colony trade, and shipment delivery. These
    /// used to be gated behind a 30-sol "strategic month"; they are now on the
    /// single sol cadence like every other system. See [`Self::advance`] for why
    /// that is balance-neutral for research and shipments, and deliberate for
    /// trade.
    ///
    /// Returns everything downstream needs to emit events for.
    fn run_systemwide_pipeline(state: &mut GameState) -> SystemwideOutcome {
        let mut completed_techs = Vec::new();

        // Drain research pool into tech progress if a registry is loaded.
        if let Some(reg) = state.tech_registry.as_ref() {
            // Clone to avoid borrow conflict; registry is read-only here.
            let reg_clone = reg.clone();
            let cost_scalar = state
                .difficulty_scalar
                .scalar_for(&ModifiableQuantity::ResearchCost);
            let result = crate::tech::apply_research_turn_scaled(
                &mut state.tech_state,
                &mut state.research_pool,
                &reg_clone,
                cost_scalar,
            );
            // Wire completed tech effects into live state.
            for effects in &result.new_effects {
                Self::apply_tech_effects(state, effects);
            }
            completed_techs = result.completed;
        }

        // ── Trade convoy arrivals ─────────────────────────────────────────
        // Deliberately *before* this sol's dispatch pass, for two reasons:
        //
        // - A convoy dispatched this sol must get a full sol of travel. If
        //   arrivals ran afterwards, the default one-sol route would be
        //   decremented to zero and delivered in the same sol it was sent —
        //   the instant teleport convoys exist to replace.
        // - Landing first means the dispatch pass sees the freshly-credited
        //   stock, so a colony can forward on what it just received.
        //
        // Unconditional, not inside the dispatch branch below: convoys already
        // in flight must still land when every route has been removed, or on a
        // sol with nothing new to send.
        let arrived = state.trade_network.advance_convoys();
        let mut convoy_arrivals: Vec<TradeConvoyArrival> = Vec::new();
        for convoy in arrived {
            let Some(colony) = state.colonies.iter_mut().find(|c| c.id == convoy.to_colony) else {
                // Destination gone (decommissioned, or promoted to something
                // else) while the cargo was travelling. The goods are lost
                // rather than returned to sender: a return trip would need a
                // second convoy and an origin that may equally be gone.
                continue;
            };
            colony.pool.deposit(&convoy.commodity_id, convoy.amount);
            convoy_arrivals.push(TradeConvoyArrival {
                convoy_id: convoy.id,
                from_colony: convoy.from_colony,
                to_colony: convoy.to_colony,
                commodity_id: convoy.commodity_id,
                amount: convoy.amount,
            });
        }

        // ── Auto trade flow ───────────────────────────────────────────────
        // Collect the shippable ids present in any colony pool.
        //
        // Colony-local resources can't appear here at all: they live in
        // `Colony.resources`, and only `Colony.pool` is iterated (issue #304).
        // That is the structural half of the guarantee — there is no flag for a
        // future caller to forget.
        //
        // The `tradeable` filter is the other half, for commodities that are
        // real cargo in every other respect but authored as non-shippable
        // (`oxygen` today). Before this, `CommodityDef::tradeable` was authored
        // on all 45 commodities and read by absolutely nothing, so every one of
        // them flowed over trade routes regardless.
        let mut commodity_set = std::collections::HashSet::new();
        for colony in &state.colonies {
            for id in colony.pool.commodity_ids() {
                // Must be a *declared, tradeable* commodity. An id the pack
                // doesn't declare as a commodity is either a resource that
                // leaked into this pool or a stale entry from a pre-#304 save;
                // shipping either would be wrong, so unknown means "don't
                // trade" rather than "assume it's fine".
                //
                // With no registry loaded there is nothing to validate against
                // and no content-driven trade to run, so everything present is
                // allowed — that's the bare-`GameState` path the older turn
                // tests exercise.
                let shippable = state
                    .registry
                    .as_ref()
                    .is_none_or(|reg| reg.commodity(id).is_some_and(|def| def.tradeable));
                if shippable {
                    commodity_set.insert(id.to_owned());
                }
            }
        }
        if !commodity_set.is_empty() && !state.trade_network.routes.is_empty() {
            let commodities: Vec<String> = commodity_set.into_iter().collect();
            let colony_ids: Vec<ColonyId> = state.colonies.iter().map(|c| c.id).collect();
            let mut pools: Vec<_> = state.colonies.iter().map(|c| c.pool.clone()).collect();

            let reserves = Self::compute_trade_reserves(state);

            let flow = crate::trade::run_trade_flow(
                &state.trade_network,
                &colony_ids,
                &mut pools,
                &commodities,
                &reserves,
            );

            // Write mutated pools back.
            for (colony, new_pool) in state.colonies.iter_mut().zip(pools) {
                colony.pool = new_pool;
            }

            // The dispatched cargo has already been debited from the senders'
            // pools and exists nowhere else — dropping it here would destroy
            // goods (issue #332).
            state.trade_network.convoys.extend(flow.dispatched);
        }

        // ── Cargo shipment delivery ───────────────────────────────────────
        // Decrement turns_remaining on all in-transit shipments, collect
        // those that have arrived (turns_remaining reaches zero).
        let mut arrived_ids: Vec<uuid::Uuid> = Vec::new();
        for (id, shipment) in &mut state.system_state.shipments {
            if shipment.turns_remaining > 0 {
                shipment.turns_remaining -= 1;
            }
            if shipment.turns_remaining == 0 {
                arrived_ids.push(*id);
            }
        }

        let mut delivery_records: Vec<CargoDeliveryRecord> = Vec::new();
        for id in arrived_ids {
            let arrived = state.system_state.shipments.remove(&id).unwrap();

            // Release hauler.
            if let Some(hauler) = state
                .system_state
                .hauler_fleet
                .haulers
                .get_mut(&arrived.hauler_id)
            {
                hauler.in_transit = false;
            }

            // Credit destination colony pool for each cargo line.
            if let Some(colony_id) = arrived.destination_colony {
                if let Some(colony) = state.colonies.iter_mut().find(|c| c.id == colony_id) {
                    for (commodity_id, amount) in &arrived.cargo {
                        colony.pool.deposit(commodity_id, *amount);
                        delivery_records.push(CargoDeliveryRecord {
                            shipment_id: arrived.id,
                            colony_id,
                            commodity_id: commodity_id.clone(),
                            amount: *amount,
                        });
                    }
                }
            }
        }

        SystemwideOutcome {
            completed_techs,
            cargo_delivered: delivery_records,
            convoy_arrivals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::colony::Colony;

    fn make_state() -> GameState {
        let mut state = GameState::new();
        state.add_colony(Colony::new("Alpha Base"), 100);
        state
    }

    #[test]
    fn turn_context_stores_cadence_and_number() {
        assert_eq!(DEFAULT_SOLS_PER_MONTH, 30);
    }

    #[test]
    fn advance_increments_sol() {
        let mut state = make_state();
        let mut proc = TurnProcessor::new(42);
        let outcome = proc.advance(&mut state);
        assert_eq!(outcome.sol, 1);
        assert_eq!(state.sol, 1);
    }

    #[test]
    /// The month is a label derived from the sol counter (issue #332).
    ///
    /// It no longer gates anything — research, trade, shipment delivery, and
    /// orbital construction all run every sol — so this pins the label
    /// arithmetic only. The old `cadences_fired`/`TurnCadence` bookkeeping is
    /// gone with the second cadence it existed to distinguish.
    fn the_month_label_tracks_the_sol_counter() {
        let mut state = make_state();
        let mut proc = TurnProcessor::with_cadence(0, 5);

        for sol in 1..5 {
            let out = proc.advance(&mut state);
            assert_eq!(out.sol, sol);
            assert_eq!(out.month, 0, "sol {sol} is still month 0");
        }
        let out = proc.advance(&mut state);
        assert_eq!(out.month, 1, "5 sols at 5 sols/month is month 1");
        assert_eq!(out.sol, 5);
    }

    #[test]
    fn advancing_a_month_of_sols_rolls_the_month_label_once() {
        const SOLS_PER_MONTH: u64 = 30;
        let mut state = make_state();
        let mut proc = TurnProcessor::with_cadence(99, SOLS_PER_MONTH);

        let mut label_changes = 0u64;
        let mut previous_month = state.month;
        for _ in 0..SOLS_PER_MONTH {
            let out = proc.advance(&mut state);
            if out.month != previous_month {
                label_changes += 1;
                previous_month = out.month;
            }
        }

        assert_eq!(state.sol, SOLS_PER_MONTH);
        assert_eq!(label_changes, 1, "the label should roll over exactly once");
        assert_eq!(state.month, 1);
    }

    #[test]
    fn deterministic_for_fixed_seed() {
        let run = |seed: u64| {
            let mut state = make_state();
            let mut proc = TurnProcessor::new(seed);
            for _ in 0..60 {
                proc.advance(&mut state);
            }
            (state.sol, state.month)
        };

        let a = run(1234);
        let b = run(1234);
        assert_eq!(a, b, "same seed must produce same outcome");

        let c = run(5678);
        assert_eq!(a.0, c.0);
        assert_eq!(a.1, c.1);
    }

    #[test]
    fn default_sols_per_month_is_thirty() {
        let mut state = make_state();
        let mut proc = TurnProcessor::new(0);
        for i in 1..30u64 {
            let out = proc.advance(&mut state);
            assert_eq!(
                out.month, 0,
                "month should not fire before sol 30 (at sol {i})"
            );
        }
        let out = proc.advance(&mut state);
        assert_eq!(out.month, 1);
    }

    #[test]
    fn game_state_tracks_colonies_and_populations() {
        let mut state = GameState::new();
        assert!(state.colonies.is_empty());
        state.add_colony(Colony::new("Outpost Alpha"), 200);
        assert_eq!(state.colonies.len(), 1);
        assert!((state.populations[0].count - 200.0).abs() < f32::EPSILON);
    }

    #[test]
    fn game_state_directives_and_manual_override_start_empty() {
        let state = GameState::new();
        assert!(state.directive_store.directives.is_empty());
        assert!(state.directive_store.manual_override.is_empty());
    }

    // ── Tech effects wiring tests (issue #81) ────────────────────────────────

    use crate::modifier::ModifiableQuantity;
    use crate::tech::{TechDef, TechEffect, TechRegistry};

    fn make_tech_registry_with_unlock(building_id: &str, cost: f32) -> (TechRegistry, String) {
        let tech_id = "unlock_test".to_string();
        let defs = vec![TechDef {
            id: tech_id.clone(),
            display_name: "Unlock Test".to_string(),
            prerequisites: vec![],
            research_cost: cost,
            effects: vec![TechEffect::UnlockBuilding {
                building_id: building_id.to_string(),
            }],
            ..TechDef::default()
        }];
        (TechRegistry::build(defs).unwrap(), tech_id)
    }

    #[test]
    fn unlock_building_applied_after_research_completes() {
        let mut state = make_state();
        let (reg, tech_id) = make_tech_registry_with_unlock("adv_lab", 10.0);
        state.tech_state.set_current_project(tech_id.clone());
        state.tech_registry = Some(reg);
        state.research_pool.deposit(20.0);

        // Advance until a strategic month fires (cadence 1 sol for speed).
        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);

        assert!(
            state.unlocked_buildings.contains("adv_lab"),
            "building should be unlocked after tech completes"
        );
        assert!(
            state.tech_state.is_researched(&tech_id),
            "tech should be marked as researched"
        );
    }

    #[test]
    fn unlock_capability_applied_after_research_completes() {
        let mut state = make_state();
        let tech_id = "warp_tech".to_string();
        let defs = vec![TechDef {
            id: tech_id.clone(),
            display_name: "Warp".to_string(),
            prerequisites: vec![],
            research_cost: 5.0,
            effects: vec![TechEffect::UnlockCapability {
                capability_id: "warp_drive".to_string(),
            }],
            ..TechDef::default()
        }];
        state.tech_state.set_current_project(tech_id);
        state.tech_registry = Some(TechRegistry::build(defs).unwrap());
        state.research_pool.deposit(10.0);

        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);

        assert!(state.unlocked_capabilities.contains("warp_drive"));
    }

    #[test]
    fn bonus_accumulates_in_modifier_accumulator() {
        let mut state = make_state();
        let tech_id = "efficiency_tech".to_string();
        let defs = vec![TechDef {
            id: tech_id.clone(),
            display_name: "Efficiency".to_string(),
            prerequisites: vec![],
            research_cost: 5.0,
            effects: vec![TechEffect::Bonus {
                category: "production_efficiency".to_string(),
                value: 0.20,
            }],
            ..TechDef::default()
        }];
        state.tech_state.set_current_project(tech_id);
        state.tech_registry = Some(TechRegistry::build(defs).unwrap());
        state.research_pool.deposit(10.0);

        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);

        let qty = ModifiableQuantity::ProductionRate("production_efficiency".to_string());
        let sum = state.modifier_accumulator.total_sum(&qty);
        assert!(
            (sum - 0.20).abs() < 1e-4,
            "expected 0.20 bonus in accumulator, got {sum}"
        );
    }

    #[test]
    fn extend_outpost_range_tech_effect_accumulates_additively() {
        let mut state = make_state();
        let tech_id = "long_range_logistics".to_string();
        let defs = vec![TechDef {
            id: tech_id.clone(),
            display_name: "Long-Range Logistics".to_string(),
            prerequisites: vec![],
            research_cost: 5.0,
            effects: vec![TechEffect::ExtendOutpostRange { bonus_au: 4.0 }],
            ..TechDef::default()
        }];
        state.tech_state.set_current_project(tech_id);
        state.tech_registry = Some(TechRegistry::build(defs).unwrap());
        state.research_pool.deposit(10.0);

        assert!((state.outpost_range_bonus_au - 0.0).abs() < 1e-6);

        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);

        assert!(
            (state.outpost_range_bonus_au - 4.0).abs() < 1e-4,
            "expected outpost_range_bonus_au to accumulate to 4.0, got {}",
            state.outpost_range_bonus_au
        );
    }

    #[test]
    fn mitigation_tech_retroactively_recomputes_existing_colony_modifier() {
        use crate::system::{Body, BodyKind, HabitabilityAttribute, RadiationLevel};

        let mut state = GameState::new();
        let mut body = Body::new("Harsh World", BodyKind::InnerPlanet, 1.0);
        body.radiation = RadiationLevel::High;
        let base_modifier = body.habitability_modifier();
        let body_id = state.system_state.node_map.add_body(body);

        // Colony founded (and linked) *before* the mitigating tech completes —
        // this is the retroactivity case (issue #185).
        state.add_colony(Colony::new("Outpost"), 100);
        state.colonies[0].home_body_id = Some(body_id.clone());
        state.colonies[0].habitability_modifier = base_modifier;

        let tech_id = "radiation_shielding".to_string();
        let defs = vec![TechDef {
            id: tech_id.clone(),
            display_name: "Radiation Shielding".to_string(),
            prerequisites: vec![],
            research_cost: 5.0,
            effects: vec![TechEffect::MitigateAttribute {
                attribute: HabitabilityAttribute::Radiation,
            }],
            ..TechDef::default()
        }];
        state.tech_state.set_current_project(tech_id);
        state.tech_registry = Some(TechRegistry::build(defs).unwrap());
        state.research_pool.deposit(10.0);

        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);

        assert!(state
            .habitability_mitigations
            .contains(&HabitabilityAttribute::Radiation));

        let body_after = state.system_state.node_map.bodies.get(&body_id).unwrap();
        let mut mitigations = std::collections::HashSet::new();
        mitigations.insert(HabitabilityAttribute::Radiation);
        let expected_modifier = body_after.habitability_modifier_with_mitigations(&mitigations);

        assert!(
            (state.colonies[0].habitability_modifier - expected_modifier).abs() < 1e-6,
            "expected retroactively-recomputed modifier {expected_modifier}, got {}",
            state.colonies[0].habitability_modifier
        );
        assert!(
            state.colonies[0].habitability_modifier > base_modifier,
            "mitigating High radiation should raise the modifier above the un-mitigated baseline"
        );
    }

    #[test]
    fn unavailable_building_excluded_before_unlock() {
        use crate::content::types::{BuildingCategory, BuildingDef};
        use std::collections::HashSet;

        let buildings = vec![BuildingDef {
            id: "adv_lab".to_string(),
            name: "Advanced Lab".to_string(),
            description: String::new(),
            category: BuildingCategory::Research,
            construction_cost: vec![],
            power_delta: 0.0,
            worker_slots: 2,
            labor_required: 1,
            slot_cost: 1,
            construction_turns: 3,
            tech_prerequisite: Some("adv_lab_tech".to_string()),
            maintenance: vec![],
            default_priority: crate::content::types::DEFAULT_BUILDING_PRIORITY,
            grants_slot_capacity: 0,
            starter_kit: false,
        }];

        let empty: HashSet<String> = HashSet::new();
        let available_before = Colony::available_buildings(buildings.iter(), &empty);
        assert!(
            available_before.is_empty(),
            "building should not be available before tech unlock"
        );

        let mut unlocked = HashSet::new();
        unlocked.insert("adv_lab_tech".to_string());
        let available_after = Colony::available_buildings(buildings.iter(), &unlocked);
        assert_eq!(
            available_after.len(),
            1,
            "building should be available after unlock"
        );
    }

    #[test]
    fn completed_techs_returned_in_turn_outcome() {
        let mut state = make_state();
        let (reg, tech_id) = make_tech_registry_with_unlock("shelter", 5.0);
        state.tech_state.set_current_project(tech_id.clone());
        state.tech_registry = Some(reg);
        state.research_pool.deposit(10.0);

        let mut proc = TurnProcessor::with_cadence(0, 1);
        let outcome = proc.advance(&mut state);

        assert!(
            outcome.completed_techs.contains(&tech_id),
            "outcome should list completed tech id"
        );
    }

    /// Issue #86: a shipment with ETA=1 strategic month delivers to colony pool.
    #[test]
    fn cargo_shipment_delivered_to_colony_pool_after_one_month() {
        use crate::system::{apply_system_command, BodyKind, SystemCommand};

        let mut state = GameState::new();
        let colony = Colony::new("Depot");
        let colony_id = colony.id;
        state.add_colony(colony, 100);

        // Set up two bodies in the system state.
        let sys = &mut state.system_state;
        let evs_a = apply_system_command(
            sys,
            &SystemCommand::AddBody {
                name: "Alpha".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 0.0,
            },
        )
        .unwrap();
        let evs_b = apply_system_command(
            sys,
            &SystemCommand::AddBody {
                name: "Beta".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.0,
            },
        )
        .unwrap();
        let body_a = match &evs_a[0] {
            crate::system::SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        let body_b = match &evs_b[0] {
            crate::system::SystemEvent::BodyAdded { body_id, .. } => body_id.clone(),
            _ => panic!("expected BodyAdded"),
        };
        apply_system_command(
            sys,
            &SystemCommand::AddShippingRoute {
                from: body_a.clone(),
                to: body_b.clone(),
            },
        )
        .unwrap();
        apply_system_command(sys, &SystemCommand::AddHauler { capacity: 100.0 }).unwrap();

        // Dispatch a shipment of 50 iron destined for the colony pool.
        // Bodies 1 AU apart → travel_time = 1 strategic month.
        apply_system_command(
            &mut state.system_state,
            &SystemCommand::DispatchShipment {
                from: body_a,
                to: body_b,
                cargo: vec![("iron".to_string(), 50.0)],
                destination_colony: Some(colony_id),
            },
        )
        .unwrap();

        assert_eq!(
            state.system_state.shipments.len(),
            1,
            "shipment should be in-transit"
        );

        // Advance 30 sols (1 strategic month). Delivery fires on the strategic-month pipeline.
        let mut proc = TurnProcessor::new(0);
        let mut delivery_records = Vec::new();
        for _ in 0..DEFAULT_SOLS_PER_MONTH {
            let out = proc.advance(&mut state);
            delivery_records.extend(out.cargo_delivered);
        }

        // Shipment should have been removed.
        assert!(
            state.system_state.shipments.is_empty(),
            "shipment should be removed after delivery"
        );

        // Colony pool should have been credited.
        let amount = state.colonies[0].pool.amount("iron");
        assert!(
            (amount - 50.0).abs() < f64::EPSILON,
            "colony pool should have 50 iron after delivery, got {amount}"
        );

        // Delivery records should contain one entry for 50 iron.
        assert_eq!(delivery_records.len(), 1);
        assert_eq!(delivery_records[0].colony_id, colony_id);
        assert_eq!(delivery_records[0].commodity_id, "iron");
        assert!((delivery_records[0].amount - 50.0).abs() < f64::EPSILON);
    }

    // ── Hazard pipeline integration tests ────────────────────────────────────

    use crate::hazard::{HazardConfig, HazardEntry, HazardKind, HazardKindConfig};

    fn all_kinds_config(probability: f32) -> HazardConfig {
        let kinds = HazardKind::ALL
            .iter()
            .map(|&kind| HazardEntry {
                kind,
                config: HazardKindConfig {
                    base_probability: probability,
                    severity_min: 0.5,
                    severity_max: 0.5,
                    stability_damage_per_severity: 0.1,
                    commodity_loss_per_severity: 0.05,
                    population_damage_per_severity: 0.01,
                },
                terrain_modifiers: Default::default(),
            })
            .collect();
        HazardConfig { kinds }
    }

    #[test]
    fn zero_probability_hazards_never_appear_in_turn_outcome() {
        let mut state = make_state();
        state.hazard_config = Some(all_kinds_config(0.0));
        let mut proc = TurnProcessor::new(42);
        let outcome = proc.advance(&mut state);
        assert!(
            outcome.hazard_outcomes.is_empty(),
            "no hazards should trigger at probability=0"
        );
    }

    #[test]
    fn probability_one_hazards_always_appear_in_turn_outcome() {
        let mut state = make_state();
        state.hazard_config = Some(all_kinds_config(1.0));
        let mut proc = TurnProcessor::new(0);
        let outcome = proc.advance(&mut state);
        // Each of the 6 kinds should fire for the 1 colony → 6 outcomes.
        assert_eq!(
            outcome.hazard_outcomes.len(),
            HazardKind::ALL.len(),
            "all 6 hazard kinds should trigger with probability=1.0"
        );
    }

    #[test]
    fn hazard_config_none_skips_hazard_rolling() {
        let mut state = make_state();
        // hazard_config is None by default — no rolling should occur.
        let mut proc = TurnProcessor::new(0);
        let outcome = proc.advance(&mut state);
        assert!(
            outcome.hazard_outcomes.is_empty(),
            "hazard rolling should be skipped when config is None"
        );
    }
    // ── Trade excludes non-shippable things (issue #304) ─────────────────────

    /// The auto trade pass must move tradeable commodities and nothing else.
    ///
    /// Two guarantees are checked at once, because they are enforced by
    /// different mechanisms:
    ///
    /// 1. **Colony resources are excluded structurally.** `power` lives in
    ///    `Colony.resources`, and the trade pass only iterates `Colony.pool`,
    ///    so no flag has to be checked — there is no code path.
    /// 2. **Non-tradeable commodities are excluded by their `tradeable` flag.**
    ///    `oxygen` is real cargo in every other respect but authored
    ///    unshippable. Before #304, `CommodityDef::tradeable` was authored on
    ///    every commodity and read by nothing, so it moved anyway.
    #[test]
    fn auto_trade_moves_tradeable_commodities_only() {
        use crate::content::types::{CommodityDef, Phase, ResourceDef, ResourceKind};
        use crate::content::{CommodityTier, ContentRegistry};
        use crate::trade::TradeRoute;

        fn commodity(id: &str, tradeable: bool) -> CommodityDef {
            CommodityDef {
                id: id.into(),
                name: id.into(),
                description: String::new(),
                category: "consumable".into(),
                phase: Phase::Solid,
                base_value: 1.0,
                tradeable,
                tier: CommodityTier::default(),
                weight: 1.0,
            }
        }

        let mut registry = ContentRegistry::default();
        registry.insert_commodity(commodity("water", true));
        registry.insert_commodity(commodity("oxygen", false));
        registry.insert_resource(ResourceDef {
            id: "power".into(),
            name: "Power".into(),
            description: String::new(),
            kind: ResourceKind::Flow,
            unit: "MW".into(),
        });

        let mut state = GameState::new();
        state.add_colony(Colony::new("Rich"), 100);
        state.add_colony(Colony::new("Poor"), 100);
        state.registry = Some(registry);

        // Rich has a surplus of all three; Poor has none.
        state.colonies[0].pool.deposit("water", 100.0);
        state.colonies[0].pool.deposit("oxygen", 100.0);
        state.colonies[0].resources.deposit("power", 100.0);

        let (rich, poor) = (state.colonies[0].id, state.colonies[1].id);
        state
            .trade_network
            .add_route(TradeRoute::new(rich, poor, 50.0));

        // Two sols: the first dispatches the convoy, the second lands it
        // (issue #332 — trade is no longer instantaneous).
        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);
        proc.advance(&mut state);

        assert!(
            state.colonies[1].pool.amount("water") > 0.0,
            "water is tradeable and should have flowed to the deficit colony"
        );
        assert_eq!(
            state.colonies[1].pool.amount("oxygen"),
            0.0,
            "oxygen is authored tradeable: false and must not flow"
        );
        assert_eq!(
            state.colonies[1].resources.amount("power"),
            0.0,
            "power is a colony resource and is not reachable from trade at all"
        );
        assert_eq!(
            state.colonies[1].pool.amount("power"),
            0.0,
            "power must not appear in the receiving colony's commodity pool either"
        );
    }
    /// A save written before issue #304 has `power` sitting in the *commodity*
    /// pool, because that is where it used to live. Production never tops it up
    /// again, but the entry is still there — and it must not become shippable
    /// cargo just because the pack no longer declares `power` as a commodity.
    #[test]
    fn a_stale_pre_resource_pool_entry_is_not_tradeable() {
        use crate::content::types::{CommodityDef, Phase, ResourceDef, ResourceKind};
        use crate::content::{CommodityTier, ContentRegistry};
        use crate::trade::TradeRoute;

        let mut registry = ContentRegistry::default();
        registry.insert_commodity(CommodityDef {
            id: "water".into(),
            name: "Water".into(),
            description: String::new(),
            category: "consumable".into(),
            phase: Phase::Liquid,
            base_value: 5.0,
            tradeable: true,
            tier: CommodityTier::default(),
            weight: 1.0,
        });
        registry.insert_resource(ResourceDef {
            id: "power".into(),
            name: "Power".into(),
            description: String::new(),
            kind: ResourceKind::Flow,
            unit: "MW".into(),
        });

        let mut state = GameState::new();
        state.add_colony(Colony::new("Legacy"), 100);
        state.add_colony(Colony::new("Neighbour"), 100);
        state.registry = Some(registry);

        // Simulate the loaded save: power in the commodity pool, not resources.
        state.colonies[0].pool.deposit("power", 500.0);
        state.colonies[0].pool.deposit("water", 100.0);

        let (a, b) = (state.colonies[0].id, state.colonies[1].id);
        state.trade_network.add_route(TradeRoute::new(a, b, 50.0));

        // Two sols: dispatch, then arrival (issue #332).
        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);
        proc.advance(&mut state);

        assert!(
            state.colonies[1].pool.amount("water") > 0.0,
            "the tradeable commodity should still flow"
        );
        assert_eq!(
            state.colonies[1].pool.amount("power"),
            0.0,
            "a stale commodity-pool `power` entry must not be shipped — `power` \
             is a colony resource now, so it is not a declared commodity"
        );
    }
    /// End-to-end: the reserve is actually wired into the strategic pass.
    ///
    /// The unit tests above drive `run_trade_flow` directly. This one goes
    /// through `TurnProcessor::advance` with a real `NeedsConfig`, so it also
    /// proves the reserve map is built from the config and populations rather
    /// than being computed and dropped.
    #[test]
    fn a_colony_does_not_trade_away_the_water_its_population_needs() {
        use crate::content::types::{CommodityDef, Phase};
        use crate::content::{CommodityTier, ContentRegistry};
        use crate::needs::{NeedDef, NeedScaling, NeedsConfig};
        use crate::trade::TradeRoute;

        let mut registry = ContentRegistry::default();
        registry.insert_commodity(CommodityDef {
            id: "water".into(),
            name: "Water".into(),
            description: String::new(),
            category: "consumable".into(),
            phase: Phase::Liquid,
            base_value: 5.0,
            tradeable: true,
            tier: CommodityTier::default(),
            weight: 1.0,
        });

        let mut state = GameState::new();
        state.add_colony(Colony::new("Wet"), 100);
        state.add_colony(Colony::new("Dry"), 100);
        state.registry = Some(registry);

        let mut config = NeedsConfig::default_survival();
        config.needs = vec![NeedDef {
            commodity_id: "water".into(),
            scaling: NeedScaling::PerCapita { rate: 0.15 },
            weight: 1.0,
        }];
        state.needs_config = Some(config);

        // 100 colonists × 0.15/sol × TRADE_RESERVE_SOLS = the reserve. Hold
        // exactly that much and nothing is spare.
        let reserve = 100.0 * 0.15 * f64::from(TRADE_RESERVE_SOLS);
        state.colonies[0].pool.deposit("water", reserve);

        let (a, b) = (state.colonies[0].id, state.colonies[1].id);
        state.trade_network.add_route(TradeRoute::new(a, b, 500.0));

        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);

        // Needs consumption during the sol will have drawn some water down, so
        // assert on what did *not* move rather than an exact remaining figure.
        assert_eq!(
            state.colonies[1].pool.amount("water"),
            0.0,
            "the water-rich colony held only its own reserve, so it must not \
             have exported any — it would have starved itself"
        );
    }
    // ── Player reserves gate export too (issue #355) ─────────────────────────

    /// A registry + two colonies joined by a fat route, for export tests.
    fn two_colonies_on_a_route(commodity: &str) -> GameState {
        use crate::content::types::{CommodityDef, Phase};
        use crate::content::{CommodityTier, ContentRegistry};
        use crate::trade::TradeRoute;

        let mut registry = ContentRegistry::default();
        registry.insert_commodity(CommodityDef {
            id: commodity.into(),
            name: commodity.into(),
            description: String::new(),
            category: "material".into(),
            phase: Phase::Solid,
            base_value: 5.0,
            tradeable: true,
            tier: CommodityTier::default(),
            weight: 1.0,
        });

        let mut state = GameState::new();
        state.add_colony(Colony::new("Source"), 100);
        state.add_colony(Colony::new("Sink"), 100);
        state.registry = Some(registry);
        let (a, b) = (state.colonies[0].id, state.colonies[1].id);
        state.trade_network.add_route(TradeRoute::new(a, b, 500.0));
        state
    }

    /// The gap #355 was filed for: a player reserve stopped industry but not
    /// export, so reserved stock was shipped to a neighbour regardless.
    ///
    /// `biomass` is deliberately *not* a population need here, so the automatic
    /// need reserve is zero and the player's floor is the only thing that can
    /// hold the stock.
    #[test]
    fn a_player_reserve_is_not_exported() {
        let mut state = two_colonies_on_a_route("biomass");
        state.colonies[0].pool.deposit("biomass", 500.0);
        state.colonies[0]
            .commodity_reserves
            .insert("biomass".to_string(), 500.0);

        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);
        proc.advance(&mut state); // dispatch sol, then arrival sol

        assert_eq!(
            state.colonies[1].pool.amount("biomass"),
            0.0,
            "the whole stock was reserved, so nothing was exportable"
        );
        assert!(
            (state.colonies[0].pool.amount("biomass") - 500.0).abs() < 1e-9,
            "the reserve must be intact, got {}",
            state.colonies[0].pool.amount("biomass")
        );
    }

    /// Only the surplus above the floor ships — a reserve is a floor, not an
    /// export ban.
    #[test]
    fn only_stock_above_the_reserve_is_exportable() {
        let mut state = two_colonies_on_a_route("biomass");
        state.colonies[0].pool.deposit("biomass", 500.0);
        state.colonies[0]
            .commodity_reserves
            .insert("biomass".to_string(), 300.0);

        let mut proc = TurnProcessor::with_cadence(0, 1);
        proc.advance(&mut state);
        proc.advance(&mut state);

        assert!(
            state.colonies[1].pool.amount("biomass") > 0.0,
            "200 was above the floor and should have moved"
        );
        assert!(
            state.colonies[0].pool.amount("biomass") >= 300.0 - 1e-9,
            "export must not dip below the floor, got {}",
            state.colonies[0].pool.amount("biomass")
        );
    }

    /// The player floor and the automatic need reserve combine by **maximum**,
    /// not by sum.
    ///
    /// Both are floors on what remains, so the binding one is whichever is
    /// higher. Summing them would withhold 450 from a player who asked for 300
    /// on top of a 150 need reserve — more than either party asked for.
    #[test]
    fn the_two_reserve_kinds_take_the_maximum_not_the_sum() {
        use crate::needs::{NeedDef, NeedScaling, NeedsConfig};

        let mut state = two_colonies_on_a_route("water");
        let mut config = NeedsConfig::default_survival();
        config.needs = vec![NeedDef {
            commodity_id: "water".into(),
            scaling: NeedScaling::PerCapita { rate: 0.05 },
            weight: 1.0,
        }];
        state.needs_config = Some(config);

        // Need reserve: 100 colonists × 0.05 × TRADE_RESERVE_SOLS.
        let need_reserve = 100.0 * 0.05 * f64::from(TRADE_RESERVE_SOLS);
        let player_reserve = need_reserve * 2.0;
        state.colonies[0]
            .commodity_reserves
            .insert("water".to_string(), player_reserve);

        let floors = TurnProcessor::compute_trade_reserves(&state);
        let combined = floors[0]["water"];
        assert!(
            (combined - player_reserve).abs() < 1e-9,
            "expected max({need_reserve}, {player_reserve}) = {player_reserve}, got {combined}"
        );
        assert!(
            combined < need_reserve + player_reserve - 1e-9,
            "must not be the sum"
        );
    }

    /// With no needs config the player's floor still applies — the control must
    /// not depend on survival needs being authored.
    #[test]
    fn a_player_reserve_applies_without_a_needs_config() {
        let mut state = two_colonies_on_a_route("biomass");
        assert!(state.needs_config.is_none());
        state.colonies[0]
            .commodity_reserves
            .insert("biomass".to_string(), 42.0);

        let floors = TurnProcessor::compute_trade_reserves(&state);
        assert!((floors[0]["biomass"] - 42.0).abs() < 1e-9);
        assert!(floors[1].is_empty(), "the other colony reserved nothing");
    }

    // ── One cadence (issue #332) ─────────────────────────────────────────────

    /// Research, trade, and shipment delivery run every sol, not every 30.
    ///
    /// The specific thing pinned here is that research *progress* happens on a
    /// sol that is not a month boundary. Before #332 it was gated behind
    /// `sol % 30 == 0`, so 29 sols out of 30 banked RP and applied none of it.
    #[test]
    fn research_progresses_on_a_non_boundary_sol() {
        use crate::tech::{TechDef, TechRegistry};

        let registry = TechRegistry::build(vec![TechDef {
            id: "cheap_tech".into(),
            display_name: "Cheap Tech".into(),
            research_cost: 5.0,
            ..TechDef::default()
        }])
        .expect("single tech with no prerequisites is a valid registry");

        let mut state = make_state();
        state.tech_registry = Some(registry);
        state
            .tech_state
            .research_queue
            .push_back("cheap_tech".into());
        state.research_pool.deposit(100.0);

        // A 30-sol month, so sol 1 is emphatically not a boundary.
        let mut proc = TurnProcessor::with_cadence(0, 30);
        let out = proc.advance(&mut state);

        assert_eq!(out.sol, 1);
        assert_eq!(out.month, 0, "sol 1 is not a month boundary");
        assert_eq!(
            out.completed_techs,
            vec!["cheap_tech".to_string()],
            "research must progress on an ordinary sol, not wait for the month"
        );
    }

    /// Trade rebalances every sol rather than once per 30.
    #[test]
    fn trade_flows_on_a_non_boundary_sol() {
        use crate::content::types::{CommodityDef, Phase};
        use crate::content::{CommodityTier, ContentRegistry};
        use crate::trade::TradeRoute;

        let mut registry = ContentRegistry::default();
        registry.insert_commodity(CommodityDef {
            id: "structural_ore".into(),
            name: "Ore".into(),
            description: String::new(),
            category: "raw_material".into(),
            phase: Phase::Solid,
            base_value: 1.0,
            tradeable: true,
            tier: CommodityTier::default(),
            weight: 1.0,
        });

        let mut state = make_state();
        state.add_colony(Colony::new("Second"), 100);
        state.registry = Some(registry);
        state.colonies[0].pool.deposit("structural_ore", 100.0);
        let (a, b) = (state.colonies[0].id, state.colonies[1].id);
        state.trade_network.add_route(TradeRoute::new(a, b, 50.0));

        let mut proc = TurnProcessor::with_cadence(0, 30);
        let out = proc.advance(&mut state);
        assert_eq!(out.month, 0, "sol 1 is not a month boundary");
        assert!(
            !state.trade_network.convoys.is_empty(),
            "the dispatch pass must run on an ordinary sol, not wait 30 of them"
        );

        // Sol 2, still inside the same month, lands it.
        let out = proc.advance(&mut state);
        assert_eq!(out.month, 0, "sol 2 is not a month boundary either");
        assert!(
            state.colonies[1].pool.amount("structural_ore") > 0.0,
            "the convoy must arrive on an ordinary sol too"
        );
    }
}
