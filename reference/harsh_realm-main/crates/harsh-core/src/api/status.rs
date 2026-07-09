//! Service-status API models. Ported from `models/api/status.py`.

use serde::{Deserialize, Serialize};

/// Basic service metadata for service-status endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Service name.
    pub service: String,
    /// Status string.
    pub status: String,
}

/// Root endpoint payload (service status plus link fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootInfo {
    /// Embedded service status.
    #[serde(flatten)]
    pub status: ServiceStatus,
    /// Docs URL.
    pub docs: String,
    /// Health URL.
    pub health: String,
    /// API base URL.
    pub api: String,
    /// WebSocket URL.
    pub websocket: String,
}

/// Runtime configuration the frontend reads to shape the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfigInfo {
    /// Whether admin mode is enabled.
    pub admin_mode: bool,
    /// Run mode (e.g. `"desktop"`).
    pub run_mode: String,
    /// Version string.
    pub version: String,
}
