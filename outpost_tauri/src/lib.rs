//! Tauri desktop host for Outpost 3.
//!
//! Embeds the Vue 3 frontend and exposes `outpost_core` GameEngine
//! directly as Tauri commands — no HTTP server needed for the desktop build.

mod commands;
mod state;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let engine_state = state::EngineState::new();
            app.manage(engine_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::is_ready,
            commands::exit_app,
            commands::snapshot,
            commands::apply_command,
            commands::run_query,
            commands::reset_engine,
            commands::save_game,
            commands::load_game,
            commands::list_saves,
            commands::get_system_bodies,
            commands::get_tech_tree,
            commands::get_colonize_targets,
            commands::list_buildings,
            commands::get_planet_map,
            commands::list_supply_packages,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Outpost 3");
}
