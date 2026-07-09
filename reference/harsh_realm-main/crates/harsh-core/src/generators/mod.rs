//! World/content generators.
//!
//! Ported incrementally from the Python `harsh_realm.generators` package. The
//! pure classification helpers land first; the DB-driven world/settlement/dungeon
//! generators follow in later batches.

pub mod biome;
pub mod content_tables;
pub mod dungeon_gen;
pub mod npc_gen;
pub mod settlement_gen;
pub mod square_gen;
pub mod water;
pub mod world_gen;
pub mod world_support;

pub use biome::{classify_forest_biome, FOREST_BIOME_KEY};
pub use content_tables::load_table_results;
pub use dungeon_gen::DungeonGenerator;
pub use npc_gen::NPCGenerator;
pub use settlement_gen::SettlementGenerator;
pub use square_gen::SquareWorldGenerator;
pub use water::{classify_water_kind, WATER_KIND_KEY};
pub use world_gen::WorldGenerator;
pub use world_support::{load_name_list, load_terrain_weights};
