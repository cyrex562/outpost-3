//! Damage resolution for Harsh Realm combat.
//!
//! Ported from `src/harsh_realm/engine/damage.py`. The result value types live
//! in `engine_results`; this module re-exports them and parses damage
//! expressions.

pub use crate::engine_results::{AttackResult, DamageResult};

/// Parse a damage expression like `"1d6"`, `"2d8+2"`, `"1d4-1"`.
///
/// Returns `(num_dice, sides, flat_modifier)` or an error string.
pub fn parse_damage_expr(expr: &str) -> Result<(i32, i32, i32), String> {
    let s = expr.trim();
    let err = || format!("Cannot parse damage expression: {expr:?}");

    // Split on the (case-insensitive) 'd'.
    let lower = s.to_ascii_lowercase();
    let d_pos = lower.find('d').ok_or_else(err)?;
    let (num_part, rest) = (&s[..d_pos], &s[d_pos + 1..]);

    // The remainder is the sides, optionally followed by a signed modifier.
    let sign_pos = rest.find(['+', '-']);
    let (sides_part, flat_part) = match sign_pos {
        Some(p) => (&rest[..p], Some(&rest[p..])),
        None => (rest, None),
    };

    let num_dice: i32 = num_part.parse().map_err(|_| err())?;
    let sides: i32 = sides_part.parse().map_err(|_| err())?;
    let flat: i32 = match flat_part {
        Some(f) => f.parse().map_err(|_| err())?,
        None => 0,
    };
    Ok((num_dice, sides, flat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_expressions() {
        assert_eq!(parse_damage_expr("1d6").unwrap(), (1, 6, 0));
        assert_eq!(parse_damage_expr("2d8+2").unwrap(), (2, 8, 2));
        assert_eq!(parse_damage_expr("1d4-1").unwrap(), (1, 4, -1));
        assert_eq!(parse_damage_expr(" 3D10 ").unwrap(), (3, 10, 0));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_damage_expr("sword").is_err());
        assert!(parse_damage_expr("d6").is_err());
        assert!(parse_damage_expr("2d").is_err());
    }
}
