# Python → Rust Port Plan (HR-417 … HR-737)

> Execution plan for the full rewrite of the Python backend (`src/harsh_realm/`)
> into Rust. Covers the ~320 "port X to Rust and delete the original" TODO items.
> Read this before starting any port batch.

## Strategy (decided)

- **End state:** Full Rust application. The web host (axum or equivalent),
  WebSocket layer, and SQLite persistence are rewritten in Rust; *all* Python
  under `src/harsh_realm/` is deleted. This supersedes the engine-only hybrid in
  `rust-core-migration-plan.md` (that doc remains the reference for the
  intent/IR seam and how pure logic is structured inside `crates/harsh-core`).
- **Per item:** hard delete — port the module to Rust, move its call sites, and
  physically delete the `.py` file (and its now-orphaned Python tests) in the
  same batch. No long-lived PyO3 shims.

## The unavoidable consequence

Rust must be built **bottom-up** (a module can't compile until its dependencies
exist in Rust). Keeping the **Python** test suite green would instead require
**top-down** deletion (only delete what nothing imports). With hard delete and
no PyO3 bridge, these cannot both hold.

**Therefore:** during the migration the Python application is **decommissioned
progressively**. As foundational modules are ported and deleted, Python modules
that still depend on them stop importing. The correctness gate moves to the
**Rust test suite** (`cargo test`), which grows each batch. We delete each
Python module together with its Python tests; the Python suite is expected to
shrink and go red in the partially-migrated middle, and reaches zero at the end.

If green-throughout is required instead, the only options are (a) reintroduce
thin PyO3 shims so remaining Python can call Rust, or (b) build the full Rust app
on a parallel track and cut over once — neither is per-item hard delete.

## Hard rule: lazy imports

This codebase uses **function-local imports** extensively to break cycles
(e.g. `from .time_support import format_time` inside a method). A static
top-level import scan **undercounts** the dependency graph. Before deleting any
module, run a full-text search for its name across `src/` and `tests/`
(`grep -rn "module_name"`), not just an AST import scan, to find every caller.

## Workflow: port first, then tests (two phases)

Per direction (2026-06-22), the migration runs in two phases:

- **Phase 1 — Port everything.** Continuously translate Python modules to Rust
  and delete the originals until **no Python remains and there is no
  cross-referencing** (Rust is self-contained). Keep the Rust crate **compiling**
  each batch (`cargo build`/`cargo check`), but do **not** stop to author
  exhaustive per-file tests — light smoke tests are fine; thorough coverage is
  Phase 2. Python test files for deleted modules are removed as we go.
- **Phase 2 — Tests.** Once the port is complete, ensure equivalent Rust tests
  exist for every module (unit + property mirroring the four-layer rule) and make
  the whole `cargo test` suite pass.

### Per-batch checklist (Phase 1)

1. Pick the next batch (a dependency-closed cluster at the current Rust frontier).
2. Implement the equivalent in Rust under `crates/harsh-core/` (pure logic) or a
   new crate for I/O layers (host/persistence). Keep `harsh-core` I/O-free.
3. Port call sites that are already Rust; record un-ported Python call sites.
4. `cargo build` green for the affected crate(s) (smoke tests optional).
5. `git rm` the Python module(s) **and** their Python test files.
6. Update this plan's checklist; commit per batch.

## Batch order (bottom-up by layer)

Membership is refined at execution time with a real toposort + the lazy-import
search. Layers are listed dependency-first.

- **B1 — Pure value types & math.** `models/grid`, `models/parser`,
  `models/item`, `models/generation`, `models/engine_results`, `engine/dice`,
  `engine/saves`, `engine/damage`. (`crates/harsh-core` already has `dice.rs`,
  `tables.rs`, `oracle.rs`, `components/`, `dsl/` — make these authoritative.)
- **B2 — Core infra.** `exceptions`, `paths`, `config`, `events`,
  `typed_events`, `payloads/*`. (Event bus → Rust event system.)
- **B3 — Engine pure logic.** `modifiers/*`, `dsl/*`, `engine/tables`,
  `engine/oracle`, `engine/skill_checks`, `engine/advancement`,
  `engine/class_progression`, `engine/loot`, `engine/encounters`,
  `engine/weather`, `engine/npc_personality`, `engine/combat/*`.
- **B4 — Content & resources.** `resources/*`, `tags/*`, `traits/*`,
  `status_effects/*`, `triggers/*`, `procedures/*`, `packs/*`, `content/*`,
  `modifiers/service`.
- **B5 — Domain models & repositories.** remaining `models/*`,
  `gm/*_repository`, `db_schema`, `db` (persistence → Rust SQLite layer).
- **B6 — Generators.** `generators/*` (incl. `biome`, `water`).
- **B7 — Scenes & GM controller.** `gm/scenes/*`, `gm/controller` + mixins,
  `gm/narrator`, event handlers, `parser/*`.
- **B8 — Faction & bot.** `faction/*`, `bot/*`.
- **B9 — Web host (outermost).** `api/*`, `api/editor/*`, `api/admin*`,
  `websocket`, `main`, `cli`, `desktop`, `admin/*`. Rewrite as the Rust HTTP +
  WebSocket server; delete last.

## Status

- [x] HR-416 — removed `admin/__main__.py` (the `python -m harsh_realm.admin`
  entry); admin config groups are covered by the Vue `/admin` panel + REST admin
  routes, so no UI gap. Removed the `-m`-based `tests/test_admin_cli.py`.
- [x] B1 (partial):
  - `models/grid` → `crates/harsh-core/src/grid.rs` (HR-637). 18 tests.
  - `engine/dice` (HR-488) — already in `dice.rs`; deleted Python + obsolete
    attr_modifier parity test.
  - `models/parser` → `crates/harsh-core/src/command.rs` (`ParsedCommand`, HR-644).
    4 tests.
  - `cargo test -p harsh-core` = 196 passed.
  - Remaining B1: `models/item`, `models/generation`, `models/engine_results`
    (value-type clusters), then `engine/saves` + `engine/damage` (these depend on
    `models/character`, which is large and ports later — saves.py is currently a
    hybrid calling `harsh_core.resolve_save`).
- [x] B2 (partial) — runtime value-type chain:
  - `models/runtime` → `runtime.rs` (HR-646), `models/engine_runtime` →
    `engine_runtime.rs` (HR-632), `models/engine_results` → `engine_results.rs`
    (HR-631). `cargo test -p harsh-core` = 214 passed, clean build.
  - Next: `payloads/*` (event/transport models), `models/scene_data` →
    `models/generation`; then `engine/saves` + `engine/damage` (need
    `models/character`).
- [x] B3 — model value types (Batches 3–5):
  - Batch 3: `map`, `combat_content`, `narrator_content`, `scene_data`.
  - Batch 4: `shop`, `loot`, `settlement`, `npc`.
  - Batch 5: `creature` (incl. YAML registry via new `serde_yaml` dep),
    `generation`, `oracle` (as `oracle_models.rs`, avoiding the existing oracle
    engine module). `cargo test -p harsh-core` = 229 passed.
  - Remaining models: `character` (large, central), `cell` (`TerrainRegistry`
    loader), `faction` (sqlite Row coupling), `entity_state`, `combat_runtime`,
    `gm_runtime`, `creature`(done), `public_api`, plus `models/api/*`,
    `models/admin/*`, `models/editor_api/*`.
- [ ] B4 … B9 — pending.
