//! Shared application state for the web host.

use std::sync::{Arc, Mutex};

use outpost_core::{Event, GameEngine};
use tokio::sync::broadcast;

use crate::config::RuntimeConfig;
use crate::sessions::SessionRegistry;

/// Capacity of the broadcast channel that distributes engine events to WebSocket clients.
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Inner shared state.
pub struct AppStateInner {
    /// Runtime configuration.
    pub config: RuntimeConfig,
    /// The live game engine, protected by a mutex for multi-handler access.
    pub engine: Mutex<GameEngine>,
    /// Broadcast sender — every `apply` call fans events out to all connected
    /// clients, paired with the id of the client that already has them.
    ///
    /// `None` means "send to everyone" and is what the WebSocket command path
    /// uses: it replies with a bare `Ack`, so the broadcast is the *only* way
    /// the sender learns what happened.
    ///
    /// `Some(id)` means "everyone except this client". `POST /api/command`
    /// returns its events in the response body — callers genuinely need them,
    /// the founding wizard reads the new colony's id straight out of them — so
    /// broadcasting to the issuer as well delivered every command-issued event
    /// twice (issue #452).
    pub events: broadcast::Sender<(Option<String>, Event)>,
    /// Registry of named game sessions (one engine each).
    pub sessions: SessionRegistry,
}

/// Cheaply-cloneable handle to the application state.
pub type AppState = Arc<AppStateInner>;

/// Construct the shared state with a fresh [`GameEngine`].
#[must_use]
pub fn new_state(config: RuntimeConfig) -> AppState {
    let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
    Arc::new(AppStateInner {
        config,
        engine: Mutex::new(GameEngine::new()),
        events: tx,
        sessions: SessionRegistry::new(),
    })
}
