#!/usr/bin/env bash
# DEPRECATED: prefer `cargo xtask release-download` (one cross-platform impl;
# native checksum verify that also works on Windows). Kept as a fallback.
#
# Download a Harsh Realm build from a GitHub Release and verify checksums.
#
# Usage: scripts/release-download.sh [--tag <tag> | --latest]
#        [--pattern <glob>] [--dir <dir>] [--smoke | --server-smoke]
#
# Requires: gh CLI authenticated to cyrex562, sha256sum, curl.
# --server-smoke additionally requires: cargo, npm, npx playwright.
set -euo pipefail

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------
usage() {
    cat <<'EOF'
Usage: scripts/release-download.sh [--tag <tag> | --latest]
       [--pattern <glob>] [--dir <dir>] [--smoke | --server-smoke]

Options:
  --tag <tag>       Exact release tag to download (e.g. v1.2.3)
  --latest          Resolve and download the latest release
  --pattern <glob>  Asset glob passed to gh release download (default: *.exe)
  --dir <dir>       Directory for downloaded files (default: dist)
  --smoke           Windows smoke test: launch the downloaded .exe in the
                    background, poll http://127.0.0.1:8080/api/worlds for
                    an HTTP 200 (up to 30 s), then terminate the process.
                    Skipped with a warning when not running on Windows.
  --server-smoke    Linux smoke test: start the local web host via
                    cargo run, poll /api/worlds for 200, then optionally
                    run Playwright @smoke tests (soft warning if none found).
  -h, --help        Show this help and exit

Examples:
  scripts/release-download.sh --latest
  scripts/release-download.sh --tag v1.2.3 --pattern "*.exe" --dir out
  scripts/release-download.sh --latest --server-smoke
EOF
}

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
TAG=""
USE_LATEST=0
PATTERN="*.exe"
DOWNLOAD_DIR="dist"
DO_SMOKE=0
DO_SERVER_SMOKE=0

# ---------------------------------------------------------------------------
# Parse arguments
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)        usage; exit 0 ;;
        --tag)            TAG="$2"; shift 2 ;;
        --latest)         USE_LATEST=1; shift ;;
        --pattern)        PATTERN="$2"; shift 2 ;;
        --dir)            DOWNLOAD_DIR="$2"; shift 2 ;;
        --smoke)          DO_SMOKE=1; shift ;;
        --server-smoke)   DO_SERVER_SMOKE=1; shift ;;
        *) echo "ERROR: unknown option: $1" >&2; usage >&2; exit 1 ;;
    esac
done

if [[ $USE_LATEST -eq 0 && -z "$TAG" ]]; then
    echo "ERROR: one of --tag or --latest is required" >&2
    usage >&2
    exit 1
fi

if [[ $USE_LATEST -eq 1 && -n "$TAG" ]]; then
    echo "ERROR: --tag and --latest are mutually exclusive" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Resolve tag
# ---------------------------------------------------------------------------
if [[ $USE_LATEST -eq 1 ]]; then
    echo "== Resolving latest release tag =="
    TAG=$(gh release view --json tagName -q .tagName)
    echo "== Latest tag: $TAG =="
fi

# ---------------------------------------------------------------------------
# OS detection helper
# ---------------------------------------------------------------------------
is_windows() {
    case "${OSTYPE:-}" in
        msys*|cygwin*|win32) return 0 ;;
    esac
    case "$(uname -s 2>/dev/null)" in
        MINGW*|CYGWIN*|Windows_NT) return 0 ;;
    esac
    return 1
}

# ---------------------------------------------------------------------------
# Create download directory
# ---------------------------------------------------------------------------
mkdir -p "$DOWNLOAD_DIR"

# ---------------------------------------------------------------------------
# Download artifacts + SHA256SUMS
# ---------------------------------------------------------------------------
echo "== Downloading '$PATTERN' assets from release $TAG into $DOWNLOAD_DIR =="
gh release download "$TAG" --pattern "$PATTERN" --dir "$DOWNLOAD_DIR" --clobber

echo "== Downloading SHA256SUMS =="
gh release download "$TAG" --pattern "SHA256SUMS" --dir "$DOWNLOAD_DIR" --clobber

# ---------------------------------------------------------------------------
# Verify checksums
# ---------------------------------------------------------------------------
echo "== Verifying SHA256SUMS =="
(cd "$DOWNLOAD_DIR" && sha256sum -c SHA256SUMS)
echo "== Checksums OK =="

# ---------------------------------------------------------------------------
# Helpers: health poll + server teardown
# ---------------------------------------------------------------------------
SERVER_PID=""

cleanup_server() {
    if [[ -n "$SERVER_PID" ]]; then
        echo "== Terminating web host (pid $SERVER_PID) =="
        kill "$SERVER_PID" 2>/dev/null || true
        SERVER_PID=""
    fi
}

poll_health() {
    local start=$SECONDS
    echo "== Polling http://127.0.0.1:8080/api/worlds (up to 30 s) =="
    while (( SECONDS - start < 30 )); do
        if curl -sf "http://127.0.0.1:8080/api/worlds" >/dev/null 2>&1; then
            echo "== Health check: HTTP 200 received =="
            return 0
        fi
        sleep 1
    done
    echo "ERROR: server did not respond within 30 s" >&2
    return 1
}

# ---------------------------------------------------------------------------
# --smoke  (Windows: launch the downloaded exe)
# ---------------------------------------------------------------------------
if [[ $DO_SMOKE -eq 1 ]]; then
    if is_windows; then
        echo "== Smoke test (Windows) =="

        # Prefer the named bare exe; fall back to the first .exe in the dir.
        EXE_PATH=""
        if [[ -f "$DOWNLOAD_DIR/harsh-realm-desktop.exe" ]]; then
            EXE_PATH="$DOWNLOAD_DIR/harsh-realm-desktop.exe"
        else
            EXE_PATH=$(ls "$DOWNLOAD_DIR"/*.exe 2>/dev/null | head -1 || true)
        fi

        if [[ -z "$EXE_PATH" ]]; then
            echo "ERROR: no .exe found in $DOWNLOAD_DIR for smoke test" >&2
            exit 1
        fi

        echo "== Launching: $EXE_PATH =="
        "$EXE_PATH" &
        SERVER_PID=$!
        trap cleanup_server EXIT

        if poll_health; then
            echo "== Smoke test PASSED =="
        else
            echo "== Smoke test FAILED =="
            exit 1
        fi
    else
        echo "== --smoke only applies on Windows; skipping exe launch =="
        echo "   (SHA256 verification above still ran successfully)"
    fi
fi

# ---------------------------------------------------------------------------
# --server-smoke  (Linux: start local harsh-web, optionally run Playwright)
# ---------------------------------------------------------------------------
if [[ $DO_SERVER_SMOKE -eq 1 ]]; then
    echo "== Server smoke test (Linux) =="
    trap cleanup_server EXIT

    echo "== Starting harsh-web (cargo run) =="
    cargo run --manifest-path crates/harsh-web/Cargo.toml &
    SERVER_PID=$!

    if poll_health; then
        echo "== Web host is up =="
    else
        echo "== ERROR: web host did not come up in time =="
        exit 1
    fi

    echo "== Running Playwright @smoke tests =="
    set +e
    PW_OUTPUT=$(cd frontend && REUSE_SERVERS=1 npx playwright test --grep @smoke 2>&1)
    PW_EXIT=$?
    set -e

    echo "$PW_OUTPUT"

    if [[ $PW_EXIT -ne 0 ]]; then
        if echo "$PW_OUTPUT" | grep -qiE 'no tests|0 tests found|0 passed'; then
            echo "== Warning: no @smoke-tagged Playwright tests found; skipping =="
        else
            echo "== ERROR: Playwright smoke tests FAILED (exit $PW_EXIT) =="
            exit $PW_EXIT
        fi
    else
        echo "== Playwright smoke tests PASSED =="
    fi
fi

echo "== Done =="
