//! Shared application state for the web host.

use std::sync::Arc;

use harsh_core::packs::paths::default_content_dir;

use crate::config::RuntimeConfig;
use crate::session::{self, SessionHandle};
use crate::worldsvc::ActorPaths;

/// Inner application state shared across all request handlers.
pub struct AppStateInner {
    pub config: RuntimeConfig,
    /// Handle to the single-world session actor (owns the live `WorldDatabase`).
    pub session: SessionHandle,
}

/// Cheaply-cloneable handle to the application state.
pub type AppState = Arc<AppStateInner>;

/// Build the shared state, spawning the session actor thread.
pub fn new_state(config: RuntimeConfig) -> AppState {
    let session = session::spawn(ActorPaths {
        worlds_dir: config.worlds_dir.clone(),
        content_dir: default_content_dir(),
        packs_root: config.packs_root.clone(),
    });
    Arc::new(AppStateInner { config, session })
}
