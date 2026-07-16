# CLAUDE.md — Outpost 3 AI Assistant Guide

**For:** AI coding assistants (Claude Code, Copilot, Cursor, etc.)
**Project:** Outpost 3 — turn-based grand-strategy colony game, star-system scale
**Design doc:** `docs/DESIGN.md` — authoritative game design (supersedes all prior versions)
**Issue tracker:** GitHub issues, milestones M1–M9 (legacy `phase-N-*` labels are all closed; see `docs/HARNESS.md`)
**Harness:** `.claude/workflows/implement-issue.js`

---

## You Are a Rust + Vue Game Developer

You write idiomatic, best-practices Rust targeting edition 2021. You keep simulation logic in a **pure library crate** (`outpost_core`) with zero I/O or framework dependencies. A feature is **not complete** until every applicable test tier passes — unit, component, and end-to-end — plus an automated code review and an independent judge pass (see [Definition of Done](#definition-of-done) and [Automated Review & Validation](#automated-review--validation) below). "Compiles" and "looks right" are not done; "tests pass" is done.

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
| **Testing (backend)** | `cargo test` (unit + integration) | All tests must pass before merge |
| **Testing (frontend)** | `vitest` (unit/component) + `@playwright/test` (e2e) | All tiers must pass before merge |
| **Review** | Haiku or Sonnet code-review subagent | Runs before every PR (see below) |
| **Validation** | Haiku judge subagent | Independently confirms acceptance criteria before merge |
| **Reference** | `reference/harsh_realm-main/` | Structural patterns only — different game |

---

## Rust Best Practices

Applies to every file under `outpost_core/`, `outpost_harness/`, `outpost_web/`, and `outpost_tauri/`.

- **Idiomatic over clever.** Prefer iterator chains over manual index loops where they read cleanly; prefer `match`/`if let` over nested `unwrap()`. Follow the standard library's naming conventions (`snake_case` fns/vars, `CamelCase` types, `SCREAMING_SNAKE_CASE` consts).
- **`clippy -D warnings` is the floor, not the ceiling.** The workspace must build with zero clippy warnings (`cargo clippy --workspace -- -D warnings`). When touching a file with pre-existing warnings outside your change, don't fix unrelated ones speculatively — but never introduce new ones.
- **No panics in library code paths.** `outpost_core`/`outpost_web`/`outpost_tauri` library code must not `unwrap()`/`expect()`/`panic!()` on data that can plausibly be invalid at runtime (user input, content-pack data, network payloads). Return `Result<_, EngineError>` (or the relevant error enum) instead. `unwrap()`/`expect()` are fine in `#[test]` functions and for invariants that are truly impossible to violate (document why with a comment when non-obvious).
- **Errors are typed, not stringly.** Use `thiserror` for error enums (see `EngineError`, `SnapshotError`, `CmdError`) rather than `String`/`anyhow`-style catch-alls in library code. `anyhow` is acceptable in `outpost_harness` (a CLI binary) but not in `outpost_core`.
- **Every public item gets a one-line doc comment** (`///`). Add a longer explanation only when the *why* is non-obvious (a subtle invariant, a workaround, a cross-reference to a design doc section) — don't restate the signature in prose.
- **Minimize cloning.** Prefer borrowing (`&T`) over `.clone()` in hot paths (the per-turn production/needs/hazard pipelines). Cloning is fine at command/query boundaries where ownership must transfer into a returned wire type.
- **No new external dependencies in `outpost_core`** beyond what's already listed in [Critical Rules](#critical-rules) without discussing it with the user first — the zero-I/O guarantee is load-bearing for testability and the future WASM/embedded targets.
- **Keep `outpost_tauri`/`outpost_web` thin.** Wire-layer crates translate `Command`/`Query`/`Event` to/from JSON and call into `outpost_core`; they should not contain simulation logic. If you find yourself writing game rules in a `#[tauri::command]` function, that logic belongs in `outpost_core` instead.

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

### 5. All Tests Must Stay Green

Run the full gate for whatever you touched before every commit. Never submit work with failing tests, and never skip a tier because it's inconvenient — see [Definition of Done](#definition-of-done).

### 6. Every PR Gets an Automated Code Review and Judge Pass

Before a PR is opened, run a code-review subagent over the diff and a judge subagent against the issue's acceptance criteria. See [Automated Review & Validation](#automated-review--validation).

### 7. Merge Only When the Gate Is Green

A PR may be merged (by the harness or by you, working an issue directly) only once tests + review + judge all pass. See [Git Workflow](#git-workflow) for the exact gate and the manual-hold exceptions.

---

## Definition of Done

A feature, bugfix, or refactor is complete only when **all applicable tiers below pass** — not when the code compiles, not when it "looks right" in a manual check. Skip a tier only when it is genuinely inapplicable (e.g. a pure-backend change with no frontend surface skips Playwright), and say so explicitly in the PR description rather than silently omitting it.

| Tier | Command | Applies to |
|---|---|---|
| Rust unit + integration | `cargo test --workspace` | Any change under `outpost_core/`, `outpost_harness/`, `outpost_web/`, `outpost_tauri/` |
| Rust lint | `cargo clippy --workspace -- -D warnings` | Same as above — zero warnings, not just zero errors |
| Rust format | `cargo fmt --check --all` | Same as above |
| Frontend unit/component | `npm run test:unit` (in `frontend/`) | Any change under `frontend/src/` |
| Frontend type-check | `npm run type-check` (in `frontend/`) | Same as above |
| Frontend build | `npm run build` (in `frontend/`) | Same as above |
| End-to-end | `npm run test:e2e` (in `frontend/`) | Any change that affects a user-facing flow (new screen, new command wiring, changed navigation) — add or update a Playwright spec under `frontend/e2e/` covering the new behavior, don't just rely on existing specs still passing |
| Balance harness | `cargo run --bin outpost_harness -- check content/checks/` | Any change to `content/` packs with a matching `content/checks/<name>/` bundle |

`outpost_tauri` cannot be built or type-checked in every environment (it needs WebKit2GTK system libs) — when that's the case, say so explicitly rather than silently skipping verification, and rely on `cargo build -p outpost_tauri`/`cargo check -p outpost_tauri` wherever it *is* available (e.g. CI, or a local dev machine) as part of the gate.

---

## Automated Review & Validation

Before opening a PR (or before merging, for the automated harness), run two independent subagents against the diff:

1. **Code-review agent** (Haiku or Sonnet — Haiku for small/mechanical diffs, Sonnet for anything touching simulation logic or cross-cutting concerns). Reviews the diff for correctness bugs, unhandled edge cases, and violations of the [Rust Best Practices](#rust-best-practices) / [Critical Rules](#critical-rules) above. Findings that are blocking (real bugs, panics on reachable input, violated architecture rules) must be fixed before shipping; non-blocking suggestions (style nits, optional simplifications) can be noted and skipped with a one-line reason.
2. **Judge agent** (Haiku). Given the original issue's acceptance criteria (its "Done when" / "Definition of Done" bullets) and the diff, independently confirms each criterion is actually met — not just that tests pass, but that the tests *test the right thing*. The judge should be blind to the review agent's findings so it isn't anchored by them; run it as a separate subagent call, not chained after the reviewer.

Both agents' verdicts (pass/fail + findings) go in the PR description under a `## Review` section so a human skimming the PR later can see what was checked automatically. If either agent finds a blocking issue, fix it, re-run that agent (not the whole pair — no need to re-review passing dimensions), and only proceed once both are clean.

---

## Running Tests

```bash
# Rust — all tests
cargo test --workspace

# Rust — with output
cargo test --workspace -- --nocapture

# Rust — single crate
cargo test -p outpost_core

# Rust — lint
cargo clippy --workspace -- -D warnings

# Rust — format check
cargo fmt --check --all
```

```bash
# Frontend — from frontend/
npm run type-check   # vue-tsc
npm run test:unit    # vitest (unit + component)
npm run build        # production build (also type-checks)
npm run test:e2e     # Playwright, headless
npm run test:e2e:ui  # Playwright, interactive UI mode (local debugging only)
```

Playwright uses the environment's pre-installed Chromium — never run `playwright install`; `frontend/playwright.config.ts` resolves the sandbox's browser automatically when present and falls back to Playwright's normal resolution otherwise. `e2e/app-shell.spec.ts` is a browser-only smoke suite (no `outpost_web` backend required); specs that exercise live game state must start `outpost_web` first.

---

## Using the Harness

The implementation harness automates the full issue → branch → implement → test → review → judge → PR → merge loop:

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

### Rust Rebuild

Well underway — the Rust workspace, `outpost_core` simulation kernel, `outpost_tauri` desktop shell, `outpost_web` secondary host, and a Vue 3 frontend all exist and are actively developed. Milestones M1–M9 track further work; treat GitHub's open issues/milestones as the source of truth for exact status rather than this file, which will drift. `outpost_tauri` is the primary UI host; `outpost_web` lags it in feature parity and is kept mainly for browser-mode development/testing.

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
- Open a PR against `main` once the [Definition of Done](#definition-of-done) gate is green and the [review + judge](#automated-review--validation) pass.
- **Auto-merge policy**: merge the PR yourself (no need to wait for a human) once, and only once, **all** of the following hold:
  1. Every applicable [Definition of Done](#definition-of-done) test tier is green.
  2. The code-review agent found no blocking issues (or blocking issues were found and fixed, then re-reviewed clean).
  3. The judge agent confirmed the issue's acceptance criteria are met.
  This applies both to the automated harness and to working an issue directly in conversation — don't stop at "PR opened" and wait to be told to merge; merge it once the gate is green, then move on or report completion.
- **Do not auto-merge** when: the user explicitly asked to review before merging; the change is to `CLAUDE.md`/`docs/HARNESS.md`/the workflow scripts themselves (policy changes get a human look before they take effect); the change touches `outpost_tauri` in a way that can't be verified in the current environment (missing WebKit2GTK libs) and no CI run has confirmed it; or any review/judge finding is ambiguous rather than clearly resolved. In these cases, open the PR, summarize review + judge results, and wait.
- Never force-push, skip hooks, or bypass a failing check to get to green.

---

## Open Design Questions (from DESIGN.md §17)

1. Automation approach — AI vs scripts vs DSL (chosen after mechanics exist)
2. Commodity graph specifics — discovered via the harness, not designed on paper
3. Building/structure roster — concrete list per scope
4. Colony flavor-image approach — static vs state-reflecting; placeholder-first
5. Balance numbers — all scalars, to be tuned via the harness
