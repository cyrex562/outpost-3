//! Predicate language for the directive and interrupt systems.

use std::collections::HashMap;

use crate::colony::ColonyId;
use serde::{Deserialize, Serialize};

/// Per-commodity snapshot used by commodity predicates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommoditySnapshot {
    /// Current stockpile amount.
    pub amount: f64,
    /// Net change last turn (positive = surplus, negative = deficit).
    pub delta: f64,
}

/// A snapshot of colony-level observable state for predicate evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredicateContext {
    /// Colony this context belongs to.
    pub colony_id: ColonyId,
    /// Current population count.
    pub population: f32,
    /// Stability in `[0.0, 1.0]`.
    pub stability: f32,
    /// Available labour units.
    pub available_labour: f32,
    /// System-wide accumulated research total.
    pub system_research: f32,
    /// Current colony-sol turn counter.
    pub sol: u64,
    /// Current strategic-month counter.
    pub month: u64,
    /// Per-commodity stockpile snapshot for commodity predicates.
    #[serde(default)]
    pub commodities: HashMap<String, CommoditySnapshot>,
}

/// A measurable quantity that a predicate can test.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "metric")]
pub enum Metric {
    /// Colony fractional population count.
    Population,
    /// Colony stability in `[0.0, 1.0]`.
    Stability,
    /// Colony available labour units.
    AvailableLabour,
    /// Current colony-sol counter.
    Sol,
    /// System-wide accumulated research total.
    SystemResearch,
}

impl Metric {
    /// Resolve this metric to a concrete `f64` from `ctx`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn resolve(&self, ctx: &PredicateContext) -> f64 {
        match self {
            Self::Population => f64::from(ctx.population),
            Self::Stability => f64::from(ctx.stability),
            Self::AvailableLabour => f64::from(ctx.available_labour),
            Self::Sol => ctx.sol as f64,
            Self::SystemResearch => f64::from(ctx.system_research),
        }
    }
}

/// A comparison operator used in leaf predicates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Cmp {
    /// Less-than.
    Lt,
    /// Less-than-or-equal.
    Le,
    /// Greater-than.
    Gt,
    /// Greater-than-or-equal.
    Ge,
    /// Equal (within 1e-9 epsilon).
    Eq,
}

impl Cmp {
    /// Apply this comparison to two `f64` operands.
    #[must_use]
    pub fn apply(&self, lhs: f64, rhs: f64) -> bool {
        match self {
            Self::Lt => lhs < rhs,
            Self::Le => lhs <= rhs,
            Self::Gt => lhs > rhs,
            Self::Ge => lhs >= rhs,
            Self::Eq => (lhs - rhs).abs() < 1e-9,
        }
    }
}

/// A boolean predicate evaluated against a [`PredicateContext`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Predicate {
    /// Always true.
    Always,
    /// Always false.
    Never,
    /// Leaf: `metric cmp threshold`.
    Threshold {
        /// Metric to sample from context.
        metric: Metric,
        /// Comparison operator.
        cmp: Cmp,
        /// Right-hand threshold value.
        threshold: f64,
    },
    /// Logical AND.
    And {
        /// Left operand.
        left: Box<Predicate>,
        /// Right operand.
        right: Box<Predicate>,
    },
    /// Logical OR.
    Or {
        /// Left operand.
        left: Box<Predicate>,
        /// Right operand.
        right: Box<Predicate>,
    },
    /// Logical NOT.
    Not {
        /// Inner predicate to negate.
        inner: Box<Predicate>,
    },
    /// True when the named commodity's stockpile is strictly below `threshold`.
    StockpileBelow {
        /// Commodity to check.
        commodity_id: String,
        /// Stockpile threshold (exclusive lower bound).
        threshold: f32,
    },
    /// True when the named commodity's stockpile is strictly above `threshold`.
    StockpileAbove {
        /// Commodity to check.
        commodity_id: String,
        /// Stockpile threshold (exclusive upper bound).
        threshold: f32,
    },
    /// True when the commodity will exhaust within `sols` turns at the current burn rate.
    ///
    /// Uses linear trend extrapolation on `pool.delta`; never triggers if delta >= 0.
    EtaToExhaustion {
        /// Commodity to monitor.
        commodity_id: String,
        /// Alert horizon in colony-sols.
        sols: u64,
    },
    /// True when the commodity had a net-negative delta last turn (production < consumption).
    ProductionShortfall {
        /// Commodity to check.
        commodity_id: String,
    },
}

impl Predicate {
    /// Evaluate this predicate against `ctx`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn evaluate(&self, ctx: &PredicateContext) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Threshold {
                metric,
                cmp,
                threshold,
            } => cmp.apply(metric.resolve(ctx), *threshold),
            Self::And { left, right } => left.evaluate(ctx) && right.evaluate(ctx),
            Self::Or { left, right } => left.evaluate(ctx) || right.evaluate(ctx),
            Self::Not { inner } => !inner.evaluate(ctx),
            Self::StockpileBelow {
                commodity_id,
                threshold,
            } => {
                let amount = ctx
                    .commodities
                    .get(commodity_id)
                    .map_or(0.0, |s| s.amount);
                amount < f64::from(*threshold)
            }
            Self::StockpileAbove {
                commodity_id,
                threshold,
            } => {
                let amount = ctx
                    .commodities
                    .get(commodity_id)
                    .map_or(0.0, |s| s.amount);
                amount > f64::from(*threshold)
            }
            Self::EtaToExhaustion { commodity_id, sols } => {
                let snap = ctx.commodities.get(commodity_id);
                let (amount, delta) = snap.map_or((0.0, 0.0), |s| (s.amount, s.delta));
                // Only fire when actively depleting (delta < 0).
                if delta >= 0.0 {
                    return false;
                }
                // turns_until_zero = amount / burn_rate
                let burn_rate = -delta; // positive
                let turns_until_zero = amount / burn_rate;
                turns_until_zero < *sols as f64
            }
            Self::ProductionShortfall { commodity_id } => {
                let delta = ctx
                    .commodities
                    .get(commodity_id)
                    .map_or(0.0, |s| s.delta);
                delta < 0.0
            }
        }
    }

    /// Shorthand: `metric < threshold`.
    #[must_use]
    pub fn lt(metric: Metric, threshold: f64) -> Self {
        Self::Threshold {
            metric,
            cmp: Cmp::Lt,
            threshold,
        }
    }

    /// Shorthand: `metric > threshold`.
    #[must_use]
    pub fn gt(metric: Metric, threshold: f64) -> Self {
        Self::Threshold {
            metric,
            cmp: Cmp::Gt,
            threshold,
        }
    }

    /// Shorthand: AND of two predicates.
    #[must_use]
    pub fn and(left: Self, right: Self) -> Self {
        Self::And {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Shorthand: OR of two predicates.
    #[must_use]
    pub fn or(left: Self, right: Self) -> Self {
        Self::Or {
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// Shorthand: NOT of a predicate.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn not(inner: Self) -> Self {
        Self::Not {
            inner: Box::new(inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn ctx() -> PredicateContext {
        PredicateContext {
            colony_id: Uuid::new_v4(),
            population: 150.0,
            stability: 0.75,
            available_labour: 120.0,
            system_research: 42.0,
            sol: 10,
            month: 0,
            commodities: HashMap::new(),
        }
    }

    fn ctx_with_commodity(id: &str, amount: f64, delta: f64) -> PredicateContext {
        let mut c = ctx();
        c.commodities.insert(
            id.to_owned(),
            CommoditySnapshot { amount, delta },
        );
        c
    }

    #[test]
    fn always_is_true() {
        assert!(Predicate::Always.evaluate(&ctx()));
    }

    #[test]
    fn never_is_false() {
        assert!(!Predicate::Never.evaluate(&ctx()));
    }

    #[test]
    fn threshold_lt() {
        assert!(Predicate::lt(Metric::Stability, 0.8).evaluate(&ctx()));
        assert!(!Predicate::lt(Metric::Stability, 0.5).evaluate(&ctx()));
    }

    #[test]
    fn threshold_gt() {
        assert!(Predicate::gt(Metric::Population, 100.0).evaluate(&ctx()));
        assert!(!Predicate::gt(Metric::Population, 200.0).evaluate(&ctx()));
    }

    #[test]
    fn and_combinator() {
        let p = Predicate::and(
            Predicate::gt(Metric::Population, 100.0),
            Predicate::lt(Metric::Stability, 0.8),
        );
        assert!(p.evaluate(&ctx()));
        assert!(!Predicate::and(
            Predicate::gt(Metric::Population, 100.0),
            Predicate::gt(Metric::Stability, 0.9),
        )
        .evaluate(&ctx()));
    }

    #[test]
    fn or_combinator() {
        assert!(
            Predicate::or(Predicate::Never, Predicate::gt(Metric::Population, 100.0))
                .evaluate(&ctx())
        );
        assert!(!Predicate::or(Predicate::Never, Predicate::Never).evaluate(&ctx()));
    }

    #[test]
    fn not_combinator() {
        assert!(Predicate::not(Predicate::Never).evaluate(&ctx()));
        assert!(!Predicate::not(Predicate::Always).evaluate(&ctx()));
    }

    #[test]
    fn predicate_round_trip_serde() {
        let p = Predicate::and(
            Predicate::lt(Metric::Stability, 0.5),
            Predicate::gt(Metric::Population, 50.0),
        );
        let json = serde_json::to_string(&p).unwrap();
        let back: Predicate = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    // ── StockpileBelow ────────────────────────────────────────────────────

    #[test]
    fn stockpile_below_fires_when_amount_is_below_threshold() {
        let ctx = ctx_with_commodity("food", 30.0, -5.0);
        let pred = Predicate::StockpileBelow {
            commodity_id: "food".into(),
            threshold: 50.0,
        };
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn stockpile_below_does_not_fire_when_at_or_above_threshold() {
        let ctx = ctx_with_commodity("food", 50.0, 0.0);
        let pred = Predicate::StockpileBelow {
            commodity_id: "food".into(),
            threshold: 50.0,
        };
        assert!(!pred.evaluate(&ctx)); // strictly below

        let ctx2 = ctx_with_commodity("food", 80.0, 0.0);
        assert!(!pred.evaluate(&ctx2));
    }

    #[test]
    fn stockpile_below_treats_unknown_commodity_as_zero() {
        let ctx = ctx(); // no commodities
        let pred = Predicate::StockpileBelow {
            commodity_id: "water".into(),
            threshold: 1.0,
        };
        assert!(pred.evaluate(&ctx)); // 0 < 1
    }

    // ── StockpileAbove ────────────────────────────────────────────────────

    #[test]
    fn stockpile_above_fires_when_amount_exceeds_threshold() {
        let ctx = ctx_with_commodity("water", 200.0, 10.0);
        let pred = Predicate::StockpileAbove {
            commodity_id: "water".into(),
            threshold: 100.0,
        };
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn stockpile_above_does_not_fire_when_at_or_below_threshold() {
        let ctx = ctx_with_commodity("water", 100.0, 0.0);
        let pred = Predicate::StockpileAbove {
            commodity_id: "water".into(),
            threshold: 100.0,
        };
        assert!(!pred.evaluate(&ctx)); // strictly above

        let ctx2 = ctx_with_commodity("water", 40.0, 0.0);
        assert!(!pred.evaluate(&ctx2));
    }

    // ── EtaToExhaustion ───────────────────────────────────────────────────

    #[test]
    fn eta_fires_when_exhaustion_is_imminent() {
        // 10 units, burning 5/turn → exhausts in 2 turns; alert at 5 turns → fires
        let ctx = ctx_with_commodity("food", 10.0, -5.0);
        let pred = Predicate::EtaToExhaustion {
            commodity_id: "food".into(),
            sols: 5,
        };
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn eta_does_not_fire_when_exhaustion_is_far_away() {
        // 100 units, burning 1/turn → exhausts in 100 turns; alert at 5 turns → no fire
        let ctx = ctx_with_commodity("food", 100.0, -1.0);
        let pred = Predicate::EtaToExhaustion {
            commodity_id: "food".into(),
            sols: 5,
        };
        assert!(!pred.evaluate(&ctx));
    }

    #[test]
    fn eta_does_not_fire_when_delta_is_non_negative() {
        // Surplus or stable — no exhaustion risk
        let ctx = ctx_with_commodity("food", 10.0, 0.0);
        let pred = Predicate::EtaToExhaustion {
            commodity_id: "food".into(),
            sols: 100,
        };
        assert!(!pred.evaluate(&ctx));

        let ctx2 = ctx_with_commodity("food", 10.0, 5.0);
        assert!(!pred.evaluate(&ctx2));
    }

    #[test]
    fn eta_boundary_exactly_at_horizon() {
        // 5 units, burning 1/turn → exhausts in exactly 5 turns; alert at 5 → fires (< 5 is false)
        let ctx = ctx_with_commodity("oxygen", 5.0, -1.0);
        let pred = Predicate::EtaToExhaustion {
            commodity_id: "oxygen".into(),
            sols: 5,
        };
        // 5.0 / 1.0 = 5.0, 5.0 < 5 is false
        assert!(!pred.evaluate(&ctx));

        // 4.9 units → 4.9 turns < 5 → fires
        let ctx2 = ctx_with_commodity("oxygen", 4.9, -1.0);
        assert!(pred.evaluate(&ctx2));
    }

    // ── ProductionShortfall ───────────────────────────────────────────────

    #[test]
    fn shortfall_fires_when_delta_is_negative() {
        let ctx = ctx_with_commodity("food", 100.0, -3.0);
        let pred = Predicate::ProductionShortfall {
            commodity_id: "food".into(),
        };
        assert!(pred.evaluate(&ctx));
    }

    #[test]
    fn shortfall_does_not_fire_when_balanced_or_surplus() {
        let pred = Predicate::ProductionShortfall {
            commodity_id: "food".into(),
        };
        let ctx_balanced = ctx_with_commodity("food", 100.0, 0.0);
        assert!(!pred.evaluate(&ctx_balanced));

        let ctx_surplus = ctx_with_commodity("food", 100.0, 5.0);
        assert!(!pred.evaluate(&ctx_surplus));
    }

    #[test]
    fn shortfall_treats_unknown_commodity_as_zero_delta() {
        let ctx = ctx(); // no commodities
        let pred = Predicate::ProductionShortfall {
            commodity_id: "minerals".into(),
        };
        // delta defaults to 0.0, not negative → no shortfall
        assert!(!pred.evaluate(&ctx));
    }

    // ── Serde round-trip for new variants ────────────────────────────────

    #[test]
    fn new_predicate_variants_round_trip_serde() {
        let variants: Vec<Predicate> = vec![
            Predicate::StockpileBelow {
                commodity_id: "food".into(),
                threshold: 50.0,
            },
            Predicate::StockpileAbove {
                commodity_id: "water".into(),
                threshold: 200.0,
            },
            Predicate::EtaToExhaustion {
                commodity_id: "oxygen".into(),
                sols: 3,
            },
            Predicate::ProductionShortfall {
                commodity_id: "energy".into(),
            },
        ];
        for p in variants {
            let json = serde_json::to_string(&p).unwrap();
            let back: Predicate = serde_json::from_str(&json).unwrap();
            assert_eq!(p, back);
        }
    }
}
