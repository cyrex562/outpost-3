//! Outpost 3 build/release orchestration, run via `cargo xtask <command>`.
//!
//! A thin orchestrator: it shells out to the real tools (`npm`, `cargo`,
//! `rustup`, `cargo-xwin`, `zip`) and owns only the glue (sequencing,
//! artifact discovery, checksums, feedback). Add new commands in `main` and
//! `print_usage`.

mod util;
mod windows_portable;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");

    let result = match cmd {
        "build-windows-portable" => windows_portable::build_windows_portable(),
        "setup-windows" => windows_portable::setup_windows(),
        "help" | "-h" | "--help" => {
            print_usage();
            Ok(())
        }
        other => {
            eprintln!("xtask: unknown command: {other}\n");
            print_usage();
            std::process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("xtask: error: {e}");
        std::process::exit(1);
    }
}

fn print_usage() {
    println!(
        "Outpost 3 build orchestration.\n\
         \n\
         Usage: cargo xtask <command>\n\
         \n\
         Commands:\n\
         \x20 build-windows-portable   Cross-compile outpost_tauri for Windows (best-effort,\n\
         \x20                          via cargo-xwin) and zip an installer-free portable\n\
         \x20                          bundle under dist/. Run on Linux/macOS.\n\
         \x20 setup-windows            Install the x86_64-pc-windows-msvc target + cargo-xwin,\n\
         \x20                          without building (useful to pre-warm a CI cache).\n\
         \x20 help                     Show this help.\n\
         \n\
         The authoritative Windows build remains `cargo tauri build`, run on\n\
         a real Windows host from inside outpost_tauri/ — this command exists\n\
         for CI smoke-testing / portable dev builds without a Windows runner.\n"
    );
}
