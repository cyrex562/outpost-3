# CLAUDE.md — Harsh Realm Project Context

> This file provides project-level context for coding agents working on Harsh Realm.
> Read this file first before starting any task. Read AGENTS.md for coding standards.
> Read docs/superpowers/specs/2026-04-22-rules-architecture-design.md for the
> current rules-based architecture reference.

> **⚠️ Rust-only as of 2026-06-26.** The Python/FastAPI backend was fully ported to
> Rust and **removed**. The backend is now `crates/harsh-core` (pure-Rust engine) +
> `crates/harsh-web` (Axum HTTP/WebSocket host that serves the Vue frontend); the
> Tauri shell runs `harsh-web` in-process. There is no Python, `uvicorn`,
> `aiosqlite`, `pyproject.toml`, or `src/harsh_realm`. Backend code follows standard
> Rust conventions (not the Python/Pydantic rules below). Sections describing
> Python/FastAPI/`aiosqlite` are **historical** — they document the pre-migration
> design; the Rust port preserves the same architecture (GM controller, event bus,
> packs, oracle, factions, repositories) in `crates/harsh-core`. Tests are
> `cargo test` + Playwright (no pytest). The four-layer testing rule still applies
> in spirit (unit + property + e2e), realized with Rust + Playwright.

## What Is Harsh Realm

A single-player MUD with a procedural world and an expert-system GM. The player explores a persistent grid-based world through text commands. All maps (world, towns, dungeons, building interiors) use square grids with 8-way movement. The software handles all mechanical resolution (dice rolls, combat, skill checks), world generation, NPC behavior, faction simulation, and narrative output. An AI GM controller orchestrates the game flow.

**Setting:** A dark, hostile feudal planet cut off from interstellar civilization (SWN "lost colony"). TL3 baseline with scattered TL4+ pretech relics. Tone: Blade Runner meets Alien meets Dune. Feudal lords hoard pretech to oppress people. Ruins are abundant and dangerous. Monsters are real (natural + engineered). The long-term goal is finding/building a starship to escape.

**Rules system:** XWN (the shared core of Stars/Worlds Without Number). 2d6 skill checks, d20 attack rolls, class/level advancement (Warrior, Expert, Adventurer). WWN/SWN faction turn system running weekly in the background.

## Architecture Summary

```
Vue 3 Frontend (chat log + input + sidebar + grid map + window manager)
        ↕ WebSocket + REST
Rust Axum Backend (crates/harsh-web — serves the frontend + REST/WS; wraps crates/harsh-core)
  ├── GM Controller (state machine: Exploration, Combat, Social, Dungeon, Rest, Shopping, etc.)
  ├── Event Bus (all state changes are events; logged, cascadable, forwarded to frontend)
  ├── Pack Registry + Content Service (modular rules/content, per-world overrides)
  ├── Rules Engine (XWN mechanics with extension points for house rules)
  ├── Oracle System (Mythic GME — fate chart, scene checks, chaos factor, Adventure Crafter)
  ├── Table Engine + Procedure Runner (weighted rolls and pack-authored multi-step generators)
  ├── Faction System (WWN/SWN native faction turns, weekly tick)
  ├── Admin Service (config CRUD — skill mappings, difficulty targets, etc.)
  ├── Generators (world, settlement, dungeon, NPC, encounter, loot)
  └── World State (SQLite — one .db file per world)
```

### Packs and Modular Rules

Harsh Realm uses a four-layer rules architecture:

- **Kernel:** engine/framework code in `src/harsh_realm/`, kept free of authored game records.
- **Frameworks:** reusable mechanics such as checks, combat, tables, oracle, factions, and content services.
- **Packs:** directory-based modules under `content/<pack-id>/` with `pack.yaml`, `content/`, optional `code/`, and optional migrations.
- **Worlds:** SQLite files that bind an ordered pack list in `world_packs` and store per-world edits in `pack_overrides`.

The built-in XWN rules/content live in `content/xwn-core/`. Runtime content reads go through pack-aware paths, `PackRegistry`, `WorldPackRepository`, or `ContentService`; new game records should be added to packs rather than hardcoded into engine modules.

Procedures are the canonical framework for multi-step generators. Author procedure YAML under `content/<pack-id>/content/procedures/`; use code-bearing pack hooks only for narrow compute functions that procedure steps invoke by qualified name.

### Key Architectural Rules

1. **The GM Controller orchestrates but does not implement game logic.** It knows what scene state we're in and what commands are valid. When the player acts, the GM emits events. Subsystems (combat, navigation, social, etc.) handle the events, update world state, and emit result events. The GM reads results to decide on scene transitions and narration.

2. **All state changes flow through the EventBus.** No direct mutation of world state except through event handlers. Events are logged to the `event_log` table. Events are forwarded to frontend via WebSocket.

3. **SQLite IS the live world state.** There is no separate in-memory world model that gets serialized. The database is always current. Manual saves create named snapshots (file copies). Periodic checkpoints for crash recovery. The `cells` table stores all grid cells.

4. **Extension points for house rules.** Core resolvers (skill checks, attacks, damage, initiative, etc.) have a default XWN implementation and an overridable method for house rules. House rule specs live in `docs/house_rules/`. Implementations live in `src/harsh_realm/house_rules/`.

5. **Data ownership, subsystems, resolver pipelines, and persistence are governed by AGENTS.md.** The canonical rules are in `AGENTS.md` under "Data Ownership, Subsystems, and Events." Entity classes hold intrinsic state only; subsystem state is owned by subsystem modules; multi-contributor resolutions use resolver pipelines; durable subsystem state owns SQLite tables.

6. **Game content lives in packs.** Random tables, generators, skill definitions, equipment, creatures, terrain, and narration templates are authored under `content/xwn-core/content/` or another pack's `content/` directory. Engine/editor schema metadata may remain outside packs.

7. **YAML is seed, SQLite is source of truth.** Editable config tables (skill_mappings, difficulty_targets, disposition_outcomes, encounter_weights, faction_asset_stats) are seeded from pack YAML at world creation. From that point, SQLite owns the data for that world. Reset operations re-read pack defaults and overwrite the row.

### Grid System

The game uses a single grid topology — `SquareGrid` (8-way movement, Chebyshev distance) for all maps (world, towns, dungeons, interiors). The `Grid` protocol lives in `models/grid.py`; `GridCoord(q, r)` is the coordinate type. `SquareGrid` is injected into consumers (GM controller, scenes, generators, combat) via constructor.

### Testing Requirements

> **⚠️ Current policy (2026-07-03) — read this first; the four-layer table below is
> historical (Python era).** Favour **unit, integration, and API tests** (Rust
> `cargo test` on both crates). **Mutation testing is not required.** **Playwright runs
> only for changes to UI behaviour**; routine UI verification is a **human checklist**,
> not an automated gate. Every bug fix ships a regression test that fails without the fix.
> The gate is `cargo xtask test` (`--ui` adds Playwright); `scripts/dev-test.sh` is the
> full gate. Multi-item backlogs run as a **fix → review → test → PR** loop with the
> human gating each merge. See **AGENTS.md → "Testing"** and **"Automated fix loop"** for
> the authoritative rules.

The four-layer description below (unit + property + mutation + Playwright, pytest/mutmut)
is **historical** and retained only for context.

Every feature must be verified by all four test layers before it is considered complete. A task is NOT done until all four layers pass. This applies to every task in every milestone.

**Feature complete = unit tests + property tests + mutation tests + Playwright tests all passing.**

| Layer | Tool | What it catches |
|---|---|---|
| Unit tests | pytest | Specific behaviors, edge cases, error paths |
| Property-based tests | Hypothesis | Invariants that hold across all valid inputs |
| Mutation tests | mutmut | Missing assertions, weak tests that don't catch bugs |
| Playwright E2E tests | Playwright | UI behavior, WebSocket updates, cross-component flows |

**When to write each type:**

- **Unit tests** for every new function, class method, or behavior. One test file per source module. Minimum one test per public function. Use `@pytest.mark.parametrize` for any logic with multiple input/output cases.
- **Property-based tests** whenever a function has: numeric inputs with invariant relationships (dice results always in range, modifiers always clamp, costs always positive), round-trip semantics (serialize/deserialize returns original), monotone properties (higher stat never produces lower modifier), or boundary conditions (values always stay within defined ranges regardless of input). Place all in `tests/test_properties.py`, organized by subsystem.
- **Mutation tests** after writing unit and property tests for any module in: `engine/`, `gm/scenes/`, `faction/`, `admin/service.py`, or any module with complex conditional logic. Target: >= 85% of mutants killed per module. Surviving mutants must be either killed (add a test) or documented as known equivalent mutants with an inline comment.
- **Playwright tests** for every new Vue component, admin tab, or UI flow. Place in `frontend/tests/e2e/`. Minimum per component: renders without errors, primary interaction works, WebSocket-driven updates reflect in UI, error states handled visibly.

**Forbidden shortcuts:**
- Do NOT mark a task complete without all four test layers.
- Do NOT write tests that only assert `is not None` or `== True` -- assert the actual value.
- Do NOT mock the thing you're testing. Mock its dependencies.
- Do NOT skip property tests because "it's obviously correct."
- Do NOT skip Playwright tests for UI components.
- If a module has surviving mutants, do NOT ignore them -- kill them or document them.

### Data Models — Pydantic Required

All new data models must use Pydantic `BaseModel`, not stdlib `@dataclass`. See AGENTS.md for full rules. Existing `@dataclass` usage in the codebase is legacy — new code must use Pydantic, and existing models should be migrated when touched. Frozen models use `model_config = ConfigDict(frozen=True)`.

**Test execution order per task:**
1. Write implementation code
2. Write unit tests -> pytest passes
3. Write property tests (if module qualifies) -> hypothesis passes
4. Run mutation tests -> >= 85% killed or document survivors
5. Write Playwright tests (if UI component created) -> playwright passes
6. Update test count in commit message
7. Task is complete

## Coding Standards

Full reference: `AGENTS.md`. The rules below are the standards Claude must follow in every edit.

> **Backend is Rust.** For `crates/harsh-core` + `crates/harsh-web`, follow
> **"Rust Backend Conventions"** in `AGENTS.md` (serde models, `Result<T, String>`
> errors, repositories over raw SQL, the session actor, `cargo test`). The
> **Python/Pydantic/pytest rules below are historical** (removed Python backend);
> they survive only for the architectural rules that carried over. The
> **Frontend (Vue 3 / TS)** rules below are current.

### Python Style

- Python 3.12+. Black formatting, 88-char line length. Double quotes for all strings.
- Imports ordered stdlib → third-party → local, blank-line separated. Every file starts with `from __future__ import annotations`.
- Type hints required on all function signatures and class attributes. Modern syntax only: `list[str]`, `str | None` — never `List[...]`, `Optional[...]`.
- Google-style docstrings on all public functions, classes, and modules.

### Data Models — Pydantic Details

- Every class holding structured data is a Pydantic `BaseModel` — internal game models, API schemas, config records, result objects, JSON column shapes, everything.
- Never use `stdlib @dataclass`, `TypedDict`, `NamedTuple`, or plain `dict` for structured data crossing function boundaries. These are forbidden even for "quick" or "internal" cases.
- Frozen/value objects: `model_config = ConfigDict(frozen=True)` — hashable, safe as dict key/set member.
- Use `Field(default=..., description="...")` with validation constraints (`ge=`, `le=`, etc.). Use `field_validator` / `model_validator` for in-model business logic.
- SQLite JSON columns: read with `ModelClass.model_validate(json.loads(row["data"]))`, write with `model.model_dump_json()`.
- FastAPI request/response bodies use the same Pydantic models — no separate "API schema" tier.

### No `object` / `Any` Escape Hatches

Do NOT use `object`, `dict[str, object]`, `list[object]`, or `Any` as convenience types. Priority of alternatives, in order:

1. Pydantic model for structured data
2. `JsonValue` / `JsonObject` alias for JSON-shaped data
3. `Protocol` for services, framework state, injected collaborators
4. Concrete union for temporary migration boundaries
5. `TypeVar` / generic parameter for reusable helpers

Narrow documented exceptions only: Pydantic validator input hooks, third-party boundaries without upstream types, very local generic internals that don't leak into application-facing APIs.

### Interfaces & Extension Points

- Use `Protocol` (from `typing`) for interfaces. Do NOT use `ABC` or `abstractmethod`.
- Extension points (resolvers) are concrete classes with a working default implementation plus overridable methods. House rules override specific methods — they do NOT inherit abstract stubs.

### Event Bus Rules

- All gameplay state changes flow through the EventBus or the in-process domain-event layer. Scene handlers emit command/request events; persistence handlers perform the write and emit result events.
- Editor/admin CRUD is NOT a gameplay event. Emit selective `editor.live_update` or `admin.config_updated` only when the mutation affects the currently loaded world or runtime-relevant config.
- Event types use dotted namespace strings: `combat.attack`, `exploration.enter_hex`, `gm.scene_change`.
- Event data payloads are flat, descriptive, JSON-serializable dicts. The envelope (`GameEvent`) is a frozen Pydantic model. Typed event wrappers (`TypedGameEvent`) layer typed payloads on top.
- Event handlers never propagate exceptions to the bus — catch, log, optionally emit an error event.

### Database Access

- `aiosqlite` only. All DB access goes through `WorldDatabase`.
- Gameplay scenes, controllers, GM routes, and engine mutation paths do NOT issue raw SQL directly — go through repository/adapter modules. Narrow allowed exceptions: the repositories themselves, editor/admin maintenance routes, import/export and bootstrap/schema plumbing.
- Always use parameterized queries. Never f-string or format SQL.
- Group related writes in a single transaction.

### Error Handling & Logging

- No silent failures. Every error path either raises, returns an explicit error value, or logs a warning.
- Built-in exceptions for programming errors (`ValueError`, `TypeError`, `KeyError`). Custom exceptions in `exceptions.py` inherit from `HarshRealmError`.
- FastAPI endpoints return structured error bodies: `{"error": "ErrorType", "message": "..."}`.
- Use Python `logging` — never `print()`. Per-module logger: `logger = logging.getLogger(__name__)`. Include context (entity ID, event type) in every message.

### File & Module Organization

- One concept per file. Keep files under 400 lines; split when a module grows beyond that.
- `__init__.py` files hold only re-exports.
- No circular imports — extract shared types into a third module.

### YAML Content Files

- Every file has top-level `id`, `category`, and `name`.
- Use `tags: [...]` arrays for contextual filtering.
- Subtable references use `{ table: "table_id" }` syntax.
- Stub tables for future content include 3–5 placeholder entries and a TODO comment.

### Frontend (Vue 3 + TypeScript)

- `tsconfig.json` has `"strict": true` always, plus `noUnusedLocals`, `noUnusedParameters`, `noImplicitReturns`, `noFallthroughCasesInSwitch`, `exactOptionalPropertyTypes`. Never disable a flag to silence an error — fix the code.
- Run `npx tsc --noEmit` on every task. Zero errors required before completion.
- No `any`. If a type is genuinely unknown, use `unknown` and narrow it explicitly.
- No non-null assertions (`!`) without an inline comment explaining the guarantee.
- No `as X` type assertions except when narrowing from `unknown` after a runtime check.
- No `@ts-ignore` / `@ts-expect-error` except for known upstream library bugs — link the issue.
- All API response shapes declared as interfaces in `frontend/src/types/api.ts`. Never inline `any` or `Record<string, unknown>`.
- Single-file components with `<script setup lang="ts">`. Tailwind for styling; no custom CSS unless Tailwind can't express it.
- Pinia for cross-component state. Composables (`use*.ts`) own reactive logic shared across components. One `useWebSocket` composable owns the connection.
- Typed `defineProps`/`defineEmits` with generic syntax. Pinia state/getters/actions explicitly typed. Composables declare explicit return types.
- Components under 400 lines. Every interactive element used in Playwright tests has a descriptive `data-testid`.
- No `localStorage` / `sessionStorage` in components (not supported in all deployment contexts).

### Frontend Testing

Mirrors the backend four-layer rule:

| Layer | Tool | Target |
|---|---|---|
| Unit | Vitest + Vue Test Utils | ≥80% coverage on stores/composables/utils |
| Property | fast-check | Invariants (clamping, round-trips, monotonicity) |
| Mutation | Stryker | ≥85% killed on stores + composables |
| E2E | Playwright | Every new component, admin tab, UI flow |

Frontend task checklist: `tsc --noEmit` 0 errors; unit coverage ≥80%; property tests pass; Stryker ≥85%; Playwright passes; no `any`, no unexplained `!`, no `@ts-ignore`.

### Git Conventions

- Branch names: `feat/<description>`, `fix/<description>`, `refactor/<description>`.
- Commit messages: imperative mood, concise — "Add skill check resolver with extension point" not "Added some stuff".
- Never push directly to `main`. Milestone completions are tagged: `milestone-0`, `milestone-1`, etc.

### What NOT to Do

- No `ABC` / `abstractmethod`.
- No stdlib `@dataclass` for data models.
- No `TypedDict` or `NamedTuple` for structured data.
- No raw SQL outside `db.py` / repository modules.
- No mutation of shared state outside event handlers and DB transactions.
- No extrinsic subsystem state stored on entity models or in entity JSON `data` columns.
- No cross-subsystem writes through direct service mutation; emit request events and let the owning subsystem write.
- No pre-commit event chains for multi-contributor resolutions; use ordered resolver pipelines.
- No `print()` for debugging — use `logging`.
- No hardcoded magic numbers — use named constants or config values.
- No files longer than 400 lines. Break up any file that grows beyond 400 lines (extract mixins/helpers/child components or split into a package).
- No `any` in TypeScript; no non-null `!` without comment; no `@ts-ignore`.
- No Vitest tests that only assert `.toBeTruthy()` / `.toBeDefined()` — assert the actual value.
- No skipping property tests for composables with numeric/clamping behavior.
- No merging frontend code without `tsc --noEmit`.
- No dependencies added without justifying necessity — prefer stdlib.

## Tech Stack

- **Backend:** Rust (edition 2021) — `crates/harsh-core` (engine) + `crates/harsh-web` (Axum + tokio + tower-http; HTTP/WS host)
- **Desktop:** Tauri 2 (`src-tauri`), runs `harsh-web` in-process
- **Frontend:** Vue 3, TypeScript, Vite, Tailwind CSS
- **Database:** SQLite via `rusqlite` (bundled), one `.db` file per world
- **Testing:** `cargo test` (Rust unit + property + IR schema-drift gate); Playwright (E2E vs the Rust host)
- **Type checking:** Rust compiler/clippy (backend), vue-tsc --strict (frontend)
- **Data:** YAML (authored content), JSON (SQLite storage)

## Current State

**1139 tests passing, 12 skipped.** Updated 2026-04-26 after the rules-based
architecture refactor removed the ECS substrate and stabilized the route and
websocket test suites.

**Dev environment note:** The venv requires these packages for the full test suite: `hypothesis`, `httpx`, `pytest-asyncio`, plus the package itself installed in editable mode (`pip install -e .`).

### Milestone Status Summary

| Milestone | Status | Tests | Date |
|-----------|--------|-------|------|
| M0 | Complete | 40 | 2026-03-13 |
| M1 | Complete | 187 | 2026-03-13 |
| M2 | Complete | 270+ | 2026-03-14 |
| M3 | Complete | 657 | 2026-03-27 |
| M4 | Complete | — | 2026-03-28 |
| M4.5 | Complete (3 deferred) | 701 | 2026-03-28 |
| M4.6 | **Complete** | 843 | 2026-04-03 |
| M4.7 | **Complete** | 857 | 2026-04-05 |
| M4.8 | **Complete** | — | 2026-04-03 |
| M4.9 | **Complete** (2 gaps) | 1139 | 2026-05-16 |
| Rules-arch | **Complete** | 1139 | 2026-04-26 |
| M5 | **Complete** (Extensions too)
| PROG | **Complete** (PROG-01, PROG-02)
| INV | **Complete** (INV-01 to INV-05)
| TWN | **Complete** (TWN-01 to TWN-03)
| ADM | **Complete** (ADM-01 to ADM-09) | 1139 | 2026-05-16 |

### Milestone 4.6 complete (2026-04-03).

Combat completion: item registry with canonical IDs (`data/items/` — 6 YAML files), equipped weapon resolution, shock damage, range bands, ammo tracking, 4-type saving throws (physical/evasion/mental/luck), structured combat log formatting. All features have unit tests; shock, saves, and item registry have Hypothesis property tests.

### Milestone 4.7 complete (2026-04-05).

Settlement-size shop tiers (via building-type YAML files in `data/shops/`), `look` lists NPCs at settlements, shop rejection outside settlement, `InventoryPanel.vue` collapsible PC inventory panel, NPC persistence verification tests.

**Deviations:** Shop tier files organised by building type (blacksmith.yaml, general_store.yaml, healer.yaml, tavern.yaml) with small/medium/large tier variants, rather than by settlement size (hamlet.yaml, village.yaml, town.yaml) as originally specced. No `shop_tiers` SQLite table — YAML-based approach used instead.

### Milestone 4.8 complete (2026-04-03).

Bot framework: `src/harsh_realm/bot/` package with BotRunner, A* pathfinder, goal/assertion system, structured JSON logger. World map API endpoint (`GET /api/world/map`). 6 bot goal tests in `tests/bot/test_first_suite.py` (skipped without `--run-bot`).

### Milestone 4.9 partial.

**Complete:** StatusSidebar displays gold/scene badge/chaos factor. ChatLog formats social events (disposition changes, skill checks, expert reroll). ChatLog formats shopping events (purchases, sales with balance). `look` lists NPCs at settlements. `docs/acceptance_criteria.md` exists and covers M0–M4.9.

**Gaps:**
- **mutmut coverage:** Not run on M4 modules. No kill rates documented.
- **Playwright E2E admin panel:** No Playwright tests for admin tabs.

### Known PLACEHOLDERs (non-blocking)

- `data/classes.yaml`: attack bonus per level, skill points start, saving throw values — reasonable approximations, not verified against source books.
- `data/equipment_kits.yaml` line 2: kit contents functional but minimal.
- `docs/rules_reference/combat.md` lines 32, 48: attack bonus progression and enemy morale checks.
- `docs/rules_reference/weapons_armor.md` line 3: verify all values against source books.
- `src/harsh_realm/models/character.py` lines 27-29: docstring comments only, actual save value (15) is used correctly.

### Earlier milestone notes

**M4.5 deviations:** Visual SVG hex editor, visual node-graph dungeon editor, useGridRenderer extraction, bulk cell selection — all deferred. No Playwright E2E for editor tabs.

**M2 deviations:** `explore town` requires data tables loaded. Panel layout persistence requires active world.

**M1 deviations:** `"w"` maps to `west` (not `wait`). Exploration scene emits 2 WS messages per action.

## Remaining Gaps (M4.7 + M4.9)

These items are incomplete from the M4.x series. See `docs/acceptance_criteria.md` for full details.

**M4.9 gaps:**
- mutmut coverage on M4 modules
- Playwright E2E tests for admin panel tabs

## Completed Subsystems (M4 series)

**Social scene** (`gm/scenes/social.py`): NPC interaction driven by UNE personality tables. Skill checks map verb → skill → attribute → difficulty, all configurable via the admin system. Disposition tracks per NPC (-3 to +3).

**Faction system** (`faction/`): Full WWN faction turns running on a weekly in-game tick. Factions have HP, Force/Cunning/Wealth stats, assets, goals, and relationships. Simple priority-based AI for turn resolution. Faction disposition modifies encounter tables in that faction's territory.

**Mythic Oracle** (`engine/oracle.py`): Full Mythic GME. Fate chart (9×9 probability matrix), chaos factor (1–9), scene checks, random event tables (focus × action × subject), thread and NPC list tracking. Plus Adventure Crafter: plotlines, themes, thread progression.

**Admin system** (`admin/`): Data-driven config tables seeded from YAML, editable per-world via REST API, CLI script, in-game commands, and Vue `/admin` panel. 12 tabs covering config and editor functions.

**Combat system** (`engine/combat.py`, `engine/saves.py`): Full XWN combat with item registry, equipped weapon resolution, shock damage, range bands, ammo tracking, 4-type saving throws, structured combat log.

**Bot framework** (`bot/`): Goal-oriented automated playtesting bot with A* pathfinder, WebSocket client, structured JSON logging, 6 goal suite.

**Shopping scene** (`gm/scenes/shopping.py`): Simple store in settlements — browse, buy/sell, inventory + encumbrance updates.

**Procedure framework** (`procedures/`): Pack-authored `roll`/`compute`/`procedure`/`format` steps with override-aware content reads. UNE personality generation and the `wickham-tables` fantasy prompt run through `ProcedureRunner`.

**Status effect service** (`status_effects/`): Durable subsystem for extrinsic entity conditions. Active effects persist in `entity_status_effects`, support replace/extend/stack semantics, expire by tick, and emit status lifecycle events.

### New config tables seeded at world creation (M4):

| Table | Seeds from |
|---|---|
| `skill_mappings` | `data/skill_mappings.yaml` |
| `difficulty_targets` | `data/difficulty_targets.yaml` |
| `disposition_outcomes` | `data/disposition_outcomes.yaml` |
| `encounter_weights` | `data/encounter_weights.yaml` |
| `faction_asset_stats` | `data/faction_assets.yaml` |

### New rules reference docs needed before coding M4:

- `docs/rules_reference/social.md` — UNE tables, disposition system, social skill check procedure
- `docs/rules_reference/faction_turns.md` — WWN faction turn mechanics, assets, actions
- `docs/rules_reference/oracle.md` — Mythic GME fate chart, scene checks, Adventure Crafter

### Key design decisions for M4:

- Skill mappings are data (YAML → SQLite), not hardcoded. The admin system allows per-world customization.
- `intimidate` uses STR modifier (not CHA) — physical presence is the mechanic.
- Failed `deceive` by 3+ flips disposition to hostile (NPC knows they were lied to).
- Faction turns fire automatically when the world clock advances past a week boundary.
- Oracle chaos factor stored in `gm_state` table, key `oracle_chaos_factor`.
- Adventure Crafter plotlines and threads stored in new `plotlines` and `threads` tables.
- Admin mode gated by `config.admin_mode = true` — in-game `admin` commands disabled by default.

## File Map (what exists vs. what M4 adds)

### Exists (do not recreate):
```
src/harsh_realm/
  main.py, config.py, db.py, events.py
  models/character.py, entity.py, grid.py, cell.py, npc.py, item.py
  engine/dice.py, skill_checks.py, combat.py, saves.py, advancement.py
  engine/enemy_ai.py, healing.py, items.py, loot.py
  engine/tables.py  ← TableEngine (M2)
  engine/oracle.py  ← PLACEHOLDER from M2, replace entirely in M4
  gm/controller.py, narrator.py
  gm/scenes/base.py, exploration.py, combat.py, respawn.py
  generators/world_gen.py, settlement_gen.py, npc_gen.py, encounter_gen.py, loot_gen.py
  parser/parser.py, commands.py
  api/routes.py, websocket.py
  house_rules/__init__.py, practice_skills.py
data/
  skills.yaml, classes.yaml, weapons.yaml, armor.yaml, equipment_kits.yaml
  tables/ (encounters, npcs, loot, terrain, settlements, names)
  creatures/ (beasts, humanoids, undead, constructs, mythical, elemental)
  templates/combat_narration.yaml
frontend/src/
  App.vue, components/ChatLog.vue, ChatPanel.vue, CommandInput.vue
  components/StatusSidebar.vue, HexMap.vue
  stores/game.ts, connection.ts
  composables/useWebSocket.ts
```

### M4 adds:
```
src/harsh_realm/
  admin/service.py, cli.py
  engine/oracle.py  ← full replacement
  engine/npc_personality.py  ← UNE generator
  engine/character_recalc.py  ← for M4.5, stub here
  gm/scenes/social.py, shopping.py
  faction/faction_turn.py, faction_ai.py, assets.py
  api/admin_routes.py
  house_rules/practice_skills.py  ← already exists, verify
data/
  skill_mappings.yaml
  difficulty_targets.yaml
  disposition_outcomes.yaml
  encounter_weights.yaml
  faction_assets.yaml
  tables/npc/une_power_level.yaml, une_descriptors.yaml
  tables/npc/une_motivation_verbs.yaml, une_motivation_nouns.yaml
  tables/npc/une_bearings.yaml, une_moods.yaml
  tables/oracle/fate_chart.yaml, event_focus.yaml
  tables/oracle/event_action.yaml, event_subject.yaml
  tables/oracle/ac_themes.yaml, ac_characters.yaml, ac_plots.yaml
  factions/  ← starting faction YAML definitions
docs/
  rules_reference/social.md  ← NEW
  rules_reference/faction_turns.md  ← NEW
  rules_reference/oracle.md  ← NEW
  milestone_4_tasks.md  ← NEW
  design/milestone_4_spec.md  ← reference (from planning session)
  design/milestone_4_admin_spec.md  ← reference (from planning session)
frontend/src/
  views/AdminView.vue
  components/admin/  ← all admin tab components
  stores/admin.ts
```
