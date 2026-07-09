//! Contest resolution + reaction application + outcome keying.
//!
//! The host materializes a [`ContestRequest`] (mechanic, the rolled die, the
//! summed modifier, and the target number resolved from its source), calls
//! [`resolve_contest`], then — at the reaction window — may fold a
//! [`ReactionEffect`] back into the in-flight result before selecting an outcome
//! branch via [`outcome_key`]. Dice and condition/target materialization are the
//! host's job; this stays pure.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::resolution::mechanic::RollMechanic;
use crate::resolution::result::ResolutionResult;

/// Who rolls in a contest (documentation + host targeting; the math is the same).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Roller {
    Actor,
    Defender,
}

/// A fully-materialized contest: the host has rolled the die, summed the
/// modifier (skill + attribute + gathered modifiers), and resolved the target
/// number from its source (difficulty / a target's defense / an opposed roll /
/// an effect DC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContestRequest {
    pub mechanic: RollMechanic,
    pub roller: Roller,
    pub rolled: i64,
    pub modifier: i64,
    pub tn: i64,
}

/// Resolve a contest into the unified margin currency.
pub fn resolve_contest(req: &ContestRequest) -> ResolutionResult {
    req.mechanic.resolve(req.rolled, req.modifier, req.tn)
}

/// How a reaction folds back into an in-flight result (the `set_event` semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionEffect {
    /// Retroactively change the target number (e.g. a parry raises the attack's
    /// effective TN); success/margin/degree are recomputed from the same roll.
    SetTn(i64),
    /// Cancel the result outright (a successful parry negates the hit, even a crit).
    Cancel,
}

/// Apply a reaction effect to an in-flight result, returning the new result.
///
/// `mechanic` is the mechanic that produced `result` — needed to recompute
/// crit/fumble/banding when the TN changes.
pub fn apply_reaction(
    result: &ResolutionResult,
    mechanic: RollMechanic,
    effect: ReactionEffect,
) -> ResolutionResult {
    match effect {
        ReactionEffect::SetTn(new_tn) => {
            // Recover the modifier (total = rolled + modifier) and re-resolve.
            let modifier = result.total - result.rolled;
            mechanic.resolve(result.rolled, modifier, new_tn)
        }
        ReactionEffect::Cancel => ResolutionResult {
            success: false,
            ..result.clone()
        },
    }
}

/// The outcome branch key for a result: one of `fumble | miss | crit | success`.
///
/// Checked success-first so a cancelled crit (success forced false) keys as a
/// `miss`, not a `crit`. Branches that key on `degree`/`raw_margin` read those
/// fields directly off the [`ResolutionResult`].
pub fn outcome_key(result: &ResolutionResult) -> &'static str {
    if result.fumble {
        "fumble"
    } else if !result.success {
        "miss"
    } else if result.crit {
        "crit"
    } else {
        "success"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attack(rolled: i64, modifier: i64, tn: i64) -> ContestRequest {
        ContestRequest {
            mechanic: RollMechanic::XwnD20Attack,
            roller: Roller::Actor,
            rolled,
            modifier,
            tn,
        }
    }

    #[test]
    fn resolve_then_key() {
        // 18 + 2 = 20 vs 15 → hit.
        let r = resolve_contest(&attack(18, 2, 15));
        assert!(r.success);
        assert_eq!(outcome_key(&r), "success");
        // nat 20 → crit.
        assert_eq!(outcome_key(&resolve_contest(&attack(20, 0, 15))), "crit");
        // nat 1 → fumble.
        assert_eq!(outcome_key(&resolve_contest(&attack(1, 100, 5))), "fumble");
        // ordinary miss.
        assert_eq!(outcome_key(&resolve_contest(&attack(5, 0, 15))), "miss");
    }

    #[test]
    fn parry_set_tn_can_turn_a_hit_into_a_miss() {
        let hit = resolve_contest(&attack(12, 3, 15)); // 15 vs 15 → hit
        assert!(hit.success);
        // Parry raises the effective TN to 18 → now a miss, same roll.
        let parried = apply_reaction(&hit, RollMechanic::XwnD20Attack, ReactionEffect::SetTn(18));
        assert!(!parried.success);
        assert_eq!(parried.raw_margin, 15 - 18);
        assert_eq!(outcome_key(&parried), "miss");
    }

    #[test]
    fn cancel_negates_even_a_crit() {
        let crit = resolve_contest(&attack(20, 0, 15));
        assert!(crit.crit && crit.success);
        let cancelled = apply_reaction(&crit, RollMechanic::XwnD20Attack, ReactionEffect::Cancel);
        assert!(!cancelled.success);
        // Roll was still a 20, but the outcome keys as a miss because success was negated.
        assert_eq!(outcome_key(&cancelled), "miss");
    }
}
