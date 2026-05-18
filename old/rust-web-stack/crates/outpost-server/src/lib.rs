//! Outpost 3 Server — Web Layer and I/O
//!
//! This crate contains the Actix-Web server, database persistence, configuration,
//! and all I/O operations. It depends on `outpost-core` for game logic.

pub mod config;
pub mod db;
pub mod event_store;
pub mod http;
pub mod queries;
pub mod services;
pub mod utils;

pub use config::*;
