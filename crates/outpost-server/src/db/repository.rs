/// Repository layer for V5 game persistence.
///
/// The `GameRepository` trait abstracts all DB I/O so it can be swapped from
/// SQLite to PostgreSQL by providing a different implementation.
///
/// Design notes:
///  - Content definitions (building types, resource types, etc.) are stored
///    separately from game state so they can be edited/imported independently.
///  - Game state is stored as a single JSON blob per save slot. The JSON covers
///    the full `GameState` except `#[serde(skip)]` content definition fields,
///    which are re-populated from the content tables on load.
///  - `source` column tracks record origin: 'yaml' | 'db' | 'import'.
///    YAML sync on startup does NOT overwrite records with source='db'.

use anyhow::{Context, Result};
use rusqlite::params;
use rusqlite::OptionalExtension;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tracing::{debug, info, warn};

use outpost_core::content::{BuildingDefinition, EventDefinition, ResourceDefinition, TechDefinition};
use outpost_core::domain::GameState;
use outpost_core::domain::Recipe;

use crate::db::DbPool;

// ─── Export table names ────────────────────────────────────────────────────────

/// Tables that can be exported or imported via the admin API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTable {
    BuildingDefs,
    ResourceDefs,
    EventDefs,
    TechDefs,
    GameEventLog,
}

impl ExportTable {
    pub fn table_name(&self) -> &'static str {
        match self {
            Self::BuildingDefs => "content_building_defs",
            Self::ResourceDefs => "content_resource_defs",
            Self::EventDefs => "content_event_defs",
            Self::TechDefs => "content_tech_defs",
            Self::GameEventLog => "game_event_log",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "building_defs" | "content_building_defs" => Some(Self::BuildingDefs),
            "resource_defs" | "content_resource_defs" => Some(Self::ResourceDefs),
            "event_defs" | "content_event_defs" => Some(Self::EventDefs),
            "tech_defs" | "content_tech_defs" => Some(Self::TechDefs),
            "game_event_log" => Some(Self::GameEventLog),
            _ => None,
        }
    }
}

// ─── Repository Trait ─────────────────────────────────────────────────────────

/// Abstraction over the persistence layer.
/// Implement this trait to swap between SQLite and PostgreSQL.
pub trait GameRepository: Send + Sync + 'static {
    // ── Game state ──────────────────────────────────────────────────────────

    /// Persist the current game state (world id = 1).
    fn save_game_state(&self, state: &GameState) -> Result<()>;

    /// Load the saved game state (world id = 1), if one exists.
    /// Returns `None` if no save is present (first run).
    fn load_game_state(&self) -> Result<Option<GameState>>;

    // ── Content: building definitions ───────────────────────────────────────

    /// Insert or update a building definition.
    /// `source`: 'yaml' | 'db' | 'import'
    fn upsert_building_def(&self, def: &BuildingDefinition, source: &str) -> Result<()>;

    /// Load all building definitions.
    fn get_all_building_defs(&self) -> Result<Vec<BuildingDefinition>>;

    /// Get a single building definition by ID.
    fn get_building_def(&self, id: &str) -> Result<Option<BuildingDefinition>>;

    /// Delete a building definition. Returns true if a row was removed.
    fn delete_building_def(&self, id: &str) -> Result<bool>;

    // ── Content: resource definitions ───────────────────────────────────────

    fn upsert_resource_def(&self, def: &ResourceDefinition, source: &str) -> Result<()>;
    fn get_all_resource_defs(&self) -> Result<Vec<ResourceDefinition>>;
    fn get_resource_def(&self, id: &str) -> Result<Option<ResourceDefinition>>;
    fn delete_resource_def(&self, id: &str) -> Result<bool>;

    // ── Content: event definitions ──────────────────────────────────────────

    fn upsert_event_def(&self, def: &EventDefinition, source: &str) -> Result<()>;
    fn get_all_event_defs(&self) -> Result<Vec<EventDefinition>>;
    fn delete_event_def(&self, id: &str) -> Result<bool>;

    // ── Content: tech definitions ────────────────────────────────────────────

    fn upsert_tech_def(&self, def: &TechDefinition, source: &str) -> Result<()>;
    fn get_all_tech_defs(&self) -> Result<Vec<TechDefinition>>;
    fn delete_tech_def(&self, id: &str) -> Result<bool>;

    // ── Content: recipes ─────────────────────────────────────────────────────

    fn upsert_recipe(&self, recipe: &Recipe, source: &str) -> Result<()>;
    fn get_all_recipes(&self) -> Result<Vec<Recipe>>;
    fn get_recipe(&self, id: &str) -> Result<Option<Recipe>>;
    fn delete_recipe(&self, id: &str) -> Result<bool>;

    // ── SVG icons ────────────────────────────────────────────────────────────

    /// Store an SVG icon for a building definition.
    fn set_building_icon_svg(&self, id: &str, svg: &str) -> Result<()>;

    /// Store an SVG icon for a resource definition.
    fn set_resource_icon_svg(&self, id: &str, svg: &str) -> Result<()>;

    // ── Export / Import ──────────────────────────────────────────────────────

    /// Export all rows from a table as JSON objects (each row as a JSON object).
    fn export_json(&self, table: ExportTable) -> Result<Vec<JsonValue>>;

    /// Import JSON records into a content table.
    /// Existing records with the same id are replaced.
    /// Returns the number of records successfully imported.
    fn import_json(&self, table: ExportTable, records: Vec<JsonValue>) -> Result<usize>;
}

// ─── SQLite Implementation ────────────────────────────────────────────────────

pub struct SqliteGameRepository {
    pool: DbPool,
}

impl SqliteGameRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    fn conn(&self) -> Result<r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>> {
        self.pool.get().context("Failed to acquire DB connection")
    }
}

/// Shared type alias so handlers can hold the repository behind a trait object.
pub type SharedRepository = Arc<dyn GameRepository>;

impl GameRepository for SqliteGameRepository {
    // ── Game state ──────────────────────────────────────────────────────────

    fn save_game_state(&self, state: &GameState) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(state).context("Failed to serialize GameState")?;
        let tick = state.clock.current_tick;

        conn.execute(
            r#"INSERT INTO game_state_v5 (id, name, tick, data_json, saved_at)
               VALUES (1, 'default', ?1, ?2, datetime('now'))
               ON CONFLICT(id) DO UPDATE SET
                   tick     = excluded.tick,
                   data_json = excluded.data_json,
                   saved_at  = excluded.saved_at"#,
            params![tick as i64, json],
        )
        .context("Failed to save game state")?;

        debug!("Game state saved at tick {}", tick);
        Ok(())
    }

    fn load_game_state(&self) -> Result<Option<GameState>> {
        let conn = self.conn()?;

        let result: Option<String> = conn
            .query_row(
                "SELECT data_json FROM game_state_v5 WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .context("Failed to query game_state_v5")?;

        match result {
            None => Ok(None),
            Some(json) => {
                let state: GameState =
                    serde_json::from_str(&json).context("Failed to deserialize GameState")?;
                info!("Game state loaded from DB at tick {}", state.clock.current_tick);
                Ok(Some(state))
            }
        }
    }

    // ── Building definitions ────────────────────────────────────────────────

    fn upsert_building_def(&self, def: &BuildingDefinition, source: &str) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(def).context("Failed to serialize BuildingDefinition")?;
        let category = format!("{:?}", def.category).to_lowercase();

        conn.execute(
            r#"INSERT INTO content_building_defs (id, name, category, data_json, source, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
               ON CONFLICT(id) DO UPDATE SET
                   name       = excluded.name,
                   category   = excluded.category,
                   data_json  = excluded.data_json,
                   source     = excluded.source,
                   updated_at = datetime('now')
               WHERE content_building_defs.source != 'db'"#,
            params![def.id, def.name, category, json, source],
        )
        .context("Failed to upsert building definition")?;

        Ok(())
    }

    fn get_all_building_defs(&self) -> Result<Vec<BuildingDefinition>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT data_json FROM content_building_defs ORDER BY name")
            .context("Failed to prepare building defs query")?;

        let defs: Vec<BuildingDefinition> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .context("Failed to query building defs")?
            .filter_map(|r| {
                r.ok().and_then(|json| {
                    serde_json::from_str(&json)
                        .map_err(|e| warn!("Failed to deserialize building def: {}", e))
                        .ok()
                })
            })
            .collect();

        Ok(defs)
    }

    fn get_building_def(&self, id: &str) -> Result<Option<BuildingDefinition>> {
        let conn = self.conn()?;
        let result: Option<String> = conn
            .query_row(
                "SELECT data_json FROM content_building_defs WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .context("Failed to query building def")?;

        result
            .map(|json| serde_json::from_str(&json).context("Failed to deserialize building def"))
            .transpose()
    }

    fn delete_building_def(&self, id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let affected = conn
            .execute("DELETE FROM content_building_defs WHERE id = ?1", params![id])
            .context("Failed to delete building def")?;
        Ok(affected > 0)
    }

    // ── Resource definitions ────────────────────────────────────────────────

    fn upsert_resource_def(&self, def: &ResourceDefinition, source: &str) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(def).context("Failed to serialize ResourceDefinition")?;
        let category = format!("{:?}", def.category).to_lowercase();

        conn.execute(
            r#"INSERT INTO content_resource_defs (id, name, category, data_json, source, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
               ON CONFLICT(id) DO UPDATE SET
                   name       = excluded.name,
                   category   = excluded.category,
                   data_json  = excluded.data_json,
                   source     = excluded.source,
                   updated_at = datetime('now')
               WHERE content_resource_defs.source != 'db'"#,
            params![def.id, def.name, category, json, source],
        )
        .context("Failed to upsert resource definition")?;

        Ok(())
    }

    fn get_all_resource_defs(&self) -> Result<Vec<ResourceDefinition>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT data_json FROM content_resource_defs ORDER BY name")
            .context("Failed to prepare resource defs query")?;

        let defs: Vec<ResourceDefinition> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .context("Failed to query resource defs")?
            .filter_map(|r| {
                r.ok().and_then(|json| {
                    serde_json::from_str(&json)
                        .map_err(|e| warn!("Failed to deserialize resource def: {}", e))
                        .ok()
                })
            })
            .collect();

        Ok(defs)
    }

    fn get_resource_def(&self, id: &str) -> Result<Option<ResourceDefinition>> {
        let conn = self.conn()?;
        let result: Option<String> = conn
            .query_row(
                "SELECT data_json FROM content_resource_defs WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .context("Failed to query resource def")?;

        result
            .map(|json| serde_json::from_str(&json).context("Failed to deserialize resource def"))
            .transpose()
    }

    fn delete_resource_def(&self, id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let affected = conn
            .execute("DELETE FROM content_resource_defs WHERE id = ?1", params![id])
            .context("Failed to delete resource def")?;
        Ok(affected > 0)
    }

    // ── Event definitions ───────────────────────────────────────────────────

    fn upsert_event_def(&self, def: &EventDefinition, source: &str) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(def).context("Failed to serialize EventDefinition")?;
        let category = format!("{:?}", def.category).to_lowercase();
        let severity = format!("{:?}", def.severity).to_lowercase();

        conn.execute(
            r#"INSERT INTO content_event_defs (id, name, category, severity, data_json, source, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'), datetime('now'))
               ON CONFLICT(id) DO UPDATE SET
                   name       = excluded.name,
                   category   = excluded.category,
                   severity   = excluded.severity,
                   data_json  = excluded.data_json,
                   source     = excluded.source,
                   updated_at = datetime('now')
               WHERE content_event_defs.source != 'db'"#,
            params![def.id, def.title, category, severity, json, source],
        )
        .context("Failed to upsert event definition")?;

        Ok(())
    }

    fn get_all_event_defs(&self) -> Result<Vec<EventDefinition>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT data_json FROM content_event_defs ORDER BY name")
            .context("Failed to prepare event defs query")?;

        let defs: Vec<EventDefinition> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .context("Failed to query event defs")?
            .filter_map(|r| {
                r.ok().and_then(|json| {
                    serde_json::from_str(&json)
                        .map_err(|e| warn!("Failed to deserialize event def: {}", e))
                        .ok()
                })
            })
            .collect();

        Ok(defs)
    }

    fn delete_event_def(&self, id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let affected = conn
            .execute("DELETE FROM content_event_defs WHERE id = ?1", params![id])
            .context("Failed to delete event def")?;
        Ok(affected > 0)
    }

    // ── Tech definitions ────────────────────────────────────────────────────

    fn upsert_tech_def(&self, def: &TechDefinition, source: &str) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(def).context("Failed to serialize TechDefinition")?;
        let category = format!("{:?}", def.category).to_lowercase();

        conn.execute(
            r#"INSERT INTO content_tech_defs (id, name, category, data_json, source, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))
               ON CONFLICT(id) DO UPDATE SET
                   name       = excluded.name,
                   category   = excluded.category,
                   data_json  = excluded.data_json,
                   source     = excluded.source,
                   updated_at = datetime('now')
               WHERE content_tech_defs.source != 'db'"#,
            params![def.id, def.name, category, json, source],
        )
        .context("Failed to upsert tech definition")?;

        Ok(())
    }

    fn get_all_tech_defs(&self) -> Result<Vec<TechDefinition>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT data_json FROM content_tech_defs ORDER BY name")
            .context("Failed to prepare tech defs query")?;

        let defs: Vec<TechDefinition> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .context("Failed to query tech defs")?
            .filter_map(|r| {
                r.ok().and_then(|json| {
                    serde_json::from_str(&json)
                        .map_err(|e| warn!("Failed to deserialize tech def: {}", e))
                        .ok()
                })
            })
            .collect();

        Ok(defs)
    }

    fn delete_tech_def(&self, id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let affected = conn
            .execute("DELETE FROM content_tech_defs WHERE id = ?1", params![id])
            .context("Failed to delete tech def")?;
        Ok(affected > 0)
    }

    // ── Recipes ──────────────────────────────────────────────────────────────

    fn upsert_recipe(&self, recipe: &Recipe, source: &str) -> Result<()> {
        let conn = self.conn()?;
        let json = serde_json::to_string(recipe).context("Failed to serialize Recipe")?;

        conn.execute(
            r#"INSERT INTO content_recipes (id, name, data_json, source, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))
               ON CONFLICT(id) DO UPDATE SET
                   name       = excluded.name,
                   data_json  = excluded.data_json,
                   source     = excluded.source,
                   updated_at = datetime('now')
               WHERE content_recipes.source != 'db'"#,
            params![recipe.id, recipe.name, json, source],
        )
        .context("Failed to upsert recipe")?;

        Ok(())
    }

    fn get_all_recipes(&self) -> Result<Vec<Recipe>> {
        let conn = self.conn()?;
        let mut stmt = conn
            .prepare("SELECT data_json FROM content_recipes ORDER BY name")
            .context("Failed to prepare recipes query")?;

        let recipes: Vec<Recipe> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .context("Failed to query recipes")?
            .filter_map(|r| {
                r.ok().and_then(|json| {
                    serde_json::from_str(&json)
                        .map_err(|e| warn!("Failed to deserialize recipe: {}", e))
                        .ok()
                })
            })
            .collect();

        Ok(recipes)
    }

    fn get_recipe(&self, id: &str) -> Result<Option<Recipe>> {
        let conn = self.conn()?;
        let result: Option<String> = conn
            .query_row(
                "SELECT data_json FROM content_recipes WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .context("Failed to query recipe")?;

        result
            .map(|json| serde_json::from_str(&json).context("Failed to deserialize recipe"))
            .transpose()
    }

    fn delete_recipe(&self, id: &str) -> Result<bool> {
        let conn = self.conn()?;
        let affected = conn
            .execute("DELETE FROM content_recipes WHERE id = ?1", params![id])
            .context("Failed to delete recipe")?;
        Ok(affected > 0)
    }

    // ── SVG icons ────────────────────────────────────────────────────────────

    fn set_building_icon_svg(&self, id: &str, svg: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE content_building_defs SET icon_svg = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![svg, id],
        )
        .context("Failed to set building icon SVG")?;
        Ok(())
    }

    fn set_resource_icon_svg(&self, id: &str, svg: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "UPDATE content_resource_defs SET icon_svg = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![svg, id],
        )
        .context("Failed to set resource icon SVG")?;
        Ok(())
    }

    // ── Export / Import ──────────────────────────────────────────────────────

    fn export_json(&self, table: ExportTable) -> Result<Vec<JsonValue>> {
        let conn = self.conn()?;
        let table_name = table.table_name();

        // For content tables, export the full data_json of each row together with metadata.
        let sql = match table {
            ExportTable::BuildingDefs | ExportTable::ResourceDefs
            | ExportTable::EventDefs | ExportTable::TechDefs => {
                format!(
                    "SELECT json_object('id', id, 'name', name, 'source', source, \
                             'updated_at', updated_at, 'data', json(data_json)) \
                     FROM {} ORDER BY id",
                    table_name
                )
            }
            ExportTable::GameEventLog => {
                format!(
                    "SELECT json_object('id', id, 'tick', tick, 'event_type', event_type, \
                             'severity', severity, 'title', title, 'body', body, \
                             'site_id', site_id, 'created_at', created_at) \
                     FROM {} ORDER BY id",
                    table_name
                )
            }
        };

        let mut stmt = conn.prepare(&sql).context("Failed to prepare export query")?;
        let rows: Vec<JsonValue> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .context("Failed to execute export query")?
            .filter_map(|r| {
                r.ok().and_then(|json_str| {
                    serde_json::from_str(&json_str)
                        .map_err(|e| warn!("Failed to parse export row: {}", e))
                        .ok()
                })
            })
            .collect();

        Ok(rows)
    }

    fn import_json(&self, table: ExportTable, records: Vec<JsonValue>) -> Result<usize> {
        let conn = self.conn()?;
        let mut count = 0usize;

        match table {
            ExportTable::BuildingDefs => {
                for record in records {
                    // Accept either {data: {...}} envelope from export or raw BuildingDefinition
                    let def_value = record.get("data").cloned().unwrap_or(record);
                    match serde_json::from_value::<BuildingDefinition>(def_value) {
                        Ok(def) => {
                            self.upsert_building_def(&def, "import")?;
                            count += 1;
                        }
                        Err(e) => warn!("Skipping invalid building def record: {}", e),
                    }
                }
            }
            ExportTable::ResourceDefs => {
                for record in records {
                    let def_value = record.get("data").cloned().unwrap_or(record);
                    match serde_json::from_value::<ResourceDefinition>(def_value) {
                        Ok(def) => {
                            self.upsert_resource_def(&def, "import")?;
                            count += 1;
                        }
                        Err(e) => warn!("Skipping invalid resource def record: {}", e),
                    }
                }
            }
            ExportTable::EventDefs => {
                for record in records {
                    let def_value = record.get("data").cloned().unwrap_or(record);
                    match serde_json::from_value::<EventDefinition>(def_value) {
                        Ok(def) => {
                            self.upsert_event_def(&def, "import")?;
                            count += 1;
                        }
                        Err(e) => warn!("Skipping invalid event def record: {}", e),
                    }
                }
            }
            ExportTable::TechDefs => {
                for record in records {
                    let def_value = record.get("data").cloned().unwrap_or(record);
                    match serde_json::from_value::<TechDefinition>(def_value) {
                        Ok(def) => {
                            self.upsert_tech_def(&def, "import")?;
                            count += 1;
                        }
                        Err(e) => warn!("Skipping invalid tech def record: {}", e),
                    }
                }
            }
            ExportTable::GameEventLog => {
                // Event log is append-only; import inserts new rows.
                for record in records {
                    let tick = record.get("tick").and_then(|v| v.as_i64()).unwrap_or(0);
                    let event_type = record.get("event_type")
                        .and_then(|v| v.as_str()).unwrap_or("imported");
                    let title = record.get("title")
                        .and_then(|v| v.as_str()).unwrap_or("Imported Event");
                    let body = record.get("body")
                        .and_then(|v| v.as_str()).unwrap_or("");
                    let severity = record.get("severity").and_then(|v| v.as_str());
                    let site_id = record.get("site_id").and_then(|v| v.as_str());

                    conn.execute(
                        r#"INSERT INTO game_event_log
                           (world_id, tick, event_type, severity, title, body, site_id, created_at)
                           VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))"#,
                        params![tick, event_type, severity, title, body, site_id],
                    )
                    .context("Failed to insert event log row")?;
                    count += 1;
                }
            }
        }

        info!("Imported {} records into {}", count, table.table_name());
        Ok(count)
    }
}

// ─── CSV helpers ──────────────────────────────────────────────────────────────

/// Convert a list of JSON objects to CSV format.
/// The header row is derived from the keys of the first record.
pub fn json_to_csv(records: &[JsonValue]) -> String {
    if records.is_empty() {
        return String::new();
    }

    // Collect column names from the first record.
    let columns: Vec<String> = records[0]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();

    let mut csv = columns.join(",");
    csv.push('\n');

    for record in records {
        let row: Vec<String> = columns
            .iter()
            .map(|col| {
                let val = record.get(col).cloned().unwrap_or(JsonValue::Null);
                csv_escape(&val)
            })
            .collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }

    csv
}

/// Parse CSV text into JSON objects using the header row for keys.
pub fn csv_to_json(csv: &str) -> Result<Vec<JsonValue>> {
    let mut lines = csv.lines();
    let header_line = lines.next().context("CSV has no header row")?;
    let columns: Vec<&str> = header_line.split(',').collect();

    let records: Vec<JsonValue> = lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let values: Vec<&str> = line.splitn(columns.len(), ',').collect();
            let mut obj = serde_json::Map::new();
            for (col, val) in columns.iter().zip(values.iter()) {
                obj.insert(col.to_string(), JsonValue::String(val.to_string()));
            }
            JsonValue::Object(obj)
        })
        .collect();

    Ok(records)
}

fn csv_escape(val: &JsonValue) -> String {
    match val {
        JsonValue::Null => String::new(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => {
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.clone()
            }
        }
        other => {
            let s = other.to_string();
            if s.contains(',') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::v5_migrations::run_v5_migrations;
    use r2d2::Pool;
    use r2d2_sqlite::SqliteConnectionManager;

    fn make_repo() -> SqliteGameRepository {
        let manager = SqliteConnectionManager::memory();
        let pool = Pool::new(manager).unwrap();
        let conn = pool.get().unwrap();
        run_v5_migrations(&conn).unwrap();
        drop(conn);
        SqliteGameRepository::new(pool)
    }

    #[test]
    fn building_def_roundtrip() {
        use outpost_core::content::BuildingCategory;

        let repo = make_repo();
        let def = BuildingDefinition {
            id: "test_building".to_string(),
            name: "Test Building".to_string(),
            description: "A test".to_string(),
            category: BuildingCategory::Mining,
            construction_costs: vec![],
            construction_time_ticks: 10,
            power: None,
            power_output_mw: None,
            worker_capacity: 3,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: std::collections::HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        };

        repo.upsert_building_def(&def, "test").unwrap();

        let loaded = repo.get_building_def("test_building").unwrap().unwrap();
        assert_eq!(loaded.id, def.id);
        assert_eq!(loaded.name, def.name);

        let all = repo.get_all_building_defs().unwrap();
        assert_eq!(all.len(), 1);

        let deleted = repo.delete_building_def("test_building").unwrap();
        assert!(deleted);

        let gone = repo.get_building_def("test_building").unwrap();
        assert!(gone.is_none());
    }

    #[test]
    fn yaml_source_not_overwritten_by_db_source() {
        use outpost_core::content::BuildingCategory;

        let repo = make_repo();
        let mut def = BuildingDefinition {
            id: "test_b".to_string(),
            name: "Original".to_string(),
            description: "".to_string(),
            category: BuildingCategory::Mining,
            construction_costs: vec![],
            construction_time_ticks: 1,
            power: None,
            power_output_mw: None,
            worker_capacity: 1,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: std::collections::HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        };

        // Insert as 'db' (user-edited)
        repo.upsert_building_def(&def, "db").unwrap();

        // Now YAML sync tries to overwrite with 'yaml' source — name should NOT change
        def.name = "YAML Override Attempt".to_string();
        repo.upsert_building_def(&def, "yaml").unwrap();

        let conn = repo.conn().unwrap();
        let (name, source): (String, String) = conn
            .query_row(
                "SELECT name, source FROM content_building_defs WHERE id = 'test_b'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(source, "db", "source should remain 'db'");
        assert_eq!(name, "Original", "db-sourced record should not be overwritten by yaml");
    }

    #[test]
    fn export_import_roundtrip() {
        use outpost_core::content::BuildingCategory;

        let repo = make_repo();
        let def = BuildingDefinition {
            id: "exp_test".to_string(),
            name: "Export Test".to_string(),
            description: "".to_string(),
            category: BuildingCategory::Storage,
            construction_costs: vec![],
            construction_time_ticks: 5,
            power: None,
            power_output_mw: None,
            worker_capacity: 2,
            storage_capacity: Some(1000.0),
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: std::collections::HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        };
        repo.upsert_building_def(&def, "db").unwrap();

        let exported = repo.export_json(ExportTable::BuildingDefs).unwrap();
        assert_eq!(exported.len(), 1);

        // Delete and re-import
        repo.delete_building_def("exp_test").unwrap();
        let count = repo.import_json(ExportTable::BuildingDefs, exported).unwrap();
        assert_eq!(count, 1);

        let reloaded = repo.get_building_def("exp_test").unwrap().unwrap();
        assert_eq!(reloaded.id, "exp_test");
    }

    #[test]
    fn csv_roundtrip() {
        let records = vec![
            serde_json::json!({"id": "a", "name": "Alpha", "value": 1}),
            serde_json::json!({"id": "b", "name": "Beta, with comma", "value": 2}),
        ];

        let csv = json_to_csv(&records);
        let parsed = csv_to_json(&csv).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["id"], "a");
        assert_eq!(parsed[1]["id"], "b");
    }
}
