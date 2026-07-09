//! Pure-Rust core game logic for Harsh Realm.
//!
//! This crate holds engine components ported from the Python implementation in
//! `src/harsh_realm/engine/`. It has no GUI or Tauri dependencies so it can be
//! built and tested in isolation, and reused both by the Tauri desktop shell
//! and by any future native tooling.
//!
//! Components are migrated one at a time; each should preserve the observable
//! behaviour of its Python counterpart and carry tests that pin that behaviour.

pub mod character_build;
pub mod character_creation;
pub mod command;
pub mod bot;
pub mod admin;
pub mod admin_api;
pub mod api;
pub mod cell;
pub mod character;
pub mod character_recalc;
pub mod class_progression;
pub mod combat_content;
pub mod advancement;
pub mod adventure_crafter;
pub mod combat;
pub mod combat_runtime;
pub mod compiler;
pub mod components;
pub mod content;
pub mod creature;
pub mod damage;
pub mod dice;
pub mod dispatch;
pub mod db;
pub mod db_schema;
pub mod difficulty;
pub mod discovery;
pub mod domain_events;
pub mod dsl;
pub mod editor;
pub mod editor_api;
pub mod encounters;
pub mod enemy_ai;
pub mod engine_results;
pub mod exceptions;
pub mod engine_runtime;
pub mod entity_state;
pub mod generators;
pub mod gm;
pub mod events;
pub mod faction;
pub mod generation;
pub mod grid;
pub mod healing;
pub mod intent;
pub mod item;
pub mod item_registry;
pub mod items;
pub mod loot;
pub mod loot_gen;
pub mod loot_source;
pub mod quest;
pub mod map;
pub mod modifiers;
pub mod narrator_content;
pub mod npc;
pub mod npc_personality;
pub mod oracle_models;
pub mod packs;
pub mod paths;
pub mod payloads;
pub mod public_api;
pub mod repositories;
pub mod runtime;
pub mod runtime_content;
pub mod saves;
pub mod scene_data;
pub mod settlement;
pub mod skill_checks;
pub mod shop;
pub mod shop_inventory;
pub mod status_effects;
pub mod low_health_narration;
pub mod table_engine;
pub mod threads;
pub mod weather;
pub mod gm_runtime;
pub mod travel;
pub mod ir;
pub mod oracle;
pub mod parser;
pub mod procedures;
pub mod resolution;
pub mod resolvers;
pub mod tables;

pub use resolution::{ResolutionResult, RollMechanic};

pub use command::ParsedCommand;
pub use dice::{attr_modifier, DiceResult, DiceRoller};
pub use dispatch::{dispatch, lower_effect};
pub use dsl::{evaluate, parse_condition, EvalContext, EvalError, ParseError};
pub use grid::{create_grid, Grid, GridCoord, GridError, GridType, SquareGrid};
pub use intent::Intent;
pub use item::{ConsumableEffect, ItemCatalog, ItemData, SaveBonusProfile, SaveType};
