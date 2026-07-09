//! XWN dice rolling, ported from `harsh_realm.engine.dice`.
//!
//! Behaviour is kept faithful to the Python implementation: the same validation
//! rules, the same 4d6-drop-lowest attribute roll, and the same attribute
//! modifier table.

use rand::Rng;
use serde::{Deserialize, Serialize};

/// The detailed outcome of a dice roll.
///
/// Mirrors the Python `DiceResult` model. `final_total` corresponds to the
/// Python field `final` (which is a reserved word in some contexts and an
/// awkward field name in Rust), serialized back to `final` for the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiceResult {
    /// Individual die faces that count toward the total.
    pub rolls: Vec<u32>,
    /// Sum of `rolls`, before the modifier.
    pub total: i32,
    /// Flat modifier added to `total`.
    pub modifier: i32,
    /// `total + modifier`.
    #[serde(rename = "final")]
    pub final_total: i32,
}

/// Error returned for invalid roll parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiceError {
    /// `num_dice` was less than 1.
    TooFewDice(u32),
    /// `sides` was less than 2.
    TooFewSides(u32),
}

impl std::fmt::Display for DiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiceError::TooFewDice(n) => write!(f, "num_dice must be >= 1, got {n}"),
            DiceError::TooFewSides(n) => write!(f, "sides must be >= 2, got {n}"),
        }
    }
}

impl std::error::Error for DiceError {}

/// XWN dice roller, generic over any [`rand::Rng`] source.
///
/// Pass a seeded RNG (e.g. `rand::rngs::SmallRng::seed_from_u64`) for
/// deterministic tests, mirroring the Python roller's optional `random.Random`.
pub struct DiceRoller<R: Rng> {
    rng: R,
}

impl<R: Rng> DiceRoller<R> {
    /// Create a roller from a given RNG source.
    pub fn new(rng: R) -> Self {
        Self { rng }
    }

    /// Roll `num_dice` dice of `sides` faces plus a flat `modifier`.
    ///
    /// Returns [`DiceError`] for `num_dice < 1` or `sides < 2`, matching the
    /// Python implementation's `ValueError` guards.
    pub fn roll(
        &mut self,
        num_dice: u32,
        sides: u32,
        modifier: i32,
    ) -> Result<DiceResult, DiceError> {
        if num_dice < 1 {
            return Err(DiceError::TooFewDice(num_dice));
        }
        if sides < 2 {
            return Err(DiceError::TooFewSides(sides));
        }

        let rolls: Vec<u32> = (0..num_dice)
            .map(|_| self.rng.gen_range(1..=sides))
            .collect();
        let total: i32 = rolls.iter().map(|&r| r as i32).sum();
        Ok(DiceResult {
            rolls,
            total,
            modifier,
            final_total: total + modifier,
        })
    }

    /// Roll 4d6, drop the lowest die. `rolls` holds only the three kept dice.
    pub fn roll_4d6_drop_lowest(&mut self) -> DiceResult {
        let mut all: Vec<u32> = (0..4).map(|_| self.rng.gen_range(1..=6)).collect();
        all.sort_unstable();
        let kept: Vec<u32> = all[1..].to_vec(); // drop lowest
        let total: i32 = kept.iter().map(|&r| r as i32).sum();
        DiceResult {
            rolls: kept,
            total,
            modifier: 0,
            final_total: total,
        }
    }

    /// Roll six attribute scores via 4d6-drop-lowest (each in 3..=18).
    pub fn roll_attribute_set(&mut self) -> Vec<i32> {
        (0..6)
            .map(|_| self.roll_4d6_drop_lowest().final_total)
            .collect()
    }
}

/// Return the XWN attribute modifier for a score.
///
/// Table: `<=3 -> -2`, `4..=7 -> -1`, `8..=13 -> 0`, `14..=17 -> +1`,
/// `>=18 -> +2`.
pub fn attr_modifier(score: i32) -> i32 {
    if score <= 3 {
        -2
    } else if score <= 7 {
        -1
    } else if score <= 13 {
        0
    } else if score <= 17 {
        1
    } else {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn roller(seed: u64) -> DiceRoller<SmallRng> {
        DiceRoller::new(SmallRng::seed_from_u64(seed))
    }

    #[test]
    fn roll_rejects_too_few_dice() {
        assert_eq!(roller(1).roll(0, 6, 0), Err(DiceError::TooFewDice(0)));
    }

    #[test]
    fn roll_rejects_too_few_sides() {
        assert_eq!(roller(1).roll(1, 1, 0), Err(DiceError::TooFewSides(1)));
    }

    #[test]
    fn roll_applies_modifier_and_sums() {
        let result = roller(42).roll(3, 6, 2).unwrap();
        assert_eq!(result.rolls.len(), 3);
        assert_eq!(
            result.total,
            result.rolls.iter().map(|&r| r as i32).sum::<i32>()
        );
        assert_eq!(result.final_total, result.total + 2);
        assert_eq!(result.modifier, 2);
    }

    #[test]
    fn roll_stays_within_face_bounds() {
        let mut r = roller(7);
        for _ in 0..1000 {
            let res = r.roll(5, 8, 0).unwrap();
            assert_eq!(res.rolls.len(), 5);
            for face in &res.rolls {
                assert!((1..=8).contains(face), "face {face} out of range");
            }
        }
    }

    #[test]
    fn four_d6_drop_lowest_keeps_three_in_range() {
        let mut r = roller(99);
        for _ in 0..1000 {
            let res = r.roll_4d6_drop_lowest();
            assert_eq!(res.rolls.len(), 3);
            assert!((3..=18).contains(&res.final_total));
            assert_eq!(res.modifier, 0);
            assert_eq!(res.final_total, res.total);
        }
    }

    #[test]
    fn attribute_set_has_six_scores_in_range() {
        let scores = roller(5).roll_attribute_set();
        assert_eq!(scores.len(), 6);
        for s in scores {
            assert!((3..=18).contains(&s));
        }
    }

    #[test]
    fn attr_modifier_table_matches_python() {
        // Exhaustively pin the same boundaries as the Python implementation.
        assert_eq!(attr_modifier(3), -2);
        assert_eq!(attr_modifier(1), -2); // clamp below 3
        assert_eq!(attr_modifier(4), -1);
        assert_eq!(attr_modifier(7), -1);
        assert_eq!(attr_modifier(8), 0);
        assert_eq!(attr_modifier(13), 0);
        assert_eq!(attr_modifier(14), 1);
        assert_eq!(attr_modifier(17), 1);
        assert_eq!(attr_modifier(18), 2);
        assert_eq!(attr_modifier(20), 2); // clamp above 18
    }

    #[test]
    fn dice_result_serializes_final_field() {
        let result = DiceResult {
            rolls: vec![3, 4],
            total: 7,
            modifier: 1,
            final_total: 8,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"final\":8"), "got {json}");
        assert!(!json.contains("final_total"));
    }

    #[test]
    fn roll_accepts_two_sided_die() {
        // sides == 2 is a valid coin flip; sides < 2 is the rejection threshold.
        let result = roller(3).roll(1, 2, 0).unwrap();
        assert!((1..=2).contains(&result.rolls[0]));
    }

    #[test]
    fn dice_error_display_messages() {
        // Verify the Display impl produces useful text, not empty/default output.
        let too_few_dice = DiceError::TooFewDice(0);
        let msg = too_few_dice.to_string();
        assert!(msg.contains("num_dice"), "got: {msg}");
        assert!(msg.contains('0'), "got: {msg}");

        let too_few_sides = DiceError::TooFewSides(1);
        let msg2 = too_few_sides.to_string();
        assert!(msg2.contains("sides"), "got: {msg2}");
        assert!(msg2.contains('1'), "got: {msg2}");
    }
}
