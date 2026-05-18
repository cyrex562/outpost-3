use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use tracing::info;

use super::pool::DbPool;
use super::schema::*;

pub fn run_migrations(pool: &DbPool) -> Result<()> {
    let mut conn = pool.get().context("Failed to get database connection")?;

    create_tables(&conn)?;
    run_schema_migrations(&mut conn)?;
    initialize_game_state(&conn)?;

    Ok(())
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute(EVENTS_TABLE, [])
        .context("Failed to create events table")?;

    conn.execute(COLONIES_TABLE, [])
        .context("Failed to create colonies table")?;

    conn.execute(BUILDINGS_TABLE, [])
        .context("Failed to create buildings table")?;

    conn.execute(PLANETS_TABLE, [])
        .context("Failed to create planets table")?;

    conn.execute(GAME_STATE_TABLE, [])
        .context("Failed to create game_state table")?;

    conn.execute(RESOURCE_STOCKPILES_TABLE, [])
        .context("Failed to create resource_stockpiles table")?;

    conn.execute(STAR_SYSTEMS_TABLE, [])
        .context("Failed to create star_systems table")?;

    conn.execute(CELESTIAL_BODIES_TABLE, [])
        .context("Failed to create celestial_bodies table")?;

    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_turn ON events(turn_number)",
        [],
    )
    .context("Failed to create events index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_buildings_colony ON buildings(colony_id)",
        [],
    )
    .context("Failed to create buildings index")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_stockpiles_colony ON resource_stockpiles(colony_id)",
        [],
    )
    .context("Failed to create resource_stockpiles index")?;

    Ok(())
}

fn run_schema_migrations(conn: &mut Connection) -> Result<()> {
    // Add level column to buildings table if it doesn't exist
    let has_level_column: Result<i64, _> = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('buildings') WHERE name='level'",
        [],
        |row| row.get(0),
    );

    if let Ok(count) = has_level_column {
        if count == 0 {
            conn.execute(
                "ALTER TABLE buildings ADD COLUMN level INTEGER NOT NULL DEFAULT 1",
                [],
            )
            .context("Failed to add level column to buildings table")?;
        }
    }

    // Add new resource types (minerals + energy) for existing colonies with default 0 stock
    run_add_new_resources_v2(conn)?;

    Ok(())
}

const NEW_RESOURCES_V2: &[&str] = &[
    // Energy
    "Solar Power",
    "Nuclear Fuel",
    "Battery Pack",
    "Hydrogen Cell",
    // Minerals
    "Nickel",
    "Cobalt",
    "Zinc",
    "Silver",
    "Gold",
    "Lithium",
    "Graphite",
];

fn run_add_new_resources_v2(conn: &mut Connection) -> Result<()> {
    info!("Running migration: add_new_resources_v2");

    let tx = conn.transaction()?;
    let mut affected_rows: usize = 0;

    let colony_ids: Vec<i64> = {
        let mut stmt = tx.prepare("SELECT colony_id FROM colonies")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };

    for colony_id in colony_ids {
        for res in NEW_RESOURCES_V2 {
            let inserted = tx.execute(
                "INSERT INTO resource_stockpiles (colony_id, resource_type, quantity)
                 SELECT ?, ?, 0
                 WHERE NOT EXISTS (
                     SELECT 1 FROM resource_stockpiles WHERE colony_id = ? AND resource_type = ?
                 )",
                params![colony_id, *res, colony_id, *res],
            )?;
            affected_rows += inserted as usize;
        }
    }

    tx.commit()?;
    info!("Migration complete: {} rows", affected_rows);
    Ok(())
}

fn initialize_game_state(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM game_state", [], |row| row.get(0))?;

    if count == 0 {
        conn.execute(
            "INSERT INTO game_state (id, current_turn, credits) VALUES (1, 1, 10000)",
            [],
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup_in_memory_conn() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        create_tables(&conn)?;
        // seed a colony so the migration has a target
        conn.execute(
            "INSERT INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level)
             VALUES (1, 1, 'Test', '2026-01-01', 100, 75.0, 0.0)",
            [],
        )?;
        Ok(conn)
    }

    #[test]
    fn add_new_resources_v2_is_idempotent() {
        let mut conn = setup_in_memory_conn().unwrap();

        // First run inserts all new resources
        run_add_new_resources_v2(&mut conn).unwrap();
        let set: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("SELECT resource_type FROM resource_stockpiles WHERE colony_id = 1")
                .unwrap();
            let rows = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
            rows.map(|r| r.unwrap()).collect()
        };
        let expected: std::collections::HashSet<String> =
            NEW_RESOURCES_V2.iter().map(|s| s.to_string()).collect();
        for needed in expected.iter() {
            assert!(
                set.contains(needed),
                "Missing resource {} after first migration",
                needed
            );
        }
        let count_first = set.intersection(&expected).count();

        // Second run should not duplicate entries
        run_add_new_resources_v2(&mut conn).unwrap();
        let set_after: std::collections::HashSet<String> = {
            let mut stmt = conn
                .prepare("SELECT resource_type FROM resource_stockpiles WHERE colony_id = 1")
                .unwrap();
            let rows = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
            rows.map(|r| r.unwrap()).collect()
        };
        let count_second = set_after.intersection(&expected).count();
        assert_eq!(count_first, count_second, "Migration should be idempotent");
    }

    #[test]
    fn add_new_resources_v2_defaults_to_zero_quantity() {
        let mut conn = setup_in_memory_conn().unwrap();
        run_add_new_resources_v2(&mut conn).unwrap();

        let mut stmt = conn
            .prepare("SELECT resource_type, quantity FROM resource_stockpiles WHERE colony_id = 1")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                let r: String = row.get(0)?;
                let q: i64 = row.get(1)?;
                Ok((r, q))
            })
            .unwrap();

        let mut zeroed = 0;
        for row in rows {
            let (_r, q) = row.unwrap();
            assert!(q >= 0, "Quantities should not be negative");
            if q == 0 {
                zeroed += 1;
            }
        }
        assert!(
            zeroed >= NEW_RESOURCES_V2.len(),
            "New resources should be seeded at zero"
        );
    }
}
