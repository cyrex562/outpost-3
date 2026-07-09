//! Export/import table registry.
//!
//! Ported from `src/harsh_realm/api/editor/transfer.py`. The HTTP route
//! declarations belong to the B9 web-host layer and are not included here.

/// Tables that can be exported from a world database, keyed by table name.
/// The value is the SELECT statement used to fetch rows for export.
pub const EXPORT_TABLES: &[(&str, &str)] = &[
    ("skill_mappings", "SELECT * FROM skill_mappings"),
    ("difficulty_targets", "SELECT * FROM difficulty_targets"),
    ("disposition_outcomes", "SELECT * FROM disposition_outcomes"),
    ("encounter_weights", "SELECT * FROM encounter_weights"),
    ("faction_asset_stats", "SELECT * FROM faction_asset_stats"),
    (
        "entities",
        "SELECT id, name, entity_type, location_q, location_r, alive, data FROM entities",
    ),
    (
        "cells",
        "SELECT q, r, terrain, features, explored, description, faction_id, data FROM cells",
    ),
    ("factions", "SELECT * FROM factions"),
    ("faction_assets", "SELECT * FROM faction_assets"),
    ("faction_relations", "SELECT * FROM faction_relations"),
    ("reputation", "SELECT * FROM reputation"),
    ("dungeons", "SELECT * FROM dungeons"),
    ("gm_state", "SELECT * FROM gm_state"),
    ("world_meta", "SELECT * FROM world_meta"),
    ("event_log", "SELECT * FROM event_log"),
    ("threads", "SELECT * FROM threads"),
    ("oracle_npcs", "SELECT * FROM oracle_npcs"),
    ("plotlines", "SELECT * FROM plotlines"),
    ("practice_log", "SELECT * FROM practice_log"),
    ("random_tables", "SELECT * FROM random_tables"),
    ("items", "SELECT * FROM items"),
    ("creature_templates", "SELECT * FROM creature_templates"),
];

/// Return the SELECT query for a named export table, or None if not supported.
pub fn export_query(table: &str) -> Option<&'static str> {
    EXPORT_TABLES.iter().find(|(k, _)| *k == table).map(|(_, q)| *q)
}

/// Return whether a table name is supported for export.
pub fn is_exportable(table: &str) -> bool {
    export_query(table).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_mappings_exportable() {
        assert!(is_exportable("skill_mappings"));
    }

    #[test]
    fn unknown_table_not_exportable() {
        assert!(!is_exportable("nonexistent_table"));
    }

    #[test]
    fn export_query_returns_select() {
        let q = export_query("cells").unwrap();
        assert!(q.starts_with("SELECT"));
    }
}
