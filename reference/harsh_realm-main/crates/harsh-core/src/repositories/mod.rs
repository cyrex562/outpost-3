//! Repositories: row <-> model access over [`crate::db::WorldDatabase`].
//!
//! Ported from the Python `*_repository` modules. Each repository borrows a
//! `WorldDatabase` and maps rows to the already-ported value models.

pub mod cell;
pub mod difficulty;
pub mod discovery;
pub mod dungeon;
pub mod editor_entity;
pub mod entity;
pub mod entity_state;
pub mod event_log;
pub mod faction;
pub mod gm_state;
pub mod oracle;
pub mod random_table;
pub mod resource_schema;
pub mod resources;
pub mod skill_config;
pub mod world_pack;
