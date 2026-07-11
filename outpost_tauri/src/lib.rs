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
            commands::new_game,
            commands::advance_sol,
            commands::found_colony,
            commands::queue_construction,
            commands::assign_labour,
            commands::enqueue_research,
            commands::get_snapshot,
            commands::save_game,
            commands::load_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Outpost 3");
}
