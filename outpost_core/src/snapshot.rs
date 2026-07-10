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

use rusqlite::{params, Connection};

use crate::colony::Colony;
use crate::population::Population;
use crate::turn::GameState;

/// Monotonically increasing schema version.
///
/// Increment this whenever the on-disk layout changes.  Forward-migration
/// documentation must accompany each bump.
/// Schema version 2: `populations.count` changed from INTEGER to REAL to support
/// fractional population growth (Phase 2 — issue #15).
pub const SCHEMA_VERSION: u32 = 2;

// ─── DDL ──────────────────────────────────────────────────────────────────────────────

const SCHEMA_SQL: &str = "
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS schema_version (
    version  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS game_state (
    id       INTEGER PRIMARY KEY CHECK (id = 1),
    sol      INTEGER NOT NULL,
    month    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS colonies (
    idx         INTEGER NOT NULL,
    id          TEXT    NOT NULL,
    name        TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS populations (
    idx         INTEGER NOT NULL,
    count       REAL    NOT NULL,
    stability   REAL    NOT NULL
);
";

// ─── Errors ───────────────────────────────────────────────────────────────────────

/// Errors that can occur during snapshot save or load.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    /// `SQLite` operation failed.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
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
    pub fn save(&mut self, state: &GameState) -> Result<(), SnapshotError> {
        let tx = self.conn.transaction()?;

        // Wipe previous snapshot data.
        tx.execute_batch(
            "DELETE FROM game_state;
             DELETE FROM colonies;
             DELETE FROM populations;",
        )?;

        // Singleton turn counters.
        tx.execute(
            "INSERT INTO game_state (id, sol, month) VALUES (1, ?1, ?2)",
            params![state.sol, state.month],
        )?;

        // Colonies and populations in parallel index order.
        for (idx, colony) in state.colonies.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            tx.execute(
                "INSERT INTO colonies (idx, id, name) VALUES (?1, ?2, ?3)",
                params![idx as i64, colony.id.to_string(), &colony.name],
            )?;
        }
        for (idx, pop) in state.populations.iter().enumerate() {
            #[allow(clippy::cast_possible_wrap)]
            tx.execute(
                "INSERT INTO populations (idx, count, stability) VALUES (?1, ?2, ?3)",
                params![idx as i64, pop.count, pop.stability],
            )?;
        }

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

        // Turn counters.
        let (sol, month): (u64, u64) = self
            .conn
            .query_row(
                "SELECT sol, month FROM game_state WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| SnapshotError::MissingGameState)?;

        // Colonies (ordered by idx).
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM colonies ORDER BY idx")?;
        let colonies: Vec<Colony> = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let name: String = row.get(1)?;
                Ok((id_str, name))
            })?
            .map(|r| {
                let (id_str, name) = r?;
                let id = uuid::Uuid::parse_str(&id_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(Colony {
                    id,
                    name,
                    pool: crate::colony::ColonyPool::new(),
                    buildings: Vec::new(),
                    build_queue: crate::colony::ConstructionQueue::new(),
                    slot_capacity: crate::colony::BASE_SLOT_CAPACITY,
                    terrain_id: None,
                })
            })
            .collect::<Result<_, rusqlite::Error>>()?;

        // Populations (ordered by idx).
        let mut stmt = self
            .conn
            .prepare("SELECT count, stability FROM populations ORDER BY idx")?;
        let populations: Vec<Population> = stmt
            .query_map([], |row| {
                let count: f64 = row.get(0)?;
                let stability: f64 = row.get(1)?;
                #[allow(clippy::cast_possible_truncation)]
                Ok(Population::with_skills(
                    count as f32,
                    stability as f32,
                    crate::population::default_skill_distribution_pub(),
                ))
            })?
            .collect::<Result<_, _>>()?;

        Ok(GameState {
            colonies,
            populations,
            sol,
            month,
            registry: None,
            needs_config: None,
            research_pool: crate::research::SystemResearchPool::new(),
            tech_state: crate::tech::TechState::new(),
            tech_registry: None,
            directive_store: crate::directive::DirectiveStore::default(),
            stability_trackers: std::collections::HashMap::new(),
            trade_network: crate::trade::TradeNetwork::new(),
            emigration_gates: Vec::new(),
            pending_migrations: Vec::new(),
            population_trackers: std::collections::HashMap::new(),
            orbital_registry: crate::orbital::OrbitalRegistry::new(),
            difficulty_preset: crate::difficulty::DifficultyPreset::Normal,
            difficulty_grade_table: crate::difficulty::default_grade_table(),
            difficulty_scalar: crate::modifier::DifficultyScalar::new(),
            menace_state: None,
            victory_state: crate::victory::VictoryState::capstone_only(),
            cumulative_research: 0,
            expedition_launched: false,
            planet_map: None,
            victory: None,
            unlocked_buildings: std::collections::HashSet::new(),
            unlocked_capabilities: std::collections::HashSet::new(),
            unlocked_commodities: std::collections::HashSet::new(),
            modifier_accumulator: crate::modifier::ModifierAccumulator::new(),
            hazard_config: None,
            system_state: crate::system::SystemState::new(),
            infra_routes: std::collections::HashMap::new(),
        })
    }

    /// Apply the DDL and initialise the `schema_version` row if absent.
    fn apply_schema(&self) -> Result<(), SnapshotError> {
        self.conn.execute_batch(SCHEMA_SQL)?;
        // Insert schema version only if not already present.
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
    use crate::turn::GameState;
    use crate::{Command, GameEngine};

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
        // Corrupt the stored version to a future value.
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

    // ── save → load → continue reproduces identical subsequent turns ─────────

    #[test]
    fn save_load_continue_identical_turns() {
        const SEED: u64 = 42;

        // Run engine A for 10 turns, snapshot, restore into engine B.
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

        // Save snapshot.
        let mut snap = Snapshot::open_in_memory().unwrap();
        snap.save(&engine_a.state).unwrap();

        // Restore into a fresh engine with the same seed.
        let restored_state = snap.load().unwrap();
        let mut engine_b = GameEngine::with_seed(SEED);
        engine_b.state = restored_state;

        // Both engines should be at the same turn counters.
        assert_eq!(engine_a.sol(), engine_b.sol());
        assert_eq!(engine_a.month(), engine_b.month());
        assert_eq!(engine_a.state.colonies.len(), engine_b.state.colonies.len());
        assert_eq!(engine_a.state.colonies[0].id, engine_b.state.colonies[0].id);
        assert_eq!(
            engine_a.state.populations[0].count,
            engine_b.state.populations[0].count
        );

        // Advance both engines one more turn — outcomes must match.
        let events_a = engine_a.apply(&Command::AdvanceColonySol).unwrap();
        let events_b = engine_b.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(engine_a.sol(), engine_b.sol());
        // Both emit the same event type.
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
}
