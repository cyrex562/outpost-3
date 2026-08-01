//! Morale resolution — issue #382.
//!
//! Morale is a colony-wide scalar **separate from [`crate::population::PopulationPool::stability`]**:
//! stability tracks life-support survival needs (food, water, oxygen, housing,
//! power, per `crate::needs`), morale tracks quality-of-life inputs. The two
//! stay independent per issue #382's design decision — folding them into one
//! composite would conflate "can the colony physically survive" with "is life
//! here good," which #348's own design note treats as distinct concepts.
//!
//! This module deliberately mirrors `crate::needs`'s shape (a weighted list of
//! per-capita [`crate::needs::NeedDef`]s feeding a smooth quadratic delta via
//! [`crate::needs::stability_delta_from_satisfaction`], reused here unchanged
//! since the curve is generic in its inputs) rather than sharing one pipeline
//! with it, so a future morale input (e.g. the food-variety bonus #348/#382
//! explicitly deferred) can be added as another [`crate::needs::NeedDef`]
//! without touching the survival-needs config at all.
//!
//! Today morale has exactly one input: a per-capita `networking_compute`
//! draw, the "networking/compute... consumed by... population morale" utility
//! from `docs/DESIGN.md §5A`.

use crate::colony::ColonyStores;
use crate::needs::{
    compute_composite, stability_delta_from_satisfaction, NeedDef, NeedSatisfaction, NeedScaling,
};

/// Tunable parameters for the morale subsystem.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MoraleConfig {
    /// Ordered list of inputs feeding the morale composite.
    pub needs: Vec<NeedDef>,
    /// Maximum morale gained per turn at 100% satisfaction (`[0.0, 1.0]`).
    pub recovery_rate: f32,
    /// Maximum morale lost per turn at 0% satisfaction (`[0.0, 1.0]`).
    pub decay_rate: f32,
}

impl MoraleConfig {
    /// The default morale configuration: a per-capita `networking_compute`
    /// draw, the sole input decided for issue #382 (a food-variety bonus was
    /// explicitly deferred — see the module doc comment).
    #[must_use]
    pub fn default_config() -> Self {
        Self {
            needs: vec![NeedDef {
                commodity_id: "networking_compute".into(),
                scaling: NeedScaling::PerCapita { rate: 0.1 },
                weight: 1.0,
            }],
            recovery_rate: 0.05,
            decay_rate: 0.05,
        }
    }
}

/// Full morale check output for one colony-turn.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MoraleReport {
    /// Per-input satisfaction breakdown.
    pub needs: Vec<NeedSatisfaction>,
    /// Weighted composite satisfaction score, in `[0.0, 1.0]`.
    pub composite_satisfaction: f32,
    /// Morale change to apply this turn (positive = improvement).
    pub morale_delta: f32,
}

/// Apply the per-turn morale check to `stores`, consuming what each input
/// needs and returning a [`MoraleReport`].
///
/// Mirrors [`crate::needs::apply_needs_check_scaled`]'s consumption logic —
/// see its doc comment for the store-routing and shortfall behavior, both
/// identical here. The caller is responsible for clamping the population's
/// `morale` field to `[0.0, 1.0]` after applying `morale_delta`.
#[must_use]
pub fn apply_morale_check_scaled(
    stores: &mut ColonyStores<'_>,
    population: f64,
    config: &MoraleConfig,
    consumption_scalar: f32,
) -> MoraleReport {
    let mut need_results: Vec<NeedSatisfaction> = Vec::with_capacity(config.needs.len());
    let scalar = f64::from(consumption_scalar.max(0.0));

    for def in &config.needs {
        let base = def.required_amount(population);
        let required = match &def.scaling {
            NeedScaling::PerCapita { .. } => base * scalar,
            NeedScaling::Housing => base,
        };

        let (consumed, satisfaction) = if required <= 0.0 {
            (0.0, 1.0_f32)
        } else {
            match &def.scaling {
                NeedScaling::PerCapita { .. } => {
                    let consumed = stores.withdraw(&def.commodity_id, required);
                    #[allow(clippy::cast_possible_truncation)]
                    let sat = (consumed / required).clamp(0.0, 1.0) as f32;
                    (consumed, sat)
                }
                NeedScaling::Housing => {
                    let capacity = stores.amount(&def.commodity_id);
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
    let morale_delta =
        stability_delta_from_satisfaction(composite, config.recovery_rate, config.decay_rate);

    MoraleReport {
        needs: need_results,
        composite_satisfaction: composite,
        morale_delta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::{ColonyPool, ColonyResourcePool};
    use crate::content::ContentRegistry;

    fn check(pool: &mut ColonyPool, population: f64, config: &MoraleConfig) -> MoraleReport {
        let registry = ContentRegistry::default();
        let mut resources = ColonyResourcePool::new();
        apply_morale_check_scaled(
            &mut ColonyStores::new(pool, &mut resources, &registry),
            population,
            config,
            1.0,
        )
    }

    #[test]
    fn full_compute_supply_gives_positive_morale_delta() {
        let cfg = MoraleConfig::default_config();
        let mut pool = ColonyPool::new();
        pool.deposit("networking_compute", 1000.0); // comfortably over demand
        let report = check(&mut pool, 100.0, &cfg);
        assert!(
            (report.composite_satisfaction - 1.0).abs() < 0.01,
            "full compute supply should give composite ≈ 1.0, got {}",
            report.composite_satisfaction
        );
        assert!(
            report.morale_delta > 0.0,
            "full compute supply should give positive morale_delta, got {}",
            report.morale_delta
        );
    }

    #[test]
    fn no_compute_supply_gives_negative_morale_delta() {
        let cfg = MoraleConfig::default_config();
        let mut pool = ColonyPool::new(); // empty
        let report = check(&mut pool, 100.0, &cfg);
        assert!(
            report.composite_satisfaction < 0.05,
            "no supply should give composite ≈ 0, got {}",
            report.composite_satisfaction
        );
        assert!(
            report.morale_delta < 0.0,
            "no compute supply should give negative morale_delta, got {}",
            report.morale_delta
        );
    }

    #[test]
    fn partial_compute_supply_gives_intermediate_composite() {
        let cfg = MoraleConfig::default_config();
        let mut pool = ColonyPool::new();
        // Exactly 50% of demand (rate 0.1 * pop 100 = 10 required).
        pool.deposit("networking_compute", 5.0);
        let report = check(&mut pool, 100.0, &cfg);
        assert!(
            report.composite_satisfaction > 0.0 && report.composite_satisfaction < 1.0,
            "partial supply should give composite in (0, 1), got {}",
            report.composite_satisfaction
        );
    }

    #[test]
    fn zero_population_is_vacuously_satisfied() {
        let cfg = MoraleConfig::default_config();
        let mut pool = ColonyPool::new();
        let report = check(&mut pool, 0.0, &cfg);
        assert!(
            (report.composite_satisfaction - 1.0).abs() < 1e-6,
            "zero population should need nothing, got {}",
            report.composite_satisfaction
        );
    }
}
