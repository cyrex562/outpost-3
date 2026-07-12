//! Effect/modifier descriptor — shared data shape for tech effects, difficulty grade-tables,
//! and menace-phase effects.
//!
//! Stacking discipline (§7A):
//! ```text
//! effective = base × (1 + Σ tech_bonuses_in_category) × difficulty_scalar
//! ```
//! Tech numeric bonuses are **additive within a category**; difficulty is a single outermost
//! multiplicative scalar applied last.

use serde::{Deserialize, Serialize};

// ─── ModifiableQuantity ───────────────────────────────────────────────────────

/// A sim quantity that can be modified by techs, difficulty, or menace phases.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModifiableQuantity {
    /// Production rate of a specific building (identified by content-pack id string).
    ProductionRate(String),
    /// Number of available building slots.
    SlotCapacity,
    /// Labour efficiency multiplier.
    LaborEfficiency,
    /// Research point generation rate.
    ResearchRate,
    /// Storage capacity for commodities.
    StorageCapacity,
    /// Colony population growth rate.
    PopulationGrowth,
    /// Stability change rate.
    StabilityRate,
    /// Environmental hazard trigger probability.
    HazardProbability,
    /// Per-capita resource consumption rate (food, water, oxygen, power).
    ///
    /// Higher values increase how much each colonist consumes per sol.
    /// Applied in [`crate::needs::apply_needs_check_scaled`].
    ResourceConsumption,
    /// Research cost multiplier applied when checking tech completion.
    ///
    /// Higher values require more research points to complete a tech.
    ResearchCost,
    /// Power requirement multiplier applied to positive `power_delta`
    /// and to recipe `power_draw` (consumers only, not generators).
    PowerRequirement,
    /// Per-building maintenance draw multiplier (issue #180).
    ///
    /// Higher values increase how much each building's authored
    /// [`crate::content::BuildingDef::maintenance`] entries drain from the
    /// colony pool each sol. Applied in
    /// [`crate::colony::process_production_scaled`].
    MaintenanceConsumption,
}

// ─── ModifierDescriptor ───────────────────────────────────────────────────────

/// A single modifier that adjusts one [`ModifiableQuantity`] by a fractional amount.
///
/// Multiple descriptors targeting the same quantity and category are summed additively
/// by [`ModifierAccumulator`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModifierDescriptor {
    /// The sim quantity this modifier affects.
    pub quantity: ModifiableQuantity,
    /// Grouping label — bonuses within the same category are summed additively.
    pub category: String,
    /// Fractional delta, e.g. `0.20` for +20%.
    pub value: f32,
}

impl ModifierDescriptor {
    /// Construct a new descriptor.
    pub fn new(quantity: ModifiableQuantity, category: impl Into<String>, value: f32) -> Self {
        Self {
            quantity,
            category: category.into(),
            value,
        }
    }
}

// ─── ModifierAccumulator ─────────────────────────────────────────────────────

/// Collects [`ModifierDescriptor`]s and computes per-quantity additive sums.
///
/// Descriptors in the **same category** for a given quantity are summed together.
/// Descriptors from different categories for the same quantity are also summed
/// (categories are labels that allow source-tracking; the final tech contribution
/// is the total additive sum regardless of category).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModifierAccumulator {
    descriptors: Vec<ModifierDescriptor>,
}

impl ModifierAccumulator {
    /// Create an empty accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a descriptor to the accumulator.
    pub fn add(&mut self, descriptor: ModifierDescriptor) {
        self.descriptors.push(descriptor);
    }

    /// Sum all descriptors for `quantity` that belong to `category`.
    #[must_use]
    pub fn additive_sum(&self, quantity: &ModifiableQuantity, category: &str) -> f32 {
        self.descriptors
            .iter()
            .filter(|d| &d.quantity == quantity && d.category == category)
            .map(|d| d.value)
            .sum()
    }

    /// Sum all descriptors for `quantity` across all categories.
    #[must_use]
    pub fn total_sum(&self, quantity: &ModifiableQuantity) -> f32 {
        self.descriptors
            .iter()
            .filter(|d| &d.quantity == quantity)
            .map(|d| d.value)
            .sum()
    }
}

// ─── DifficultyScalar ────────────────────────────────────────────────────────

/// Per-quantity outermost difficulty multiplier.
///
/// Applied after all tech bonuses: `effective = base × (1 + tech_sum) × difficulty`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DifficultyScalar {
    scalars: std::collections::HashMap<ModifiableQuantity, f32>,
}

impl DifficultyScalar {
    /// Create with no overrides (all quantities default to scalar `1.0`).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the scalar for a quantity.
    pub fn set(&mut self, quantity: ModifiableQuantity, scalar: f32) {
        self.scalars.insert(quantity, scalar);
    }

    /// Return the scalar for a quantity (`1.0` if not set).
    #[must_use]
    pub fn scalar_for(&self, quantity: &ModifiableQuantity) -> f32 {
        self.scalars.get(quantity).copied().unwrap_or(1.0)
    }
}

// ─── resolve ─────────────────────────────────────────────────────────────────

/// Resolve a final effective value using the stacking formula:
/// `effective = base × (1 + Σ tech_bonuses) × difficulty_scalar`.
///
/// This is the **single authoritative** computation point for modifier resolution.
#[must_use]
pub fn resolve(
    base: f32,
    quantity: &ModifiableQuantity,
    accum: &ModifierAccumulator,
    difficulty: &DifficultyScalar,
) -> f32 {
    let tech_sum = accum.total_sum(quantity);
    let diff_scalar = difficulty.scalar_for(quantity);
    base * (1.0 + tech_sum) * diff_scalar
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn qty() -> ModifiableQuantity {
        ModifiableQuantity::LaborEfficiency
    }

    /// Three +20% bonuses in the same category must resolve to +60%, not ×1.728.
    #[test]
    fn additive_within_category_not_compound() {
        let mut accum = ModifierAccumulator::new();
        for _ in 0..3 {
            accum.add(ModifierDescriptor::new(qty(), "tech", 0.20));
        }
        let difficulty = DifficultyScalar::new();
        let result = resolve(1.0, &qty(), &accum, &difficulty);
        // Expected: 1.0 × (1 + 0.60) × 1.0 = 1.60  (not 1.20^3 = 1.728)
        assert!((result - 1.60).abs() < 1e-4, "expected 1.60 got {result}");
    }

    /// Bonuses in different categories still accumulate additively in the total sum.
    #[test]
    fn category_isolation_tracks_per_category() {
        let mut accum = ModifierAccumulator::new();
        accum.add(ModifierDescriptor::new(qty(), "efficiency", 0.20));
        accum.add(ModifierDescriptor::new(qty(), "research", 0.10));
        assert!((accum.additive_sum(&qty(), "efficiency") - 0.20).abs() < 1e-4);
        assert!((accum.additive_sum(&qty(), "research") - 0.10).abs() < 1e-4);
        // Total for formula includes both categories
        assert!((accum.total_sum(&qty()) - 0.30).abs() < 1e-4);
    }

    /// Difficulty scalar is applied outermost — after all tech bonuses.
    #[test]
    fn difficulty_outermost_ordering() {
        let mut accum = ModifierAccumulator::new();
        accum.add(ModifierDescriptor::new(qty(), "tech", 0.50)); // +50% tech bonus

        let mut difficulty = DifficultyScalar::new();
        difficulty.set(qty(), 0.80); // difficulty makes it 80% of the tech-boosted value

        // base=100, tech: 100×(1+0.50)=150, difficulty: 150×0.80=120
        let result = resolve(100.0, &qty(), &accum, &difficulty);
        assert!((result - 120.0).abs() < 1e-4, "expected 120.0 got {result}");

        // Verify ordering: if difficulty were interleaved it would be different
        // (interleaved would give the same result here, but we confirm the formula)
        let no_difficulty = DifficultyScalar::new();
        let tech_only = resolve(100.0, &qty(), &accum, &no_difficulty);
        assert!((tech_only - 150.0).abs() < 1e-4); // 100 × 1.5 = 150
    }

    /// Unrelated quantities do not contaminate each other.
    #[test]
    fn different_quantities_are_isolated() {
        let mut accum = ModifierAccumulator::new();
        accum.add(ModifierDescriptor::new(
            ModifiableQuantity::ResearchRate,
            "tech",
            0.50,
        ));
        // LaborEfficiency gets no bonus
        assert!((accum.total_sum(&ModifiableQuantity::LaborEfficiency)).abs() < 1e-4);
    }

    /// Difficulty defaults to 1.0 when not set.
    #[test]
    fn difficulty_defaults_to_one() {
        let difficulty = DifficultyScalar::new();
        assert!((difficulty.scalar_for(&qty()) - 1.0).abs() < 1e-4);
    }

    /// ProductionRate keyed by building id works correctly.
    #[test]
    fn production_rate_keyed_by_building() {
        let mine = ModifiableQuantity::ProductionRate("mine".into());
        let lab = ModifiableQuantity::ProductionRate("lab".into());
        let mut accum = ModifierAccumulator::new();
        accum.add(ModifierDescriptor::new(mine.clone(), "tech", 0.30));
        assert!((accum.total_sum(&mine) - 0.30).abs() < 1e-4);
        assert!((accum.total_sum(&lab)).abs() < 1e-4);
    }
}
