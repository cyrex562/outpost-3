//! Discovery system.
//!
//! Ported from `src/harsh_realm/engine/discovery.py`. Handles the `search`
//! command: discovery probability, terrain-specific tables with a common
//! fallback, gated skill checks, and a per-hex search cooldown persisted via
//! `last_searched_tick` in the cell data.

use rand::Rng;

use crate::character::Character;
use crate::db::WorldDatabase;
use crate::engine_results::{DiscoveryResult, DiscoverySkillCheck};
use crate::engine_runtime::DiscoveryFindRecord;
use crate::repositories::discovery::DiscoveryRepository;
use crate::runtime::{JsonObject, JsonValue};
use crate::table_engine::{TableEngine, TableError};

/// Minimum ticks between searches of the same hex.
const SEARCH_COOLDOWN: i64 = 100;

const NO_FIND_MESSAGES: [&str; 4] = [
    "You search carefully but find nothing of note.",
    "The area yields nothing useful.",
    "Whatever was here, it's long gone.",
    "You turn up nothing.",
];

/// Terrain -> (skill, attribute) for discovery skill checks.
fn terrain_skill(terrain: &str) -> (&'static str, &'static str) {
    match terrain {
        "forest" | "plains" | "hills" | "swamp" | "desert" | "wasteland" => ("survive", "wis"),
        "ruins" | "settlement" | "dungeon" => ("notice", "int"),
        _ => ("survive", "wis"),
    }
}

/// Manages hex search mechanics.
pub struct DiscoverySystem<'a, R: Rng> {
    table_engine: TableEngine<'a, R>,
    rng: R,
}

impl<'a, R: Rng> DiscoverySystem<'a, R> {
    /// Create a discovery system over a table engine plus a roll RNG.
    pub fn new(table_engine: TableEngine<'a, R>, rng: R) -> Self {
        DiscoverySystem { table_engine, rng }
    }

    /// Search the current hex for discoveries.
    pub fn search_hex(
        &mut self,
        hex_data: &JsonObject,
        character: &Character,
        db: &WorldDatabase,
        tick: i64,
        persist: bool,
    ) -> Result<DiscoveryResult, String> {
        let q = hex_data.get("q").and_then(|v| v.as_i64()).unwrap_or(0);
        let r = hex_data.get("r").and_then(|v| v.as_i64()).unwrap_or(0);
        let terrain = hex_data
            .get("terrain")
            .and_then(|v| v.as_str())
            .unwrap_or("plains")
            .to_string();
        let explored = hex_data
            .get("explored")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let repository = DiscoveryRepository::new(db);

        let fallback = coerce_data(hex_data.get("data"));
        let mut cell_data = repository.load_cell_data(q, r, fallback)?;

        // Cooldown check.
        if let Some(last) = cell_data.get("last_searched_tick").and_then(|v| v.as_i64()) {
            if tick - last < SEARCH_COOLDOWN {
                return Ok(DiscoveryResult {
                    found: false,
                    category: None,
                    result: None,
                    message: "You've already searched this area recently. Give it some time."
                        .to_string(),
                    skill_check: None,
                    discovered_items: Vec::new(),
                    updated_data: None,
                    updated_features: None,
                });
            }
        }

        let probability = if explored { 0.30 } else { 0.50 };
        let found_something = self.rng.gen::<f64>() < probability;

        cell_data.insert("last_searched_tick".into(), JsonValue::from(tick));
        if persist {
            repository.save_cell_data(q, r, &cell_data)?;
        }

        if !found_something {
            let idx = self.rng.gen_range(0..NO_FIND_MESSAGES.len());
            return Ok(DiscoveryResult {
                found: false,
                category: None,
                result: None,
                message: NO_FIND_MESSAGES[idx].to_string(),
                skill_check: None,
                discovered_items: Vec::new(),
                updated_data: Some(cell_data),
                updated_features: None,
            });
        }

        // Roll on the terrain-specific table, falling back to discoveries_common.
        let table_id = format!("discoveries_{terrain}");
        let table_result = match self.table_engine.roll_on(&table_id, None) {
            Ok(result) => result,
            Err(TableError::NotFound(_)) => {
                match self.table_engine.roll_on("discoveries_common", None) {
                    Ok(result) => result,
                    Err(TableError::NotFound(_)) => {
                        return Ok(DiscoveryResult {
                            found: false,
                            category: None,
                            result: None,
                            message: "You search the area but find nothing of note.".to_string(),
                            skill_check: None,
                            discovered_items: Vec::new(),
                            updated_data: Some(cell_data),
                            updated_features: None,
                        });
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Err(e) => return Err(e.to_string()),
        };

        let result_type = table_result.result_type.clone();
        let record: DiscoveryFindRecord =
            serde_json::from_value(JsonValue::Object(table_result.result))
                .map_err(|e| e.to_string())?;
        let category: String = if record.r#type.is_empty() {
            result_type
        } else {
            record.r#type.clone()
        };
        let min_difficulty = record.min_difficulty;

        // Skill check if needed.
        let mut skill_check: Option<DiscoverySkillCheck> = None;
        if min_difficulty > 0 {
            let (skill_name, attribute) = terrain_skill(&terrain);
            let check = self.perform_skill_check(character, skill_name, attribute, min_difficulty);
            if !check.success {
                let message = format!(
                    "You sense something here but can't quite find it. ({} check: {} + {} = {} vs DC {} — Failure)",
                    capitalize(skill_name),
                    check.roll,
                    check.modifier,
                    check.total,
                    min_difficulty
                );
                return Ok(DiscoveryResult {
                    found: true,
                    category: Some(category),
                    result: Some(record),
                    message,
                    skill_check: Some(check),
                    discovered_items: Vec::new(),
                    updated_data: Some(cell_data),
                    updated_features: None,
                });
            }
            skill_check = Some(check);
        }

        let description = record.description.clone();
        let item_name = record.name.clone();
        let mut discovered_items: Vec<JsonObject> = Vec::new();
        let mut updated_features: Option<Vec<String>> = None;

        let mut message = match category.as_str() {
            "environmental" => {
                let mut features: Vec<String> = hex_data
                    .get("features")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let feature_text = if !description.is_empty() {
                    description.clone()
                } else {
                    item_name.clone()
                };
                if !feature_text.is_empty() && !features.contains(&feature_text) {
                    features.push(feature_text);
                    if persist {
                        repository.save_cell_features(q, r, &features)?;
                    }
                }
                updated_features = Some(features);
                if description.is_empty() {
                    "You notice something interesting in the environment.".to_string()
                } else {
                    description.clone()
                }
            }
            "item" | "relic" => {
                discovered_items = record.items.clone();
                if discovered_items.is_empty() && !item_name.is_empty() {
                    let mut fallback = JsonObject::new();
                    fallback.insert("name".into(), JsonValue::from(item_name.clone()));
                    fallback.insert("type".into(), JsonValue::from(category.clone()));
                    fallback.insert("description".into(), JsonValue::from(description.clone()));
                    discovered_items = vec![fallback];
                }
                if !item_name.is_empty() && !description.is_empty() {
                    format!("You find {item_name}: {description}")
                } else if !item_name.is_empty() {
                    format!("You find {item_name}.")
                } else if !description.is_empty() {
                    description.clone()
                } else {
                    "You find something useful.".to_string()
                }
            }
            "clue" => {
                if description.is_empty() {
                    "You discover a clue.".to_string()
                } else {
                    description.clone()
                }
            }
            _ => {
                if !description.is_empty() {
                    description.clone()
                } else if !item_name.is_empty() {
                    item_name.clone()
                } else {
                    "You find something of note.".to_string()
                }
            }
        };

        if let Some(check) = &skill_check {
            message.push_str(&format!(
                " ({} check: {} + {} = {} vs DC {} — Success)",
                capitalize(&check.skill),
                check.roll,
                check.modifier,
                check.total,
                min_difficulty
            ));
        }

        Ok(DiscoveryResult {
            found: true,
            category: Some(category),
            result: Some(record),
            message,
            skill_check,
            discovered_items,
            updated_data: Some(cell_data),
            updated_features,
        })
    }

    fn perform_skill_check(
        &mut self,
        character: &Character,
        skill: &str,
        attribute: &str,
        difficulty: i32,
    ) -> DiscoverySkillCheck {
        let roll = self.rng.gen_range(1..=6) + self.rng.gen_range(1..=6);
        let skill_level = character.skills.get(skill).copied().unwrap_or(-1);
        let attr_mod = character.attr_mods.get(attribute).copied().unwrap_or(0);
        let modifier = skill_level + attr_mod;
        let total = roll + modifier;
        DiscoverySkillCheck {
            skill: skill.to_string(),
            attribute: attribute.to_string(),
            roll,
            modifier,
            total,
            difficulty,
            success: total >= difficulty,
        }
    }
}

/// Coerce a cell `data` field (JSON string or object) to an optional object.
fn coerce_data(value: Option<&JsonValue>) -> Option<JsonObject> {
    match value {
        Some(JsonValue::Object(map)) => Some(map.clone()),
        Some(JsonValue::String(s)) => match serde_json::from_str(s) {
            Ok(JsonValue::Object(map)) => Some(map),
            _ => None,
        },
        _ => None,
    }
}

/// Capitalize like Python `str.capitalize`: first char upper, rest lower.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::random_table::RandomTableRepository;
    use crate::scene_data::RandomTableEntry;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn hex(terrain: &str, explored: bool) -> JsonObject {
        let mut h = JsonObject::new();
        h.insert("q".into(), serde_json::json!(0));
        h.insert("r".into(), serde_json::json!(0));
        h.insert("terrain".into(), serde_json::json!(terrain));
        h.insert("explored".into(), serde_json::json!(explored));
        h
    }

    fn seed_cell(db: &WorldDatabase, q: i64, r: i64) {
        db.execute(
            "INSERT INTO cells (q, r, terrain, explored, features, data) VALUES (?, ?, ?, 0, ?, ?)",
            &[&q, &r, &"plains", &"[]", &"{}"],
        )
        .unwrap();
    }

    fn seed_table(db: &WorldDatabase, table_id: &str, record: serde_json::Value) {
        let entry: RandomTableEntry = serde_json::from_value(serde_json::json!({
            "weight": 1.0, "result": record,
        }))
        .unwrap();
        RandomTableRepository::new(db)
            .upsert_table(table_id, "discoveries", table_id, &[], &[entry], "test")
            .unwrap();
    }

    #[test]
    fn cooldown_blocks_recent_search() {
        let db = WorldDatabase::open_in_memory().unwrap();
        seed_cell(&db, 0, 0);
        // Persist a recent search tick.
        let mut data = JsonObject::new();
        data.insert("last_searched_tick".into(), serde_json::json!(50));
        DiscoveryRepository::new(&db).save_cell_data(0, 0, &data).unwrap();

        let engine = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let mut sys = DiscoverySystem::new(engine, SmallRng::seed_from_u64(1));
        let result = sys
            .search_hex(&hex("plains", true), &Character::default(), &db, 60, true)
            .unwrap();
        assert!(!result.found);
        assert!(result.message.contains("already searched"));
    }

    #[test]
    fn finds_item_when_table_yields_one() {
        let db = WorldDatabase::open_in_memory().unwrap();
        seed_cell(&db, 0, 0);
        seed_table(
            &db,
            "discoveries_plains",
            serde_json::json!({
                "type": "item", "name": "a rusty key", "description": "old and bent",
                "min_difficulty": 0,
            }),
        );
        // Find an RNG seed that triggers a find (prob 0.5 unexplored).
        let mut found = None;
        for seed in 0..50 {
            let mut sys = DiscoverySystem::new(
                TableEngine::new(&db, SmallRng::seed_from_u64(2)),
                SmallRng::seed_from_u64(seed),
            );
            let result = sys
                .search_hex(&hex("plains", false), &Character::default(), &db, 0, false)
                .unwrap();
            if result.found {
                found = Some(result);
                break;
            }
        }
        let result = found.expect("a find should occur within 50 seeds");
        assert_eq!(result.category.as_deref(), Some("item"));
        assert!(result.message.contains("a rusty key"));
        assert_eq!(result.discovered_items.len(), 1);
    }

    #[test]
    fn capitalize_matches_python() {
        assert_eq!(capitalize("notice"), "Notice");
        assert_eq!(capitalize(""), "");
    }
}
