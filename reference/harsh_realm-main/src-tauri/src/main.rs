// Prevent an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `log` macros (used in the backend supervisor) are no-ops until a logger is
    // installed. Developers wanting output can add `env_logger` and init it here.
    harsh_realm_desktop_lib::run()
}
