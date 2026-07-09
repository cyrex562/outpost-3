//! Support types and helpers for faction turns.
//!
//! Ported from `src/harsh_realm/faction/turn_support.py`.

/// Actions that produce GameEvents (vs silent bookkeeping actions).
pub const SIGNIFICANT_ACTIONS: &[&str] = &["attack", "expand", "seize", "create", "sell"];

/// Parse a dice expression like `1d6` or `2d8+1`.
///
/// Returns 0 for empty or unparseable strings. Uses a deterministic RNG via
/// the caller-supplied `roll_fn` (takes dice sides, returns 1..=sides).
pub fn parse_dice_with<F>(expr: &str, mut roll_fn: F) -> i32
where
    F: FnMut(i32) -> i32,
{
    let expr = expr.trim();
    if expr.is_empty() {
        return 0;
    }
    // Pattern: NdS or NdS±M
    let lower = expr.to_lowercase();
    let d_pos = match lower.find('d') {
        Some(p) => p,
        None => return 0,
    };
    let num_str = &lower[..d_pos];
    let rest = &lower[d_pos + 1..];
    let num: i32 = num_str.parse().unwrap_or(0);
    if num <= 0 {
        return 0;
    }
    let (sides_str, modifier) = if let Some(plus) = rest.find('+') {
        let m: i32 = rest[plus + 1..].parse().unwrap_or(0);
        (&rest[..plus], m)
    } else if let Some(minus) = rest.find('-') {
        let m: i32 = rest[minus + 1..].parse().unwrap_or(0);
        (&rest[..minus], -m)
    } else {
        (rest, 0)
    };
    let sides: i32 = sides_str.parse().unwrap_or(0);
    if sides <= 0 {
        return 0;
    }
    let total: i32 = (0..num).map(|_| roll_fn(sides)).sum();
    total + modifier
}

/// Parse a dice expression using a simple sequential roll (max for tests).
/// For production use, supply a seeded RNG via `parse_dice_with`.
pub fn parse_dice_max(expr: &str) -> i32 {
    parse_dice_with(expr, |sides| sides)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_is_zero() {
        assert_eq!(parse_dice_with("", |s| s), 0);
    }

    #[test]
    fn parse_1d6_max() {
        // max roll always gives 6
        assert_eq!(parse_dice_with("1d6", |s| s), 6);
    }

    #[test]
    fn parse_2d8_plus1_min() {
        // min roll (always 1) for 2d8+1 = 2+1 = 3
        assert_eq!(parse_dice_with("2d8+1", |_| 1), 3);
    }

    #[test]
    fn parse_1d6_minus1_min() {
        assert_eq!(parse_dice_with("1d6-1", |_| 1), 0);
    }

    #[test]
    fn parse_invalid_is_zero() {
        assert_eq!(parse_dice_with("not_dice", |s| s), 0);
    }

    #[test]
    fn significant_actions_contains_attack() {
        assert!(SIGNIFICANT_ACTIONS.contains(&"attack"));
        assert!(!SIGNIFICANT_ACTIONS.contains(&"repair"));
        assert!(!SIGNIFICANT_ACTIONS.contains(&"harvest"));
    }
}
