#!/usr/bin/env bash
# DEPRECATED: prefer `cargo xtask report-finding` (one cross-platform impl).
# Kept as a fallback; will be retired once the xtask flow is confirmed in use.
#
# Append a playtest or build finding to todo.md under the
# "## Playtest / build feedback" section, auto-assigning the next HR-### id.
#
# Usage: scripts/report-finding.sh "<title>" ["<note>"]
#
# Scans todo.md AND todo-archive.md for the highest existing HR-### id and
# assigns the next one. Writes atomically (temp file → mv). git is the safety
# net; no backup is created.
#
# Run from the repo root.
set -euo pipefail

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------
usage() {
    cat <<'EOF'
Usage: scripts/report-finding.sh "<title>" ["<note>"]

Arguments:
  title  Short description of the finding (required)
  note   Optional longer note added as an indented bullet under the item

Options:
  -h, --help  Show this help message and exit

Examples:
  scripts/report-finding.sh "Crash on empty world name"
  scripts/report-finding.sh "Fonts missing in installer" "Reproduced on Win 11 clean install"
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    usage; exit 0
fi

if [[ $# -lt 1 || -z "${1:-}" ]]; then
    echo "ERROR: <title> is required" >&2
    usage >&2
    exit 1
fi

TITLE="$1"
NOTE="${2:-}"

TODO_FILE="todo.md"
ARCHIVE_FILE="todo-archive.md"

if [[ ! -f "$TODO_FILE" ]]; then
    echo "ERROR: $TODO_FILE not found — run from the repo root" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# Compute next HR id: scan both files, take max, add 1
# ---------------------------------------------------------------------------
MAX_ID=0
for f in "$TODO_FILE" "$ARCHIVE_FILE"; do
    if [[ -f "$f" ]]; then
        FOUND=$(grep -oE 'HR-[0-9]+' "$f" | sed 's/HR-//' | sort -n | tail -1 || true)
        if [[ -n "$FOUND" && "$FOUND" -gt "$MAX_ID" ]]; then
            MAX_ID="$FOUND"
        fi
    fi
done
NEW_NUM=$(( MAX_ID + 1 ))
NEW_ID="HR-${NEW_NUM}"

# ---------------------------------------------------------------------------
# Build item line(s)
# ---------------------------------------------------------------------------
ITEM_LINE="- [ ] ${NEW_ID}: ${TITLE}"
SECTION_HEADER="## Playtest / build feedback"

# ---------------------------------------------------------------------------
# Write updated todo.md atomically
# ---------------------------------------------------------------------------
TMPFILE=$(mktemp)
trap 'rm -f "$TMPFILE"' EXIT

if grep -qF "$SECTION_HEADER" "$TODO_FILE"; then
    # Section already exists: append item before the next ## section or at EOF.
    awk \
        -v item="$ITEM_LINE" \
        -v note="$NOTE" \
        -v hdr="$SECTION_HEADER" \
    '
        BEGIN { in_sec=0; done=0 }

        $0 == hdr { in_sec=1; print; next }

        /^## / && in_sec && !done {
            print item
            if (note != "") print "  - " note
            done=1
            in_sec=0
        }

        { print }

        END {
            if (in_sec && !done) {
                print item
                if (note != "") print "  - " note
            }
        }
    ' "$TODO_FILE" > "$TMPFILE"
else
    # Section is absent: append it at END of file so it stays a self-contained
    # group and does not sweep the file's existing loose items (e.g. HR-755)
    # under the new heading. Subsequent findings land in the "exists" branch
    # above, which appends at EOF too — keeping all findings contiguous.
    awk \
        -v item="$ITEM_LINE" \
        -v note="$NOTE" \
        -v hdr="$SECTION_HEADER" \
    '
        { print }

        END {
            print ""
            print hdr
            print ""
            print item
            if (note != "") print "  - " note
        }
    ' "$TODO_FILE" > "$TMPFILE"
fi

mv "$TMPFILE" "$TODO_FILE"
trap - EXIT

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
echo "Added $NEW_ID to $TODO_FILE:"
printf '  %s\n' "$ITEM_LINE"
if [[ -n "$NOTE" ]]; then
    printf '    - %s\n' "$NOTE"
fi
