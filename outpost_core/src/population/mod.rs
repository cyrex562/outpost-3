//! Population subsystem — aggregate pool, stability, skill distribution, and labour derivation.
//!
//! Population in Outpost 3 is an aggregate scalar per colony, not a
//! per-citizen simulation (see `docs/DESIGN.md §6`). This module owns:
//!
//! - The population pool and its growth/attrition placeholder.
//! - The stability scalar and its modifiers.
//! - Skill distribution: a fractional breakdown across skill types.
//! - Labour derivation: `available_labor() = count × stability`.

use std::collections::HashMap;

/// Skill categories that colonists can belong to.
///
/// A colony's [`PopulationPool`] holds a fractional distribution across these
/// types; the fractions must sum to `1.0`. Buildings declare a `primary_skill`
/// requirement; when the matching skill fraction is insufficient the building
/// gets a degraded labor efficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillType {
    /// General-purpose manual workers.
    Laborer,
    /// Technical and mechanical specialists.
    Engineer,
    /// Equipment operators and systems technicians.
    Operator,
    /// Agricultural and food-production specialists.
    Farmer,
    /// Medical personnel.
    Medic,
    /// Researchers and scientists.
    Scientist,
}

/// Aggregate population state for a single colony.
///
/// All fields are scalars; individual citizens are not modelled.
/// Labor is derived each turn as `count × stability`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PopulationPool {
    /// Total number of colonists (head-count, stored as f32 for fractional growth).
    pub count: f32,
    /// Stability scalar in the range `[0.0, 1.0]`.
    ///
    /// At `1.0` the colony is fully stable; at `0.0` it is on the verge of
    /// collapse. Values outside this range are clamped by the engine.
    pub stability: f32,
    /// Fractional skill distribution — values must sum to `1.0`.
    ///
    /// Keys are [`SkillType`] variants. Missing keys are treated as `0.0`.
    /// Use [`PopulationPool::new`] to get a sensible default distribution.
    pub skill_distribution: HashMap<SkillType, f32>,
}

impl PopulationPool {
    /// Create a starting population with the given head-count, full stability,
    /// and the canonical default skill distribution from `docs/DESIGN.md §6`.
    #[must_use]
    pub fn new(count: f32) -> Self {
        let skill_distribution = default_skill_distribution();
        Self {
            count,
            stability: 1.0,
            skill_distribution,
        }
    }

    /// Create a population with a fully custom skill distribution.
    ///
    /// The caller is responsible for ensuring `skill_distribution` values sum
    /// to approximately `1.0`; [`PopulationPool::skill_fraction`] clamps inputs
    /// to `[0.0, 1.0]` so an un-normalised distribution degrades gracefully.
    #[must_use]
    pub fn with_skills(
        count: f32,
        stability: f32,
        skill_distribution: HashMap<SkillType, f32>,
    ) -> Self {
        Self {
            count,
            stability: stability.clamp(0.0, 1.0),
            skill_distribution,
        }
    }

    /// Derive the number of available labor units this turn.
    ///
    /// `available_labor = count × stability` (clamped to `[0.0, count]`).
    #[must_use]
    pub fn available_labor(&self) -> f32 {
        (self.count * self.stability.clamp(0.0, 1.0)).max(0.0)
    }

    /// Fraction of the population with the given skill type, in `[0.0, 1.0]`.
    #[must_use]
    pub fn skill_fraction(&self, skill: SkillType) -> f32 {
        self.skill_distribution
            .get(&skill)
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    }

    /// Labor units available that have the given skill type.
    ///
    /// `skilled_labor(skill) = available_labor() × skill_fraction(skill)`
    #[must_use]
    pub fn skilled_labor(&self, skill: SkillType) -> f32 {
        self.available_labor() * self.skill_fraction(skill)
    }

    /// Compute the skill-match efficiency for a building that requires `skill`.
    ///
    /// Efficiency is the ratio of available skilled labor to demanded labor,
    /// clamped to `[0.0, 1.0]`. Laborers can substitute for any skill at a
    /// reduced efficiency of `0.5`.
    ///
    /// If `labor_demanded` is `0.0`, returns `1.0` (no constraint).
    #[must_use]
    pub fn skill_match_efficiency(&self, skill: SkillType, labor_demanded: f32) -> f32 {
        if labor_demanded <= 0.0 {
            return 1.0;
        }
        let primary = self.skilled_labor(skill);
        // Laborers can fill skill gaps at half efficiency.
        let laborer_fill = if skill == SkillType::Laborer {
            0.0
        } else {
            self.skilled_labor(SkillType::Laborer) * 0.5
        };
        let effective = (primary + laborer_fill).min(labor_demanded);
        (effective / labor_demanded).clamp(0.0, 1.0)
    }

    /// Backward-compatible shim: returns available labor as a truncated `u64`.
    ///
    /// Prefer [`PopulationPool::available_labor`] for new code.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn available_labour(&self) -> u64 {
        self.available_labor() as u64
    }

    /// Apply one turn of population growth (placeholder for Phase 7 full model).
    ///
    /// When stability is `≥ 0.8` the population grows by `GROWTH_RATE_PER_SOL`
    /// per sol. This is a deliberately simple placeholder; the full demographic
    /// model (immigration waves, fluid migration) ships in Phase 7.
    pub fn apply_growth_tick(&mut self) {
        self.apply_growth_tick_with_scalar(1.0);
    }

    /// Apply one turn of population growth with a difficulty scalar.
    ///
    /// The `growth_scalar` is the outermost difficulty multiplier applied to the
    /// growth rate: `effective_rate = base_rate × growth_scalar`.
    pub fn apply_growth_tick_with_scalar(&mut self, growth_scalar: f32) {
        const GROWTH_RATE_PER_SOL: f32 = 0.001; // 0.1 % per sol at full stability
        if self.stability >= 0.8 {
            self.count += self.count * GROWTH_RATE_PER_SOL * growth_scalar * (self.stability - 0.79);
        }
    }
}

/// Back-compat type alias so existing code that references `Population` still compiles.
pub type Population = PopulationPool;

/// Public re-export of the default skill distribution (used by snapshot restore).
#[must_use]
pub fn default_skill_distribution_pub() -> HashMap<SkillType, f32> {
    default_skill_distribution()
}

/// Default skill fractions matching the C# `DifficultySettings.StartingSkills` ratios.
fn default_skill_distribution() -> HashMap<SkillType, f32> {
    [
        (SkillType::Laborer, 0.40),
        (SkillType::Engineer, 0.20),
        (SkillType::Operator, 0.15),
        (SkillType::Farmer, 0.12),
        (SkillType::Medic, 0.07),
        (SkillType::Scientist, 0.06),
    ]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Labor derivation ──────────────────────────────────────────────────────

    #[test]
    fn new_population_has_full_stability() {
        let p = PopulationPool::new(100.0);
        assert!((p.stability - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn full_stability_labor_equals_count() {
        let p = PopulationPool::new(50.0);
        assert!((p.available_labor() - 50.0).abs() < 1e-4);
    }

    #[test]
    fn zero_stability_yields_no_labor() {
        let p = PopulationPool::with_skills(100.0, 0.0, default_skill_distribution());
        assert!((p.available_labor()).abs() < 1e-4);
    }

    #[test]
    fn half_stability_halves_labor() {
        let p = PopulationPool::with_skills(100.0, 0.5, default_skill_distribution());
        assert!((p.available_labor() - 50.0).abs() < 1e-4);
    }

    // ── Stability effects on labor ────────────────────────────────────────────

    #[test]
    fn stability_clamps_above_one() {
        let p = PopulationPool::with_skills(100.0, 1.5, default_skill_distribution());
        assert!((p.stability - 1.0).abs() < f32::EPSILON);
        assert!((p.available_labor() - 100.0).abs() < 1e-4);
    }

    #[test]
    fn stability_clamps_below_zero() {
        let p = PopulationPool::with_skills(100.0, -0.5, default_skill_distribution());
        assert!((p.stability).abs() < f32::EPSILON);
        assert!((p.available_labor()).abs() < 1e-4);
    }

    #[test]
    fn labor_scales_linearly_with_stability() {
        for pct in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let p = PopulationPool::with_skills(200.0, pct, default_skill_distribution());
            let expected = 200.0 * pct;
            assert!(
                (p.available_labor() - expected).abs() < 1e-3,
                "at stability {pct}: got {}, want {expected}",
                p.available_labor()
            );
        }
    }

    // ── Skill distribution ────────────────────────────────────────────────────

    #[test]
    fn default_skill_distribution_sums_to_one() {
        let p = PopulationPool::new(100.0);
        let total: f32 = p.skill_distribution.values().sum();
        assert!(
            (total - 1.0).abs() < 1e-4,
            "skill fractions sum to {total}, want 1.0"
        );
    }

    #[test]
    fn missing_skill_returns_zero_fraction() {
        let p = PopulationPool::with_skills(100.0, 1.0, HashMap::new());
        assert!((p.skill_fraction(SkillType::Engineer)).abs() < f32::EPSILON);
    }

    // ── Skill-match efficiency ────────────────────────────────────────────────

    #[test]
    fn full_skill_match_gives_efficiency_one() {
        // 100 pop × 1.0 stability × 0.20 engineer fraction = 20 engineers.
        // demand = 20 → efficiency = 1.0.
        let p = PopulationPool::new(100.0);
        let eff = p.skill_match_efficiency(SkillType::Engineer, 20.0);
        assert!((eff - 1.0).abs() < 1e-4, "efficiency was {eff}");
    }

    #[test]
    fn skill_shortage_degrades_efficiency() {
        // 100 pop × 1.0 stability × 0.20 engineer = 20 engineers available.
        // demand = 40 → primary covers half. Laborers: 40 × 0.5 = 20.
        // effective = (20 + 20).min(40) = 40 → eff = 1.0 (laborers fill gap).
        // Now remove laborers to expose the shortfall.
        let mut dist = HashMap::new();
        dist.insert(SkillType::Engineer, 0.20);
        // No laborers.
        let p = PopulationPool::with_skills(100.0, 1.0, dist);
        // 20 engineers, 0 laborers, demand = 40 → eff = 20/40 = 0.5
        let eff = p.skill_match_efficiency(SkillType::Engineer, 40.0);
        assert!(
            (eff - 0.5).abs() < 1e-4,
            "expected efficiency 0.5, got {eff}"
        );
    }

    #[test]
    fn laborers_partially_fill_skill_gap() {
        // 100 pop, 0 engineers, 0.40 laborers → 40 laborers.
        // demand = 20 engineers. primary = 0. laborer fill = 40 * 0.5 = 20.
        // effective = min(20, 20) = 20 → eff = 1.0.
        let mut dist = HashMap::new();
        dist.insert(SkillType::Laborer, 0.40);
        let p = PopulationPool::with_skills(100.0, 1.0, dist);
        let eff = p.skill_match_efficiency(SkillType::Engineer, 20.0);
        assert!((eff - 1.0).abs() < 1e-4, "laborer fill: eff was {eff}");
    }

    #[test]
    fn laborer_efficiency_not_double_counted() {
        // When asking for Laborer skill, the 0.5 sub-fill should not apply.
        let p = PopulationPool::new(100.0);
        // 40 laborers, demand = 40 → eff = 1.0.
        let eff = p.skill_match_efficiency(SkillType::Laborer, 40.0);
        assert!((eff - 1.0).abs() < 1e-4, "laborer self-fill: eff was {eff}");
    }

    #[test]
    fn zero_demand_always_gives_efficiency_one() {
        let p = PopulationPool::new(100.0);
        assert!((p.skill_match_efficiency(SkillType::Engineer, 0.0) - 1.0).abs() < f32::EPSILON);
    }

    // ── Growth placeholder ────────────────────────────────────────────────────

    #[test]
    fn stable_population_grows_each_tick() {
        let mut p = PopulationPool::new(1000.0);
        let before = p.count;
        p.apply_growth_tick();
        assert!(p.count > before, "stable population should grow");
    }

    #[test]
    fn unstable_population_does_not_grow() {
        let mut p = PopulationPool::with_skills(1000.0, 0.5, default_skill_distribution());
        let before = p.count;
        p.apply_growth_tick();
        assert!(
            (p.count - before).abs() < f32::EPSILON,
            "unstable population should not grow"
        );
    }

    #[test]
    fn growth_is_proportional_to_population() {
        let mut small = PopulationPool::new(100.0);
        let mut large = PopulationPool::new(1000.0);
        small.apply_growth_tick();
        large.apply_growth_tick();
        let small_delta = small.count - 100.0;
        let large_delta = large.count - 1000.0;
        assert!(
            large_delta > small_delta,
            "larger pop should gain more absolute colonists per tick"
        );
    }
}
