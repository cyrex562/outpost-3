//! Needs resolution and stability dynamics — §5, §6.
//!
//! Each turn the colony pool is checked against the population's bulk needs
//! (food, water, oxygen, housing, power). The satisfaction ratios are combined
//! into a composite that drives `stability_delta` along a smooth curve.
//!
//! **No step-function cliffs.** The prior C# implementation had hard thresholds;
//! this module uses quadratic interpolation so fractional shortfalls produce
//! proportional stability changes.
//!
//! Stability feeds back into population growth and decline:
//! - `stability >= growth_threshold` + housing available → population grows.
//! - `stability < decline_threshold` → growth stalls, then population loss.

use crate::colony::ColonyPool;

// ─── Need definitions ─────────────────────────────────────────────────────────

/// How a need's required quantity is scaled relative to population.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum NeedScaling {
    /// Consumed per colonist per turn.  Required = rate × population.
    PerCapita {
        /// Commodity units consumed per colonist per turn.
        rate: f64,
    },
    /// Housing slot check: required = population; pool holds total capacity.
    /// Nothing is consumed — the pool just needs to hold ≥ population units.
    Housing,
}

/// One bulk need that a colony must satisfy each turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NeedDef {
    /// Commodity that satisfies this need.
    pub commodity_id: String,
    /// How the required amount is scaled from population.
    pub scaling: NeedScaling,
    /// Relative weight in the composite satisfaction score (`[0.0, 1.0]`).
    ///
    /// Weights across all needs need not sum to 1 — they are normalised
    /// automatically in [`compute_composite`].
    pub weight: f32,
}

impl NeedDef {
    /// Calculate how much of this commodity is required for `population` colonists.
    #[must_use]
    pub fn required_amount(&self, population: f64) -> f64 {
        match self.scaling {
            NeedScaling::PerCapita { rate } => rate * population,
            NeedScaling::Housing => population,
        }
    }
}

// ─── Configuration ────────────────────────────────────────────────────────────

/// Tunable parameters for the needs and population-dynamics subsystem.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NeedsConfig {
    /// Ordered list of needs the colony must satisfy each turn.
    pub needs: Vec<NeedDef>,

    /// Maximum stability gained per turn at 100 % satisfaction (`[0.0, 1.0]`).
    pub stability_recovery_rate: f32,

    /// Maximum stability lost per turn at 0 % satisfaction (`[0.0, 1.0]`).
    pub stability_decay_rate: f32,

    /// Stability at or above which the population may grow (`[0.0, 1.0]`).
    pub growth_stability_threshold: f32,

    /// Fractional population growth per turn when growth is active.
    pub growth_rate: f32,

    /// Stability below which the population begins to decline (`[0.0, 1.0]`).
    pub decline_stability_threshold: f32,

    /// Fractional population loss per turn when decline is active.
    pub decline_rate: f32,
}

impl NeedsConfig {
    /// A sensible default configuration for a basic survival colony.
    ///
    /// Food, water, oxygen, housing, and power are included at per-capita
    /// rates that a small starter base can satisfy with minimal infrastructure.
    #[must_use]
    pub fn default_survival() -> Self {
        Self {
            needs: vec![
                NeedDef {
                    commodity_id: "food".into(),
                    scaling: NeedScaling::PerCapita { rate: 0.1 },
                    weight: 1.0,
                },
                NeedDef {
                    commodity_id: "water".into(),
                    scaling: NeedScaling::PerCapita { rate: 0.15 },
                    weight: 1.0,
                },
                NeedDef {
                    commodity_id: "oxygen".into(),
                    scaling: NeedScaling::PerCapita { rate: 0.05 },
                    weight: 1.0,
                },
                NeedDef {
                    commodity_id: "housing".into(),
                    scaling: NeedScaling::Housing,
                    weight: 0.8,
                },
                NeedDef {
                    commodity_id: "power".into(),
                    scaling: NeedScaling::PerCapita { rate: 0.2 },
                    weight: 0.6,
                },
            ],
            stability_recovery_rate: 0.05,
            stability_decay_rate: 0.10,
            growth_stability_threshold: 0.70,
            growth_rate: 0.002,
            decline_stability_threshold: 0.30,
            decline_rate: 0.001,
        }
    }
}

// ─── Per-need result ──────────────────────────────────────────────────────────

/// Satisfaction result for a single need after the turn's consumption check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NeedSatisfaction {
    /// Commodity id for this need.
    pub commodity_id: String,
    /// Amount the population required this turn.
    pub required: f64,
    /// Amount actually consumed from the pool (≤ required).
    pub consumed: f64,
    /// Fraction of the need met, in `[0.0, 1.0]`.
    pub satisfaction: f32,
}

// ─── Report ──────────────────────────────────────────────────────────────────

/// Full needs check output for one colony-turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NeedsReport {
    /// Per-need satisfaction breakdown.
    pub needs: Vec<NeedSatisfaction>,
    /// Weighted composite satisfaction score, in `[0.0, 1.0]`.
    pub composite_satisfaction: f32,
    /// Stability change to apply this turn (positive = improvement).
    pub stability_delta: f32,
    /// Population growth delta this turn (positive = new arrivals).
    pub population_delta: f32,
}

// ─── Core functions ───────────────────────────────────────────────────────────

/// Compute weighted composite satisfaction from individual need results.
///
/// Weights are normalised so the scale of individual weights doesn't matter.
/// Returns `1.0` when there are no needs (vacuously satisfied).
#[must_use]
pub fn compute_composite(needs: &[NeedSatisfaction], defs: &[NeedDef]) -> f32 {
    let mut weighted_sum = 0.0_f32;
    let mut weight_total = 0.0_f32;
    for (sat, def) in needs.iter().zip(defs.iter()) {
        weighted_sum += sat.satisfaction * def.weight;
        weight_total += def.weight;
    }
    if weight_total <= 0.0 {
        return 1.0;
    }
    (weighted_sum / weight_total).clamp(0.0, 1.0)
}

/// Compute `stability_delta` from `composite_satisfaction` using a smooth curve.
///
/// The curve is quadratic in both directions (no step-function cliffs):
/// - `composite = 1.0` → `+recovery_rate`   (maximum stability gain)
/// - `composite = 0.5` → small residual      (near-neutral)
/// - `composite = 0.0` → `-decay_rate`       (maximum stability loss)
///
/// Formula: `recovery_rate * c² - decay_rate * (1 - c)²`
#[must_use]
pub fn stability_delta_from_satisfaction(
    composite: f32,
    recovery_rate: f32,
    decay_rate: f32,
) -> f32 {
    let c = composite.clamp(0.0, 1.0);
    let gain = recovery_rate * c * c;
    let loss = decay_rate * (1.0 - c) * (1.0 - c);
    gain - loss
}

/// Apply the per-turn needs check to `pool`, consuming commodities and returning a [`NeedsReport`].
///
/// Housing needs are **not** consumed — they are a capacity check only.
/// For per-capita needs the pool is drawn down by `required`; if the pool
/// cannot cover the full demand, only the available amount is consumed and the
/// satisfaction ratio reflects the shortfall.
///
/// After computing `stability_delta`, the caller is responsible for clamping
/// the population's `stability` field to `[0.0, 1.0]`.
#[must_use]
pub fn apply_needs_check(
    pool: &mut ColonyPool,
    population: f64,
    config: &NeedsConfig,
) -> NeedsReport {
    let mut need_results: Vec<NeedSatisfaction> = Vec::with_capacity(config.needs.len());

    for def in &config.needs {
        let required = def.required_amount(population);

        let (consumed, satisfaction) = if required <= 0.0 {
            (0.0, 1.0_f32)
        } else {
            match &def.scaling {
                NeedScaling::PerCapita { .. } => {
                    let consumed = pool.withdraw(&def.commodity_id, required);
                    #[allow(clippy::cast_possible_truncation)]
                    let sat = (consumed / required).clamp(0.0, 1.0) as f32;
                    (consumed, sat)
                }
                NeedScaling::Housing => {
                    // Housing is a capacity check — nothing consumed.
                    let capacity = pool.amount(&def.commodity_id);
                    #[allow(clippy::cast_possible_truncation)]
                    let sat = (capacity / required).clamp(0.0, 1.0) as f32;
                    (0.0, sat)
                }
            }
        };

        need_results.push(NeedSatisfaction {
            commodity_id: def.commodity_id.clone(),
            required,
            consumed,
            satisfaction,
        });
    }

    let composite = compute_composite(&need_results, &config.needs);
    let stability_delta = stability_delta_from_satisfaction(
        composite,
        config.stability_recovery_rate,
        config.stability_decay_rate,
    );

    NeedsReport {
        needs: need_results,
        composite_satisfaction: composite,
        stability_delta,
        population_delta: 0.0, // filled in by apply_population_dynamics
    }
}

/// Apply population growth and decline based on current stability.
///
/// Returns the net population change (positive = growth, negative = decline).
/// The caller must add this to `population.count`.
///
/// Growth occurs when `stability >= growth_threshold` *and* housing
/// satisfaction is positive (the colony has room for newcomers).
/// Decline occurs when `stability < decline_threshold`.
#[must_use]
pub fn apply_population_dynamics(
    population: f64,
    stability: f32,
    housing_satisfaction: f32,
    config: &NeedsConfig,
) -> f32 {
    #[allow(clippy::cast_possible_truncation)]
    let pop_f32 = population as f32;

    if stability >= config.growth_stability_threshold && housing_satisfaction > 0.0 {
        // Growth: proportional to population and how far above threshold we are.
        let excess = stability - config.growth_stability_threshold;
        let scale = (excess / (1.0 - config.growth_stability_threshold)).clamp(0.0, 1.0);
        pop_f32 * config.growth_rate * scale
    } else if stability < config.decline_stability_threshold {
        // Decline: population loss proportional to how far below threshold.
        let deficit = config.decline_stability_threshold - stability;
        let scale = (deficit / config.decline_stability_threshold).clamp(0.0, 1.0);
        -(pop_f32 * config.decline_rate * scale)
    } else {
        0.0 // in the neutral band
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::ColonyPool;

    /// Build a pool with enough food, water, oxygen, and power for `pop` colonists.
    fn full_supply_pool(pop: f64, housing_slots: f64) -> ColonyPool {
        let cfg = NeedsConfig::default_survival();
        let mut pool = ColonyPool::new();
        for def in &cfg.needs {
            match &def.scaling {
                NeedScaling::PerCapita { rate } => {
                    pool.deposit(&def.commodity_id, rate * pop * 10.0); // 10× buffer
                }
                NeedScaling::Housing => {
                    pool.deposit(&def.commodity_id, housing_slots);
                }
            }
        }
        pool
    }

    // ── Full supply → growth ────────────────────────────────────────────────

    #[test]
    fn full_supply_composite_is_one() {
        let mut pool = full_supply_pool(100.0, 200.0);
        let cfg = NeedsConfig::default_survival();
        let report = apply_needs_check(&mut pool, 100.0, &cfg);
        assert!(
            (report.composite_satisfaction - 1.0).abs() < 0.01,
            "full supply should give composite ≈ 1.0, got {}",
            report.composite_satisfaction
        );
    }

    #[test]
    fn full_supply_stability_delta_is_positive() {
        let mut pool = full_supply_pool(100.0, 200.0);
        let cfg = NeedsConfig::default_survival();
        let report = apply_needs_check(&mut pool, 100.0, &cfg);
        assert!(
            report.stability_delta > 0.0,
            "full supply should give positive stability_delta, got {}",
            report.stability_delta
        );
    }

    #[test]
    fn well_supplied_colony_grows() {
        let cfg = NeedsConfig::default_survival();
        let pop = 100.0_f64;
        let stability = 0.90_f32;
        let housing_sat = 1.0_f32;
        let delta = apply_population_dynamics(pop, stability, housing_sat, &cfg);
        assert!(
            delta > 0.0,
            "well-supplied colony should grow, delta={delta}"
        );
    }

    // ── Starvation → decline ────────────────────────────────────────────────

    #[test]
    fn no_supply_composite_is_zero() {
        let mut pool = ColonyPool::new(); // empty
        let cfg = NeedsConfig::default_survival();
        let report = apply_needs_check(&mut pool, 100.0, &cfg);
        assert!(
            report.composite_satisfaction < 0.05,
            "no supply should give composite ≈ 0, got {}",
            report.composite_satisfaction
        );
    }

    #[test]
    fn starvation_gives_negative_stability_delta() {
        let mut pool = ColonyPool::new();
        let cfg = NeedsConfig::default_survival();
        let report = apply_needs_check(&mut pool, 100.0, &cfg);
        assert!(
            report.stability_delta < 0.0,
            "starvation should give negative stability_delta, got {}",
            report.stability_delta
        );
    }

    #[test]
    fn starved_colony_loses_population_after_stability_falls() {
        let cfg = NeedsConfig::default_survival();
        let pop = 100.0_f64;
        let stability = 0.10_f32; // well below decline threshold
        let housing_sat = 1.0_f32;
        let delta = apply_population_dynamics(pop, stability, housing_sat, &cfg);
        assert!(
            delta < 0.0,
            "low stability should cause population decline, delta={delta}"
        );
    }

    // ── Partial supply → smooth curve ───────────────────────────────────────

    #[test]
    fn partial_supply_composite_is_between_zero_and_one() {
        let mut pool = ColonyPool::new();
        let cfg = NeedsConfig::default_survival();
        // Provide exactly 50 % of each per-capita need.
        let pop = 100.0;
        for def in &cfg.needs {
            if let NeedScaling::PerCapita { rate } = def.scaling {
                pool.deposit(&def.commodity_id, rate * pop * 0.5);
            } else {
                pool.deposit(&def.commodity_id, pop * 0.5);
            }
        }
        let report = apply_needs_check(&mut pool, pop, &cfg);
        assert!(
            report.composite_satisfaction > 0.0 && report.composite_satisfaction < 1.0,
            "partial supply should give composite in (0, 1), got {}",
            report.composite_satisfaction
        );
    }

    #[test]
    fn stability_delta_smooth_no_cliff_between_0_and_1() {
        // Verify stability_delta is monotonically increasing in composite.
        let recovery = 0.05_f32;
        let decay = 0.10_f32;
        let mut prev = stability_delta_from_satisfaction(0.0, recovery, decay);
        for i in 1..=20u32 {
            let c = i as f32 / 20.0;
            let delta = stability_delta_from_satisfaction(c, recovery, decay);
            assert!(
                delta >= prev,
                "stability_delta must be non-decreasing in composite: delta({c})={delta} < prev={prev}"
            );
            prev = delta;
        }
    }

    #[test]
    fn stability_delta_at_zero_is_max_negative() {
        let recovery = 0.05_f32;
        let decay = 0.10_f32;
        let d = stability_delta_from_satisfaction(0.0, recovery, decay);
        assert!(
            (d - (-decay)).abs() < 1e-5,
            "delta at composite=0 should be -decay_rate, got {d}"
        );
    }

    #[test]
    fn stability_delta_at_one_is_max_positive() {
        let recovery = 0.05_f32;
        let decay = 0.10_f32;
        let d = stability_delta_from_satisfaction(1.0, recovery, decay);
        assert!(
            (d - recovery).abs() < 1e-5,
            "delta at composite=1 should be recovery_rate, got {d}"
        );
    }

    #[test]
    fn higher_satisfaction_gives_higher_stability_delta() {
        let cfg = NeedsConfig::default_survival();
        let d_low = stability_delta_from_satisfaction(
            0.2,
            cfg.stability_recovery_rate,
            cfg.stability_decay_rate,
        );
        let d_high = stability_delta_from_satisfaction(
            0.8,
            cfg.stability_recovery_rate,
            cfg.stability_decay_rate,
        );
        assert!(
            d_high > d_low,
            "80% satisfaction should give higher stability_delta than 20%"
        );
    }

    // ── Integration: multi-turn simulation ──────────────────────────────────

    #[test]
    fn full_supply_colony_grows_over_many_turns() {
        let cfg = NeedsConfig::default_survival();
        let mut pop = 100.0_f64;
        let mut stability = 0.5_f32;

        // Stock a large buffer so we don't run out mid-test.
        let buffer_per_turn = 500.0;
        let turns = 50;

        for _ in 0..turns {
            let mut pool = ColonyPool::new();
            for def in &cfg.needs {
                match &def.scaling {
                    NeedScaling::PerCapita { rate } => {
                        pool.deposit(&def.commodity_id, rate * pop * buffer_per_turn);
                    }
                    NeedScaling::Housing => {
                        pool.deposit(&def.commodity_id, pop * 2.0);
                    }
                }
            }

            let report = apply_needs_check(&mut pool, pop, &cfg);
            stability = (stability + report.stability_delta).clamp(0.0, 1.0);

            let housing_sat = report
                .needs
                .iter()
                .find(|n| n.commodity_id == "housing")
                .map(|n| n.satisfaction)
                .unwrap_or(1.0);

            let pop_delta = apply_population_dynamics(pop, stability, housing_sat, &cfg);
            pop = (pop + pop_delta as f64).max(0.0);
        }

        assert!(
            pop > 100.0,
            "well-supplied colony should grow over 50 turns, pop={pop}"
        );
        assert!(
            stability > 0.5,
            "stability should have improved from starting 0.5, stability={stability}"
        );
    }

    #[test]
    fn starved_colony_loses_stability_then_population() {
        let cfg = NeedsConfig::default_survival();
        let mut pop = 100.0_f64;
        let mut stability = 0.8_f32;

        // No supplies at all
        let turns = 30;
        let initial_stability = stability;

        for _ in 0..turns {
            let mut pool = ColonyPool::new(); // empty every turn
            let report = apply_needs_check(&mut pool, pop, &cfg);
            stability = (stability + report.stability_delta).clamp(0.0, 1.0);

            let housing_sat = report
                .needs
                .iter()
                .find(|n| n.commodity_id == "housing")
                .map(|n| n.satisfaction)
                .unwrap_or(0.0);

            let pop_delta = apply_population_dynamics(pop, stability, housing_sat, &cfg);
            pop = (pop + pop_delta as f64).max(0.0);
        }

        assert!(
            stability < initial_stability,
            "stability should fall from {initial_stability} when starved, got {stability}"
        );
        assert!(
            pop < 100.0,
            "starved colony should lose population over 30 turns, pop={pop}"
        );
    }

    // ── Housing check ────────────────────────────────────────────────────────

    #[test]
    fn housing_check_does_not_consume_from_pool() {
        let mut pool = ColonyPool::new();
        pool.deposit("housing", 200.0);
        let initial = pool.amount("housing");

        let cfg = NeedsConfig::default_survival();
        let _ = apply_needs_check(&mut pool, 100.0, &cfg);

        assert!(
            (pool.amount("housing") - initial).abs() < f64::EPSILON,
            "housing check must not consume from pool"
        );
    }

    #[test]
    fn insufficient_housing_reduces_satisfaction() {
        let cfg = NeedsConfig::default_survival();
        let pop = 100.0;

        let mut pool_low = ColonyPool::new();
        pool_low.deposit("housing", 50.0); // only 50 % housing

        let mut pool_full = ColonyPool::new();
        pool_full.deposit("housing", 200.0);

        let report_low = apply_needs_check(&mut pool_low, pop, &cfg);
        let report_full = apply_needs_check(&mut pool_full, pop, &cfg);

        let housing_low = report_low
            .needs
            .iter()
            .find(|n| n.commodity_id == "housing")
            .unwrap()
            .satisfaction;
        let housing_full = report_full
            .needs
            .iter()
            .find(|n| n.commodity_id == "housing")
            .unwrap()
            .satisfaction;

        assert!(
            housing_low < housing_full,
            "insufficient housing should lower satisfaction"
        );
        assert!(
            (housing_full - 1.0).abs() < 0.01,
            "full housing should give sat≈1.0"
        );
    }

    // ── No growth without housing ────────────────────────────────────────────

    #[test]
    fn no_growth_without_housing() {
        let cfg = NeedsConfig::default_survival();
        let delta = apply_population_dynamics(100.0, 0.95, 0.0, &cfg);
        assert!(
            delta <= 0.0,
            "high stability but no housing should not grow population"
        );
    }

    // ── Neutral band ────────────────────────────────────────────────────────

    #[test]
    fn mid_stability_in_neutral_band_has_no_pop_delta() {
        let cfg = NeedsConfig::default_survival();
        // stability between decline (0.30) and growth (0.70) thresholds
        let delta = apply_population_dynamics(100.0, 0.50, 1.0, &cfg);
        assert!(
            (delta).abs() < f32::EPSILON,
            "stability in neutral band should produce zero pop delta, got {delta}"
        );
    }
}
