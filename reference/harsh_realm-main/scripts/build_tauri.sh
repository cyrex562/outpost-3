#!/usr/bin/env bash
# Build the Harsh Realm Tauri desktop shell.
#
# Intended to run inside the build container (docker/tauri-build.Dockerfile)
# with the repo bind-mounted at the working directory, but works on any host
# that has the Tauri prerequisites installed (see src-tauri/README.md).
#
#   docker run --rm -v "$PWD":/work -w /work \
#     -v harsh-realm-cargo:/cargo harsh-realm-tauri-build \
#     bash scripts/build_tauri.sh [--bundle]
#
# Without --bundle it compiles the shell (debug) and runs the harsh-core tests,
# which is enough to verify everything links. With --bundle it produces release
# installers under src-tauri/target/release/bundle/.
set -euo pipefail

BUNDLE=0
[[ "${1:-}" == "--bundle" ]] && BUNDLE=1

echo "== Generating application icons =="
tauri icon src-tauri/icons/app-icon-source.png

echo "== Testing pure-Rust core (harsh-core) =="
cargo test --manifest-path crates/harsh-core/Cargo.toml

echo "== Installing + building frontend =="
npm --prefix frontend ci
npm --prefix frontend run build

if [[ "$BUNDLE" -eq 1 ]]; then
    echo "== Building release bundle (tauri build) =="
    tauri build
else
    echo "== Compiling desktop shell (debug) =="
    cargo build --manifest-path src-tauri/Cargo.toml
fi

echo "== Done =="
