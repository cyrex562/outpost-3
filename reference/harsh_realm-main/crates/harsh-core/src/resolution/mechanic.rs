//! The roll-mechanic registry — `resolve(rolled, modifier, tn) -> ResolutionResult`.
//!
//! A mechanic owns the comparison direction, crit/fumble rules, and degree
//! banding. *Which* mechanic an action/world uses is data; the *set* is code.
//! The dice themselves are rolled by the host (the engine stays pure), so each
//! mechanic takes the already-rolled value plus the summed modifier.
//!
//! The XWN mechanics here are the R2 resolvers re-expressed to produce the
//! unified [`ResolutionResult`]; an in-crate parity test pins that they agree
//! with `resolvers::resolve_hit` / `resolve_skill_check` / `resolve_save`, so the
//! Python-parity already proven for those transitively covers these.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::resolution::result::ResolutionResult;

/// XWN degree band width: every 4 points of margin beyond the success boundary
/// is one degree (matching XWN's "exceptional at ±4" convention). Tunable per
/// mechanic if other systems need finer/coarser bands.
const XWN_BAND_WIDTH: i64 = 4;

/// The registered roll mechanics. New systems (GURPS roll-under, Savage exploding
/// trait dice) are added here when their packs become real.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RollMechanic {
    /// `2d6 + modifier ≥ tn`; XWN skill checks. No crit/fumble.
    Xwn2d6,
    /// `d20 + modifier ≥ tn`; nat 1 = fumble (always miss), nat 20 = crit (always hit).
    XwnD20Attack,
    /// `d20 + modifier ≥ tn`; XWN saving throws. No crit/fumble.
    XwnD20Save,
}

impl RollMechanic {
    /// Resolve this mechanic by id (the data-selectable form).
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "xwn_2d6" => Some(RollMechanic::Xwn2d6),
            "xwn_d20_attack" => Some(RollMechanic::XwnD20Attack),
            "xwn_d20_save" => Some(RollMechanic::XwnD20Save),
            _ => None,
        }
    }

    /// The wire id for this mechanic.
    pub fn id(self) -> &'static str {
        match self {
            RollMechanic::Xwn2d6 => "xwn_2d6",
            RollMechanic::XwnD20Attack => "xwn_d20_attack",
            RollMechanic::XwnD20Save => "xwn_d20_save",
        }
    }

    /// Resolve a contest: `rolled` is the raw die value (2d6 sum, or d20), `modifier`
    /// the summed bonus, `tn` the target number.
    pub fn resolve(self, rolled: i64, modifier: i64, tn: i64) -> ResolutionResult {
        let total = rolled + modifier;
        let raw_margin = total - tn;
        match self {
            RollMechanic::Xwn2d6 => ResolutionResult {
                success: total >= tn,
                raw_margin,
                degree: band(raw_margin),
                crit: false,
                fumble: false,
                rolled,
                total,
                tn,
            },
            RollMechanic::XwnD20Save => ResolutionResult {
                success: total >= tn,
                raw_margin,
                degree: band(raw_margin),
                crit: false,
                fumble: false,
                rolled,
                total,
                tn,
            },
            RollMechanic::XwnD20Attack => {
                let fumble = rolled == 1;
                let crit = rolled == 20;
                // Nat 1 always misses; nat 20 always hits; else compare.
                let success = if fumble {
                    false
                } else if crit {
                    true
                } else {
                    total >= tn
                };
                ResolutionResult {
                    success,
                    raw_margin,
                    degree: band(raw_margin),
                    crit,
                    fumble,
                    rolled,
                    total,
                    tn,
                }
            }
        }
    }
}

/// XWN degree banding: signed full-band steps of [`XWN_BAND_WIDTH`], truncated
/// toward zero so it is symmetric (`-3..=3 → 0`, `±4..±7 → ±1`, …).
fn band(raw_margin: i64) -> i64 {
    raw_margin / XWN_BAND_WIDTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_round_trips() {
        for m in [
            RollMechanic::Xwn2d6,
            RollMechanic::XwnD20Attack,
            RollMechanic::XwnD20Save,
        ] {
            assert_eq!(RollMechanic::from_id(m.id()), Some(m));
        }
        assert_eq!(RollMechanic::from_id("gurps_3d6_under"), None);
    }

    #[test]
    fn d20_attack_nat_1_and_20_override() {
        // Nat 1 misses even against a trivial TN.
        let r = RollMechanic::XwnD20Attack.resolve(1, 100, 5);
        assert!(!r.success && r.fumble && !r.crit);
        // Nat 20 hits even against an impossible TN.
        let r = RollMechanic::XwnD20Attack.resolve(20, -100, 50);
        assert!(r.success && r.crit && !r.fumble);
        // Ordinary: 10 + 5 = 15 vs 15 → hit.
        assert!(RollMechanic::XwnD20Attack.resolve(10, 5, 15).success);
        assert!(!RollMechanic::XwnD20Attack.resolve(10, 4, 15).success);
    }

    #[test]
    fn margin_and_degree_banding() {
        // 2d6=8, +3 = 11 vs 8 → margin 3 → degree 0 (bare/solid band).
        let r = RollMechanic::Xwn2d6.resolve(8, 3, 8);
        assert_eq!(r.raw_margin, 3);
        assert_eq!(r.degree, 0);
        // margin 4 → degree +1; margin -4 → degree -1 (symmetric).
        assert_eq!(RollMechanic::Xwn2d6.resolve(9, 3, 8).degree, 1); // margin 4
        assert_eq!(RollMechanic::Xwn2d6.resolve(2, 3, 9).degree, -1); // total 5 vs 9 = -4
    }

    #[test]
    fn save_has_no_crit_or_fumble() {
        let r = RollMechanic::XwnD20Save.resolve(1, 0, 15);
        assert!(!r.crit && !r.fumble && !r.success);
        let r = RollMechanic::XwnD20Save.resolve(20, 0, 15);
        assert!(!r.crit && r.success);
    }
}
