//! `cargo xtask build-windows-portable` / `cargo xtask setup-windows` — build
//! a portable (installer-free) Windows bundle of the Tauri desktop shell.
//!
//! Two build strategies, picked automatically by host OS:
//!
//! - **On Windows** (the common case — this is the machine that will run the
//!   app anyway): build `outpost_tauri` **natively**, using whatever MSVC
//!   toolchain `rustup`/Visual Studio Build Tools already provide. No
//!   `cargo-xwin` involved — cross-compilation is pointless when the host
//!   already *is* the target.
//! - **On Linux/macOS**: best-effort cross-compile via `cargo-xwin`, which
//!   downloads (`xwin`) a redistributable snapshot of the MSVC CRT/Windows
//!   SDK so linking can happen without a real Windows install. This sidesteps
//!   `outpost_tauri`'s WebKit2GTK requirement (Tauri only pulls that in for
//!   the `linux` target — see `CLAUDE.md` for why `outpost_tauri` is excluded
//!   from the root workspace) but `xwin`'s CRT/SDK "splat" step creates
//!   filesystem symlinks, which needs Developer Mode or admin elevation if
//!   ever run *on* Windows itself (`os error 1314`) — one more reason this
//!   path is Linux/macOS-only here.
//!
//! Either way, the output is a "portable" bundle — the raw `.exe` plus any
//! DLLs Tauri's build script placed alongside it, zipped up so it can just be
//! unzipped and run with no install step. This matches how Tauri v2's default
//! (evergreen) WebView2 runtime already works without a bundled DLL, as long
//! as WebView2 itself is present on the target machine (pre-installed on
//! Windows 11 / recent Windows 10).
//!
//! The authoritative *installer* build remains `cargo tauri build`, run on
//! Windows from inside `outpost_tauri/` — this command exists for a
//! zip-and-go dev/distribution artifact, not to replace NSIS/WiX packaging.

use std::fs;
use std::path::PathBuf;

use crate::util::{capture, repo_root, run, run_ok, sha256_file, Res};

const TARGET: &str = "x86_64-pc-windows-msvc";
const BIN_NAME: &str = "outpost_tauri";
const PRODUCT_NAME: &str = "Outpost 3";

/// Install the Windows cross-compile target + `cargo-xwin`, if missing.
/// Only relevant for the Linux/macOS cross-compile path — a no-op concept on
/// Windows itself, where [`build_windows_portable`] skips straight to a
/// native build instead of calling this.
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

/// Build `outpost_tauri` and assemble a portable (installer-free) zip under
/// `dist/`. Builds natively when already running on Windows; cross-compiles
/// via `cargo-xwin` otherwise (see module docs for why the two paths exist).
pub fn build_windows_portable() -> Res<()> {
    println!("== Building frontend ==");
    // `npm install` rather than `npm ci`: this is a local dev/portable-build
    // helper, not a CI pipeline — `ci`'s strict lockfile-sync requirement
    // (any drift between package.json and package-lock.json, including
    // benign per-platform optional-dependency differences) turns into a
    // hard failure here where `install` would just reconcile and continue.
    run("npm", &["--prefix", "frontend", "install"])?;
    run("npm", &["--prefix", "frontend", "run", "build"])?;

    let exe = if cfg!(windows) {
        build_native()?
    } else {
        build_cross_compiled()?
    };

    println!("== Assembling portable bundle ==");
    let root = repo_root();
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

    let readme_note = if cfg!(windows) {
        "This build was compiled natively on Windows via `cargo xtask build-windows-portable`."
    } else {
        "This build was cross-compiled from a non-Windows host via cargo-xwin\n\
         (`cargo xtask build-windows-portable`) rather than built natively with\n\
         `cargo tauri build` on Windows — treat it as best-effort and verify it\n\
         against a native build before release."
    };
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
             {readme_note}\n"
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
    let zip_ok = zip_directory(&root.join("dist/windows-portable"), PRODUCT_NAME, &zip_path);

    println!();
    println!("== Build complete ==");
    println!("  Staged folder: {}", stage.display());
    println!("  Exe checksum : {hash}");
    if zip_ok {
        println!("  Zip archive  : {}", zip_path.display());
    } else {
        println!(
            "  No zip tool available (tried `zip`{}) — the staged folder above is still a \
             complete portable bundle, just not zipped.",
            if cfg!(windows) {
                " and PowerShell's Compress-Archive"
            } else {
                ""
            }
        );
    }
    Ok(())
}

/// Build `outpost_tauri` natively for the host toolchain — the correct path
/// when already running on Windows, where the "cross-compile" target and
/// `cargo-xwin`'s downloaded CRT/SDK snapshot are both unnecessary (and
/// `xwin`'s symlink-based extraction step fails outright without Developer
/// Mode or admin elevation — `os error 1314`).
fn build_native() -> Res<PathBuf> {
    println!("== Building {BIN_NAME} natively (already on Windows) ==");
    run(
        "cargo",
        &[
            "build",
            "--release",
            "--manifest-path",
            "outpost_tauri/Cargo.toml",
            "--features",
            "custom-protocol",
        ],
    )?;
    let exe = repo_root()
        .join("outpost_tauri/target/release")
        .join(format!("{BIN_NAME}.exe"));
    if !exe.exists() {
        return Err(format!("expected exe not found: {}", exe.display()));
    }
    Ok(exe)
}

/// Cross-compile `outpost_tauri` for Windows from a non-Windows host via
/// `cargo-xwin`. Best-effort — see module docs.
fn build_cross_compiled() -> Res<PathBuf> {
    setup_windows()?;

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
            "--features",
            "custom-protocol",
        ],
    );
    if !cross_ok {
        return Err("cross-compilation failed.\n\
             cargo-xwin cross-builds are best-effort and not the recommended release path.\n\
             Fall back to building on a real Windows host:\n\
             \x20   cd outpost_tauri && cargo tauri build"
            .into());
    }

    let exe = repo_root()
        .join("outpost_tauri/target")
        .join(TARGET)
        .join("release")
        .join(format!("{BIN_NAME}.exe"));
    if !exe.exists() {
        return Err(format!("expected exe not found: {}", exe.display()));
    }
    Ok(exe)
}

/// Zip `dir_name` (a subdirectory of `parent_dir`) into `zip_path`. Tries the
/// `zip` CLI first (present by default on Linux/macOS, sometimes on Windows
/// via Git for Windows); falls back to PowerShell's `Compress-Archive` on
/// Windows, where `zip` usually isn't installed. Returns `false` (not an
/// error) if neither is available — the caller treats an unzipped staged
/// folder as an acceptable, if less convenient, result.
fn zip_directory(parent_dir: &std::path::Path, dir_name: &str, zip_path: &std::path::Path) -> bool {
    let zip_cli_ok = std::process::Command::new("zip")
        .args(["-r", "-q"])
        .arg(zip_path)
        .arg(dir_name)
        .current_dir(parent_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if zip_cli_ok {
        return true;
    }
    if !cfg!(windows) {
        return false;
    }
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Compress-Archive -Path '{dir_name}' -DestinationPath '{}' -Force",
                zip_path.display()
            ),
        ])
        .current_dir(parent_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
