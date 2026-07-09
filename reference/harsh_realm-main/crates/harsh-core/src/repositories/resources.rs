//! Repository for durable per-entity resource instances.
//!
//! Ported from `src/harsh_realm/resources/repository.py`.

use serde::{Deserialize, Serialize};

use crate::db::{Row, WorldDatabase};

/// Canonical qualified resource id for gold (ported from `resources/ids.py`).
pub const GOLD_RESOURCE_ID: &str = "xwn-core:resources.gold";
/// Canonical qualified resource id for HP (ported from `resources/ids.py`).
pub const HP_RESOURCE_ID: &str = "xwn-core:resources.hp";

/// A persisted resource instance owned by one entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceInstance {
    /// Owning entity id.
    pub entity_id: String,
    /// Resource id.
    pub resource_id: String,
    /// Current amount.
    pub current: i32,
    /// Maximum amount, if bounded.
    pub max: Option<i32>,
    /// Tick of last regeneration.
    #[serde(default)]
    pub last_regen_tick: i32,
}

/// Owns SQL access for the `entity_resources` table.
pub struct ResourceRepository<'a> {
    db: &'a WorldDatabase,
}

fn row_to_instance(row: &Row) -> ResourceInstance {
    ResourceInstance {
        entity_id: row.get("entity_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        resource_id: row.get("resource_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        current: row.get("current").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        max: row.get("max").and_then(|v| v.as_i64()).map(|x| x as i32),
        last_regen_tick: row.get("last_regen_tick").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
    }
}

impl<'a> ResourceRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        ResourceRepository { db }
    }

    /// Return one resource instance, or `None` when absent.
    pub fn get(&self, entity_id: &str, resource_id: &str) -> Result<Option<ResourceInstance>, String> {
        let row = self.db.fetch_one(
            "SELECT * FROM entity_resources WHERE entity_id = ? AND resource_id = ?",
            &[&entity_id, &resource_id],
        )?;
        Ok(row.as_ref().map(row_to_instance))
    }

    /// Insert or update one resource instance.
    pub fn set(&self, instance: &ResourceInstance) -> Result<(), String> {
        let max = instance.max.map(|m| m as i64);
        self.db.execute(
            "INSERT INTO entity_resources (entity_id, resource_id, current, max, last_regen_tick) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(entity_id, resource_id) DO UPDATE SET current = excluded.current, \
             max = excluded.max, last_regen_tick = excluded.last_regen_tick",
            &[
                &instance.entity_id,
                &instance.resource_id,
                &(instance.current as i64),
                &max,
                &(instance.last_regen_tick as i64),
            ],
        )?;
        Ok(())
    }

    /// Return all resource instances for an entity.
    pub fn list_for_entity(&self, entity_id: &str) -> Result<Vec<ResourceInstance>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM entity_resources WHERE entity_id = ? ORDER BY resource_id",
            &[&entity_id],
        )?;
        Ok(rows.iter().map(row_to_instance).collect())
    }

    /// Return all resource instances.
    pub fn list_all(&self) -> Result<Vec<ResourceInstance>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM entity_resources ORDER BY entity_id, resource_id",
            &[],
        )?;
        Ok(rows.iter().map(row_to_instance).collect())
    }

    /// Delete one resource instance; returns whether a row was removed.
    pub fn delete(&self, entity_id: &str, resource_id: &str) -> Result<bool, String> {
        let affected = self.db.execute(
            "DELETE FROM entity_resources WHERE entity_id = ? AND resource_id = ?",
            &[&entity_id, &resource_id],
        )?;
        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_crud() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = ResourceRepository::new(&db);
        let inst = ResourceInstance {
            entity_id: "hero".into(),
            resource_id: "hp".into(),
            current: 8,
            max: Some(10),
            last_regen_tick: 0,
        };
        repo.set(&inst).unwrap();
        assert_eq!(repo.get("hero", "hp").unwrap().unwrap().current, 8);
        let updated = ResourceInstance { current: 10, ..inst.clone() };
        repo.set(&updated).unwrap();
        assert_eq!(repo.get("hero", "hp").unwrap().unwrap().current, 10);
        assert_eq!(repo.list_for_entity("hero").unwrap().len(), 1);
        assert_eq!(repo.list_all().unwrap().len(), 1);
        assert!(repo.delete("hero", "hp").unwrap());
        assert!(repo.get("hero", "hp").unwrap().is_none());
    }
}
