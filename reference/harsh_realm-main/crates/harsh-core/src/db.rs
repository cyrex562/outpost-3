//! World database wrapper.
//!
//! Ported from `src/harsh_realm/db.py` and its query/schema mixins. The Python
//! version is async (`aiosqlite`); this is synchronous `rusqlite` — the web host
//! runs DB work off the request thread. The legacy backfill mixin
//! (`_db_backfill_mixin.py`, which migrated pre-relational Python worlds) is
//! intentionally not ported: Rust worlds are created relational from
//! `SCHEMA_SQL`.

use std::path::{Path, PathBuf};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, ToSql};

use crate::admin::common::WorldFileInfo;
use crate::db_schema::{REQUIRED_TABLES, SCHEMA_SQL};
use crate::runtime::{JsonObject, JsonValue};

/// A database row as a column-name -> JSON-value map (mirrors `aiosqlite.Row`
/// dict access).
pub type Row = JsonObject;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn valueref_to_json(v: ValueRef<'_>) -> JsonValue {
    match v {
        ValueRef::Null => JsonValue::Null,
        ValueRef::Integer(i) => JsonValue::from(i),
        ValueRef::Real(f) => JsonValue::from(f),
        ValueRef::Text(t) => JsonValue::from(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => JsonValue::from(b.to_vec()),
    }
}

/// Synchronous wrapper around a SQLite connection to one world `.db` file.
pub struct WorldDatabase {
    conn: Connection,
    filepath: PathBuf,
}

impl WorldDatabase {
    /// Create a new world database file with the full schema and base metadata.
    pub fn create(
        filepath: impl AsRef<Path>,
        name: &str,
        settings: Option<&JsonObject>,
    ) -> Result<Self, String> {
        let path = filepath.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        // Match Python (aiosqlite default): foreign keys are not enforced.
        conn.execute_batch("PRAGMA foreign_keys = OFF;").map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())?;
        let db = WorldDatabase { conn, filepath: path };

        let now = now_iso();
        db.set_meta("name", name)?;
        db.set_meta("created_at", &now)?;
        db.set_meta("updated_at", &now)?;
        db.set_meta("tick", "0")?;
        if let Some(settings) = settings {
            for (key, value) in settings {
                let stored = match value {
                    JsonValue::String(s) => s.clone(),
                    other => other.to_string(),
                };
                db.set_meta(key, &stored)?;
            }
        }
        Ok(db)
    }

    /// Open an existing world database file (runs the idempotent schema, then
    /// verifies required tables).
    pub fn open(filepath: impl AsRef<Path>) -> Result<Self, String> {
        let path = filepath.as_ref().to_path_buf();
        if !path.exists() {
            return Err(format!("World database not found: {path:?}"));
        }
        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys = OFF;").map_err(|e| e.to_string())?;
        let db = WorldDatabase { conn, filepath: path };
        // SCHEMA_SQL is CREATE TABLE IF NOT EXISTS — safe forward migration.
        db.conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())?;
        db.verify_schema()?;
        Ok(db)
    }

    /// Open an in-memory database (for tests).
    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys = OFF;").map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA_SQL).map_err(|e| e.to_string())?;
        Ok(WorldDatabase {
            conn,
            filepath: PathBuf::from(":memory:"),
        })
    }

    /// Execute a parameterized statement; returns the number of affected rows.
    pub fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> Result<usize, String> {
        self.conn.execute(sql, params).map_err(|e| e.to_string())
    }

    /// Execute a multi-statement SQL script.
    pub fn execute_script(&self, sql: &str) -> Result<(), String> {
        self.conn.execute_batch(sql).map_err(|e| e.to_string())
    }

    /// Row id of the most recent successful `INSERT` on this connection.
    pub fn last_insert_rowid(&self) -> i64 {
        self.conn.last_insert_rowid()
    }

    /// Execute a query and return the first row, if any.
    pub fn fetch_one(&self, sql: &str, params: &[&dyn ToSql]) -> Result<Option<Row>, String> {
        Ok(self.fetch_all(sql, params)?.into_iter().next())
    }

    /// Execute a query and return all rows as column-name -> value maps.
    pub fn fetch_all(&self, sql: &str, params: &[&dyn ToSql]) -> Result<Vec<Row>, String> {
        let mut stmt = self.conn.prepare(sql).map_err(|e| e.to_string())?;
        let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows = stmt
            .query_map(params, |row| {
                let mut map = Row::new();
                for (i, name) in cols.iter().enumerate() {
                    map.insert(name.clone(), valueref_to_json(row.get_ref(i)?));
                }
                Ok(map)
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    /// Retrieve a value from `world_meta`.
    pub fn get_meta(&self, key: &str) -> Result<Option<String>, String> {
        let row = self.fetch_one("SELECT value FROM world_meta WHERE key = ?", &[&key])?;
        Ok(row.and_then(|r| r.get("value").and_then(|v| v.as_str().map(str::to_string))))
    }

    /// Insert or replace a value in `world_meta`.
    pub fn set_meta(&self, key: &str, value: &str) -> Result<(), String> {
        self.execute(
            "INSERT OR REPLACE INTO world_meta (key, value) VALUES (?, ?)",
            &[&key, &value],
        )?;
        Ok(())
    }

    /// Copy the database to a named snapshot file alongside the world file.
    pub fn save_snapshot(&self, name: &str) -> Result<PathBuf, String> {
        let stem = self
            .filepath
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("world");
        let snapshot_path = self.filepath.with_file_name(format!("{stem}_{name}.db"));
        let mut dest = Connection::open(&snapshot_path).map_err(|e| e.to_string())?;
        let backup = rusqlite::backup::Backup::new(&self.conn, &mut dest)
            .map_err(|e| e.to_string())?;
        backup
            .run_to_completion(100, std::time::Duration::from_millis(0), None)
            .map_err(|e| e.to_string())?;
        Ok(snapshot_path)
    }

    /// Verify all required tables exist.
    pub fn verify_schema(&self) -> Result<(), String> {
        let rows = self.fetch_all(
            "SELECT name FROM sqlite_master WHERE type='table'",
            &[],
        )?;
        let existing: std::collections::BTreeSet<String> = rows
            .iter()
            .filter_map(|r| r.get("name").and_then(|v| v.as_str().map(str::to_string)))
            .collect();
        let missing: Vec<&str> = REQUIRED_TABLES
            .iter()
            .copied()
            .filter(|t| !existing.contains(*t))
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!("World database is missing tables: {missing:?}"))
        }
    }

    /// Run `f` inside a single SQLite transaction: BEGIN IMMEDIATE, run the
    /// closure, COMMIT on Ok / ROLLBACK on Err.  Groups multiple repository
    /// writes so they all commit or all roll back.
    ///
    /// **NON-REENTRANT** — do not call from inside another `transaction()`.
    /// SQLite does not support nested `BEGIN`; a nested call will return an
    /// error.  The closure does its writes via the same `&self` (e.g.
    /// `EntityRepository::new(db).save_character(…)`).
    /// The closure must not panic; a panic leaves the transaction open (no
    /// ROLLBACK is issued on unwind).
    pub fn transaction<F, T>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        self.execute("BEGIN IMMEDIATE", &[])?;
        match f() {
            Ok(v) => match self.execute("COMMIT", &[]) {
                Ok(_) => Ok(v),
                Err(e) => {
                    // COMMIT failed — roll back so the transaction is not left
                    // open (which would poison the next BEGIN).
                    let _ = self.execute("ROLLBACK", &[]);
                    Err(e)
                }
            },
            Err(e) => {
                // Best-effort rollback; return the original error.
                let _ = self.execute("ROLLBACK", &[]);
                Err(e)
            }
        }
    }

    /// Path to the world database file.
    pub fn filepath(&self) -> &Path {
        &self.filepath
    }

    /// List all world database files in a directory, newest first.
    pub fn list_worlds(directory: impl AsRef<Path>) -> Vec<WorldFileInfo> {
        let dir = directory.as_ref();
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            return vec![];
        };
        let mut worlds: Vec<WorldFileInfo> = read_dir
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "db"))
            .filter_map(|p| {
                let meta = std::fs::metadata(&p).ok()?;
                let modified = meta.modified().ok()?;
                let last_modified = chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339();
                Some(WorldFileInfo {
                    file: p.file_name()?.to_string_lossy().into_owned(),
                    name: p.file_stem()?.to_string_lossy().into_owned(),
                    last_modified,
                })
            })
            .collect();
        worlds.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
        worlds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_initializes_schema_and_meta() {
        let db = WorldDatabase::open_in_memory().unwrap();
        db.verify_schema().unwrap();
        db.set_meta("name", "Ashfall").unwrap();
        assert_eq!(db.get_meta("name").unwrap().as_deref(), Some("Ashfall"));
        assert_eq!(db.get_meta("missing").unwrap(), None);
    }

    #[test]
    fn execute_and_fetch_rows() {
        let db = WorldDatabase::open_in_memory().unwrap();
        db.execute(
            "INSERT INTO entities (id, entity_type, name, alive, data, created_at, updated_at) VALUES (?,?,?,?,?,?,?)",
            &[&"e1", &"character", &"Hero", &1i64, &"{}", &"now", &"now"],
        )
        .unwrap();
        let row = db
            .fetch_one("SELECT name, alive FROM entities WHERE id = ?", &[&"e1"])
            .unwrap()
            .unwrap();
        assert_eq!(row.get("name").unwrap().as_str(), Some("Hero"));
        assert_eq!(row.get("alive").unwrap().as_i64(), Some(1));
        let all = db.fetch_all("SELECT id FROM entities", &[]).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn transaction_commits_on_ok() {
        let db = WorldDatabase::open_in_memory().unwrap();
        // A closure that does a real INSERT and returns Ok — should be visible after.
        let result = db.transaction(|| {
            db.execute(
                "INSERT INTO entities (id, entity_type, name, alive, data, created_at, updated_at) \
                 VALUES (?,?,?,?,?,?,?)",
                &[&"tx1", &"character", &"TxHero", &1i64, &"{}", &"now", &"now"],
            )?;
            Ok(42u32)
        });
        assert_eq!(result.unwrap(), 42u32);
        let row = db
            .fetch_one("SELECT name FROM entities WHERE id = ?", &[&"tx1"])
            .unwrap()
            .expect("row should be committed");
        assert_eq!(row.get("name").unwrap().as_str(), Some("TxHero"));
    }

    #[test]
    fn transaction_rolls_back_on_err() {
        let db = WorldDatabase::open_in_memory().unwrap();
        // A closure that inserts a row then returns Err — the row must NOT be present.
        let result: Result<(), String> = db.transaction(|| {
            db.execute(
                "INSERT INTO entities (id, entity_type, name, alive, data, created_at, updated_at) \
                 VALUES (?,?,?,?,?,?,?)",
                &[&"tx2", &"character", &"GhostHero", &1i64, &"{}", &"now", &"now"],
            )?;
            Err("simulated failure".to_string())
        });
        assert!(result.is_err());
        // ROLLBACK must have undone the insert.
        let row = db
            .fetch_one("SELECT name FROM entities WHERE id = ?", &[&"tx2"])
            .unwrap();
        assert!(row.is_none(), "rolled-back row should not be visible");
    }

    #[test]
    fn create_world_file_and_list() {
        let dir = std::env::temp_dir().join(format!("hr_db_{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("ashfall.db");
        let db = WorldDatabase::create(&path, "Ashfall", None).unwrap();
        assert_eq!(db.get_meta("name").unwrap().as_deref(), Some("Ashfall"));
        assert_eq!(db.get_meta("tick").unwrap().as_deref(), Some("0"));
        let worlds = WorldDatabase::list_worlds(&dir);
        assert!(worlds.iter().any(|w| w.name == "ashfall"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
