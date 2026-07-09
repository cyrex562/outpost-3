//! Quest content records (authored YAML under `content/xwn-core/content/quests/`).

use serde::{Deserialize, Serialize};

/// One objective within a quest. `target` is the count required for a countable
/// objective (e.g. "collect 5"); `None` marks a simple boolean objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestObjective {
    /// Stable key used by `update_progress`.
    pub key: String,
    /// Player-facing objective text.
    pub description: String,
    /// Count required to satisfy the objective, if countable.
    #[serde(default)]
    pub target: Option<i32>,
}

/// An authored quest definition (content, not instance state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quest {
    /// Stable content id (the pack slug, e.g. `"retrieve_artifact"`).
    pub id: String,
    /// Display name.
    pub name: String,
    /// Briefing text.
    #[serde(default)]
    pub description: String,
    /// Ordered objectives.
    #[serde(default)]
    pub objectives: Vec<QuestObjective>,
    /// Gold reward on completion.
    #[serde(default)]
    pub reward_gold: i32,
    /// XP reward on completion.
    #[serde(default)]
    pub reward_xp: i32,
    /// Item ids granted on completion (reward *processing* is out of scope; the
    /// completion event carries these for a later granting step).
    #[serde(default)]
    pub reward_items: Vec<String>,
    /// Quest ids that must be completed before this can be accepted.
    #[serde(default)]
    pub prerequisites: Vec<String>,
    /// Optional world tick after which the quest can no longer be accepted/held.
    #[serde(default)]
    pub expiry_tick: Option<i32>,
    /// Optional faction that gives or requires the quest.
    #[serde(default)]
    pub faction_id: Option<String>,
}
