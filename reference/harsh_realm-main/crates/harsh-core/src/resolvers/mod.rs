//! Pure rule resolvers ported from `harsh_realm.engine`.
//!
//! Each function here is deterministic: dice rolls are taken by the host and
//! passed in, so the resolvers can be exactly parity-tested against their Python
//! counterparts. RNG-driven entry points stay in the host until a later phase.

pub mod advancement;
pub mod character;
pub mod combat;
pub mod faction_ai;
pub mod saves;
pub mod scene;
pub mod skill_check;

pub use advancement::{
    attack_bonus_for_class, can_level_up, hit_die_for_class, hp_gained, saves_improved,
    skill_points_for_class, xp_for_level, DEFAULT_XP_TABLE,
};
pub use character::{recalculate, RecalcInput, RecalcOutput};
pub use faction_ai::{choose_action as choose_faction_action, ActionChoice, FactionTurnInput};
pub use combat::{
    damage_total, parse_damage_expr, resolve_hit, resolve_shock, DamageOutcome, HitOutcome,
};
pub use saves::{resolve_save, save_stat_key, SaveOutcome};
pub use scene::{
    resolve_awareness, resolve_first_aid, resolve_flee_check, AwarenessOutcome, FirstAidOutcome,
    FleeOutcome,
};
pub use skill_check::{
    classify_margin, deceive_failure_delta, resolve_skill_check, SkillCheckOutcome,
};

#[cfg(test)]
mod properties {
    //! Invariant tests over input grids (exhaustive small ranges — stronger than
    //! random sampling for these integer domains).
    use super::*;

    #[test]
    fn skill_check_success_iff_nonnegative_margin() {
        for roll in 2..=12 {
            for skill in -2..=4 {
                for attr in -2..=2 {
                    for diff in 2..=15 {
                        let o = resolve_skill_check(roll, skill, attr, diff);
                        assert_eq!(o.success, o.margin >= 0);
                        assert_eq!(o.outcome_key, classify_margin(o.margin));
                        assert_eq!(o.margin, roll + skill + attr - diff);
                    }
                }
            }
        }
    }

    #[test]
    fn attack_bonus_monotone_nondecreasing_in_level() {
        for class in ["warrior", "expert", "adventurer", "unknown"] {
            for level in 1..20 {
                assert!(
                    attack_bonus_for_class(class, level + 1)
                        >= attack_bonus_for_class(class, level),
                    "{class} regressed at level {level}"
                );
            }
        }
    }

    #[test]
    fn hit_invariants_over_grid() {
        for roll in 1..=20 {
            for bonus in -5..=10 {
                for ac in 5..=25 {
                    let o = resolve_hit(roll, bonus, 0, 0, ac);
                    match roll {
                        1 => assert!(!o.hit, "nat 1 must miss"),
                        20 => assert!(o.hit, "nat 20 must hit"),
                        _ => assert_eq!(o.hit, o.total >= ac),
                    }
                }
            }
        }
    }

    #[test]
    fn damage_total_never_below_one() {
        for dice in 0..=20 {
            for flat in -10..=10 {
                for attr in -5..=5 {
                    for warrior in [false, true] {
                        let d = damage_total(dice, flat, attr, warrior, 5);
                        assert!(d.total >= 1);
                    }
                }
            }
        }
    }

    #[test]
    fn save_passes_iff_total_at_least_target() {
        for roll in 1..=20 {
            for mods in -5..=5 {
                for target in 2..=20 {
                    let o = resolve_save(roll, mods, 0, target, 0);
                    assert_eq!(o.passed, o.total >= o.target);
                }
            }
        }
    }
}
