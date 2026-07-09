//! Random table engine.
//!
//! Ported from `src/harsh_realm/engine/tables.py`. Loads YAML table definitions
//! into SQLite and rolls on them with weighted entries, range selection,
//! subtable resolution, and tag-based filtering.

use std::path::{Path, PathBuf};

use rand::Rng;

use crate::engine_results::TableResult;
use crate::generation::{GeneratorDefinition, GeneratorExecutionResult, RandomTableDefinition};
use crate::repositories::random_table::RandomTableRepository;
use crate::runtime::{JsonObject, JsonValue};
use crate::scene_data::RandomTableEntry;
use crate::tables::weighted_select;

/// Errors raised by the table engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// The requested table does not exist (Python `KeyError`).
    NotFound(String),
    /// The table exists but has no entries (Python `ValueError`).
    Empty(String),
    /// A filesystem, YAML, or database error occurred.
    Other(String),
}

impl std::fmt::Display for TableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TableError::NotFound(m) => write!(f, "Table not found: {m:?}"),
            TableError::Empty(m) => write!(f, "Table {m:?} has no entries"),
            TableError::Other(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for TableError {}

/// Load YAML tables into SQLite and roll on them with weighted entries.
pub struct TableEngine<'a, R: Rng> {
    tables: RandomTableRepository<'a>,
    rng: R,
    data_dir: Option<PathBuf>,
}

impl<'a, R: Rng> TableEngine<'a, R> {
    /// Create a table engine over a world database with an explicit RNG.
    pub fn new(db: &'a crate::db::WorldDatabase, rng: R) -> Self {
        TableEngine {
            tables: RandomTableRepository::new(db),
            rng,
            data_dir: None,
        }
    }

    /// Scan `data_dir/tables/` recursively for YAML files and upsert into DB.
    ///
    /// Returns the number of tables loaded. Malformed files are skipped.
    pub fn load_tables(&mut self, data_dir: &Path) -> Result<usize, String> {
        self.data_dir = Some(data_dir.to_path_buf());
        let tables_dir = data_dir.join("tables");
        if !tables_dir.exists() {
            return Ok(0);
        }
        let mut yaml_paths: Vec<PathBuf> = Vec::new();
        collect_yaml_files(&tables_dir, &mut yaml_paths);
        yaml_paths.sort();

        let mut count = 0;
        for yaml_path in &yaml_paths {
            let Ok(text) = std::fs::read_to_string(yaml_path) else {
                continue;
            };
            let Ok(JsonValue::Object(doc)) = serde_yaml::from_str::<JsonValue>(&text) else {
                continue;
            };
            if !doc.contains_key("id") {
                continue;
            }
            let definition: RandomTableDefinition =
                match serde_json::from_value(JsonValue::Object(doc)) {
                    Ok(def) => def,
                    Err(_) => continue,
                };
            self.upsert_table(&definition, &yaml_path.to_string_lossy())?;
            count += 1;
        }
        Ok(count)
    }

    fn upsert_table(&self, doc: &RandomTableDefinition, source: &str) -> Result<(), String> {
        let name = if doc.name.is_empty() {
            doc.id.clone()
        } else {
            doc.name.clone()
        };
        self.tables
            .upsert_table(&doc.id, &doc.category, &name, &doc.tags, &doc.entries, source)
    }

    /// Fetch a table, perform a weighted/range roll, and resolve subtables.
    pub fn roll_on(
        &mut self,
        table_id: &str,
        context: Option<&JsonObject>,
    ) -> Result<TableResult, TableError> {
        self.roll_on_chained(table_id, context, Vec::new())
    }

    fn roll_on_chained(
        &mut self,
        table_id: &str,
        context: Option<&JsonObject>,
        mut chain: Vec<String>,
    ) -> Result<TableResult, TableError> {
        chain.push(table_id.to_string());

        let row = self
            .tables
            .load_table(table_id)
            .map_err(TableError::Other)?
            .ok_or_else(|| TableError::NotFound(table_id.to_string()))?;
        let entries = row.entries;
        if entries.is_empty() {
            return Err(TableError::Empty(table_id.to_string()));
        }

        // Range-based selection (e.g. d100 tables).
        if entries.iter().any(|e| e.range.is_some()) {
            let max_val = entries
                .iter()
                .filter_map(|e| e.range_min_max())
                .map(|(_, hi)| hi)
                .max()
                .unwrap_or(0);
            if max_val > 0 {
                let roll = self.rng.gen_range(1..=max_val);
                if let Some(chosen) = entries.iter().find(|e| {
                    e.range_min_max()
                        .map(|(lo, hi)| lo <= roll && roll <= hi)
                        .unwrap_or(false)
                }) {
                    return self.resolve_entry(table_id, chosen.clone(), chain, context);
                }
            }
        }

        // Weighted selection fallback.
        let weights: Vec<f64> = entries.iter().map(|e| e.weight).collect();
        let total: f64 = weights.iter().sum();
        let r = if total > 0.0 {
            self.rng.gen_range(0.0..total)
        } else {
            0.0
        };
        let index = weighted_select(&weights, r);
        let chosen = entries[index].clone();
        self.resolve_entry(table_id, chosen, chain, context)
    }

    fn resolve_entry(
        &mut self,
        table_id: &str,
        entry: RandomTableEntry,
        chain: Vec<String>,
        context: Option<&JsonObject>,
    ) -> Result<TableResult, TableError> {
        match entry.result {
            JsonValue::String(text) => Ok(TableResult {
                table_id: table_id.to_string(),
                roll_chain: chain,
                result_type: "text".to_string(),
                result: text_payload(&text),
                raw_text: Some(text),
            }),
            JsonValue::Object(result_dict) => {
                // Subtable reference.
                if let Some(table_ref) = result_dict.get("table") {
                    let subtable_id = value_to_text(table_ref);
                    return self.roll_on_chained(&subtable_id, context, chain);
                }
                let result_type = result_dict
                    .get("type")
                    .map(value_to_text)
                    .unwrap_or_else(|| "unknown".to_string());
                let raw_text = result_dict
                    .get("name")
                    .or_else(|| result_dict.get("text"))
                    .map(value_to_text);
                Ok(TableResult {
                    table_id: table_id.to_string(),
                    roll_chain: chain,
                    result_type,
                    result: result_dict,
                    raw_text,
                })
            }
            other => {
                let text = value_to_text(&other);
                Ok(TableResult {
                    table_id: table_id.to_string(),
                    roll_chain: chain,
                    result_type: "text".to_string(),
                    result: text_payload(&text),
                    raw_text: Some(text),
                })
            }
        }
    }

    /// Find tables matching category + any of tags, pick one, roll on it.
    pub fn roll_with_tags(
        &mut self,
        category: &str,
        tags: &[String],
    ) -> Result<TableResult, TableError> {
        let rows = self
            .tables
            .list_tables_by_category(category)
            .map_err(TableError::Other)?;
        let matching: Vec<String> = rows
            .into_iter()
            .filter(|row| row.tags.iter().any(|t| tags.contains(t)))
            .map(|row| row.id)
            .collect();
        if matching.is_empty() {
            return Err(TableError::Other(format!(
                "No tables found for category={category:?} with tags={tags:?}"
            )));
        }
        let index = self.rng.gen_range(0..matching.len());
        let chosen_id = matching[index].clone();
        self.roll_on(&chosen_id, None)
    }

    /// Execute a multi-step generator YAML definition from `data_dir/generators/`.
    pub fn generate(
        &mut self,
        generator_id: &str,
    ) -> Result<GeneratorExecutionResult, TableError> {
        let data_dir = self.data_dir.clone().ok_or_else(|| {
            TableError::Other(
                "data_dir not set — call load_tables() first to establish data_dir".to_string(),
            )
        })?;
        let gen_path = data_dir
            .join("generators")
            .join(format!("{generator_id}.yaml"));
        if !gen_path.exists() {
            return Err(TableError::Other(format!(
                "Generator not found: {}",
                gen_path.display()
            )));
        }
        let text = std::fs::read_to_string(&gen_path).map_err(|e| TableError::Other(e.to_string()))?;
        let doc: JsonValue =
            serde_yaml::from_str(&text).map_err(|e| TableError::Other(e.to_string()))?;
        if !doc.is_object() {
            return Err(TableError::Other(format!(
                "Generator YAML must be a dict: {}",
                gen_path.display()
            )));
        }
        let definition: GeneratorDefinition =
            serde_json::from_value(doc).map_err(|e| TableError::Other(e.to_string()))?;

        let mut assignments = std::collections::BTreeMap::new();
        for step in &definition.steps {
            let table_result = self.roll_on(&step.roll, None)?;
            let value = if table_result.result_type == "text" && table_result.raw_text.is_some() {
                JsonValue::String(table_result.raw_text.unwrap())
            } else {
                JsonValue::Object(table_result.result)
            };
            assignments.insert(step.assign.clone(), value);
        }
        Ok(GeneratorExecutionResult { assignments })
    }
}

fn text_payload(text: &str) -> JsonObject {
    let mut map = JsonObject::new();
    map.insert("text".into(), JsonValue::String(text.to_string()));
    map
}

/// Render a JSON value the way Python `str()` would for table results.
fn value_to_text(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Bool(true) => "True".to_string(),
        JsonValue::Bool(false) => "False".to_string(),
        JsonValue::Null => "None".to_string(),
        other => other.to_string(),
    }
}

fn collect_yaml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn engine(db: &WorldDatabase) -> TableEngine<'_, SmallRng> {
        TableEngine::new(db, SmallRng::seed_from_u64(7))
    }

    fn entry(weight: f64, range: Option<&str>, result: JsonValue) -> RandomTableEntry {
        RandomTableEntry {
            weight,
            range: range.map(str::to_string),
            result,
        }
    }

    #[test]
    fn rolls_text_entry() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut eng = engine(&db);
        eng.tables
            .upsert_table(
                "greetings",
                "social",
                "Greetings",
                &[],
                &[entry(1.0, None, JsonValue::String("hello".into()))],
                "test",
            )
            .unwrap();
        let result = eng.roll_on("greetings", None).unwrap();
        assert_eq!(result.result_type, "text");
        assert_eq!(result.raw_text.as_deref(), Some("hello"));
        assert_eq!(result.roll_chain, vec!["greetings".to_string()]);
    }

    #[test]
    fn missing_table_is_not_found() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut eng = engine(&db);
        assert_eq!(
            eng.roll_on("ghost", None).unwrap_err(),
            TableError::NotFound("ghost".to_string())
        );
    }

    #[test]
    fn resolves_subtable_reference() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut eng = engine(&db);
        eng.tables
            .upsert_table(
                "outer",
                "c",
                "Outer",
                &[],
                &[entry(1.0, None, serde_json::json!({"table": "inner"}))],
                "t",
            )
            .unwrap();
        eng.tables
            .upsert_table(
                "inner",
                "c",
                "Inner",
                &[],
                &[entry(1.0, None, JsonValue::String("deep".into()))],
                "t",
            )
            .unwrap();
        let result = eng.roll_on("outer", None).unwrap();
        assert_eq!(result.raw_text.as_deref(), Some("deep"));
        assert_eq!(result.roll_chain, vec!["outer".to_string(), "inner".to_string()]);
    }

    #[test]
    fn range_selection_picks_matching_band() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut eng = engine(&db);
        // Single full-range band guarantees the deterministic roll matches it.
        eng.tables
            .upsert_table(
                "d100",
                "c",
                "D100",
                &[],
                &[entry(1.0, Some("1-100"), JsonValue::String("only".into()))],
                "t",
            )
            .unwrap();
        let result = eng.roll_on("d100", None).unwrap();
        assert_eq!(result.raw_text.as_deref(), Some("only"));
    }

    #[test]
    fn typed_dict_entry_keeps_type_and_name() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let mut eng = engine(&db);
        eng.tables
            .upsert_table(
                "loot",
                "c",
                "Loot",
                &[],
                &[entry(
                    1.0,
                    None,
                    serde_json::json!({"type": "weapon", "name": "Dagger"}),
                )],
                "t",
            )
            .unwrap();
        let result = eng.roll_on("loot", None).unwrap();
        assert_eq!(result.result_type, "weapon");
        assert_eq!(result.raw_text.as_deref(), Some("Dagger"));
    }
}
