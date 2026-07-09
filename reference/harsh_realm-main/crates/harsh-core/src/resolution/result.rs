//! The unified resolution result — the "margin currency" every contest yields.
//!
//! One structure, consumed by every outcome spec, is what lets a single action
//! definition work across dice systems (per the action-resolution-primitives
//! design, §2.4). Outcome specs may key on `raw_margin` thresholds **or** the
//! mechanic-banded `degree`.

use serde::{Deserialize, Serialize};

/// The outcome of resolving one contest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolutionResult {
    /// Whether the attempt succeeded.
    pub success: bool,
    /// Signed margin: `total − tn` (roll-over) or `skill − roll` (roll-under).
    pub raw_margin: i64,
    /// Mechanic-banded degree: `0` = bare, positive = better, negative = worse.
    pub degree: i64,
    /// Mechanic-defined critical success.
    pub crit: bool,
    /// Mechanic-defined fumble.
    pub fumble: bool,
    /// The raw die roll (sum of dice, before modifiers).
    pub rolled: i64,
    /// `rolled + modifiers` (the effective value compared to `tn`).
    pub total: i64,
    /// The target number this was resolved against.
    pub tn: i64,
}
