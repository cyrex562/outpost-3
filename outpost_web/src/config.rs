//! Runtime configuration for the web host.

use std::path::PathBuf;

/// Configuration loaded at startup and shared across handlers.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Interface to bind, e.g. `"127.0.0.1"`.
    pub host: String,
    /// TCP port to listen on.
    pub port: u16,
    /// Path to the compiled Vue frontend `dist/` directory.
    /// When `None` the static-file middleware is omitted (API-only mode for tests).
    pub frontend_dist: Option<PathBuf>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 3000,
            frontend_dist: None,
        }
    }
}
