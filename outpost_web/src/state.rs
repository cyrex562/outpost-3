//! Shared application state for the web host.

use std::sync::{Arc, Mutex};

use outpost_core::{Event, GameEngine};
use tokio::sync::broadcast;

use crate::config::RuntimeConfig;

/// Capacity of the broadcast channel that distributes engine events to WebSocket clients.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Inner shared state.
pub struct AppStateInner {
    /// Runtime configuration.
    pub config: RuntimeConfig,
    /// The live game engine, protected by a mutex for multi-handler access.
    pub engine: Mutex<GameEngine>,
    /// Broadcast sender — every `apply` call fans events out to all connected clients.
    pub events: broadcast::Sender<Event>,
}

/// Cheaply-cloneable handle to the application state.
pub type AppState = Arc<AppStateInner>;

/// Construct the shared state with a fresh [`GameEngine`].
pub fn new_state(config: RuntimeConfig) -> AppState {
    let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
    Arc::new(AppStateInner {
        config,
        engine: Mutex::new(GameEngine::new()),
        events: tx,
    })
}
