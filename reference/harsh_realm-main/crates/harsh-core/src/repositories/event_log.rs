//! Event-log persistence handler.
//!
//! Ported from the `EventLogger` in `src/harsh_realm/events.py` (deferred there
//! until the persistence layer existed). Writes each event to `event_log`.

use crate::db::WorldDatabase;
use crate::events::{describe_event_for_log, GameEvent};

/// Persists game events to the `event_log` table.
pub struct EventLogger<'a> {
    db: &'a WorldDatabase,
}

impl<'a> EventLogger<'a> {
    /// Create a logger over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        EventLogger { db }
    }

    /// Write one event to `event_log`.
    pub fn handle(&self, event: &GameEvent) -> Result<(), String> {
        let meta = describe_event_for_log(event);
        let event_kind = meta
            .get("event_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("domain_result")
            .to_string();
        let authoritative: i64 = meta
            .get("authoritative")
            .and_then(|v| v.as_bool())
            .map(|b| b as i64)
            .unwrap_or(1);
        let data_json = serde_json::to_string(&event.data).unwrap_or_else(|_| "{}".to_string());
        self.db.execute(
            "INSERT INTO event_log (tick, event_id, event_type, source, event_kind, authoritative, data, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                &event.tick,
                &event.id,
                &event.event_type,
                &event.source,
                &event_kind,
                &authoritative,
                &data_json,
                &event.timestamp,
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::JsonObject;

    #[test]
    fn writes_event_row() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let logger = EventLogger::new(&db);
        let ev = GameEvent::new(3, "combat.attack", JsonObject::new());
        logger.handle(&ev).unwrap();
        let row = db
            .fetch_one("SELECT tick, event_type, event_kind FROM event_log", &[])
            .unwrap()
            .unwrap();
        assert_eq!(row.get("tick").unwrap().as_i64(), Some(3));
        assert_eq!(row.get("event_type").unwrap().as_str(), Some("combat.attack"));
        assert_eq!(row.get("event_kind").unwrap().as_str(), Some("domain_result"));
    }
}
