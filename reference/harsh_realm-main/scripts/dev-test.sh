#!/usr/bin/env bash
# Run the full Linux correctness loop for Harsh Realm.
#
# Executes each verification gate in order, collects pass/fail for every gate,
# and prints a summary table at the end.  All gates run regardless of
# individual failures so you get a complete picture in one pass.
#
# Usage:
#   scripts/dev-test.sh [flags]
#
# Flags:
#   --fast                 Skip the Playwright e2e gate (step 5).
#   --no-frontend-install  Skip "npm ci"; reuse existing node_modules.
#   --serve                Instead of running gates, launch the web host for a
#                          manual UI check at http://localhost:8080.
#                          Mutually exclusive with the test gates.
#   -h, --help             Print this help and exit.
#
# Run from the repository root.
set -euo pipefail

# ---------------------------------------------------------------------------
# Usage
# ---------------------------------------------------------------------------
usage() {
    cat <<'EOF'
Usage: scripts/dev-test.sh [flags]

  --fast                 Skip the Playwright e2e gate (step 5).
  --no-frontend-install  Skip "npm ci"; reuse existing node_modules.
  --serve                Launch the web host at http://localhost:8080
                         instead of running test gates.
  -h, --help             Print this help and exit.
EOF
}

# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------
FAST=0
NO_FRONTEND_INSTALL=0
SERVE=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --fast)                FAST=1 ;;
        --no-frontend-install) NO_FRONTEND_INSTALL=1 ;;
        --serve)               SERVE=1 ;;
        -h|--help)             usage; exit 0 ;;
        *) echo "Unknown flag: $1" >&2; echo "" >&2; usage >&2; exit 1 ;;
    esac
    shift
done

# ---------------------------------------------------------------------------
# --serve mode: launch the web host and hand off; no summary needed.
# ---------------------------------------------------------------------------
if [[ "$SERVE" -eq 1 ]]; then
    echo "== Launching harsh-web for manual UI check =="
    echo "   UI available at http://localhost:8080"
    echo "   (Set HARSH_REALM_PORT to override the port.)"
    echo "   Press Ctrl-C to stop."
    exec cargo run --manifest-path crates/harsh-web/Cargo.toml
fi

# ---------------------------------------------------------------------------
# Gate runner
# ---------------------------------------------------------------------------
gate_names=()
gate_results=()
gate_times=()
any_failed=0

# run_gate "Human-readable name" cmd [args...]
# Runs the command, records PASS/FAIL and elapsed seconds, never aborts the
# script on failure (all gates are always attempted).
run_gate() {
    local name="$1"
    shift
    local start=$SECONDS
    local result
    if "$@"; then
        result="PASS"
    else
        result="FAIL"
        any_failed=1
    fi
    local elapsed=$(( SECONDS - start ))
    gate_names+=("$name")
    gate_results+=("$result")
    gate_times+=("${elapsed}s")
    echo "== [${result}] '${name}' completed in ${elapsed}s =="
}

print_summary() {
    echo ""
    echo "======================================================="
    echo "== dev-test summary                                  =="
    echo "======================================================="
    local i
    for i in "${!gate_names[@]}"; do
        printf "  [%-4s]  %-44s  %s\n" \
            "${gate_results[$i]}" "${gate_names[$i]}" "${gate_times[$i]}"
    done
    echo "======================================================="
    if [[ "$any_failed" -eq 0 ]]; then
        echo "== All gates PASSED =="
    else
        echo "== One or more gates FAILED =="
    fi
}

# Print summary on any exit (including unexpected failures from set -e).
trap print_summary EXIT

# ---------------------------------------------------------------------------
# Gate 1: harsh-core tests
# Note: the cargo test suite for harsh-core includes the IR schema-drift gate
# that verifies the Intermediate Representation types haven't silently changed.
# ---------------------------------------------------------------------------
echo "== Gate 1: cargo test (harsh-core, includes IR schema-drift gate) =="
run_gate "harsh-core tests (incl. IR schema-drift)" \
    cargo test --manifest-path crates/harsh-core/Cargo.toml

# ---------------------------------------------------------------------------
# Gate 2: harsh-web tests
# ---------------------------------------------------------------------------
echo "== Gate 2: cargo test (harsh-web) =="
run_gate "harsh-web tests" \
    cargo test --manifest-path crates/harsh-web/Cargo.toml

# ---------------------------------------------------------------------------
# Gate 3: harsh-web build (confirm the binary links cleanly)
# ---------------------------------------------------------------------------
echo "== Gate 3: cargo build (harsh-web) =="
run_gate "harsh-web build" \
    cargo build --manifest-path crates/harsh-web/Cargo.toml

# ---------------------------------------------------------------------------
# Gate 4: frontend install + build (vue-tsc --noEmit + vite build)
# ---------------------------------------------------------------------------
echo "== Gate 4: frontend build =="

# Wrap the conditional two-step in a function so run_gate gets a single exit
# code covering both npm ci and npm run build.
frontend_build() {
    if [[ "$NO_FRONTEND_INSTALL" -eq 0 ]]; then
        npm --prefix frontend ci
    else
        echo "   Skipping npm ci (--no-frontend-install set)."
    fi
    npm --prefix frontend run build
}

run_gate "frontend build (vue-tsc + vite)" frontend_build

# ---------------------------------------------------------------------------
# Gate 5: Playwright e2e
# The playwright.config.ts auto-starts both servers (harsh-web + vite dev).
# Set REUSE_SERVERS=1 if they are already running.
# ---------------------------------------------------------------------------
if [[ "$FAST" -eq 1 ]]; then
    echo "== Gate 5: Playwright e2e — SKIPPED (--fast) =="
else
    echo "== Gate 5: Playwright e2e =="
    run_gate "Playwright e2e" bash -c 'cd frontend && npm run test:e2e'
fi

# Summary is printed by the EXIT trap above.
exit "$any_failed"
