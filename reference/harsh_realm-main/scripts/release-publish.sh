#!/usr/bin/env bash
# DEPRECATED: prefer `cargo xtask release-publish` (one cross-platform impl).
# Kept as a fallback; will be retired once the xtask flow is exercised against
# a real GitHub Release.
#
# Publish Harsh Realm build artifacts to a GitHub Release.
#
# Usage: scripts/release-publish.sh <tag> <artifact> [<artifact>...]
#
# The tag must match vX.Y.Z. A SHA256SUMS file is computed (basenames only)
# and uploaded alongside the artifacts. If the release already exists the
# assets are uploaded with --clobber; otherwise the release is created.
#
# Requires: gh CLI authenticated to cyrex562, sha256sum, realpath.
set -euo pipefail

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------
usage() {
    cat <<'EOF'
Usage: scripts/release-publish.sh <tag> <artifact> [<artifact>...]

Arguments:
  tag        Release tag in vX.Y.Z format (e.g. v1.2.3)
  artifact   One or more local file paths to upload

Options:
  -h, --help  Show this help message and exit

Examples:
  scripts/release-publish.sh v1.2.3 dist/harsh-realm-desktop.exe
  scripts/release-publish.sh v1.2.3 installer.exe harsh-realm-desktop.exe
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage; exit 0
fi

# ---------------------------------------------------------------------------
# Validate arguments
# ---------------------------------------------------------------------------
if [[ $# -lt 2 ]]; then
    echo "ERROR: expected <tag> and at least one <artifact>" >&2
    usage >&2
    exit 1
fi

TAG="$1"; shift
ARTIFACTS=("$@")

if ! [[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "ERROR: tag must match vX.Y.Z (e.g. v1.2.3), got: $TAG" >&2
    exit 1
fi

MISSING=0
for art in "${ARTIFACTS[@]}"; do
    if [[ ! -f "$art" ]]; then
        echo "ERROR: artifact not found: $art" >&2
        MISSING=1
    fi
done
[[ "$MISSING" -eq 1 ]] && exit 1

# ---------------------------------------------------------------------------
# Compute SHA256SUMS (basenames only — cd into each artifact's directory)
# ---------------------------------------------------------------------------
SUMS_DIR=$(mktemp -d)
trap 'rm -rf "$SUMS_DIR"' EXIT
SHA256FILE="$SUMS_DIR/SHA256SUMS"

echo "== Computing SHA256SUMS =="
for art in "${ARTIFACTS[@]}"; do
    ART_ABS=$(realpath "$art")
    ART_DIR=$(dirname "$ART_ABS")
    ART_BASE=$(basename "$ART_ABS")
    (cd "$ART_DIR" && sha256sum "$ART_BASE") >> "$SHA256FILE"
done
cat "$SHA256FILE"

# ---------------------------------------------------------------------------
# Create or update the GitHub Release
# ---------------------------------------------------------------------------
if gh release view "$TAG" &>/dev/null; then
    echo "== Uploading to existing release $TAG =="
    gh release upload "$TAG" --clobber "${ARTIFACTS[@]}" "$SHA256FILE"
else
    echo "== Creating release $TAG =="
    gh release create "$TAG" \
        --title "$TAG" \
        --notes "Harsh Realm $TAG desktop build" \
        "${ARTIFACTS[@]}" "$SHA256FILE"
fi

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
echo "== Release URL =="
gh release view "$TAG" --json url -q .url

echo "== Done =="
