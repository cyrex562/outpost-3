//! Skill-check resolution — pure core of `harsh_realm.engine.skill_checks`.
//!
//! The 2d6 roll stays with the host (RNG); this module ports the deterministic
//! parts: margin classification, the success/margin computation given a roll,
//! and the graduated `deceive` failure delta.

use serde::{Deserialize, Serialize};

/// The outcome band for a skill-check margin. Strings match the Python
/// `classify_margin` return values exactly.
pub fn classify_margin(margin: i64) -> &'static str {
    if margin <= -4 {
        "exceptional_failure"
    } else if margin < 0 {
        "failure"
    } else if margin <= 1 {
        "bare_success"
    } else if margin <= 3 {
        "solid_success"
    } else {
        "exceptional_success"
    }
}

/// The deterministic result of a skill check given an already-rolled 2d6 value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCheckOutcome {
    pub roll: i64,
    pub modifier: i64,
    pub total: i64,
    pub difficulty: i64,
    pub margin: i64,
    pub success: bool,
    pub outcome_key: String,
}

/// Resolve a skill check from a rolled 2d6 value plus modifiers.
///
/// `modifier = skill_level + attr_mod`; `margin = roll + modifier - difficulty`.
pub fn resolve_skill_check(
    roll: i64,
    skill_level: i64,
    attr_mod: i64,
    difficulty: i64,
) -> SkillCheckOutcome {
    let modifier = skill_level + attr_mod;
    let total = roll + modifier;
    let margin = total - difficulty;
    SkillCheckOutcome {
        roll,
        modifier,
        total,
        difficulty,
        margin,
        success: margin >= 0,
        outcome_key: classify_margin(margin).to_string(),
    }
}

/// Graduated `deceive` failure delta (default, table-free path): fail by 1 → -1,
/// fail by 2–3 → -2, fail by 4+ → -3.
pub fn deceive_failure_delta(margin: i64) -> i64 {
    let abs_margin = margin.abs();
    if abs_margin >= 4 {
        -3
    } else if abs_margin >= 2 {
        -2
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_margin_bands() {
        assert_eq!(classify_margin(-4), "exceptional_failure");
        assert_eq!(classify_margin(-5), "exceptional_failure");
        assert_eq!(classify_margin(-1), "failure");
        assert_eq!(classify_margin(0), "bare_success");
        assert_eq!(classify_margin(1), "bare_success");
        assert_eq!(classify_margin(2), "solid_success");
        assert_eq!(classify_margin(3), "solid_success");
        assert_eq!(classify_margin(4), "exceptional_success");
    }

    #[test]
    fn resolve_computes_margin_and_success() {
        let o = resolve_skill_check(8, 1, 2, 10);
        assert_eq!(o.modifier, 3);
        assert_eq!(o.total, 11);
        assert_eq!(o.margin, 1);
        assert!(o.success);
        assert_eq!(o.outcome_key, "bare_success");
    }

    #[test]
    fn deceive_graduated() {
        assert_eq!(deceive_failure_delta(-1), -1);
        assert_eq!(deceive_failure_delta(-2), -2);
        assert_eq!(deceive_failure_delta(-3), -2);
        assert_eq!(deceive_failure_delta(-4), -3);
    }
}
