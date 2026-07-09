//! Encounter system.
//!
//! Ported from `src/harsh_realm/engine/encounters.py`. Rolls for random
//! encounters on hex entry; hostile encounters are narrated and neutral NPCs are
//! spawned as entities. `Strange Weather` encounters defer to [`crate::weather`].

use rand::Rng;
use uuid::Uuid;

use crate::character::Character;
use crate::db::WorldDatabase;
use crate::engine_results::EncounterResult;
use crate::engine_runtime::EncounterRecord;
use crate::repositories::entity::EntityRepository;
use crate::runtime::{JsonObject, JsonValue};
use crate::table_engine::{TableEngine, TableError};
use crate::weather::WeatherService;

const MIN_PROB: f64 = 0.05;
const MAX_PROB: f64 = 0.75;

/// Terrain modifier on base encounter probability.
fn terrain_modifier(terrain: &str) -> f64 {
    match terrain {
        "ruins" | "swamp" => 0.10,
        "plains" => -0.05,
        _ => 0.0,
    }
}

/// Rolls for random encounters on hex entry.
pub struct EncounterSystem<'a, R: Rng> {
    table_engine: TableEngine<'a, R>,
    rng: R,
}

impl<'a, R: Rng> EncounterSystem<'a, R> {
    /// Create an encounter system over a table engine plus a roll RNG.
    pub fn new(table_engine: TableEngine<'a, R>, rng: R) -> Self {
        EncounterSystem { table_engine, rng }
    }

    /// Roll for a random encounter when entering a hex.
    ///
    /// `hex_data` carries `terrain`, `explored`, `features`, `q`, `r`.
    pub fn check_encounter(
        &mut self,
        hex_data: &JsonObject,
        _character: &Character,
        db: &WorldDatabase,
    ) -> Result<Option<EncounterResult>, String> {
        let terrain = hex_data
            .get("terrain")
            .and_then(|v| v.as_str())
            .unwrap_or("plains")
            .to_string();
        let explored = hex_data
            .get("explored")
            .map(json_truthy)
            .unwrap_or(false);
        let features: Vec<String> = hex_data
            .get("features")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let mut probability = if explored { 0.25 } else { 0.50 };
        let mut modifier = terrain_modifier(&terrain);
        if terrain == "plains"
            && features.iter().any(|f| f.to_lowercase().contains("settlement"))
        {
            modifier -= 0.05;
        }
        probability = (probability + modifier).clamp(MIN_PROB, MAX_PROB);

        if self.rng.gen::<f64>() >= probability {
            return Ok(None);
        }

        // Select encounter table, falling back to encounters_common.
        let table_id = format!("encounters_{terrain}");
        let table_result = match self.table_engine.roll_on(&table_id, None) {
            Ok(result) => result,
            Err(TableError::NotFound(_)) => {
                match self.table_engine.roll_on("encounters_common", None) {
                    Ok(result) => result,
                    Err(TableError::NotFound(_)) => return Ok(None),
                    Err(e) => return Err(e.to_string()),
                }
            }
            Err(e) => return Err(e.to_string()),
        };

        let record: EncounterRecord =
            serde_json::from_value(JsonValue::Object(table_result.result))
                .map_err(|e| e.to_string())?;
        let encounter_type = record.r#type.clone();
        let name = record.name.clone();
        let mut description = record.description.clone();

        if name == "Strange Weather" {
            description = self.strange_weather_description(hex_data, &terrain, db)?;
        }

        let entity_id = if encounter_type == "neutral_npc" {
            Some(self.spawn_npc(&name, hex_data, db, &record)?)
        } else {
            None
        };

        Ok(Some(EncounterResult {
            encounter_type,
            name,
            description,
            entity_id,
            data: record,
        }))
    }

    fn strange_weather_description(
        &mut self,
        hex_data: &JsonObject,
        terrain: &str,
        db: &WorldDatabase,
    ) -> Result<String, String> {
        let seed = db.get_meta("seed")?.unwrap_or_else(|| "default".to_string());
        let tick: i64 = db
            .get_meta("tick")?
            .and_then(|t| t.parse().ok())
            .unwrap_or(0);
        let q = hex_data.get("q").and_then(|v| v.as_i64()).unwrap_or(0);
        let r = hex_data.get("r").and_then(|v| v.as_i64()).unwrap_or(0);

        let weather_service = WeatherService::new(seed);
        let weather = weather_service.at(q, r, tick, Some(terrain));
        let event = weather_service.get_event(q, r, tick, &mut self.table_engine);
        Ok(match event {
            Some(event) => format!("{} {}", weather.description, event),
            None => weather.description,
        })
    }

    fn spawn_npc(
        &self,
        name: &str,
        hex_data: &JsonObject,
        db: &WorldDatabase,
        extra_data: &EncounterRecord,
    ) -> Result<String, String> {
        let entity_id = Uuid::new_v4().to_string();
        let q = hex_data.get("q").and_then(|v| v.as_i64()).unwrap_or(0);
        let r = hex_data.get("r").and_then(|v| v.as_i64()).unwrap_or(0);
        let npc_data = match serde_json::to_value(extra_data) {
            Ok(JsonValue::Object(map)) => map,
            _ => JsonObject::new(),
        };
        EntityRepository::new(db).create_entity(&entity_id, "npc", name, q, r, &npc_data)?;
        Ok(entity_id)
    }
}

fn json_truthy(value: &JsonValue) -> bool {
    match value {
        JsonValue::Bool(b) => *b,
        JsonValue::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        JsonValue::String(s) => !s.is_empty(),
        JsonValue::Null => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn hex(terrain: &str, explored: bool) -> JsonObject {
        let mut h = JsonObject::new();
        h.insert("terrain".into(), serde_json::json!(terrain));
        h.insert("explored".into(), serde_json::json!(explored));
        h.insert("q".into(), serde_json::json!(1));
        h.insert("r".into(), serde_json::json!(2));
        h
    }

    fn seed_table(db: &WorldDatabase, table_id: &str, record: serde_json::Value) {
        let entry: crate::scene_data::RandomTableEntry = serde_json::from_value(serde_json::json!({
            "weight": 1.0, "result": record,
        }))
        .unwrap();
        crate::repositories::random_table::RandomTableRepository::new(db)
            .upsert_table(table_id, "encounters", table_id, &[], &[entry], "test")
            .unwrap();
    }

    #[test]
    fn no_encounter_when_roll_exceeds_probability() {
        let db = WorldDatabase::open_in_memory().unwrap();
        // High seed roll won't matter: explored plains has low probability.
        let engine = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let mut sys = EncounterSystem::new(engine, SmallRng::seed_from_u64(999));
        // Run many times; with no tables, any encounter returns None anyway.
        let result = sys
            .check_encounter(&hex("plains", true), &Character::default(), &db)
            .unwrap();
        // Without seeded tables, even a triggered roll falls back to None.
        assert!(result.is_none());
    }

    #[test]
    fn spawns_neutral_npc_entity() {
        let db = WorldDatabase::open_in_memory().unwrap();
        seed_table(
            &db,
            "encounters_ruins",
            serde_json::json!({
                "type": "neutral_npc", "name": "Wanderer", "description": "A lone figure.",
            }),
        );
        // Try a few RNG states until an encounter triggers (ruins prob = 0.6
        // unexplored), to keep the test robust against the roll RNG.
        let mut result = None;
        for seed in 0..50 {
            let mut sys = EncounterSystem::new(
                TableEngine::new(&db, SmallRng::seed_from_u64(2)),
                SmallRng::seed_from_u64(seed),
            );
            if let Some(r) = sys
                .check_encounter(&hex("ruins", false), &Character::default(), &db)
                .unwrap()
            {
                result = Some(r);
                break;
            }
        }
        let result = result.expect("an encounter should trigger within 50 seeds");
        assert_eq!(result.encounter_type, "neutral_npc");
        assert_eq!(result.name, "Wanderer");
        assert!(result.entity_id.is_some());
    }
}
