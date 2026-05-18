//! Turn processing for the simulation
//!
//! Note: This module is being refactored for the V5 event-sourced architecture.
//! The TurnProcessor will be reimplemented as a pure function that takes GameState
//! and returns Vec<GameEvent> without any I/O dependencies.

use crate::events::EventType;

/// Placeholder for turn processing logic
/// TODO: Implement as pure function (GameState) -> Vec<GameEvent>
pub struct TurnProcessor {
    // Placeholder
}

impl TurnProcessor {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TurnProcessor {
    fn default() -> Self {
        Self::new()
    }
}
