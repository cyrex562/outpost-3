//! Repository for durable active status effect rows.
//!
//! Ported from `src/harsh_realm/status_effects/repository.py`.

use crate::db::{Row, WorldDatabase};
use crate::runtime::JsonObject;

use super::models::ActiveStatusEffect;

/// Owns SQL access for the `entity_status_effects` table.
pub struct StatusEffectRepository<'a> {
    db: &'a WorldDatabase,
}

impl<'a> StatusEffectRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        StatusEffectRepository { db }
    }

    /// Insert an active status effect row.
    pub fn add(
        &self,
        entity_id: &str,
        effect_id: &str,
        applied_at_tick: i32,
        expires_at_tick: Option<i32>,
        source: Option<&str>,
        data: JsonObject,
    ) -> Result<ActiveStatusEffect, String> {
        let data_json = serde_json::to_string(&data).map_err(|e| e.to_string())?;
        let expires = expires_at_tick.map(|v| v as i64);
        self.db.execute(
            "INSERT INTO entity_status_effects (\
                entity_id, effect_id, applied_at_tick, expires_at_tick, source, data_json\
             ) VALUES (?, ?, ?, ?, ?, ?)",
            &[
                &entity_id,
                &effect_id,
                &(applied_at_tick as i64),
                &expires,
                &source,
                &data_json,
            ],
        )?;
        Ok(ActiveStatusEffect {
            id: self.db.last_insert_rowid(),
            entity_id: entity_id.to_string(),
            effect_id: effect_id.to_string(),
            applied_at_tick,
            expires_at_tick,
            source: source.map(str::to_string),
            data,
        })
    }

    /// Update an active status effect row.
    pub fn update(&self, effect: &ActiveStatusEffect) -> Result<(), String> {
        let data_json = serde_json::to_string(&effect.data).map_err(|e| e.to_string())?;
        let expires = effect.expires_at_tick.map(|v| v as i64);
        self.db.execute(
            "UPDATE entity_status_effects \
             SET expires_at_tick = ?, source = ?, data_json = ? WHERE id = ?",
            &[&expires, &effect.source, &data_json, &effect.id],
        )?;
        Ok(())
    }

    /// List active effects for an entity.
    pub fn list_for_entity(&self, entity_id: &str) -> Result<Vec<ActiveStatusEffect>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM entity_status_effects WHERE entity_id = ? ORDER BY id",
            &[&entity_id],
        )?;
        rows.iter().map(row_to_active_effect).collect()
    }

    /// List active instances of one effect for an entity.
    pub fn list_for_entity_effect(
        &self,
        entity_id: &str,
        effect_id: &str,
    ) -> Result<Vec<ActiveStatusEffect>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM entity_status_effects \
             WHERE entity_id = ? AND effect_id = ? ORDER BY id",
            &[&entity_id, &effect_id],
        )?;
        rows.iter().map(row_to_active_effect).collect()
    }

    /// Remove all active instances of one effect from an entity.
    pub fn remove(
        &self,
        entity_id: &str,
        effect_id: &str,
    ) -> Result<Vec<ActiveStatusEffect>, String> {
        let removed = self.list_for_entity_effect(entity_id, effect_id)?;
        self.db.execute(
            "DELETE FROM entity_status_effects WHERE entity_id = ? AND effect_id = ?",
            &[&entity_id, &effect_id],
        )?;
        Ok(removed)
    }

    /// Remove one active effect by row id.
    pub fn remove_by_id(
        &self,
        active_effect_id: i64,
    ) -> Result<Option<ActiveStatusEffect>, String> {
        let row = self.db.fetch_one(
            "SELECT * FROM entity_status_effects WHERE id = ?",
            &[&active_effect_id],
        )?;
        let Some(row) = row else {
            return Ok(None);
        };
        let removed = row_to_active_effect(&row)?;
        self.db.execute(
            "DELETE FROM entity_status_effects WHERE id = ?",
            &[&active_effect_id],
        )?;
        Ok(Some(removed))
    }

    /// Distinct entity ids that currently carry at least one active effect.
    ///
    /// The world-clock heartbeat uses this to fan `time.tick` evaluation out over
    /// exactly the entities with over-time effects to resolve.
    pub fn distinct_entity_ids(&self) -> Result<Vec<String>, String> {
        let rows = self.db.fetch_all(
            "SELECT DISTINCT entity_id FROM entity_status_effects ORDER BY entity_id",
            &[],
        )?;
        Ok(rows
            .iter()
            .filter_map(|r| r.get("entity_id").and_then(|v| v.as_str()).map(str::to_string))
            .collect())
    }

    /// Remove and return effects whose expiry is due at `current_tick`.
    pub fn expire_due(&self, current_tick: i32) -> Result<Vec<ActiveStatusEffect>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM entity_status_effects \
             WHERE expires_at_tick IS NOT NULL AND expires_at_tick <= ? ORDER BY id",
            &[&(current_tick as i64)],
        )?;
        let expired: Vec<ActiveStatusEffect> =
            rows.iter().map(row_to_active_effect).collect::<Result<_, _>>()?;
        self.db.execute(
            "DELETE FROM entity_status_effects \
             WHERE expires_at_tick IS NOT NULL AND expires_at_tick <= ?",
            &[&(current_tick as i64)],
        )?;
        Ok(expired)
    }
}

fn row_to_active_effect(row: &Row) -> Result<ActiveStatusEffect, String> {
    let data_str = row
        .get("data_json")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("{}");
    let data: JsonObject = serde_json::from_str(data_str).map_err(|e| e.to_string())?;
    Ok(ActiveStatusEffect {
        id: row.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
        entity_id: row
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        effect_id: row
            .get("effect_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        applied_at_tick: row.get("applied_at_tick").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        expires_at_tick: row
            .get("expires_at_tick")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        source: row
            .get("source")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_update_remove_expire() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = StatusEffectRepository::new(&db);

        let mut data = JsonObject::new();
        data.insert("potency".into(), serde_json::json!(3));
        let active = repo
            .add("hero", "poison", 5, Some(10), Some("trap"), data)
            .unwrap();
        assert!(active.id > 0);
        assert_eq!(active.expires_at_tick, Some(10));

        let listed = repo.list_for_entity("hero").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].data.get("potency").unwrap().as_i64(), Some(3));

        let mut updated = active.clone();
        updated.expires_at_tick = Some(20);
        repo.update(&updated).unwrap();
        assert_eq!(
            repo.list_for_entity("hero").unwrap()[0].expires_at_tick,
            Some(20)
        );

        // Not yet due.
        assert!(repo.expire_due(15).unwrap().is_empty());
        // Now due.
        let expired = repo.expire_due(25).unwrap();
        assert_eq!(expired.len(), 1);
        assert!(repo.list_for_entity("hero").unwrap().is_empty());
    }

    #[test]
    fn distinct_entity_ids_dedupes() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = StatusEffectRepository::new(&db);
        repo.add("hero", "poison", 0, None, None, JsonObject::new()).unwrap();
        repo.add("hero", "bless", 0, None, None, JsonObject::new()).unwrap();
        repo.add("goblin", "poison", 0, None, None, JsonObject::new()).unwrap();
        let ids = repo.distinct_entity_ids().unwrap();
        assert_eq!(ids, vec!["goblin".to_string(), "hero".to_string()]);
    }

    #[test]
    fn remove_and_remove_by_id() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = StatusEffectRepository::new(&db);
        let a = repo
            .add("hero", "bless", 0, None, None, JsonObject::new())
            .unwrap();
        repo.add("hero", "bless", 1, None, None, JsonObject::new())
            .unwrap();
        let removed = repo.remove("hero", "bless").unwrap();
        assert_eq!(removed.len(), 2);
        assert!(repo.list_for_entity("hero").unwrap().is_empty());

        repo.add("hero", "haste", 0, None, None, JsonObject::new())
            .unwrap();
        assert!(repo.remove_by_id(a.id).unwrap().is_none());
        let haste = repo.list_for_entity("hero").unwrap();
        assert_eq!(haste.len(), 1);
        assert!(repo.remove_by_id(haste[0].id).unwrap().is_some());
    }
}
