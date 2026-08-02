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
    /// Commodity cost multiplier applied to every construction project's
    /// authored `construction_cost` (issue #306).
    ///
    /// Higher values make building more expensive in materials. Applied once,
    /// when the project is queued, so the stored cost — and therefore the
    /// per-sol instalment, the UI readout, and the cancellation refund — all
    /// agree on one number.
    ///
    /// Replaced the `SlotCapacity` variant, which was authored in
    /// `content/difficulty.yaml` with a full grade row but never resolved
    /// anywhere: the difficulty selector promised fewer build slots and
    /// delivered nothing. Slot capacity is now bought with the
    /// colony-infrastructure project instead of handed out by preset.
    ConstructionCost,
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
    /// Morale change rate (issue #382 added morale; this scalar was added
    /// separately when Easy difficulty turned out to have zero effect on
    /// morale decay speed — see [`crate::morale`]'s module doc for why
    /// morale is its own quantity rather than folded into `StabilityRate`).
    ///
    /// Higher values mean faster morale swings (gain and loss alike),
    /// mirroring how `StabilityRate` scales [`crate::needs`]'s stability
    /// delta. Applied in the morale resolution step of the turn pipeline.
    MoraleRate,
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
    /// Resource-deposit generosity multiplier (issue #232).
    ///
    /// Scales [`crate::system::BodyDeposit::abundance`] at system-generation
    /// time — higher values make a generated system's raw-material presence
    /// more generous. Applied once, in
    /// [`crate::system_gen::generate_system`], not per-turn like the other
    /// variants here (deposit generosity is a generation-time property, not
    /// something recomputed every sol).
    DepositAbundance,
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
    /// Serialized as a flat list of `(quantity, scalar)` pairs, not as a JSON
    /// object: [`ModifiableQuantity::ProductionRate`] is a newtype variant, and
    /// `serde_json` — the save format — cannot use one as an object key. The
    /// default grade table always seeds a `ProductionRate` entry, so every game
    /// that set a difficulty failed to save with `"key must be a string"`
    /// (issue #337).
    #[serde(with = "scalars_serde")]
    scalars: std::collections::HashMap<ModifiableQuantity, f32>,
}

/// (De)serialize [`DifficultyScalar::scalars`] as a `Vec<(ModifiableQuantity, f32)>`.
///
/// See the field's doc comment for why the map form is unusable.
mod scalars_serde {
    use super::ModifiableQuantity;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub(super) fn serialize<S: Serializer>(
        scalars: &HashMap<ModifiableQuantity, f32>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        // Sorted by debug form so save files stay byte-stable — `HashMap`
        // iteration order is not stable across runs.
        let mut pairs: Vec<(&ModifiableQuantity, &f32)> = scalars.iter().collect();
        pairs.sort_by_key(|(q, _)| format!("{q:?}"));
        pairs.serialize(serializer)
    }

    /// Accepts the new pair-list form *and* the legacy JSON-object form.
    ///
    /// A save written before #337 stored this field as an object. Only an
    /// **empty** one can exist — serializing any key at all failed, so a save
    /// with a populated scalar map was never written to disk — but `{}` was a
    /// perfectly valid `SCHEMA_VERSION = 9` payload for a game saved before a
    /// difficulty was chosen. Reading both shapes keeps those saves loadable
    /// instead of rejecting them on a version bump.
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<HashMap<ModifiableQuantity, f32>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Either {
            Pairs(Vec<(ModifiableQuantity, f32)>),
            /// Pre-#337 object form. Keys are read as opaque strings because
            /// `ModifiableQuantity` cannot be parsed back from a JSON key.
            Legacy(HashMap<String, f32>),
        }

        match Either::deserialize(deserializer)? {
            Either::Pairs(pairs) => Ok(pairs.into_iter().collect()),
            Either::Legacy(legacy) if legacy.is_empty() => Ok(HashMap::new()),
            // Unreachable from any file this code could have written. Refuse
            // rather than silently discard scalars, in case one turns up.
            Either::Legacy(legacy) => Err(serde::de::Error::custom(format!(
                "difficulty scalars use the pre-#337 object form and are not empty \
                 ({} entries); this save cannot be read",
                legacy.len()
            ))),
        }
    }
}

impl ModifiableQuantity {
    /// Every scalar-tunable quantity, in a stable display order.
    ///
    /// Excludes [`Self::ProductionRate`], which is parameterised by a
    /// content-pack building id and so is not a single dial — it would need one
    /// row per building rather than one row per quantity.
    ///
    /// Used by the live balance editor (playtesting) to enumerate what can be
    /// tuned without hardcoding the list in a host or the frontend.
    pub const TUNABLE: &'static [Self] = &[
        Self::ConstructionCost,
        Self::LaborEfficiency,
        Self::ResearchRate,
        Self::ResearchCost,
        Self::StorageCapacity,
        Self::PopulationGrowth,
        Self::StabilityRate,
        Self::MoraleRate,
        Self::HazardProbability,
        Self::ResourceConsumption,
        Self::PowerRequirement,
        Self::MaintenanceConsumption,
        Self::DepositAbundance,
    ];

    /// Stable wire identifier for this quantity.
    ///
    /// Hosts and the frontend key on this string rather than serialising the
    /// enum, so adding a variant cannot silently reshape an existing payload.
    /// Returns `None` for [`Self::ProductionRate`], which is not a single dial.
    #[must_use]
    pub fn slug(&self) -> Option<&'static str> {
        Some(match self {
            Self::ConstructionCost => "construction_cost",
            Self::LaborEfficiency => "labor_efficiency",
            Self::ResearchRate => "research_rate",
            Self::ResearchCost => "research_cost",
            Self::StorageCapacity => "storage_capacity",
            Self::PopulationGrowth => "population_growth",
            Self::StabilityRate => "stability_rate",
            Self::MoraleRate => "morale_rate",
            Self::HazardProbability => "hazard_probability",
            Self::ResourceConsumption => "resource_consumption",
            Self::PowerRequirement => "power_requirement",
            Self::MaintenanceConsumption => "maintenance_consumption",
            Self::DepositAbundance => "deposit_abundance",
            Self::ProductionRate(_) => return None,
        })
    }

    /// Resolve a [`Self::slug`] back to its quantity.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::TUNABLE
            .iter()
            .find(|q| q.slug() == Some(slug))
            .cloned()
    }
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

    /// A save written before #337 stored `scalars` as a JSON object. Only the
    /// empty form can exist on disk, and it must still load — bumping the
    /// schema version instead would reject those saves outright.
    #[test]
    fn the_pre_337_empty_object_form_still_loads() {
        let legacy: DifficultyScalar =
            serde_json::from_str(r#"{"scalars":{}}"#).expect("a pre-#337 save must still load");
        assert!(
            (legacy.scalar_for(&ModifiableQuantity::ResearchRate) - 1.0).abs() < 1e-6,
            "an empty scalar map means no overrides"
        );
    }

    /// The current form round-trips, including the newtype-variant key that
    /// JSON cannot use as an object key.
    #[test]
    fn the_pair_list_form_round_trips_a_newtype_variant_key() {
        let mut ds = DifficultyScalar::new();
        ds.set(ModifiableQuantity::ProductionRate("mine".into()), 0.85);
        ds.set(ModifiableQuantity::ResearchRate, 1.2);

        let json = serde_json::to_string(&ds).expect("must serialize");
        let back: DifficultyScalar = serde_json::from_str(&json).expect("must deserialize");

        assert!(
            (back.scalar_for(&ModifiableQuantity::ProductionRate("mine".into())) - 0.85).abs()
                < 1e-6
        );
        assert!((back.scalar_for(&ModifiableQuantity::ResearchRate) - 1.2).abs() < 1e-6);
        // An unset key still falls back rather than being invented.
        assert!((back.scalar_for(&ModifiableQuantity::ConstructionCost) - 1.0).abs() < 1e-6);
    }

    /// A populated legacy object cannot exist, but if one turns up it must be
    /// refused rather than silently read as "no overrides".
    #[test]
    fn a_populated_pre_337_object_is_refused_not_silently_dropped() {
        let err = serde_json::from_str::<DifficultyScalar>(r#"{"scalars":{"ResearchRate":0.5}}"#)
            .expect_err("a populated legacy object must not parse silently");
        assert!(
            format!("{err}").contains("pre-#337")
                || format!("{err}").contains("data did not match"),
            "unexpected error: {err}"
        );
    }
}
