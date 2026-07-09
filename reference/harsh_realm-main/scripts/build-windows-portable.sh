#!/usr/bin/env bash
# Cross-compile a portable Windows .exe for Harsh Realm from Linux.
#
# ╔══════════════════════════════════════════════════════════════════════════╗
# ║  BEST-EFFORT BUILD  —  Track A                                          ║
# ║                                                                          ║
# ║  This script uses cargo-xwin to cross-compile the Tauri desktop shell   ║
# ║  for Windows (x86_64-pc-windows-msvc) without a Windows host.           ║
# ║                                                                          ║
# ║  Cross-compilation is FRAGILE:                                           ║
# ║    • Tauri's WiX/NSIS installer bundlers do not work cross-platform.    ║
# ║    • Icon/resource embedding may be incomplete.                          ║
# ║    • Linking edge cases can appear that are hard to debug from Linux.   ║
# ║                                                                          ║
# ║  The authoritative Windows release build is Track B:                    ║
# ║    scripts/build-windows.ps1   (run on a real Windows host)             ║
# ║                                                                          ║
# ║  Use this script only for smoke-testing or CI gating where you don't    ║
# ║  have access to a Windows runner and a raw .exe is sufficient.          ║
# ╚══════════════════════════════════════════════════════════════════════════╝
#
# Usage:
#   scripts/build-windows-portable.sh [flags]
#
# Flags:
#   --setup-only  Install the MSVC target and cargo-xwin, then exit.
#   -h, --help    Print this help and exit.
#
# Output:
#   src-tauri/target/x86_64-pc-windows-msvc/release/harsh-realm-desktop.exe
#   (+ a .sha256 sidecar file)
#
# Run from the repository root.
set -euo pipefail

# ---------------------------------------------------------------------------
# Usage
# ---------------------------------------------------------------------------
usage() {
    cat <<'EOF'
Usage: scripts/build-windows-portable.sh [flags]

  --setup-only  Install rustup target + cargo-xwin, then exit.
  -h, --help    Print this help and exit.
EOF
}

# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------
SETUP_ONLY=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --setup-only) SETUP_ONLY=1 ;;
        -h|--help)    usage; exit 0 ;;
        *) echo "Unknown flag: $1" >&2; echo "" >&2; usage >&2; exit 1 ;;
    esac
    shift
done

# ---------------------------------------------------------------------------
# Banner
# ---------------------------------------------------------------------------
echo "================================================================"
echo "==  Harsh Realm — BEST-EFFORT Windows portable build          =="
echo "==  Track A (Linux cross-compile via cargo-xwin)              =="
echo "==  Authoritative build: scripts/build-windows.ps1 (Track B)  =="
echo "================================================================"

# ---------------------------------------------------------------------------
# Preflight / setup (each step guarded — only runs if missing)
# ---------------------------------------------------------------------------
echo "== Preflight: checking x86_64-pc-windows-msvc target =="
if ! rustup target list --installed | grep -q 'x86_64-pc-windows-msvc'; then
    echo "   Target not found — running: rustup target add x86_64-pc-windows-msvc"
    rustup target add x86_64-pc-windows-msvc
else
    echo "   x86_64-pc-windows-msvc already installed."
fi

echo "== Preflight: checking cargo-xwin =="
if ! command -v cargo-xwin &>/dev/null; then
    echo "   cargo-xwin not found — running: cargo install cargo-xwin"
    cargo install cargo-xwin
else
    echo "   cargo-xwin found at $(command -v cargo-xwin)."
fi

if [[ "$SETUP_ONLY" -eq 1 ]]; then
    echo "== Setup complete (--setup-only). Exiting. =="
    exit 0
fi

# ---------------------------------------------------------------------------
# Build frontend assets (embedded into the Tauri shell)
# ---------------------------------------------------------------------------
echo "== Building frontend =="
npm --prefix frontend run build

# ---------------------------------------------------------------------------
# Cross-compile the Tauri desktop shell
# ---------------------------------------------------------------------------
EXE_PATH="src-tauri/target/x86_64-pc-windows-msvc/release/harsh-realm-desktop.exe"
SHA_PATH="${EXE_PATH}.sha256"

echo "== Cross-compiling Tauri desktop shell for x86_64-pc-windows-msvc =="

# Capture cross-compile failure without triggering set -e so we can print a
# clear, actionable message before exiting.
cross_failed=0
cargo xwin build --release \
    --manifest-path src-tauri/Cargo.toml \
    --target x86_64-pc-windows-msvc \
    || cross_failed=1

if [[ "$cross_failed" -eq 1 ]]; then
    echo "" >&2
    echo "================================================================" >&2
    echo "==  CROSS-COMPILATION FAILED                                  ==" >&2
    echo "==                                                            ==" >&2
    echo "==  cargo-xwin cross-compilation is fragile and is not the   ==" >&2
    echo "==  recommended release path for Harsh Realm on Windows.      ==" >&2
    echo "==                                                            ==" >&2
    echo "==  Fall back to the authoritative Track B build:            ==" >&2
    echo "==    scripts/build-windows.ps1   (run on a Windows host)    ==" >&2
    echo "================================================================" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Post-build: checksum + output summary
# ---------------------------------------------------------------------------
echo "== Computing SHA-256 checksum =="
CHECKSUM=$(sha256sum "$EXE_PATH" | awk '{print $1}')
echo "${CHECKSUM}  harsh-realm-desktop.exe" > "$SHA_PATH"

echo ""
echo "== Build complete =="
echo "   Exe:      ${EXE_PATH}"
echo "   Checksum: ${CHECKSUM}"
echo "   SHA file: ${SHA_PATH}"
echo ""
echo "================================================================"
echo "==  REMINDER — WebView2 required on the target Windows host   =="
echo "==                                                            =="
echo "==  WebView2 is pre-installed on Windows 11 and recent        =="
echo "==  Windows 10 builds.  Older hosts need the standalone       =="
echo "==  WebView2 Runtime:                                         =="
echo "==  https://developer.microsoft.com/en-us/microsoft-edge/     =="
echo "==  webview2/                                                  =="
echo "================================================================"
