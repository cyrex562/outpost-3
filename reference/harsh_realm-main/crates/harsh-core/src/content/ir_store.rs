//! Per-world store for compiled IR records.
//!
//! Ported from `src/harsh_realm/content/ir_store.py`.

use std::collections::HashMap;

use serde_json::Value;

use crate::db::WorldDatabase;
use crate::payloads::JsonObject;

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    // ISO 8601 UTC approximation (seconds precision).
    let (y, mo, d, h, mi, s) = epoch_to_parts(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}+00:00")
}

fn epoch_to_parts(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let mins = secs / 60;
    let mi = mins % 60;
    let hours = mins / 60;
    let h = hours % 24;
    let days = hours / 24;
    // Approximate Gregorian calendar.
    let y400 = days / 146097;
    let rem = days % 146097;
    let y100 = rem / 36524;
    let rem = rem % 36524;
    let y4 = rem / 1461;
    let rem = rem % 1461;
    let y1 = rem / 365;
    let rem = rem % 365;
    let year = 1970 + y400 * 400 + y100 * 100 + y4 * 4 + y1;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let month_days: &[u64] = if leap {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut rem = rem;
    let mut mo = 1u64;
    for &md in month_days {
        if rem < md {
            break;
        }
        rem -= md;
        mo += 1;
    }
    (year, mo, rem + 1, h, mi, s)
}

/// CRUD for the per-world `ir_records` table.
pub struct IRRecordRepository<'db> {
    db: &'db WorldDatabase,
}

impl<'db> IRRecordRepository<'db> {
    pub fn new(db: &'db WorldDatabase) -> Self {
        Self { db }
    }

    /// Insert or replace compiled records. Returns the number written.
    pub fn upsert_many(&self, records: &[JsonObject], source: &str) -> Result<usize, String> {
        let now = now_iso();
        let mut written = 0;
        for record in records {
            let component_type = match record.get("component_type").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let record_id = match record.get("id").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let data_json = serde_json::to_string(record).map_err(|e| e.to_string())?;
            self.db.execute(
                "INSERT INTO ir_records (component_type, record_id, data_json, source, updated_at) \
                 VALUES (?, ?, ?, ?, ?) \
                 ON CONFLICT(component_type, record_id) DO UPDATE SET \
                   data_json = excluded.data_json, \
                   source = excluded.source, \
                   updated_at = excluded.updated_at",
                &[&component_type as &dyn rusqlite::types::ToSql,
                  &record_id,
                  &data_json,
                  &source.to_string(),
                  &now],
            )?;
            written += 1;
        }
        Ok(written)
    }

    /// Retrieve one record by (component_type, record_id).
    pub fn get(&self, component_type: &str, record_id: &str) -> Result<Option<JsonObject>, String> {
        let rows = self.db.fetch_all(
            "SELECT data_json FROM ir_records WHERE component_type = ? AND record_id = ?",
            &[&component_type.to_string() as &dyn rusqlite::types::ToSql,
              &record_id.to_string()],
        )?;
        match rows.first() {
            None => Ok(None),
            Some(row) => {
                let json_str = row.get("data_json")
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let parsed: JsonObject = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
                Ok(Some(parsed))
            }
        }
    }

    /// List all records, optionally filtered by component_type.
    pub fn list_records(&self, component_type: Option<&str>) -> Result<Vec<JsonObject>, String> {
        let rows = if let Some(ct) = component_type {
            self.db.fetch_all(
                "SELECT data_json FROM ir_records WHERE component_type = ? ORDER BY record_id",
                &[&ct.to_string() as &dyn rusqlite::types::ToSql],
            )?
        } else {
            self.db.fetch_all(
                "SELECT data_json FROM ir_records ORDER BY component_type, record_id",
                &[],
            )?
        };
        rows.iter()
            .map(|row| {
                let s = row.get("data_json").and_then(|v| v.as_str()).unwrap_or("{}");
                serde_json::from_str(s).map_err(|e| e.to_string())
            })
            .collect()
    }

    /// Delete one record.
    pub fn delete(&self, component_type: &str, record_id: &str) -> Result<(), String> {
        self.db.execute(
            "DELETE FROM ir_records WHERE component_type = ? AND record_id = ?",
            &[&component_type.to_string() as &dyn rusqlite::types::ToSql,
              &record_id.to_string()],
        )?;
        Ok(())
    }

    /// Return a `{component_type: count}` summary of stored records.
    pub fn counts(&self) -> Result<HashMap<String, usize>, String> {
        let rows = self.db.fetch_all(
            "SELECT component_type, COUNT(*) AS n FROM ir_records \
             GROUP BY component_type ORDER BY component_type",
            &[],
        )?;
        let mut map = HashMap::new();
        for row in &rows {
            let ct = row.get("component_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let n = row.get("n").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
            map.insert(ct, n);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;

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

    fn make_record(ct: &str, id: &str) -> JsonObject {
        let mut m = serde_json::Map::new();
        m.insert("component_type".into(), Value::String(ct.into()));
        m.insert("id".into(), Value::String(id.into()));
        m.insert("name".into(), Value::String(format!("{ct}/{id}")));
        m
    }

    #[test]
    fn upsert_and_get() {
        let db = test_db();
        let repo = IRRecordRepository::new(&db);
        let rec = make_record("creature", "wolf");
        let n = repo.upsert_many(&[rec.clone()], "authored").unwrap();
        assert_eq!(n, 1);
        let got = repo.get("creature", "wolf").unwrap().unwrap();
        assert_eq!(got["id"], "wolf");
    }

    #[test]
    fn upsert_skips_missing_fields() {
        let db = test_db();
        let repo = IRRecordRepository::new(&db);
        let mut rec: JsonObject = serde_json::Map::new();
        rec.insert("name".into(), Value::String("no-type".into()));
        let n = repo.upsert_many(&[rec], "authored").unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn list_and_counts() {
        let db = test_db();
        let repo = IRRecordRepository::new(&db);
        repo.upsert_many(&[make_record("creature", "wolf"), make_record("creature", "bear")], "authored").unwrap();
        repo.upsert_many(&[make_record("item", "sword")], "authored").unwrap();
        let all = repo.list_records(None).unwrap();
        assert_eq!(all.len(), 3);
        let creatures = repo.list_records(Some("creature")).unwrap();
        assert_eq!(creatures.len(), 2);
        let counts = repo.counts().unwrap();
        assert_eq!(counts["creature"], 2);
        assert_eq!(counts["item"], 1);
    }

    #[test]
    fn delete_record() {
        let db = test_db();
        let repo = IRRecordRepository::new(&db);
        repo.upsert_many(&[make_record("trait", "tough")], "authored").unwrap();
        repo.delete("trait", "tough").unwrap();
        assert!(repo.get("trait", "tough").unwrap().is_none());
    }

    #[test]
    fn upsert_updates_existing() {
        let db = test_db();
        let repo = IRRecordRepository::new(&db);
        let mut rec = make_record("item", "axe");
        repo.upsert_many(&[rec.clone()], "authored").unwrap();
        rec.insert("extra".into(), Value::String("yes".into()));
        repo.upsert_many(&[rec], "migrated").unwrap();
        let got = repo.get("item", "axe").unwrap().unwrap();
        assert_eq!(got["extra"], "yes");
    }
}
