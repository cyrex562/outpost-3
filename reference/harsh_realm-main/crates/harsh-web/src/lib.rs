//! Harsh Realm web host (the "B9" layer).
//!
//! An Axum HTTP + WebSocket server that wraps the pure-Rust `harsh-core` engine
//! and serves the Vue frontend. Replaces the deleted Python FastAPI backend.
//! Used both as a standalone binary (Docker/headless) and embedded in-process by
//! the Tauri desktop shell.

pub mod admin;
pub mod chargen;
pub mod config;
pub mod content;
pub mod demo;
pub mod difficulty_routes;
pub mod editor;
pub mod gameplay;
pub mod response;
pub mod routes;
pub mod session;
pub mod state;
pub mod worldsvc;
pub mod ws;
pub mod wsmsg;

pub use config::RuntimeConfig;
pub use state::{new_state, AppState, AppStateInner};

/// Serve the web host on `config.host:config.port` until the process exits.
///
/// Requires a tokio runtime (call from `#[tokio::main]` or `Runtime::block_on`).
pub async fn serve(config: RuntimeConfig) -> anyhow::Result<()> {
    let addr = format!("{}:{}", config.host, config.port);
    let state = new_state(config);
    let app = routes::build_router(state);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Convenience entry point that owns a multi-threaded tokio runtime and blocks.
///
/// Used by the standalone binary and by the Tauri shell (on a background thread).
pub fn run_blocking(config: RuntimeConfig) -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(serve(config))
}
