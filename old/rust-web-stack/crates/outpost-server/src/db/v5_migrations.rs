/// Versioned migration runner for V5 schema.
///
/// Each migration is a (version, name, sql) tuple.
/// Migrations are applied in order and tracked in `schema_migrations`.
/// Adding a new migration is safe: existing versions are skipped.

use anyhow::{Context, Result};
use rusqlite::Connection;
use tracing::info;

use super::v5_schema::*;

/// All V5 schema migrations in order.
/// New migrations go at the END of this list.
static MIGRATIONS: &[(u32, &str, &str)] = &[
    (
        1,
        "create_schema_migrations",
        SCHEMA_MIGRATIONS,
    ),
    (
        2,
        "create_game_state_v5",
        GAME_STATE_V5,
    ),
    (
        3,
        "create_content_building_defs",
        CONTENT_BUILDING_DEFS,
    ),
    (
        4,
        "create_content_resource_defs",
        CONTENT_RESOURCE_DEFS,
    ),
    (
        5,
        "create_content_event_defs",
        CONTENT_EVENT_DEFS,
    ),
    (
        6,
        "create_content_tech_defs",
        CONTENT_TECH_DEFS,
    ),
    (
        7,
        "create_game_event_log",
        GAME_EVENT_LOG,
    ),
    (
        8,
        "create_indexes",
        // Run all index DDL as separate statements below
        "",
    ),
    (
        9,
        "create_content_recipes",
        CONTENT_RECIPES,
    ),
    (
        10,
        "create_content_recipes_index",
        // Run separately in the version-10 branch
        "",
    ),
];

/// Apply all pending V5 migrations to the given connection.
///
/// This function is idempotent — safe to call on every startup.
pub fn run_v5_migrations(conn: &Connection) -> Result<()> {
    // Ensure the tracking table exists first (bootstrap).
    conn.execute_batch(SCHEMA_MIGRATIONS)
        .context("Failed to create schema_migrations table")?;

    for (version, name, sql) in MIGRATIONS {
        let already_applied: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
                rusqlite::params![version],
                |row| row.get::<_, i64>(0),
            )
            .context("Failed to query schema_migrations")?
            > 0;

        if already_applied {
            continue;
        }

        info!("Applying V5 migration {}: {}", version, name);

        // Version 8 is the index bundle — run them separately.
        if *version == 8 {
            conn.execute_batch(GAME_EVENT_LOG_IDX)
                .context("Failed to create game_event_log index")?;
            conn.execute_batch(CONTENT_BUILDING_DEFS_IDX)
                .context("Failed to create content_building_defs index")?;
            conn.execute_batch(CONTENT_RESOURCE_DEFS_IDX)
                .context("Failed to create content_resource_defs index")?;
        } else if *version == 10 {
            conn.execute_batch(CONTENT_RECIPES_IDX)
                .context("Failed to create content_recipes index")?;
        } else if !sql.is_empty() {
            conn.execute_batch(sql)
                .with_context(|| format!("Failed to apply migration {}: {}", version, name))?;
        }

        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, datetime('now'))",
            rusqlite::params![version, name],
        )
        .with_context(|| format!("Failed to record migration {}", version))?;

        info!("Migration {} ({}) applied successfully", version, name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn in_memory() -> Connection {
        Connection::open_in_memory().unwrap()
    }

    #[test]
    fn migrations_run_successfully() {
        let conn = in_memory();
        run_v5_migrations(&conn).unwrap();
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = in_memory();
        run_v5_migrations(&conn).unwrap();
        // Running again should not fail.
        run_v5_migrations(&conn).unwrap();
    }

    #[test]
    fn all_tables_created() {
        let conn = in_memory();
        run_v5_migrations(&conn).unwrap();

        let tables = [
            "schema_migrations",
            "game_state_v5",
            "content_building_defs",
            "content_resource_defs",
            "content_event_defs",
            "content_tech_defs",
            "game_event_log",
        ];

        for table in &tables {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Table '{}' was not created", table);
        }
    }
}
