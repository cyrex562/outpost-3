//! Typed persistence helpers for character and NPC entity state.
//!
//! Ported from `src/harsh_realm/gm/entity_state_repository.py`.

use crate::character::Character;
use crate::db::{Row, WorldDatabase};
use crate::entity_state::{CharacterState, NpcState};
use crate::item::SaveBonusProfile;
use crate::runtime::{JsonObject, JsonValue, SceneNpcRecord};

/// Reads and writes typed persistence rows for character and NPC aggregates.
pub struct EntityStateRepository<'a> {
    db: &'a WorldDatabase,
}

fn rs(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn ri(row: &Row, key: &str) -> i64 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}
fn ros(row: &Row, key: &str) -> Option<String> {
    row.get(key).and_then(|v| v.as_str()).map(str::to_string)
}
fn rmap(row: &Row, key: &str) -> std::collections::BTreeMap<String, i32> {
    serde_json::from_str(row.get(key).and_then(|v| v.as_str()).unwrap_or("{}")).unwrap_or_default()
}

impl<'a> EntityStateRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        EntityStateRepository { db }
    }

    /// Insert or update typed character state for an entity.
    pub fn upsert_character(&self, character: &Character) -> Result<(), String> {
        let s = CharacterState::from_character(character);
        let attrs = serde_json::to_string(&s.attributes).map_err(|e| e.to_string())?;
        let mods = serde_json::to_string(&s.attr_mods).map_err(|e| e.to_string())?;
        let skills = serde_json::to_string(&s.skills).map_err(|e| e.to_string())?;
        let saves = serde_json::to_string(&s.save_bonuses).map_err(|e| e.to_string())?;
        let equip = serde_json::to_string(&s.equipment).map_err(|e| e.to_string())?;
        let abilities = serde_json::to_string(&s.class_abilities).map_err(|e| e.to_string())?;
        self.db.execute(
            "INSERT INTO character_state (entity_id, character_class, level, xp, xp_next, hp, max_hp, ac, \
             attack_bonus, physical_save, evasion_save, mental_save, position_q, position_r, attributes_json, \
             attr_mods_json, skills_json, save_bonuses_json, equipment_json, class_abilities_json, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
             ON CONFLICT(entity_id) DO UPDATE SET character_class=excluded.character_class, level=excluded.level, \
             xp=excluded.xp, xp_next=excluded.xp_next, hp=excluded.hp, max_hp=excluded.max_hp, ac=excluded.ac, \
             attack_bonus=excluded.attack_bonus, physical_save=excluded.physical_save, evasion_save=excluded.evasion_save, \
             mental_save=excluded.mental_save, position_q=excluded.position_q, position_r=excluded.position_r, \
             attributes_json=excluded.attributes_json, attr_mods_json=excluded.attr_mods_json, skills_json=excluded.skills_json, \
             save_bonuses_json=excluded.save_bonuses_json, equipment_json=excluded.equipment_json, \
             class_abilities_json=excluded.class_abilities_json, updated_at=CURRENT_TIMESTAMP",
            &[
                &s.entity_id, &s.character_class, &(s.level as i64), &(s.xp as i64), &(s.xp_next as i64),
                &(s.hp as i64), &(s.max_hp as i64), &(s.ac as i64), &(s.attack_bonus as i64),
                &(s.physical_save as i64), &(s.evasion_save as i64), &(s.mental_save as i64),
                &(s.position_q as i64), &(s.position_r as i64), &attrs, &mods, &skills, &saves, &equip, &abilities,
            ],
        )?;
        Ok(())
    }

    /// Load the first living character from typed persistence.
    pub fn load_first_character(&self) -> Result<Option<Character>, String> {
        let row = self.db.fetch_one(
            "SELECT e.name, cs.* FROM character_state cs JOIN entities e ON e.id = cs.entity_id \
             WHERE e.entity_type = 'character' AND e.alive = 1 LIMIT 1",
            &[],
        )?;
        Ok(row.map(|r| character_from_row(&r)))
    }

    /// Load a character by entity id.
    pub fn load_character(&self, entity_id: &str) -> Result<Option<Character>, String> {
        let row = self.db.fetch_one(
            "SELECT e.name, cs.* FROM character_state cs JOIN entities e ON e.id = cs.entity_id \
             WHERE cs.entity_id = ?",
            &[&entity_id],
        )?;
        Ok(row.map(|r| character_from_row(&r)))
    }

    /// Update only the typed character position mirror.
    pub fn update_character_position(&self, entity_id: &str, q: i64, r: i64) -> Result<(), String> {
        self.db.execute(
            "UPDATE character_state SET position_q = ?, position_r = ?, updated_at = CURRENT_TIMESTAMP WHERE entity_id = ?",
            &[&q, &r, &entity_id],
        )?;
        Ok(())
    }

    /// Insert or update typed NPC state for an entity.
    pub fn upsert_npc(&self, entity_id: &str, data: &JsonObject) -> Result<(), String> {
        let mut payload = data.clone();
        payload.insert("entity_id".into(), JsonValue::from(entity_id));
        let state: NpcState = serde_json::from_value(JsonValue::Object(payload))
            .map_err(|e| e.to_string())?;
        let traits = serde_json::to_string(&state.personality_traits).map_err(|e| e.to_string())?;
        let une_json = state
            .une_personality
            .as_ref()
            .map(|u| serde_json::to_string(u).unwrap_or_default());
        self.db.execute(
            "INSERT INTO npc_state (entity_id, occupation, personality_traits_json, motivation, appearance, \
             greeting, faction_id, disposition, une_personality_json, building_id, establishment_type, \
             establishment_name, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
             ON CONFLICT(entity_id) DO UPDATE SET occupation=excluded.occupation, \
             personality_traits_json=excluded.personality_traits_json, motivation=excluded.motivation, \
             appearance=excluded.appearance, greeting=excluded.greeting, faction_id=excluded.faction_id, \
             disposition=excluded.disposition, une_personality_json=excluded.une_personality_json, \
             building_id=excluded.building_id, establishment_type=excluded.establishment_type, \
             establishment_name=excluded.establishment_name, updated_at=CURRENT_TIMESTAMP",
            &[
                &state.entity_id, &state.occupation, &traits, &state.motivation, &state.appearance,
                &state.greeting, &state.faction_id, &(state.disposition as i64), &une_json,
                &state.building_id, &state.establishment_type, &state.establishment_name,
            ],
        )?;
        Ok(())
    }

    /// Load one NPC as a typed scene record.
    pub fn load_npc_record(&self, entity_id: &str) -> Result<Option<SceneNpcRecord>, String> {
        let row = self.db.fetch_one(
            "SELECT e.id, e.name, ns.* FROM npc_state ns JOIN entities e ON e.id = ns.entity_id WHERE ns.entity_id = ?",
            &[&entity_id],
        )?;
        Ok(row.map(|r| npc_record_from_row(&r)))
    }

    /// Update an NPC disposition.
    pub fn update_npc_disposition(&self, entity_id: &str, disposition: i64) -> Result<(), String> {
        self.db.execute(
            "UPDATE npc_state SET disposition = ?, updated_at = CURRENT_TIMESTAMP WHERE entity_id = ?",
            &[&disposition, &entity_id],
        )?;
        Ok(())
    }

    /// Load living NPCs at a location as typed scene records.
    pub fn list_npcs_at_location(&self, q: i64, r: i64) -> Result<Vec<SceneNpcRecord>, String> {
        let rows = self.db.fetch_all(
            "SELECT e.id, e.name, ns.* FROM npc_state ns JOIN entities e ON e.id = ns.entity_id \
             WHERE e.entity_type = 'npc' AND e.location_q = ? AND e.location_r = ? AND e.alive = 1 ORDER BY e.name",
            &[&q, &r],
        )?;
        Ok(rows.iter().map(npc_record_from_row).collect())
    }

    /// Remove typed state rows when an entity is deleted or retyped.
    pub fn delete_typed_state(&self, entity_id: &str, entity_type: &str) -> Result<(), String> {
        match entity_type {
            "character" => {
                self.db.execute("DELETE FROM character_state WHERE entity_id = ?", &[&entity_id])?;
            }
            "npc" => {
                self.db.execute("DELETE FROM npc_state WHERE entity_id = ?", &[&entity_id])?;
            }
            _ => {}
        }
        Ok(())
    }
}

fn character_from_row(row: &Row) -> Character {
    let entity_id = if row.contains_key("entity_id") {
        rs(row, "entity_id")
    } else {
        rs(row, "id")
    };
    let save_bonuses: SaveBonusProfile =
        serde_json::from_str(row.get("save_bonuses_json").and_then(|v| v.as_str()).unwrap_or("{}"))
            .unwrap_or_default();
    let equipment: Vec<JsonObject> =
        serde_json::from_str(row.get("equipment_json").and_then(|v| v.as_str()).unwrap_or("[]"))
            .unwrap_or_default();
    let class_abilities: JsonObject =
        serde_json::from_str(row.get("class_abilities_json").and_then(|v| v.as_str()).unwrap_or("{}"))
            .unwrap_or_default();
    let state = CharacterState {
        entity_id,
        name: rs(row, "name"),
        character_class: rs(row, "character_class"),
        level: ri(row, "level") as i32,
        xp: ri(row, "xp") as i32,
        xp_next: ri(row, "xp_next") as i32,
        attributes: rmap(row, "attributes_json"),
        attr_mods: rmap(row, "attr_mods_json"),
        skills: rmap(row, "skills_json"),
        hp: ri(row, "hp") as i32,
        max_hp: ri(row, "max_hp") as i32,
        ac: ri(row, "ac") as i32,
        attack_bonus: ri(row, "attack_bonus") as i32,
        physical_save: ri(row, "physical_save") as i32,
        evasion_save: ri(row, "evasion_save") as i32,
        mental_save: ri(row, "mental_save") as i32,
        save_bonuses,
        equipment,
        class_abilities,
        position_q: ri(row, "position_q") as i32,
        position_r: ri(row, "position_r") as i32,
    };
    state.to_character()
}

fn npc_record_from_row(row: &Row) -> SceneNpcRecord {
    let une_personality: Option<JsonObject> = row
        .get("une_personality_json")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(s).ok());
    let personality_traits: Vec<String> =
        serde_json::from_str(row.get("personality_traits_json").and_then(|v| v.as_str()).unwrap_or("[]"))
            .unwrap_or_default();
    SceneNpcRecord {
        entity_id: ros(row, "id"),
        name: rs(row, "name"),
        occupation: rs(row, "occupation"),
        personality_traits,
        motivation: rs(row, "motivation"),
        appearance: rs(row, "appearance"),
        greeting: rs(row, "greeting"),
        faction_id: ros(row, "faction_id"),
        disposition: JsonValue::from(ri(row, "disposition")),
        une_personality,
        building_id: ros(row, "building_id"),
        establishment_type: ros(row, "establishment_type"),
        establishment_name: ros(row, "establishment_name"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_entity(db: &WorldDatabase, id: &str, etype: &str, name: &str, q: i64, r: i64) {
        db.execute(
            "INSERT INTO entities (id, entity_type, name, location_q, location_r, alive, data, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, 1, '{}', 'now', 'now')",
            &[&id, &etype, &name, &q, &r],
        )
        .unwrap();
    }

    #[test]
    fn character_round_trip() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = EntityStateRepository::new(&db);
        let mut c = Character::new("Hero", "warrior");
        c.id = "hero".into();
        c.level = 4;
        c.attr_mods.insert("str".into(), 2);
        seed_entity(&db, "hero", "character", "Hero", 0, 0);
        repo.upsert_character(&c).unwrap();
        let loaded = repo.load_character("hero").unwrap().unwrap();
        assert_eq!(loaded.level, 4);
        assert_eq!(loaded.attr_mods.get("str"), Some(&2));
        assert_eq!(loaded.name, "Hero");
        let first = repo.load_first_character().unwrap().unwrap();
        assert_eq!(first.id, "hero");
    }

    #[test]
    fn npc_round_trip_and_disposition() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let repo = EntityStateRepository::new(&db);
        seed_entity(&db, "n1", "npc", "Smith", 2, 3);
        let data: JsonObject =
            serde_json::from_str(r#"{"occupation":"smith","disposition":"friendly","motivation":"coin"}"#).unwrap();
        repo.upsert_npc("n1", &data).unwrap();
        let rec = repo.load_npc_record("n1").unwrap().unwrap();
        assert_eq!(rec.occupation, "smith");
        assert_eq!(rec.disposition, JsonValue::from(2)); // "friendly" normalized
        repo.update_npc_disposition("n1", -1).unwrap();
        let at = repo.list_npcs_at_location(2, 3).unwrap();
        assert_eq!(at.len(), 1);
        assert_eq!(at[0].disposition, JsonValue::from(-1));
    }
}
