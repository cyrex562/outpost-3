# AGENTS.md — Coding Standards & Conventions

> This file defines the coding standards, patterns, and conventions for all code
> in the Harsh Realm project. All coding agents must follow these rules.

> **⚠️ Rust-only as of 2026-06-26.** The Python backend was ported to Rust and
> removed. The **Python sections below are historical** — they describe the
> pre-migration backend that no longer exists. Backend code now lives in
> `crates/harsh-core` + `crates/harsh-web` and follows standard Rust conventions
> (rustfmt, clippy `-D warnings`, `Result<T, String>`/`anyhow` error handling,
> sync `rusqlite` via `WorldDatabase`, serde for (de)serialization). The
> architectural rules (data ownership, subsystems owning their state, resolver
> pipelines, events through the bus, no raw SQL outside repositories) **still
> apply** — they are realized in Rust. The **Frontend** standards (Vue 3 / TS /
> Tailwind / Playwright) below are current and unchanged.

## Design Vision — IR as the Game's Interpreter

> Read this before touching `ir/`, `dispatch`, `intent`, `runtime_content/`, or
> any scene resolver. It is the north star the day-to-day rules below serve.

Harsh Realm's engine is meant to be a **content interpreter**, not a hardcoded
game. Think of the classic Infocom Z-machine: ZIL authors wrote *what exists and
how it reacts*, and a small, stable virtual machine ran it. Our intermediate
representation (IR) is that authored layer; `harsh-core` is the machine. The aim
is the same shape as Zork's interpreter, **but more detailed** — typed damage
pools, named defenses, statuses, modifiers, traits, tables, and procedures
instead of flag bits — so that nearly all game behavior is *authored data
reacting to events*, with the engine providing only primitives.

**The interpreter loop (the one mental model to hold).** Everything funnels
through one cycle, and new features should extend it rather than route around it:

```text
event ──▶ gather triggers an entity carries (statuses, items' granted traits,
          intrinsic traits, and — later — world/room/object subscriptions)
      ──▶ evaluate each trigger's `when` against an EvalContext (pure DSL)
      ──▶ lower fired `do` effects to typed Intents   (dispatch::lower_effect)
      ──▶ apply Intents to state                       (IntentApplier)
      ──▶ cascade any emitted events (bounded depth) ──▶ (loop)
```

Implemented in `runtime_content::TriggerRuntime`. Authors write triggers/effects
in YAML; they never write Rust to add content.

**Invariants — do not break these; they are why the codebase stays extensible:**

1. **The engine is pure.** `harsh-core` performs no I/O and mutates no durable
   state directly. Evaluating content yields `Intent`s — typed, serializable
   *descriptions* of change. A host applies them. The `Intent` enum is the stable
   contract between rules and persistence; widen it deliberately, never bypass it.
2. **Events are the only trigger.** State changes are events; content reacts to
   events; reactions emit events; the loop cascades. No side door that mutates
   state without an event others can observe.
3. **One content model is the goal.** We currently have **two** — the legacy
   `CreatureData`/`ItemData`/`TableEngine` registries and the IR records. This
   split is technical debt, not design: new content goes in IR, and the legacy
   catalog is being migrated to IR. Do not add a third model or deepen the legacy
   one. Converging on IR is what keeps "adding things" from getting harder.
4. **Primitives in the kernel, behavior in content.** If a thing can be authored
   (a creature's bite, a status's tick, a room's reaction), it belongs in a pack
   as IR — not as a `match` arm in engine code. Add engine code only for new
   *primitives* (a new effect verb, a new intent, a new pool type), and add them
   as small, composable, well-tested units.

**Why this matters (the anti-Dwarf-Fortress rule).** A simulation gets *more*
fun as it gets deeper, but its codebase usually gets *less* maintainable — DF is
the cautionary tale: enormous emergent depth on top of a code layout that makes
each new system harder to add than the last. We avoid that by keeping the layers
honest: a thin pure kernel of primitives, a stable intent/event boundary, and
depth that accretes in **authored content**, not in tangled engine branches.
When you reach for a feature, the default question is *"can this be IR content
the existing interpreter already runs?"* — and only if not, *"what is the
smallest new primitive that lets it be?"* Resist scene-specific special-casing in
the controller; that is exactly the growth pattern that calcifies a simulation.

See the roadmap for the open work that extends the interpreter (firing outside
combat, world-clock `time.tick` over-time effects, compute effects, the action
model, and the legacy→IR catalog migration).

## Rust Backend Conventions

These are the authoritative rules for backend code (`crates/harsh-core` +
`crates/harsh-web`). They supersede the Python sections below for anything
server-side. Where a Python rule has a direct Rust analogue it is noted.

### Crate split

- **`crates/harsh-core`** — the pure, synchronous game engine: models,
  repositories, resolvers, generators, GM controller, oracle, factions,
  procedures, content/pack services. **No async, no HTTP, no web framework.**
  `WorldDatabase` wraps a single synchronous `rusqlite` connection.
- **`crates/harsh-web`** — the Axum HTTP + WebSocket host (the old FastAPI "B9"
  layer). All async, routing, request/response shaping, and the single-world
  **session actor** live here. It depends on `harsh-core`; `harsh-core` never
  depends on it.
- **`src-tauri`** — the desktop shell; runs `harsh-web` in-process.
- Each crate has its **own `Cargo.lock`** (no workspace). When `harsh-web` needs
  a crate that `harsh-core` also uses (e.g. `rusqlite`), match the version +
  features so the native lib unifies to one build.

### Style & tooling

- **Edition 2021.** Format with `rustfmt` (default settings). Treat `clippy`
  warnings as errors (`cargo clippy -- -D warnings`); fix the code, never
  `#[allow(...)]` to silence without a comment explaining why.
- **Doc comments** (`///`) on all public items; module-level `//!` headers
  describing the file's role. This replaces the Google-style docstring rule.
- Imports grouped std → external crates → `crate::` local, blank-line separated.
- No `unwrap()`/`expect()` on fallible runtime paths — return `Result`. `expect`
  is acceptable only for genuine invariants (e.g. a literal that must parse) with
  a message explaining the guarantee.

### Data models — serde, not Pydantic

- Structured data is a `struct`/`enum` deriving `serde::{Serialize, Deserialize}`
  (the Rust analogue of the "use Pydantic for everything" rule). Use
  `#[serde(default)]`, `#[serde(rename = "...")]`, and `#[serde(flatten)]` to
  shape JSON; validate in constructors or dedicated methods.
- JSON-shaped data uses the `JsonObject` / `JsonValue` aliases
  (`serde_json::Map` / `serde_json::Value`) — the analogue of the no-`Any` rule.
  Don't reach for `serde_json::Value` when a typed struct fits.
- SQLite JSON columns: read with `serde_json::from_str`, write with
  `serde_json::to_string`. `WorldDatabase::fetch_*` returns rows as `JsonObject`
  (column-name → value), so `SELECT *` rows are already JSON.

### Database access

- All world state is SQLite via `WorldDatabase` (`rusqlite`, bundled). Gameplay,
  controllers, and generators go through **repository modules**
  (`repositories::*`, `editor::*`) — not raw SQL. Allowed raw-SQL exceptions, as
  before: the repositories themselves, and **editor/admin maintenance handlers**
  in `harsh-web` (e.g. `admin.rs`, `editor/*.rs`), plus schema/bootstrap plumbing.
- Always use **parameterised queries** (`&[&dyn ToSql]`); never format values
  into SQL. When a table name must be interpolated (e.g. `PRAGMA table_info`),
  validate it against a whitelist first (see `editor::transfer::is_exportable`).
- Group related writes; `WorldDatabase` runs on one connection so callers on the
  session-actor thread are already serialized.

### Async, the web host, and the session actor

- Only `harsh-web` is async (tokio + Axum). `harsh-core` stays sync and is called
  from blocking contexts.
- The active world is owned by a dedicated **session-actor thread**
  (`session.rs`): it holds the `WorldDatabase` + a long-lived `GMController` and
  processes jobs from an mpsc channel. HTTP handlers reach the DB via
  `state.session.read(|db| ... )`, which runs an arbitrary closure (reads *and*
  writes) on that thread — this is the serialization point.
- Editor/admin/GM endpoints target the **active world only** and ignore the
  optional `?world=` query param (the single-actor model owns one loaded world).
- Handler shape: deserialize a typed body, run a closure via `session.read`,
  return `Result<Value, String>` and map it with a small `respond()` helper
  (`Ok → Json`, `Err → 409 { error, message }`). See `editor/mod.rs`.

### Error handling & logging

- Fallible functions return `Result<T, String>` (the codebase's lingua franca)
  or a typed error; the web layer converts to structured JSON bodies
  `{ "error": ..., "message": ... }`. No silent failures — propagate or log.
- Built-in panics (`unwrap`/`expect`/indexing) are for invariants only.
- Logging: the engine surfaces errors via `Result`; the host may use `eprintln!`
  / `tracing` for operational logs. Never `print!` for debugging in committed code.

### Extension points & interfaces

- Use **traits** for injected collaborators/extension seams (the analogue of
  Python `Protocol`). Resolvers are concrete types with a working default plus
  overridable methods — house rules override specific methods, they do not
  implement abstract stubs. Avoid trait objects where generics suffice.

### Testing

Testing policy (updated 2026-07-03): favour **unit, integration, and API tests**;
**mutation testing is not required**; **Playwright runs only for changes to UI
behaviour** — day-to-day UI verification is a **human checklist**, not an automated gate.

- **Unit + property:** `#[cfg(test)]` modules with `cargo test`; property tests use
  `proptest`/seeded RNG for invariants (ranges, clamping, round-trips, monotonicity).
  Assert real values, not just `is_some()`. Every bug fix ships a regression test that
  **fails without the fix and passes with it**.
- **Integration + API:** `crates/harsh-core/tests/` (end-to-end engine flows) and
  `crates/harsh-web` tests (Axum route / websocket / API-shape tests) are the primary
  gate for backend behaviour. Prefer these over Playwright for anything not visual.
- **Schema-drift gate:** `ir::tests::committed_schema_matches_export` guards IR schema
  changes. (Known Windows-only `autocrlf` false-positive; green on CI.)
- **E2E (UI only):** Playwright (`frontend/e2e/`) drives the built frontend against a
  live `harsh-web`, and is required **only when a change alters UI function**. API-level
  specs may use `page.request` against the real host.
- **The gate:** `cargo xtask test` runs cargo unit+integration+API on both crates;
  `cargo xtask test --ui` adds the Playwright suite (use for UI-behaviour changes);
  `--core-only` is the fast inner loop. `scripts/dev-test.sh` remains the full gate.

### Issue tracking

Active bugs/features are tracked in **GitHub Issues** (as of 2026-07-04). File and read
them with the `gh` CLI — `gh issue list`, `gh issue view N`, `gh issue create`,
`gh issue comment N`, `gh issue close N` — so no commit/push is needed to share an item.
`todo.md` is a **historical archive** (HR-771…799); do not add new items there. Issue
titles keep the `HR-###` prefix for continuity with commit messages and design docs, but
the **GitHub issue number is the canonical id** going forward.

### Automated fix loop

Multi-item backlogs are worked as a **fix → review → test → PR** loop, one item at a
time, with the **human as the final gate for merging** each PR:

1. **Fix** — one agent implements a single issue on a branch off the latest `main`,
   including a regression test that fails without the fix.
2. **Review** — a second agent code-reviews the diff (correctness, event wiring,
   persistence, payload alignment, adherence to these conventions) and the fix agent
   addresses findings.
3. **Test** — a third agent runs and, if needed, strengthens the gate:
   `cargo xtask test` (add `--ui` only when UI behaviour changed).
4. **PR** — open a PR whose body contains `Fixes #N` (auto-closes the issue on merge);
   **stop**. The human reviews and merges. The next item branches off the updated `main`.

Keep each item's PR focused. The `Fixes #N` reference closes the issue on merge; add a
short completion note (what changed + the regression test) in the PR body.

### Pack-aware content

- New game records belong in **packs** (`content/<pack-id>/...`), not hardcoded in
  engine modules. Runtime reads go through `PackRegistry` / `WorldPackRepository`
  / `ContentService`. Pack code hooks (compute callables) register into
  `ComputeRegistry` — see `ComputeRegistry::register_builtins`.

---

> The sections below (Python Style, Pydantic, pytest/Hypothesis/mutmut, aiosqlite)
> are **historical** — they document the removed Python backend. They remain as a
> reference for the pre-migration design and for the architectural rules that
> carried over. For backend work, follow **Rust Backend Conventions** above.

## Python Style

- **Version:** Python 3.12+
- **Formatter:** black, 88-character line length
- **Quotes:** Double quotes for all strings (`"hello"` not `'hello'`)
- **Imports:** stdlib → third-party → local, separated by blank lines. Use `from __future__ import annotations` in every file.
- **Type hints:** Required on all function signatures and class attributes. Use modern syntax (`list[str]` not `List[str]`, `str | None` not `Optional[str]`).
- **Docstrings:** Required on all public functions, classes, and modules. Use Google-style docstrings.

```python
from __future__ import annotations

def resolve_skill_check(
    skill: str,
    attr_mod: int,
    skill_level: int,
    difficulty: int = 8,
    modifiers: list[Modifier] | None = None,
) -> SkillCheckResult:
    """Resolve an XWN skill check (2d6 + skill + attr_mod vs. difficulty).

    Args:
        skill: Name of the skill being checked.
        attr_mod: Attribute modifier (-2 to +2).
        skill_level: Skill level (-1 to +4).
        difficulty: Target number (default 8).
        modifiers: Optional list of situational modifiers.

    Returns:
        SkillCheckResult with roll details and success/failure.
    """
```

## Data Models

**Use Pydantic for all data models.** This applies to every class that holds structured data — internal game models, API schemas, config records, result objects, everything. Do not use stdlib `@dataclass` for anything that will be serialized, deserialized, validated, or stored.

### Which Pydantic base to use

| Use case | Base class | Notes |
|---|---|---|
| Mutable game entity (character, NPC, faction) | `pydantic.BaseModel` | Default. Supports validation, serialization. |
| Immutable result/value object (dice result, check result) | `pydantic.BaseModel` with `model_config = ConfigDict(frozen=True)` | Hashable, safe to use as dict key or in sets. |
| FastAPI request/response body | `pydantic.BaseModel` | Same as internal models — no distinction. |
| SQLite JSON column deserialization | `pydantic.BaseModel` | Use `model_validate(json.loads(row["data"]))`. |

**Never use:**
- `stdlib @dataclass` for any data model (use Pydantic instead)
- `TypedDict` for anything that needs validation or serialization
- Plain `dict` passed between functions where a model would be appropriate
- `NamedTuple` for structured data

### Field rules

- All fields must have explicit types. No `Any` unless the field genuinely accepts anything, in which case document why.
- Do not use bare `object` as a convenience type. If the shape is known, model it.
- Do not use `Any` as a convenience type. Treat it as an escape hatch only at narrow framework/library boundaries, and narrow it immediately.
- Use `str | None` not `Optional[str]`. Use `list[str]` not `List[str]`.
- Use `Field(default=..., description="...")` for fields that need documentation or validation constraints.
- JSON columns in SQLite store the model's data. Read with `ModelClass.model_validate(json.loads(data))`. Write with `model_instance.model_dump_json()`.

### No `object` / `Any` escape hatches

Do not use `object`, `dict[str, object]`, `list[object]`, or `Any` to get around
writing a real type.

Prefer, in order:

1. A Pydantic model for structured data
2. `JsonValue` / `JsonObject` for JSON-shaped data
3. A `Protocol` for services, framework state, or injected collaborators
4. A concrete union for temporary migration boundaries
5. A `TypeVar` or generic parameter for reusable helpers

Allowed exceptions are narrow and should be documented:

- Pydantic validators that accept arbitrary input before narrowing
- third-party or framework boundaries where no stable type exists upstream
- very local generic internals that do not leak into application-facing APIs

Bad:

```python
def handle(app_state: Any, payload: dict[str, object]) -> object:
    ...
```

Better:

```python
class AppStateProtocol(Protocol):
    world: WorldDatabase | None
    event_bus: EventBus | None


class MovePayload(BaseModel):
    q: int
    r: int


def handle(app_state: AppStateProtocol, payload: MovePayload) -> GameEvent:
    ...
```

```python
from pydantic import BaseModel, Field, ConfigDict

class SkillCheckResult(BaseModel):
    """Result of an XWN skill check."""
    model_config = ConfigDict(frozen=True)

    roll: int = Field(ge=2, le=12, description="Raw 2d6 roll")
    total: int = Field(description="Roll + skill level + attribute modifier")
    target: int = Field(description="Difficulty target number")
    margin: int = Field(description="total - target; positive = success")
    success: bool
    natural_2: bool = Field(description="True if raw roll was 2 (always fails)")
    natural_12: bool = Field(description="True if raw roll was 12 (always succeeds)")


class Character(BaseModel):
    """A player or NPC character."""
    id: str
    name: str
    character_class: str = Field(
        description="One of: warrior, expert, adventurer"
    )
    level: int = Field(default=1, ge=1, le=10)
    xp: int = Field(default=0, ge=0)
    attributes: dict[str, int] = Field(default_factory=dict)
    skills: dict[str, int] = Field(default_factory=dict)
    hp: int = Field(default=0)
    max_hp: int = Field(default=0)
    ac: int = Field(default=10, ge=0)
    equipment: list[str] = Field(default_factory=list)


class NPCPersonality(BaseModel):
    """UNE-generated personality block, stored in entity data JSON."""
    model_config = ConfigDict(frozen=True)

    power_level: str
    descriptor: str
    motivation_verb: str
    motivation_noun: str
    bearing: str
    bearing_focus: str
    base_disposition: int = Field(default=0, ge=-3, le=3)


# Reading from SQLite JSON column:
row = await db.fetchone("SELECT data FROM entities WHERE id = ?", (entity_id,))
character = Character.model_validate(json.loads(row["data"]))

# Writing to SQLite JSON column:
await db.execute(
    "UPDATE entities SET data = ? WHERE id = ?",
    (character.model_dump_json(), entity_id),
)
```

### Pydantic validators

Use validators for business logic that belongs in the model:

```python
from pydantic import field_validator, model_validator

class FactionStats(BaseModel):
    force: int = Field(ge=0, le=8)
    cunning: int = Field(ge=0, le=8)
    wealth: int = Field(ge=0, le=8)
    hp: int = Field(ge=0)
    max_hp: int = Field(ge=1)

    @model_validator(mode="after")
    def hp_cannot_exceed_max(self) -> "FactionStats":
        if self.hp > self.max_hp:
            raise ValueError(f"hp ({self.hp}) cannot exceed max_hp ({self.max_hp})")
        return self
```

## Data Ownership, Subsystems, and Events

These four rules govern how simulation state is distributed across entity classes and subsystem modules, how subsystems interact, and how multi-contributor resolutions work. They are load-bearing for the codebase's long-term health and are enforced in code review.

### Terminology

| Term | Meaning |
|---|---|
| **Entity class** | Pydantic `BaseModel` with stable identity, representing what a thing *is*. Examples: `Character`, `NPC`, `Cell`, `Item`, `Faction`. Holds intrinsic state only. |
| **Sub-model** | Pydantic `BaseModel` embedded as a field on an entity class or another sub-model. Examples: `SaveBonusProfile` on `Character`. Compositional building blocks -- *not* ECS components. |
| **Subsystem** | A module with a clear domain boundary that owns a slice of simulation state. Exposes typed read API plus event-driven write path. Examples: `WorldClock`, `FactionService`, `OracleService`. |
| **Event** | A post-commit fact published on the `EventBus`. Used for reactions, UI updates, audit. Never used mid-resolution. |
| **Resolver pipeline** | An ordered chain of typed modifier resolvers registered with an owning subsystem. Used for multi-contributor resolutions (damage, skill checks, saves). |

### Rule 1 -- Data ownership: intrinsic vs. extrinsic

Data that answers "what IS this entity" lives on the entity class. Data that answers "what is happening TO or AROUND the entity" lives in a subsystem.

**Scale tiebreaker.** When the intrinsic/extrinsic call is unclear, check: does the data have a temporal dimension (changes over game time independently of the entity), a relational dimension (many-to-many between entities), or a cross-entity query requirement? If yes, it moves to a subsystem.

**Concrete:**

- Entity: `id`, `name`, `attributes`, `skills`, `hp`, `ac`, `inventory`, `equipment`, `position`.
- Subsystem: weather, season, time-of-day, status effects, multi-faction reputation, faction-to-faction relationships, quest state, economy.

**Known carried-forward violation.** `Character.faction_id` / `NPC.faction_id` (single scalar) is retained as a temporary simplification until a reputation/affiliation subsystem is built in a future spec. Do not add more single-faction scalars to entity classes.

**Bad:**

```python
class Character(BaseModel):
    id: str
    name: str
    hp: int
    # ...
    current_weather: str  # violates Rule 1 -- extrinsic, belongs in WeatherService
    active_diseases: list[str]  # violates Rule 1 -- temporal + cross-entity, belongs in subsystem
    kingdom_id: str  # violates Rule 1 -- second single-faction scalar, relational
```

**Good:**

```python
# Entity holds intrinsic state only.
class Character(BaseModel):
    id: str
    name: str
    hp: int
    # ...
    faction_id: str | None  # carried-forward simplification; flag in punch list

# Weather lives in its own subsystem.
class WeatherService:
    def at(self, region_id: str, tick: int) -> WeatherState:
        ...
```

### Rule 2 -- Cross-subsystem interaction

- **Reads are direct typed method calls** on the owning service. Example: `weather_service.at(region_id, tick) -> WeatherState`. No event round-trips for reads.
- **Cross-subsystem writes go through events.** A subsystem that needs to cause change in another's domain emits a request event; the owning subsystem's handler performs the write and emits a result event.
- **Internal work inside a subsystem is not event-mediated.** A subsystem may do arbitrary work within its own boundaries -- synchronous or `async` -- without emitting events. It emits an event only when a publishable fact has changed: a fact another subsystem, the narrator, or the UI might reasonably want to know.

**Observable-fact test.** Before emitting, ask: *would anyone outside this subsystem care that this specifically changed?* If no, don't emit.

**Good:**

```python
# Reading is a direct call.
weather = await weather_service.at(cell.region_id, world_clock.tick)

# Cross-subsystem write goes through the bus.
await event_bus.emit(GameEvent(
    event_type="status.apply_requested",
    data={"entity_id": target_id, "effect": "burning", "duration_ticks": 5},
))
# StatusEffectService handler performs the write, emits status.applied
```

**Bad:**

```python
# Direct mutation across subsystems bypasses the audit log.
await status_effect_service.apply(target_id, "burning", 5)  # violates Rule 2

# Emitting on internal work creates noise.
for region in regions:
    temp = _compute_temperature(region)
    await event_bus.emit(GameEvent(event_type="weather.temp_computed", ...))  # violates Rule 2
```

### Rule 3 -- Multi-contributor resolutions use resolver pipelines

When multiple subsystems need to contribute modifiers to a single outcome (damage, skill checks, saves, movement cost, perception, loot generation), the owning subsystem defines:

1. A typed resolution context (Pydantic model) capturing the full input state.
2. An ordered pipeline of modifier resolvers registered at startup.
3. A single terminal event emitted post-commit with the final committed values.

Other subsystems participate by registering modifier resolvers -- not by subscribing to pre-commit events.

**What this rule prohibits.** Passing mutable event payloads through the bus with the expectation that observers will modify them before commit.

**Good:**

```python
class DamageResolution(BaseModel):
    attacker_id: str
    target_id: str
    base_damage: int
    range_band: str
    modifiers_applied: list[DamageModifier] = Field(default_factory=list)
    final_damage: int | None = None


class CombatService:
    def __init__(self) -> None:
        self._damage_resolvers: list[DamageResolver] = []

    def register_damage_resolver(self, resolver: DamageResolver) -> None:
        self._damage_resolvers.append(resolver)

    async def resolve_damage(self, ctx: DamageResolution) -> DamageResolution:
        for resolver in self._damage_resolvers:
            ctx = resolver.apply(ctx)
        ctx.final_damage = compute_final(ctx)
        # commit to target...
        await self._event_bus.emit(
            GameEvent(event_type="combat.damage_applied", data=ctx.model_dump())
        )
        return ctx
```

**Bad:**

```python
# Pre-commit event chain with mutating payload -- prohibited.
await event_bus.emit(GameEvent(
    event_type="combat.damage_proposing",
    data={"ctx": mutable_damage_ctx},  # observers mutate this
))
final = mutable_damage_ctx.damage
```

### Rule 4 -- Persistence: durable vs. derivable

Each subsystem declares, via an explicit module-level or class-level attribute (e.g. `PERSISTENCE = "durable"` or `PERSISTENCE = "derivable"`), whether its state must survive restart.

- **Durable subsystems own their own SQLite tables.** Only that subsystem's repository module issues SQL against those tables. Cross-entity queries require relational columns -- never JSON scans.
- **Derivable subsystems hold state in memory**, recomputed on world load from world seed + current tick + (optionally) replayed events.
- **Entity JSON `data` columns hold only intrinsic sub-model fields.** Never extrinsic subsystem-owned state.
- **New durable subsystems ship with their own schema migration.**

**Good:**

```python
class StatusEffectService:
    PERSISTENCE = "durable"

    async def apply(self, entity_id: str, effect: StatusEffect) -> None:
        async with self._db.transaction():
            await self._db.execute(
                "INSERT INTO entity_status_effects (...) VALUES (...)",
                (...),
            )

class WeatherService:
    PERSISTENCE = "derivable"

    def at(self, region_id: str, tick: int) -> WeatherState:
        return self._compute_from_seed(self._seed, region_id, tick)
```

**Bad:**

```python
# Extrinsic subsystem data smuggled into entity JSON column.
character.data["status_effects"] = [...]  # violates Rule 4
```

### How the rules relate

Rule 1 decides where data lives. Rule 2 decides how subsystems read and write across boundaries. Rule 3 decides how multiple subsystems contribute to a single outcome. Rule 4 decides how it all persists. For any new data: classify under Rule 1, apply Rule 2 for interaction, use Rule 3 if multi-contributor, decide persistence under Rule 4.

## Interfaces & Protocols

- Use `Protocol` (from `typing`) for interfaces, NOT `ABC`.
- Extension points (resolvers) use a concrete base class with overridable methods, not abstract methods.

```python
from typing import Protocol

class SceneHandler(Protocol):
    """Interface for GM scene state handlers."""
    def get_valid_commands(self) -> list[CommandSpec]: ...
    def get_prompt(self, world: WorldDatabase) -> str: ...
    def handle_command(self, cmd: ParsedCommand, world: WorldDatabase) -> list[GameEvent]: ...
    def tick(self, world: WorldDatabase) -> list[GameEvent]: ...
    def check_transitions(self, events: list[GameEvent]) -> SceneState | None: ...
```

## Extension Point Pattern

Resolvers have a working default implementation. House rules override specific methods.

```python
class SkillCheckResolver:
    """XWN skill check resolver. Override resolve() for house rules."""

    def resolve(
        self,
        skill: str,
        attr_mod: int,
        skill_level: int,
        difficulty: int = 8,
        modifiers: list[Modifier] | None = None,
    ) -> SkillCheckResult:
        """Default XWN implementation. Override for house rules."""
        mods = modifiers or []
        roll = self._roll_2d6()
        total = roll + skill_level + attr_mod + sum(m.value for m in mods)
        margin = total - difficulty
        return SkillCheckResult(
            roll=roll,
            total=total,
            target=difficulty,
            margin=margin,
            success=total >= difficulty,
            natural_2=(roll == 2),
            natural_12=(roll == 12),
        )

    def _roll_2d6(self) -> int:
        """Isolate randomness for testability."""
        return random.randint(1, 6) + random.randint(1, 6)
```

## Event Bus

- **All gameplay state changes must flow through the EventBus or the in-process domain-event layer.**
  Scene handlers, GM command routes, and other gameplay mutation paths should emit
  command/request events and let persistence handlers perform the write.
- **Editor/admin mutations use a narrower rule.** Do not turn all CRUD into gameplay
  events. Emit live-update events only when the mutation affects the currently loaded
  world or runtime-relevant config that subscribers may need to react to.
- Events use dotted namespace strings: `"combat.attack"`, `"exploration.enter_hex"`, `"gm.scene_change"`.
- Event data payloads are plain dicts (JSON-serializable). Keep them flat and descriptive.
- Event handlers must not raise exceptions that propagate to the bus. Catch and log errors, emit error events if needed.

Current event boundary:
- gameplay commands and state transitions: request event -> persistence/repository handler -> public result event
- selective editor live updates: `editor.live_update`
- selective runtime-config updates: `admin.config_updated`
- audit/editor CRUD events may still exist separately and should not be treated as gameplay-domain events by default

```python
from pydantic import BaseModel, Field, ConfigDict

class GameEvent(BaseModel):
    """A single game event flowing through the EventBus."""
    model_config = ConfigDict(frozen=True)

    id: str = Field(description="Auto-generated UUID")
    tick: int = Field(description="World clock value")
    event_type: str = Field(description="Dotted namespace, e.g. 'combat.attack'")
    data: dict[str, Any] = Field(description="JSON-serializable payload")
    source: str = Field(default="system", description="'player', 'system', 'gm', or subsystem name")
```

## Database Access

- Use `aiosqlite` for all database operations.
- The `WorldDatabase` class wraps all DB access.
- Gameplay scenes, controllers, GM routes, and engine mutation paths should not issue raw SQL directly.
  Put DB reads/writes behind repository or adapter modules.
- Raw SQL is tolerated in a narrow exception surface:
  - repository/adapter modules that intentionally own persistence
  - editor/admin maintenance routes and services
  - import/export and bootstrap/schema plumbing
- Use parameterized queries. Never f-string or format SQL.
- JSON columns use `json.dumps()` on write and `json.loads()` on read.
- Transactions: group related writes in a single `async with db.execute(...):` block.

## Pack-Aware Code

- Game content records live in `content/<pack-id>/content/`, not in engine modules.
- The built-in default content pack is `content/xwn-core/`; engine/editor metadata such as editor schemas lives under `content/schemas/`.
- Runtime game-content reads should go through `ContentService`, `PackRegistry`, `WorldPackRepository`, or `harsh_realm.packs.paths.default_content_dir()` during compatibility migration. Do not add new hardcoded root `data/...` paths.
- Per-world content edits live in `pack_overrides`; the owning repository is `WorldPackRepository`.
- Code-bearing packs place code under `content/<pack-id>/code/` and expose `register(app_state)` from `code/__init__.py`; registration is handled by `load_pack_code()`.
- Multi-step generators are authored as procedure records under `content/<pack-id>/content/procedures/` and executed through `ProcedureRunner`; code-bearing packs may register compute functions for procedure steps.
- New record types added in future cycles should ship as pack content with tests that prove discovery/loading through the pack layer.

```python
# Good
await db.execute(
    "UPDATE entities SET data = ?, updated_at = ? WHERE id = ?",
    (json.dumps(entity_data), now_iso(), entity_id),
)

# Bad — never do this
await db.execute(f"UPDATE entities SET data = '{data}' WHERE id = '{id}'")
```

## Testing

Testing is not optional and is not a separate phase. Tests are written alongside the implementation, not after. A feature is incomplete until all four test layers pass.

### Layer 1: Unit Tests (pytest)

- **Framework:** pytest + pytest-asyncio
- **Location:** `tests/` mirroring source structure
- **Command:** `pytest --tb=short -q`
- **Coverage command:** `pytest --cov=src/harsh_realm --cov-report=term-missing`
- **Coverage target:** >= 80% line coverage overall; >= 90% on all `engine/`, `gm/scenes/`, `faction/`, `admin/` modules

**Rules:**
- One test file per source module: `src/harsh_realm/engine/dice.py` -> `tests/test_dice.py`
- Test function names describe behavior: `test_skill_check_natural_2_always_fails`, not `test_skill_check_1`
- Use `@pytest.mark.parametrize` for any test with multiple input/expected-output cases -- never copy-paste test bodies
- All randomness must be seeded or injected: `DiceRoller(seed=42)`, never live random in unit tests
- Async tests use `@pytest.mark.asyncio`
- Fixtures in conftest.py for: in-memory world database, seeded dice roller, event bus, sample character, sample faction

```python
# Good -- parametrized, describes behavior, asserts real values
@pytest.mark.parametrize("roll,skill_level,attr_mod,difficulty,expected", [
    (12, 1, 1, 8, True),   # natural 12 always succeeds
    (2,  4, 2, 8, False),  # natural 2 always fails
    (7,  1, 1, 8, True),   # 7 + 1 + 1 = 9 >= 8
    (5,  1, 1, 8, False),  # 5 + 1 + 1 = 7 < 8
])
def test_skill_check_success(roll, skill_level, attr_mod, difficulty, expected):
    resolver = SkillCheckResolver()
    resolver._roll_2d6 = lambda: roll
    result = resolver.resolve("stab", attr_mod=attr_mod,
                              skill_level=skill_level, difficulty=difficulty)
    assert result.success == expected
    assert result.roll == roll
    assert result.margin == (roll + skill_level + attr_mod) - difficulty

# Bad -- vague name, no assertion on actual values
def test_skill_check():
    result = SkillCheckResolver().resolve("stab", 1, 1)
    assert result is not None
```

### Layer 2: Property-Based Tests (Hypothesis)

- **Framework:** hypothesis
- **Location:** `tests/test_properties.py`
- **Command:** `pytest tests/test_properties.py --hypothesis-seed=0`

Write property tests for any function where an invariant should hold across all valid inputs. Do not write property tests for things already covered by specific parametrized cases -- write them for things that should be true *for all inputs*.

```python
from hypothesis import given
from hypothesis import strategies as st

# Dice: result always within mathematical bounds
@given(num_dice=st.integers(1, 10), sides=st.integers(2, 20), modifier=st.integers(-10, 10))
def test_dice_result_always_in_range(num_dice, sides, modifier):
    roller = DiceRoller(seed=0)
    result = roller.roll(num_dice, sides, modifier)
    assert result.final >= num_dice + modifier
    assert result.final <= (num_dice * sides) + modifier

# Attribute modifiers: monotone and bounded
@given(score=st.integers(3, 18))
def test_attr_modifier_bounded(score):
    assert -2 <= attr_modifier(score) <= 2

# Disposition: always clamped to [-3, 3]
@given(initial=st.integers(-3, 3), delta=st.integers(-10, 10))
def test_disposition_always_clamped(initial, delta):
    result = max(-3, min(3, initial + delta))
    assert -3 <= result <= 3

# Chaos factor: always in [1, 9] regardless of adjustment sequence
@given(start=st.integers(1, 9), adjustments=st.lists(st.integers(-3, 3), max_size=100))
def test_chaos_always_bounded(start, adjustments):
    tracker = ChaosTracker(start)
    for adj in adjustments:
        if adj > 0: tracker.increase()
        elif adj < 0: tracker.decrease()
    assert 1 <= tracker.value <= 9
```

**Minimum property test targets per milestone:**
- M4: dice, attr_mod, disposition, chaos_factor, skill_check, admin round-trips
- M5: encumbrance slot totals, dungeon room connectivity, loot value ranges
- M6: faction XP conservation, faction stat bounds, world tick ordering

### Layer 3: Mutation Tests (mutmut)

- **Framework:** mutmut
- **Command:** `mutmut run --paths-to-mutate src/harsh_realm/<module>.py`
- **Results:** `mutmut results`
- **Target:** >= 85% mutants killed per module

Run mutation tests on these module categories after writing unit + property tests:

```bash
# Run after completing any engine module
mutmut run --paths-to-mutate src/harsh_realm/engine/skill_checks.py
mutmut run --paths-to-mutate src/harsh_realm/engine/combat.py
mutmut run --paths-to-mutate src/harsh_realm/engine/oracle.py

# Run after completing any scene handler
mutmut run --paths-to-mutate src/harsh_realm/gm/scenes/social.py

# Run after completing faction or admin modules
mutmut run --paths-to-mutate src/harsh_realm/faction/faction_turn.py
mutmut run --paths-to-mutate src/harsh_realm/admin/service.py
```

**Handling surviving mutants:**

```python
# Option A -- write a test that kills the mutant
# mutmut changed: "margin >= 0" to "margin > 0"
# Kill it by adding a test for the exact boundary:
def test_skill_check_success_at_exact_difficulty():
    """Margin of 0 (roll exactly equals difficulty) is a success."""
    resolver = SkillCheckResolver()
    resolver._roll_2d6 = lambda: 6  # 6 + 1 + 1 = 8 == difficulty 8
    result = resolver.resolve("stab", attr_mod=1, skill_level=1, difficulty=8)
    assert result.success is True
    assert result.margin == 0

# Option B -- document equivalent mutant
def clamp_chaos(value: int) -> int:
    # EQUIVALENT MUTANT: mutmut changes `max(1, ...)` to `max(0, ...)` --
    # equivalent because chaos is never passed a value below 1 in production.
    return max(1, min(9, value))
```

Never leave a surviving mutant undocumented. Either kill it or explain why it's equivalent.

### Layer 4: Playwright E2E Tests

- **Framework:** Playwright
- **Location:** `frontend/tests/e2e/`
- **Config:** `frontend/playwright.config.ts`
- **Command:** `npx playwright test`
- **Prerequisite:** Backend running on :8080, Vite dev server on :5173

One spec file per major UI surface. Required structure:

```typescript
// frontend/tests/e2e/admin-skill-mappings.spec.ts
import { test, expect } from '@playwright/test'

test.describe('Admin -- Skill Mappings Tab', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/admin')
    await page.click('[data-tab="skill-mappings"]')
  })

  test('renders all 7 default verbs', async ({ page }) => {
    const rows = page.locator('[data-testid="skill-mapping-row"]')
    await expect(rows).toHaveCount(7)
  })

  test('inline edit and save updates value', async ({ page }) => {
    await page.fill('[data-verb="convince"] [data-field="difficulty"]', '14')
    await page.click('[data-verb="convince"] [data-action="save"]')
    await page.reload()
    const val = page.locator('[data-verb="convince"] [data-field="difficulty"]')
    await expect(val).toHaveValue('14')
  })
})
```

**Required `data-testid` attributes:** Every interactive element in the admin panel and game UI must have a `data-testid` attribute for Playwright targeting. Never use CSS selectors or text content as locators.

**Minimum coverage per milestone's new UI:**
- M4: Admin panel tabs (skill mappings, difficulties, disposition, encounter weights, faction assets, tables)
- M4.5: Hex editor, character editor, dungeon editor, world operations
- Every future milestone: any new Vue component, scene transition visible in UI, WebSocket-pushed update

### Test Execution Order Per Task

1. Write implementation code
2. Write unit tests -> pytest passes
3. Write property tests (if module qualifies) -> hypothesis passes
4. Run mutation tests -> >= 85% killed or document survivors
5. Write Playwright tests (if UI component created) -> playwright passes
6. Update test count in commit message
7. Task is complete

Do not proceed to the next task until all four layers pass for the current task.

## Error Handling

- **No silent failures.** Every error path must either raise an exception, return an explicit error value, or log a warning.
- Use built-in exceptions for programming errors (`ValueError`, `TypeError`, `KeyError`).
- Define custom exceptions in a `exceptions.py` module for game-logic errors.
- FastAPI endpoints return structured error responses: `{"error": "ErrorType", "message": "description"}`.
- Event handlers must catch exceptions and log them. Never let a handler crash the event bus.

```python
class HarshRealmError(Exception):
    """Base exception for game logic errors."""

class EntityNotFoundError(HarshRealmError):
    """Raised when an entity ID doesn't exist in the world."""

class InvalidCommandError(HarshRealmError):
    """Raised when a command is not valid in the current scene state."""

class InsufficientResourceError(HarshRealmError):
    """Raised when an action requires resources the entity doesn't have."""
```

## Logging

- Use Python's `logging` module (not print statements).
- Logger per module: `logger = logging.getLogger(__name__)`
- Log levels: DEBUG for internal state, INFO for game events, WARNING for recoverable issues, ERROR for failures.
- Include context in log messages (entity ID, event type, etc.).

```python
logger = logging.getLogger(__name__)

logger.info("Combat started", extra={"entities": [e.id for e in combatants]})
logger.debug("Attack roll: %d + %d vs AC %d", roll, attack_bonus, target_ac)
logger.warning("Unknown command verb: %s", cmd.verb)
logger.error("Failed to load table: %s", table_id, exc_info=True)
```

## File & Module Organization

- One concept per file. Don't put unrelated classes in the same module.
- Keep files under 400 lines. Split a module (extract mixins, helpers, or a package) when it grows beyond that.
- Use `__init__.py` for re-exports. Keep `__init__.py` files minimal — just imports.
- No circular imports. If A and B need each other, extract the shared type into a third module.

## YAML Content Files

- All hand-authored game content is YAML in `data/`.
- Every YAML file must have a top-level `id`, `category`, and `name` for table files.
- Use `tags` arrays for contextual filtering.
- Subtable references use `{ table: "table_id" }` syntax.
- When creating stub tables for content the developer will fill in, include 3-5 placeholder entries and a comment noting the file needs population from source material.

```yaml
# data/tables/encounters/forest.yaml
id: encounters_forest
category: encounter
name: Forest Encounters
tags: [wilderness, forest, temperate]
# TODO: Populate from source material. Placeholder entries below.
entries:
  - weight: 3
    result: { type: "creature", description: "A pack of wild dogs" }
  - weight: 2
    result: { type: "npc", table: "traveling_npcs" }
  - weight: 1
    result: { type: "nothing", description: "The forest is quiet." }
```

## Frontend (Vue 3 + TypeScript)

### TypeScript: Strict Mode Required

**`tsconfig.json` must include `"strict": true` at all times.** This enables the full strict
flag group: `noImplicitAny`, `strictNullChecks`, `strictFunctionTypes`,
`strictPropertyInitialization`, `noImplicitThis`, `alwaysStrict`. Never disable any of these
individually to silence an error -- fix the code instead.

Required `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "exactOptionalPropertyTypes": true,
    "jsx": "preserve",
    "lib": ["ESNext", "DOM"],
    "types": ["vitest/globals"],
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src/**/*.ts", "src/**/*.vue", "tests/**/*.ts"]
}
```

Run `npx tsc --noEmit` as part of every task. Zero errors required before a task is complete.

### Type Rules

Every piece of TypeScript must be fully typed. No escape hatches:

- **No `any`.** If a type is genuinely unknown, use `unknown` and narrow it explicitly.
  The only acceptable `any` is when consuming a third-party library that has no types --
  wrap it in a typed adapter and document why.
- **No non-null assertions (`!`) without a comment** explaining why the value is guaranteed
  non-null at that point.
- **No type assertions (`as X`)** except when narrowing from `unknown` after a runtime check.
  Never use `as X` to paper over a type mismatch.
- **No `@ts-ignore` or `@ts-expect-error`** except for known upstream library bugs --
  document with a link to the issue.

### API Response Types

All API responses must have corresponding TypeScript interfaces in `frontend/src/types/api.ts`.
Never use inline `any` or `Record<string, unknown>` for API response shapes.

```typescript
// frontend/src/types/api.ts

export interface SkillMapping {
  verb: string
  skill: string
  attribute: string
  base_difficulty: number
  opposed: boolean
  description: string
}

export interface Character {
  id: string
  name: string
  character_class: 'warrior' | 'expert' | 'adventurer'
  level: number
  xp: number
  attributes: Record<string, number>
  skills: Record<string, number>
  hp: number
  max_hp: number
  ac: number
  equipment: string[]
}

export interface GameEvent {
  id: string
  tick: number
  event_type: string
  data: Record<string, unknown>
  source: string
}

export interface WebSocketMessage {
  type: 'narration' | 'player_input' | 'game_event' | 'error' | 'state_update'
  content: string
  data?: Record<string, unknown>
}

// All admin panel types, faction types, hex types, item types, dungeon types, etc.
// follow the same pattern -- one interface per API response shape.
```

### Component Typing Rules

Every Vue component must be fully typed:

```typescript
// Props -- always use defineProps with generic syntax, never runtime-only
const props = defineProps<{
  characterId: string
  readonly?: boolean
}>()

// Emits -- always typed
const emit = defineEmits<{
  saved: [character: Character]
  cancelled: []
  'disposition-changed': [npcId: string, newScore: number]
}>()

// Computed -- return type inferred but must be unambiguous; add explicit type if inference
// would produce a union that's too wide
const dispositionLabel = computed<string>(() => {
  return DISPOSITION_LABELS[props.score] ?? 'Unknown'
})

// Template refs -- always typed
const inputRef = useTemplateRef<HTMLInputElement>('input')

// Stores -- always typed via Pinia's return type, never cast
const adminStore = useAdminStore()  // type flows from store definition
```

### Pinia Store Typing

Every store must have explicit state, getter, and action types. No implicit `any` from
untyped state fields:

```typescript
// frontend/src/stores/admin.ts
import { defineStore } from 'pinia'
import type { SkillMapping, WorldMeta } from '@/types/api'

interface AdminState {
  activeWorldPath: string
  availableWorlds: WorldMeta[]
  isDirty: boolean
  activeTab: AdminTab
  pendingHexSelection: { q: number; r: number } | null
}

type AdminTab =
  | 'skill-mappings'
  | 'difficulties'
  | 'disposition'
  | 'encounter-weights'
  | 'faction-assets'
  | 'tables'
  | 'worlds'
  | 'hex-editor'
  | 'character-editor'
  | 'faction-editor'
  | 'dungeon-editor'
  | 'yaml-files'
  | 'world-meta'

export const useAdminStore = defineStore('admin', {
  state: (): AdminState => ({
    activeWorldPath: '',
    availableWorlds: [],
    isDirty: false,
    activeTab: 'skill-mappings',
    pendingHexSelection: null,
  }),
  // getters and actions follow -- all explicitly typed
})
```

### Composable Typing

All composables must declare explicit return types:

```typescript
// Good -- explicit return type
export function useDisposition(initialScore: number): {
  score: Ref<number>
  label: ComputedRef<string>
  apply: (delta: number) => void
  reset: () => void
} {
  const score = ref(initialScore)
  const label = computed(() => DISPOSITION_LABELS[score.value] ?? 'Unknown')
  const apply = (delta: number) => {
    score.value = Math.max(-3, Math.min(3, score.value + delta))
  }
  const reset = () => { score.value = initialScore }
  return { score, label, apply, reset }
}

// Bad -- return type inferred as a wide object type with possible `any` fields
export function useDisposition(initialScore: number) {
  // ...
}
```

### Component Conventions

- **Single-file components** (`.vue`) with `<script setup lang="ts">` -- always.
- **Tailwind CSS** for all styling. No custom CSS unless Tailwind genuinely cannot express it.
- **Pinia** for all cross-component state. No prop drilling beyond two levels.
- **Composables** (`use*.ts`) for any reactive logic used by more than one component.
- Keep components under 400 lines. Extract child components, composables, or helpers when one grows beyond that.
- WebSocket connection owned by a single composable (`useWebSocket`), never per-component.
- Every interactive element used in Playwright tests must have a `data-testid` attribute.
  Name it descriptively: `data-testid="skill-mapping-row-convince"` not `data-testid="row-1"`.

## Frontend Testing

Frontend features require the same four-layer test coverage as Python code.
A frontend task is NOT complete until all applicable layers pass.

### Layer 1: Unit Tests (Vitest)

**Framework:** Vitest + Vue Test Utils
**Location:** `frontend/tests/unit/`
**Config:** `frontend/vitest.config.ts`
**Command:** `npm run test:unit` (runs `vitest run`)
**Coverage command:** `npm run test:unit -- --coverage`
**Coverage target:** >= 80% line coverage on all stores, composables, and utility functions

What to unit-test:
- **All Pinia stores** -- every action and getter in isolation, with mocked API calls
- **All composables** -- pure reactive logic tested without mounting a component
- **All utility functions** in `frontend/src/utils/` or `frontend/src/lib/`
- **Complex computed properties** in components where the logic is non-trivial

What NOT to unit-test with Vitest:
- Simple template rendering (use Playwright instead)
- WebSocket integration (use Playwright instead)
- Cross-component interactions (use Playwright instead)

```typescript
// frontend/vitest.config.ts
import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  test: {
    globals: true,
    environment: 'jsdom',
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      include: ['src/stores/**', 'src/composables/**', 'src/utils/**'],
      thresholds: { lines: 80, functions: 80, branches: 75 },
    },
  },
  resolve: {
    alias: { '@': resolve(__dirname, 'src') },
  },
})
```

Store test pattern:

```typescript
// frontend/tests/unit/stores/admin.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useAdminStore } from '@/stores/admin'
import type { SkillMapping } from '@/types/api'

// Mock the API module -- never make real HTTP calls in unit tests
vi.mock('@/api/admin', () => ({
  fetchSkillMappings: vi.fn(),
  updateSkillMapping: vi.fn(),
  resetSkillMapping: vi.fn(),
}))

import { fetchSkillMappings, updateSkillMapping } from '@/api/admin'

describe('useAdminStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('loads skill mappings from API on fetchAll', async () => {
    const mockMappings: SkillMapping[] = [
      { verb: 'convince', skill: 'Talk', attribute: 'CHA',
        base_difficulty: 8, opposed: true, description: '' },
    ]
    vi.mocked(fetchSkillMappings).mockResolvedValue(mockMappings)

    const store = useAdminStore()
    await store.fetchSkillMappings()

    expect(store.skillMappings).toHaveLength(1)
    expect(store.skillMappings[0].verb).toBe('convince')
    expect(store.skillMappings[0].base_difficulty).toBe(8)
  })

  it('marks store dirty when a mapping is edited but not saved', () => {
    const store = useAdminStore()
    expect(store.isDirty).toBe(false)
    store.editSkillMapping('convince', { base_difficulty: 14 })
    expect(store.isDirty).toBe(true)
  })

  it('clears dirty flag after save', async () => {
    vi.mocked(updateSkillMapping).mockResolvedValue(undefined)
    const store = useAdminStore()
    store.editSkillMapping('convince', { base_difficulty: 14 })
    await store.saveSkillMapping('convince')
    expect(store.isDirty).toBe(false)
  })
})
```

Composable test pattern:

```typescript
// frontend/tests/unit/composables/useDisposition.test.ts
import { describe, it, expect } from 'vitest'
import { useDisposition } from '@/composables/useDisposition'

describe('useDisposition', () => {
  it('initialises with the provided score', () => {
    const { score } = useDisposition(0)
    expect(score.value).toBe(0)
  })

  it('clamps positive delta at +3', () => {
    const { score, apply } = useDisposition(2)
    apply(5)
    expect(score.value).toBe(3)
  })

  it('clamps negative delta at -3', () => {
    const { score, apply } = useDisposition(-2)
    apply(-5)
    expect(score.value).toBe(-3)
  })

  it('label reflects current score', () => {
    const { label, apply } = useDisposition(0)
    expect(label.value).toBe('Indifferent')
    apply(2)
    expect(label.value).toBe('Friendly')
  })

  it('reset returns to initial score', () => {
    const { score, apply, reset } = useDisposition(1)
    apply(2)
    reset()
    expect(score.value).toBe(1)
  })
})
```

### Layer 2: Property-Based Tests (fast-check)

**Framework:** fast-check (integrates with Vitest)
**Location:** `frontend/tests/unit/properties/`
**Command:** same as unit tests (`npm run test:unit`)

Write property tests for any frontend logic with numeric invariants, clamping behavior,
round-trip semantics, or functions that should hold for all valid inputs.

Minimum property test targets per milestone:
- **M4:** Disposition clamping, chaos factor bounds, skill mapping round-trips, admin store dirty-flag invariants
- **M4.5:** Hex coordinate round-trips, character derived-stat monotonicity (higher STR never produces lower melee), dungeon room/connection graph invariants (no orphan rooms, no self-loops)
- **Future milestones:** Any new composable with numeric/clamping behavior, any new parser/serializer

### Layer 3: Mutation Tests (Stryker)

**Framework:** Stryker Mutator (JavaScript/TypeScript mutation testing)
**Location:** `frontend/stryker.config.json`
**Command:** `npx stryker run`
**Target:** >= 85% mutants killed in stores and composables

```json
{
  "$schema": "./node_modules/@stryker-mutator/core/schema/stryker-schema.json",
  "testRunner": "vitest",
  "mutate": [
    "src/stores/**/*.ts",
    "src/composables/**/*.ts",
    "src/utils/**/*.ts",
    "!**/*.d.ts"
  ],
  "thresholds": {
    "high": 85,
    "low": 70,
    "break": 60
  },
  "reporters": ["html", "clear-text"],
  "tempDirName": ".stryker-tmp"
}
```

Run Stryker after completing unit and property tests for any store, composable, or utility.
Handle surviving mutants the same way as Python mutmut: either write a test that kills the
mutant or add an inline comment explaining why it is a known equivalent mutant.

### Layer 4: Playwright E2E Tests

Playwright tests are covered in the main Testing section of this document (Layer 4).
The key addition for TypeScript: all Playwright test files must also pass `tsc --noEmit`.
Playwright config must include TypeScript strict checking for test files:

```typescript
// frontend/playwright.config.ts
import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,  // serial -- game state is shared
  retries: process.env.CI ? 2 : 0,
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
  },
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
  ],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173',
    reuseExistingServer: !process.env.CI,
  },
})
```

### Required `package.json` dev dependencies

```json
{
  "devDependencies": {
    "@playwright/test": "^1.40.0",
    "@stryker-mutator/core": "^8.0.0",
    "@stryker-mutator/vitest-runner": "^8.0.0",
    "@vitejs/plugin-vue": "^5.0.0",
    "@vitest/coverage-v8": "^1.0.0",
    "@vue/test-utils": "^2.4.0",
    "fast-check": "^3.15.0",
    "jsdom": "^24.0.0",
    "typescript": "^5.3.0",
    "vitest": "^1.0.0"
  },
  "scripts": {
    "test:unit": "vitest run",
    "test:unit:watch": "vitest",
    "test:unit:coverage": "vitest run --coverage",
    "test:e2e": "playwright test",
    "test:mutation": "stryker run",
    "typecheck": "tsc --noEmit",
    "test:all": "npm run typecheck && npm run test:unit:coverage && npm run test:e2e"
  }
}
```

### Frontend Task Completion Checklist

A frontend task is complete when ALL of the following pass:

```
[] npx tsc --noEmit -> 0 errors
[] npm run test:unit:coverage -> all tests pass, coverage >= 80%
[] Property tests in tests/unit/properties/ pass
[] npx stryker run -> >= 85% mutants killed (stores + composables)
[] npx playwright test -> all E2E tests pass
[] No 'any', no non-null assertions without comments, no @ts-ignore
```

## Git Conventions

- Branch names: `feat/<description>`, `fix/<description>`, `refactor/<description>`
- Commit messages: imperative mood, concise. `"Add skill check resolver with extension point"` not `"Added some stuff"`.
- Never push directly to `main`. All work on feature branches.
- Each milestone completion is tagged: `milestone-0`, `milestone-1`, etc.

## What NOT to Do

- Do NOT use `ABC` or `abstractmethod`. Use `Protocol` or concrete base classes.
- Do NOT use stdlib `@dataclass` for data models. Use Pydantic `BaseModel` instead. The only acceptable use of stdlib `@dataclass` is for non-data classes (e.g., a class that holds only methods and no persistent state).
- Do NOT write raw SQL outside of `db.py`.
- Do NOT mutate shared state outside of event handlers and database transactions.
- Do NOT use `print()` for debugging. Use `logging`.
- Do NOT hardcode magic numbers. Use named constants or config values.
- Do NOT hardcode root `data/...` paths for game content. Use pack-aware APIs.
- Do NOT put game records in engine code. New records belong in packs.
- Do NOT add new hardcoded multi-step generators in Python. Use a procedure record and `ProcedureRunner`.
- Do NOT create files longer than 400 lines. When a file grows beyond 400 lines, break it up (extract mixins/helpers/child components or split into a package).
- Do NOT use `localStorage` or `sessionStorage` in Vue components (not supported in all deployment contexts).
- Do NOT use `any` in TypeScript. Use `unknown` with explicit narrowing, or a typed interface.
- Do NOT use non-null assertions (`!`) without an inline comment explaining the guarantee.
- Do NOT disable TypeScript strict mode flags to silence errors. Fix the code.
- Do NOT write Vitest tests that only assert `.toBeTruthy()` or `.toBeDefined()` -- assert the actual value.
- Do NOT skip property tests for composables with numeric/clamping behavior.
- Do NOT merge frontend code without running `tsc --noEmit` first.
- Do NOT add dependencies without checking that they're necessary. Prefer stdlib solutions.
