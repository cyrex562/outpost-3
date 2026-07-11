//! Shared Tauri application state.

use outpost_core::GameEngine;
use std::sync::Mutex;

/// Thread-safe wrapper around the game engine for Tauri command handlers.
pub struct EngineState {
    pub engine: Mutex<Option<GameEngine>>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(None),
        }
    }
}
