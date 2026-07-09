//! Repository helpers for random table storage.
//!
//! Ported from `src/harsh_realm/engine/random_table_repository.py`.

use crate::db::{Row, WorldDatabase};
use crate::scene_data::{RandomTableEntry, RandomTableRow};

/// Centralizes `random_tables` access for the table engine.
pub struct RandomTableRepository<'a> {
    db: &'a WorldDatabase,
}

fn row_str(row: &Row, key: &str) -> String {
    row.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn decode_row(row: &Row) -> RandomTableRow {
    let tags: Vec<String> =
        serde_json::from_str(row.get("tags").and_then(|v| v.as_str()).unwrap_or("[]"))
            .unwrap_or_default();
    let entries: Vec<RandomTableEntry> =
        serde_json::from_str(row.get("entries").and_then(|v| v.as_str()).unwrap_or("[]"))
            .unwrap_or_default();
    RandomTableRow {
        id: row_str(row, "id"),
        category: row_str(row, "category"),
        name: row_str(row, "name"),
        tags,
        entries,
        source: row_str(row, "source"),
    }
}

impl<'a> RandomTableRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        RandomTableRepository { db }
    }

    /// Insert or replace a table definition.
    pub fn upsert_table(
        &self,
        table_id: &str,
        category: &str,
        name: &str,
        tags: &[String],
        entries: &[RandomTableEntry],
        source: &str,
    ) -> Result<(), String> {
        let tags_json = serde_json::to_string(tags).map_err(|e| e.to_string())?;
        let entries_json = serde_json::to_string(entries).map_err(|e| e.to_string())?;
        self.db.execute(
            "INSERT OR REPLACE INTO random_tables (id, category, name, tags, entries, source) VALUES (?, ?, ?, ?, ?, ?)",
            &[&table_id, &category, &name, &tags_json, &entries_json, &source],
        )?;
        Ok(())
    }

    /// Return one decoded table row.
    pub fn load_table(&self, table_id: &str) -> Result<Option<RandomTableRow>, String> {
        let row = self.db.fetch_one(
            "SELECT id, category, name, tags, entries, source FROM random_tables WHERE id = ?",
            &[&table_id],
        )?;
        Ok(row.as_ref().map(decode_row))
    }

    /// Return decoded table rows for a category.
    pub fn list_tables_by_category(&self, category: &str) -> Result<Vec<RandomTableRow>, String> {
        let rows = self.db.fetch_all(
            "SELECT id, category, name, tags, entries, source FROM random_tables WHERE category = ?",
            &[&category],
        )?;
        Ok(rows.iter().map(decode_row).collect())
    }

    /// Return the total number of stored random tables.
    pub fn count_tables(&self) -> Result<i64, String> {
        let row = self
            .db
            .fetch_one("SELECT COUNT(*) AS count FROM random_tables", &[])?;
        Ok(row
            .and_then(|r| r.get("count").and_then(|v| v.as_i64()))
            .unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::JsonValue;

    #[test]
    fn upsert_load_and_count() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = RandomTableRepository::new(&db);
        assert_eq!(repo.count_tables().unwrap(), 0);

        let entries = vec![RandomTableEntry {
            weight: 1.0,
            range: Some("1-6".into()),
            result: JsonValue::from("a sword"),
        }];
        repo.upsert_table("loot.basic", "loot", "Basic Loot", &["common".into()], &entries, "test")
            .unwrap();

        let loaded = repo.load_table("loot.basic").unwrap().unwrap();
        assert_eq!(loaded.name, "Basic Loot");
        assert_eq!(loaded.tags, vec!["common".to_string()]);
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].range.as_deref(), Some("1-6"));

        assert_eq!(repo.list_tables_by_category("loot").unwrap().len(), 1);
        assert_eq!(repo.count_tables().unwrap(), 1);
    }
}
