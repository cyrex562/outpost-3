//! Harsh Realm build/release orchestration, run via `cargo xtask <command>`.
//!
//! A cross-platform (Linux + Windows) replacement for the per-platform
//! shell/PowerShell scripts under `scripts/`. It is a thin orchestrator: it
//! shells out to the real tools (`npm`, `cargo`, `cargo tauri`, `gh`) and owns
//! only the glue (sequencing, artifact discovery, checksums, feedback).
//!
//! Add new commands in `main` and `print_usage`.

mod build;
mod codegen;
mod finding;
mod release;
mod test;
mod util;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("help");
    let rest: &[String] = if args.is_empty() { &[] } else { &args[1..] };

    let result = match cmd {
        "build-windows" => build::build_windows(),
        "gen-types" => codegen::gen_types(rest),
        "test" => test::test(rest),
        "release-publish" => release::release_publish(rest),
        "release-download" => release::release_download(rest),
        "report-finding" => finding::report_finding(rest),
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
        "Harsh Realm build orchestration.\n\
         \n\
         Usage: cargo xtask <command> [args]\n\
         \n\
         Commands:\n\
         \x20 build-windows                       Build the Tauri desktop bundle (Windows only) + SHA256SUMS.\n\
         \x20 gen-types                           Regenerate frontend/src/types/events.gen.ts from Rust payloads.\n\
         \x20 test [--ui] [--core-only]           Test gate: cargo unit+integration+API; Playwright only with --ui.\n\
         \x20 release-publish <tag> <artifact>... Upload artifacts + checksums to a GitHub Release.\n\
         \x20 release-download [opts]             Download a release, verify checksums, optional smoke test.\n\
         \x20 report-finding \"<title>\" [\"<note>\"]  Append a finding to todo.md with the next HR-### id.\n\
         \x20 help                                Show this help.\n\
         \n\
         Run `cargo xtask <command> --help` for command-specific options.\n"
    );
}
