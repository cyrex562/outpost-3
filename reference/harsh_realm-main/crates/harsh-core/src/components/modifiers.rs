//! Modifier collection, condition evaluation, and resolution — pure core of
//! `harsh_realm.modifiers`.
//!
//! The host gathers candidate contributions (from traits, statuses, transient
//! sources) and supplies the entity/target tag+trait sets; this module filters
//! by target and condition and aggregates with the Phase-2 stacking rules. The
//! arithmetic mirrors `_resolve_final_value` exactly, including float order, so
//! it is byte-for-byte parity-testable.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ir::{Modifier, ModifierCondition, ModifierPredicate, ModifierStacking};

/// A modifier contribution with its source identified (mirrors the Python model).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifierContribution {
    pub modifier: Modifier,
    pub source_type: String,
    pub source_id: String,
}

/// The state needed to filter modifier contributions (subset of the Python
/// `ResolutionContext` relevant to pure resolution).
#[derive(Debug, Clone, Default)]
pub struct ResolutionContext {
    pub target: String,
    pub entity_id: String,
    pub tick: i64,
    pub entity_tags: BTreeSet<String>,
    pub entity_traits: BTreeSet<String>,
    pub target_tags: BTreeSet<String>,
}

/// Evaluate a modifier condition. Mirrors `modifiers.context.evaluate_condition`:
/// `always` holds unconditionally; the others test the relevant set; a non-`always`
/// predicate without an argument is an error.
pub fn evaluate_condition(
    condition: &ModifierCondition,
    ctx: &ResolutionContext,
) -> Result<bool, String> {
    if condition.predicate == ModifierPredicate::Always {
        return Ok(true);
    }
    let arg = condition.arg.as_ref().ok_or_else(|| {
        format!(
            "Modifier condition predicate {:?} requires arg.",
            condition.predicate
        )
    })?;
    Ok(match condition.predicate {
        ModifierPredicate::EntityHasTag => ctx.entity_tags.contains(arg),
        ModifierPredicate::TargetHasTag => ctx.target_tags.contains(arg),
        ModifierPredicate::EntityHasTrait => ctx.entity_traits.contains(arg),
        ModifierPredicate::Always => true,
    })
}

/// Filter contributions to those matching the context's target whose condition
/// holds. A condition that errors propagates (fail-closed).
pub fn collect(
    ctx: &ResolutionContext,
    contributions: &[ModifierContribution],
) -> Result<Vec<ModifierContribution>, String> {
    let mut out = Vec::new();
    for c in contributions {
        if c.modifier.target == ctx.target && evaluate_condition(&c.modifier.condition, ctx)? {
            out.push(c.clone());
        }
    }
    Ok(out)
}

/// The final value plus the contributions that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedModifierResult {
    pub base_value: i64,
    pub contributions: Vec<ModifierContribution>,
    pub final_value: i64,
}

/// Aggregate already-filtered contributions into a final value, applying the
/// Phase-2 stacking rules in order: replace, additive, multiplicative, then
/// max/min clamps. Exact port of `_resolve_final_value`.
pub fn resolve_final_value(base_value: i64, contributions: &[ModifierContribution]) -> i64 {
    // replace: highest-priority replacement wins; ties keep the FIRST (as Python
    // `max(..., key=priority)` does), so only replace on strictly-greater.
    let mut replacement: Option<&Modifier> = None;
    for c in contributions {
        if c.modifier.stacking == ModifierStacking::Replace {
            match replacement {
                None => replacement = Some(&c.modifier),
                Some(best) if c.modifier.priority > best.priority => {
                    replacement = Some(&c.modifier)
                }
                _ => {}
            }
        }
    }
    let mut final_value = replacement.map(|m| m.value).unwrap_or(base_value);

    // additive
    final_value += contributions
        .iter()
        .filter(|c| c.modifier.stacking == ModifierStacking::Additive)
        .map(|c| c.modifier.value)
        .sum::<i64>();

    // multiplicative (percentage), folded in contribution order to match Python
    let mut multiplier = 1.0_f64;
    for c in contributions {
        if c.modifier.stacking == ModifierStacking::Multiplicative {
            multiplier *= 1.0 + (c.modifier.value as f64 / 100.0);
        }
    }
    // int(final_value * multiplier): truncate toward zero, like Python int().
    final_value = (final_value as f64 * multiplier) as i64;

    // max clamp
    if let Some(m) = contributions
        .iter()
        .filter(|c| c.modifier.stacking == ModifierStacking::Max)
        .map(|c| c.modifier.value)
        .max()
    {
        final_value = final_value.max(m);
    }
    // min clamp
    if let Some(m) = contributions
        .iter()
        .filter(|c| c.modifier.stacking == ModifierStacking::Min)
        .map(|c| c.modifier.value)
        .min()
    {
        final_value = final_value.min(m);
    }

    final_value
}

/// Collect and aggregate in one step.
pub fn resolve(
    ctx: &ResolutionContext,
    base_value: i64,
    contributions: &[ModifierContribution],
) -> Result<ResolvedModifierResult, String> {
    let filtered = collect(ctx, contributions)?;
    let final_value = resolve_final_value(base_value, &filtered);
    Ok(ResolvedModifierResult {
        base_value,
        contributions: filtered,
        final_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contrib(value: i64, stacking: ModifierStacking, priority: i64) -> ModifierContribution {
        ModifierContribution {
            modifier: Modifier {
                target: "skill.stab".into(),
                value,
                stacking,
                priority,
                condition: ModifierCondition::default(),
                description: String::new(),
            },
            source_type: "trait".into(),
            source_id: "t1".into(),
        }
    }

    #[test]
    fn additive_stacks() {
        let c = vec![
            contrib(2, ModifierStacking::Additive, 0),
            contrib(3, ModifierStacking::Additive, 0),
        ];
        assert_eq!(resolve_final_value(1, &c), 6);
    }

    #[test]
    fn replace_takes_highest_priority_then_additive() {
        let c = vec![
            contrib(10, ModifierStacking::Replace, 1),
            contrib(20, ModifierStacking::Replace, 5),
            contrib(2, ModifierStacking::Additive, 0),
        ];
        // replacement 20 (priority 5) + additive 2 = 22
        assert_eq!(resolve_final_value(0, &c), 22);
    }

    #[test]
    fn multiplicative_is_percentage_and_truncates() {
        let c = vec![
            contrib(10, ModifierStacking::Additive, 0), // base 0 + 10 = 10
            contrib(50, ModifierStacking::Multiplicative, 0), // *1.5 = 15
        ];
        assert_eq!(resolve_final_value(0, &c), 15);
    }

    #[test]
    fn max_and_min_clamps() {
        let lo = vec![contrib(5, ModifierStacking::Max, 0)];
        assert_eq!(resolve_final_value(1, &lo), 5); // clamp up to 5
        let hi = vec![contrib(3, ModifierStacking::Min, 0)];
        assert_eq!(resolve_final_value(10, &hi), 3); // clamp down to 3
    }

    #[test]
    fn condition_filtering() {
        let mut ctx = ResolutionContext {
            target: "skill.stab".into(),
            entity_id: "pc".into(),
            ..Default::default()
        };
        ctx.entity_tags.insert("undead".into());
        let mut c = contrib(2, ModifierStacking::Additive, 0);
        c.modifier = Modifier {
            condition: ModifierCondition {
                predicate: ModifierPredicate::EntityHasTag,
                arg: Some("undead".into()),
            },
            ..c.modifier
        };
        assert_eq!(collect(&ctx, &[c.clone()]).unwrap().len(), 1);
        // Different required tag → filtered out.
        let mut c2 = c;
        c2.modifier.condition.arg = Some("angel".into());
        assert!(collect(&ctx, &[c2]).unwrap().is_empty());
    }

    #[test]
    fn non_always_without_arg_errors() {
        let ctx = ResolutionContext {
            target: "x".into(),
            entity_id: "pc".into(),
            ..Default::default()
        };
        let cond = ModifierCondition {
            predicate: ModifierPredicate::EntityHasTag,
            arg: None,
        };
        assert!(evaluate_condition(&cond, &ctx).is_err());
    }
}
