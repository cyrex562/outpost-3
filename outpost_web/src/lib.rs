//! Outpost 3 web host.
//!
//! An Axum HTTP + WebSocket server that wraps the pure-Rust `outpost_core` engine
//! and serves the Vue 3 frontend. Provides a typed server→client event contract
//! so the frontend derives its world model from the event stream.
//!
//! # Architecture
//!
//! - `state`   — shared `AppState` (engine + broadcast channel)
//! - `routes`  — HTTP REST endpoints + static frontend serving
//! - `ws`      — WebSocket game-loop handler
//! - `wsmsg`   — typed WebSocket message contract (server→client, client→server)
//! - `config`  — runtime configuration

#![warn(missing_docs)]

pub mod config;
pub mod routes;
pub mod state;
pub mod ws;
pub mod wsmsg;

pub use config::RuntimeConfig;
pub use state::{new_state, AppState};

/// Serve the web host on `config.host:config.port` until the process exits.
///
/// Requires a tokio runtime (call from `#[tokio::main]` or `Runtime::block_on`).
///
/// # Errors
///
/// Returns an error if the address cannot be parsed or the TCP listener fails to bind.
pub async fn serve(config: RuntimeConfig) -> anyhow::Result<()> {
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid address: {e}"))?;
    let state = new_state(config);
    let app = routes::build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
