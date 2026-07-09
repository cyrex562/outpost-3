//! The resolution layer (R5-P) — the engine's L0 contest machinery.
//!
//! Increment 1: the unified [`ResolutionResult`] margin currency and the
//! [`RollMechanic`] registry. Later increments add the phased interruptible
//! resolver (`resolution.*` stage events + reaction window), the damage/health
//! pipeline, and targeting. See `docs/design/2026-06-19-action-resolution-
//! primitives-design.md`.

pub mod contest;
pub mod damage;
pub mod economy;
pub mod mechanic;
pub mod result;
pub mod stage;
pub mod targeting;

pub use contest::{
    apply_reaction, outcome_key, resolve_contest, ContestRequest, ReactionEffect, Roller,
};
pub use damage::{
    apply_damage, DamageApplication, DamagePacket, DamageTier, Mitigation, MitigationKind, Pool,
    PoolDelta,
};
pub use economy::{can_afford, Activation, Cost, Economy, UseScope, Uses};
pub use mechanic::RollMechanic;
pub use result::ResolutionResult;
pub use stage::Stage;
pub use targeting::{chebyshev_distance, in_range, TargetShape, Targeting};

#[cfg(test)]
mod parity {
    //! The mechanics are the R2 resolvers re-expressed; pin that they agree, so
    //! the Python parity already proven for the R2 resolvers transitively covers
    //! these (no separate cross-language test needed).
    use super::RollMechanic;
    use crate::resolvers::{resolve_hit, resolve_save, resolve_skill_check};

    #[test]
    fn d20_attack_matches_resolve_hit() {
        for roll in 1..=20 {
            for bonus in -3..=8 {
                for ac in 8..=22 {
                    let hit = resolve_hit(roll, bonus, 0, 0, ac);
                    let m = RollMechanic::XwnD20Attack.resolve(roll, bonus, ac);
                    assert_eq!(m.success, hit.hit, "roll={roll} bonus={bonus} ac={ac}");
                    assert_eq!(m.crit, hit.natural_20);
                    assert_eq!(m.fumble, hit.natural_1);
                    assert_eq!(m.total, hit.total);
                }
            }
        }
    }

    #[test]
    fn xwn_2d6_matches_resolve_skill_check() {
        for roll in 2..=12 {
            for skill in -1..=4 {
                for attr in -2..=2 {
                    for diff in 4..=15 {
                        let sc = resolve_skill_check(roll, skill, attr, diff);
                        let m = RollMechanic::Xwn2d6.resolve(roll, skill + attr, diff);
                        assert_eq!(m.success, sc.success);
                        assert_eq!(m.raw_margin, sc.margin);
                        assert_eq!(m.total, sc.total);
                    }
                }
            }
        }
    }

    #[test]
    fn d20_save_matches_resolve_save() {
        for roll in 1..=20 {
            for modifier in -4..=4 {
                for target in 5..=20 {
                    let s = resolve_save(roll, modifier, 0, target, 0);
                    let m = RollMechanic::XwnD20Save.resolve(roll, modifier, target);
                    assert_eq!(m.success, s.passed);
                    assert_eq!(m.total, s.total);
                }
            }
        }
    }
}
