# CLAUDE.md — Outpost 3 AI Assistant Guide

**For:** AI coding assistants (Claude Code, Copilot, Cursor, etc.)
**Project:** Outpost 3 — turn-based grand-strategy colony game, star-system scale
**Design doc:** `docs/DESIGN.md` — authoritative game design (supersedes all prior versions)
**Issue list:** GitHub issues #7–#35 (see `docs/HARNESS.md` for the full table)
**Harness:** `.claude/workflows/implement-issue.js`

---

## You Are a Rust + Vue Game Developer

You write idiomatic Rust targeting edition 2021. You keep simulation logic in a **pure library crate** (`outpost_core`) with zero I/O or framework dependencies. You iterate until **`cargo test --workspace` passes** before considering any task complete.

---

## Technology Stack

| Layer | Technology | Notes |
|---|---|---|
| **Sim core** | Pure Rust lib (`outpost_core`) | No I/O, no async, no framework deps |
| **CLI / Harness** | Rust binary (`outpost_harness`) | Balance calculator + prototyping runner |
| **Web host** | Rust + Axum (`outpost_web`) | Phase 6+; wraps core, serves Vue |
| **Frontend** | Vue 3 + TypeScript + Vite | Phase 6+; Pinia state, strict TS |
| **Content** | YAML / JSON pack files (`content/`) | Loaded at runtime; never hardcoded in kernel |
| **Persistence** | SQLite (snapshot between turns) | NOT per-mutation live state |
| **Testing** | `cargo test` (unit + integration) | All tests must pass before merge |
| **Reference** | `reference/harsh_realm-main/` | Structural patterns only — different game |

---

## Project Structure

```
outpost-3/
├── CLAUDE.md                        # This file
├── docs/
│   ├── DESIGN.md                    # Authoritative game design (read this first)
│   ├── ISSUES.md                    # GitHub issue breakdown
│   ├── HARNESS.md                   # How to use the implement-issue workflow
│   ├── REVIEW.md                    # Codebase review (2026-07-08)
│   └── TODO.md                      # Legacy Godot+C# task list (archived)
├── content/                         # Data pack files (YAML/JSON)
│   └── checks/                      # Balance harness test bundles
├── reference/
│   └── harsh_realm-main/            # Reference architecture (read-only)
├── .claude/
│   └── workflows/
│       └── implement-issue.js       # Main implementation harness
├── Cargo.toml                       # Workspace root (Phase 1+)
├── outpost_core/                    # Pure sim library (Phase 1+)
├── outpost_harness/                 # Balance harness binary (Phase 3+)
├── outpost_web/                     # Axum web host (Phase 6+)
├── frontend/                        # Vue 3 app (Phase 6+)
└── godot/                           # Legacy Godot+C# implementation (behavioral spec)
    └── src/Core/                    # C# code — read as spec, do not modify
```

---

## Critical Rules

### 1. `outpost_core` Has Zero External Dependencies (Except serde + rusqlite)

Files in `outpost_core/src/` must **never** reference:
- `tokio`, `actix-web`, `axum`, or any async runtime
- `std::fs`, `std::io`, or any I/O
- Any HTTP/network crates
- Godot types (the C# layer is archived, not active)

Allowed in core: `serde`, `serde_yaml`, `serde_json`, `rusqlite`, `thiserror`, `uuid`, standard library.

### 2. Content Is Data, Never Code

New buildings, commodities, recipes, events, tech nodes → `content/<pack>/`
Never hardcode authored records in kernel modules.

### 3. Drive Interface Is the Only Mutation Point

`GameEngine::apply(cmd: Command) -> Result<Vec<Event>, EngineError>`

No direct struct mutation from outside `outpost_core`. Tests use `apply()`. Frontend uses `apply()` via the web API.

### 4. SQLite Is Snapshot-Only

Call snapshot after `apply()` pipeline completes. Never write to SQLite *during* a turn. No per-mutation write-through.

### 5. `cargo test` Must Stay Green

Run `cargo test --workspace` before every commit. Never submit work with failing tests.

---

## Running Tests

```bash
# All tests
cargo test --workspace

# With output
cargo test --workspace -- --nocapture

# Single crate
cargo test -p outpost_core

# Lint
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --check --all
```

---

## Using the Harness

The implementation harness automates the full issue → branch → implement → test → PR → merge loop:

```bash
# Implement the next open issue automatically
claude workflow implement-issue

# Target a specific issue
claude workflow implement-issue --args '{"issue_number": 7}'
```

See `docs/HARNESS.md` for the full guide.

---

## What Is Currently Implemented

### Legacy Godot+C# (behavioral spec — do not modify)

The `godot/` directory contains a complete Godot 4 + C# implementation with 163 passing tests.
This is now a **behavioral specification** for the Rust rebuild. Read it to understand:
- What systems to implement and how they should behave
- Edge cases and validation rules (from `tests/OutpostCore.Tests/`)
- Content definitions (from `godot/src/Core/Content/EmbeddedContent.cs`)

Do NOT add new C# code. Do NOT run `dotnet test` as a quality gate (use `cargo test`).

### Rust Rebuild (Phase 1+)

Not started yet. Issue #7 (scaffold) is the first task.

---

## Reference: harsh_realm

`reference/harsh_realm-main/` is a copy of the Harsh Realm project — a Rust + Vue single-player MUD with an expert-system GM. It uses the same architectural patterns we're borrowing:

- Pure Rust core library (`crates/harsh-core/`)
- Axum web host (`crates/harsh-web/`)
- Vue 3 + Pinia frontend (`frontend/`)
- Content packs (`content/`)
- Event bus architecture

**Read it for structural patterns. Do NOT copy game logic** — it is a completely different game domain.

---

## Git Workflow

- Branch from `main` for each issue: `issue-{N}-{slug}`
- Commit message: `Phase N: brief description\n\nCloses #N`
- All checks must pass before merging: `cargo test`, `clippy`, `fmt`
- The harness handles branching, committing, and merging automatically

---

## Open Design Questions (from DESIGN.md §17)

1. Automation approach — AI vs scripts vs DSL (chosen after mechanics exist)
2. Commodity graph specifics — discovered via the harness, not designed on paper
3. Building/structure roster — concrete list per scope
4. Colony flavor-image approach — static vs state-reflecting; placeholder-first
5. Balance numbers — all scalars, to be tuned via the harness
