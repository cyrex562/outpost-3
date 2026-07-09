//! GM scene handlers.
//!
//! Ported incrementally from the Python `harsh_realm.gm.scenes` package.

pub mod base;
pub mod character_creation_scene;
pub mod combat_scene;
pub mod dungeon;
pub mod exploration;
pub mod exploration_support;
pub mod level_up;
pub mod respawn;
pub mod shopping;
pub mod social;
pub mod social_support;
pub mod time_support;
pub mod town;
pub mod town_support;

pub use base::{SceneHandler, SceneState};
pub use character_creation_scene::CharacterCreationScene;
pub use combat_scene::CombatScene;
pub use dungeon::DungeonScene;
pub use exploration::ExplorationScene;
pub use exploration_support::blocked_message;
pub use level_up::LevelUpScene;
pub use respawn::RespawnScene;
pub use shopping::ShoppingScene;
pub use social::SocialScene;
pub use social_support::disposition_label;
pub use time_support::{format_time, get_time_description};
pub use town::TownScene;
pub use town_support::{is_impassable_town, normalize_settlement_summary};
