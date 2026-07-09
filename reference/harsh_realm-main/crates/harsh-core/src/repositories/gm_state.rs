//! Repository helpers for `gm_state` access.
//!
//! Ported from `src/harsh_realm/gm/gm_state_repository.py`.

use crate::db::WorldDatabase;

/// Centralizes simple `gm_state` reads/writes used by gameplay code.
pub struct GMStateRepository<'a> {
    db: &'a WorldDatabase,
}

impl<'a> GMStateRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        GMStateRepository { db }
    }

    /// Return a raw `gm_state` value by key.
    pub fn get_value(&self, key: &str) -> Result<Option<String>, String> {
        let row = self
            .db
            .fetch_one("SELECT value FROM gm_state WHERE key = ?", &[&key])?;
        Ok(row.and_then(|r| r.get("value").and_then(|v| v.as_str().map(str::to_string))))
    }

    /// Return an integer `gm_state` value, falling back on parse failure.
    pub fn get_int(&self, key: &str, default: i64) -> Result<i64, String> {
        Ok(match self.get_value(key)? {
            Some(s) => s.trim().parse::<i64>().unwrap_or(default),
            None => default,
        })
    }

    /// Persist a `gm_state` value.
    pub fn set_value(&self, key: &str, value: &str) -> Result<(), String> {
        self.db.execute(
            "INSERT OR REPLACE INTO gm_state (key, value) VALUES (?, ?)",
            &[&key, &value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_values_and_ints() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = GMStateRepository::new(&db);
        assert_eq!(repo.get_value("chaos").unwrap(), None);
        assert_eq!(repo.get_int("chaos", 5).unwrap(), 5);
        repo.set_value("chaos", "7").unwrap();
        assert_eq!(repo.get_value("chaos").unwrap().as_deref(), Some("7"));
        assert_eq!(repo.get_int("chaos", 5).unwrap(), 7);
        repo.set_value("chaos", "notnum").unwrap();
        assert_eq!(repo.get_int("chaos", 5).unwrap(), 5);
    }
}
