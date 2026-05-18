/// V5 persistence schema for Outpost 3.
///
/// Design principles:
///  - Hybrid normalized+JSON: queryable columns for filtering + `data_json` for full fidelity
///  - ANSI SQL throughout — no SQLite-specific functions in queries so PostgreSQL migration is easy
///  - TEXT for UUIDs (portable; PostgreSQL can use native UUID type with a column-type change)
///  - TEXT/JSON blob columns become JSONB in PostgreSQL for native indexing
///  - SVG icons stored as TEXT (XML text; works in both SQLite and PostgreSQL)

/// Tracks which versioned migrations have been applied.
pub const SCHEMA_MIGRATIONS: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
    version     INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL,
    applied_at  TEXT    NOT NULL
);
"#;

/// Saved game worlds (one row per save slot; default world id = 1).
///
/// `data_json` holds the full serialized `GameState` minus content definitions.
/// Content definitions live in their own tables so they can be edited independently.
pub const GAME_STATE_V5: &str = r#"
CREATE TABLE IF NOT EXISTS game_state_v5 (
    id          INTEGER PRIMARY KEY,
    name        TEXT    NOT NULL DEFAULT 'default',
    tick        INTEGER NOT NULL DEFAULT 0,
    data_json   TEXT    NOT NULL,
    saved_at    TEXT    NOT NULL
);
"#;

/// Building type definitions (editable via admin UI, importable from JSON/CSV).
///
/// `source` tracks origin: 'yaml' = loaded from YAML content file,
/// 'db' = created/edited in admin UI, 'import' = imported via API.
/// YAML startup sync will NOT overwrite records with source='db'.
pub const CONTENT_BUILDING_DEFS: &str = r#"
CREATE TABLE IF NOT EXISTS content_building_defs (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    category    TEXT    NOT NULL,
    data_json   TEXT    NOT NULL,
    icon_svg    TEXT,
    source      TEXT    NOT NULL DEFAULT 'yaml',
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
"#;

/// Resource type definitions.
pub const CONTENT_RESOURCE_DEFS: &str = r#"
CREATE TABLE IF NOT EXISTS content_resource_defs (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    category    TEXT    NOT NULL,
    data_json   TEXT    NOT NULL,
    icon_svg    TEXT,
    source      TEXT    NOT NULL DEFAULT 'yaml',
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
"#;

/// Gameplay event definitions (random/scripted events that fire during simulation).
pub const CONTENT_EVENT_DEFS: &str = r#"
CREATE TABLE IF NOT EXISTS content_event_defs (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    category    TEXT    NOT NULL,
    severity    TEXT    NOT NULL,
    data_json   TEXT    NOT NULL,
    source      TEXT    NOT NULL DEFAULT 'yaml',
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
"#;

/// Tech tree definitions.
pub const CONTENT_TECH_DEFS: &str = r#"
CREATE TABLE IF NOT EXISTS content_tech_defs (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    category    TEXT    NOT NULL,
    data_json   TEXT    NOT NULL,
    source      TEXT    NOT NULL DEFAULT 'yaml',
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
"#;

/// Persistent log of fired gameplay events (what actually happened in the game).
///
/// Separate from content_event_defs — this is the runtime event log.
pub const GAME_EVENT_LOG: &str = r#"
CREATE TABLE IF NOT EXISTS game_event_log (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    world_id        INTEGER NOT NULL DEFAULT 1,
    tick            INTEGER NOT NULL,
    event_def_id    TEXT,
    event_type      TEXT    NOT NULL,
    severity        TEXT,
    title           TEXT    NOT NULL,
    body            TEXT    NOT NULL DEFAULT '',
    site_id         TEXT,
    data_json       TEXT,
    created_at      TEXT    NOT NULL,
    FOREIGN KEY (world_id) REFERENCES game_state_v5(id)
);
"#;

pub const GAME_EVENT_LOG_IDX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_game_event_log_world_tick
    ON game_event_log(world_id, tick);
"#;

pub const CONTENT_BUILDING_DEFS_IDX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_content_building_defs_category
    ON content_building_defs(category);
"#;

pub const CONTENT_RESOURCE_DEFS_IDX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_content_resource_defs_category
    ON content_resource_defs(category);
"#;

/// Production recipe definitions.
pub const CONTENT_RECIPES: &str = r#"
CREATE TABLE IF NOT EXISTS content_recipes (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    data_json   TEXT    NOT NULL,
    source      TEXT    NOT NULL DEFAULT 'yaml',
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
"#;

pub const CONTENT_RECIPES_IDX: &str = r#"
CREATE INDEX IF NOT EXISTS idx_content_recipes_name ON content_recipes(name);
"#;
