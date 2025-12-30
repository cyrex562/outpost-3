use anyhow::{Context, Result};
use rusqlite::params;

use crate::db::pool::DbPool;
use super::GameEvent;

pub struct EventStore {
    pool: DbPool,
}

impl EventStore {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn save_event(&self, event: &GameEvent) -> Result<()> {
        let conn = self.pool.get()
            .context("Failed to get database connection")?;

        let event_data = serde_json::to_string(&event.event_type)
            .context("Failed to serialize event")?;

        conn.execute(
            "INSERT INTO events (event_id, timestamp, turn_number, event_type, event_data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                event.event_id,
                event.timestamp.to_rfc3339(),
                event.turn_number,
                serde_json::to_string(&event.event_type).unwrap(),
                event_data,
            ],
        ).context("Failed to insert event")?;

        Ok(())
    }

    pub fn get_all_events(&self) -> Result<Vec<GameEvent>> {
        let conn = self.pool.get()
            .context("Failed to get database connection")?;

        let mut stmt = conn.prepare(
            "SELECT event_id, timestamp, turn_number, event_data FROM events ORDER BY event_id ASC"
        )?;

        let events = stmt.query_map([], |row| {
            let event_id: u64 = row.get(0)?;
            let timestamp_str: String = row.get(1)?;
            let turn_number: u64 = row.get(2)?;
            let event_data: String = row.get(3)?;

            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap();

            let event_type = serde_json::from_str(&event_data).unwrap();

            Ok(GameEvent {
                event_id,
                timestamp,
                turn_number,
                event_type,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    pub fn get_next_event_id(&self) -> Result<u64> {
        let conn = self.pool.get()
            .context("Failed to get database connection")?;

        let count: u64 = conn.query_row(
            "SELECT COALESCE(MAX(event_id), 0) + 1 FROM events",
            [],
            |row| row.get(0)
        )?;

        Ok(count)
    }
}
