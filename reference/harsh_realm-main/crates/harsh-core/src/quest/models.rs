//! Runtime model for a per-world quest instance.

use serde::{Deserialize, Serialize};

use crate::runtime::JsonObject;

/// Quest lifecycle states. `list_active` returns `Accepted` + `InProgress`;
/// `Completed`/`Failed` are terminal.
pub const STATUS_ACCEPTED: &str = "accepted";
pub const STATUS_IN_PROGRESS: &str = "in_progress";
pub const STATUS_COMPLETED: &str = "completed";
pub const STATUS_FAILED: &str = "failed";

/// The full, ordered set of valid quest statuses.
pub const QUEST_STATUSES: [&str; 4] = [
    STATUS_ACCEPTED,
    STATUS_IN_PROGRESS,
    STATUS_COMPLETED,
    STATUS_FAILED,
];

/// Whether a status is non-terminal (returned by `list_active`).
pub fn is_active_status(status: &str) -> bool {
    status == STATUS_ACCEPTED || status == STATUS_IN_PROGRESS
}

/// A per-world quest instance held by a character.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveQuest {
    /// Row id.
    pub id: i64,
    /// Owning entity (the player).
    pub entity_id: String,
    /// Content id of the quest.
    pub quest_id: String,
    /// One of [`QUEST_STATUSES`].
    pub status: String,
    /// World tick the quest was accepted.
    pub accepted_tick: i32,
    /// World tick the quest reached a terminal state, if it has.
    pub completed_tick: Option<i32>,
    /// Per-objective progress (`objective_key` → value).
    #[serde(default)]
    pub progress: JsonObject,
}
