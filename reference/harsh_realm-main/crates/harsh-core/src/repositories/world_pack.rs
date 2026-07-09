//! Persistence for packs bound to a world and per-world overrides.
//!
//! Ported from `src/harsh_realm/packs/world_repository.py`.

use serde::{Deserialize, Serialize};

use crate::db::WorldDatabase;
use crate::runtime::JsonObject;

/// Pack version and load order recorded for a world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackBinding {
    /// Pack id.
    pub pack_id: String,
    /// Version string.
    pub version: String,
    /// Load order (>= 0).
    pub load_order: i32,
}

/// Per-world content override row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverrideRecord {
    /// Pack id.
    pub pack_id: String,
    /// Qualified content id.
    pub qualified_id: String,
    /// Override payload.
    pub data: JsonObject,
    /// Update timestamp.
    pub updated_at: String,
}

/// Persistence for `world_packs` and `pack_overrides`.
pub struct WorldPackRepository<'a> {
    db: &'a WorldDatabase,
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl<'a> WorldPackRepository<'a> {
    /// Create a pack repository for a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        WorldPackRepository { db }
    }

    /// Replace the world's recorded pack list.
    pub fn set_pack_list(&self, packs: &[PackBinding]) -> Result<(), String> {
        self.db.execute("DELETE FROM world_packs", &[])?;
        let mut sorted: Vec<&PackBinding> = packs.iter().collect();
        sorted.sort_by_key(|p| p.load_order);
        for pack in sorted {
            let order = pack.load_order as i64;
            self.db.execute(
                "INSERT INTO world_packs (pack_id, version, load_order) VALUES (?, ?, ?)",
                &[&pack.pack_id, &pack.version, &order],
            )?;
        }
        Ok(())
    }

    /// Return the world's pack bindings ordered by load order.
    pub fn get_pack_list(&self) -> Result<Vec<PackBinding>, String> {
        let rows = self.db.fetch_all(
            "SELECT pack_id, version, load_order FROM world_packs ORDER BY load_order",
            &[],
        )?;
        Ok(rows
            .iter()
            .map(|r| PackBinding {
                pack_id: r.get("pack_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                version: r.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                load_order: r.get("load_order").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            })
            .collect())
    }

    /// Return an override payload, or `None` when absent.
    pub fn get_override(&self, pack_id: &str, qualified_id: &str) -> Result<Option<JsonObject>, String> {
        let row = self.db.fetch_one(
            "SELECT data_json FROM pack_overrides WHERE pack_id = ? AND qualified_id = ?",
            &[&pack_id, &qualified_id],
        )?;
        Ok(row.and_then(|r| {
            r.get("data_json")
                .and_then(|v| v.as_str())
                .and_then(|s| serde_json::from_str(s).ok())
        }))
    }

    /// Upsert a content override.
    pub fn set_override(&self, pack_id: &str, qualified_id: &str, data: &JsonObject) -> Result<(), String> {
        let json = serde_json::to_string(data).map_err(|e| e.to_string())?;
        let now = now_iso();
        self.db.execute(
            "INSERT OR REPLACE INTO pack_overrides (pack_id, qualified_id, data_json, updated_at) VALUES (?, ?, ?, ?)",
            &[&pack_id, &qualified_id, &json, &now],
        )?;
        Ok(())
    }

    /// Delete an override; returns whether a row was removed.
    pub fn delete_override(&self, pack_id: &str, qualified_id: &str) -> Result<bool, String> {
        let affected = self.db.execute(
            "DELETE FROM pack_overrides WHERE pack_id = ? AND qualified_id = ?",
            &[&pack_id, &qualified_id],
        )?;
        Ok(affected > 0)
    }

    /// Return all override rows ordered by pack and qualified id.
    pub fn list_overrides(&self) -> Result<Vec<OverrideRecord>, String> {
        let rows = self.db.fetch_all(
            "SELECT pack_id, qualified_id, data_json, updated_at FROM pack_overrides ORDER BY pack_id, qualified_id",
            &[],
        )?;
        Ok(rows
            .iter()
            .map(|r| OverrideRecord {
                pack_id: r.get("pack_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                qualified_id: r.get("qualified_id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                data: r.get("data_json").and_then(|v| v.as_str()).and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default(),
                updated_at: r.get("updated_at").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::JsonValue;

    #[test]
    fn pack_list_round_trip_ordered() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = WorldPackRepository::new(&db);
        repo.set_pack_list(&[
            PackBinding { pack_id: "b".into(), version: "1.0.0".into(), load_order: 1 },
            PackBinding { pack_id: "a".into(), version: "1.0.0".into(), load_order: 0 },
        ])
        .unwrap();
        let list = repo.get_pack_list().unwrap();
        assert_eq!(list[0].pack_id, "a"); // ordered by load_order
        assert_eq!(list[1].pack_id, "b");
    }

    #[test]
    fn overrides_crud() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = WorldPackRepository::new(&db);
        let mut data = JsonObject::new();
        data.insert("hp".into(), JsonValue::from(9));
        repo.set_override("xwn", "creature.wolf", &data).unwrap();
        assert_eq!(repo.get_override("xwn", "creature.wolf").unwrap().unwrap()["hp"], JsonValue::from(9));
        assert_eq!(repo.list_overrides().unwrap().len(), 1);
        assert!(repo.delete_override("xwn", "creature.wolf").unwrap());
        assert!(repo.get_override("xwn", "creature.wolf").unwrap().is_none());
    }
}
