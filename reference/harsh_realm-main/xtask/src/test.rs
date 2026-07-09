//! `cargo xtask test` — the test gate for the automated fix loop.
//!
//! Policy (see AGENTS.md "Testing"): unit + integration + API tests via
//! `cargo test` on both crates. No mutation testing. Playwright runs only when
//! `--ui` is passed (i.e. the change touches UI behaviour); day-to-day UI
//! verification is a human checklist, not part of this gate.

use crate::util::{run, run_in, Res};

/// `cargo xtask test [--ui] [--core-only]`
pub fn test(args: &[String]) -> Res<()> {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("Usage: cargo xtask test [--ui] [--core-only]");
        println!("  Runs cargo unit+integration+API tests on harsh-core and harsh-web.");
        println!("  --ui         also run the Playwright suite (UI changes only)");
        println!("  --core-only  only harsh-core (fastest inner loop)");
        return Ok(());
    }
    let ui = args.iter().any(|a| a == "--ui");
    let core_only = args.iter().any(|a| a == "--core-only");

    // Unit + property + integration tests (harsh-core).
    run(
        "cargo",
        &["test", "--manifest-path", "crates/harsh-core/Cargo.toml"],
    )?;

    if !core_only {
        // API / route / websocket integration tests + build (harsh-web).
        run(
            "cargo",
            &["test", "--manifest-path", "crates/harsh-web/Cargo.toml"],
        )?;
        run(
            "cargo",
            &["build", "--manifest-path", "crates/harsh-web/Cargo.toml"],
        )?;
    }

    // Frontend type-check + fast unit tests (pure TS / vitest, no browser needed;
    // always runs so a worldModel regression surfaces on every `cargo xtask test`).
    run_in("frontend", "npm", &["run", "type-check"])?;
    run_in("frontend", "npm", &["run", "test:unit"])?;

    if ui {
        // Full build (includes vue-tsc) + Playwright (UI behaviour changes only).
        // Run from the frontend dir so Playwright config resolves correctly.
        run_in("frontend", "npm", &["run", "build"])?;
        run_in("frontend", "npm", &["run", "test:e2e"])?;
    } else {
        println!();
        println!("== Skipped Playwright (no --ui). Pass --ui for UI-behaviour changes. ==");
    }

    println!();
    println!("== test gate: PASS ==");
    Ok(())
}
