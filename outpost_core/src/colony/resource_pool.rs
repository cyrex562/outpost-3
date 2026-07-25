//! Colony-local resource pool (issue #304).
//!
//! Holds the resources a colony produces and consumes **in place** — power,
//! housing capacity, research output — as opposed to the tradeable commodities
//! in [`crate::colony::ColonyPool`].
//!
//! The separation is the point. Trade routes, haulers, and supply packages are
//! only ever handed a `ColonyPool`, so a colony resource cannot be shipped: it
//! isn't reachable from the code that would ship it. Before this existed,
//! `CommodityDef::tradeable` was authored on every commodity and read by
//! nothing, so `power`, `housing`, and `research` all flowed over trade routes
//! despite being marked non-tradeable.
//!
//! Resources do **not** persist across sols. [`ColonyResourcePool::clear`] runs
//! at the end of every colony sol, so the pool always reports *this* sol's
//! throughput (or standing capacity) rather than an accumulated total. That
//! also fixes a pair of unbounded-accumulation bugs: `power` netted a positive
//! surplus every sol and banked it forever, and `housing` — a capacity check
//! that consumes nothing — grew by a full habitat's worth every sol.

use std::collections::HashMap;

/// Per-sol amounts of each colony-local resource.
///
/// Deliberately simpler than [`crate::colony::ColonyPool`]: no capacity, no
/// storage weight, and no cross-turn delta, because none of those mean anything
/// for a quantity that is reset every sol.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ColonyResourcePool {
    /// Map from resource id to the amount available this sol.
    amounts: HashMap<String, f64>,
}

impl ColonyResourcePool {
    /// Create an empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Amount of `resource_id` available this sol, or `0.0` if absent.
    #[must_use]
    pub fn amount(&self, resource_id: &str) -> f64 {
        self.amounts.get(resource_id).copied().unwrap_or(0.0)
    }

    /// Add `qty` of `resource_id`. Negative quantities are ignored — callers
    /// remove via [`Self::withdraw`], which reports what was actually available.
    pub fn deposit(&mut self, resource_id: &str, qty: f64) {
        if qty <= 0.0 {
            return;
        }
        *self.amounts.entry(resource_id.to_owned()).or_insert(0.0) += qty;
    }

    /// Remove up to `qty`, returning the amount actually removed.
    ///
    /// Never goes negative: a colony can't run a power deficit into debt, it
    /// simply doesn't get the power, and the caller scales its output down.
    pub fn withdraw(&mut self, resource_id: &str, qty: f64) -> f64 {
        if qty <= 0.0 {
            return 0.0;
        }
        let entry = self.amounts.entry(resource_id.to_owned()).or_insert(0.0);
        let removed = qty.min(*entry);
        *entry -= removed;
        removed
    }

    /// Drop every amount to zero — called at the end of each colony sol.
    ///
    /// Entries are removed rather than zeroed so a resource that stops being
    /// produced disappears from the readout instead of lingering at 0.
    pub fn clear(&mut self) {
        self.amounts.clear();
    }

    /// Iterate over `(resource_id, amount)` pairs present this sol.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f64)> {
        self.amounts.iter().map(|(k, v)| (k.as_str(), *v))
    }

    /// Whether the pool holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.amounts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_and_withdraw_track_the_available_amount() {
        let mut pool = ColonyResourcePool::new();
        assert_eq!(pool.amount("power"), 0.0);

        pool.deposit("power", 24.0);
        assert!((pool.amount("power") - 24.0).abs() < f64::EPSILON);

        let taken = pool.withdraw("power", 10.0);
        assert!((taken - 10.0).abs() < f64::EPSILON);
        assert!((pool.amount("power") - 14.0).abs() < f64::EPSILON);
    }

    #[test]
    fn withdraw_is_capped_at_what_is_available_and_never_goes_negative() {
        let mut pool = ColonyResourcePool::new();
        pool.deposit("power", 5.0);

        let taken = pool.withdraw("power", 50.0);
        assert!((taken - 5.0).abs() < f64::EPSILON, "only 5 was available");
        assert_eq!(pool.amount("power"), 0.0, "must not go negative");
    }

    #[test]
    fn non_positive_quantities_are_ignored() {
        let mut pool = ColonyResourcePool::new();
        pool.deposit("power", -10.0);
        assert_eq!(pool.amount("power"), 0.0, "negative deposit is a no-op");
        assert_eq!(pool.withdraw("power", -10.0), 0.0);
    }

    #[test]
    fn clear_empties_the_pool_so_nothing_carries_into_the_next_sol() {
        let mut pool = ColonyResourcePool::new();
        pool.deposit("power", 24.0);
        pool.deposit("housing", 110.0);

        pool.clear();

        assert!(pool.is_empty());
        assert_eq!(pool.amount("power"), 0.0);
        assert_eq!(pool.amount("housing"), 0.0);
        assert_eq!(pool.iter().count(), 0, "entries are removed, not zeroed");
    }
}
