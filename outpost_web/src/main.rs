//! Binary entry point for the Outpost 3 web host.

use outpost_web::{serve, RuntimeConfig};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Log to stdout (as before) AND, best-effort, to a rolling daily file
    // under `logs/` relative to the process's working directory —
    // browser-mode's equivalent of the Tauri desktop shell's
    // `tauri_plugin_log` file, so a session that goes wrong (a panic inside
    // a command/query handler, an uncaught frontend exception forwarded via
    // `POST /api/log`) leaves a trace on disk instead of only in a terminal
    // nobody was watching. `_file_guard` must stay alive for the process
    // lifetime — dropping it stops the background writer thread that
    // flushes to the file.
    //
    // `tracing_appender::rolling::daily` panics internally if it can't
    // create the log directory (read-only filesystem, permission denied),
    // which would turn optional logging into a hard startup failure. Create
    // the directory ourselves first so that failure is recoverable: log a
    // warning to stdout and run with stdout-only logging instead.
    let (file_layer, _file_guard) = match std::fs::create_dir_all("logs") {
        Ok(()) => {
            let file_appender = tracing_appender::rolling::daily("logs", "outpost_web.log");
            let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
            let layer = tracing_subscriber::fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false);
            (Some(layer), Some(guard))
        }
        Err(e) => {
            eprintln!("warning: could not create logs/ directory ({e}); file logging disabled, stdout only");
            (None, None)
        }
    };

    // `try_from_default_env` also errs when `RUST_LOG` is simply unset (the
    // common case) — only warn when it was actually set to something
    // malformed, not on every default-config run.
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|e| {
        if std::env::var("RUST_LOG").is_ok() {
            eprintln!("warning: RUST_LOG is set but invalid ({e}); defaulting to \"info\"");
        }
        "info".into()
    });

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer())
        .with(file_layer)
        .init();

    let config = RuntimeConfig::default();
    serve(config).await
}
