//! Runtime configuration for the web host.
//!
//! Values come from environment variables with sensible defaults. This replaces
//! the former Python `config.py` / `config.yaml`.

use std::path::PathBuf;

use harsh_core::paths::{get_base_dir, get_frontend_dist_dir};

/// Host configuration assembled at startup.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Bind address.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Directory holding world `.db` files.
    pub worlds_dir: PathBuf,
    /// Root directory containing content packs.
    pub packs_root: PathBuf,
    /// Compiled frontend (`frontend/dist`) directory.
    pub frontend_dist: PathBuf,
    /// Whether admin tooling is enabled.
    pub admin_mode: bool,
    /// How the app was launched: "server" or "desktop".
    pub run_mode: String,
    /// Crate version string.
    pub version: String,
}

fn env_flag(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|raw| {
        matches!(raw.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
    })
}

impl RuntimeConfig {
    /// Build the configuration from environment variables + defaults.
    pub fn from_env() -> Self {
        let base = get_base_dir();
        let host = std::env::var("HARSH_REALM_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port = std::env::var("HARSH_REALM_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        let worlds_dir = std::env::var("HARSH_REALM_WORLDS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.join("worlds"));
        let packs_root = std::env::var("HARSH_REALM_PACKS_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| base.join("content"));
        let run_mode = std::env::var("HARSH_REALM_RUN_MODE").unwrap_or_else(|_| "server".into());
        let admin_mode = env_flag("HARSH_REALM_ADMIN_MODE").unwrap_or(false);

        Self {
            host,
            port,
            worlds_dir,
            packs_root,
            frontend_dist: get_frontend_dist_dir(),
            admin_mode,
            run_mode,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::from_env()
    }
}
