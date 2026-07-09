//! Settlement generator.
//!
//! Ported from `src/harsh_realm/generators/settlement_gen.py`. Generates named
//! settlements with buildings and operator/resident NPCs, persisting them as
//! `settlement` / `building` / `npc` entities and stashing a backwards-compat
//! settlement summary plus town cells on the hex's cell data.

use rand::Rng;
use uuid::Uuid;

use crate::db::WorldDatabase;
use crate::npc::{GeneratedNPC, NPCGenerationContext};
use crate::repositories::cell::CellRepository;
use crate::repositories::entity::EntityRepository;
use crate::runtime::{JsonObject, JsonValue};
use crate::scene_data::SettlementBuilding;
use crate::settlement::{
    required_buildings, size_to_tier, BuildingData, SettlementData, SettlementSummary,
};
use crate::table_engine::TableEngine;

use super::npc_gen::NPCGenerator;
use super::square_gen::SquareWorldGenerator;

const NAME_PREFIXES: [&str; 7] = ["Mill", "Iron", "Ash", "Stone", "Grey", "Crow", "Thorn"];
const NAME_SUFFIXES: [&str; 7] = ["haven", "ford", "gate", "wick", "moor", "bridge", "hollow"];

fn npc_count_range(size: &str) -> (i32, i32) {
    match size {
        "hamlet" => (3, 4),
        "village" => (5, 8),
        "town" => (8, 12),
        _ => (3, 4),
    }
}

fn building_occupation(building_type: &str) -> &str {
    match building_type {
        "healer" => "healer",
        "general_store" => "merchant",
        "blacksmith" => "blacksmith",
        "tavern" => "innkeeper",
        other => other,
    }
}

/// Reverse of the establishment->building map, used for table lookups.
fn reverse_est_type(building_type: &str) -> &str {
    match building_type {
        "blacksmith" => "forge",
        "general_store" => "trading_post",
        other => other, // tavern -> tavern, healer -> healer
    }
}

/// Generate complete settlements with buildings and NPCs.
pub struct SettlementGenerator<'a, R: Rng> {
    engine: TableEngine<'a, R>,
    npc_gen: NPCGenerator<'a, R>,
    rng: R,
}

impl<'a, R: Rng> SettlementGenerator<'a, R> {
    /// Create a settlement generator from a table engine, NPC generator, and RNG.
    pub fn new(engine: TableEngine<'a, R>, npc_gen: NPCGenerator<'a, R>, rng: R) -> Self {
        SettlementGenerator {
            engine,
            npc_gen,
            rng,
        }
    }

    /// Generate a complete settlement and persist it to the database.
    pub fn generate_settlement(
        &mut self,
        hex_coord: (i32, i32),
        size: &str,
        terrain: &str,
        db: &WorldDatabase,
    ) -> Result<SettlementSummary, String> {
        let (q, r) = hex_coord;
        let tier = size_to_tier(size).unwrap_or("small").to_string();
        let settlement_id = Uuid::new_v4().to_string();

        let name = self.roll_name();
        let description = self.roll_description(size, terrain);

        let mut required: Vec<&str> = required_buildings(size).to_vec();
        if required.is_empty() {
            required = vec!["healer", "general_store"];
        }

        let mut building_ids: Vec<String> = Vec::new();
        let mut establishments: Vec<SettlementBuilding> = Vec::new();

        for building_type in &required {
            let building_id = Uuid::new_v4().to_string();
            let building_name = self.roll_building_name(building_type);
            let occupation = building_occupation(building_type).to_string();

            let mut npc_data = self.npc_gen.generate_npc(Some(NPCGenerationContext {
                settlement_size: None,
                terrain: None,
                occupation_override: Some(occupation),
            }))?;
            let npc_id = Uuid::new_v4().to_string();
            npc_data.building_id = Some(building_id.clone());
            npc_data.establishment_type = Some(building_type.to_string());
            npc_data.establishment_name = Some(building_name.clone());
            store_npc(db, &npc_id, &npc_data, q, r)?;

            let bdata = BuildingData {
                building_type: building_type.to_string(),
                building_name: building_name.clone(),
                tier: tier.clone(),
                settlement_id: settlement_id.clone(),
                operator_npc_id: Some(npc_id.clone()),
            };
            store_building(db, &building_id, &building_name, &bdata, q, r)?;
            building_ids.push(building_id.clone());

            establishments.push(SettlementBuilding {
                id: None,
                name: building_name,
                r#type: Some(building_type.to_string()),
                q: None,
                r: None,
                w: None,
                h: None,
                npc_id: Some(npc_id),
                npc_name: Some(npc_data.name.clone()),
                building_id: Some(building_id),
                extra: JsonObject::new(),
            });
        }

        // Resident NPCs.
        let (npc_min, npc_max) = npc_count_range(size);
        let total_npc_target = self.rng.gen_range(npc_min..=npc_max);
        let resident_count = (total_npc_target - required.len() as i32).max(0);

        let mut resident_npc_ids: Vec<String> = Vec::new();
        for _ in 0..resident_count {
            let npc_data = self.npc_gen.generate_npc(Some(NPCGenerationContext {
                settlement_size: Some(size.to_string()),
                terrain: Some(terrain.to_string()),
                occupation_override: None,
            }))?;
            let npc_id = Uuid::new_v4().to_string();
            store_npc(db, &npc_id, &npc_data, q, r)?;
            resident_npc_ids.push(npc_id);
        }

        let sdata = SettlementData {
            name: name.clone(),
            size: size.to_string(),
            description: description.clone(),
            building_ids,
            resident_npc_ids: resident_npc_ids.clone(),
        };
        store_settlement(db, &settlement_id, &name, &sdata, q, r)?;

        let settlement = SettlementSummary {
            name,
            size: size.to_string(),
            description,
            settlement_id,
            establishments,
            resident_npc_ids,
        };

        // Generate town cells for the enter-town scene.
        let town_result =
            SquareWorldGenerator.generate_town(20, 20, Some(settlement.clone()), &mut self.rng);
        let town_cells: Vec<JsonValue> = town_result
            .cells
            .iter()
            .map(|cell| serde_json::to_value(cell).unwrap_or(JsonValue::Null))
            .collect();

        let cell_repo = CellRepository::new(db);
        let mut existing = cell_repo.load_cell_data(q as i64, r as i64)?.unwrap_or_default();
        existing.insert(
            "settlement".into(),
            serde_json::to_value(&settlement).map_err(|e| e.to_string())?,
        );
        existing.insert("town_cells".into(), JsonValue::Array(town_cells));
        cell_repo.save_cell_data(q as i64, r as i64, &existing)?;

        Ok(settlement)
    }

    fn roll_name(&mut self) -> String {
        let prefix = match self.engine.roll_on("settlement_names_prefix", None) {
            Ok(result) => result.raw_text.unwrap_or_else(|| "Stone".to_string()),
            Err(_) => NAME_PREFIXES[self.rng.gen_range(0..NAME_PREFIXES.len())].to_string(),
        };
        let suffix = match self.engine.roll_on("settlement_names_suffix", None) {
            Ok(result) => result.raw_text.unwrap_or_else(|| "haven".to_string()),
            Err(_) => NAME_SUFFIXES[self.rng.gen_range(0..NAME_SUFFIXES.len())].to_string(),
        };
        format!("{prefix}{suffix}")
    }

    fn roll_description(&mut self, size: &str, terrain: &str) -> String {
        for _ in 0..10 {
            match self.engine.roll_on("settlement_descriptions", None) {
                Ok(result) => {
                    if result.result_type == "description" {
                        if let Some(text) = result.result.get("text").and_then(|v| v.as_str()) {
                            if !text.is_empty() {
                                return text.to_string();
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
        format!("A {size} settlement situated in {terrain} country.")
    }

    fn roll_building_name(&mut self, building_type: &str) -> String {
        let est_type_alt = reverse_est_type(building_type);
        for _ in 0..15 {
            match self.engine.roll_on("settlement_establishment_names", None) {
                Ok(result) => {
                    let matches_type =
                        result.result_type == building_type || result.result_type == est_type_alt;
                    if matches_type {
                        if let Some(name) = result.result.get("name").and_then(|v| v.as_str()) {
                            if !name.is_empty() {
                                return name.to_string();
                            }
                        }
                    }
                    if let Some(t) = result.result.get("type").and_then(|v| v.as_str()) {
                        if t == building_type || t == est_type_alt {
                            if let Some(name) = result.result.get("name").and_then(|v| v.as_str()) {
                                if !name.is_empty() {
                                    return name.to_string();
                                }
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
        format!("The {}", title_case(&building_type.replace('_', " ")))
    }
}

fn store_npc(
    db: &WorldDatabase,
    npc_id: &str,
    npc_data: &GeneratedNPC,
    q: i32,
    r: i32,
) -> Result<(), String> {
    EntityRepository::new(db).create_entity(npc_id, "npc", &npc_data.name, q as i64, r as i64, &to_object(npc_data)?)
}

fn store_building(
    db: &WorldDatabase,
    building_id: &str,
    name: &str,
    bdata: &BuildingData,
    q: i32,
    r: i32,
) -> Result<(), String> {
    EntityRepository::new(db).create_entity(building_id, "building", name, q as i64, r as i64, &to_object(bdata)?)
}

fn store_settlement(
    db: &WorldDatabase,
    settlement_id: &str,
    name: &str,
    sdata: &SettlementData,
    q: i32,
    r: i32,
) -> Result<(), String> {
    EntityRepository::new(db).create_entity(settlement_id, "settlement", name, q as i64, r as i64, &to_object(sdata)?)
}

fn to_object<T: serde::Serialize>(value: &T) -> Result<JsonObject, String> {
    match serde_json::to_value(value).map_err(|e| e.to_string())? {
        JsonValue::Object(map) => Ok(map),
        _ => Err("expected object payload".to_string()),
    }
}

/// Title-case each word, like Python `str.title()`.
fn title_case(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_data::RandomTableEntry;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn seed_npc_basic(db: &WorldDatabase) {
        // Seed name tables so npc_basic generation produces deterministic output.
        for (id, result) in [
            ("first_names", serde_json::json!("Mara")),
            ("surnames", serde_json::json!("Stone")),
        ] {
            let entry: RandomTableEntry =
                serde_json::from_value(serde_json::json!({"weight": 1.0, "result": result}))
                    .unwrap();
            crate::repositories::random_table::RandomTableRepository::new(db)
                .upsert_table(id, "npc", id, &[], &[entry], "t")
                .unwrap();
        }
    }

    #[test]
    fn title_case_matches_python() {
        assert_eq!(title_case("general store"), "General Store");
        assert_eq!(title_case("healer"), "Healer");
    }

    #[test]
    fn reverse_est_type_maps() {
        assert_eq!(reverse_est_type("blacksmith"), "forge");
        assert_eq!(reverse_est_type("general_store"), "trading_post");
        assert_eq!(reverse_est_type("tavern"), "tavern");
    }

    #[test]
    fn npc_count_ranges() {
        assert_eq!(npc_count_range("village"), (5, 8));
        assert_eq!(npc_count_range("unknown"), (3, 4));
    }

    #[test]
    fn roll_name_uses_fallback_without_tables() {
        let db = WorldDatabase::open_in_memory().unwrap();
        seed_npc_basic(&db);
        let engine = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let npc_gen = NPCGenerator::new(TableEngine::new(&db, SmallRng::seed_from_u64(2)));
        let mut gen = SettlementGenerator::new(engine, npc_gen, SmallRng::seed_from_u64(3));
        let name = gen.roll_name();
        // Fallback name is prefix+suffix from the constant lists.
        assert!(!name.is_empty());
        assert!(NAME_PREFIXES.iter().any(|p| name.starts_with(p)));
    }
}
