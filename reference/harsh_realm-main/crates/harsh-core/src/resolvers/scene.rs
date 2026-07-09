//! Small scene resolutions (R5-R10): first aid, pre-combat awareness, and the
//! flee check. Each is deterministic given the host's dice rolls — the Python
//! scenes roll, then apply the outcome (narration, movement, RNG consequences
//! stay host).
//!
//! System-specific values (difficulties, bases, floors) are parameters supplied by
//! the host, not hardcoded — the engine stays system-agnostic (R5-R: constants out
//! of the engine into host config/content).

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FirstAidOutcome {
    pub total: i64,
    pub success: bool,
    pub hp_restored: i64,
    pub new_hp: i64,
    pub actual_restored: i64,
}

/// Resolve a first-aid attempt. `roll` is the 2d6 check; `heal_roll` the 1d6 heal
/// die (only used on success); `difficulty` is supplied by the host. Untrained
/// heal (-1) contributes 0 to healing.
pub fn resolve_first_aid(
    roll: i64,
    heal_level: i64,
    wis_mod: i64,
    heal_roll: i64,
    current_hp: i64,
    max_hp: i64,
    difficulty: i64,
) -> FirstAidOutcome {
    let total = roll + heal_level + wis_mod;
    let success = total >= difficulty;
    if !success {
        return FirstAidOutcome {
            total,
            success: false,
            hp_restored: 0,
            new_hp: current_hp,
            actual_restored: 0,
        };
    }
    let skill_bonus = heal_level.max(0);
    let hp_restored = (heal_roll + skill_bonus).max(1);
    let new_hp = (current_hp + hp_restored).min(max_hp);
    FirstAidOutcome {
        total,
        success: true,
        hp_restored,
        new_hp,
        actual_restored: new_hp - current_hp,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AwarenessOutcome {
    pub player_total: i64,
    pub player_succeeded: bool,
    pub enemy_difficulty: i64,
    pub enemy_succeeded: bool,
    /// `player_surprise` | `enemy_surprise` | `mutual_awareness`.
    pub result: String,
}

/// Resolve a pre-combat awareness check from both sides' 2d6 rolls. The enemy's
/// detection difficulty is `max(enemy_floor, enemy_base + sneak_level)`, both
/// supplied by the host.
#[allow(clippy::too_many_arguments)] // a flat numeric resolver; a struct adds noise
pub fn resolve_awareness(
    player_roll: i64,
    skill_level: i64,
    wis_mod: i64,
    awareness_difficulty: i64,
    enemy_roll: i64,
    sneak_level: i64,
    enemy_base: i64,
    enemy_floor: i64,
) -> AwarenessOutcome {
    let player_total = player_roll + skill_level + wis_mod;
    let player_succeeded = player_total >= awareness_difficulty;
    // Higher sneak makes the player harder to detect; clamp to the floor.
    let enemy_difficulty = (enemy_base + sneak_level).max(enemy_floor);
    let enemy_succeeded = enemy_roll >= enemy_difficulty;
    let result = if player_succeeded && !enemy_succeeded {
        "player_surprise"
    } else if !player_succeeded && enemy_succeeded {
        "enemy_surprise"
    } else {
        "mutual_awareness"
    };
    AwarenessOutcome {
        player_total,
        player_succeeded,
        enemy_difficulty,
        enemy_succeeded,
        result: result.to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FleeOutcome {
    pub best_mod: i64,
    pub total: i64,
    pub clean: bool,
}

/// Resolve the flee skill check: best of (Exert+STR)/(Sneak+DEX), 2d6 + mod vs
/// difficulty. A `messy` flee can never be clean.
pub fn resolve_flee_check(
    roll: i64,
    exert_mod: i64,
    sneak_mod: i64,
    difficulty: i64,
    messy: bool,
) -> FleeOutcome {
    let best_mod = exert_mod.max(sneak_mod);
    let total = roll + best_mod;
    FleeOutcome {
        best_mod,
        total,
        clean: !messy && total >= difficulty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_aid_success_heals_and_clamps() {
        // total = 6 + 1 + 1 = 8 >= 8 → success; restore max(1, 4 + max(0,1)) = 5,
        // clamped to max_hp.
        let o = resolve_first_aid(6, 1, 1, 4, 8, 10, 8); // difficulty 8
        assert!(o.success);
        assert_eq!(o.hp_restored, 5);
        assert_eq!(o.new_hp, 10); // 8 + 5 clamped to 10
        assert_eq!(o.actual_restored, 2);
    }

    #[test]
    fn first_aid_failure_heals_nothing() {
        let o = resolve_first_aid(3, -1, 0, 6, 4, 10, 8);
        assert!(!o.success);
        assert_eq!(o.hp_restored, 0);
        assert_eq!(o.new_hp, 4);
    }

    #[test]
    fn awareness_classifies_all_three() {
        // player succeeds (roll 12 + 0 + 0 = 12 >= 8), enemy fails (roll 2 < diff).
        let p = resolve_awareness(12, 0, 0, 8, 2, 0, 8, 2);
        assert_eq!(p.result, "player_surprise");
        // player fails, enemy succeeds.
        let e = resolve_awareness(2, 0, 0, 8, 12, 0, 8, 2);
        assert_eq!(e.result, "enemy_surprise");
        // both succeed → mutual.
        let m = resolve_awareness(12, 0, 0, 8, 12, 0, 8, 2);
        assert_eq!(m.result, "mutual_awareness");
        assert_eq!(p.enemy_difficulty, 8); // base 8 + sneak 0
    }

    #[test]
    fn flee_uses_best_mod_and_respects_messy() {
        let o = resolve_flee_check(7, 1, 3, 9, false); // best 3, total 10 >= 9
        assert_eq!(o.best_mod, 3);
        assert!(o.clean);
        // messy can never be clean even on a high total.
        assert!(!resolve_flee_check(12, 5, 5, 6, true).clean);
    }
}
