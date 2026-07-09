//! `cargo xtask build-windows` — build the Tauri desktop bundle on Windows.

use std::fs;

use crate::util::{first_exe, repo_root, run, run_ok, sha256_file, Res};

/// Build the Windows desktop bundle (Track B) and emit `SHA256SUMS`.
///
/// Mirrors the former `scripts/build-windows.ps1`. Requires (on the Windows
/// host): Node/npm, a Rust toolchain, `cargo install tauri-cli --version "^2"`,
/// and the MSVC C++ build tools.
pub fn build_windows() -> Res<()> {
    if !cfg!(windows) {
        return Err("build-windows produces the Windows desktop bundle and must run on Windows.\n\
                    On Linux, use the best-effort cross-compile instead: scripts/build-windows-portable.sh"
            .into());
    }

    // 0. Preflight: the Tauri CLI provides the `cargo tauri` subcommand. Check
    //    it up front so we fail fast (before the slow frontend build) with an
    //    actionable message instead of cargo's terse "no such command".
    if !run_ok("cargo", &["tauri", "--version"]) {
        return Err("the Tauri CLI (`cargo tauri`) is not installed.\n\
                    Install it, then re-run `cargo xtask build-windows`:\n\
                    \x20   cargo install tauri-cli --version \"^2\"\n\
                    (this is the cargo subcommand; `npm i -g @tauri-apps/cli` gives a\n\
                    \x20standalone `tauri`, which this build does not use.)"
            .into());
    }

    // 1. Frontend.
    run("npm", &["--prefix", "frontend", "ci"])?;
    run("npm", &["--prefix", "frontend", "run", "build"])?;

    // 2. Tauri desktop bundle (NSIS installer + bare exe).
    run("cargo", &["tauri", "build"])?;

    // 3. Locate artifacts.
    let root = repo_root();
    let nsis_dir = root.join("src-tauri/target/release/bundle/nsis");
    let nsis = first_exe(&nsis_dir)?
        .ok_or_else(|| format!("NSIS installer not found under {}", nsis_dir.display()))?;
    let bare = root.join("src-tauri/target/release/harsh-realm-desktop.exe");
    if !bare.exists() {
        return Err(format!("bare executable not found: {}", bare.display()).into());
    }

    // 4. Checksums: "<lowercase-hash>  <basename>\n", LF endings so
    //    `sha256sum -c SHA256SUMS` on Linux parses it cleanly.
    let sums_path = root.join("SHA256SUMS");
    let mut sums = String::new();
    for artifact in [&nsis, &bare] {
        let hash = sha256_file(artifact)?;
        let name = artifact
            .file_name()
            .expect("artifact path has a file name")
            .to_string_lossy();
        sums.push_str(&format!("{hash}  {name}\n"));
    }
    fs::write(&sums_path, sums.as_bytes())?;

    // 5. Summary + next step.
    println!();
    println!("== Build complete ==");
    println!("  NSIS installer : {}", nsis.display());
    println!("  Bare executable: {}", bare.display());
    println!("  Checksums      : {}", sums_path.display());
    println!();
    println!("Next step - publish (from the repo root):");
    println!(
        "  cargo xtask release-publish TAG \"{}\" \"{}\"",
        nsis.display(),
        bare.display()
    );
    println!("  (replace TAG with the release tag, e.g. v0.50.1)");
    Ok(())
}
