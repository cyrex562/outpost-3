//! `cargo xtask gen-types` — regenerate `frontend/src/types/events.gen.ts`.
//!
//! Runs the `export-types` binary from `crates/harsh-core`, which calls
//! `harsh_core::payloads::client_event::generate_ts_bindings()` and writes
//! the result to `frontend/src/types/events.gen.ts`.
//!
//! After running, commit `events.gen.ts` so the `committed_events_gen_matches_export`
//! drift-gate test passes in CI.

use crate::util::Res;

/// `cargo xtask gen-types`
pub fn gen_types(args: &[String]) -> Res<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("Usage: cargo xtask gen-types");
        println!("  Regenerates frontend/src/types/events.gen.ts from Rust payload structs.");
        println!("  Commit the result so the drift-gate test passes in CI.");
        return Ok(());
    }

    crate::util::run(
        "cargo",
        &[
            "run",
            "--manifest-path",
            "crates/harsh-core/Cargo.toml",
            "--bin",
            "export-types",
            "--quiet",
        ],
    )?;

    println!();
    println!("== gen-types: PASS (commit frontend/src/types/events.gen.ts) ==");
    Ok(())
}
