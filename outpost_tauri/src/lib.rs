//! Tauri desktop host for Outpost 3.
//!
//! Embeds the Vue 3 frontend and exposes `outpost_core` GameEngine
//! directly as Tauri commands — no HTTP server needed for the desktop build.
//!
//! Verbose file logging (`tauri_plugin_log`, writing to the OS log dir —
//! `%LOCALAPPDATA%\<identifier>\logs\outpost3.log` on Windows) plus a panic
//! hook are wired up first, before anything else in [`run`], so both
//! ordinary command success/failure and any unexpected panic (e.g. a
//! poisoned `Mutex` from a prior panic — every command handler in
//! `commands.rs` uses `.lock().unwrap()`) land in one place a player can
//! attach to a bug report.

mod commands;
mod state;

use tauri::Manager;
use tauri_plugin_log::{Target, TargetKind};

/// Install a panic hook that logs the panic (via the `log` crate, so it
/// lands in the same file the rest of the app logs to) before falling back
/// to the default hook's stderr output. Panics inside a Tauri command
/// handler otherwise just fail that one IPC call silently from the
/// frontend's perspective — the webview sees a rejected promise with no
/// detail, which is exactly what a blank-screen-after-an-unlogged-panic bug
/// looks like from the player's side.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("PANIC: {info}");
        default_hook(info);
    }));
}

pub fn run() {
    install_panic_hook();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .targets([
                    Target::new(TargetKind::LogDir {
                        file_name: Some("outpost3".into()),
                    }),
                    Target::new(TargetKind::Stdout),
                ])
                .build(),
        )
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let log_dir = app
                .path()
                .app_log_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|e| format!("<unresolvable: {e}>"));
            log::info!("Outpost 3 starting up; log file: {log_dir}/outpost3.log");
            let engine_state = state::EngineState::new();
            app.manage(engine_state);
            log::info!("engine_state managed; ready to accept bootstrap");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::is_ready,
            commands::exit_app,
            commands::log_frontend_error,
            commands::snapshot,
            commands::apply_command,
            commands::run_query,
            commands::reset_engine,
            commands::save_game,
            commands::load_game,
            commands::list_saves,
            commands::get_system_bodies,
            commands::get_tech_tree,
            commands::get_interrupt_digest,
            commands::get_colonize_targets,
            commands::list_buildings,
            commands::get_planet_map,
            commands::get_balance_scalars,
            commands::get_body_surface,
            commands::get_system_name,
            commands::list_supply_packages,
            commands::get_difficulty_knobs,
            commands::list_custom_presets,
            commands::save_custom_preset,
            commands::delete_custom_preset,
            commands::list_outposts,
            commands::get_outpost_targets,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Outpost 3");
}
