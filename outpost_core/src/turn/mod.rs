//! Turn model — two-cadence turn processing.
//!
//! Outpost 3 uses two interleaved turn cadences (see `docs/DESIGN.md §4`):
//! - **Colony-sol**: the shorter operational cadence for day-to-day colony
//!   management (production, labour allocation, events).
//! - **Strategic-month**: the longer cadence for fleet movement, diplomacy,
//!   and star-system-scale actions.
//!
//! The [`TurnProcessor`] owns cadence bookkeeping and fires the strategic-month
//! sub-pipeline every `sols_per_month` sols (default 30). RNG is injected as a
//! seeded [`rand_chacha::ChaCha8Rng`] stream so turn resolution is deterministic.

use std::collections::{HashMap, HashSet};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::colony::{Colony, ColonyId};
use crate::content::ContentRegistry;
use crate::difficulty::{default_grade_table, DifficultyGradeTable, DifficultyPreset};
use crate::directive::DirectiveStore;
use crate::interrupt::StabilityTracker;
use crate::menace::MenaceState;
use crate::migration::{PendingMigration, PopulationTracker};
use crate::modifier::{
    DifficultyScalar, ModifiableQuantity, ModifierAccumulator, ModifierDescriptor,
};
use crate::needs::NeedsConfig;
use crate::orbital::OrbitalRegistry;
use crate::population::Population;
use crate::research::SystemResearchPool;
use crate::tech::TechEffect;
use crate::tech::{TechRegistry, TechState};
use crate::system::SystemState;
use crate::trade::TradeNetwork;
use crate::victory::{VictoryCondition, VictoryState};

/// Default number of colony-sols that constitute one strategic-month.
pub const DEFAULT_SOLS_PER_MONTH: u64 = 30;

/// Identifies which turn cadence is being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TurnCadence {
    /// Day-to-day colony operations cadence.
    ColonySol,
    /// Star-system-scale strategic cadence.
    StrategicMonth,
}

/// A record of one completed turn produced by [`TurnProcessor::advance`].
#[derive(Debug, Clone)]
pub struct TurnOutcome {
    /// Which cadence(s) fired this tick.
    pub cadences_fired: Vec<TurnCadence>,
    /// Colony-sol counter after this advance.
    pub sol: u64,
    /// Strategic-month counter after this advance (if a month just fired).
    pub month: u64,
    /// Tech IDs that completed during this strategic month (empty if no month fired).
    pub completed_techs: Vec<String>,
}

/// Top-level in-memory game state.
///
/// Owns all live simulation sub-state. Persistence (`SQLite` snapshots) is
/// handled outside this struct between turns — never written during a turn.
#[derive(Debug, Clone)]
pub struct GameState {
    /// All colonies under player management.
    pub colonies: Vec<Colony>,
    /// Population data parallel to `colonies` (same index = same colony).
    pub populations: Vec<Population>,
    /// Colony-sol turn counter (monotonically increasing).
    pub sol: u64,
    /// Strategic-month turn counter (monotonically increasing).
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
    /// Planetary trade network: infrastructure routes + per-colony overrides.
    pub trade_network: TradeNetwork,
    /// In-transit migration batches (voluntary + forced + immigration waves).
    pub pending_migrations: Vec<PendingMigration>,
    /// Per-colony population history for predictive population warnings.
    pub population_trackers: HashMap<ColonyId, PopulationTracker>,
    /// System-wide orbital infrastructure registry (stations + constellations).
    pub orbital_registry: OrbitalRegistry,

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
    /// System-zoom state: megaprojects, celestial bodies, hauler fleet.
    pub system_state: SystemState,

    // ── Phase M1: Tech effects wired to live state ────────────────────────
    /// Building IDs unlocked by completed tech nodes.
    pub unlocked_buildings: HashSet<String>,
    /// Capability slugs unlocked by completed tech nodes.
    pub unlocked_capabilities: HashSet<String>,
    /// Commodity IDs unlocked by completed tech nodes.
    pub unlocked_commodities: HashSet<String>,
    /// Accumulated numeric tech bonuses (additive, per category).
    pub modifier_accumulator: ModifierAccumulator,
}

impl GameState {
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
            trade_network: TradeNetwork::new(),
            pending_migrations: Vec::new(),
            population_trackers: HashMap::new(),
            orbital_registry: OrbitalRegistry::new(),
            difficulty_preset: DifficultyPreset::Normal,
            difficulty_grade_table: default_grade_table(),
            difficulty_scalar: DifficultyScalar::new(),
            menace_state: None,
            victory_state: VictoryState::capstone_only(),
            cumulative_research: 0,
            expedition_launched: false,
            victory: None,
            system_state: SystemState::new(),
            unlocked_buildings: HashSet::new(),
            unlocked_capabilities: HashSet::new(),
            unlocked_commodities: HashSet::new(),
            modifier_accumulator: ModifierAccumulator::new(),
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

/// Processes colony-sol and strategic-month turns against a [`GameState`].
///
/// Each call to [`TurnProcessor::advance`] fires exactly one colony-sol.
/// When `sol % sols_per_month == 0` (after increment) the strategic-month
/// sub-pipeline also fires. The sub-pipeline is a placeholder in Phase 1 and
/// will be wired to real mechanics in Phase 5+.
///
/// RNG is a seeded [`ChaCha8Rng`] injected at construction — never a global
/// source — ensuring deterministic, reproducible turn resolution.
#[derive(Debug)]
pub struct TurnProcessor {
    /// Number of colony-sols per strategic-month.
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
    /// Fires the strategic-month sub-pipeline when `state.sol % sols_per_month == 0`
    /// after incrementing. Returns a [`TurnOutcome`] describing what fired.
    pub fn advance(&mut self, state: &mut GameState) -> TurnOutcome {
        state.sol += 1;
        let mut cadences_fired = vec![TurnCadence::ColonySol];

        self.run_colony_sol_pipeline(state);

        let mut completed_techs = Vec::new();
        if state.sol.is_multiple_of(self.sols_per_month) {
            state.month += 1;
            cadences_fired.push(TurnCadence::StrategicMonth);
            completed_techs = Self::run_strategic_month_pipeline(state);
        }

        TurnOutcome {
            cadences_fired,
            sol: state.sol,
            month: state.month,
            completed_techs,
        }
    }

    /// Colony-sol sub-pipeline (cadence bookkeeping and RNG advancement).
    fn run_colony_sol_pipeline(&mut self, state: &mut GameState) {
        let _tick: u64 = rand::RngCore::next_u64(&mut self.rng);

        if state.needs_config.is_none() {
            for pop in &mut state.populations {
                pop.apply_growth_tick();
            }
        }
    }

    /// Apply a flat list of [`TechEffect`]s to live [`GameState`].
    ///
    /// Called after each research-turn completion to wire unlocks and bonuses.
    pub fn apply_tech_effects(state: &mut GameState, effects: &[TechEffect]) {
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
            }
        }
    }

    /// Strategic-month sub-pipeline.
    ///
    /// Returns the IDs of techs completed this month (may be empty).
    fn run_strategic_month_pipeline(state: &mut GameState) -> Vec<String> {
        let mut completed_techs = Vec::new();

        // Drain research pool into tech progress if a registry is loaded.
        if let Some(reg) = state.tech_registry.as_ref() {
            // Clone to avoid borrow conflict; registry is read-only here.
            let reg_clone = reg.clone();
            let result = crate::tech::apply_research_turn(
                &mut state.tech_state,
                &mut state.research_pool,
                &reg_clone,
            );
            // Wire completed tech effects into live state.
            for effects in &result.new_effects {
                Self::apply_tech_effects(state, effects);
            }
            completed_techs = result.completed;
        }

        // ── Auto trade flow ───────────────────────────────────────────────
        // Collect all commodity ids currently present in any colony pool.
        let mut commodity_set = std::collections::HashSet::new();
        for colony in &state.colonies {
            for id in colony.pool.commodity_ids() {
                commodity_set.insert(id.to_owned());
            }
        }
        if !commodity_set.is_empty() && !state.trade_network.routes.is_empty() {
            let commodities: Vec<String> = commodity_set.into_iter().collect();
            let colony_ids: Vec<ColonyId> = state.colonies.iter().map(|c| c.id).collect();
            let mut pools: Vec<_> = state.colonies.iter().map(|c| c.pool.clone()).collect();

            crate::trade::run_trade_flow(
                &state.trade_network,
                &colony_ids,
                &mut pools,
                &commodities,
            );

            // Write mutated pools back.
            for (colony, new_pool) in state.colonies.iter_mut().zip(pools) {
                colony.pool = new_pool;
            }
        }

        completed_techs
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
    fn strategic_month_fires_at_configured_interval() {
        let mut state = make_state();
        let mut proc = TurnProcessor::with_cadence(0, 5);

        for _ in 0..4 {
            let out = proc.advance(&mut state);
            assert!(!out.cadences_fired.contains(&TurnCadence::StrategicMonth));
            assert_eq!(out.month, 0);
        }
        let out = proc.advance(&mut state);
        assert!(out.cadences_fired.contains(&TurnCadence::StrategicMonth));
        assert_eq!(out.month, 1);
        assert_eq!(out.sol, 5);
    }

    #[test]
    fn advance_m_sols_and_assert_one_strategic_month() {
        const SOLS_PER_MONTH: u64 = 30;
        let mut state = make_state();
        let mut proc = TurnProcessor::with_cadence(99, SOLS_PER_MONTH);

        let mut months_fired = 0u64;
        for _ in 0..SOLS_PER_MONTH {
            let out = proc.advance(&mut state);
            if out.cadences_fired.contains(&TurnCadence::StrategicMonth) {
                months_fired += 1;
            }
        }

        assert_eq!(state.sol, SOLS_PER_MONTH);
        assert_eq!(months_fired, 1);
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
}
