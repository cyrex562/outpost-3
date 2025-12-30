use anyhow::{Context, Result};
use rusqlite::Connection;

use super::pool::DbPool;
use super::schema::*;

pub fn run_migrations(pool: &DbPool) -> Result<()> {
    let conn = pool.get().context("Failed to get database connection")?;

    create_tables(&conn)?;
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

    // Create indexes
    conn.execute("CREATE INDEX IF NOT EXISTS idx_events_turn ON events(turn_number)", [])
        .context("Failed to create events index")?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_buildings_colony ON buildings(colony_id)", [])
        .context("Failed to create buildings index")?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_stockpiles_colony ON resource_stockpiles(colony_id)", [])
        .context("Failed to create resource_stockpiles index")?;

    Ok(())
}

fn initialize_game_state(conn: &Connection) -> Result<()> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM game_state",
        [],
        |row| row.get(0)
    )?;

    if count == 0 {
        conn.execute(
            "INSERT INTO game_state (id, current_turn, credits) VALUES (1, 1, 10000)",
            []
        )?;
    }

    Ok(())
}
