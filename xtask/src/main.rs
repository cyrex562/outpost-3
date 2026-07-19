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
        "install-windows" => windows_portable::install_windows(),
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
         \x20 build-windows-portable   Build outpost_tauri and zip an installer-free portable\n\
         \x20                          bundle under dist/. Natively on Windows; cross-compiled\n\
         \x20                          via cargo-xwin (best-effort) on Linux/macOS.\n\
         \x20 install-windows          Windows-only: build natively and copy the result into\n\
         \x20                          %LOCALAPPDATA%\\Outpost3\\ — a stable install for\n\
         \x20                          repeat playtesting, and prints the verbose log file's\n\
         \x20                          path (%LOCALAPPDATA%\\<identifier>\\logs\\outpost3.log)\n\
         \x20                          for attaching to bug reports.\n\
         \x20 setup-windows            Install the x86_64-pc-windows-msvc target + cargo-xwin,\n\
         \x20                          without building. Only needed for the Linux/macOS\n\
         \x20                          cross-compile path — a no-op concept on Windows itself.\n\
         \x20 help                     Show this help.\n\
         \n\
         The authoritative *installer* build remains `cargo tauri build`, run\n\
         on Windows from inside outpost_tauri/ — this command exists for a\n\
         zip-and-go dev/distribution artifact, not to replace NSIS/WiX\n\
         packaging.\n"
    );
}
