//! Predicate language for the directive and interrupt systems.

use crate::colony::ColonyId;
use serde::{Deserialize, Serialize};

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
}

impl Predicate {
    /// Evaluate this predicate against `ctx`.
    #[must_use]
    pub fn evaluate(&self, ctx: &PredicateContext) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Threshold { metric, cmp, threshold } => cmp.apply(metric.resolve(ctx), *threshold),
            Self::And { left, right } => left.evaluate(ctx) && right.evaluate(ctx),
            Self::Or  { left, right } => left.evaluate(ctx) || right.evaluate(ctx),
            Self::Not { inner } => !inner.evaluate(ctx),
        }
    }

    /// Shorthand: `metric < threshold`.
    #[must_use]
    pub fn lt(metric: Metric, threshold: f64) -> Self {
        Self::Threshold { metric, cmp: Cmp::Lt, threshold }
    }

    /// Shorthand: `metric > threshold`.
    #[must_use]
    pub fn gt(metric: Metric, threshold: f64) -> Self {
        Self::Threshold { metric, cmp: Cmp::Gt, threshold }
    }

    /// Shorthand: AND of two predicates.
    #[must_use]
    pub fn and(left: Self, right: Self) -> Self {
        Self::And { left: Box::new(left), right: Box::new(right) }
    }

    /// Shorthand: OR of two predicates.
    pub fn or(left: Self, right: Self) -> Self {
        Self::Or { left: Box::new(left), right: Box::new(right) }
    }

    /// Shorthand: NOT of a predicate.
    pub fn not(inner: Self) -> Self {
        Self::Not { inner: Box::new(inner) }
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
        }
    }

    #[test]
    fn always_is_true() { assert!(Predicate::Always.evaluate(&ctx())); }

    #[test]
    fn never_is_false() { assert!(!Predicate::Never.evaluate(&ctx())); }

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
        ).evaluate(&ctx()));
    }

    #[test]
    fn or_combinator() {
        assert!(Predicate::or(Predicate::Never, Predicate::gt(Metric::Population, 100.0)).evaluate(&ctx()));
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
}
