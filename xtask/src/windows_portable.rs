//! `cargo xtask build-windows-portable` / `cargo xtask setup-windows` —
//! best-effort cross-compile of the Tauri desktop shell to Windows from a
//! non-Windows host, producing a portable (installer-free) bundle.
//!
//! This is a BEST-EFFORT path, not the authoritative Windows release build:
//!
//! - Tauri's NSIS/WiX installer bundlers only run on a real Windows host.
//!   `outpost_tauri` is deliberately excluded from the root workspace (see
//!   `Cargo.toml`) because it needs WebKit2GTK system libs on Linux and
//!   MSVC + WebView2 on Windows — see `CLAUDE.md`.
//! - Cross-compiling sidesteps WebKit2GTK (Tauri only pulls it in for the
//!   `linux` target) but still needs the Windows target plus `cargo-xwin`
//!   (a prebuilt MSVC CRT/Windows SDK snapshot, since there's no real
//!   Windows SDK on a Linux host).
//! - Rather than an installer, this produces a "portable" bundle: the raw
//!   `.exe` plus any DLLs Tauri's build script placed alongside it, zipped
//!   up so it can just be unzipped and run — no install step, matching how
//!   Tauri v2's default (evergreen) WebView2 runtime already works without
//!   a bundled DLL as long as WebView2 itself is present on the target
//!   machine (pre-installed on Windows 11 / recent Windows 10).
//!
//! The authoritative Windows build remains `cargo tauri build`, run ON
//! Windows from inside `outpost_tauri/`.

use std::fs;

use crate::util::{capture, repo_root, run, run_ok, sha256_file, Res};

const TARGET: &str = "x86_64-pc-windows-msvc";
const BIN_NAME: &str = "outpost_tauri";
const PRODUCT_NAME: &str = "Outpost 3";

/// Install the Windows cross-compile target + `cargo-xwin`, if missing.
/// Idempotent — safe to run repeatedly, and reused as
/// [`build_windows_portable`]'s preflight step.
pub fn setup_windows() -> Res<()> {
    println!("== Preflight: rustup target {TARGET} ==");
    let target_list = capture("rustup", &["target", "list", "--installed"]).ok_or_else(|| {
        "`rustup` is required to add the Windows cross-compile target".to_string()
    })?;
    if target_list.lines().any(|l| l.trim() == TARGET) {
        println!("   {TARGET} already installed.");
    } else {
        println!("   installing: rustup target add {TARGET}");
        run("rustup", &["target", "add", TARGET])?;
    }

    println!("== Preflight: cargo-xwin ==");
    if run_ok("cargo", &["xwin", "--version"]) {
        println!("   cargo-xwin already installed.");
    } else {
        println!("   installing: cargo install cargo-xwin");
        run("cargo", &["install", "cargo-xwin"])?;
    }
    Ok(())
}

/// Cross-compile `outpost_tauri` for Windows and assemble a portable
/// (installer-free) zip under `dist/`.
pub fn build_windows_portable() -> Res<()> {
    setup_windows()?;

    println!("== Building frontend ==");
    run("npm", &["--prefix", "frontend", "ci"])?;
    run("npm", &["--prefix", "frontend", "run", "build"])?;

    println!("== Cross-compiling {BIN_NAME} for {TARGET} (cargo-xwin) ==");
    let cross_ok = run_ok(
        "cargo",
        &[
            "xwin",
            "build",
            "--release",
            "--manifest-path",
            "outpost_tauri/Cargo.toml",
            "--target",
            TARGET,
        ],
    );
    if !cross_ok {
        return Err("cross-compilation failed.\n\
             cargo-xwin cross-builds are best-effort and not the recommended release path.\n\
             Fall back to building on a real Windows host:\n\
             \x20   cd outpost_tauri && cargo tauri build"
            .into());
    }

    let root = repo_root();
    let exe = root
        .join("outpost_tauri/target")
        .join(TARGET)
        .join("release")
        .join(format!("{BIN_NAME}.exe"));
    if !exe.exists() {
        return Err(format!("expected exe not found: {}", exe.display()));
    }

    println!("== Assembling portable bundle ==");
    let stage = root.join("dist/windows-portable").join(PRODUCT_NAME);
    if stage.exists() {
        fs::remove_dir_all(&stage).map_err(|e| format!("clean {}: {e}", stage.display()))?;
    }
    fs::create_dir_all(&stage).map_err(|e| format!("mkdir {}: {e}", stage.display()))?;

    let staged_exe = stage.join(format!("{PRODUCT_NAME}.exe"));
    fs::copy(&exe, &staged_exe)
        .map_err(|e| format!("copy {} -> {}: {e}", exe.display(), staged_exe.display()))?;

    // Any DLLs tauri-build placed next to the exe (e.g. a WebView2Loader.dll,
    // when using the fixed-version runtime rather than the default
    // evergreen one) travel with it — copied opportunistically so a
    // fixed-runtime build still works portably even though the default
    // build needs none of these.
    if let Some(parent) = exe.parent() {
        for entry in
            fs::read_dir(parent).map_err(|e| format!("read_dir {}: {e}", parent.display()))?
        {
            let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("dll") {
                let dest = stage.join(entry.file_name());
                fs::copy(&path, &dest)
                    .map_err(|e| format!("copy {} -> {}: {e}", path.display(), dest.display()))?;
            }
        }
    }

    let readme = stage.join("README.txt");
    fs::write(
        &readme,
        format!(
            "{PRODUCT_NAME} — portable Windows build\n\
             \n\
             Run \"{PRODUCT_NAME}.exe\" directly; no installer needed.\n\
             \n\
             Requires the Microsoft Edge WebView2 Runtime, pre-installed on\n\
             Windows 11 and recent Windows 10 builds. If the app fails to\n\
             start, install it from:\n\
             \x20   https://developer.microsoft.com/microsoft-edge/webview2/\n\
             \n\
             This build was cross-compiled from a non-Windows host via\n\
             cargo-xwin (`cargo xtask build-windows-portable`) rather than\n\
             built natively with `cargo tauri build` on Windows — treat it\n\
             as best-effort and verify it against a native build before\n\
             release.\n"
        ),
    )
    .map_err(|e| format!("write {}: {e}", readme.display()))?;

    let hash = sha256_file(&staged_exe)?;
    fs::write(
        stage.join("SHA256SUMS"),
        format!("{hash}  {PRODUCT_NAME}.exe\n"),
    )
    .map_err(|e| format!("write SHA256SUMS: {e}"))?;

    println!("== Zipping ==");
    let zip_path = root.join("dist/outpost3-windows-portable-x86_64.zip");
    if zip_path.exists() {
        fs::remove_file(&zip_path).map_err(|e| format!("rm {}: {e}", zip_path.display()))?;
    }
    let zip_ok = std::process::Command::new("zip")
        .args(["-r", "-q"])
        .arg(&zip_path)
        .arg(PRODUCT_NAME)
        .current_dir(root.join("dist/windows-portable"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    println!();
    println!("== Build complete ==");
    println!("  Staged folder: {}", stage.display());
    println!("  Exe checksum : {hash}");
    if zip_ok {
        println!("  Zip archive  : {}", zip_path.display());
    } else {
        println!(
            "  `zip` unavailable or failed — the staged folder above is still a complete \
             portable bundle, just not zipped."
        );
    }
    Ok(())
}
