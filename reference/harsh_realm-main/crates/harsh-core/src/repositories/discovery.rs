//! Repository helpers for discovery-related cell persistence.
//!
//! Ported from `src/harsh_realm/engine/discovery_repository.py`. `CellDataPayload`
//! is a plain `JsonObject` here.

use crate::db::WorldDatabase;
use crate::repositories::cell::CellRepository;
use crate::runtime::JsonObject;

/// Centralizes the cell reads and writes used by the discovery system.
pub struct DiscoveryRepository<'a> {
    db: &'a WorldDatabase,
    cells: CellRepository<'a>,
}

impl<'a> DiscoveryRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        DiscoveryRepository {
            db,
            cells: CellRepository::new(db),
        }
    }

    /// Load a cell's JSON payload, falling back to provided data.
    pub fn load_cell_data(
        &self,
        q: i64,
        r: i64,
        fallback_data: Option<JsonObject>,
    ) -> Result<JsonObject, String> {
        if let Some(data) = self.cells.load_cell_data(q, r)? {
            return Ok(data);
        }
        Ok(fallback_data.unwrap_or_default())
    }

    /// Persist a cell JSON payload.
    pub fn save_cell_data(&self, q: i64, r: i64, data: &JsonObject) -> Result<(), String> {
        self.cells.save_cell_data(q, r, data)
    }

    /// Persist a cell feature list.
    pub fn save_cell_features(&self, q: i64, r: i64, features: &[String]) -> Result<(), String> {
        let json = serde_json::to_string(features).map_err(|e| e.to_string())?;
        self.db.execute(
            "UPDATE cells SET features = ? WHERE q = ? AND r = ?",
            &[&json, &q, &r],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::JsonValue;

    #[test]
    fn load_falls_back_when_cell_absent() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = DiscoveryRepository::new(&db);
        let mut fallback = JsonObject::new();
        fallback.insert("seed".into(), JsonValue::from(1));
        let data = repo.load_cell_data(9, 9, Some(fallback.clone())).unwrap();
        assert_eq!(data.get("seed"), Some(&JsonValue::from(1)));
    }

    #[test]
    fn save_and_reload_cell_data() {
        let db = WorldDatabase::open_in_memory().unwrap();
        db.execute(
            "INSERT INTO cells (q, r, terrain, features, explored, data) VALUES (0, 0, 'plains', '[]', 0, '{}')",
            &[],
        )
        .unwrap();
        let repo = DiscoveryRepository::new(&db);
        let mut data = JsonObject::new();
        data.insert("searched".into(), JsonValue::from(true));
        repo.save_cell_data(0, 0, &data).unwrap();
        let loaded = repo.load_cell_data(0, 0, None).unwrap();
        assert_eq!(loaded.get("searched"), Some(&JsonValue::from(true)));
    }
}
