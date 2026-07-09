//! Repository helpers for cell/world persistence used by gameplay handlers.
//!
//! Ported from `src/harsh_realm/gm/cell_repository.py`. The Python
//! `CellDataPayload` wrapper is just a `JsonObject` here.

use std::collections::BTreeSet;

use crate::cell::{CellData, CellDeathMarker, CellSearchState, CellSettlementState, TerrainRegistry};
use crate::db::{Row, WorldDatabase};
use crate::grid::{Grid, GridCoord};
use crate::runtime::{JsonObject, JsonValue};
use crate::scene_data::{SettlementBuilding, TownCell};

/// Centralizes common cell update patterns for gameplay mutations.
pub struct CellRepository<'a> {
    db: &'a WorldDatabase,
}

fn parse_obj(raw: Option<&str>) -> JsonObject {
    serde_json::from_str(raw.unwrap_or("{}")).unwrap_or_default()
}

fn row_obj(row: &Row, key: &str) -> JsonObject {
    parse_obj(row.get(key).and_then(|v| v.as_str()))
}

fn row_i64(row: &Row, key: &str) -> i64 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

fn row_string(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}

fn row_opt_string(row: &Row, key: &str) -> Option<String> {
    row.get(key).and_then(|v| v.as_str()).map(str::to_string)
}

fn json_list_of_obj(raw: Option<&str>) -> Vec<JsonObject> {
    serde_json::from_str(raw.unwrap_or("[]")).unwrap_or_default()
}

impl<'a> CellRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        CellRepository { db }
    }

    /// Update a cell's explored state.
    pub fn mark_explored(&self, q: i64, r: i64, explored: i64) -> Result<(), String> {
        self.db.execute(
            "UPDATE cells SET explored = ? WHERE q = ? AND r = ?",
            &[&explored, &q, &r],
        )?;
        Ok(())
    }

    /// Reveal an adjacent cell if it is currently unexplored.
    pub fn reveal_adjacent(&self, q: i64, r: i64) -> Result<(), String> {
        self.db.execute(
            "UPDATE cells SET explored = 2 WHERE q = ? AND r = ? AND explored = 0",
            &[&q, &r],
        )?;
        Ok(())
    }

    /// Persist a cell payload, splitting known state into typed tables.
    pub fn save_cell_data(&self, q: i64, r: i64, data: &JsonObject) -> Result<(), String> {
        let mut payload = data.clone();
        self.save_settlement_state(q, r, &mut payload)?;
        self.save_search_state(q, r, &mut payload)?;
        self.save_death_markers_internal(q, r, &mut payload)?;
        let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        self.db.execute(
            "UPDATE cells SET data = ? WHERE q = ? AND r = ?",
            &[&json, &q, &r],
        )?;
        Ok(())
    }

    /// Load a cell payload by coordinate, hydrating typed cell-state rows.
    pub fn load_cell_data(&self, q: i64, r: i64) -> Result<Option<JsonObject>, String> {
        let row = self
            .db
            .fetch_one("SELECT data FROM cells WHERE q = ? AND r = ?", &[&q, &r])?;
        let Some(row) = row else { return Ok(None) };
        let mut data = row_obj(&row, "data");

        if let Some(settlement) = self.load_settlement_state(q, r)? {
            data.insert("settlement".into(), JsonValue::Object(settlement.to_payload()));
            let town_cells: Vec<JsonValue> = settlement
                .town_cells
                .iter()
                .map(|tc| serde_json::to_value(tc).unwrap_or(JsonValue::Null))
                .collect();
            data.insert("town_cells".into(), JsonValue::from(town_cells));
        }
        if let Some(search) = self.load_search_state(q, r)? {
            data.insert(
                "last_searched_tick".into(),
                JsonValue::from(search.last_searched_tick),
            );
        }
        let markers = self.load_death_markers_internal(q, r)?;
        if !markers.is_empty() {
            let arr: Vec<JsonValue> = markers
                .iter()
                .map(|m| {
                    let mut o = JsonObject::new();
                    o.insert("items".into(), JsonValue::from(m.items.clone()));
                    JsonValue::Object(o)
                })
                .collect();
            data.insert("death_markers".into(), JsonValue::from(arr));
        }
        Ok(Some(data))
    }

    /// Persist a cell's feature list.
    pub fn save_cell_features(&self, q: i64, r: i64, features: &[String]) -> Result<(), String> {
        let json = serde_json::to_string(features).map_err(|e| e.to_string())?;
        self.db.execute(
            "UPDATE cells SET features = ? WHERE q = ? AND r = ?",
            &[&json, &q, &r],
        )?;
        Ok(())
    }

    /// Find the preferred starting settlement, or a passable fallback.
    pub fn find_starting_hex(&self) -> Result<(i64, i64), String> {
        if let Some(row) = self
            .db
            .fetch_one("SELECT q, r FROM cell_settlements WHERE starting = 1 LIMIT 1", &[])?
        {
            return Ok((row_i64(&row, "q"), row_i64(&row, "r")));
        }
        let rows = self.db.fetch_all(
            "SELECT q, r, data FROM cells WHERE features LIKE '%settlement%'",
            &[],
        )?;
        for row in &rows {
            let data = row_obj(row, "data");
            let starting = data
                .get("settlement")
                .and_then(|s| s.get("starting"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if starting {
                return Ok((row_i64(row, "q"), row_i64(row, "r")));
            }
        }
        if let Some(first) = rows.first() {
            return Ok((row_i64(first, "q"), row_i64(first, "r")));
        }
        if let Some(row) = self.db.fetch_one(
            "SELECT q, r FROM cells WHERE terrain NOT IN ('mountains', 'water') LIMIT 1",
            &[],
        )? {
            return Ok((row_i64(&row, "q"), row_i64(&row, "r")));
        }
        Ok((0, 0))
    }

    /// Mark adjacent cells as seen around a center point.
    pub fn reveal_neighbors(&self, q: i64, r: i64, grid: &dyn Grid) -> Result<(), String> {
        let center = GridCoord::new(q as i32, r as i32);
        for direction in grid.directions() {
            if let Ok(nb) = grid.neighbor(center, direction) {
                self.reveal_adjacent(nb.q as i64, nb.r as i64)?;
            }
        }
        Ok(())
    }

    /// Return cell coordinates whose feature list contains a feature marker.
    pub fn list_cells_with_feature(&self, feature: &str) -> Result<Vec<GridCoord>, String> {
        let like = format!("%{feature}%");
        let rows = self
            .db
            .fetch_all("SELECT q, r FROM cells WHERE features LIKE ?", &[&like])?;
        Ok(rows
            .iter()
            .map(|row| GridCoord::new(row_i64(row, "q") as i32, row_i64(row, "r") as i32))
            .collect())
    }

    /// Return the set of (q, r) whose terrain is passable (for pathfinding).
    pub fn list_passable_coords(
        &self,
        registry: &TerrainRegistry,
    ) -> Result<BTreeSet<(i64, i64)>, String> {
        let rows = self.db.fetch_all("SELECT q, r, terrain FROM cells", &[])?;
        let mut passable = BTreeSet::new();
        for row in &rows {
            let terrain_id = row_string(row, "terrain");
            let is_passable = match registry.get(&terrain_id) {
                Some(t) => t.passable,
                None => true,
            };
            if is_passable {
                passable.insert((row_i64(row, "q"), row_i64(row, "r")));
            }
        }
        Ok(passable)
    }

    /// Load a cell row into a `CellData` object.
    pub fn fetch_cell(&self, coord: GridCoord) -> Result<Option<CellData>, String> {
        let q = coord.q as i64;
        let r = coord.r as i64;
        let row = self.db.fetch_one(
            "SELECT q, r, terrain, features, explored, description, faction_id, data \
             FROM cells WHERE q = ? AND r = ?",
            &[&q, &r],
        )?;
        let Some(row) = row else { return Ok(None) };
        let cell_data = self.load_cell_data(q, r)?.unwrap_or_default();
        let features: Vec<String> =
            serde_json::from_str(row.get("features").and_then(|v| v.as_str()).unwrap_or("[]"))
                .unwrap_or_default();
        Ok(Some(CellData {
            q: row_i64(&row, "q") as i32,
            r: row_i64(&row, "r") as i32,
            terrain: row_string(&row, "terrain"),
            features,
            explored: row_i64(&row, "explored") != 0,
            description: row_opt_string(&row, "description"),
            faction_id: row_opt_string(&row, "faction_id"),
            data: cell_data,
        }))
    }

    /// Persist only death-marker state for a cell.
    pub fn save_death_markers(
        &self,
        q: i64,
        r: i64,
        markers: Vec<JsonObject>,
    ) -> Result<(), String> {
        let mut payload = JsonObject::new();
        let arr: Vec<JsonValue> = markers.into_iter().map(JsonValue::Object).collect();
        payload.insert("death_markers".into(), JsonValue::from(arr));
        self.save_death_markers_internal(q, r, &mut payload)
    }

    // --- typed-state helpers -------------------------------------------------

    fn save_settlement_state(
        &self,
        q: i64,
        r: i64,
        payload: &mut JsonObject,
    ) -> Result<(), String> {
        let raw_settlement = payload.remove("settlement");
        let raw_town_cells = payload.remove("town_cells");
        let Some(JsonValue::Object(s)) = raw_settlement else {
            return Ok(());
        };
        let establishments: Vec<SettlementBuilding> = s
            .get("establishments")
            .cloned()
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();
        let town_cells: Vec<TownCell> = raw_town_cells
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();
        let est_json = serde_json::to_string(&establishments).map_err(|e| e.to_string())?;
        let residents: Vec<String> = s
            .get("resident_npc_ids")
            .cloned()
            .map(|v| serde_json::from_value(v).unwrap_or_default())
            .unwrap_or_default();
        let res_json = serde_json::to_string(&residents).map_err(|e| e.to_string())?;
        let tc_json = serde_json::to_string(&town_cells).map_err(|e| e.to_string())?;
        let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("Settlement").to_string();
        let size = s.get("size").and_then(|v| v.as_str()).unwrap_or("hamlet").to_string();
        let desc = s.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let sid = s.get("settlement_id").and_then(|v| v.as_str()).unwrap_or("generated").to_string();
        let starting: i64 = s.get("starting").and_then(|v| v.as_bool()).unwrap_or(false) as i64;
        self.db.execute(
            "INSERT INTO cell_settlements (q, r, name, size, description, settlement_id, starting, \
             establishments_json, resident_npc_ids_json, town_cells_json) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(q, r) DO UPDATE SET name=excluded.name, size=excluded.size, \
             description=excluded.description, settlement_id=excluded.settlement_id, \
             starting=excluded.starting, establishments_json=excluded.establishments_json, \
             resident_npc_ids_json=excluded.resident_npc_ids_json, town_cells_json=excluded.town_cells_json",
            &[&q, &r, &name, &size, &desc, &sid, &starting, &est_json, &res_json, &tc_json],
        )?;
        Ok(())
    }

    fn save_search_state(&self, q: i64, r: i64, payload: &mut JsonObject) -> Result<(), String> {
        let Some(raw) = payload.remove("last_searched_tick") else {
            return Ok(());
        };
        let tick = raw.as_i64().unwrap_or(0);
        self.db.execute(
            "INSERT INTO cell_search_state (q, r, last_searched_tick) VALUES (?, ?, ?) \
             ON CONFLICT(q, r) DO UPDATE SET last_searched_tick = excluded.last_searched_tick",
            &[&q, &r, &tick],
        )?;
        Ok(())
    }

    fn save_death_markers_internal(
        &self,
        q: i64,
        r: i64,
        payload: &mut JsonObject,
    ) -> Result<(), String> {
        let Some(raw) = payload.remove("death_markers") else {
            return Ok(());
        };
        let markers: Vec<JsonValue> = raw.as_array().cloned().unwrap_or_default();
        self.db.execute(
            "DELETE FROM cell_death_markers WHERE q = ? AND r = ?",
            &[&q, &r],
        )?;
        for (index, raw_marker) in markers.iter().enumerate() {
            let items = raw_marker.get("items").cloned().unwrap_or(JsonValue::Array(vec![]));
            let items_json = serde_json::to_string(&items).map_err(|e| e.to_string())?;
            let idx = index as i64;
            self.db.execute(
                "INSERT INTO cell_death_markers (q, r, marker_index, items_json) VALUES (?, ?, ?, ?)",
                &[&q, &r, &idx, &items_json],
            )?;
        }
        Ok(())
    }

    fn load_settlement_state(
        &self,
        q: i64,
        r: i64,
    ) -> Result<Option<CellSettlementState>, String> {
        let row = self.db.fetch_one(
            "SELECT q, r, name, size, description, settlement_id, starting, \
             establishments_json, resident_npc_ids_json, town_cells_json \
             FROM cell_settlements WHERE q = ? AND r = ?",
            &[&q, &r],
        )?;
        let Some(row) = row else { return Ok(None) };
        let establishments: Vec<SettlementBuilding> =
            serde_json::from_str(row.get("establishments_json").and_then(|v| v.as_str()).unwrap_or("[]"))
                .unwrap_or_default();
        let resident_npc_ids: Vec<String> =
            serde_json::from_str(row.get("resident_npc_ids_json").and_then(|v| v.as_str()).unwrap_or("[]"))
                .unwrap_or_default();
        let town_cells: Vec<TownCell> =
            serde_json::from_str(row.get("town_cells_json").and_then(|v| v.as_str()).unwrap_or("[]"))
                .unwrap_or_default();
        Ok(Some(CellSettlementState {
            q: row_i64(&row, "q") as i32,
            r: row_i64(&row, "r") as i32,
            name: row_string(&row, "name"),
            size: row_string(&row, "size"),
            description: row_string(&row, "description"),
            settlement_id: row_string(&row, "settlement_id"),
            starting: row_i64(&row, "starting") != 0,
            establishments,
            resident_npc_ids,
            town_cells,
        }))
    }

    fn load_search_state(&self, q: i64, r: i64) -> Result<Option<CellSearchState>, String> {
        let row = self.db.fetch_one(
            "SELECT q, r, last_searched_tick FROM cell_search_state WHERE q = ? AND r = ?",
            &[&q, &r],
        )?;
        Ok(row.map(|row| CellSearchState {
            q: row_i64(&row, "q") as i32,
            r: row_i64(&row, "r") as i32,
            last_searched_tick: row_i64(&row, "last_searched_tick") as i32,
        }))
    }

    fn load_death_markers_internal(
        &self,
        q: i64,
        r: i64,
    ) -> Result<Vec<CellDeathMarker>, String> {
        let rows = self.db.fetch_all(
            "SELECT q, r, marker_index, items_json FROM cell_death_markers \
             WHERE q = ? AND r = ? ORDER BY marker_index",
            &[&q, &r],
        )?;
        Ok(rows
            .iter()
            .map(|row| CellDeathMarker {
                q: row_i64(row, "q") as i32,
                r: row_i64(row, "r") as i32,
                marker_index: row_i64(row, "marker_index") as i32,
                items: json_list_of_obj(row.get("items_json").and_then(|v| v.as_str())),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_cell(db: &WorldDatabase, q: i64, r: i64, terrain: &str) {
        db.execute(
            "INSERT INTO cells (q, r, terrain, features, explored, data) VALUES (?, ?, ?, '[]', 0, '{}')",
            &[&q, &r, &terrain],
        )
        .unwrap();
    }

    #[test]
    fn explore_and_fetch() {
        let db = WorldDatabase::open_in_memory().unwrap();
        seed_cell(&db, 1, 2, "plains");
        let repo = CellRepository::new(&db);
        repo.mark_explored(1, 2, 1).unwrap();
        let cell = repo.fetch_cell(GridCoord::new(1, 2)).unwrap().unwrap();
        assert_eq!(cell.terrain, "plains");
        assert!(cell.explored);
    }

    #[test]
    fn save_and_hydrate_settlement_and_markers() {
        let db = WorldDatabase::open_in_memory().unwrap();
        seed_cell(&db, 0, 0, "plains");
        let repo = CellRepository::new(&db);
        let data: JsonObject = serde_json::from_str(
            r#"{"settlement":{"name":"Ashford","size":"village","settlement_id":"s1","starting":true},"death_markers":[{"items":[{"name":"coin"}]}],"last_searched_tick":42,"misc":1}"#,
        )
        .unwrap();
        repo.save_cell_data(0, 0, &data).unwrap();

        let loaded = repo.load_cell_data(0, 0).unwrap().unwrap();
        assert_eq!(loaded["settlement"]["name"], JsonValue::from("Ashford"));
        assert_eq!(loaded["last_searched_tick"], JsonValue::from(42));
        assert!(loaded["death_markers"].is_array());
        // misc stayed in the residual cells.data blob.
        assert_eq!(loaded["misc"], JsonValue::from(1));

        let (sq, sr) = repo.find_starting_hex().unwrap();
        assert_eq!((sq, sr), (0, 0));
    }

    #[test]
    fn passable_coords_uses_registry() {
        let db = WorldDatabase::open_in_memory().unwrap();
        seed_cell(&db, 0, 0, "plains");
        seed_cell(&db, 1, 0, "wall");
        let repo = CellRepository::new(&db);
        let mut reg = TerrainRegistry::new();
        // Without registered terrains, everything is treated passable.
        assert_eq!(repo.list_passable_coords(&reg).unwrap().len(), 2);
        // Register a non-passable wall.
        let _ = &mut reg;
        let dir = std::env::temp_dir().join(format!("hr_cellrepo_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("terrain.yaml"),
            "terrains:\n  - id: plains\n    name: P\n    passable: true\n    blocks_vision: false\n  - id: wall\n    name: W\n    passable: false\n    blocks_vision: true\n",
        )
        .unwrap();
        reg.load(&dir.join("terrain.yaml"), true).unwrap();
        assert_eq!(repo.list_passable_coords(&reg).unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
