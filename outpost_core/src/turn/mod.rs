//! Turn model — two-cadence turn processing.
//!
//! Outpost 3 uses two interleaved turn cadences (see `docs/DESIGN.md §4`):
//! - **Colony-sol**: the shorter operational cadence for day-to-day colony
//!   management (production, labour allocation, events).
//! - **Strategic-month**: the longer cadence for fleet movement, diplomacy,
//!   and star-system-scale actions.
//!
//! This module will own the turn processor logic once game mechanics are
//! implemented (issues #8+). For now it is a typed stub.

/// Identifies which turn cadence is being processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TurnCadence {
    /// Day-to-day colony operations cadence.
    ColonySol,
    /// Star-system-scale strategic cadence.
    StrategicMonth,
}

/// Placeholder turn context passed through the processing pipeline.
///
/// Will be expanded with colony state, content packs, and RNG seed once
/// game mechanics are introduced.
#[derive(Debug, Clone)]
pub struct TurnContext {
    /// Which cadence this turn belongs to.
    pub cadence: TurnCadence,
    /// Monotonically increasing turn number within the cadence.
    pub turn_number: u64,
}

impl TurnContext {
    /// Construct a new turn context.
    #[must_use]
    pub fn new(cadence: TurnCadence, turn_number: u64) -> Self {
        Self {
            cadence,
            turn_number,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_context_stores_cadence_and_number() {
        let ctx = TurnContext::new(TurnCadence::ColonySol, 42);
        assert_eq!(ctx.cadence, TurnCadence::ColonySol);
        assert_eq!(ctx.turn_number, 42);
    }
}
