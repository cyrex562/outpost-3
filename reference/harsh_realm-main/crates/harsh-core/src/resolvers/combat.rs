//! Combat math — pure core of `harsh_realm.engine.damage` and
//! `engine.combat.resolvers`.
//!
//! Dice rolls stay with the host; this ports damage-expression parsing, hit
//! determination given a d20, the damage-total formula given a dice sum, and
//! shock damage.

use serde::{Deserialize, Serialize};

/// Parsed components of a damage expression: `(num_dice, sides, flat_modifier)`.
pub type DamageExpr = (u32, u32, i32);

/// Parse a damage expression like `"1d6"`, `"2d8+2"`, `"1d4-1"`.
///
/// Matches the Python regex `^(\d+)d(\d+)([+-]\d+)?$` (case-insensitive on `d`).
pub fn parse_damage_expr(expr: &str) -> Result<DamageExpr, String> {
    let s = expr.trim();
    let bytes = s.as_bytes();
    let mut i = 0;

    let num_dice = take_uint(bytes, &mut i)
        .ok_or_else(|| format!("Cannot parse damage expression: {expr:?}"))?;

    match bytes.get(i) {
        Some(b'd') | Some(b'D') => i += 1,
        _ => return Err(format!("Cannot parse damage expression: {expr:?}")),
    }

    let sides = take_uint(bytes, &mut i)
        .ok_or_else(|| format!("Cannot parse damage expression: {expr:?}"))?;

    let mut flat: i32 = 0;
    if i < bytes.len() {
        let sign = match bytes[i] {
            b'+' => 1,
            b'-' => -1,
            _ => return Err(format!("Cannot parse damage expression: {expr:?}")),
        };
        i += 1;
        let magnitude = take_uint(bytes, &mut i)
            .ok_or_else(|| format!("Cannot parse damage expression: {expr:?}"))?;
        flat = sign * magnitude as i32;
    }

    if i != bytes.len() {
        return Err(format!("Cannot parse damage expression: {expr:?}"));
    }
    Ok((num_dice, sides, flat))
}

fn take_uint(bytes: &[u8], i: &mut usize) -> Option<u32> {
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_digit() {
        *i += 1; // cargo-mutants: replacing += with *= causes an infinite loop (0*=1==0); untestable via assertions
    }
    if *i == start {
        return None;
    }
    std::str::from_utf8(&bytes[start..*i]).ok()?.parse().ok()
}

/// The deterministic outcome of an attack roll.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitOutcome {
    pub roll: i64,
    pub total: i64,
    pub target_ac: i64,
    pub hit: bool,
    pub natural_1: bool,
    pub natural_20: bool,
}

/// Resolve a hit from a rolled d20. Natural 1 always misses, natural 20 always
/// hits; otherwise `roll + attack_bonus + skill_level + attr_mod >= target_ac`.
pub fn resolve_hit(
    roll: i64,
    attack_bonus: i64,
    skill_level: i64,
    attr_mod: i64,
    target_ac: i64,
) -> HitOutcome {
    let total = roll + attack_bonus + skill_level + attr_mod;
    let natural_1 = roll == 1;
    let natural_20 = roll == 20;
    let hit = if natural_1 {
        false
    } else if natural_20 {
        true
    } else {
        total >= target_ac
    };
    HitOutcome {
        roll,
        total,
        target_ac,
        hit,
        natural_1,
        natural_20,
    }
}

/// The deterministic damage result given an already-rolled dice sum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageOutcome {
    /// `dice_sum + flat_mod` (matches the Python `roll` field).
    pub roll: i64,
    pub modifier: i64,
    pub total: i64,
}

/// Compute damage from a dice sum: `total = max(1, dice_sum + flat_mod + attr_mod
/// + warrior_bonus)`, where the warrior bonus is `ceil(level/2)` if `is_warrior`.
pub fn damage_total(
    dice_sum: i64,
    flat_mod: i64,
    attr_mod: i64,
    is_warrior: bool,
    level: i64,
) -> DamageOutcome {
    let raw_total = dice_sum + flat_mod;
    let mut modifier = attr_mod;
    if is_warrior {
        modifier += (level + 1).div_euclid(2); // ceil(level/2) for level >= 0
    }
    DamageOutcome {
        roll: dice_sum + flat_mod,
        modifier,
        total: (raw_total + modifier).max(1),
    }
}

/// Shock damage dealt on a melee miss: `max(0, shock_damage + str_modifier)` when
/// the weapon has shock and the target AC is at or below the shock threshold.
pub fn resolve_shock(
    str_modifier: i64,
    shock_damage: i64,
    shock_ac_threshold: i64,
    target_ac: i64,
) -> i64 {
    if shock_damage == 0 || target_ac > shock_ac_threshold {
        return 0;
    }
    (shock_damage + str_modifier).max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_damage_expressions() {
        assert_eq!(parse_damage_expr("1d6"), Ok((1, 6, 0)));
        assert_eq!(parse_damage_expr("2d8+2"), Ok((2, 8, 2)));
        assert_eq!(parse_damage_expr("1d4-1"), Ok((1, 4, -1)));
        assert_eq!(parse_damage_expr(" 3D10 "), Ok((3, 10, 0)));
    }

    #[test]
    fn rejects_bad_damage_expressions() {
        for bad in ["d6", "1d", "1x6", "1d6+", "1d6+2x", "abc", ""] {
            assert!(parse_damage_expr(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn natural_rolls_override_ac() {
        assert!(!resolve_hit(1, 100, 0, 0, 5).hit, "nat 1 always misses");
        assert!(resolve_hit(20, -100, 0, 0, 50).hit, "nat 20 always hits");
        assert!(resolve_hit(10, 5, 0, 0, 15).hit); // 15 >= 15
        assert!(!resolve_hit(10, 4, 0, 0, 15).hit); // 14 < 15
    }

    #[test]
    fn damage_total_floors_at_one_and_adds_warrior_bonus() {
        // dice 1, flat -5, no mods → raw -4 → floored to 1.
        assert_eq!(damage_total(1, -5, 0, false, 1).total, 1);
        // warrior level 3 → ceil(3/2)=2 bonus.
        let d = damage_total(4, 0, 1, true, 3);
        assert_eq!(d.modifier, 3); // attr 1 + warrior 2
        assert_eq!(d.total, 7);
    }

    #[test]
    fn shock_rules() {
        assert_eq!(resolve_shock(1, 2, 15, 10), 3); // applies (AC below threshold)
        assert_eq!(resolve_shock(1, 0, 15, 10), 0); // no shock weapon
        assert_eq!(resolve_shock(1, 2, 15, 16), 0); // AC above threshold
        assert_eq!(resolve_shock(-5, 2, 15, 10), 0); // floored at 0
        // Shock applies when target_ac == shock_ac_threshold (boundary: > not >=).
        assert_eq!(resolve_shock(1, 2, 15, 15), 3);
    }

    #[test]
    fn resolve_hit_all_bonus_terms_contribute() {
        // Exercises all four additive terms so that mutations to any operator
        // produce a wrong total and flip the hit/miss boundary.

        // kill :82:37 (+ between attack_bonus and skill_level → -):
        // roll=3, attack=4, skill=3, attr=0 → total=10, AC=10 → hit
        // mutation: 3+4-3+0=4 < 10 → miss → caught.
        assert!(resolve_hit(3, 4, 3, 0, 10).hit);

        // kill :82:51 (+ between skill_level and attr_mod → -):
        // roll=7, attack=0, skill=3, attr=1 → total=11, AC=11 → hit
        // mutation(-): 7+0+3-1=9 < 11 → miss → caught.
        assert!(resolve_hit(7, 0, 3, 1, 11).hit);

        // kill :82:51 (+ between skill_level and attr_mod → *):
        // same row: 7+0+3*1=7+3=10 < 11 → miss → caught.
        // (operator precedence: * binds tighter, so result is 7+0+(3*1)=10)

        // Confirm a marginal miss so the boundary is tight.
        assert!(!resolve_hit(7, 0, 3, 0, 11).hit); // 7+0+3+0=10 < 11
    }

    #[test]
    fn hit_outcome_fields_are_accurate() {
        let h = resolve_hit(12, 3, 1, 1, 17);
        assert_eq!(h.roll, 12);
        assert_eq!(h.total, 17); // 12+3+1+1
        assert_eq!(h.target_ac, 17);
        assert!(h.hit); // 17 >= 17
        assert!(!h.natural_1);
        assert!(!h.natural_20);
    }

    #[test]
    fn damage_outcome_roll_field_uses_flat_mod() {
        // The `roll` field in DamageOutcome is `dice_sum + flat_mod`.
        // Tests using flat_mod=0 cannot catch mutations to that operator.
        let d = damage_total(5, 3, 0, false, 1);
        assert_eq!(d.roll, 8); // 5 + 3
        assert_eq!(d.total, 8);

        let d_neg = damage_total(5, -2, 0, false, 1);
        assert_eq!(d_neg.roll, 3); // 5 + (-2)
        assert_eq!(d_neg.total, 3);
    }

    #[test]
    fn parse_damage_expr_all_components_correct() {
        // Verify that each component is parsed and placed correctly.
        let (n, s, f) = parse_damage_expr("3d8+4").unwrap();
        assert_eq!(n, 3);
        assert_eq!(s, 8);
        assert_eq!(f, 4);

        let (n, s, f) = parse_damage_expr("2d6-2").unwrap();
        assert_eq!(n, 2);
        assert_eq!(s, 6);
        assert_eq!(f, -2);
    }
}
