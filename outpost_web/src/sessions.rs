//! Session management for the web host.
//!
//! Each HTTP session wraps an independent [`GameEngine`] instance so multiple
//! games can run concurrently without interfering with each other.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use outpost_core::GameEngine;
use uuid::Uuid;

/// Opaque identifier for one game session.
pub type SessionId = Uuid;

/// A single live game session.
pub struct Session {
    /// The engine for this session, guarded against concurrent mutation.
    pub engine: Mutex<GameEngine>,
}

impl Session {
    /// Create a new session with a fresh [`GameEngine`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(GameEngine::new()),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry of all active sessions, shared across handlers.
#[derive(Default)]
pub struct SessionRegistry {
    inner: Mutex<HashMap<SessionId, Arc<Session>>>,
}

impl SessionRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a fresh session and return its id.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (should not happen in normal operation).
    pub fn create(&self) -> SessionId {
        let id = Uuid::new_v4();
        let mut map = self.inner.lock().expect("session registry lock");
        map.insert(id, Arc::new(Session::new()));
        id
    }

    /// Look up a session by id.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (should not happen in normal operation).
    pub fn get(&self, id: &SessionId) -> Option<Arc<Session>> {
        let map = self.inner.lock().expect("session registry lock");
        map.get(id).cloned()
    }

    /// Remove a session, returning `true` if it existed.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (should not happen in normal operation).
    pub fn remove(&self, id: &SessionId) -> bool {
        let mut map = self.inner.lock().expect("session registry lock");
        map.remove(id).is_some()
    }
}
