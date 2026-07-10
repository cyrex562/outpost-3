//! Binary entry point for the Outpost 3 web host.

use outpost_web::{serve, RuntimeConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let config = RuntimeConfig::default();
    serve(config).await
}
