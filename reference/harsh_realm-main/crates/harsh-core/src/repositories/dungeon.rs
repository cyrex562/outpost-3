//! Typed persistence helpers for dungeon records.
//!
//! Ported from `src/harsh_realm/gm/dungeon_repository.py`.

use std::collections::BTreeMap;

use rusqlite::ToSql;
use uuid::Uuid;

use crate::admin::oracle_dungeon::{DungeonCreated, DungeonRecord, DungeonSummary};
use crate::db::{Row, WorldDatabase};
use crate::editor_api::dungeons::DungeonUpdateRequest;
use crate::runtime::{JsonObject, JsonValue};
use crate::scene_data::{DungeonConnection, DungeonRoom};

/// Read/write adapter for dungeon persistence.
pub struct DungeonRepository<'a> {
    db: &'a WorldDatabase,
}

fn rs(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn roi(row: &Row, key: &str) -> Option<i32> {
    row.get(key).and_then(|v| v.as_i64()).map(|x| x as i32)
}

impl<'a> DungeonRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        DungeonRepository { db }
    }

    /// Return all dungeons ordered by name.
    pub fn list_dungeons(&self) -> Result<Vec<DungeonSummary>, String> {
        let rows = self.db.fetch_all(
            "SELECT id, name, location_q, location_r FROM dungeons ORDER BY name",
            &[],
        )?;
        let counts = self.load_room_counts()?;
        Ok(rows
            .iter()
            .map(|row| {
                let id = rs(row, "id");
                DungeonSummary {
                    room_count: counts.get(&id).copied().unwrap_or(0),
                    id,
                    name: rs(row, "name"),
                    location_q: roi(row, "location_q"),
                    location_r: roi(row, "location_r"),
                }
            })
            .collect())
    }

    /// Return a dungeon by id, or `None`.
    pub fn get_dungeon(&self, dungeon_id: &str) -> Result<Option<DungeonRecord>, String> {
        let row = self
            .db
            .fetch_one("SELECT * FROM dungeons WHERE id=?", &[&dungeon_id])?;
        let Some(row) = row else { return Ok(None) };
        let rooms: Vec<JsonObject> = self
            .load_rooms(dungeon_id)?
            .iter()
            .map(|r| serde_json::to_value(r).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default())
            .collect();
        let connections: Vec<JsonObject> = self
            .load_connections(dungeon_id)?
            .iter()
            .map(|c| serde_json::to_value(c).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default())
            .collect();
        let data: JsonObject =
            serde_json::from_str(row.get("data").and_then(|v| v.as_str()).unwrap_or("{}")).unwrap_or_default();
        Ok(Some(DungeonRecord {
            id: rs(&row, "id"),
            name: rs(&row, "name"),
            location_q: roi(&row, "location_q"),
            location_r: roi(&row, "location_r"),
            rooms,
            connections,
            data,
        }))
    }

    /// Return the first dungeon at a map location, or `None`.
    pub fn get_dungeon_at_location(&self, q: i64, r: i64) -> Result<Option<DungeonRecord>, String> {
        let row = self.db.fetch_one(
            "SELECT id FROM dungeons WHERE location_q=? AND location_r=? ORDER BY name LIMIT 1",
            &[&q, &r],
        )?;
        match row {
            Some(row) => self.get_dungeon(&rs(&row, "id")),
            None => Ok(None),
        }
    }

    /// Create a dungeon and return the creation result.
    pub fn create_dungeon(&self, body: &DungeonUpdateRequest) -> Result<DungeonCreated, String> {
        let dungeon_id = Uuid::new_v4().to_string()[..8].to_string();
        let name = body.name.clone().unwrap_or_else(|| "New Dungeon".to_string());
        let data_json = serde_json::to_string(body.data.as_ref().unwrap_or(&JsonObject::new()))
            .map_err(|e| e.to_string())?;
        self.db.execute(
            "INSERT INTO dungeons (id, name, location_q, location_r, rooms, connections, data) VALUES (?, ?, ?, ?, ?, ?, ?)",
            &[
                &dungeon_id,
                &name,
                &(body.location_q.unwrap_or(0) as i64),
                &(body.location_r.unwrap_or(0) as i64),
                &"[]",
                &"[]",
                &data_json,
            ],
        )?;
        let rooms = body.rooms.clone().unwrap_or_else(default_entrance);
        self.replace_rooms(&dungeon_id, &rooms)?;
        self.replace_connections(&dungeon_id, &body.connections.clone().unwrap_or_default())?;
        Ok(DungeonCreated { id: dungeon_id, name })
    }

    /// Update an existing dungeon. Returns false when it does not exist.
    pub fn update_dungeon(&self, dungeon_id: &str, body: &DungeonUpdateRequest) -> Result<bool, String> {
        if self.db.fetch_one("SELECT id FROM dungeons WHERE id=?", &[&dungeon_id])?.is_none() {
            return Ok(false);
        }
        let mut set_parts: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        if let Some(name) = &body.name {
            set_parts.push("name = ?".into());
            params.push(Box::new(name.clone()));
        }
        if let Some(q) = body.location_q {
            set_parts.push("location_q = ?".into());
            params.push(Box::new(q as i64));
        }
        if let Some(r) = body.location_r {
            set_parts.push("location_r = ?".into());
            params.push(Box::new(r as i64));
        }
        if let Some(data) = &body.data {
            set_parts.push("data = ?".into());
            params.push(Box::new(serde_json::to_string(data).map_err(|e| e.to_string())?));
        }
        if !set_parts.is_empty() {
            params.push(Box::new(dungeon_id.to_string()));
            let sql = format!("UPDATE dungeons SET {} WHERE id=?", set_parts.join(", "));
            let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
            self.db.execute(&sql, &refs)?;
        }
        if let Some(rooms) = &body.rooms {
            self.replace_rooms(dungeon_id, rooms)?;
        }
        if let Some(connections) = &body.connections {
            self.replace_connections(dungeon_id, connections)?;
        }
        Ok(true)
    }

    /// Delete a dungeon by id. Returns false when it does not exist.
    pub fn delete_dungeon(&self, dungeon_id: &str) -> Result<bool, String> {
        if self.db.fetch_one("SELECT id FROM dungeons WHERE id=?", &[&dungeon_id])?.is_none() {
            return Ok(false);
        }
        self.db.execute("DELETE FROM dungeon_rooms WHERE dungeon_id=?", &[&dungeon_id])?;
        self.db.execute("DELETE FROM dungeon_connections WHERE dungeon_id=?", &[&dungeon_id])?;
        self.db.execute("DELETE FROM dungeons WHERE id=?", &[&dungeon_id])?;
        Ok(true)
    }

    /// Return ordered room models for a dungeon (falls back to legacy JSON).
    pub fn load_rooms(&self, dungeon_id: &str) -> Result<Vec<DungeonRoom>, String> {
        let rows = self.db.fetch_all(
            "SELECT room_json FROM dungeon_rooms WHERE dungeon_id=? ORDER BY room_index",
            &[&dungeon_id],
        )?;
        if rows.is_empty() {
            let legacy = self.db.fetch_one("SELECT rooms FROM dungeons WHERE id=?", &[&dungeon_id])?;
            let Some(legacy) = legacy else { return Ok(vec![]) };
            return Ok(serde_json::from_str(legacy.get("rooms").and_then(|v| v.as_str()).unwrap_or("[]")).unwrap_or_default());
        }
        Ok(rows
            .iter()
            .filter_map(|row| serde_json::from_str(row.get("room_json").and_then(|v| v.as_str())?).ok())
            .collect())
    }

    /// Return ordered connection models for a dungeon (falls back to legacy JSON).
    pub fn load_connections(&self, dungeon_id: &str) -> Result<Vec<DungeonConnection>, String> {
        let rows = self.db.fetch_all(
            "SELECT connection_json FROM dungeon_connections WHERE dungeon_id=? ORDER BY connection_index",
            &[&dungeon_id],
        )?;
        if rows.is_empty() {
            let legacy = self.db.fetch_one("SELECT connections FROM dungeons WHERE id=?", &[&dungeon_id])?;
            let Some(legacy) = legacy else { return Ok(vec![]) };
            return Ok(serde_json::from_str(legacy.get("connections").and_then(|v| v.as_str()).unwrap_or("[]")).unwrap_or_default());
        }
        Ok(rows
            .iter()
            .filter_map(|row| serde_json::from_str(row.get("connection_json").and_then(|v| v.as_str())?).ok())
            .collect())
    }

    fn replace_rooms(&self, dungeon_id: &str, rooms: &[DungeonRoom]) -> Result<(), String> {
        self.db.execute("DELETE FROM dungeon_rooms WHERE dungeon_id=?", &[&dungeon_id])?;
        for (index, room) in rooms.iter().enumerate() {
            let json = serde_json::to_string(room).map_err(|e| e.to_string())?;
            let idx = index as i64;
            self.db.execute(
                "INSERT INTO dungeon_rooms (dungeon_id, room_id, room_index, room_json) VALUES (?, ?, ?, ?)",
                &[&dungeon_id, &room.id, &idx, &json],
            )?;
        }
        Ok(())
    }

    fn replace_connections(&self, dungeon_id: &str, connections: &[DungeonConnection]) -> Result<(), String> {
        self.db.execute("DELETE FROM dungeon_connections WHERE dungeon_id=?", &[&dungeon_id])?;
        for (index, connection) in connections.iter().enumerate() {
            let json = serde_json::to_string(connection).map_err(|e| e.to_string())?;
            let idx = index as i64;
            self.db.execute(
                "INSERT INTO dungeon_connections (dungeon_id, connection_index, connection_json) VALUES (?, ?, ?)",
                &[&dungeon_id, &idx, &json],
            )?;
        }
        Ok(())
    }

    fn load_room_counts(&self) -> Result<BTreeMap<String, i32>, String> {
        let rows = self.db.fetch_all(
            "SELECT dungeon_id, COUNT(*) AS room_count FROM dungeon_rooms GROUP BY dungeon_id",
            &[],
        )?;
        Ok(rows
            .iter()
            .map(|row| (rs(row, "dungeon_id"), row.get("room_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32))
            .collect())
    }
}

fn default_entrance() -> Vec<DungeonRoom> {
    let mut data = JsonObject::new();
    let mut pos = JsonObject::new();
    pos.insert("x".into(), JsonValue::from(200));
    pos.insert("y".into(), JsonValue::from(200));
    data.insert("editor_pos".into(), JsonValue::Object(pos));
    vec![DungeonRoom {
        id: "room_1".into(),
        name: "Entrance".into(),
        r#type: "entrance".into(),
        x: None,
        y: None,
        w: None,
        h: None,
        center_q: None,
        center_r: None,
        editor_pos: None,
        description: Some(String::new()),
        loot: vec![],
        hidden_loot: vec![],
        traps: vec![],
        encounter: None,
        data,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_get_update_delete() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = DungeonRepository::new(&db);
        let mut body = DungeonUpdateRequest::default();
        body.name = Some("Crypt".into());
        body.location_q = Some(3);
        body.location_r = Some(4);
        let created = repo.create_dungeon(&body).unwrap();
        assert_eq!(created.name, "Crypt");

        let got = repo.get_dungeon(&created.id).unwrap().unwrap();
        assert_eq!(got.name, "Crypt");
        assert_eq!(got.rooms.len(), 1); // default entrance

        let list = repo.list_dungeons().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].room_count, 1);

        let at = repo.get_dungeon_at_location(3, 4).unwrap();
        assert!(at.is_some());

        let mut upd = DungeonUpdateRequest::default();
        upd.name = Some("Deep Crypt".into());
        assert!(repo.update_dungeon(&created.id, &upd).unwrap());
        assert_eq!(repo.get_dungeon(&created.id).unwrap().unwrap().name, "Deep Crypt");

        assert!(repo.delete_dungeon(&created.id).unwrap());
        assert!(repo.get_dungeon(&created.id).unwrap().is_none());
        assert!(!repo.delete_dungeon("missing").unwrap());
    }
}
