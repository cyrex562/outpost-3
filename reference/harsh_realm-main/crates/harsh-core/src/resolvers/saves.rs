//! Saving-throw resolution — pure core of `harsh_realm.engine.saves`.
//!
//! The d20 roll stays with the host; this ports the target/modifier math and
//! pass determination, plus the save-type → attribute mapping.

use serde::{Deserialize, Serialize};

/// The attribute modifier key that applies to each save type (`""` = none).
/// Mirrors `_SAVE_STAT_MAP`.
pub fn save_stat_key(save_type: &str) -> &'static str {
    match save_type {
        "physical" => "con",
        "evasion" => "dex",
        "mental" => "wis",
        _ => "", // luck (and unknown) get no stat modifier
    }
}

/// The deterministic result of a save given an already-rolled d20 value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveOutcome {
    pub roll: i64,
    pub modifier: i64,
    pub total: i64,
    pub target: i64,
    pub passed: bool,
}

/// Resolve a save: `total = roll + stat_mod + bonus`; passes when
/// `total >= base_target + difficulty_modifier`.
pub fn resolve_save(
    roll: i64,
    stat_mod: i64,
    bonus: i64,
    base_target: i64,
    difficulty_modifier: i64,
) -> SaveOutcome {
    let modifier = stat_mod + bonus;
    let total = roll + modifier;
    let target = base_target + difficulty_modifier;
    SaveOutcome {
        roll,
        modifier,
        total,
        target,
        passed: total >= target,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_keys_match_python_map() {
        assert_eq!(save_stat_key("physical"), "con");
        assert_eq!(save_stat_key("evasion"), "dex");
        assert_eq!(save_stat_key("mental"), "wis");
        assert_eq!(save_stat_key("luck"), "");
        assert_eq!(save_stat_key("bogus"), "");
    }

    #[test]
    fn save_passes_at_or_above_target() {
        let o = resolve_save(15, 1, 0, 15, 0);
        assert_eq!(o.total, 16);
        assert!(o.passed);
        let fail = resolve_save(1, 0, 0, 15, 0);
        assert!(!fail.passed);
    }

    #[test]
    fn difficulty_modifier_raises_target() {
        let o = resolve_save(15, 0, 0, 15, 2);
        assert_eq!(o.target, 17);
        assert!(!o.passed);
    }

    #[test]
    fn bonus_adds_to_modifier_not_subtracts() {
        // Tests with bonus=0 cannot catch a mutation that changes + to - in
        // the `modifier = stat_mod + bonus` line (:38:29).
        let o = resolve_save(10, 2, 3, 20, 0);
        // modifier = stat_mod(2) + bonus(3) = 5
        assert_eq!(o.modifier, 5);
        assert_eq!(o.total, 15); // 10 + 5
        // With - mutation: modifier = 2-3 = -1, total = 9 → different.

        // Verify with negative bonus to confirm sign handling.
        let o2 = resolve_save(10, 2, -1, 12, 0);
        assert_eq!(o2.modifier, 1); // 2 + (-1)
        assert_eq!(o2.total, 11);
    }
}
