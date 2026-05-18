//! Outpost 3 — Pure Domain Logic (Zero I/O)
//!
//! This crate contains all game logic, domain models, commands, events, and simulation code.
//! It has **zero I/O dependencies** — no database, no filesystem, no network, no async runtime.
//! All I/O happens in `outpost-server`.

pub mod commands;
pub mod content;
pub mod domain;
pub mod events;
pub mod simulation;

// Re-export commonly used types
pub use domain::*;
pub use events::*;
pub use commands::*;
pub use content::*;
