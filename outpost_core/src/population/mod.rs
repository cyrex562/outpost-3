//! Population subsystem — aggregate pool, stability, and labour derivation.
//!
//! Population in Outpost 3 is an aggregate scalar per colony, not a
//! per-citizen simulation (see `docs/DESIGN.md §6`). This module will own:
//!
//! - The population pool and its growth/attrition model.
//! - The stability scalar and its modifiers.
//! - Labour derivation: how many worker-units the colony can assign each turn.

/// Aggregate population state for a single colony.
///
/// All fields are scalars derived from the population pool; individual
/// citizens are not modelled.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Population {
    /// Total number of colonists (aggregate head-count).
    pub count: u64,
    /// Stability scalar in the range `[0.0, 1.0]`.
    ///
    /// At 1.0 the colony is fully stable; at 0.0 it is on the verge of
    /// collapse. Values outside this range are clamped by the engine.
    pub stability: f64,
}

impl Population {
    /// Create a starting population with the given head-count and full stability.
    #[must_use]
    pub fn new(count: u64) -> Self {
        Self {
            count,
            stability: 1.0,
        }
    }

    /// Derive the number of available labour units this turn.
    ///
    /// Labour is proportional to population × stability. The exact formula
    /// will be tuned via the balance harness; this stub uses a simple product.
    #[must_use]
    pub fn available_labour(&self) -> u64 {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let labour = (self.count as f64 * self.stability.clamp(0.0, 1.0)) as u64;
        labour
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_population_has_full_stability() {
        let p = Population::new(100);
        assert!((p.stability - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn full_stability_labour_equals_count() {
        let p = Population::new(50);
        assert_eq!(p.available_labour(), 50);
    }

    #[test]
    fn zero_stability_yields_no_labour() {
        let p = Population {
            count: 100,
            stability: 0.0,
        };
        assert_eq!(p.available_labour(), 0);
    }

    #[test]
    fn half_stability_halves_labour() {
        let p = Population {
            count: 100,
            stability: 0.5,
        };
        assert_eq!(p.available_labour(), 50);
    }
}
