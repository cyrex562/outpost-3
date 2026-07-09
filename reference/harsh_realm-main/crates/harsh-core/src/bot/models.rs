//! Bot framework data models.
//!
//! Ported from `src/harsh_realm/bot/models.py`.

use serde::{Deserialize, Serialize};

/// Mutable state tracked by the bot during a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotState {
    pub world_path: Option<String>,
    pub character_created: bool,
    pub current_q: i32,
    pub current_r: i32,
    pub current_scene: String,
    pub gold: i32,
    pub hp: i32,
    pub inventory: Vec<String>,
    pub last_response: String,
    pub combat_won: bool,
    pub combat_fled: bool,
    pub shop_purchased: bool,
    pub npc_talked: bool,
    pub social_scene_completed: bool,
    pub social_disposition_changed: bool,
    pub healed: bool,
    pub hp_before_heal: i32,
    pub died: bool,
    pub respawned: bool,
    pub respawn_completed: bool,
    pub dungeon_entered: bool,
    pub dungeon_rooms_visited: i32,
    pub dungeon_exited: bool,
    pub advanced_in_combat: bool,
    pub withdrew_in_combat: bool,
    pub harvested_loot: bool,
}

impl Default for BotState {
    fn default() -> Self {
        BotState {
            world_path: None,
            character_created: false,
            current_q: 0,
            current_r: 0,
            current_scene: "exploration".to_string(),
            gold: 0,
            hp: 0,
            inventory: vec![],
            last_response: String::new(),
            combat_won: false,
            combat_fled: false,
            shop_purchased: false,
            npc_talked: false,
            social_scene_completed: false,
            social_disposition_changed: false,
            healed: false,
            hp_before_heal: 0,
            died: false,
            respawned: false,
            respawn_completed: false,
            dungeon_entered: false,
            dungeon_rooms_visited: 0,
            dungeon_exited: false,
            advanced_in_combat: false,
            withdrew_in_combat: false,
            harvested_loot: false,
        }
    }
}

/// Result of running a single goal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionResult {
    pub goal_name: String,
    pub passed: bool,
    pub threshold: f64,
    pub actual_ratio: f64,
    pub notes: String,
}

/// Combat detection keywords used to identify combat responses.
pub const COMBAT_KEYWORDS: &[&str] = &[
    "initiative",
    "attacks you",
    "combat",
    "round 1",
    "your turn",
    "encounter state",
    "something lurks",
    "you notice",
    "combat begins",
];
