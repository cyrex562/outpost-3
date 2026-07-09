//! Editor-facing entity reads.
//!
//! The content/world editor presents an entity as a single `data` blob, but a
//! character's derived stats live in the normalised `character_state` table (and
//! NPC personality on `npc_state`). This repository owns the SQL that assembles
//! those rows into the editor's `data` shape and lists/loads entities, so the
//! web layer no longer reimplements it with raw queries (one place to follow the
//! schema). The write-back path stays in the web layer for now (see HR-750).

use rusqlite::ToSql;
use serde_json::{Map, Value};

use crate::admin::oracle_dungeon::EditorEntityRecord;
use crate::db::{Row, WorldDatabase};
use crate::runtime::JsonObject;

/// Owns SQL access for editor entity reads.
pub struct EditorEntityRepository<'a> {
    db: &'a WorldDatabase,
}

impl<'a> EditorEntityRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        EditorEntityRepository { db }
    }

    /// Assemble the `character_state` row (+ NPC personality) for an entity into
    /// the editor `data` shape, or `None` if the entity has no character state.
    pub fn assemble_character_data(&self, id: &str) -> Result<Option<JsonObject>, String> {
        let Some(row) = self
            .db
            .fetch_one("SELECT * FROM character_state WHERE entity_id = ?", &[&id])?
        else {
            return Ok(None);
        };
        let mut data = Map::new();
        for key in [
            "level", "xp", "xp_next", "hp", "max_hp", "ac", "attack_bonus", "physical_save",
            "evasion_save", "mental_save", "position_q", "position_r",
        ] {
            data.insert(
                key.into(),
                Value::from(row.get(key).and_then(Value::as_i64).unwrap_or(0)),
            );
        }
        data.insert(
            "character_class".into(),
            Value::from(row.get("character_class").and_then(Value::as_str).unwrap_or("")),
        );
        for (json_key, col) in [
            ("attributes", "attributes_json"),
            ("attr_mods", "attr_mods_json"),
            ("skills", "skills_json"),
            ("equipment", "equipment_json"),
            ("class_abilities", "class_abilities_json"),
        ] {
            let parsed: Value = row
                .get(col)
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_else(|| if json_key == "equipment" { Value::Array(vec![]) } else { Value::Object(Map::new()) });
            data.insert(json_key.into(), parsed);
        }
        // Personality lives on npc_state; include it when present.
        if let Some(npc) = self.db.fetch_one(
            "SELECT une_personality_json FROM npc_state WHERE entity_id = ?",
            &[&id],
        )? {
            if let Some(p) = npc
                .get("une_personality_json")
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
            {
                data.insert("personality".into(), p);
            }
        }
        Ok(Some(data))
    }

    /// Load one entity for the editor: the `entities` row with `data` set to the
    /// assembled `character_state` (when present) or the stored `data` blob.
    pub fn load(&self, id: &str) -> Result<Option<EditorEntityRecord>, String> {
        let Some(row) = self.db.fetch_one(
            "SELECT id, name, entity_type, location_q, location_r, alive, data \
             FROM entities WHERE id = ?",
            &[&id],
        )?
        else {
            return Ok(None);
        };
        let data = match self.assemble_character_data(id)? {
            Some(state) => state,
            None => row
                .get("data")
                .and_then(Value::as_str)
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .and_then(|v| match v {
                    Value::Object(m) => Some(m),
                    _ => None,
                })
                .unwrap_or_default(),
        };
        Ok(Some(record_from_row(&row, data)))
    }

    /// List entities, optionally filtered by type and/or alive flag, ordered by
    /// name. Summary rows: `data` is empty (the list view doesn't need it).
    pub fn list(
        &self,
        entity_type: Option<&str>,
        alive: Option<bool>,
    ) -> Result<Vec<EditorEntityRecord>, String> {
        let mut sql = String::from(
            "SELECT id, name, entity_type, location_q, location_r, alive FROM entities",
        );
        let mut clauses: Vec<&str> = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        if let Some(t) = entity_type {
            clauses.push("entity_type = ?");
            params.push(Box::new(t.to_string()));
        }
        if let Some(a) = alive {
            clauses.push("alive = ?");
            params.push(Box::new(a as i32));
        }
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY name");
        let refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
        let rows = self.db.fetch_all(&sql, &refs)?;
        Ok(rows
            .iter()
            .map(|row| record_from_row(row, JsonObject::new()))
            .collect())
    }
}

/// Build an [`EditorEntityRecord`] from an `entities` row plus an assembled `data`.
fn record_from_row(row: &Row, data: JsonObject) -> EditorEntityRecord {
    EditorEntityRecord {
        id: str_col(row, "id"),
        entity_type: str_col(row, "entity_type"),
        name: str_col(row, "name"),
        location_q: row.get("location_q").and_then(Value::as_i64).map(|v| v as i32),
        location_r: row.get("location_r").and_then(Value::as_i64).map(|v| v as i32),
        alive: row.get("alive").and_then(Value::as_i64).unwrap_or(1) != 0,
        data,
    }
}

fn str_col(row: &Row, col: &str) -> String {
    row.get(col).and_then(Value::as_str).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::Character;
    use crate::db_schema::SCHEMA_SQL;
    use crate::repositories::entity::EntityRepository;

    fn seeded_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn make_character(id: &str, name: &str) -> Character {
        let mut c = Character::new(name, "warrior");
        c.id = id.into();
        c.hp = 9;
        c.max_hp = 12;
        c.ac = 14;
        c
    }

    #[test]
    fn load_assembles_character_state_into_data() {
        let db = seeded_db();
        EntityRepository::new(&db)
            .create_character(&make_character("pc", "Reaver"))
            .unwrap();

        let repo = EditorEntityRepository::new(&db);
        let rec = repo.load("pc").unwrap().expect("entity loads");
        assert_eq!(rec.id, "pc");
        assert_eq!(rec.name, "Reaver");
        assert!(rec.alive);
        // The derived state was assembled into `data`.
        assert_eq!(rec.data.get("hp").and_then(Value::as_i64), Some(9));
        assert_eq!(rec.data.get("max_hp").and_then(Value::as_i64), Some(12));
        assert_eq!(rec.data.get("ac").and_then(Value::as_i64), Some(14));
        assert!(rec.data.contains_key("attributes"));
        assert!(rec.data.contains_key("character_class"));
    }

    #[test]
    fn load_falls_back_to_data_blob_without_character_state() {
        let db = seeded_db();
        // A plain entity (no character_state) with a stored data blob.
        let data: JsonObject = serde_json::from_value(serde_json::json!({"note": "hi"})).unwrap();
        EntityRepository::new(&db)
            .create_entity("rock", "object", "Rock", 1, 2, &data)
            .unwrap();

        let repo = EditorEntityRepository::new(&db);
        let rec = repo.load("rock").unwrap().expect("entity loads");
        assert_eq!(rec.data.get("note").and_then(Value::as_str), Some("hi"));
        assert!(repo.load("missing").unwrap().is_none());
    }

    #[test]
    fn list_filters_by_type_and_alive() {
        let db = seeded_db();
        let entities = EntityRepository::new(&db);
        entities.create_character(&make_character("pc", "Reaver")).unwrap();
        let blank: JsonObject = JsonObject::new();
        entities.create_entity("npc1", "npc", "Trader", 0, 0, &blank).unwrap();
        entities.create_entity("npc2", "npc", "Guard", 0, 0, &blank).unwrap();

        let repo = EditorEntityRepository::new(&db);
        assert_eq!(repo.list(None, None).unwrap().len(), 3);
        assert_eq!(repo.list(Some("npc"), None).unwrap().len(), 2);
        // Ordered by name: Guard, Trader.
        let npcs = repo.list(Some("npc"), None).unwrap();
        assert_eq!(npcs[0].name, "Guard");
        // Summary rows carry no data.
        assert!(npcs[0].data.is_empty());
        assert_eq!(repo.list(None, Some(true)).unwrap().len(), 3);
    }
}
