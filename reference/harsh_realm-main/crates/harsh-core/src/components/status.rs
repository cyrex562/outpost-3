//! Status-effect lifecycle — pure core of `harsh_realm.status_effects.service`.
//!
//! Durable rows are still written by Python; this module computes the
//! deterministic lifecycle: expiry ticks, the `extend` stacking expiry, the
//! due-for-expiry predicate, and the set of [`Intent::RemoveStatus`] a tick
//! produces. Applying the resulting intents is the host's job.

use serde::{Deserialize, Serialize};

use crate::intent::Intent;

/// A minimal view of an applied status effect (lifecycle-relevant fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveStatus {
    pub entity_id: String,
    pub effect_id: String,
    /// Absolute tick at which this expires; `None` means permanent.
    pub expires_at_tick: Option<i64>,
}

/// Expiry tick for a freshly-applied effect: `None` if `duration == 0`
/// (permanent), else `applied_at_tick + duration`.
pub fn expiry_tick(applied_at_tick: i64, duration: i64) -> Option<i64> {
    if duration == 0 {
        None
    } else {
        Some(applied_at_tick + duration)
    }
}

/// New expiry under `extend` stacking. Mirrors `_extended_expiry`: a permanent
/// existing effect (or a zero duration) stays permanent; otherwise extend from
/// the later of the existing expiry and now.
pub fn extended_expiry(
    existing_expiry: Option<i64>,
    current_tick: i64,
    duration: i64,
) -> Option<i64> {
    match existing_expiry {
        Some(existing) if duration != 0 => Some(existing.max(current_tick) + duration),
        _ => None,
    }
}

/// Whether an effect is due to expire at `current_tick` (has an expiry that is
/// at or before now).
pub fn is_due(expires_at_tick: Option<i64>, current_tick: i64) -> bool {
    matches!(expires_at_tick, Some(e) if e <= current_tick)
}

/// The [`Intent::RemoveStatus`] intents for every effect due at `current_tick`,
/// in input order.
pub fn expirations(active: &[ActiveStatus], current_tick: i64) -> Vec<Intent> {
    active
        .iter()
        .filter(|s| is_due(s.expires_at_tick, current_tick))
        .map(|s| Intent::RemoveStatus {
            entity_id: s.entity_id.clone(),
            status_id: s.effect_id.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expiry_tick_zero_is_permanent() {
        assert_eq!(expiry_tick(10, 0), None);
        assert_eq!(expiry_tick(10, 3), Some(13));
    }

    #[test]
    fn extended_expiry_rules() {
        // permanent existing stays permanent
        assert_eq!(extended_expiry(None, 5, 3), None);
        // zero duration becomes permanent
        assert_eq!(extended_expiry(Some(10), 5, 0), None);
        // extend from the later of existing/now
        assert_eq!(extended_expiry(Some(10), 5, 3), Some(13));
        assert_eq!(extended_expiry(Some(4), 8, 3), Some(11)); // now (8) > existing (4)
    }

    #[test]
    fn due_predicate_and_expirations() {
        assert!(is_due(Some(5), 5));
        assert!(is_due(Some(4), 5));
        assert!(!is_due(Some(6), 5));
        assert!(!is_due(None, 5)); // permanent never due

        let active = vec![
            ActiveStatus {
                entity_id: "pc".into(),
                effect_id: "poisoned".into(),
                expires_at_tick: Some(5),
            },
            ActiveStatus {
                entity_id: "pc".into(),
                effect_id: "blessed".into(),
                expires_at_tick: Some(99),
            },
            ActiveStatus {
                entity_id: "pc".into(),
                effect_id: "cursed".into(),
                expires_at_tick: None,
            },
        ];
        let intents = expirations(&active, 10);
        assert_eq!(
            intents,
            vec![Intent::RemoveStatus {
                entity_id: "pc".into(),
                status_id: "poisoned".into(),
            }]
        );
    }
}
