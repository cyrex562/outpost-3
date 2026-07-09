# Python → Rust Port — Session Handoff

> Living handoff doc for the incremental Python→Rust migration of the Harsh Realm
> backend. Read this first when resuming the port. Update the "Current state" and
> "Already ported" sections as batches land.

## Strategy (binding decisions from earlier sessions)

- **Full Rust rewrite**: port every Python module to Rust (crate at
  `crates/harsh-core/`), then delete the `.py`.
- **Port + hard-delete per item**: each batch ports a module to Rust *and*
  `git rm`s the original `.py` plus its directly-corresponding Python tests in the
  same commit.
- **Rust tests are the correctness gate**:
  `cargo test --manifest-path crates/harsh-core/Cargo.toml` must stay green
  (zero warnings). The Python suite decommissions progressively as modules are
  deleted — that is expected; do **not** try to keep the Python suite green.
- **Phase 1 (current)**: port everything, keeping the crate compiling. **Phase 2
  (later)**: ensure equivalent Rust tests exist for everything.

## Current state

- PR #26 was merged into `main`; all porting work through **Batch 56** is on
  `main`. Start each new session by branching fresh off the latest `main`
  (do **not** push to `main` directly).
- Last green checkpoint: **443 Rust tests passing, zero warnings.**
- **GitHub Actions / CI is currently broken and needs to be redone eventually.**
  Flag it, but it does not block porting — local `cargo test` is the gate.

## Already ported (do NOT re-port — grep the crate if unsure)

- **Foundations**: entity repository, `status_effects/*`, domain event dispatcher,
  `procedures/*` (schema, compute_registry, runner), packs core (manifest, loader,
  discovery, registry, content_service).
- **Entire `engine/` layer**: tables (TableEngine), skill_checks, advancement,
  weather, threads, enemy_ai, low_health_narration, adventure_crafter, encounters,
  oracle, discovery, `combat/*` (awareness, creation, resolvers, flee, narration).
- **Entire `generators/` directory**: biome, water, content_tables, world_support,
  dungeon_gen, npc_gen, square_gen, settlement_gen, world_gen + mixins.
- **`parser/*`** (CommandParser + alias tables).
- **GM leaf layer** in `crates/harsh-core/src/gm/`: `scenes/base` (`SceneState`
  enum + `SceneHandler` trait), `scenes/{time,exploration,social,town}_support`,
  and `gm/narrator.rs` (`Narrator`).

## Remaining to port (the coupled core — roughly dependency order)

1. **GM scene handlers** — start with the smallest self-contained ones
   (`respawn`, `level_up`) to establish the "one struct implementing the
   `SceneHandler` trait" pattern, then `exploration`, `social`, `shopping`,
   `dungeon`, `town`, `character_creation`, `combat`. In Python each scene is a
   pile of `_*_mixin.py` files composed via multiple-inheritance MRO; in Rust
   **collapse each scene into a single struct** — do not port the
   aggregator/mixin `.py` files 1:1.
2. **GM controller** — `gm/controller.py` + `_controller_*_mixin.py` +
   `*_event_handlers.py` + `gm_factory.py` / `controller_support.py`. Orchestrates
   scenes and the event bus.
3. **`api/` + websocket** — per the full-rewrite decision this becomes an **axum**
   web host.
4. **Smaller subsystems still in Python**: `bot/`, `triggers/`, `plugins/`,
   `modifiers/`, `faction/` turn logic (faction_turn, faction_ai, reputation,
   turn_support), `admin/`, `dsl/`, `traits/`, `resources/`, plus
   `main`/`cli`/`config`/`desktop`.

`todo.md` tracks every item by HR-number with `[x]`/`[ ]` and per-item notes.
A linter reformats `todo.md` after edits — that's intentional.

## Per-batch workflow (follow exactly)

1. Read the Python module(s) and confirm their Rust dependencies already exist
   (grep the crate). Pydantic models → serde structs; collapse Python mixins into
   one Rust struct. Match surrounding Rust idioms: `JsonObject` =
   `serde_json::Map`, `JsonValue` = `serde_json::Value` (both aliased in
   `crate::runtime`).
2. Write the Rust module(s) **with unit tests**; register in the parent
   `mod.rs` / `lib.rs`.
3. `cargo test --manifest-path crates/harsh-core/Cargo.toml` — green, zero
   warnings (fix any).
4. `git rm` the ported `.py` files **and** their directly-corresponding Python
   tests (leave broad integration/scene tests that still cover not-yet-ported
   code).
5. Mark the HR item(s) `[x]` in `todo.md` with a short note + "(Port Batch N)".
6. Commit (imperative subject + body) and push with retry to the dev branch.
   The commit message MUST end with these two trailers:

   ```
   Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
   Claude-Session: <session url>
   ```

   Do **not** put the model identifier anywhere in commits/code/PR — chat only.
7. Pushing updates the PR; only open a new PR if the user asks.

## Suggested next step

Branch off `main`, confirm `cargo test` is green, then port the `respawn` scene
as the first full `SceneHandler` implementation to establish the pattern, then
proceed batch by batch.
