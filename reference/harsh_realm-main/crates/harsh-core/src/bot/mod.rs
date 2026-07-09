//! Bot framework — automated playtesting models, pathfinder, and assertions.
//!
//! Ported from `src/harsh_realm/bot/`. The async WebSocket runner
//! (`runner.py` and its mixins) and goal suites (`goals/`) require tokio +
//! a WebSocket crate and are deferred to the B9 web-host / standalone-binary
//! phase.

pub mod assertions;
pub mod models;
pub mod pathfinder;

pub use assertions::{assert_contains, assert_exact, assert_threshold};
pub use models::{AssertionResult, BotState, COMBAT_KEYWORDS};
pub use pathfinder::Pathfinder;
