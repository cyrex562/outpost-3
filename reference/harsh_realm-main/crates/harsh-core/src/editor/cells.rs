//! Editor cell repository — CRUD for world cells used by the editor API.
//!
//! Ported from `src/harsh_realm/api/editor/cells.py`. The HTTP route
//! declarations (`@router.get`, etc.) belong to the B9 web-host layer and are
//! not included here.

use rusqlite::ToSql;

use crate::cell::CellData;
use crate::db::{Row, WorldDatabase};
use crate::editor_api::cells::{BulkCellUpdateBody, CellUpdateBody};
use crate::repositories::cell::CellRepository as RuntimeCellRepository;
use crate::runtime::{JsonObject, JsonValue};

fn row_i32(row: &Row, key: &str) -> i32 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
}
fn row_str(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn row_opt_str(row: &Row, key: &str) -> Option<String> {
    row.get(key).and_then(|v| v.as_str()).map(str::to_string)
}
fn row_bool(row: &Row, key: &str) -> bool {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0) != 0
}
fn parse_features(raw: Option<&str>) -> Vec<String> {
    serde_json::from_str(raw.unwrap_or("[]")).unwrap_or_default()
}
fn parse_obj(raw: Option<&str>) -> JsonObject {
    serde_json::from_str(raw.unwrap_or("{}")).unwrap_or_default()
}

/// Read/write adapter for cell editor operations.
pub struct EditorCellRepository<'a> {
    db: &'a WorldDatabase,
    runtime: RuntimeCellRepository<'a>,
}

impl<'a> EditorCellRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        EditorCellRepository { db, runtime: RuntimeCellRepository::new(db) }
    }

    /// List cells with pagination.
    pub fn list_cells(&self, limit: i64, offset: i64) -> Result<Vec<CellData>, String> {
        let rows = self.db.fetch_all(
            "SELECT q, r, terrain, features, explored, description, faction_id, data \
             FROM cells ORDER BY r, q LIMIT ? OFFSET ?",
            &[&limit, &offset],
        )?;
        rows.iter().map(|row| self.row_to_cell(row)).collect()
    }

    /// Fetch one cell by coordinate.
    pub fn get_cell(&self, q: i32, r: i32) -> Result<Option<CellData>, String> {
        let row = self.db.fetch_one(
            "SELECT q, r, terrain, features, explored, description, faction_id, data \
             FROM cells WHERE q=? AND r=?",
            &[&q, &r],
        )?;
        row.as_ref().map(|r| self.row_to_cell(r)).transpose()
    }

    /// Apply a partial cell update.
    pub fn update_cell(&self, q: i32, r: i32, body: &CellUpdateBody) -> Result<(), String> {
        let mut sets: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();

        if let Some(terrain) = &body.terrain {
            sets.push("terrain = ?".to_string());
            params.push(Box::new(terrain.clone()));
        }
        if let Some(features) = &body.features {
            sets.push("features = ?".to_string());
            params.push(Box::new(serde_json::to_string(features).unwrap_or_default()));
        }
        if let Some(explored) = body.explored {
            sets.push("explored = ?".to_string());
            params.push(Box::new(explored as i32));
        }
        if let Some(description) = &body.description {
            sets.push("description = ?".to_string());
            params.push(Box::new(description.clone()));
        }
        if let Some(faction_id) = &body.faction_id {
            sets.push("faction_id = ?".to_string());
            params.push(Box::new(faction_id.clone()));
        }
        if let Some(data) = &body.data {
            self.runtime.save_cell_data(q as i64, r as i64, data)?;
        }
        if !sets.is_empty() {
            let sql =
                format!("UPDATE cells SET {} WHERE q=? AND r=?", sets.join(", "));
            let mut refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
            let qv: &dyn ToSql = &q;
            let rv: &dyn ToSql = &r;
            refs.push(qv);
            refs.push(rv);
            self.db.execute(&sql, &refs)?;
        }
        Ok(())
    }

    /// Apply the same updates to multiple cells. Returns count updated.
    pub fn bulk_update_cells(&self, body: &BulkCellUpdateBody) -> Result<usize, String> {
        let mut updated = 0usize;
        for coord in &body.cells {
            let patch = CellUpdateBody {
                terrain: body.terrain.clone(),
                faction_id: body.faction_id.clone(),
                explored: body.explored,
                ..Default::default()
            };
            let empty =
                patch.terrain.is_none() && patch.faction_id.is_none() && patch.explored.is_none();
            if !empty {
                self.update_cell(coord.q, coord.r, &patch)?;
                updated += 1;
            }
        }
        Ok(updated)
    }

    fn row_to_cell(&self, row: &Row) -> Result<CellData, String> {
        let q = row_i32(row, "q");
        let r = row_i32(row, "r");
        let runtime_data = self.runtime.load_cell_data(q as i64, r as i64)?;
        let fallback_data: JsonValue =
            row.get("data").and_then(|v| v.as_str()).map(|s| {
                serde_json::from_str::<JsonValue>(s).unwrap_or(JsonValue::Object(Default::default()))
            }).unwrap_or(JsonValue::Object(Default::default()));
        let cell_data = runtime_data.unwrap_or_else(|| {
            if let JsonValue::Object(map) = fallback_data {
                map
            } else {
                Default::default()
            }
        });
        Ok(CellData {
            q,
            r,
            terrain: row_str(row, "terrain"),
            features: parse_features(row.get("features").and_then(|v| v.as_str())),
            explored: row_bool(row, "explored"),
            description: row_opt_str(row, "description"),
            faction_id: row_opt_str(row, "faction_id"),
            data: cell_data,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::editor_api::cells::CellCoordinate;

    fn make_db() -> WorldDatabase {
        WorldDatabase::open_in_memory().expect("in-memory db")
    }

    fn seed_cell(db: &WorldDatabase, q: i32, r: i32, terrain: &str) {
        db.execute(
            "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
             VALUES (?, ?, ?, '[]', 0, '{}')",
            &[&q, &r, &terrain],
        )
        .expect("seed");
    }

    #[test]
    fn list_cells_empty() {
        let db = make_db();
        let repo = EditorCellRepository::new(&db);
        let cells = repo.list_cells(100, 0).unwrap();
        assert!(cells.is_empty());
    }

    #[test]
    fn get_cell_none() {
        let db = make_db();
        let repo = EditorCellRepository::new(&db);
        assert!(repo.get_cell(0, 0).unwrap().is_none());
    }

    #[test]
    fn update_cell_terrain() {
        let db = make_db();
        seed_cell(&db, 1, 1, "plains");
        let repo = EditorCellRepository::new(&db);

        let body = CellUpdateBody {
            terrain: Some("forest".to_string()),
            ..Default::default()
        };
        repo.update_cell(1, 1, &body).unwrap();
        let cell = repo.get_cell(1, 1).unwrap().unwrap();
        assert_eq!(cell.terrain, "forest");
    }

    #[test]
    fn bulk_update_explored() {
        let db = make_db();
        seed_cell(&db, 0, 0, "plains");
        seed_cell(&db, 1, 0, "plains");
        let repo = EditorCellRepository::new(&db);

        let body = BulkCellUpdateBody {
            cells: vec![
                CellCoordinate { q: 0, r: 0 },
                CellCoordinate { q: 1, r: 0 },
            ],
            explored: Some(true),
            ..Default::default()
        };
        let count = repo.bulk_update_cells(&body).unwrap();
        assert_eq!(count, 2);
        assert!(repo.get_cell(0, 0).unwrap().unwrap().explored);
        assert!(repo.get_cell(1, 0).unwrap().unwrap().explored);
    }
}
