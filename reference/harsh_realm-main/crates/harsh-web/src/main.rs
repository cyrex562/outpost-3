//! Standalone server binary for the Harsh Realm web host.
//!
//! Configuration comes from environment variables (see [`harsh_web::RuntimeConfig`]).
//! The Tauri shell instead embeds the server in-process via `harsh_web::serve`.

fn main() -> anyhow::Result<()> {
    let config = harsh_web::RuntimeConfig::from_env();
    eprintln!(
        "harsh-web serving on http://{}:{} (run_mode={}, worlds={:?})",
        config.host, config.port, config.run_mode, config.worlds_dir
    );
    harsh_web::run_blocking(config)
}
