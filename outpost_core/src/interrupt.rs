//! Interrupt tiers, sources, and the "advance until interrupted" fast-forward loop.
//!
//! The interrupt system is the primary player-facing time-control mechanism
//! (see `docs/DESIGN.md §4A, §12A`). The player issues "advance up to N turns"
//! and the sim halts at the first interrupt that meets or exceeds the player's
//! threshold tier, returning a digest of lower-tier items that accumulated.
//!
//! Tiers from highest to lowest:
//! - [`Tier::Blocking`] — requires a decision; rare.
//! - [`Tier::Urgent`]   — stops fast-forward; act or dismiss.
//! - [`Tier::Notable`]  — logged in digest; does not stop fast-forward.
//! - [`Tier::Ambient`]  — background; visible only in the event log.

use serde::{Deserialize, Serialize};

use crate::colony::ColonyId;

/// An opaque event identifier referenced by [`InterruptSource::EventFired`].
pub type EventId = String;

/// Interrupt severity tier, governing when fast-forward halts.
///
/// Variants are ordered from highest to lowest severity so that
/// `Blocking > Urgent > Notable > Ambient` comparisons work with `PartialOrd`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    /// Background noise; visible only in the event log.
    Ambient = 0,
    /// Informational; accumulated in the digest but never halts fast-forward.
    Notable = 1,
    /// Halts fast-forward and hands control back; act or dismiss.
    Urgent = 2,
    /// Requires an immediate decision before the sim can continue.
    Blocking = 3,
}

/// The origin of an [`Interrupt`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
#[non_exhaustive]
pub enum InterruptSource {
    /// Predictive trajectory warning: a metric is declining and will cross a
    /// critical threshold in approximately `eta_turns` turns.
    PredictiveWarning {
        /// Current value of the metric that is declining (e.g. stability).
        quantity: f32,
        /// Estimated turns until the metric hits the crisis threshold.
        eta_turns: u32,
    },
    /// An in-world event was fired.
    EventFired(EventId),
    /// A construction project completed in the colony.
    ConstructionComplete,
    /// A tech node was unlocked.
    TechUnlocked,
    /// The colony's stability dropped below the critical floor.
    StabilityCritical(ColonyId),
}

/// A single interrupt emitted during turn processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interrupt {
    /// Severity tier; governs whether fast-forward halts.
    pub tier: Tier,
    /// What caused this interrupt.
    pub source: InterruptSource,
    /// Colony this interrupt belongs to, if colony-scoped.
    pub colony_id: Option<ColonyId>,
    /// Human-readable description shown to the player.
    pub message: String,
}

impl Interrupt {
    /// Construct a new interrupt.
    #[must_use]
    pub fn new(
        tier: Tier,
        source: InterruptSource,
        colony_id: Option<ColonyId>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            tier,
            source,
            colony_id,
            message: message.into(),
        }
    }
}

/// The outcome of [`crate::GameEngine::advance_until_interrupted`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdvanceResult {
    /// Fast-forward halted because an interrupt met or exceeded the threshold.
    Halted {
        /// Sol turn at which the halt occurred.
        at_turn: u64,
        /// The interrupt that triggered the halt.
        interrupt: Interrupt,
        /// Digest of `Notable` and `Ambient` interrupts that accumulated before
        /// the halt.
        digest: Vec<Interrupt>,
    },
    /// All N turns completed without a halt-level interrupt.
    Completed {
        /// Number of turns actually advanced.
        turns_advanced: u32,
        /// Digest of `Notable` and `Ambient` interrupts that accumulated.
        digest: Vec<Interrupt>,
    },
}

/// Per-colony stability tracking used to compute predictive warning trajectory.
///
/// Stores the last [`StabilityTracker::WINDOW`] stability samples so that
/// linear trend extrapolation stays O(1) per turn.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StabilityTracker {
    /// Recent stability samples, oldest first.
    pub samples: std::collections::VecDeque<f32>,
}

impl StabilityTracker {
    /// Number of samples retained for trend extrapolation.
    pub const WINDOW: usize = 5;

    /// Record a new stability sample.
    pub fn push(&mut self, stability: f32) {
        self.samples.push_back(stability);
        if self.samples.len() > Self::WINDOW {
            self.samples.pop_front();
        }
    }

    /// Estimate how many turns until stability reaches `floor`, using a simple
    /// linear extrapolation of the average per-turn delta.
    ///
    /// Returns `None` when stability is not declining or when fewer than two
    /// samples are available (insufficient data for a trend).
    #[must_use]
    pub fn eta_to_floor(&self, floor: f32) -> Option<u32> {
        if self.samples.len() < 2 {
            return None;
        }
        let current = *self.samples.back()?;
        if current <= floor {
            // Already at or below floor — the crisis is now, not predicted.
            return None;
        }
        // Average per-turn delta over the window (f64 for precision).
        let first = f64::from(*self.samples.front()?);
        let current_f64 = f64::from(current);
        let floor_f64 = f64::from(floor);
        // samples.len() is at most MAX_SAMPLES (5) so the subtraction fits f64 exactly.
        let n = f64::from(u32::try_from(self.samples.len() - 1).unwrap_or(1));
        let avg_delta = (current_f64 - first) / n; // negative = declining
        if avg_delta >= 0.0 {
            // Not declining.
            return None;
        }
        // turns_to_floor = (current - floor) / -avg_delta
        // Subtract a small epsilon before ceiling to correct for floating-point
        // imprecision: e.g. (0.4 / 0.1) = 4.000000000000001 in IEEE 754, which
        // would ceil to 5 instead of the expected 4.
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let turns = ((current_f64 - floor_f64) / (-avg_delta) - 1e-6).ceil() as u32;
        Some(turns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_ordering_blocking_highest() {
        assert!(Tier::Blocking > Tier::Urgent);
        assert!(Tier::Urgent > Tier::Notable);
        assert!(Tier::Notable > Tier::Ambient);
    }

    #[test]
    fn stability_tracker_eta_declining() {
        let mut tracker = StabilityTracker::default();
        // Use exact binary fractions to avoid float rounding:
        // 1.0 → 0.5, delta = -0.5/turn, headroom to 0.0 = 0.5, eta = ceil(0.5/0.5) = 1
        tracker.push(1.0);
        tracker.push(0.5);
        let eta = tracker.eta_to_floor(0.0);
        assert!(eta.is_some());
        assert_eq!(eta.unwrap(), 1);
    }

    #[test]
    fn stability_tracker_not_declining_returns_none() {
        let mut tracker = StabilityTracker::default();
        for &s in &[0.5f32, 0.6, 0.7] {
            tracker.push(s);
        }
        assert!(tracker.eta_to_floor(0.2).is_none());
    }

    #[test]
    fn stability_tracker_already_at_floor_returns_none() {
        let mut tracker = StabilityTracker::default();
        for &s in &[0.3f32, 0.2] {
            tracker.push(s);
        }
        assert!(tracker.eta_to_floor(0.2).is_none());
    }

    #[test]
    fn stability_tracker_insufficient_data_returns_none() {
        let mut tracker = StabilityTracker::default();
        tracker.push(0.8);
        assert!(tracker.eta_to_floor(0.2).is_none());
    }

    #[test]
    fn advance_result_serde_round_trip() {
        let result = AdvanceResult::Completed {
            turns_advanced: 5,
            digest: vec![Interrupt::new(
                Tier::Notable,
                InterruptSource::ConstructionComplete,
                None,
                "greenhouse built",
            )],
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: AdvanceResult = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            back,
            AdvanceResult::Completed {
                turns_advanced: 5,
                ..
            }
        ));
    }
}
