//! Repository helpers for entity JSON persistence used by gameplay scenes.
//!
//! Ported from `src/harsh_realm/gm/entity_repository.py`.

use serde_json::Value;

use crate::character::Character;
use crate::db::{Row, WorldDatabase};
use crate::payloads::transport::EntityRecord;
use crate::repositories::entity_state::EntityStateRepository;
use crate::repositories::resources::{ResourceInstance, ResourceRepository, GOLD_RESOURCE_ID, HP_RESOURCE_ID};
use crate::runtime::{JsonObject, JsonValue, SceneNpcRecord};

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn parse_entity_data(raw: Option<&str>) -> JsonObject {
    serde_json::from_str(raw.unwrap_or("{}")).unwrap_or_default()
}

fn rs(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn ri_opt(row: &Row, key: &str) -> Option<i32> {
    row.get(key).and_then(|v| v.as_i64()).map(|x| x as i32)
}

/// Centralizes common character and entity JSON access patterns.
pub struct EntityRepository<'a> {
    db: &'a WorldDatabase,
    state_repo: EntityStateRepository<'a>,
}

impl<'a> EntityRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        EntityRepository {
            db,
            state_repo: EntityStateRepository::new(db),
        }
    }

    fn character_from_entity_row(row: &Row) -> Option<Character> {
        let mut payload = parse_entity_data(row.get("data").and_then(|v| v.as_str()));
        payload.entry("id").or_insert_with(|| JsonValue::from(rs(row, "id")));
        payload.entry("name").or_insert_with(|| JsonValue::from(rs(row, "name")));
        payload
            .entry("position_q")
            .or_insert_with(|| JsonValue::from(row.get("location_q").and_then(|v| v.as_i64()).unwrap_or(0)));
        payload
            .entry("position_r")
            .or_insert_with(|| JsonValue::from(row.get("location_r").and_then(|v| v.as_i64()).unwrap_or(0)));
        serde_json::from_value(Value::Object(payload)).ok()
    }

    fn entity_record_from_row(row: &Row) -> EntityRecord {
        EntityRecord {
            id: rs(row, "id"),
            name: rs(row, "name"),
            location_q: ri_opt(row, "location_q"),
            location_r: ri_opt(row, "location_r"),
            entity_data: parse_entity_data(row.get("data").and_then(|v| v.as_str())),
            entity_type: row.get("entity_type").and_then(|v| v.as_str()).map(str::to_string),
            alive: row.get("alive").cloned(),
        }
    }

    /// Load the first living character, preferring a valid legacy entity payload.
    pub fn load_first_character(&self) -> Result<Option<Character>, String> {
        let row = self.db.fetch_one(
            "SELECT id, name, location_q, location_r, data FROM entities WHERE entity_type = 'character' AND alive = 1 LIMIT 1",
            &[],
        )?;
        if let Some(row) = row {
            if let Some(c) = Self::character_from_entity_row(&row) {
                return Ok(Some(c));
            }
        }
        self.state_repo.load_first_character()
    }

    /// Whether a living character exists in the world.
    pub fn has_living_character(&self) -> Result<bool, String> {
        Ok(self
            .db
            .fetch_one("SELECT id FROM entities WHERE entity_type = 'character' AND alive = 1 LIMIT 1", &[])?
            .is_some())
    }

    /// Load a single entity row with parsed JSON payload.
    pub fn load_entity_record(&self, entity_id: &str) -> Result<Option<EntityRecord>, String> {
        let row = self.db.fetch_one(
            "SELECT id, entity_type, name, location_q, location_r, alive, data FROM entities WHERE id = ?",
            &[&entity_id],
        )?;
        Ok(row.as_ref().map(Self::entity_record_from_row))
    }

    /// Return entity rows at a location with parsed JSON payloads.
    pub fn list_entity_records_at_location(
        &self,
        entity_type: &str,
        q: i64,
        r: i64,
        alive_only: bool,
    ) -> Result<Vec<EntityRecord>, String> {
        let sql = if alive_only {
            "SELECT id, entity_type, name, location_q, location_r, alive, data FROM entities WHERE entity_type = ? AND location_q = ? AND location_r = ? AND alive = 1"
        } else {
            "SELECT id, entity_type, name, location_q, location_r, alive, data FROM entities WHERE entity_type = ? AND location_q = ? AND location_r = ?"
        };
        let rows = self.db.fetch_all(sql, &[&entity_type, &q, &r])?;
        Ok(rows.iter().map(Self::entity_record_from_row).collect())
    }

    /// Persist a character JSON snapshot and optional mirrored columns/resources.
    pub fn save_character(
        &self,
        character: &Character,
        sync_position: bool,
        sync_hp: bool,
        sync_gold: bool,
    ) -> Result<(), String> {
        let data_json = serde_json::to_string(character).map_err(|e| e.to_string())?;
        let now = now_iso();
        if sync_position {
            self.db.execute(
                "UPDATE entities SET data = ?, updated_at = ?, location_q = ?, location_r = ? WHERE id = ?",
                &[&data_json, &now, &(character.position_q as i64), &(character.position_r as i64), &character.id],
            )?;
        } else {
            self.db.execute(
                "UPDATE entities SET data = ?, updated_at = ? WHERE id = ?",
                &[&data_json, &now, &character.id],
            )?;
        }
        self.state_repo.upsert_character(character)?;
        let resources = ResourceRepository::new(self.db);
        if sync_hp {
            resources.set(&ResourceInstance {
                entity_id: character.id.clone(),
                resource_id: HP_RESOURCE_ID.to_string(),
                current: character.hp,
                max: Some(character.max_hp),
                last_regen_tick: 0,
            })?;
        }
        if sync_gold {
            let gold = character
                .class_abilities
                .get("gold")
                .map(coerce_gold)
                .unwrap_or(0)
                .max(0);
            resources.set(&ResourceInstance {
                entity_id: character.id.clone(),
                resource_id: GOLD_RESOURCE_ID.to_string(),
                current: gold,
                max: None,
                last_regen_tick: 0,
            })?;
        }
        Ok(())
    }

    /// Return living NPCs at a location as typed scene records.
    pub fn list_npcs_at_location(&self, q: i64, r: i64) -> Result<Vec<SceneNpcRecord>, String> {
        let records = self.state_repo.list_npcs_at_location(q, r)?;
        if !records.is_empty() {
            return Ok(records);
        }
        let mut result = Vec::new();
        for row in self.list_entity_records_at_location("npc", q, r, true)? {
            let mut payload = row.entity_data.clone();
            payload.insert("entity_id".into(), JsonValue::from(row.id.clone()));
            payload.insert("name".into(), JsonValue::from(row.name.clone()));
            if let Ok(rec) = serde_json::from_value(Value::Object(payload)) {
                result.push(rec);
            }
        }
        Ok(result)
    }

    /// Return living NPC rows at a location with parsed JSON payloads.
    pub fn list_npc_records_at_location(&self, q: i64, r: i64) -> Result<Vec<EntityRecord>, String> {
        self.list_entity_records_at_location("npc", q, r, true)
    }

    /// Load the first character plus its typed entity record.
    pub fn load_character_with_record(&self) -> Result<(Option<Character>, Option<EntityRecord>), String> {
        let row = self.db.fetch_one(
            "SELECT id, name, location_q, location_r, data FROM entities WHERE entity_type = 'character' LIMIT 1",
            &[],
        )?;
        if let Some(row) = row {
            if let Some(c) = Self::character_from_entity_row(&row) {
                let rec = self.load_entity_record(&c.id)?;
                return Ok((Some(c), rec));
            }
        }
        match self.state_repo.load_first_character()? {
            Some(c) => {
                let rec = self.load_entity_record(&c.id)?;
                Ok((Some(c), rec))
            }
            None => Ok((None, None)),
        }
    }

    /// Persist an entity's location and optional mirrored JSON position.
    pub fn update_entity_position(
        &self,
        entity_id: &str,
        q: i64,
        r: i64,
        entity_data: Option<&JsonObject>,
    ) -> Result<(), String> {
        let now = now_iso();
        match entity_data {
            None => {
                self.db.execute(
                    "UPDATE entities SET location_q = ?, location_r = ?, updated_at = ? WHERE id = ?",
                    &[&q, &r, &now, &entity_id],
                )?;
                let row = self.db.fetch_one("SELECT entity_type FROM entities WHERE id = ?", &[&entity_id])?;
                if row.map(|r| rs(&r, "entity_type")) == Some("character".to_string()) {
                    self.state_repo.update_character_position(entity_id, q, r)?;
                }
                Ok(())
            }
            Some(data) => {
                let json = serde_json::to_string(data).map_err(|e| e.to_string())?;
                self.db.execute(
                    "UPDATE entities SET location_q = ?, location_r = ?, data = ?, updated_at = ? WHERE id = ?",
                    &[&q, &r, &json, &now, &entity_id],
                )?;
                self.sync_typed_state_from_payload(entity_id, data, None, None, Some(q), Some(r))
            }
        }
    }

    /// Insert a new entity row with JSON payload.
    pub fn create_entity(
        &self,
        entity_id: &str,
        entity_type: &str,
        name: &str,
        q: i64,
        r: i64,
        entity_data: &JsonObject,
    ) -> Result<(), String> {
        let now = now_iso();
        let json = serde_json::to_string(entity_data).map_err(|e| e.to_string())?;
        self.db.execute(
            "INSERT INTO entities (id, entity_type, name, location_q, location_r, alive, data, created_at, updated_at) VALUES (?, ?, ?, ?, ?, 1, ?, ?, ?)",
            &[&entity_id, &entity_type, &name, &q, &r, &json, &now, &now],
        )?;
        self.sync_typed_state_from_payload(entity_id, entity_data, Some(entity_type), Some(name), Some(q), Some(r))
    }

    /// Insert a new character row with mirrored location columns.
    pub fn create_character(&self, character: &Character) -> Result<(), String> {
        let data: JsonObject = serde_json::to_value(character)
            .ok()
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        self.create_entity(
            &character.id,
            "character",
            &character.name,
            character.position_q as i64,
            character.position_r as i64,
            &data,
        )?;
        self.save_character(character, false, true, true)
    }

    /// Persist character state to both legacy JSON and typed tables.
    pub fn save_character_state(&self, character: &mut Character) -> Result<(), String> {
        character.equipment = Self::stack_items(&character.equipment);
        self.save_character(character, true, true, true)
    }

    /// Merge identical items in the equipment list using quantity counters.
    pub fn stack_items(equipment: &[JsonObject]) -> Vec<JsonObject> {
        let mut stacked: Vec<InventoryStack> = Vec::new();
        for raw in equipment {
            let name = raw.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
            let kind = raw.get("type").and_then(|v| v.as_str()).unwrap_or("misc").to_string();
            let quantity = raw.get("quantity").and_then(|v| v.as_i64()).unwrap_or(1);
            if let Some(existing) = stacked.iter_mut().find(|s| s.name == name && s.kind == kind) {
                existing.quantity += quantity;
            } else {
                stacked.push(InventoryStack { name, kind, quantity, raw: raw.clone() });
            }
        }
        stacked
            .into_iter()
            .map(|mut s| {
                s.raw.insert("quantity".into(), JsonValue::from(s.quantity));
                s.raw
            })
            .collect()
    }

    /// Persist an entity JSON payload and sync typed mirrors when applicable.
    pub fn save_entity_data(&self, entity_id: &str, entity_data: &JsonObject) -> Result<(), String> {
        let json = serde_json::to_string(entity_data).map_err(|e| e.to_string())?;
        let now = now_iso();
        self.db.execute(
            "UPDATE entities SET data = ?, updated_at = ? WHERE id = ?",
            &[&json, &now, &entity_id],
        )?;
        self.sync_typed_state_from_payload(entity_id, entity_data, None, None, None, None)
    }

    /// Load one NPC from typed persistence.
    pub fn load_npc_record(&self, entity_id: &str) -> Result<Option<SceneNpcRecord>, String> {
        self.state_repo.load_npc_record(entity_id)
    }

    /// Persist NPC disposition changes to typed and legacy storage.
    pub fn update_npc_disposition(&self, entity_id: &str, disposition: i64) -> Result<(), String> {
        if let Some(row) = self.db.fetch_one("SELECT data FROM entities WHERE id = ?", &[&entity_id])? {
            let mut payload = parse_entity_data(row.get("data").and_then(|v| v.as_str()));
            payload.insert("disposition".into(), JsonValue::from(disposition));
            let json = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
            let now = now_iso();
            self.db.execute(
                "UPDATE entities SET data = ?, updated_at = ? WHERE id = ?",
                &[&json, &now, &entity_id],
            )?;
        }
        self.state_repo.update_npc_disposition(entity_id, disposition)
    }

    #[allow(clippy::too_many_arguments)]
    fn sync_typed_state_from_payload(
        &self,
        entity_id: &str,
        data: &JsonObject,
        entity_type: Option<&str>,
        name: Option<&str>,
        q: Option<i64>,
        r: Option<i64>,
    ) -> Result<(), String> {
        let (resolved_type, resolved_name, resolved_q, resolved_r) =
            if entity_type.is_none() || name.is_none() || q.is_none() || r.is_none() {
                let Some(row) = self.db.fetch_one(
                    "SELECT entity_type, name, location_q, location_r FROM entities WHERE id = ?",
                    &[&entity_id],
                )? else {
                    return Ok(());
                };
                (
                    entity_type.map(str::to_string).unwrap_or_else(|| rs(&row, "entity_type")),
                    name.map(str::to_string).unwrap_or_else(|| rs(&row, "name")),
                    q.unwrap_or_else(|| row.get("location_q").and_then(|v| v.as_i64()).unwrap_or(0)),
                    r.unwrap_or_else(|| row.get("location_r").and_then(|v| v.as_i64()).unwrap_or(0)),
                )
            } else {
                (entity_type.unwrap().to_string(), name.unwrap().to_string(), q.unwrap(), r.unwrap())
            };

        if resolved_type == "character" {
            let mut payload = data.clone();
            payload.entry("id").or_insert_with(|| JsonValue::from(entity_id));
            payload.entry("name").or_insert_with(|| JsonValue::from(resolved_name));
            payload.insert("position_q".into(), JsonValue::from(resolved_q));
            payload.insert("position_r".into(), JsonValue::from(resolved_r));
            if let Ok(character) = serde_json::from_value::<Character>(Value::Object(payload)) {
                self.state_repo.upsert_character(&character)?;
            }
        } else if resolved_type == "npc" {
            self.state_repo.upsert_npc(entity_id, data)?;
        }
        Ok(())
    }
}

struct InventoryStack {
    name: String,
    kind: String,
    quantity: i64,
    raw: JsonObject,
}

fn coerce_gold(value: &JsonValue) -> i32 {
    match value {
        JsonValue::Number(n) => n.as_i64().map(|x| x as i32).or_else(|| n.as_f64().map(|f| f as i32)).unwrap_or(0),
        JsonValue::String(s) => s.parse::<f64>().ok().map(|f| f as i32).unwrap_or(0),
        JsonValue::Bool(b) => *b as i32,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_load_character() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = EntityRepository::new(&db);
        let mut c = Character::new("Hero", "warrior");
        c.id = "hero".into();
        c.hp = 8;
        c.max_hp = 10;
        c.class_abilities.insert("gold".into(), JsonValue::from(50));
        repo.create_character(&c).unwrap();

        assert!(repo.has_living_character().unwrap());
        let loaded = repo.load_first_character().unwrap().unwrap();
        assert_eq!(loaded.id, "hero");
        assert_eq!(loaded.hp, 8);

        // HP + gold mirrored into entity_resources
        let resources = ResourceRepository::new(&db);
        assert_eq!(resources.get("hero", HP_RESOURCE_ID).unwrap().unwrap().current, 8);
        assert_eq!(resources.get("hero", GOLD_RESOURCE_ID).unwrap().unwrap().current, 50);
    }

    #[test]
    fn stack_items_merges_quantities() {
        let eq: Vec<JsonObject> = vec![
            serde_json::from_str(r#"{"name":"Arrow","type":"ammo","quantity":5}"#).unwrap(),
            serde_json::from_str(r#"{"name":"Arrow","type":"ammo","quantity":3}"#).unwrap(),
            serde_json::from_str(r#"{"name":"Sword","type":"weapon"}"#).unwrap(),
        ];
        let stacked = EntityRepository::stack_items(&eq);
        assert_eq!(stacked.len(), 2);
        let arrow = stacked.iter().find(|i| i["name"] == JsonValue::from("Arrow")).unwrap();
        assert_eq!(arrow["quantity"], JsonValue::from(8));
    }

    #[test]
    fn update_position_and_npc_disposition() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = EntityRepository::new(&db);
        let mut npc_data = JsonObject::new();
        npc_data.insert("occupation".into(), JsonValue::from("smith"));
        repo.create_entity("n1", "npc", "Smith", 1, 1, &npc_data).unwrap();
        repo.update_entity_position("n1", 2, 2, None).unwrap();
        let rec = repo.load_entity_record("n1").unwrap().unwrap();
        assert_eq!(rec.location_q, Some(2));
        repo.update_npc_disposition("n1", -2).unwrap();
        assert_eq!(repo.load_npc_record("n1").unwrap().unwrap().disposition, JsonValue::from(-2));
    }
}
