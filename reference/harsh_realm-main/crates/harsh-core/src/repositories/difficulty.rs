//! Repository for the `difficulty_settings` table.
//!
//! Mirrors `repositories::skill_config` in structure. Reads grade rows and
//! maps them through the const grade tables in `crate::difficulty`.

use std::collections::BTreeMap;

use crate::db::{Row, WorldDatabase};
use crate::difficulty::{
    all_dimension_meta, resolve, DifficultyDimension, DifficultyProfile, GRADE_COUNT, NORMAL_GRADE,
};

fn rs(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn ri(row: &Row, key: &str) -> i32 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
}

/// Read/write access to the `difficulty_settings` table.
pub struct DifficultySettingsRepository<'a> {
    db: &'a WorldDatabase,
}

impl<'a> DifficultySettingsRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        DifficultySettingsRepository { db }
    }

    /// Load and resolve the current difficulty settings into a [`DifficultyProfile`].
    ///
    /// Missing rows fall back to grade 3 (Normal). Returns `DifficultyProfile::default()`
    /// if the table cannot be read (e.g. schema not yet applied).
    pub fn load_profile(&self) -> Result<DifficultyProfile, String> {
        let rows = self
            .db
            .fetch_all("SELECT key, grade FROM difficulty_settings", &[])?;
        let mut grades: BTreeMap<String, i32> = BTreeMap::new();
        for row in &rows {
            grades.insert(rs(row, "key"), ri(row, "grade"));
        }
        Ok(resolve(&grades))
    }

    /// List all dimensions with their current grade and metadata (for the API).
    pub fn list_dimensions(&self) -> Result<Vec<DifficultyDimension>, String> {
        let rows = self
            .db
            .fetch_all("SELECT key, grade FROM difficulty_settings", &[])?;
        let mut grade_map: BTreeMap<String, i32> = BTreeMap::new();
        for row in &rows {
            grade_map.insert(rs(row, "key"), ri(row, "grade"));
        }
        let metas = all_dimension_meta();
        let dims = metas
            .iter()
            .map(|meta| {
                let grade = grade_map.get(meta.key).copied().unwrap_or(NORMAL_GRADE);
                DifficultyDimension::from_meta(meta, grade)
            })
            .collect();
        Ok(dims)
    }

    /// Write (upsert) a single grade for `key`.
    ///
    /// Returns `Err` if `key` is not a known dimension or `grade` is out of range.
    pub fn set_grade(&self, key: &str, grade: i32) -> Result<(), String> {
        // Validate key.
        let known: Vec<&str> = all_dimension_meta().into_iter().map(|m| m.key).collect();
        if !known.contains(&key) {
            return Err(format!("unknown difficulty dimension: '{key}'"));
        }
        // Validate grade range.
        if !(0..GRADE_COUNT).contains(&grade) {
            return Err(format!(
                "grade {grade} out of range: must be 0..{}",
                GRADE_COUNT - 1
            ));
        }
        self.db.execute(
            "INSERT INTO difficulty_settings (key, grade) VALUES (?, ?) \
             ON CONFLICT(key) DO UPDATE SET grade = excluded.grade",
            &[&key, &(grade as i64)],
        )?;
        Ok(())
    }

    /// Return the `DifficultyDimension` for a single key with the current grade.
    ///
    /// Returns `Err` if the key is not known.
    pub fn get_dimension(&self, key: &str) -> Result<DifficultyDimension, String> {
        let metas = all_dimension_meta();
        let meta = metas
            .iter()
            .find(|m| m.key == key)
            .ok_or_else(|| format!("unknown difficulty dimension: '{key}'"))?;
        let row = self.db.fetch_one(
            "SELECT grade FROM difficulty_settings WHERE key = ?",
            &[&key],
        )?;
        let grade = row
            .as_ref()
            .and_then(|r| r.get("grade"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .unwrap_or(NORMAL_GRADE);
        Ok(DifficultyDimension::from_meta(meta, grade))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;

    fn make_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().expect("in-memory db");
        db.execute(
            "CREATE TABLE IF NOT EXISTS difficulty_settings (key TEXT PRIMARY KEY, grade INTEGER NOT NULL)",
            &[],
        )
        .expect("create table");
        db
    }

    fn seed_defaults(db: &WorldDatabase) {
        for key in crate::difficulty::dimension_keys() {
            db.execute(
                "INSERT OR IGNORE INTO difficulty_settings (key, grade) VALUES (?, ?)",
                &[&key, &(NORMAL_GRADE as i64)],
            )
            .expect("seed");
        }
    }

    #[test]
    fn load_profile_empty_table_returns_identity() {
        let db = make_db();
        let repo = DifficultySettingsRepository::new(&db);
        let profile = repo.load_profile().unwrap();
        assert_eq!(profile, DifficultyProfile::default());
    }

    #[test]
    fn load_profile_with_seeded_defaults_returns_identity() {
        let db = make_db();
        seed_defaults(&db);
        let repo = DifficultySettingsRepository::new(&db);
        let profile = repo.load_profile().unwrap();
        assert_eq!(profile, DifficultyProfile::default());
    }

    #[test]
    fn set_grade_updates_profile() {
        let db = make_db();
        seed_defaults(&db);
        let repo = DifficultySettingsRepository::new(&db);
        repo.set_grade("enemy_hp", 6).unwrap();
        let profile = repo.load_profile().unwrap();
        assert_eq!(profile.enemy_hp_mult, 2.0);
        // Others unchanged.
        assert_eq!(profile.xp_mult, 1.0);
    }

    #[test]
    fn set_grade_rejects_unknown_key() {
        let db = make_db();
        let repo = DifficultySettingsRepository::new(&db);
        let err = repo.set_grade("does_not_exist", 3).unwrap_err();
        assert!(err.contains("unknown difficulty dimension"));
    }

    #[test]
    fn set_grade_rejects_out_of_range() {
        let db = make_db();
        let repo = DifficultySettingsRepository::new(&db);
        let err = repo.set_grade("enemy_hp", 7).unwrap_err();
        assert!(err.contains("out of range"));
        let err2 = repo.set_grade("enemy_hp", -1).unwrap_err();
        assert!(err2.contains("out of range"));
    }

    #[test]
    fn list_dimensions_returns_seven_items() {
        let db = make_db();
        seed_defaults(&db);
        let repo = DifficultySettingsRepository::new(&db);
        let dims = repo.list_dimensions().unwrap();
        assert_eq!(dims.len(), 7);
        // All at grade 3 (Normal) after seeding.
        for d in &dims {
            assert_eq!(d.grade, NORMAL_GRADE);
            assert_eq!(d.normal_grade, NORMAL_GRADE);
            assert_eq!(d.grade_count, 7);
        }
    }

    #[test]
    fn get_dimension_returns_updated_grade() {
        let db = make_db();
        seed_defaults(&db);
        let repo = DifficultySettingsRepository::new(&db);
        repo.set_grade("xp", 0).unwrap();
        let dim = repo.get_dimension("xp").unwrap();
        assert_eq!(dim.grade, 0);
        assert!(!dim.current_effect.is_empty());
        let effect_at_0 = crate::difficulty::current_effect("xp", 0);
        assert_eq!(dim.current_effect, effect_at_0);
    }
}
