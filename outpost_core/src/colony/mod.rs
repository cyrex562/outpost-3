//! Colony simulation — pooled commodities, building slots, production chains.
//!
//! A colony is the primary player-managed entity in Outpost 3
//! (see `docs/DESIGN.md §5, §6, §7`). This module owns:
//!
//! - The commodity store (pooled resources per colony) — [`ColonyPool`].
//! - Building slots and their labour assignments.
//! - Production-chain resolution (inputs → outputs per turn).
//!
//! All types here are pure data structures with no I/O; they are driven by
//! [`crate::GameEngine::apply`] through the turn processor.

pub mod pool;

pub use pool::{ColonyPool, RecipeOutcome, StockpileDelta};

/// Unique identifier for a colony.
pub type ColonyId = uuid::Uuid;

/// Stub representation of a colony.
///
/// Fields will be expanded once commodity and building mechanics are
/// implemented (issues #8+).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Colony {
    /// Stable identifier for this colony.
    pub id: ColonyId,
    /// Human-readable colony name.
    pub name: String,
    /// Pooled commodity stockpile for this colony.
    pub pool: ColonyPool,
}

impl Colony {
    /// Create a new colony with the given name, a random ID, and an empty pool.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            pool: ColonyPool::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colony_new_assigns_unique_ids() {
        let a = Colony::new("Alpha");
        let b = Colony::new("Beta");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn colony_stores_name() {
        let c = Colony::new("Gamma Station");
        assert_eq!(c.name, "Gamma Station");
    }

    #[test]
    fn colony_starts_with_empty_pool() {
        let c = Colony::new("Delta");
        assert_eq!(c.pool.amount("water"), 0.0);
    }
}
