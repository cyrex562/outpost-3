//! Tauri desktop shell for Harsh Realm.
//!
//! The shell starts the Rust web host ([`harsh_web`]) in-process on a free port,
//! then opens the main window pointed at that host's URL, so the origin-relative
//! frontend (its `/api` and `/ws` calls) works unchanged. Native Rust IPC
//! commands ([`commands`]) are also available on that page via `withGlobalTauri`.

mod backend;
mod commands;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Run the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::roll_dice,
            commands::roll_attribute_set,
            commands::attribute_modifier,
        ])
        .setup(|app| {
            // Point the in-process web host at the bundled read-only assets and a
            // writable data dir, so the packaged app works regardless of the
            // working directory. `frontend/dist` and `content/` are bundled as
            // Tauri resources (see tauri.conf.json); the host resolves both from
            // `HARSH_REALM_BASE_DIR`, and writes worlds under the OS app-data dir.
            //
            // In `cargo tauri dev` the resources aren't staged, so we only set
            // the base dir when the bundled `content/` is actually present and
            // otherwise leave the cwd-relative defaults for development.
            if let Ok(res) = app.path().resource_dir() {
                if res.join("content").join("xwn-core").is_dir() {
                    std::env::set_var("HARSH_REALM_BASE_DIR", &res);
                }
            }
            if let Ok(data) = app.path().app_data_dir() {
                let worlds = data.join("worlds");
                let _ = std::fs::create_dir_all(&worlds);
                std::env::set_var("HARSH_REALM_WORLDS_DIR", &worlds);
            }

            // Start the in-process web host. It runs on a background thread and
            // dies with the app — there is no child process to supervise.
            let backend = backend::launch()?;
            let url = backend.url().to_string();

            let external = url
                .parse()
                .map_err(|e| format!("invalid backend url {url}: {e}"))?;

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(external))
                .title("Harsh Realm")
                .inner_size(1280.0, 800.0)
                .min_inner_size(1024.0, 600.0)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Harsh Realm desktop shell");
}
