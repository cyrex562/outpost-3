//! Interrupt tiers, threshold system, and "advance until interrupted" loop.
//!
//! See `docs/DESIGN.md §4A, §12A`.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::colony::ColonyId;

// ─── Tier ─────────────────────────────────────────────────────────────────────

/// Priority tier that governs whether a fast-forward halts on this interrupt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Tier {
    /// Background; visible only in the event log.
    Ambient,
    /// Logged and shown in the digest; does not stop fast-forward.
    Notable,
    /// Stops fast-forward and hands back control; act or dismiss-and-continue.
    Urgent,
    /// Halts time; requires a decision to continue. Rare.
    Blocking,
}

// ─── InterruptSource ──────────────────────────────────────────────────────────

/// Identifies what generated an [`Interrupt`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum InterruptSource {
    /// A metric is trending toward a crisis threshold; warns before the actual crash.
    PredictiveWarning {
        /// Current value of the declining quantity (e.g. stability).
        quantity: f32,
        /// Extrapolated turns until the quantity crosses the danger threshold.
        eta_turns: u32,
    },
    /// An event with the given id fired.
    EventFired(String),
    /// A construction project completed.
    ConstructionComplete,
    /// A tech node was unlocked.
    TechUnlocked,
}

// ─── Interrupt ────────────────────────────────────────────────────────────────

/// A single interrupt generated during turn resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interrupt {
    /// Priority tier — governs fast-forward halting.
    pub tier: Tier,
    /// What generated this interrupt.
    pub source: InterruptSource,
    /// Colony the interrupt is associated with, if any.
    pub colony_id: Option<ColonyId>,
    /// Human-readable description.
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

// ─── AdvanceResult ────────────────────────────────────────────────────────────

/// Result returned by `GameEngine::advance_until_interrupted`.
#[derive(Debug, Clone)]
pub enum AdvanceResult {
    /// Advance halted early because an interrupt ≥ threshold was encountered.
    Halted {
        /// Sol at which the halt occurred.
        at_turn: u64,
        /// The interrupt that caused the halt.
        interrupt: Interrupt,
        /// Accumulated `Notable` and `Ambient` interrupts up to the halt point.
        digest: Vec<Interrupt>,
    },
    /// Advance ran the full requested number of turns with no halting interrupt.
    Completed {
        /// Number of turns actually advanced.
        turns_advanced: u32,
        /// Accumulated `Notable` and `Ambient` interrupts across all turns.
        digest: Vec<Interrupt>,
    },
}

// ─── StabilityTracker ────────────────────────────────────────────────────────

/// Ring buffer of recent stability samples used for linear trend extrapolation.
///
/// Cheap O(1) push; `eta_to_floor` computes the number of turns until the
/// linear trend projects the value below a given floor.
#[derive(Debug, Clone, Default)]
pub struct StabilityTracker {
    /// Most-recent stability samples (up to `MAX_SAMPLES`).
    pub samples: VecDeque<f32>,
}

impl StabilityTracker {
    /// Maximum number of samples retained.
    const MAX_SAMPLES: usize = 5;

    /// Record a new stability sample, evicting the oldest if at capacity.
    pub fn push(&mut self, stability: f32) {
        if self.samples.len() >= Self::MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(stability);
    }

    /// Compute the estimated number of turns until stability reaches `floor`.
    ///
    /// Returns `Some(eta)` when the linear trend projects a crossing within a
    /// reasonable horizon, `None` when insufficient data or trend is flat/rising.
    ///
    /// Uses cheap linear extrapolation across the available sample window.
    #[must_use]
    pub fn eta_to_floor(&self, floor: f32) -> Option<u32> {
        if self.samples.len() < 2 {
            return None;
        }
        let current = f64::from(*self.samples.back()?);
        let oldest = f64::from(*self.samples.front()?);
        let floor_f64 = f64::from(floor);
        #[allow(clippy::cast_precision_loss)]
        let n = self.samples.len() as f64;
        // Average rate of change per turn across the sample window (f64 for precision).
        let delta_per_turn = (current - oldest) / (n - 1.0);

        if delta_per_turn >= 0.0 {
            // Flat or rising — no warning.
            return None;
        }

        let gap = current - floor_f64;
        if gap <= 0.0 {
            // Already at or below floor.
            return None;
        }

        // eta = ceil(gap / |delta_per_turn|)
        // Snap to nearest integer when within epsilon to avoid f32 precision
        // artifacts (e.g. 2.0000002 must not ceil to 3).
        let ratio = gap / (-delta_per_turn);
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let eta = {
            let rounded = ratio.round();
            if (ratio - rounded).abs() < 1e-4 {
                rounded as u32
            } else {
                ratio.ceil() as u32
            }
        };
        Some(eta)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Tier ordering ──────────────────────────────────────────────────────────

    #[test]
    fn tier_ordering_is_correct() {
        assert!(Tier::Ambient < Tier::Notable);
        assert!(Tier::Notable < Tier::Urgent);
        assert!(Tier::Urgent < Tier::Blocking);
    }

    // ── StabilityTracker ──────────────────────────────────────────────────────

    #[test]
    fn tracker_no_warning_with_single_sample() {
        let mut t = StabilityTracker::default();
        t.push(0.5);
        assert!(t.eta_to_floor(0.2).is_none());
    }

    #[test]
    fn tracker_no_warning_when_rising() {
        let mut t = StabilityTracker::default();
        t.push(0.3);
        t.push(0.5);
        assert!(t.eta_to_floor(0.2).is_none());
    }

    #[test]
    fn tracker_eta_computed_correctly() {
        let mut t = StabilityTracker::default();
        // Push 0.6 then 0.4 → delta = -0.2/turn, gap to 0.0 = 0.4, eta = 2.
        t.push(0.6);
        t.push(0.4);
        let eta = t.eta_to_floor(0.0).expect("should compute eta");
        assert_eq!(eta, 2);
    }

    #[test]
    fn tracker_returns_none_when_already_at_floor() {
        let mut t = StabilityTracker::default();
        t.push(0.5);
        t.push(0.15); // already below floor 0.2
        assert!(t.eta_to_floor(0.2).is_none());
    }

    #[test]
    fn tracker_evicts_oldest_sample_at_capacity() {
        let mut t = StabilityTracker::default();
        for i in 0..=StabilityTracker::MAX_SAMPLES {
            t.push(1.0 - (i as f32 * 0.01));
        }
        assert_eq!(t.samples.len(), StabilityTracker::MAX_SAMPLES);
    }

    // ── Interrupt construction ─────────────────────────────────────────────────

    #[test]
    fn interrupt_fields_stored_correctly() {
        let id = uuid::Uuid::new_v4();
        let irq = Interrupt::new(
            Tier::Urgent,
            InterruptSource::ConstructionComplete,
            Some(id),
            "building done",
        );
        assert_eq!(irq.tier, Tier::Urgent);
        assert!(matches!(irq.source, InterruptSource::ConstructionComplete));
        assert_eq!(irq.colony_id, Some(id));
        assert_eq!(irq.message, "building done");
    }

    // ── AdvanceResult variants ─────────────────────────────────────────────────

    #[test]
    fn advance_result_halted_fields() {
        let irq = Interrupt::new(Tier::Blocking, InterruptSource::TechUnlocked, None, "tech");
        let result = AdvanceResult::Halted {
            at_turn: 7,
            interrupt: irq,
            digest: vec![],
        };
        match result {
            AdvanceResult::Halted {
                at_turn, digest, ..
            } => {
                assert_eq!(at_turn, 7);
                assert!(digest.is_empty());
            }
            AdvanceResult::Completed { .. } => panic!("wrong variant"),
        }
    }

    #[test]
    fn advance_result_completed_fields() {
        let result = AdvanceResult::Completed {
            turns_advanced: 10,
            digest: vec![],
        };
        match result {
            AdvanceResult::Completed {
                turns_advanced,
                digest,
            } => {
                assert_eq!(turns_advanced, 10);
                assert!(digest.is_empty());
            }
            AdvanceResult::Halted { .. } => panic!("wrong variant"),
        }
    }
}
