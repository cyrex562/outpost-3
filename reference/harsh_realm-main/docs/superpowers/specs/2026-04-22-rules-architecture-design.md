# Rules-Based Architecture Design

**Date:** 2026-04-22
**Status:** Implemented 2026-04-26
**Related follow-on specs (deferred):** Events refinements, new simulation subsystems (weather, seasons, status effects, economy, reputation), content ingestion from source books.

---

## 1. Overview

### Goal

Establish architectural rules that prevent entity classes from bloating as new simulation systems are added, and apply those rules to the existing codebase in the same spec. Rip out the in-progress ECS substrate that was introduced speculatively and replace it with a focused-entity + domain-subsystem pattern coordinated by events.

### In scope

- Four architectural rules covering:
  1. Data ownership (entity vs. subsystem).
  2. Cross-subsystem interaction (reads, writes, internal work).
  3. Multi-contributor resolutions (resolver pipelines).
  4. Persistence (durable vs. derivable).
- Removal of `src/harsh_realm/ecs/` — approximately 1,700 lines across components, world, systems, adapters, context, types, encounter, actor_systems.
- Migration of the pilot `LowHealthWarningSystem` to a normal event handler.
- Case-by-case consolidation of model splits (`_runtime` / `_content` / `_state`) introduced to feed the ECS.
- Codebase review pass producing a per-file punch list appendix; `fix-in-this-spec` items fixed; `defer-to-future-spec` items catalogued.
- Updates to `CLAUDE.md` and `AGENTS.md`.

### Not in scope (deferred)

- Any *new* simulation subsystems — weather, seasons, status effects, economy, reputation, quests. Each gets its own spec when the feature is actually wanted.
- Deeper event-bus refinements beyond formalizing the rules agreed here.
- Content ingestion from source books.
- Visual / UI work.
- Any re-introduction of ECS at any future point.

### Principles

- Tests stay green at every commit (857 currently passing; count may not decrease).
- Rules must be enforceable — every rule in this spec has a concrete test for "is this code in violation?".
- Terminology is locked: "component" means *sub-model*, never "ECS component".
- The rules are documented in the canonical locations (`AGENTS.md`, `CLAUDE.md`) so future agents cannot miss them.

---

## 2. Terminology

| Term | Meaning |
|---|---|
| **Entity class** | Pydantic `BaseModel` with stable identity, representing what a thing *is*. Examples: `Character`, `NPC`, `Cell`, `Item`, `Faction`. Holds intrinsic state only. |
| **Sub-model** | Pydantic `BaseModel` embedded as a field on an entity class or another sub-model. Examples: `SaveBonusProfile` on `Character`, `UNEPersonality` on an NPC. Compositional building blocks — *not* ECS components. |
| **Subsystem** | A module with a clear domain boundary that owns a slice of simulation state. Examples: `WorldClock`, `FactionService`, `OracleService`, and future additions. Exposes typed read API plus event-driven write path. |
| **Event** | A post-commit fact published on the `EventBus`. `damage.applied`, `faction.tick_completed`. Used for reactions, UI updates, and audit. Never used mid-resolution. |
| **Resolver pipeline** | An ordered chain of typed modifier resolvers registered with an owning subsystem. Used for multi-contributor resolutions (damage, skill checks, saves). |

---

## 3. The Four Architectural Rules

### Rule 1 — Data ownership: intrinsic vs. extrinsic, with scale tiebreaker

**Statement.** Data that answers "what IS this entity" lives on the entity class. Data that answers "what is happening TO or AROUND the entity" lives in a subsystem.

When the intrinsic/extrinsic call is unclear, apply the **scale tiebreaker**: if the data has a temporal dimension (changes independently of the entity over game time), a relational dimension (many-to-many between entities), or requires cross-entity queries, it moves to a subsystem.

**Rationale.** Without this rule, entity classes accrete fields every time a new simulation concern is added, producing God classes that nobody can safely modify. The intrinsic/extrinsic frame tracks how a tabletop player would think about a character sheet vs. the world around them. The scale tiebreaker catches the cases where the primary test is ambiguous.

**Concrete calls for this codebase:**

| Domain | Where it lives | Why |
|---|---|---|
| `id`, `name`, `character_class`, `level`, `attributes`, `skills` | `Character` | Intrinsic identity |
| `hp`, `max_hp`, `ac`, `attack_bonus`, saving throws | `Character` | Intrinsic state, 1:1 with entity |
| `inventory`, `equipment`, `position` | `Character` | Intrinsic, 1:1 |
| `disposition_to_player` on an NPC | `NPC` | Intrinsic to that NPC, 1:1 |
| Status effects (burning, poisoned, cursed) | Future `StatusEffectService` | Temporal + cross-entity queryable |
| Weather, season, time-of-day | Future `WeatherService` / existing `WorldClock` | Extrinsic, regional/global |
| Multi-faction reputation and affiliation (guild, council, cult, kingdom with scores + binding levels) | Future `ReputationService` / `FactionAffiliationService` | Many-to-many, relational, feeds skill-check modifiers |
| Faction-to-faction relationships | Future `FactionRelationshipService` | Many-to-many |
| Quest / plotline state | Future `QuestService` / `PlotlineService` | Extrinsic, crosscuts entities |
| Economy, prices, trade routes | Future `EconomyService` | Extrinsic, relational |

**Known carried-forward violation.** The existing `Character.faction_id` / `NPC.faction_id` scalar is retained as a temporary simplification until a reputation/affiliation subsystem is built in a future spec. **Do not add more single-faction scalars to entity classes in the meantime** (no `guild_id`, `kingdom_id`, `cult_id`, etc.). When the reputation subsystem arrives, these fields migrate out.

**Enforcement test.** "Does this field describe what the entity IS, or what is happening to/around it?" If the field has a temporal, relational, or cross-entity dimension, it belongs in a subsystem — even if the subsystem doesn't exist yet (flag it for a future spec; do not add the field to the entity).

---

### Rule 2 — Cross-subsystem interaction

**Statement.**
- **Reads are direct typed method calls** on the owning service. Example: `weather_service.at(region_id, tick) -> WeatherState`. No event round-trips for reads.
- **Cross-subsystem writes go through events.** A subsystem that needs to cause change in another's domain emits a request event; the owning subsystem's handler performs the write and emits a result event.
- **Internal work inside a subsystem is not event-mediated.** A subsystem may do arbitrary work within its own boundaries — synchronous or `async` — without emitting events. It emits an event only when a **publishable fact** has changed: a fact another subsystem, the narrator, or the UI might reasonably want to know. (The word *synchronous* here would be misleading: `async def` methods that run without awaiting another service are still "internal work" in the sense this rule intends. The rule is about pub/sub ceremony, not `async` vs. `def`.)

**Rationale.** Direct reads avoid pub/sub ceremony for retrieving state. Event-driven writes preserve the `event_log` table as a meaningful audit trail. The "internal work is not event-mediated" carve-out prevents event-spam where every variable assignment becomes a message.

**Observable-fact test.** Before emitting, ask: *"Would anyone outside this subsystem care that this specifically changed?"* If the answer is no, don't emit. Internal helper calculations, intermediate state, and housekeeping are not events.

**Example — good:**

```python
# Reading is a direct call.
weather = await weather_service.at(cell.region_id, world_clock.tick)

# Cross-subsystem write goes through the bus.
await event_bus.emit(GameEvent(
    event_type="status.apply_requested",
    data={"entity_id": target_id, "effect": "burning", "duration_ticks": 5},
))
# ... StatusEffectService's handler performs the write, emits status.applied
```

**Example — bad:**

```python
# Direct mutation across subsystems bypasses the audit log.
await status_effect_service.apply(target_id, "burning", 5)  # violates Rule 2

# Emitting on internal work creates noise.
for region in regions:
    temp = compute_temperature(region)
    await event_bus.emit(GameEvent(event_type="weather.temp_computed", ...))  # violates Rule 2
```

**Enforcement test.** Walk each subsystem module. For each cross-subsystem interaction:
- Is it a read? It must be a typed method call.
- Is it a write? It must go through an event.
- Is it internal? It must not emit unless an externally-interesting fact changed.

---

### Rule 3 — Multi-contributor resolutions use resolver pipelines, not event chains

**Statement.** When multiple subsystems need to contribute modifiers to a single outcome (damage, skill checks, saves, movement cost, perception, loot generation), the owning subsystem defines:

1. **A typed resolution context** — Pydantic model capturing the full input state. Example: `DamageResolution(attacker, target, base_damage, range_band, modifiers_applied)`.
2. **An ordered pipeline of modifier resolvers** registered at startup. Each resolver receives the context and returns a modified version (either mutating a `modifiers_applied` list or producing a new context with adjusted fields).
3. **A single terminal event** (`damage.applied`, `skill_check.resolved`) emitted **post-commit** with the final committed values.

Other subsystems participate by registering modifier resolvers — not by subscribing to pre-commit events. House rules become resolvers in the same pipeline (`practice_skills.py` is the canonical example, generalized).

**Rationale.** Resolver pipelines give deterministic ordering, listable/inspectable contributor sets, clean event logs (one event per resolution, not one per intermediate), and direct alignment with the existing "extension point" pattern in `house_rules/`. Pre-commit mutating events through pub/sub would introduce ordering ambiguity and audit-log confusion.

**What this rule prohibits.** Passing mutable event payloads through the bus with the expectation that observers will modify them before commit.

**What this rule enables.** Listing the full resolver order in one place — anyone can answer "what affected this damage number?" by reading the pipeline registration, not by replaying events.

**Example — good:**

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

**Example — bad:**

```python
# Pre-commit event chain with mutating payload - prohibited.
await event_bus.emit(GameEvent(
    event_type="combat.damage_proposing",
    data={"ctx": mutable_damage_ctx},  # observers mutate this -> violates Rule 3
))
final = mutable_damage_ctx.damage  # whatever survived
```

**Enforcement test.** Identify code paths that compute a final value by combining inputs from multiple sources. Each must use a typed context + ordered registered resolvers + a single terminal event. Inlined if/else modifier stacks are flagged.

---

### Rule 4 — Persistence: durable vs. derivable

**Statement.** Each subsystem declares, via an explicit module-level or class-level attribute (e.g. `PERSISTENCE = "durable"` or `PERSISTENCE = "derivable"`), whether its state must survive restart.

- **Durable subsystems own their own SQLite tables.** Only that subsystem's repository module issues SQL against those tables. Cross-entity queries (*"which NPCs have status X"*) require proper relational columns — never JSON scans.
- **Derivable subsystems hold state in memory**, recomputed on world load from world seed + current tick + (optionally) replayed events. Example: weather from `(seed, region_id, tick)`.
- **Entity JSON `data` columns hold only intrinsic sub-model fields** (a Character's `save_bonuses`, a Cell's `settlement_payload`). Never extrinsic subsystem-owned state.
- **New durable subsystems ship with their own schema migration.** Existing tables are never repurposed for another subsystem's data.

**Rationale.** Without a declared ownership model, subsystem state leaks into whatever JSON column is convenient, losing query power and making ownership ambiguous. The explicit declaration makes durability reviewable, not vibes.

**Corollary — event log as history.** The `event_log` table persists what happened. For borderline-derivable subsystems (e.g., reputation as the sum of all reputation changes), the default is to store the current value in a subsystem table (fast reads) and use the event log for audit and replay. Event-sourcing-only is available for subsystems that prefer it, but is not the default.

**Example — good:**

```python
# Durable subsystem, owns its table.
class StatusEffectService:
    PERSISTENCE = "durable"

    async def apply(self, entity_id: str, effect: StatusEffect) -> None:
        async with self._db.transaction():
            await self._db.execute(
                "INSERT INTO entity_status_effects (...) VALUES (...)",
                (...),
            )

# Derivable subsystem, memory only.
class WeatherService:
    PERSISTENCE = "derivable"

    def at(self, region_id: str, tick: int) -> WeatherState:
        return self._compute_from_seed(self._seed, region_id, tick)
```

**Example — bad:**

```python
# Extrinsic subsystem data smuggled into entity JSON column.
character.data["status_effects"] = [...]  # violates Rule 4
await character_repo.save(character)
```

**Enforcement test.** Walk the SQLite schema and every `.data` JSON column usage. For each column, classify as intrinsic sub-model (fine) or extrinsic subsystem state (flag). For each table, confirm single subsystem ownership.

---

### How the rules relate

- **Rule 1** decides *where data lives* (entity vs. subsystem).
- **Rule 2** decides *how subsystems read and write across boundaries* (direct reads, eventful writes).
- **Rule 3** decides *how multiple subsystems contribute to a single outcome* (resolver pipelines, not event chains).
- **Rule 4** decides *how it all persists* (durable vs. derivable; subsystem-owned tables).

Applying them in order: for any new data, first decide under Rule 1 whether it's intrinsic or extrinsic. If extrinsic, the owning subsystem is the write authority under Rule 2. If its computation pulls from multiple subsystems, the computation uses Rule 3's pipeline pattern. Persistence is then decided under Rule 4.

---

## 4. Migration Plan

### What gets removed

- Entire `src/harsh_realm/ecs/` package:
  - `components.py` (216 lines)
  - `world.py` (183 lines)
  - `systems.py` (44 lines)
  - `actor_systems.py` (78 lines)
  - `context.py` (20 lines)
  - `types.py` (5 lines)
  - `encounter.py` (100 lines)
  - `__init__.py` (18 lines)
  - `adapters/actors.py` (499 lines)
  - `adapters/spatial.py` (529 lines)
  - `adapters/__init__.py` (9 lines)
  - Total: approximately 1,700 lines
- All `harsh_realm.ecs.*` imports in scenes, generators, engine modules, tests.
- Any test files exercising only ECS internals — deleted, not silenced.

### What gets preserved and migrated

- **`LowHealthWarningSystem` behavior.** Becomes a regular event handler subscribed to the terminal damage event (name TBD in migration — likely `combat.attack_resolved` or a dedicated `health.changed`). Same "warn at most once per session per entity at ≤25% HP" semantics preserved. Tests rewritten to exercise the event handler.
- **Typed sub-models from `ecs/components.py`.** For each ECS component:
  - If the fields already exist on an entity class → delete the component.
  - If the fields are missing from the entity class and are genuinely intrinsic → merge into the entity class.
  - If the fields are extrinsic (subsystem-owned under Rule 1) → the component is deleted; the data stays wherever it currently persists until the relevant subsystem is built in a future spec. Flagged in the punch list as a carried-forward violation.
- **Existing extension points** (`practice_skills.py` and similar house-rule hooks) stay and get formalized under Rule 3. The existing pattern *is* a resolver pipeline with one registered resolver; the refactor exposes the registration mechanism and the ordered list rather than keeping it implicit.

### Model consolidation — case by case

Files split into `_runtime` / `_content` / `_state` variants are reviewed individually. **Test:** is this split separating two genuinely different concepts, or was it only separating "data that feeds ECS" from "data that doesn't"?

| Current split | Likely decision | Reasoning |
|---|---|---|
| `combat_runtime.py` + `combat_content.py` | Merge runtime portions into `character.py` / `npc.py`; keep `combat_content.py` if it holds authored weapon/armor content | Authored content vs. runtime state is a real split; runtime state vs. entity state is not |
| `faction_runtime.py` + `faction_state.py` | Consolidate into a single `faction.py` entity + faction service module | Both halves described the same faction; split was ECS-serving |
| `cell.py` + `cell_state.py` | Consolidate; answer the existing `CellData` TODOs by typing the `data` column properly | Same entity, two shapes |
| `engine_runtime.py` + `engine_results.py` | Evaluate separately; frozen result value-objects may justify staying separate | Value objects are a legitimate split |
| `entity_state.py` | Review for whether it was an ECS feeder or holds real shared state | Decision on inspection |
| `gm_runtime.py` | Review — likely stays if it captures GM-controller transient state distinct from gameplay state | Decision on inspection |

Each merge decision is recorded in the punch list with reasoning so a future reader can see why the split did or didn't survive.

### Order of operations

1. **Write rules to `AGENTS.md` + update `CLAUDE.md`.** The rules must land in the canonical docs before any migration PR touches code, so review and migration PRs can cite them.
2. **Codebase review pass.** Produce the punch list as an appendix in this spec. No code changes yet.
3. **Remove `src/harsh_realm/ecs/`.** Single atomic change. `LowHealthWarningSystem` migrated in the same commit. All imports broken simultaneously; fix imports in the same commit. Tests pass at the end.
4. **Model consolidations.** One PR per domain (combat, faction, cell, etc.). Each PR cites the punch list entries it closes and confirms tests pass.
5. **Punch list closeout.** Verify every `fix-in-this-spec` item is closed; every `defer` item is labeled and findable for the future spec that addresses it.

### Gates

- **Tests green at every commit.** 857 tests currently pass. Every commit in this spec ends with tests green. A refactor that requires changing assertions is fine if justified; deleting a test to silence a failure is never fine.
- **No `harsh_realm.ecs` import survives.** `grep -r "harsh_realm.ecs" src/ tests/ frontend/` returns nothing at the end.
- **No behavioral regressions.** Tests exercising game mechanics must not change in ways that alter what they assert about behavior. Assertion wording may shift; assertion substance may not.

---

## 5. Execution Approach

### Codebase review methodology

The review produces an **appendix in this spec document** (§7 below) that catalogs every finding. It is performed **rule-by-rule**, not file-by-file, so each rule is applied uniformly across the codebase.

**Rule 1 review.** Walk every file in `src/harsh_realm/models/`. For each entity class, list every field and classify as intrinsic / extrinsic / ambiguous. Extrinsic fields are flagged with a target subsystem (even if that subsystem doesn't exist yet). `Character.faction_id` is the known pre-flagged violation.

**Rule 2 review.** Walk every subsystem-like module (`admin/`, `faction/`, `gm/`, `engine/`, `generators/`). For each: list its public read API, its public write paths, any place where it reads from another subsystem's storage directly, any place where it writes to another subsystem's data without an event. Direct cross-subsystem writes are flagged.

**Rule 3 review.** Identify code paths that compute a final value by combining multiple inputs from different sources (skill checks, combat damage, saving throws, encumbrance). For each, list: does it use a typed resolution context + ordered resolvers, or does it inline all modifier logic? Inlined resolutions are flagged; the existing house-rule extension point pattern is identified as the proto-pipeline.

**Rule 4 review.** Walk the SQLite schema and every `.data` JSON column usage. For each JSON column, classify as intrinsic sub-model (fine) or extrinsic subsystem state (flag). For each table, confirm single subsystem ownership. Note derivable-vs-durable declarations for existing subsystems.

**Output format.** One appendix section per rule. Each finding records: file path and line range, short description, category (`fix-in-this-spec` / `defer-to-future-spec`), and for deferrals, which future spec it belongs to.

**Time-box.** Review is pure reading, no edits. Expected to produce on the order of 30–80 findings. If the punch list exceeds 150 items, that's a signal to narrow scope and move part of the review to a follow-on spec.

### Documentation placement

**`AGENTS.md` — new section "Data ownership, subsystems, and events"** placed after "Data Models", before "Database Access". Contents:
- Terminology block (§2 above, verbatim).
- Four subsections, one per rule, each with: the rule statement, one paragraph of rationale, good/bad code examples.
- A short closing subsection on how the rules relate.

**`CLAUDE.md` — updates:**
- In "Key Architectural Rules", add a new item pointing to the `AGENTS.md` section as the canonical rules reference, with a one-sentence summary of each rule.
- In "What NOT to Do", remove any ECS-related items if present, and add: "No fields on entity classes that are extrinsic under Rule 1 — take it to a subsystem."
- In "File Map", remove `src/harsh_realm/ecs/` entries.
- Update "Current State" footer to reflect the post-migration state and note that ECS was removed in this spec.

**`docs/ARCHITECTURE.md`** (if it exists — confirm during review) — updated to remove ECS descriptions and replace with the subsystem model. If it doesn't exist, not created in this spec.

### Testing during migration

- Unit tests stay green at every commit.
- Tests exercising ECS internals get rewritten to exercise the new event handler (for `LowHealthWarningSystem`) or deleted (for pure ECS plumbing with no behavioral equivalent). Each deletion is justified in the commit message.
- Property tests may gain assertions about **resolver pipeline determinism** — given the same resolvers in the same order and same context, the output is the same.
- Mutation tests (`mutmut`) — not expanded in this spec. This is cleanup, not feature work. Existing coverage should not regress.
- Playwright tests — no changes expected. Confirm they still pass at the end.

### Success criteria

The spec is done when **all** of the following hold:

1. `src/harsh_realm/ecs/` and all subdirectories do not exist.
2. `grep -r "harsh_realm.ecs\|from harsh_realm import ecs" src/ tests/` returns nothing.
3. Rules 1–4 are documented in `AGENTS.md` with good/bad examples for each.
4. `CLAUDE.md` has been updated per the placement plan above.
5. The codebase review appendix (§7) exists with every `fix-in-this-spec` item closed and every `defer` item labeled for its future spec.
6. Test count is ≥ 857 (allowing for small shifts from rewritten tests; the number may not *decrease*).
7. All tests pass: pytest, Hypothesis property tests, Playwright.
8. `mypy --strict` (backend) and `vue-tsc --strict` (frontend) produce zero new errors.
9. A single-line persistence declaration (`PERSISTENCE = "durable"` or `"derivable"`) exists on each module identified as a subsystem during the review.
10. No `harsh_realm.ecs` references remain in any documentation file.

---

## 6. Open Items Acknowledged

None blocking. Items deliberately deferred:

- **Resolver pipeline registration mechanism.** The exact shape (protocol-based resolver interface vs. callable-based, where resolvers register, when they register) is a low-level implementation choice decided during the migration PRs, guided by the existing `house_rules/` pattern. The rule itself is fixed; the mechanism is not.
- **Event naming convention for result vs. request events.** Existing pattern (`combat.attack_requested` → `combat.attack_resolved`) is retained; if any new request/result pairs are introduced during migration, they follow that convention.
- **Future subsystem seams.** The places where the punch list will flag "this data belongs in `ReputationService`" or similar — those subsystems are not built here. The flags remain in the punch list as future-spec markers.

---

## 7. Codebase Review Appendix

This appendix records the architecture-review findings that drove the ECS
removal and model-consolidation work. Closed items are the in-scope cleanup
already completed by this spec; deferred items are intentionally left for
focused follow-on specs.

### 7.1 Rule 1 findings — Data ownership

#### F1-15A — `models/combat_runtime.py`

- Category: fix-in-this-spec (closed)
- Finding: Initial review treated combat runtime state as a possible entity-field
  split, but the file now holds transient combat-subsystem state:
  `CombatState`, `Combatant`, awareness, flee, and last-stand result models.
- Resolution: Kept `combat_runtime.py` as the combat subsystem's runtime model
  boundary and clarified its module docstring. No fields were merged into
  `Character` or NPC models because the state is encounter-scoped and not the
  durable owner of player/NPC data.

#### F1-15B — `models/combat_content.py`

- Category: fix-in-this-spec (closed)
- Finding: Combat narration models are authored YAML-loaded content, not runtime
  entity state.
- Resolution: Kept `combat_content.py`; its models were already frozen
  Pydantic value objects. Updated the module docstring to state that authored
  combat content is immutable.

#### F1-16 — `models/faction_runtime.py` + `models/faction_state.py`

- Category: fix-in-this-spec (closed)
- Finding: Faction entity/asset/relation state and faction-turn runtime action
  payloads were split across two model files, with action params importing
  faction asset data indirectly through the repository.
- Resolution: Consolidated both files into `models/faction.py`, preserving
  faction entity, asset, relation, action-result, weekly-turn, special-rule,
  and action-parameter models behind one faction model boundary. Importers now
  depend on `harsh_realm.models.faction`; the `_runtime` and `_state` modules
  were deleted.

#### F1-17 — `models/cell.py` + `models/cell_state.py`

- Category: fix-in-this-spec (closed)
- Finding: Cell row data and typed cell-backed runtime state were split across
  `cell.py` and `cell_state.py`, while `CellData.data` remained a bare
  `JsonObject`.
- Resolution: Moved `CellSettlementState`, `CellSearchState`, and
  `CellDeathMarker` into `cell.py`; deleted `cell_state.py`; and changed
  `CellData.data` to the concrete `CellDataPayload` wrapper. The repository
  still hydrates/dehydrates legacy JSON keys for compatibility while known
  settlement, search, and death-marker state lives in typed tables.

#### F1-18 — `models/engine_runtime.py` + `models/engine_results.py`

- Category: fix-in-this-spec (closed)
- Finding: The split looked suspicious during the model review, but
  `engine_results.py` contains frozen result value objects while
  `engine_runtime.py` contains typed payload records consumed by discovery,
  encounter, loot, and combat runtime flows.
- Resolution: Kept the split. Updated `engine_results.py`'s module docstring
  to make the value-object boundary explicit.

#### F1-19 — `models/entity_state.py`

- Category: fix-in-this-spec (closed)
- Finding: The `_state` suffix looked like a possible ECS feeder split, but the
  file holds typed relational persistence rows for the durable
  `character_state` and `npc_state` tables.
- Resolution: Kept `entity_state.py` as the model boundary owned by
  `EntityStateRepository` and clarified its module docstring. Character and NPC
  gameplay entities continue to live in their concrete entity model files.

#### F1-20 — `models/gm_runtime.py`

- Category: fix-in-this-spec (closed)
- Finding: `gm_runtime.py` holds transient GM scene-local state, not durable
  entity state and not ECS materialization.
- Resolution: Kept `gm_runtime.py` as the GM runtime model boundary. Removed
  the stale `gm/runtime_models.py` re-export shim and updated scene imports to
  use `harsh_realm.models.gm_runtime` directly.

### 7.2 Rule 2 findings — Cross-subsystem interaction

#### F2-01 — Exploration gameplay mutations

- Category: fix-in-this-spec (closed)
- Finding: Exploration scene code previously mixed command interpretation,
  persistence writes, and ECS projection updates in the scene layer.
- Resolution: `ExplorationEventHandler` now owns persistence for
  `exploration.move_requested`, `exploration.rest_requested`,
  `exploration.take_requested`, and `exploration.search_requested`. Scenes emit
  request events; the handler writes through `EntityRepository` and
  `CellRepository`, then emits public result events.

#### F2-02 — Shopping character mutations

- Category: fix-in-this-spec (closed)
- Finding: Shopping commands mutate character inventory and gold across the
  gameplay/entity boundary.
- Resolution: Shopping commands now emit `shopping.purchase_requested` and
  `shopping.sale_requested`; `ShoppingEventHandler` performs the repository
  write and emits terminal shopping result events.

#### F2-03 — GM command mutations

- Category: fix-in-this-spec (closed)
- Finding: GM command routes are cross-cutting admin/gameplay write paths that
  can move entities, spawn entities, give items, and set HP or gold.
- Resolution: GM routes dispatch `gm.*_requested` events through the domain
  dispatcher. `GMCommandHandler` performs the writes and emits terminal
  `gm.teleport`, `gm.spawn`, `gm.give_item`, `gm.set_hp`, and `gm.set_gold`
  events.

#### F2-04 — GM controller state persistence

- Category: fix-in-this-spec (closed)
- Finding: Controller-owned `gm_state` updates needed an explicit owning write
  path rather than being scattered through scene helpers.
- Resolution: `GMStateEventHandler` handles `gm.state_persist_requested` and
  writes through `GMStateRepository`. Scene/controller code emits the request
  event instead of issuing raw state writes directly.

#### F2-05 — Editor/admin maintenance SQL

- Category: defer-to-future-spec
- Target spec: `editor-admin-repository-boundaries`
- Finding: Editor and admin routes still issue raw SQL in several places. This
  is inside the documented admin/editor exception surface, so it is not a
  gameplay Rule 2 violation, but it remains a cleanup target for clearer
  repository ownership.
- Resolution: Deferred. Future work should consolidate editor/admin SQL behind
  typed repository or service methods where that reduces duplication without
  turning normal CRUD into gameplay-domain events.

### 7.3 Rule 3 findings — Resolver pipelines

#### F3-23 — `engine/skill_checks.py`

- Category: defer-to-future-spec
- Target spec: `resolver-pipeline-formalization`
- Finding: Skill check resolution currently inlines mapping lookup, difficulty
  adjustment, roll math, margin classification, special verb handling, and
  narration in one resolver method. There is no existing registered
  modifier-list shape to safely extract in this ECS-removal spec.
- Resolution: Deferred formal pipeline extraction to a focused future spec and
  added a TODO in `skill_checks.py` pointing at Rule 3.

### 7.4 Rule 4 findings — Persistence

#### F4-17 — Cell JSON payload typing

- Category: fix-in-this-spec (closed)
- Finding: `CellData.data` represented SQLite `cells.data` as a loose
  `JsonObject`, even though settlement/search/death-marker state already had
  typed persistence tables.
- Resolution: `CellData.data` is now `CellDataPayload`, with the typed
  settlement/search/death-marker models colocated in `cell.py`. `CellRepository`
  remains the persistence boundary that splits known payload keys into typed
  child tables and rehydrates legacy-compatible payloads for existing callers.

#### F4-22 — Existing subsystem persistence declarations

- Category: fix-in-this-spec (closed)
- Finding: Durable subsystem boundaries owned SQLite-backed state without an
  explicit `PERSISTENCE` declaration.
- Resolution: Added `PERSISTENCE = "durable"` declarations to the faction
  repository, cell repository, typed entity-state repository, GM-state
  repository, and admin config service.

### 7.5 Deferrals summary

#### Target spec: `resolver-pipeline-formalization`

- F3-23: `engine/skill_checks.py` — extract skill-check modifier
  participation into a typed context plus ordered resolver pipeline.

#### Target spec: `editor-admin-repository-boundaries`

- F2-05: editor/admin routes — consolidate remaining raw SQL behind typed
  repository or service boundaries where useful, while preserving the narrower
  admin/editor event rule.
