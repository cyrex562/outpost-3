//! Transient modifier storage and retrieval.
//!
//! Ported from `src/harsh_realm/modifiers/transient.py`.
//!
//! The async Python `TransientModifierRepository` is here as a sync version
//! backed by `WorldDatabase`. The modifier JSON serializes the `ir::Modifier`
//! struct exactly.

use crate::db::WorldDatabase;
use crate::ir::Modifier;

/// A row from the `entity_transient_modifiers` table.
#[derive(Debug, Clone)]
pub struct TransientModifierRecord {
    pub id: i64,
    pub entity_id: String,
    pub modifier: Modifier,
    pub applied_at_tick: i64,
    pub expires_at_tick: Option<i64>,
    pub source: Option<String>,
}

/// CRUD for the `entity_transient_modifiers` table.
pub struct TransientModifierRepository<'db> {
    db: &'db WorldDatabase,
}

impl<'db> TransientModifierRepository<'db> {
    pub fn new(db: &'db WorldDatabase) -> Self {
        Self { db }
    }

    /// Add a transient modifier. Returns the new row id.
    pub fn add(
        &self,
        entity_id: &str,
        modifier: &Modifier,
        applied_at: i64,
        duration: Option<i64>,
        source: Option<&str>,
    ) -> Result<i64, String> {
        let expires_at: Option<i64> = duration.map(|d| applied_at + d);
        let modifier_json = serde_json::to_string(modifier).map_err(|e| e.to_string())?;
        let rows = self.db.fetch_all(
            "INSERT INTO entity_transient_modifiers \
             (entity_id, modifier_json, applied_at_tick, expires_at_tick, source) \
             VALUES (?, ?, ?, ?, ?) RETURNING id",
            &[
                &entity_id.to_string() as &dyn rusqlite::types::ToSql,
                &modifier_json,
                &applied_at,
                &expires_at,
                &source.map(|s| s.to_string()),
            ],
        )?;
        let id = rows
            .first()
            .and_then(|r| r.get("id"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        Ok(id)
    }

    /// Remove a transient modifier by id.
    pub fn remove(&self, modifier_id: i64) -> Result<(), String> {
        self.db.execute(
            "DELETE FROM entity_transient_modifiers WHERE id = ?",
            &[&modifier_id as &dyn rusqlite::types::ToSql],
        )?;
        Ok(())
    }

    /// Remove all of an entity's transient modifiers that came from `source`.
    /// Returns the number of rows removed.
    pub fn remove_by_source(&self, entity_id: &str, source: &str) -> Result<usize, String> {
        let rows = self.db.fetch_all(
            "DELETE FROM entity_transient_modifiers WHERE entity_id = ? AND source = ? \
             RETURNING id",
            &[
                &entity_id.to_string() as &dyn rusqlite::types::ToSql,
                &source.to_string(),
            ],
        )?;
        Ok(rows.len())
    }

    /// List active transient modifiers for an entity at the given tick.
    pub fn list_for_entity(
        &self,
        entity_id: &str,
        current_tick: i64,
    ) -> Result<Vec<TransientModifierRecord>, String> {
        let rows = self.db.fetch_all(
            "SELECT id, entity_id, modifier_json, applied_at_tick, expires_at_tick, source \
             FROM entity_transient_modifiers \
             WHERE entity_id = ? \
             AND (expires_at_tick IS NULL OR expires_at_tick > ?)",
            &[
                &entity_id.to_string() as &dyn rusqlite::types::ToSql,
                &current_tick,
            ],
        )?;
        rows.iter()
            .map(|row| {
                let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
                let eid = row.get("entity_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let json_str = row.get("modifier_json").and_then(|v| v.as_str()).unwrap_or("{}");
                let modifier: Modifier =
                    serde_json::from_str(json_str).map_err(|e| e.to_string())?;
                let applied_at = row.get("applied_at_tick").and_then(|v| v.as_i64()).unwrap_or(0);
                let expires_at = row.get("expires_at_tick").and_then(|v| v.as_i64());
                let source = row.get("source").and_then(|v| v.as_str()).map(|s| s.to_string());
                Ok(TransientModifierRecord {
                    id,
                    entity_id: eid,
                    modifier,
                    applied_at_tick: applied_at,
                    expires_at_tick: expires_at,
                    source,
                })
            })
            .collect()
    }

    /// Delete all expired transient modifiers.
    pub fn cleanup_expired(&self, current_tick: i64) -> Result<(), String> {
        self.db.execute(
            "DELETE FROM entity_transient_modifiers WHERE expires_at_tick <= ?",
            &[&current_tick as &dyn rusqlite::types::ToSql],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use crate::ir::{Modifier, ModifierCondition, ModifierStacking};

    fn test_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn simple_modifier(target: &str, value: i64) -> Modifier {
        Modifier {
            target: target.to_string(),
            value,
            stacking: ModifierStacking::Additive,
            priority: 0,
            condition: ModifierCondition::default(),
            description: String::new(),
        }
    }

    #[test]
    fn add_and_list() {
        let db = test_db();
        let repo = TransientModifierRepository::new(&db);
        let m = simple_modifier("skill.stab", 2);
        let id = repo.add("ent-1", &m, 0, None, None).unwrap();
        assert!(id > 0);
        let records = repo.list_for_entity("ent-1", 0).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].modifier.value, 2);
    }

    #[test]
    fn remove_modifier() {
        let db = test_db();
        let repo = TransientModifierRepository::new(&db);
        let m = simple_modifier("skill.stab", 1);
        let id = repo.add("ent-1", &m, 0, None, None).unwrap();
        repo.remove(id).unwrap();
        let records = repo.list_for_entity("ent-1", 0).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn expired_modifiers_excluded() {
        let db = test_db();
        let repo = TransientModifierRepository::new(&db);
        let m = simple_modifier("combat.ac", 3);
        repo.add("ent-1", &m, 0, Some(5), None).unwrap(); // expires at tick 5
        // At tick 4 it should still appear.
        assert_eq!(repo.list_for_entity("ent-1", 4).unwrap().len(), 1);
        // At tick 5 it should be excluded (expires_at_tick <= current_tick).
        assert_eq!(repo.list_for_entity("ent-1", 5).unwrap().len(), 0);
    }

    #[test]
    fn cleanup_expired() {
        let db = test_db();
        let repo = TransientModifierRepository::new(&db);
        let m = simple_modifier("skill.heal", 1);
        repo.add("ent-1", &m, 0, Some(3), None).unwrap();
        repo.add("ent-1", &m, 0, None, None).unwrap(); // permanent
        repo.cleanup_expired(10).unwrap();
        // Only the permanent one remains.
        let records = repo.list_for_entity("ent-1", 0).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].expires_at_tick.is_none());
    }

    #[test]
    fn source_and_duration_stored() {
        let db = test_db();
        let repo = TransientModifierRepository::new(&db);
        let m = simple_modifier("save.physical", -1);
        repo.add("ent-1", &m, 10, Some(20), Some("poison")).unwrap();
        let records = repo.list_for_entity("ent-1", 10).unwrap();
        assert_eq!(records[0].source.as_deref(), Some("poison"));
        assert_eq!(records[0].expires_at_tick, Some(30));
    }
}
