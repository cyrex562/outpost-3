use crate::events::{EventType, EventStore};
use anyhow::Result;

pub struct TurnProcessor {
    event_store: EventStore,
}

impl TurnProcessor {
    pub fn new(event_store: EventStore) -> Self {
        Self { event_store }
    }

    pub fn process_turn(&self, turn_number: u64) -> Result<Vec<EventType>> {
        let mut events = vec![];

        // Turn processing logic will go here
        // For now, just advance the turn
        events.push(EventType::TurnAdvanced { turn_number: turn_number + 1 });

        Ok(events)
    }
}
