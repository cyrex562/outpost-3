//! `SQLite` snapshot / restore for [`crate::turn::GameState`].
//!
//! # Architecture
//!
//! `SQLite` is used as a **checkpoint**, not a live per-mutation store.
//! The caller must:
//! 1. Run a full turn pipeline in memory via [`crate::GameEngine::apply`].
//! 2. Call [`Snapshot::save`] once per turn boundary to persist state.
//! 3. On resume, call [`Snapshot::load`] to restore the state and continue.
//!
//! One `.db` file per save; named saves = file copies.
//!
//! # Schema versioning
//!
//! The `schema_version` table stores the current schema integer.  Loading a
//! snapshot whose version differs from [`SCHEMA_VERSION`] returns
//! [`SnapshotError::SchemaMismatch`] so callers get a clear error rather than
//! silent corruption.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::colony::{Colony, ColonyId};
use crate::difficulty::{DifficultyGradeTable, DifficultyPreset};
use crate::directive::DirectiveStore;
use crate::expedition::{Expedition, ExpeditionRegistry};
use crate::interrupt::{InterruptConfig, StabilityTracker};
use crate::map::PlanetMap;
use crate::menace::MenaceState;
use crate::migration::{EmigrationGate, PendingMigration, PopulationTracker};
use crate::modifier::{DifficultyScalar, ModifierAccumulator};
use crate::orbital::{OrbitalConstructionProject, OrbitalRegistry};
use crate::population::Population;
use crate::research::SystemResearchPool;
use crate::system::{BodyId, SystemState};
use crate::tech::TechState;
use crate::trade::TradeNetwork;
use crate::turn::GameState;
use crate::victory::{VictoryCondition, VictoryState};

/// Monotonically increasing schema version.
///
/// Increment this whenever the on-disk layout changes.  Forward-migration
/// documentation must accompany each bump.
/// Schema version 2: `populations.count` changed from INTEGER to REAL.
/// Schema version 3: full `GameState` blob added (`state_json` column); `sandbox_mode` stored in blob.
/// Schema version 4: `expeditions` field added to `FullStateBlob`.
/// Schema version 5: `habitability_mitigations` field added to `FullStateBlob` (issue #185).
/// Schema version 6: `outposts` field added to `FullStateBlob` (issue #233).
/// Schema version 7: `expedition_registry` field added to `FullStateBlob` (issue #235).
/// Schema version 8: `tech_survey_modifiers`/`propulsion_transit_scalar` fields added (issue #236).
/// Schema version 9: `outpost_range_bonus_au` field added (issue #241).
/// Schema version 10: `infrastructure_cost_scalar`/`infrastructure_time_scalar` fields added (issue #306).
/// Schema version 11: `planet_map` replaced by body-keyed `planet_maps` + `home_body_id` (issue #300).
/// Schema version 12: `PlanetMap` topology changed from hex-of-radius-N (`radius`) to a
/// rectangular `width` × `height` east-west-wrapping grid (issue #315) — an old save's
/// serialized `PlanetMap`/`Command::SeedPlanet`/`Event::PlanetSeeded` shapes no longer
/// deserialize, so the version bump alone is what rejects them with a clear
/// [`SnapshotError::SchemaVersionMismatch`] rather than silently reshaping a player's
/// colony placements onto the new topology.
/// Schema version 13: `pending_colonies` field added to `FullStateBlob` (issue #359) —
/// directly-founded colonies now arrive after a distance-derived transit delay instead
/// of materialising instantly, and the in-flight queue must round-trip through snapshot.
pub const SCHEMA_VERSION: u32 = 13;

// ─── DDL ──────────────────────────────────────────────────────────────────────────────

const SCHEMA_SQL: &str = "
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS schema_version (
    version  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS game_state (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    sol        INTEGER NOT NULL,
    month      INTEGER NOT NULL,
    state_json TEXT    NOT NULL DEFAULT '{}'
);
";

// ─── Full-state blob ─────────────────────────────────────────────────────────────────

/// Serialisable mirror of [`GameState`] for `SQLite` persistence.
///
/// Fields that cannot be serialised (content registries, hazard config loaded
/// from YAML at runtime) are excluded; the caller must reload them after
/// [`Snapshot::load`] returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
struct FullStateBlob {
    // ── Turn counters ────────────────────────────────────────────────────────
    sol: u64,
    month: u64,

    // ── Colonies + populations ───────────────────────────────────────────────
    colonies: Vec<Colony>,
    populations: Vec<Population>,

    // ── Tech ────────────────────────────────────────────────────────────────
    tech_state: TechState,
    research_pool: SystemResearchPool,
    cumulative_research: u64,

    // ── Trade ────────────────────────────────────────────────────────────────
    trade_network: TradeNetwork,
    #[serde(default)]
    infra_routes: Vec<((ColonyId, ColonyId), uuid::Uuid)>,

    // ── Directives ───────────────────────────────────────────────────────────
    directive_store: DirectiveStore,

    // ── Orbital ──────────────────────────────────────────────────────────────
    orbital_registry: OrbitalRegistry,
    #[serde(default)]
    orbital_construction_queue: Vec<OrbitalConstructionProject>,

    // ── System state ─────────────────────────────────────────────────────────
    system_state: SystemState,

    // ── Migration ────────────────────────────────────────────────────────────
    emigration_gates: Vec<EmigrationGate>,
    pending_migrations: Vec<PendingMigration>,

    // ── Trackers ─────────────────────────────────────────────────────────────
    stability_trackers: HashMap<ColonyId, StabilityTracker>,
    population_trackers: HashMap<ColonyId, PopulationTracker>,

    // ── M7: Per-colony interrupt config ──────────────────────────────────────
    #[serde(default)]
    interrupt_configs: HashMap<ColonyId, InterruptConfig>,

    // ── Difficulty / Menace / Victory ────────────────────────────────────────
    difficulty_preset: DifficultyPreset,
    difficulty_grade_table: DifficultyGradeTable,
    difficulty_scalar: DifficultyScalar,
    menace_state: Option<MenaceState>,
    victory_state: VictoryState,
    expedition_launched: bool,
    victory: Option<VictoryCondition>,

    // ── Tech-derived unlocks + modifiers ─────────────────────────────────────
    unlocked_buildings: HashSet<String>,
    unlocked_capabilities: HashSet<String>,
    unlocked_commodities: HashSet<String>,
    modifier_accumulator: ModifierAccumulator,

    // ── Planet maps ───────────────────────────────────────────────────────────
    /// Surface maps keyed by body (issue #300).
    #[serde(default)]
    planet_maps: HashMap<BodyId, PlanetMap>,
    /// Which key in `planet_maps` is the founding planet (issue #300).
    #[serde(default)]
    home_body_id: Option<BodyId>,

    // ── Sandbox mode (issue #96) ──────────────────────────────────────────────
    #[serde(default)]
    sandbox_mode: bool,

    // ── In-progress expeditions (issue #139) ─────────────────────────────────
    #[serde(default)]
    expeditions: Vec<Expedition>,

    // ── #161 Custom difficulty ────────────────────────────────────────────────
    #[serde(default = "default_true")]
    hazards_enabled: bool,
    #[serde(default)]
    last_menace_definition: Option<crate::menace::MenaceDefinition>,

    // ── #180 Maintenance ──────────────────────────────────────────────────────
    #[serde(default = "default_true")]
    maintenance_enabled: bool,

    // ── #317 Deposit depletion ───────────────────────────────────────────────
    #[serde(default)]
    deposit_depletion_enabled: bool,

    // ── #185 Tech-driven habitability mitigation ────────────────────────────
    #[serde(default)]
    habitability_mitigations: HashSet<crate::system::HabitabilityAttribute>,

    // ── #233 Outposts ─────────────────────────────────────────────────────────
    #[serde(default)]
    outposts: Vec<crate::outpost::Outpost>,

    // ── #235 Body-scouting survey expeditions ────────────────────────────────
    #[serde(default)]
    expedition_registry: ExpeditionRegistry,

    // ── #236 Tech-driven expedition modifiers ────────────────────────────────
    #[serde(default)]
    tech_survey_modifiers: crate::expedition::SurveyModifiers,
    #[serde(default = "default_transit_scalar")]
    propulsion_transit_scalar: f32,

    // ── #241 Outpost tech/range gating ───────────────────────────────────────
    #[serde(default)]
    outpost_range_bonus_au: f32,

    // ── #306 Site-preparation tech modifiers ─────────────────────────────────
    #[serde(default = "default_infrastructure_scalar")]
    infrastructure_cost_scalar: f32,
    #[serde(default = "default_infrastructure_scalar")]
    infrastructure_time_scalar: f32,

    // ── #359 Direct-founding transit delay ───────────────────────────────────
    #[serde(default)]
    pending_colonies: Vec<crate::PendingColony>,
}

fn default_infrastructure_scalar() -> f32 {
    1.0
}

fn default_transit_scalar() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

impl FullStateBlob {
    fn from_game_state(state: &GameState) -> Self {
        // HashMap<(ColonyId, ColonyId), Uuid> → Vec for JSON (tuple keys not supported directly).
        let infra_routes: Vec<_> = state.infra_routes.iter().map(|(k, v)| (*k, *v)).collect();

        Self {
            sol: state.sol,
            month: state.month,
            colonies: state.colonies.clone(),
            populations: state.populations.clone(),
            tech_state: state.tech_state.clone(),
            research_pool: state.research_pool.clone(),
            cumulative_research: state.cumulative_research,
            trade_network: state.trade_network.clone(),
            infra_routes,
            directive_store: state.directive_store.clone(),
            orbital_registry: state.orbital_registry.clone(),
            orbital_construction_queue: state.orbital_construction_queue.clone(),
            system_state: state.system_state.clone(),
            emigration_gates: state.emigration_gates.clone(),
            pending_migrations: state.pending_migrations.clone(),
            stability_trackers: state.stability_trackers.clone(),
            population_trackers: state.population_trackers.clone(),
            interrupt_configs: state.interrupt_configs.clone(),
            difficulty_preset: state.difficulty_preset,
            difficulty_grade_table: state.difficulty_grade_table.clone(),
            difficulty_scalar: state.difficulty_scalar.clone(),
            menace_state: state.menace_state.clone(),
            victory_state: state.victory_state.clone(),
            expedition_launched: state.expedition_launched,
            victory: state.victory.clone(),
            unlocked_buildings: state.unlocked_buildings.clone(),
            unlocked_capabilities: state.unlocked_capabilities.clone(),
            unlocked_commodities: state.unlocked_commodities.clone(),
            modifier_accumulator: state.modifier_accumulator.clone(),
            planet_maps: state.planet_maps.clone(),
            home_body_id: state.home_body_id.clone(),
            sandbox_mode: state.sandbox_mode,
            expeditions: state.expeditions.clone(),
            hazards_enabled: state.hazards_enabled,
            last_menace_definition: state.last_menace_definition.clone(),
            maintenance_enabled: state.maintenance_enabled,
            deposit_depletion_enabled: state.deposit_depletion_enabled,
            habitability_mitigations: state.habitability_mitigations.clone(),
            outposts: state.outposts.clone(),
            expedition_registry: state.expedition_registry.clone(),
            tech_survey_modifiers: state.tech_survey_modifiers.clone(),
            propulsion_transit_scalar: state.propulsion_transit_scalar,
            infrastructure_cost_scalar: state.infrastructure_cost_scalar,
            infrastructure_time_scalar: state.infrastructure_time_scalar,
            outpost_range_bonus_au: state.outpost_range_bonus_au,
            pending_colonies: state.pending_colonies.clone(),
        }
    }

    fn into_game_state(mut self) -> GameState {
        let infra_routes: HashMap<(ColonyId, ColonyId), uuid::Uuid> =
            self.infra_routes.into_iter().collect();

        // Backfill building instance numbers (issue #307). A pre-#307 save has
        // every `ordinal` at 0, which would display as a bare type name and give
        // the player no way to tell two mines apart. Numbering on load is a
        // one-way migration: once assigned, the numbers are saved and stable.
        for colony in &mut self.colonies {
            backfill_ordinals(&mut colony.buildings);
        }
        for outpost in &mut self.outposts {
            backfill_ordinals(&mut outpost.buildings);
        }

        GameState {
            sol: self.sol,
            month: self.month,
            colonies: self.colonies,
            populations: self.populations,
            tech_state: self.tech_state,
            research_pool: self.research_pool,
            cumulative_research: self.cumulative_research,
            trade_network: self.trade_network,
            infra_routes,
            directive_store: self.directive_store,
            orbital_registry: self.orbital_registry,
            orbital_construction_queue: self.orbital_construction_queue,
            system_state: self.system_state,
            emigration_gates: self.emigration_gates,
            pending_migrations: self.pending_migrations,
            stability_trackers: self.stability_trackers,
            population_trackers: self.population_trackers,
            interrupt_configs: self.interrupt_configs,
            difficulty_preset: self.difficulty_preset,
            difficulty_grade_table: self.difficulty_grade_table,
            difficulty_scalar: self.difficulty_scalar,
            menace_state: self.menace_state,
            victory_state: self.victory_state,
            expedition_launched: self.expedition_launched,
            victory: self.victory,
            unlocked_buildings: self.unlocked_buildings,
            unlocked_capabilities: self.unlocked_capabilities,
            unlocked_commodities: self.unlocked_commodities,
            modifier_accumulator: self.modifier_accumulator,
            planet_maps: self.planet_maps,
            home_body_id: self.home_body_id,
            sandbox_mode: self.sandbox_mode,
            expeditions: self.expeditions,
            hazards_enabled: self.hazards_enabled,
            last_menace_definition: self.last_menace_definition,
            maintenance_enabled: self.maintenance_enabled,
            deposit_depletion_enabled: self.deposit_depletion_enabled,
            habitability_mitigations: self.habitability_mitigations,
            outposts: self.outposts,
            expedition_registry: self.expedition_registry,
            tech_survey_modifiers: self.tech_survey_modifiers,
            propulsion_transit_scalar: self.propulsion_transit_scalar,
            infrastructure_cost_scalar: self.infrastructure_cost_scalar,
            infrastructure_time_scalar: self.infrastructure_time_scalar,
            outpost_range_bonus_au: self.outpost_range_bonus_au,
            pending_colonies: self.pending_colonies,
            // Runtime-only fields that are reloaded from content packs after load:
            registry: None,
            needs_config: None,
            tech_registry: None,
            hazard_config: None,
        }
    }
}

/// Give every unnumbered building in `buildings` an instance number.
///
/// Numbers already assigned are left alone, and new ones continue after the
/// highest in use per type, so a partially-numbered list (a save written after
/// #307 but holding buildings placed before it) doesn't collide.
///
/// Ordered by `id` so the assignment is deterministic: the same save always
/// backfills to the same numbers, which matters because those numbers then
/// persist and the player sees them.
fn backfill_ordinals(buildings: &mut [crate::colony::PlacedBuilding]) {
    if buildings.iter().all(|b| b.ordinal != 0) {
        return;
    }

    let mut next: HashMap<String, u32> = HashMap::new();
    for b in buildings.iter() {
        let slot = next.entry(b.building_type.clone()).or_insert(0);
        *slot = (*slot).max(b.ordinal);
    }

    let mut unnumbered: Vec<&mut crate::colony::PlacedBuilding> =
        buildings.iter_mut().filter(|b| b.ordinal == 0).collect();
    unnumbered.sort_by_key(|b| b.id);
    for b in unnumbered {
        let slot = next.entry(b.building_type.clone()).or_insert(0);
        *slot = slot.saturating_add(1);
        b.ordinal = *slot;
    }
}

// ─── Errors ───────────────────────────────────────────────────────────────────────

/// Errors that can occur during snapshot save or load.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    /// `SQLite` operation failed.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// JSON serialisation or deserialisation failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// The on-disk schema version does not match [`SCHEMA_VERSION`].
    #[error("schema version mismatch: file has v{found}, code expects v{expected}")]
    SchemaMismatch {
        /// Version found in the database file.
        found: u32,
        /// Version this binary expects.
        expected: u32,
    },
    /// The snapshot file contained no game state (likely corrupt or empty).
    #[error("snapshot is missing required game_state row")]
    MissingGameState,
}

// ─── Snapshot ─────────────────────────────────────────────────────────────────────

/// Handle to a `SQLite` snapshot file.
///
/// Use [`Snapshot::open`] for a file on disk, or [`Snapshot::open_in_memory`]
/// for tests.
pub struct Snapshot {
    conn: Connection,
}

impl Snapshot {
    /// Open (or create) a snapshot at the given file path.
    ///
    /// Applies the schema DDL idempotently and writes `schema_version` if the
    /// file is new.  Existing files are validated against [`SCHEMA_VERSION`]
    /// at load time (in [`Snapshot::load`]), not here, so callers can still
    /// inspect a mismatched file if they need to.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::Sqlite`] if the file cannot be opened or the
    /// schema cannot be applied.
    pub fn open(path: &std::path::Path) -> Result<Self, SnapshotError> {
        let conn = Connection::open(path)?;
        let snap = Self { conn };
        snap.apply_schema()?;
        Ok(snap)
    }

    /// Open an in-memory snapshot (useful in tests without touching the filesystem).
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::Sqlite`] if the in-memory database cannot be
    /// initialised.
    pub fn open_in_memory() -> Result<Self, SnapshotError> {
        let conn = Connection::open_in_memory()?;
        let snap = Self { conn };
        snap.apply_schema()?;
        Ok(snap)
    }

    /// Write `state` as a full snapshot, replacing any previous data.
    ///
    /// This is the only correct time to call save: after the turn pipeline has
    /// completed in memory (i.e., after [`crate::GameEngine::apply`]).
    ///
    /// The operation runs inside a single transaction so a crash mid-write
    /// leaves the previous snapshot intact.
    ///
    /// # Errors
    ///
    /// Returns [`SnapshotError::Sqlite`] on any database error.
    /// Returns [`SnapshotError::Json`] if state serialisation fails.
    pub fn save(&mut self, state: &GameState) -> Result<(), SnapshotError> {
        let blob = FullStateBlob::from_game_state(state);
        let state_json = serde_json::to_string(&blob)?;

        let tx = self.conn.transaction()?;

        tx.execute_batch("DELETE FROM game_state;")?;

        tx.execute(
            "INSERT INTO game_state (id, sol, month, state_json) VALUES (1, ?1, ?2, ?3)",
            params![state.sol, state.month, state_json],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Restore a [`GameState`] from this snapshot.
    ///
    /// Validates the schema version before reading any data.
    ///
    /// # Errors
    ///
    /// - [`SnapshotError::SchemaMismatch`] if the file was written by a
    ///   different version of the code.
    /// - [`SnapshotError::MissingGameState`] if the snapshot has no turn data.
    /// - [`SnapshotError::Sqlite`] on any other database error.
    /// - [`SnapshotError::Json`] if the stored JSON cannot be deserialised.
    pub fn load(&self) -> Result<GameState, SnapshotError> {
        // Version check first.
        let found: u32 =
            self.conn
                .query_row("SELECT version FROM schema_version LIMIT 1", [], |row| {
                    row.get(0)
                })?;
        if found != SCHEMA_VERSION {
            return Err(SnapshotError::SchemaMismatch {
                found,
                expected: SCHEMA_VERSION,
            });
        }

        let state_json: String = self
            .conn
            .query_row(
                "SELECT state_json FROM game_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| SnapshotError::MissingGameState)?;

        let blob: FullStateBlob = serde_json::from_str(&state_json)?;
        Ok(blob.into_game_state())
    }

    /// Apply the DDL and initialise the `schema_version` row if absent.
    fn apply_schema(&self) -> Result<(), SnapshotError> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO schema_version (version) VALUES (?1)",
            params![SCHEMA_VERSION],
        )?;
        Ok(())
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::Colony;
    use crate::modifier::ModifiableQuantity;
    use crate::turn::GameState;
    use crate::{Command, GameEngine};

    // ── Building instance-number backfill (issue #307) ───────────────────────

    #[test]
    fn backfill_numbers_unnumbered_buildings_per_type() {
        let mut buildings = vec![
            crate::colony::PlacedBuilding::new("mine", 1),
            crate::colony::PlacedBuilding::new("mine", 1),
            crate::colony::PlacedBuilding::new("farm", 1),
        ];
        assert!(
            buildings.iter().all(|b| b.ordinal == 0),
            "starts unnumbered"
        );

        backfill_ordinals(&mut buildings);

        let mut mines: Vec<u32> = buildings
            .iter()
            .filter(|b| b.building_type == "mine")
            .map(|b| b.ordinal)
            .collect();
        mines.sort_unstable();
        assert_eq!(mines, vec![1, 2]);
        let farm = buildings
            .iter()
            .find(|b| b.building_type == "farm")
            .unwrap();
        assert_eq!(farm.ordinal, 1, "each type numbers from one");
    }

    /// A save written after #307 can still hold buildings placed before it. The
    /// backfill must continue past the numbers already in use rather than
    /// colliding with them.
    #[test]
    fn backfill_does_not_collide_with_numbers_already_assigned() {
        let mut numbered = crate::colony::PlacedBuilding::new("mine", 1);
        numbered.ordinal = 7;
        let mut buildings = vec![numbered, crate::colony::PlacedBuilding::new("mine", 1)];

        backfill_ordinals(&mut buildings);

        let mut ordinals: Vec<u32> = buildings.iter().map(|b| b.ordinal).collect();
        ordinals.sort_unstable();
        assert_eq!(ordinals, vec![7, 8], "continues past the highest in use");
    }

    /// The numbers get saved and shown to the player, so the same save must always
    /// backfill identically — hence ordering by id rather than by vec position.
    #[test]
    fn backfill_is_deterministic_for_the_same_set_of_buildings() {
        let a = crate::colony::PlacedBuilding::new("mine", 1);
        let b = crate::colony::PlacedBuilding::new("mine", 1);
        let c = crate::colony::PlacedBuilding::new("mine", 1);

        let assign = |mut v: Vec<crate::colony::PlacedBuilding>| {
            backfill_ordinals(&mut v);
            let mut pairs: Vec<(uuid::Uuid, u32)> = v.iter().map(|x| (x.id, x.ordinal)).collect();
            pairs.sort();
            pairs
        };

        let forward = assign(vec![a.clone(), b.clone(), c.clone()]);
        let reversed = assign(vec![c, b, a]);
        assert_eq!(forward, reversed, "same buildings, same numbers");
    }

    #[test]
    fn backfill_leaves_a_fully_numbered_list_alone() {
        let mut one = crate::colony::PlacedBuilding::new("mine", 1);
        one.ordinal = 3;
        let mut two = crate::colony::PlacedBuilding::new("mine", 1);
        two.ordinal = 9;
        let mut buildings = vec![one, two];

        backfill_ordinals(&mut buildings);

        assert_eq!(
            buildings.iter().map(|b| b.ordinal).collect::<Vec<_>>(),
            vec![3, 9],
            "nothing to backfill, nothing renumbered"
        );
    }

    fn make_state_with_colonies() -> GameState {
        let mut s = GameState::new();
        s.sol = 5;
        s.month = 1;
        s.add_colony(Colony::new("Alpha Base"), 100);
        s.add_colony(Colony::new("Beta Station"), 250);
        s
    }

    // ── round-trip fidelity ─────────────────────────────────────────────────────

    #[test]
    fn save_succeeds_with_trade_overrides_present() {
        // Regression: `TradeNetwork.overrides` is a `HashMap<(ColonyId,
        // String), _>` — a tuple key that `serde_json` cannot serialize as a
        // JSON object. A non-empty overrides map (set once the player tweaks
        // trade priorities) used to make every save fail with a JSON error.
        let mut snap = Snapshot::open_in_memory().unwrap();
        let mut state = make_state_with_colonies();
        let colony_id = state.colonies[0].id;
        state
            .trade_network
            .set_override(crate::trade::TradeOverride {
                colony_id,
                commodity_id: "water".into(),
                suppress_auto: true,
                cap: Some(5.0),
            });

        snap.save(&state)
            .expect("save must succeed with trade overrides");
        let restored = snap.load().unwrap();
        let ov = restored
            .trade_network
            .overrides
            .get(&(colony_id, "water".to_string()))
            .expect("override must survive the round trip");
        assert!(ov.suppress_auto);
        assert_eq!(ov.cap, Some(5.0));
    }

    #[test]
    fn round_trip_preserves_turn_counters() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let state = make_state_with_colonies();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();
        assert_eq!(restored.sol, 5);
        assert_eq!(restored.month, 1);
    }

    #[test]
    fn round_trip_preserves_colonies() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let state = make_state_with_colonies();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.colonies.len(), 2);
        assert_eq!(restored.colonies[0].id, state.colonies[0].id);
        assert_eq!(restored.colonies[0].name, "Alpha Base");
        assert_eq!(restored.colonies[1].id, state.colonies[1].id);
        assert_eq!(restored.colonies[1].name, "Beta Station");
    }

    #[test]
    fn round_trip_preserves_populations() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let state = make_state_with_colonies();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert!((restored.populations[0].count - 100.0).abs() < 1.0);
        assert!((restored.populations[0].stability - 1.0).abs() < 0.01);
        assert!((restored.populations[1].count - 250.0).abs() < 1.0);
    }

    #[test]
    fn round_trip_empty_state() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let state = GameState::new(); // no colonies, sol=0, month=0
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();
        assert_eq!(restored.sol, 0);
        assert_eq!(restored.month, 0);
        assert!(restored.colonies.is_empty());
        assert!(restored.populations.is_empty());
    }

    // ── version mismatch ─────────────────────────────────────────────────────

    #[test]
    fn version_mismatch_returns_clear_error() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.conn
            .execute("UPDATE schema_version SET version = 999", [])
            .unwrap();

        let state = GameState::new();
        snap.save(&state).unwrap();

        let err = snap.load().unwrap_err();
        assert!(
            matches!(
                err,
                SnapshotError::SchemaMismatch {
                    found: 999,
                    expected: SCHEMA_VERSION
                }
            ),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn version_mismatch_error_message_is_informative() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.conn
            .execute("UPDATE schema_version SET version = 42", [])
            .unwrap();
        snap.save(&GameState::new()).unwrap();
        let err = snap.load().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("42"),
            "error message should contain found version"
        );
        assert!(
            msg.contains(&SCHEMA_VERSION.to_string()),
            "error message should contain expected version"
        );
    }

    // ── full state round-trip (complex state) ─────────────────────────────────

    #[test]
    fn round_trip_tech_state() {
        use crate::tech::{TechDef, TechEffect, TechRegistry};

        let mut state = make_state_with_colonies();
        // Mark a tech as researched.
        let tech_id = "power_grid_v2".to_string();
        let defs = vec![TechDef {
            id: tech_id.clone(),
            display_name: "Power Grid v2".to_string(),
            prerequisites: vec![],
            research_cost: 10.0,
            effects: vec![TechEffect::UnlockBuilding {
                building_id: "adv_reactor".to_string(),
            }],
            ..TechDef::default()
        }];
        state.tech_registry = Some(TechRegistry::build(defs).unwrap());
        state.tech_state.set_current_project(tech_id.clone());
        state.research_pool.deposit(50.0);
        state.cumulative_research = 100;

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.cumulative_research, 100);
        // research_pool available should round-trip.
        assert!((restored.research_pool.total() - state.research_pool.total()).abs() < 1e-4);
    }

    #[test]
    fn round_trip_trade_network() {
        use crate::trade::TradeRoute;

        let mut state = GameState::new();
        state.add_colony(Colony::new("Alpha"), 100);
        state.add_colony(Colony::new("Beta"), 200);
        let id_a = state.colonies[0].id;
        let id_b = state.colonies[1].id;

        let route = TradeRoute::new(id_a, id_b, 50.0);
        let route_id = route.id;
        state.trade_network.routes.push(route);

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.trade_network.routes.len(), 1);
        assert_eq!(restored.trade_network.routes[0].id, route_id);
        assert!((restored.trade_network.routes[0].throughput_cap - 50.0).abs() < 1e-4);
    }

    /// In-flight convoys hold cargo that has already been debited from the
    /// sender's pool and exists nowhere else, so a snapshot that drops them
    /// destroys goods across a save/load (issue #332).
    #[test]
    fn round_trip_trade_convoys_in_flight() {
        use crate::trade::{TradeConvoy, TradeRoute};

        let mut state = GameState::new();
        state.add_colony(Colony::new("Alpha"), 100);
        state.add_colony(Colony::new("Beta"), 200);
        let id_a = state.colonies[0].id;
        let id_b = state.colonies[1].id;

        let route = TradeRoute::with_transit(id_a, id_b, 50.0, 4);
        let route_id = route.id;
        state.trade_network.routes.push(route);
        state.trade_network.convoys.push(TradeConvoy {
            id: uuid::Uuid::new_v4(),
            route_id,
            from_colony: id_a,
            to_colony: id_b,
            commodity_id: "water".into(),
            amount: 37.5,
            sols_remaining: 3,
        });

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.trade_network.routes[0].transit_sols, 4);
        assert_eq!(restored.trade_network.convoys.len(), 1);
        let convoy = &restored.trade_network.convoys[0];
        assert_eq!(convoy.to_colony, id_b);
        assert_eq!(convoy.commodity_id, "water");
        assert_eq!(convoy.sols_remaining, 3);
        assert!((convoy.amount - 37.5).abs() < 1e-6);
    }

    #[test]
    fn round_trip_directive_store() {
        use crate::directive::Directive;
        use crate::predicate::Predicate;

        let mut state = GameState::new();
        state.add_colony(Colony::new("Alpha"), 100);
        let colony_id = state.colonies[0].id;

        let directive = Directive::new(colony_id, Predicate::Always, Command::AdvanceColonySol, 10);
        let directive_id = directive.id;
        state.directive_store.directives.push(directive);

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.directive_store.directives.len(), 1);
        assert_eq!(restored.directive_store.directives[0].id, directive_id);
    }

    #[test]
    fn round_trip_orbital_registry() {
        use crate::orbital::{OrbitType, OrbitalStation, StationType};

        let mut state = GameState::new();
        state.add_colony(Colony::new("Alpha"), 100);
        let colony_id = state.colonies[0].id;

        let station =
            OrbitalStation::new(StationType::Habitat, OrbitType::Low, colony_id, None, 2).unwrap();
        let station_id = station.id;
        state.orbital_registry.stations.push(station);

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.orbital_registry.stations.len(), 1);
        assert_eq!(restored.orbital_registry.stations[0].id, station_id);
        assert_eq!(
            restored.orbital_registry.stations[0].orbit_type,
            OrbitType::Low
        );
    }

    #[test]
    fn round_trip_pending_migrations() {
        use crate::migration::PendingMigration;

        let mut state = GameState::new();
        state.add_colony(Colony::new("Origin"), 500);
        state.add_colony(Colony::new("Destination"), 100);
        let from = state.colonies[0].id;
        let to = state.colonies[1].id;

        let migration = PendingMigration::new(Some(from), to, 50.0, 10, false, 5.0);
        let migration_id = migration.id;
        state.pending_migrations.push(migration);

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.pending_migrations.len(), 1);
        assert_eq!(restored.pending_migrations[0].id, migration_id);
        assert_eq!(restored.pending_migrations[0].turns_remaining, 10);
        assert!((restored.pending_migrations[0].count - 50.0).abs() < 1e-4);
    }

    /// A directly-founded colony in transit must round-trip through a
    /// snapshot exactly like any other in-flight state (issue #359).
    #[test]
    fn round_trip_pending_colonies() {
        use crate::map::HexCoord;
        use crate::system::{Body, BodyKind};
        use crate::trade::SiteId;
        use crate::PendingColony;

        let mut state = GameState::new();
        state.add_colony(Colony::new("Sponsor"), 500);
        let sponsor_id = state.colonies[0].id;
        let target_body = Body::new("Target", BodyKind::InnerPlanet, 1.5).id;

        let pending = PendingColony {
            id: uuid::Uuid::new_v4(),
            name: "In Transit".into(),
            starting_population: 120,
            site_id: SiteId::new(),
            coord: HexCoord { q: 1, r: 2 },
            body_id: target_body.clone(),
            link_body_id: Some(target_body.clone()),
            focus: None,
            supplies: None,
            sponsor_colony_id: Some(sponsor_id),
            home_body_modifier: None,
            sols_remaining: 7,
        };
        let pending_id = pending.id;
        state.pending_colonies.push(pending);

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.pending_colonies.len(), 1);
        assert_eq!(restored.pending_colonies[0].id, pending_id);
        assert_eq!(restored.pending_colonies[0].sols_remaining, 7);
        assert_eq!(restored.pending_colonies[0].body_id, target_body);
    }

    #[test]
    fn round_trip_stability_and_population_trackers() {
        use crate::interrupt::StabilityTracker;
        use crate::migration::PopulationTracker;

        let mut state = GameState::new();
        state.add_colony(Colony::new("Alpha"), 100);
        let colony_id = state.colonies[0].id;

        let mut st = StabilityTracker::default();
        st.push(0.8);
        st.push(0.75);
        state.stability_trackers.insert(colony_id, st);

        let mut pt = PopulationTracker::default();
        pt.push(100.0);
        pt.push(105.0);
        state.population_trackers.insert(colony_id, pt);

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert!(restored.stability_trackers.contains_key(&colony_id));
        assert!(restored.population_trackers.contains_key(&colony_id));
    }

    #[test]
    fn round_trip_system_state() {
        use crate::system::{apply_system_command, BodyKind, SystemCommand};

        let mut state = GameState::new();
        apply_system_command(
            &mut state.system_state,
            &SystemCommand::AddBody {
                name: "Mars".into(),
                kind: BodyKind::InnerPlanet,
                distance_au: 1.52,
            },
        )
        .unwrap();
        apply_system_command(
            &mut state.system_state,
            &SystemCommand::AddHauler { capacity: 200.0 },
        )
        .unwrap();

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(
            restored.system_state.node_map.bodies.len(),
            state.system_state.node_map.bodies.len()
        );
        assert_eq!(
            restored.system_state.hauler_fleet.haulers.len(),
            state.system_state.hauler_fleet.haulers.len()
        );
    }

    #[test]
    fn round_trip_unlocks_and_modifiers() {
        let mut state = GameState::new();
        state.unlocked_buildings.insert("advanced_lab".to_string());
        state.unlocked_capabilities.insert("warp_drive".to_string());
        state.unlocked_commodities.insert("antimatter".to_string());

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert!(restored.unlocked_buildings.contains("advanced_lab"));
        assert!(restored.unlocked_capabilities.contains("warp_drive"));
        assert!(restored.unlocked_commodities.contains("antimatter"));
    }

    #[test]
    fn round_trip_difficulty_and_victory() {
        use crate::difficulty::DifficultyPreset;

        let mut state = GameState::new();
        state.difficulty_preset = DifficultyPreset::Hard;
        state.expedition_launched = true;
        state.cumulative_research = 9999;

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(restored.difficulty_preset, DifficultyPreset::Hard);
        assert!(restored.expedition_launched);
        assert_eq!(restored.cumulative_research, 9999);
    }

    // ── save → load → continue reproduces identical subsequent turns ─────────

    #[test]
    fn save_load_continue_identical_turns() {
        const SEED: u64 = 42;

        let mut engine_a = GameEngine::with_seed(SEED);
        engine_a
            .apply(&Command::FoundColony {
                name: "Persist Colony".into(),
                starting_population: 500,
            })
            .unwrap();
        for _ in 0..10 {
            engine_a.apply(&Command::AdvanceColonySol).unwrap();
        }

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&engine_a.state).unwrap();

        let restored_state = snap.load().unwrap();
        let mut engine_b = GameEngine::with_seed(SEED);
        engine_b.state = restored_state;

        assert_eq!(engine_a.sol(), engine_b.sol());
        assert_eq!(engine_a.month(), engine_b.month());
        assert_eq!(engine_a.state.colonies.len(), engine_b.state.colonies.len());
        assert_eq!(engine_a.state.colonies[0].id, engine_b.state.colonies[0].id);
        assert_eq!(
            engine_a.state.populations[0].count,
            engine_b.state.populations[0].count
        );

        let events_a = engine_a.apply(&Command::AdvanceColonySol).unwrap();
        let events_b = engine_b.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(engine_a.sol(), engine_b.sol());
        assert_eq!(events_a.len(), events_b.len());
        if let (
            Command::AdvanceColonySol,
            crate::Event::ColonySolAdvanced { sol: sol_a },
            crate::Event::ColonySolAdvanced { sol: sol_b },
        ) = (Command::AdvanceColonySol, &events_a[0], &events_b[0])
        {
            assert_eq!(sol_a, sol_b);
        }
    }

    // ── overwrite (save multiple times) ────────────────────────────────────

    #[test]
    fn second_save_overwrites_first() {
        let mut snap = Snapshot::open_in_memory().unwrap();

        let mut state1 = GameState::new();
        state1.sol = 1;
        snap.save(&state1).unwrap();

        let mut state2 = GameState::new();
        state2.sol = 99;
        state2.month = 3;
        snap.save(&state2).unwrap();

        let restored = snap.load().unwrap();
        assert_eq!(restored.sol, 99);
        assert_eq!(restored.month, 3);
    }

    // ── sandbox_mode round-trip (issue #96) ─────────────────────────────
    #[test]
    fn round_trip_preserves_sandbox_mode_false() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let state = GameState::new();
        assert!(!state.sandbox_mode);
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();
        assert!(
            !restored.sandbox_mode,
            "sandbox_mode should survive as false"
        );
    }

    #[test]
    fn round_trip_preserves_sandbox_mode_true() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let mut state = GameState::new();
        state.sandbox_mode = true;
        state.victory_state.activate_sandbox_continue();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();
        assert!(restored.sandbox_mode, "sandbox_mode should survive as true");
        assert!(
            restored.victory_state.sandbox_continue,
            "victory_state.sandbox_continue must mirror sandbox_mode on load"
        );
    }

    // ── expeditions round-trip (issue #139) ──────────────────────────────

    #[test]
    fn round_trip_preserves_in_transit_expedition() {
        use crate::expedition::{Expedition, ExpeditionStatus};
        use crate::map::HexCoord;

        let mut state = GameState::new();
        state.add_colony(Colony::new("Launch Base"), 500);
        let colony_id = state.colonies[0].id;

        let exp = Expedition {
            id: crate::expedition::FieldExpeditionId::new(),
            origin_colony: colony_id,
            target_hex: HexCoord { q: 3, r: -1 },
            crew_count: 8,
            supply_consumed_per_sol: 2.5,
            sol_launched: 10,
            eta_sol: 25,
            status: ExpeditionStatus::InTransit,
            supplies_remaining: 87.5,
            sol_arrived: None,
            discovered_resources: vec![],
            is_deep_space: false,
        };
        let exp_id = exp.id;
        state.expeditions.push(exp);

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();

        assert_eq!(
            restored.expeditions.len(),
            1,
            "expedition count must survive round-trip"
        );
        let r = &restored.expeditions[0];
        assert_eq!(r.id, exp_id, "expedition id must be stable");
        assert_eq!(r.origin_colony, colony_id);
        assert_eq!(r.crew_count, 8);
        assert_eq!(r.eta_sol, 25);
        assert_eq!(r.status, ExpeditionStatus::InTransit);
        assert!((r.supplies_remaining - 87.5).abs() < 1e-4);
    }

    // ── on-disk file round-trip ───────────────────────────────────────────

    #[test]
    fn file_round_trip() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("game.db");

        {
            let mut snap = Snapshot::open(&path).unwrap();
            let mut state = GameState::new();
            state.sol = 7;
            state.add_colony(Colony::new("Disk Colony"), 300);
            snap.save(&state).unwrap();
        }

        {
            let snap = Snapshot::open(&path).unwrap();
            let restored = snap.load().unwrap();
            assert_eq!(restored.sol, 7);
            assert_eq!(restored.colonies.len(), 1);
            assert_eq!(restored.colonies[0].name, "Disk Colony");
        }
    }

    /// A real game always has a generated planet map, and `PlanetMap.cells` is
    /// keyed by `HexCoord` — a struct, which JSON cannot use as an object key.
    /// Every other test builds a `GameState` with no planet maps at all, which is
    /// why saving looked healthy while no real game could be saved at all.
    #[test]
    fn a_state_with_a_planet_map_can_be_saved() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let mut state = GameState::new();
        state.add_colony(Colony::new("Mapped"), 300);
        let home = crate::system::BodyId::placeholder_home();
        state.home_body_id = Some(home.clone());
        state
            .planet_maps
            .insert(home, crate::map::PlanetMap::generate(42, 4, 4));

        snap.save(&state)
            .expect("a state with a planet map must save");

        let restored = snap.load().unwrap();
        let map = restored
            .home_map()
            .cloned()
            .expect("the map must survive the trip");
        assert!(!map.cells.is_empty(), "cells were dropped");
        let original = state.home_map().unwrap();
        assert_eq!(map.cells.len(), original.cells.len());
        for (coord, cell) in &original.cells {
            assert_eq!(
                map.cells.get(coord).map(|c| &c.terrain),
                Some(&cell.terrain),
                "cell at {coord:?} did not round-trip"
            );
        }
    }

    /// The broader lesson of #337: every save test built its `GameState` by
    /// hand, so no test ever serialized the state a *playing* game actually
    /// holds. Drive the real command path instead, so the next field with an
    /// unserializable key fails here rather than in someone's playtest.
    #[test]
    fn a_state_built_through_the_command_path_can_be_saved() {
        let mut engine = GameEngine::new();
        engine
            .apply(&Command::SeedPlanet {
                seed: 7,
                width: 5,
                height: 4,
            })
            .expect("seed planet");
        // Picking a difficulty is part of every real setup, and it seeds
        // `DifficultyScalar` with a `ProductionRate(..)` key — the second
        // unserializable-key bug in #337. Without this line the test passes
        // while a real game still cannot be saved.
        engine
            .apply(&Command::SetDifficulty {
                preset: DifficultyPreset::Hard,
            })
            .expect("set difficulty");
        engine.state.add_colony(Colony::new("Playtest"), 300);
        engine
            .apply(&Command::AdvanceColonySol)
            .expect("advance a sol");

        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&engine.state)
            .expect("a state produced by real commands must save");

        let restored = snap.load().unwrap();
        assert_eq!(restored.sol, engine.state.sol);
        assert!(
            restored.home_map().is_some_and(|m| !m.cells.is_empty()),
            "the seeded map must survive"
        );
        assert_eq!(
            restored
                .difficulty_scalar
                .scalar_for(&ModifiableQuantity::ProductionRate("*".into())),
            engine
                .state
                .difficulty_scalar
                .scalar_for(&ModifiableQuantity::ProductionRate("*".into())),
            "the difficulty scalar's newtype-variant key must round-trip"
        );
    }

    // ── schema version is stored and checked ──────────────────────────────

    // ── #180 Maintenance flag round-trips ─────────────────────────────────

    #[test]
    fn round_trip_preserves_maintenance_enabled() {
        let mut snap = Snapshot::open_in_memory().unwrap();
        let mut state = make_state_with_colonies();
        state.maintenance_enabled = false;
        snap.save(&state).unwrap();
        let restored = snap.load().unwrap();
        assert!(
            !restored.maintenance_enabled,
            "maintenance_enabled=false must persist across snapshot round-trip"
        );
    }

    #[test]
    fn snapshot_without_maintenance_field_defaults_to_true() {
        // Pre-#180 snapshots don't carry `maintenance_enabled`. The serde
        // default must restore the flag to `true` so authored upkeep still
        // applies for old save games.
        //
        // Emulate a legacy blob by serialising the current shape, stripping
        // the field, then deserialising back.
        let state = make_state_with_colonies();
        let blob = FullStateBlob::from_game_state(&state);
        let mut value: serde_json::Value = serde_json::to_value(&blob).unwrap();
        value.as_object_mut().unwrap().remove("maintenance_enabled");
        let restored: FullStateBlob = serde_json::from_value(value).unwrap();
        assert!(
            restored.maintenance_enabled,
            "missing maintenance_enabled must default to true for pre-#180 saves"
        );
    }

    #[test]
    fn schema_version_is_current() {
        let snap = Snapshot::open_in_memory().unwrap();
        let ver: u32 = snap
            .conn
            .query_row("SELECT version FROM schema_version LIMIT 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(ver, SCHEMA_VERSION);
    }
}
