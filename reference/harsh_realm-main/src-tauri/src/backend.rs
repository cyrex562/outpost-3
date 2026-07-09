//! Runs the Harsh Realm web host (`harsh-web`) in-process for the desktop shell.
//!
//! Responsibilities:
//!   * pick a free TCP port,
//!   * start the Axum server on a background thread (its own tokio runtime),
//!   * block until it accepts connections,
//!   * hand back the URL the webview should load.
//!
//! Unlike the former model (a Python `uvicorn` child process), the server is
//! linked directly and runs inside this process, so there is no child to
//! supervise or orphan — it dies with the app.

use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};

use harsh_web::RuntimeConfig;

const HOST: &str = "127.0.0.1";
const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// The in-process web host plus the URL it is reachable at.
pub struct Backend {
    url: String,
}

impl Backend {
    /// The base URL (e.g. `http://127.0.0.1:51823`) the frontend should load.
    pub fn url(&self) -> &str {
        &self.url
    }
}

/// Ask the OS for an unused TCP port by binding to port 0 and reading it back.
fn find_free_port() -> Result<u16> {
    let listener = TcpListener::bind((HOST, 0))?;
    let port = listener.local_addr()?.port();
    // The listener is dropped here, freeing the port for the server to claim.
    Ok(port)
}

/// Return true once `host:port` accepts a TCP connection.
fn port_is_open(host: &str, port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("{host}:{port}")
            .parse()
            .expect("host:port is a valid socket address"),
        Duration::from_millis(200),
    )
    .is_ok()
}

/// Start the web host in-process and wait until it is ready to serve requests.
///
/// Paths (frontend, content, worlds) come from the environment, which the Tauri
/// launcher configures for a packaged build (see `lib.rs`); in dev they fall
/// back to cwd-relative defaults.
pub fn launch() -> Result<Backend> {
    let port = find_free_port()?;
    log::info!("Starting in-process web host on {HOST}:{port}");

    let mut config = RuntimeConfig::from_env();
    config.host = HOST.to_string();
    config.port = port;
    config.run_mode = "desktop".to_string();

    std::thread::Builder::new()
        .name("harsh-web".into())
        .spawn(move || {
            if let Err(e) = harsh_web::run_blocking(config) {
                log::error!("harsh-web server exited with error: {e}");
            }
        })?;

    let url = format!("http://{HOST}:{port}");
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    while Instant::now() < deadline {
        if port_is_open(HOST, port) {
            log::info!("Web host ready at {url}");
            return Ok(Backend { url });
        }
        std::thread::sleep(POLL_INTERVAL);
    }

    Err(anyhow!(
        "web host did not become ready within {STARTUP_TIMEOUT:?}"
    ))
}
