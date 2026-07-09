//! Outpost 3 simulation core.
//!
//! A pure Rust library crate with zero I/O or framework dependencies.
//! All game logic lives here and is driven through the [`GameEngine`] interface.
//!
//! # Architecture
//!
//! The only mutation point from outside this crate is:
//! ```ignore
//! GameEngine::apply(cmd: Command) -> Result<Vec<Event>, EngineError>
//! ```
//!
//! Module layout mirrors the major design systems described in `docs/DESIGN.md`:
//! - [`turn`]       — two-cadence turn model (colony-sol, strategic-month)
//! - [`colony`]     — pooled commodities, slots, production chains
//! - [`content`]    — content-pack loading for authored data
//! - [`population`] — aggregate population pool, stability, labour derivation

#![warn(missing_docs)]

pub mod colony;
pub mod content;
pub mod population;
pub mod turn;

use thiserror::Error;

use turn::{GameState, TurnProcessor};

/// Default RNG seed used when constructing a [`GameEngine`] with [`GameEngine::new`].
pub const DEFAULT_SEED: u64 = 0;

/// A command submitted to the engine from the outside world.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Command {
    /// Advance the simulation by one colony-sol turn.
    AdvanceColonySol,
    /// Advance the simulation by one strategic-month turn (manual override).
    AdvanceStrategicMonth,
}

/// An event produced by the engine in response to a [`Command`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Event {
    /// A colony-sol turn completed.
    ColonySolAdvanced {
        /// The new colony-sol counter value after advancement.
        sol: u64,
    },
    /// A strategic-month turn completed.
    StrategicMonthAdvanced {
        /// The new strategic-month counter value after advancement.
        month: u64,
    },
}

/// All errors that the engine can return.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// A command was submitted while the engine was in an invalid state.
    #[error("invalid engine state: {0}")]
    InvalidState(String),
}

/// The top-level game engine.
///
/// Wraps [`GameState`] and [`TurnProcessor`] behind the single `apply` interface.
/// All external callers (CLI, web host, tests) drive the simulation through
/// [`GameEngine::apply`].
#[derive(Debug)]
pub struct GameEngine {
    /// In-memory live game state.
    pub state: GameState,
    /// Turn processor responsible for cadence bookkeeping.
    processor: TurnProcessor,
}

impl GameEngine {
    /// Create a new engine with the default seed.
    #[must_use]
    pub fn new() -> Self {
        Self::with_seed(DEFAULT_SEED)
    }

    /// Create a new engine with an explicit RNG seed.
    ///
    /// Use a fixed seed in tests to get deterministic results.
    #[must_use]
    pub fn with_seed(seed: u64) -> Self {
        Self {
            state: GameState::new(),
            processor: TurnProcessor::new(seed),
        }
    }

    /// Apply a [`Command`] and return the resulting [`Event`]s.
    ///
    /// This is the **only** mutation point from outside `outpost_core`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError`] when the command cannot be applied in the
    /// current engine state.
    pub fn apply(&mut self, cmd: &Command) -> Result<Vec<Event>, EngineError> {
        match *cmd {
            Command::AdvanceColonySol => {
                let outcome = self.processor.advance(&mut self.state);
                let mut events = vec![Event::ColonySolAdvanced { sol: outcome.sol }];
                if outcome
                    .cadences_fired
                    .contains(&turn::TurnCadence::StrategicMonth)
                {
                    events.push(Event::StrategicMonthAdvanced {
                        month: outcome.month,
                    });
                }
                Ok(events)
            }
            Command::AdvanceStrategicMonth => {
                self.state.month += 1;
                Ok(vec![Event::StrategicMonthAdvanced {
                    month: self.state.month,
                }])
            }
        }
    }

    /// Return the current colony-sol counter.
    #[must_use]
    pub fn sol(&self) -> u64 {
        self.state.sol
    }

    /// Return the current strategic-month counter.
    #[must_use]
    pub fn month(&self) -> u64 {
        self.state.month
    }
}

impl Default for GameEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_starts_at_zero() {
        let engine = GameEngine::new();
        assert_eq!(engine.sol(), 0);
        assert_eq!(engine.month(), 0);
    }

    #[test]
    fn advance_colony_sol_increments_counter() {
        let mut engine = GameEngine::new();
        let events = engine.apply(&Command::AdvanceColonySol).unwrap();
        assert_eq!(engine.sol(), 1);
        assert!(matches!(events[0], Event::ColonySolAdvanced { sol: 1 }));
    }

    #[test]
    fn advance_strategic_month_increments_counter() {
        let mut engine = GameEngine::new();
        let events = engine.apply(&Command::AdvanceStrategicMonth).unwrap();
        assert_eq!(engine.month(), 1);
        assert!(matches!(
            events[0],
            Event::StrategicMonthAdvanced { month: 1 }
        ));
    }

    #[test]
    fn multiple_advances_accumulate() {
        let mut engine = GameEngine::new();
        for _ in 0..5 {
            engine.apply(&Command::AdvanceColonySol).unwrap();
        }
        assert_eq!(engine.sol(), 5);
    }

    #[test]
    fn strategic_month_fires_automatically_after_30_sols() {
        let mut engine = GameEngine::with_seed(42);
        let mut month_events = 0usize;
        for _ in 0..30 {
            let events = engine.apply(&Command::AdvanceColonySol).unwrap();
            month_events += events
                .iter()
                .filter(|e| matches!(e, Event::StrategicMonthAdvanced { .. }))
                .count();
        }
        assert_eq!(engine.sol(), 30);
        assert_eq!(month_events, 1);
        assert_eq!(engine.month(), 1);
    }

    #[test]
    fn engine_is_deterministic_for_fixed_seed() {
        let run = || {
            let mut engine = GameEngine::with_seed(7777);
            for _ in 0..60 {
                engine.apply(&Command::AdvanceColonySol).unwrap();
            }
            (engine.sol(), engine.month())
        };
        assert_eq!(run(), run());
    }
}
