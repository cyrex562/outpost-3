//! Schema constants for the world database.
//!
//! Ported verbatim from `src/harsh_realm/db_schema.py`.

/// Tables that must exist in a valid world database.
pub const REQUIRED_TABLES: [&str; 42] = [
    "cell_death_markers",
    "cell_search_state",
    "cell_settlements",
    "cells",
    "character_state",
    "creature_templates",
    "difficulty_settings",
    "difficulty_targets",
    "disposition_outcomes",
    "dungeon_connections",
    "dungeon_rooms",
    "dungeons",
    "encounter_weights",
    "entities",
    "entity_resources",
    "entity_status_effects",
    "entity_transient_modifiers",
    "event_log",
    "faction_asset_state",
    "faction_asset_stats",
    "faction_assets",
    "faction_goals",
    "faction_relation_history",
    "faction_relations",
    "faction_tags",
    "factions",
    "gm_state",
    "items",
    "npc_state",
    "oracle_npcs",
    "pack_overrides",
    "plotline_scenes",
    "plotlines",
    "practice_log",
    "quest_instances",
    "random_tables",
    "reputation",
    "skill_mappings",
    "threads",
    "world_meta",
    "world_packs",
    "xp_progression",
];

/// Full DDL for a new world database (idempotent: `CREATE TABLE IF NOT EXISTS`).
pub const SCHEMA_SQL: &str = r##"
CREATE TABLE IF NOT EXISTS world_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cells (
    q           INTEGER NOT NULL,
    r           INTEGER NOT NULL,
    terrain     TEXT NOT NULL,
    features    TEXT DEFAULT '[]',
    explored    INTEGER DEFAULT 0,
    description TEXT,
    faction_id  TEXT,
    data        TEXT DEFAULT '{}',
    PRIMARY KEY (q, r)
);

CREATE TABLE IF NOT EXISTS cell_settlements (
    q                  INTEGER NOT NULL,
    r                  INTEGER NOT NULL,
    name               TEXT NOT NULL,
    size               TEXT NOT NULL,
    description        TEXT NOT NULL DEFAULT '',
    settlement_id      TEXT NOT NULL,
    starting           INTEGER NOT NULL DEFAULT 0,
    establishments_json TEXT NOT NULL DEFAULT '[]',
    resident_npc_ids_json TEXT NOT NULL DEFAULT '[]',
    town_cells_json    TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (q, r)
);

CREATE TABLE IF NOT EXISTS cell_search_state (
    q                  INTEGER NOT NULL,
    r                  INTEGER NOT NULL,
    last_searched_tick INTEGER NOT NULL,
    PRIMARY KEY (q, r)
);

CREATE TABLE IF NOT EXISTS cell_death_markers (
    q           INTEGER NOT NULL,
    r           INTEGER NOT NULL,
    marker_index INTEGER NOT NULL,
    items_json  TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (q, r, marker_index)
);

CREATE TABLE IF NOT EXISTS entities (
    id          TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    name        TEXT NOT NULL,
    location_q  INTEGER,
    location_r  INTEGER,
    alive       INTEGER DEFAULT 1,
    data        TEXT DEFAULT '{}',
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS character_state (
    entity_id            TEXT PRIMARY KEY REFERENCES entities(id),
    character_class      TEXT NOT NULL,
    level                INTEGER NOT NULL DEFAULT 1,
    xp                   INTEGER NOT NULL DEFAULT 0,
    xp_next              INTEGER NOT NULL DEFAULT 1500,
    hp                   INTEGER NOT NULL DEFAULT 0,
    max_hp               INTEGER NOT NULL DEFAULT 0,
    ac                   INTEGER NOT NULL DEFAULT 10,
    attack_bonus         INTEGER NOT NULL DEFAULT 0,
    physical_save        INTEGER NOT NULL DEFAULT 15,
    evasion_save         INTEGER NOT NULL DEFAULT 15,
    mental_save          INTEGER NOT NULL DEFAULT 15,
    position_q           INTEGER NOT NULL DEFAULT 0,
    position_r           INTEGER NOT NULL DEFAULT 0,
    attributes_json      TEXT NOT NULL DEFAULT '{}',
    attr_mods_json       TEXT NOT NULL DEFAULT '{}',
    skills_json          TEXT NOT NULL DEFAULT '{}',
    save_bonuses_json    TEXT NOT NULL DEFAULT '{}',
    equipment_json       TEXT NOT NULL DEFAULT '[]',
    class_abilities_json TEXT NOT NULL DEFAULT '{}',
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS npc_state (
    entity_id                TEXT PRIMARY KEY REFERENCES entities(id),
    occupation               TEXT NOT NULL DEFAULT '',
    personality_traits_json  TEXT NOT NULL DEFAULT '[]',
    motivation               TEXT NOT NULL DEFAULT '',
    appearance               TEXT NOT NULL DEFAULT '',
    greeting                 TEXT NOT NULL DEFAULT '',
    faction_id               TEXT,
    disposition              INTEGER NOT NULL DEFAULT 0,
    une_personality_json     TEXT,
    building_id              TEXT,
    establishment_type       TEXT,
    establishment_name       TEXT,
    created_at               TEXT NOT NULL,
    updated_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS factions (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    hp          INTEGER NOT NULL DEFAULT 7,
    max_hp      INTEGER NOT NULL DEFAULT 7,
    force       INTEGER NOT NULL DEFAULT 0,
    cunning     INTEGER NOT NULL DEFAULT 0,
    wealth      INTEGER NOT NULL DEFAULT 0,
    xp          INTEGER NOT NULL DEFAULT 0,
    home_q      INTEGER,
    home_r      INTEGER,
    goals       TEXT DEFAULT '[]',
    tags        TEXT DEFAULT '[]',
    data        TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS faction_goals (
    faction_id  TEXT NOT NULL REFERENCES factions(id),
    goal_index  INTEGER NOT NULL,
    goal        TEXT NOT NULL,
    PRIMARY KEY (faction_id, goal_index)
);

CREATE TABLE IF NOT EXISTS faction_tags (
    faction_id  TEXT NOT NULL REFERENCES factions(id),
    tag_index   INTEGER NOT NULL,
    tag         TEXT NOT NULL,
    PRIMARY KEY (faction_id, tag_index)
);

CREATE TABLE IF NOT EXISTS faction_assets (
    id          TEXT PRIMARY KEY,
    faction_id  TEXT NOT NULL REFERENCES factions(id),
    asset_type  TEXT NOT NULL,
    category    TEXT NOT NULL,
    hp          INTEGER NOT NULL,
    max_hp      INTEGER NOT NULL,
    location_q  INTEGER,
    location_r  INTEGER,
    data        TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS faction_asset_state (
    asset_id     TEXT PRIMARY KEY REFERENCES faction_assets(id),
    expanded     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS faction_relations (
    faction_a   TEXT NOT NULL REFERENCES factions(id),
    faction_b   TEXT NOT NULL REFERENCES factions(id),
    disposition TEXT NOT NULL DEFAULT 'neutral',
    history     TEXT DEFAULT '[]',
    PRIMARY KEY (faction_a, faction_b)
);

CREATE TABLE IF NOT EXISTS faction_relation_history (
    faction_a      TEXT NOT NULL REFERENCES factions(id),
    faction_b      TEXT NOT NULL REFERENCES factions(id),
    history_index  INTEGER NOT NULL,
    note           TEXT NOT NULL,
    PRIMARY KEY (faction_a, faction_b, history_index)
);

CREATE TABLE IF NOT EXISTS reputation (
    entity_id   TEXT NOT NULL REFERENCES entities(id),
    faction_id  TEXT NOT NULL REFERENCES factions(id),
    score       INTEGER DEFAULT 0,
    PRIMARY KEY (entity_id, faction_id)
);

CREATE TABLE IF NOT EXISTS random_tables (
    id          TEXT PRIMARY KEY,
    category    TEXT NOT NULL,
    name        TEXT NOT NULL,
    entries     TEXT NOT NULL,
    tags        TEXT DEFAULT '[]',
    source      TEXT
);

CREATE TABLE IF NOT EXISTS event_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    tick        INTEGER NOT NULL,
    event_id    TEXT NOT NULL,
    event_type  TEXT NOT NULL,
    source      TEXT NOT NULL DEFAULT 'system',
    event_kind  TEXT NOT NULL DEFAULT 'domain_result',
    authoritative INTEGER NOT NULL DEFAULT 1,
    data        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gm_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS dungeons (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    location_q  INTEGER NOT NULL,
    location_r  INTEGER NOT NULL,
    rooms       TEXT NOT NULL,
    connections TEXT NOT NULL,
    data        TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS dungeon_rooms (
    dungeon_id   TEXT NOT NULL REFERENCES dungeons(id),
    room_id      TEXT NOT NULL,
    room_index   INTEGER NOT NULL,
    room_json    TEXT NOT NULL,
    PRIMARY KEY (dungeon_id, room_id)
);

CREATE TABLE IF NOT EXISTS dungeon_connections (
    dungeon_id        TEXT NOT NULL REFERENCES dungeons(id),
    connection_index  INTEGER NOT NULL,
    connection_json   TEXT NOT NULL,
    PRIMARY KEY (dungeon_id, connection_index)
);

CREATE TABLE IF NOT EXISTS practice_log (
    entity_id   TEXT NOT NULL REFERENCES entities(id),
    skill       TEXT NOT NULL,
    ticks       REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (entity_id, skill)
);

CREATE TABLE IF NOT EXISTS skill_mappings (
    verb            TEXT PRIMARY KEY,
    skill           TEXT NOT NULL,
    attribute       TEXT NOT NULL,
    base_difficulty INTEGER NOT NULL DEFAULT 8,
    opposed         INTEGER DEFAULT 0,
    description     TEXT,
    notes           TEXT
);

CREATE TABLE IF NOT EXISTS difficulty_settings (
    key   TEXT PRIMARY KEY,
    grade INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS difficulty_targets (
    name        TEXT PRIMARY KEY,
    target      INTEGER NOT NULL,
    description TEXT
);

CREATE TABLE IF NOT EXISTS disposition_outcomes (
    outcome_key     TEXT PRIMARY KEY,
    delta           INTEGER NOT NULL,
    description     TEXT
);

CREATE TABLE IF NOT EXISTS encounter_weights (
    faction_disposition TEXT NOT NULL,
    encounter_tag       TEXT NOT NULL,
    weight_modifier     INTEGER NOT NULL,
    PRIMARY KEY (faction_disposition, encounter_tag)
);

CREATE TABLE IF NOT EXISTS xp_progression (
    level       INTEGER PRIMARY KEY,
    xp_needed   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS faction_asset_stats (
    asset_type      TEXT PRIMARY KEY,
    category        TEXT NOT NULL,
    min_attribute   INTEGER NOT NULL,
    cost            INTEGER NOT NULL,
    upkeep          INTEGER DEFAULT 0,
    max_hp          INTEGER NOT NULL,
    attack_stat     TEXT,
    counter_stat    TEXT,
    attack_roll     TEXT,
    special         TEXT,
    description     TEXT
);

CREATE TABLE IF NOT EXISTS threads (
    id        TEXT PRIMARY KEY,
    type      TEXT NOT NULL,
    title     TEXT NOT NULL,
    status    TEXT DEFAULT 'active',
    progress  INTEGER DEFAULT 0,
    data      TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS oracle_npcs (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    status    TEXT DEFAULT 'active',
    notes     TEXT,
    entity_id TEXT REFERENCES entities(id)
);

CREATE TABLE IF NOT EXISTS plotlines (
    id        TEXT PRIMARY KEY,
    title     TEXT NOT NULL,
    theme     TEXT NOT NULL,
    status    TEXT DEFAULT 'active',
    scenes    TEXT DEFAULT '[]',
    data      TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS plotline_scenes (
    plotline_id   TEXT NOT NULL REFERENCES plotlines(id),
    scene_index   INTEGER NOT NULL,
    scene_json    TEXT NOT NULL,
    PRIMARY KEY (plotline_id, scene_index)
);

CREATE TABLE IF NOT EXISTS items (
    id     TEXT PRIMARY KEY,
    name   TEXT NOT NULL,
    data   TEXT DEFAULT '{}',
    source TEXT
);

CREATE TABLE IF NOT EXISTS creature_templates (
    id     TEXT PRIMARY KEY,
    name   TEXT NOT NULL,
    data   TEXT DEFAULT '{}',
    source TEXT
);

CREATE TABLE IF NOT EXISTS world_packs (
    pack_id    TEXT NOT NULL,
    version    TEXT NOT NULL,
    load_order INTEGER NOT NULL,
    PRIMARY KEY (pack_id),
    UNIQUE (load_order)
);

CREATE TABLE IF NOT EXISTS pack_overrides (
    pack_id      TEXT NOT NULL,
    qualified_id TEXT NOT NULL,
    data_json    TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (pack_id, qualified_id)
);

CREATE TABLE IF NOT EXISTS ir_records (
    component_type TEXT NOT NULL,
    record_id      TEXT NOT NULL,
    data_json      TEXT NOT NULL,
    source         TEXT NOT NULL DEFAULT 'authored',
    updated_at     TEXT NOT NULL,
    PRIMARY KEY (component_type, record_id)
);

CREATE TABLE IF NOT EXISTS entity_status_effects (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    effect_id       TEXT NOT NULL,
    applied_at_tick INTEGER NOT NULL,
    expires_at_tick INTEGER,
    source          TEXT,
    data_json       TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_entity_status_effects_entity
ON entity_status_effects(entity_id);

CREATE INDEX IF NOT EXISTS idx_entity_status_effects_expiry
ON entity_status_effects(expires_at_tick);

CREATE TABLE IF NOT EXISTS entity_resources (
    entity_id       TEXT NOT NULL,
    resource_id     TEXT NOT NULL,
    current         INTEGER NOT NULL,
    max             INTEGER,
    last_regen_tick INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_id, resource_id)
);

CREATE INDEX IF NOT EXISTS idx_entity_resources_entity
ON entity_resources(entity_id);

CREATE TABLE IF NOT EXISTS entity_transient_modifiers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    modifier_json   TEXT NOT NULL,
    applied_at_tick INTEGER NOT NULL,
    expires_at_tick INTEGER,
    source          TEXT
);

CREATE INDEX IF NOT EXISTS idx_entity_transient_modifiers_entity
ON entity_transient_modifiers(entity_id);

CREATE INDEX IF NOT EXISTS idx_entity_transient_modifiers_expiry
ON entity_transient_modifiers(expires_at_tick);

-- Quest instances (HR-19): per-world quest state owned by QuestService.
CREATE TABLE IF NOT EXISTS quest_instances (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id      TEXT NOT NULL,
    quest_id       TEXT NOT NULL,
    status         TEXT NOT NULL DEFAULT 'accepted',
    accepted_tick  INTEGER NOT NULL,
    completed_tick INTEGER,
    progress_json  TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_quest_instances_entity
ON quest_instances(entity_id);
"##;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_mentions_every_required_table() {
        for table in REQUIRED_TABLES {
            let needle = format!("CREATE TABLE IF NOT EXISTS {table} ");
            assert!(SCHEMA_SQL.contains(&needle), "missing DDL for {table}");
        }
    }

    #[test]
    fn required_tables_count() {
        assert_eq!(REQUIRED_TABLES.len(), 42);
    }
}
