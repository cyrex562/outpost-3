//! Quest event payloads (HR-19).
//!
//! Result notices carry the `#[derive(TS)]` client-facing shapes; the
//! `*Requested` structs are server-side command intents (Rule 2) resolved in
//! `GMController::resolve_domain_events`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::runtime::JsonObject;

// ---------------------------------------------------------------------------
// Request payloads (server-side; no client-facing TS type)
// ---------------------------------------------------------------------------

/// Payload for `quest.accept_requested`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestAcceptRequested {
    pub entity_id: String,
    pub quest_id: String,
}

/// Payload for `quest.progress_requested`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestProgressRequested {
    pub entity_id: String,
    pub quest_id: String,
    pub objective_key: String,
    pub value: i32,
}

/// Payload for `quest.complete_requested` and `quest.fail_requested`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestOutcomeRequested {
    pub entity_id: String,
    pub quest_id: String,
}

// ---------------------------------------------------------------------------
// Result notices (client-facing)
// ---------------------------------------------------------------------------

/// Payload for `quest.accepted`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct QuestAcceptedNotice {
    pub quest_id: String,
    pub quest_name: String,
    pub description: String,
}

/// Payload for `quest.progress_updated`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct QuestProgressUpdatedNotice {
    pub quest_id: String,
    pub objective_key: String,
    pub value: i32,
    #[ts(type = "Record<string, unknown>")]
    pub progress: JsonObject,
}

/// Payload for `quest.completed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct QuestCompletedNotice {
    pub quest_id: String,
    pub quest_name: String,
    pub reward_gold: i32,
    pub reward_xp: i32,
    pub reward_items: Vec<String>,
}

/// Payload for `quest.failed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct QuestFailedNotice {
    pub quest_id: String,
    pub quest_name: String,
}
