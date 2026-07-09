//! Faction reputation system — pure scoring and disposition logic.
//!
//! Ported from `src/harsh_realm/faction/reputation.py`.
//!
//! The async `ReputationSystem` (DB + EventBus) is B9 (web host); only the
//! pure functions are ported here.

/// Reputation deltas for player actions.
pub const REPUTATION_DELTAS: &[(&str, i32)] = &[
    ("kill_faction_member", -10),
    ("complete_faction_task", 15),
    ("bribe_faction_member", 5),
    ("destroy_faction_asset", -15),
    ("help_faction_ally", 10),
];

/// Convert a reputation score to a faction disposition label.
///
/// Thresholds (from `faction_turns.md`):
/// - ≤ −30 → hostile
/// - −29 to −10 → unfriendly
/// - −9 to +9 → neutral
/// - +10 to +29 → friendly
/// - ≥ +30 → allied
pub fn reputation_to_disposition(score: i32) -> &'static str {
    if score <= -30 {
        "hostile"
    } else if score <= -10 {
        "unfriendly"
    } else if score <= 9 {
        "neutral"
    } else if score <= 29 {
        "friendly"
    } else {
        "allied"
    }
}

/// Return the delta for a named player action, or 0 if unknown.
pub fn delta_for_action(action: &str) -> i32 {
    REPUTATION_DELTAS
        .iter()
        .find(|(k, _)| *k == action)
        .map(|(_, v)| *v)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostile_at_minus_30() {
        assert_eq!(reputation_to_disposition(-30), "hostile");
        assert_eq!(reputation_to_disposition(-100), "hostile");
    }

    #[test]
    fn unfriendly_range() {
        assert_eq!(reputation_to_disposition(-29), "unfriendly");
        assert_eq!(reputation_to_disposition(-10), "unfriendly");
    }

    #[test]
    fn neutral_range() {
        assert_eq!(reputation_to_disposition(-9), "neutral");
        assert_eq!(reputation_to_disposition(0), "neutral");
        assert_eq!(reputation_to_disposition(9), "neutral");
    }

    #[test]
    fn friendly_range() {
        assert_eq!(reputation_to_disposition(10), "friendly");
        assert_eq!(reputation_to_disposition(29), "friendly");
    }

    #[test]
    fn allied_at_30() {
        assert_eq!(reputation_to_disposition(30), "allied");
        assert_eq!(reputation_to_disposition(100), "allied");
    }

    #[test]
    fn delta_for_known_action() {
        assert_eq!(delta_for_action("kill_faction_member"), -10);
        assert_eq!(delta_for_action("complete_faction_task"), 15);
    }

    #[test]
    fn delta_for_unknown_action_is_zero() {
        assert_eq!(delta_for_action("unknown_action"), 0);
    }
}
