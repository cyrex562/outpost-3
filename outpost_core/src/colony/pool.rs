//! Colony commodity pool — shared stockpile with capacity and delta tracking.
//!
//! Commodities in Outpost 3 are colony-pooled: all buildings draw from and
//! add to a single shared stockpile. There is no intra-colony routing.
//! See `docs/DESIGN.md §5, §7`.

use std::collections::HashMap;

use crate::content::RecipeDef;

/// Per-commodity entry in a colony stockpile.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StockpileEntry {
    /// Current amount stored.
    pub amount: f64,
    /// Maximum amount that can be stored; `f64::INFINITY` means unlimited.
    pub capacity: f64,
    /// Net change last turn (positive = surplus, negative = deficit).
    pub delta: f64,
}

impl StockpileEntry {
    /// Create a new entry with zero amount and unlimited capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            amount: 0.0,
            capacity: f64::INFINITY,
            delta: 0.0,
        }
    }
}

impl Default for StockpileEntry {
    fn default() -> Self {
        Self::new()
    }
}

/// Colony-pooled commodity stockpile.
///
/// All buildings in a colony draw from and contribute to this single pool.
/// No intra-colony routing: the interesting question is whether the pool
/// stays balanced, not how goods move between buildings.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ColonyPool {
    /// Map from commodity id to its stockpile entry.
    entries: HashMap<String, StockpileEntry>,
}

impl ColonyPool {
    /// Create a new empty pool.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the current amount of a commodity, or `0.0` if not present.
    #[must_use]
    pub fn amount(&self, commodity_id: &str) -> f64 {
        self.entries.get(commodity_id).map_or(0.0, |e| e.amount)
    }

    /// Return the capacity for a commodity, or `f64::INFINITY` if unset.
    #[must_use]
    pub fn capacity(&self, commodity_id: &str) -> f64 {
        self.entries
            .get(commodity_id)
            .map_or(f64::INFINITY, |e| e.capacity)
    }

    /// Return the last-turn delta for a commodity, or `0.0` if not tracked.
    #[must_use]
    pub fn delta(&self, commodity_id: &str) -> f64 {
        self.entries.get(commodity_id).map_or(0.0, |e| e.delta)
    }

    /// Set the capacity for a commodity.
    pub fn set_capacity(&mut self, commodity_id: &str, capacity: f64) {
        self.entry_mut(commodity_id).capacity = capacity;
    }

    /// Add `amount` of a commodity to the pool, capped at capacity.
    ///
    /// Returns the amount actually added (may be less than requested if near cap).
    pub fn deposit(&mut self, commodity_id: &str, amount: f64) -> f64 {
        let entry = self.entry_mut(commodity_id);
        let space = (entry.capacity - entry.amount).max(0.0);
        let added = amount.min(space);
        entry.amount += added;
        entry.delta += added;
        added
    }

    /// Remove `amount` of a commodity from the pool.
    ///
    /// Returns the amount actually removed (may be less if supply runs out).
    pub fn withdraw(&mut self, commodity_id: &str, amount: f64) -> f64 {
        let entry = self.entry_mut(commodity_id);
        let removed = amount.min(entry.amount);
        entry.amount -= removed;
        entry.delta -= removed;
        removed
    }

    /// Reset all delta trackers to zero (call once per turn).
    pub fn reset_deltas(&mut self) {
        for entry in self.entries.values_mut() {
            entry.delta = 0.0;
        }
    }

    /// Iterate over all commodity ids present in the pool.
    pub fn commodity_ids(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    /// Get or create a mutable entry for a commodity.
    fn entry_mut(&mut self, commodity_id: &str) -> &mut StockpileEntry {
        self.entries.entry(commodity_id.to_owned()).or_default()
    }

    /// Apply one production cycle of `recipe` to this pool.
    ///
    /// Returns [`RecipeOutcome::Produced`] with per-commodity deltas on
    /// success, or [`RecipeOutcome::Stalled`] with the first missing
    /// commodity if any input is in shortage.
    ///
    /// # Shortage semantics
    ///
    /// If the pool holds less than the required quantity of *any* input,
    /// the recipe is stalled: **no** inputs are consumed and **no** outputs
    /// are produced. This matches the "all-or-nothing" batch behaviour used
    /// in the C# `ResourceStore` behavioural spec.
    pub fn apply_recipe(&mut self, recipe: &RecipeDef) -> RecipeOutcome {
        // Check all inputs are satisfied before modifying anything.
        for ingredient in &recipe.inputs {
            if self.amount(&ingredient.id) < ingredient.quantity {
                return RecipeOutcome::Stalled {
                    missing_commodity: ingredient.id.clone(),
                    required: ingredient.quantity,
                    available: self.amount(&ingredient.id),
                };
            }
        }

        // Consume inputs.
        let mut deltas: Vec<StockpileDelta> = Vec::new();
        for ingredient in &recipe.inputs {
            let removed = self.withdraw(&ingredient.id, ingredient.quantity);
            deltas.push(StockpileDelta {
                commodity_id: ingredient.id.clone(),
                change: -removed,
            });
        }

        // Produce outputs.
        for ingredient in &recipe.outputs {
            let added = self.deposit(&ingredient.id, ingredient.quantity);
            deltas.push(StockpileDelta {
                commodity_id: ingredient.id.clone(),
                change: added,
            });
        }

        RecipeOutcome::Produced { deltas }
    }
}

/// The net stockpile change for one commodity after a recipe cycle.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StockpileDelta {
    /// Commodity that changed.
    pub commodity_id: String,
    /// Amount changed (negative = consumed, positive = produced).
    pub change: f64,
}

/// Outcome of applying one recipe cycle to a [`ColonyPool`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RecipeOutcome {
    /// The recipe ran: inputs consumed, outputs deposited.
    Produced {
        /// Per-commodity stockpile changes this cycle.
        deltas: Vec<StockpileDelta>,
    },
    /// The recipe could not run due to input shortage; pool is unchanged.
    Stalled {
        /// The first commodity found to be in shortage.
        missing_commodity: String,
        /// Amount required by the recipe.
        required: f64,
        /// Amount actually available in the pool.
        available: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{Ingredient, RecipeDef};

    fn smelt_iron_recipe() -> RecipeDef {
        RecipeDef {
            id: "smelt_iron".into(),
            name: "Smelt Iron".into(),
            building: "smelter".into(),
            inputs: vec![Ingredient {
                id: "iron_ore".into(),
                quantity: 2.0,
            }],
            outputs: vec![Ingredient {
                id: "iron_plate".into(),
                quantity: 1.0,
            }],
            cycle_sols: 2,
            power_draw: 10.0,
        }
    }

    // ── ColonyPool basic operations ───────────────────────────────────────

    #[test]
    fn empty_pool_returns_zero_for_unknown_commodity() {
        let pool = ColonyPool::new();
        assert_eq!(pool.amount("iron_ore"), 0.0);
    }

    #[test]
    fn deposit_increases_amount_and_delta() {
        let mut pool = ColonyPool::new();
        pool.deposit("water", 50.0);
        assert_eq!(pool.amount("water"), 50.0);
        assert_eq!(pool.delta("water"), 50.0);
    }

    #[test]
    fn withdraw_decreases_amount_and_delta() {
        let mut pool = ColonyPool::new();
        pool.deposit("water", 100.0);
        pool.reset_deltas();
        pool.withdraw("water", 30.0);
        assert_eq!(pool.amount("water"), 70.0);
        assert!((pool.delta("water") - -30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deposit_is_capped_at_capacity() {
        let mut pool = ColonyPool::new();
        pool.set_capacity("water", 80.0);
        let added = pool.deposit("water", 100.0);
        assert!((added - 80.0).abs() < f64::EPSILON);
        assert!((pool.amount("water") - 80.0).abs() < f64::EPSILON);
    }

    #[test]
    fn withdraw_cannot_go_below_zero() {
        let mut pool = ColonyPool::new();
        pool.deposit("water", 10.0);
        let removed = pool.withdraw("water", 50.0);
        assert!((removed - 10.0).abs() < f64::EPSILON);
        assert_eq!(pool.amount("water"), 0.0);
    }

    #[test]
    fn reset_deltas_clears_to_zero() {
        let mut pool = ColonyPool::new();
        pool.deposit("water", 100.0);
        pool.reset_deltas();
        assert_eq!(pool.delta("water"), 0.0);
    }

    // ── Recipe application — happy path ──────────────────────────────────

    #[test]
    fn apply_recipe_consumes_inputs_and_produces_outputs() {
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 10.0);

        let recipe = smelt_iron_recipe();
        let outcome = pool.apply_recipe(&recipe);

        assert!(
            matches!(outcome, RecipeOutcome::Produced { .. }),
            "expected Produced, got {outcome:?}"
        );

        // 2 iron_ore consumed; 8 remain.
        assert!((pool.amount("iron_ore") - 8.0).abs() < f64::EPSILON);
        // 1 iron_plate produced.
        assert!((pool.amount("iron_plate") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn apply_recipe_returns_correct_deltas() {
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 4.0);

        let recipe = smelt_iron_recipe();
        let RecipeOutcome::Produced { deltas } = pool.apply_recipe(&recipe) else {
            panic!("expected Produced");
        };

        let ore_delta = deltas
            .iter()
            .find(|d| d.commodity_id == "iron_ore")
            .expect("ore delta present");
        assert!((ore_delta.change - -2.0).abs() < f64::EPSILON);

        let plate_delta = deltas
            .iter()
            .find(|d| d.commodity_id == "iron_plate")
            .expect("plate delta present");
        assert!((plate_delta.change - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn apply_recipe_multiple_cycles_accumulate_correctly() {
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 6.0);

        let recipe = smelt_iron_recipe();
        pool.apply_recipe(&recipe);
        pool.apply_recipe(&recipe);
        pool.apply_recipe(&recipe);

        // 3 cycles × 2 ore = 6 consumed; 3 plates produced.
        assert!((pool.amount("iron_ore")).abs() < f64::EPSILON);
        assert!((pool.amount("iron_plate") - 3.0).abs() < f64::EPSILON);
    }

    // ── Recipe application — shortage stall ──────────────────────────────

    #[test]
    fn apply_recipe_stalls_when_input_is_missing() {
        let mut pool = ColonyPool::new();
        // No iron_ore in pool.

        let recipe = smelt_iron_recipe();
        let outcome = pool.apply_recipe(&recipe);

        assert!(
            matches!(
                outcome,
                RecipeOutcome::Stalled {
                    ref missing_commodity,
                    ..
                } if missing_commodity == "iron_ore"
            ),
            "expected Stalled with iron_ore, got {outcome:?}"
        );
    }

    #[test]
    fn apply_recipe_stalls_when_input_is_insufficient() {
        let mut pool = ColonyPool::new();
        // Only 1 unit; recipe needs 2.
        pool.deposit("iron_ore", 1.0);

        let recipe = smelt_iron_recipe();
        let outcome = pool.apply_recipe(&recipe);

        assert!(
            matches!(outcome, RecipeOutcome::Stalled { .. }),
            "expected Stalled, got {outcome:?}"
        );
    }

    #[test]
    fn stalled_recipe_does_not_modify_pool() {
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 1.0); // insufficient

        let recipe = smelt_iron_recipe();
        pool.apply_recipe(&recipe);

        // Pool must be unchanged — no inputs consumed, no outputs added.
        assert!((pool.amount("iron_ore") - 1.0).abs() < f64::EPSILON);
        assert_eq!(pool.amount("iron_plate"), 0.0);
    }

    #[test]
    fn stalled_outcome_reports_required_and_available() {
        let mut pool = ColonyPool::new();
        pool.deposit("iron_ore", 0.5);

        let recipe = smelt_iron_recipe();
        let RecipeOutcome::Stalled {
            required,
            available,
            ..
        } = pool.apply_recipe(&recipe)
        else {
            panic!("expected Stalled");
        };

        assert!((required - 2.0).abs() < f64::EPSILON);
        assert!((available - 0.5).abs() < f64::EPSILON);
    }
}
