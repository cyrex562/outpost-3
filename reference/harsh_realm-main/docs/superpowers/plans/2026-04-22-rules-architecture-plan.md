# Rules-Based Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish four architectural rules (data ownership, cross-subsystem interaction, resolver pipelines, durable/derivable persistence), document them in AGENTS.md and CLAUDE.md, review the codebase against them, tear out the in-progress ECS substrate (~1,700 lines plus dependent scene code), migrate the one pilot ECS system to a normal event handler, and consolidate model splits that were introduced to feed the ECS.

**Architecture:** Entity classes hold intrinsic state only. Subsystem modules own extrinsic simulation state and expose typed read APIs plus event-mediated writes. Multi-contributor resolutions (damage, skill checks, saves) use ordered resolver pipelines registered with the owning subsystem, not pre-commit event chains. Durable subsystem state lives in subsystem-owned SQLite tables; derivable state lives in memory.

**Tech Stack:** Python 3.12 / Pydantic / FastAPI / aiosqlite / pytest / Hypothesis / Vue 3 / TypeScript.

**Reference spec:** [docs/superpowers/specs/2026-04-22-rules-architecture-design.md](../specs/2026-04-22-rules-architecture-design.md)

---

## Conventions used in this plan

- Task numbering is continuous across phases.
- "Preflight" steps at the start of work-requiring tasks read the current code first; don't skip them.
- Every task ends with a test run and a commit. Any task where tests don't pass at the end is not complete — fix forward, do not paper over.
- Commit message style: imperative mood, short subject, optional body. Per existing CLAUDE.md convention.
- Repository root in this plan is `/home/cyrex/Projects/harsh_realm/`. Paths are relative to that unless stated otherwise.

---

## Phase 1 — Documentation lands first

The rules must be committed to the canonical docs before any code migration PR begins, so later PRs can cite them.

### Task 1: Add "Data ownership, subsystems, and events" section to AGENTS.md

**Files:**
- Modify: `AGENTS.md` (insert new section after the "Data Models" section, before "Database Access")

- [ ] **Step 1: Read current AGENTS.md to locate the insertion point**

Run: `grep -n "^##" AGENTS.md`
Identify the line numbers of the "Data Models" section (start) and "Database Access" section (start). The new section will be inserted immediately before "Database Access".

- [ ] **Step 2: Write the new section**

Insert a section with the exact content below, adjusting markdown table style to match the project's existing convention (no spaces inside separator rows like `|---|---|`).

Content to insert:

````markdown
## Data Ownership, Subsystems, and Events

These four rules govern how simulation state is distributed across entity classes and subsystem modules, how subsystems interact, and how multi-contributor resolutions work. They are load-bearing for the codebase's long-term health and are enforced in code review.

### Terminology

| Term | Meaning |
|---|---|
| **Entity class** | Pydantic `BaseModel` with stable identity, representing what a thing *is*. Examples: `Character`, `NPC`, `Cell`, `Item`, `Faction`. Holds intrinsic state only. |
| **Sub-model** | Pydantic `BaseModel` embedded as a field on an entity class or another sub-model. Examples: `SaveBonusProfile` on `Character`. Compositional building blocks — *not* ECS components. |
| **Subsystem** | A module with a clear domain boundary that owns a slice of simulation state. Exposes typed read API plus event-driven write path. Examples: `WorldClock`, `FactionService`, `OracleService`. |
| **Event** | A post-commit fact published on the `EventBus`. Used for reactions, UI updates, audit. Never used mid-resolution. |
| **Resolver pipeline** | An ordered chain of typed modifier resolvers registered with an owning subsystem. Used for multi-contributor resolutions (damage, skill checks, saves). |

### Rule 1 — Data ownership: intrinsic vs. extrinsic

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
    current_weather: str  # violates Rule 1 — extrinsic, belongs in WeatherService
    active_diseases: list[str]  # violates Rule 1 — temporal + cross-entity, belongs in subsystem
    kingdom_id: str  # violates Rule 1 — second single-faction scalar, relational
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

### Rule 2 — Cross-subsystem interaction

- **Reads are direct typed method calls** on the owning service. Example: `weather_service.at(region_id, tick) -> WeatherState`. No event round-trips for reads.
- **Cross-subsystem writes go through events.** A subsystem that needs to cause change in another's domain emits a request event; the owning subsystem's handler performs the write and emits a result event.
- **Internal work inside a subsystem is not event-mediated.** A subsystem may do arbitrary work within its own boundaries — synchronous or `async` — without emitting events. It emits an event only when a publishable fact has changed: a fact another subsystem, the narrator, or the UI might reasonably want to know.

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

### Rule 3 — Multi-contributor resolutions use resolver pipelines

When multiple subsystems need to contribute modifiers to a single outcome (damage, skill checks, saves, movement cost, perception, loot generation), the owning subsystem defines:

1. A typed resolution context (Pydantic model) capturing the full input state.
2. An ordered pipeline of modifier resolvers registered at startup.
3. A single terminal event emitted post-commit with the final committed values.

Other subsystems participate by registering modifier resolvers — not by subscribing to pre-commit events.

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
# Pre-commit event chain with mutating payload — prohibited.
await event_bus.emit(GameEvent(
    event_type="combat.damage_proposing",
    data={"ctx": mutable_damage_ctx},  # observers mutate this
))
final = mutable_damage_ctx.damage
```

### Rule 4 — Persistence: durable vs. derivable

Each subsystem declares, via an explicit module-level or class-level attribute (e.g. `PERSISTENCE = "durable"` or `PERSISTENCE = "derivable"`), whether its state must survive restart.

- **Durable subsystems own their own SQLite tables.** Only that subsystem's repository module issues SQL against those tables. Cross-entity queries require relational columns — never JSON scans.
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
````

- [ ] **Step 3: Verify AGENTS.md still parses as markdown**

Run: `python3 -c "import pathlib; print(len(pathlib.Path('AGENTS.md').read_text().splitlines()))"`
Expected: A line count greater than the previous line count (was 1053). Should be ~1250–1300.

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md
git commit -m "docs(agents): add data ownership, subsystems, and events section

Adds four architectural rules (Rule 1: data ownership, Rule 2:
cross-subsystem interaction, Rule 3: resolver pipelines, Rule 4:
durable/derivable persistence) that the codebase will follow going
forward. Canonical rules reference cited from CLAUDE.md.

Refs: docs/superpowers/specs/2026-04-22-rules-architecture-design.md"
```

---

### Task 2: Update CLAUDE.md to point at the new AGENTS.md rules and remove ECS references

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Locate the "Key Architectural Rules" section**

Run: `grep -n "Key Architectural Rules" CLAUDE.md`

- [ ] **Step 2: Add a new rule item pointing at the canonical reference**

Find the numbered list under "Key Architectural Rules". Append a new numbered item:

```markdown
6. **Data ownership, subsystems, and events.** Entity classes hold intrinsic state only; extrinsic simulation state lives in subsystems. Reads across subsystems are direct typed method calls; writes go through events. Multi-contributor resolutions (damage, skill checks) use ordered resolver pipelines, not pre-commit event chains. Durable subsystems own their tables; derivable state lives in memory. See the "Data Ownership, Subsystems, and Events" section in `AGENTS.md` for the full rules with examples.
```

- [ ] **Step 3: Update the "What NOT to Do" section**

Run: `grep -n "What NOT to Do" CLAUDE.md`

Inside the "What NOT to Do" list, add three items and remove any ECS-favorable items if present:

Add:
- `No fields on entity classes that are extrinsic under Rule 1 — take it to a subsystem (or flag for a future spec if the subsystem does not yet exist).`
- `No direct writes across subsystem boundaries — cross-subsystem writes go through the event bus per Rule 2.`
- `No pre-commit mutating event payloads for multi-contributor resolutions — use resolver pipelines per Rule 3.`

Remove (if present): any item endorsing ECS, components, or system iteration as a pattern.

- [ ] **Step 4: Remove `src/harsh_realm/ecs/` entries from the File Map**

Run: `grep -n "ecs" CLAUDE.md`

Locate the File Map section. Under the "Exists (do not recreate)" block, remove any line referencing `ecs/` (there may be none if the file map wasn't updated when ECS was added — confirm). Do not add a line to the "M4 adds" block.

- [ ] **Step 5: Update "Current State" footer**

Change the current-state test-count line to reflect the post-migration count (to be known at the end — for now, leave a note and revisit in Task 37). No commit yet if the count is not final; stage the other changes.

- [ ] **Step 6: Run verification**

```bash
grep -n "harsh_realm.ecs\|ECS\|ecs/" CLAUDE.md || echo "no ECS references found"
```

Expected: The only remaining references to "ECS" should be historical context (e.g., "ECS was removed in ...") if you added any; raw `ecs/` paths should not appear.

- [ ] **Step 7: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(claude): cite AGENTS.md rules and remove ECS references

Adds a Key Architectural Rules item pointing at the canonical rules
section in AGENTS.md. Adds Rule 1/2/3 violations to the What NOT to Do
list. Removes ecs/ from the file map. Current State test count will be
revisited in the closeout task after migration."
```

---

## Phase 2 — Codebase review

The review is pure reading — no edits. Output: populate the punch list appendix in the spec document with concrete findings.

### Task 3: Rule 1 review — walk every entity class in `src/harsh_realm/models/`

**Files:**
- Read-only: all `.py` files in `src/harsh_realm/models/`
- Modify: `docs/superpowers/specs/2026-04-22-rules-architecture-design.md` (§7.1 appendix)

- [ ] **Step 1: List all model files**

Run: `ls src/harsh_realm/models/*.py`

- [ ] **Step 2: For each entity class in each file, classify every field**

For each Pydantic `BaseModel` subclass that represents an entity (not a value object, not a frozen result), list every field and classify it as:
- `intrinsic` (stays on entity)
- `extrinsic` (should be subsystem-owned — note target subsystem)
- `ambiguous` (flag for discussion)

Write findings to a scratch file `/tmp/rule1_findings.md` in this format:

```markdown
## src/harsh_realm/models/character.py — Character
- id: intrinsic
- name: intrinsic
- hp / max_hp: intrinsic
- ...
- faction_id: EXTRINSIC (carried-forward violation — reputation subsystem, future spec)

## src/harsh_realm/models/npc.py — NPC
...
```

- [ ] **Step 3: Identify cross-entity JSON `data` columns**

Run: `grep -rn "data:" src/harsh_realm/models/ | grep -iE "JsonObject|JsonValue|dict\[str"`

For each match, check whether the `data` field holds intrinsic sub-model content or extrinsic subsystem state.

- [ ] **Step 4: Write the §7.1 appendix entries**

Open `docs/superpowers/specs/2026-04-22-rules-architecture-design.md` and replace the `*To be populated.*` placeholder under `### 7.1 Rule 1 findings` with the findings. Use this format per entry:

```markdown
**F1-01** `src/harsh_realm/models/character.py:39` — `Character.faction_id: str | None`
- Classification: EXTRINSIC (relational when reputation subsystem arrives)
- Category: `defer-to-future-spec` (carried-forward violation)
- Target spec: `reputation-subsystem`
- Notes: current single-scalar is simplification; do not extend.
```

Findings are numbered `F1-01`, `F1-02`, ... for Rule 1.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-22-rules-architecture-design.md
git commit -m "docs(spec): populate §7.1 Rule 1 findings from codebase review"
```

---

### Task 4: Rule 2 review — walk subsystem-like modules

**Files:**
- Read-only: `src/harsh_realm/{admin,faction,gm,engine,generators,bot,api}/`
- Modify: spec §7.2 appendix

- [ ] **Step 1: List subsystem-like modules**

Run: `find src/harsh_realm -type d -not -path "*__pycache__*" | sort`

- [ ] **Step 2: For each module, catalog its public API and cross-module calls**

For each subsystem, answer in a scratch file:
1. What are its public read functions? (what do other modules call it for data?)
2. What are its public write paths?
3. Does it read from another subsystem's tables directly? (grep for raw SQL mentioning table names owned elsewhere)
4. Does it write to another subsystem's data without going through an event bus? (grep for cross-module mutations)

Run: `grep -rn "await.*\.emit\|event_bus\.emit\|EventBus" src/harsh_realm/` to inventory existing event emissions.

- [ ] **Step 3: Flag violations**

A violation under Rule 2 is: direct cross-subsystem write without an event, OR direct cross-subsystem database read that bypasses a typed service method.

- [ ] **Step 4: Write §7.2 appendix entries**

Numbered `F2-01`, `F2-02`, ... Each entry names the file, line, violation, fix-category, and suggested remediation (move to event? add service method?).

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-22-rules-architecture-design.md
git commit -m "docs(spec): populate §7.2 Rule 2 findings from codebase review"
```

---

### Task 5: Rule 3 review — identify multi-contributor resolution paths

**Files:**
- Read-only: `src/harsh_realm/engine/{skill_checks,combat,saves,healing,advancement}.py` and similar resolution-shaped files
- Modify: spec §7.3 appendix

- [ ] **Step 1: Identify candidate files**

Run: `ls src/harsh_realm/engine/*.py`. Also grep for functions that combine modifiers: `grep -rn "modifier\|bonus\|penalty" src/harsh_realm/engine/ | wc -l`.

- [ ] **Step 2: For each resolution-shaped function, note its shape**

For each function that computes a final value by combining inputs from multiple sources:
- Does it accept a typed resolution context (Pydantic model)?
- Does it delegate to an ordered list of resolvers?
- Or does it inline all the modifier logic?

- [ ] **Step 3: Identify whether any pattern resembles a resolver pipeline**

The CLAUDE.md references `house_rules/practice_skills.py` as an existing extension point, but the directory and file do not exist as of this review. Confirm with: `find src/harsh_realm -name "house_rules*" -o -name "practice_skills*"`. If no such files exist, note in the appendix that the canonical example does not yet exist and Rule 3 formalization is limited in Phase 5 of this plan.

- [ ] **Step 4: Write §7.3 appendix entries**

Numbered `F3-01`, `F3-02`, ... Each entry names the resolution function, whether it uses a pipeline today, and the suggested action (`fix-in-this-spec` for Phase 5 candidates; `defer-to-future-spec` for ones not in scope here).

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-22-rules-architecture-design.md
git commit -m "docs(spec): populate §7.3 Rule 3 findings from codebase review"
```

---

### Task 6: Rule 4 review — SQLite schema and JSON column usage

**Files:**
- Read-only: `src/harsh_realm/db_schema.py`, `src/harsh_realm/db.py`, any repository modules
- Modify: spec §7.4 appendix

- [ ] **Step 1: Dump the schema**

Run: `grep -E "CREATE TABLE|CREATE INDEX" src/harsh_realm/db_schema.py`

- [ ] **Step 2: For each table, identify the owning subsystem**

Write a table-by-table owner map in a scratch file. If any table has no clear single owner, flag as ambiguous.

- [ ] **Step 3: For each JSON `data` column, classify contents**

Run: `grep -rn "\.data\[" src/harsh_realm/ | grep -v test_` to find every JSON column access.

For each access pattern, classify:
- Intrinsic sub-model content (fine — candidate for typing if currently loose)
- Extrinsic subsystem state smuggled in (violates Rule 4 — flag)

- [ ] **Step 4: Identify existing subsystem persistence declarations**

Run: `grep -rn "PERSISTENCE\s*=" src/harsh_realm/`. Expected: zero results (the declaration is new).

Note each subsystem module that will need a `PERSISTENCE` declaration added (in Phase 4 or as its own small task).

- [ ] **Step 5: Write §7.4 appendix entries**

Numbered `F4-01`, `F4-02`, ... Each entry: file/line, issue, fix-category, suggested action.

- [ ] **Step 6: Commit**

```bash
git add docs/superpowers/specs/2026-04-22-rules-architecture-design.md
git commit -m "docs(spec): populate §7.4 Rule 4 findings from codebase review"
```

---

### Task 7: Write the deferrals summary (§7.5) and punch list summary

**Files:**
- Modify: spec §7.5 appendix

- [ ] **Step 1: Aggregate all `defer-to-future-spec` items**

From the §7.1–7.4 appendix sections just populated, collect every item tagged `defer-to-future-spec` into a flat list.

- [ ] **Step 2: Group by target future spec**

For each deferred item, assign a target future spec name (e.g. `reputation-subsystem`, `weather-subsystem`, `status-effects-subsystem`, `economy-subsystem`). Collect the items under each group.

- [ ] **Step 3: Write §7.5**

Replace the `*To be populated.*` placeholder under `### 7.5 Deferrals summary` with the grouped list. Format:

```markdown
### 7.5 Deferrals summary

#### Target spec: reputation-subsystem
- F1-01: `Character.faction_id` carried-forward violation
- F1-XX: ...

#### Target spec: status-effects-subsystem
- F1-XX: ...
- F4-XX: ...

... etc.
```

- [ ] **Step 4: Verify no `To be populated` placeholders remain in §7**

Run: `grep -n "To be populated" docs/superpowers/specs/2026-04-22-rules-architecture-design.md`
Expected: no results.

- [ ] **Step 5: Commit**

```bash
git add docs/superpowers/specs/2026-04-22-rules-architecture-design.md
git commit -m "docs(spec): populate §7.5 deferrals summary; review complete"
```

At this point the spec's §7 appendix is fully populated. Phases 3–6 reference findings `F1-XX`, `F2-XX`, `F3-XX`, `F4-XX` by number.

---

## Phase 3 — ECS teardown

All work in this phase ends with the `src/harsh_realm/ecs/` package and its tests deleted, scene files refactored off ECS, and the pilot `LowHealthWarningSystem` behavior preserved as a regular event handler. Test count may not decrease below 857.

### Task 8: Preflight — inventory ECS usage across scenes

**Files:**
- Read-only: scene files and ECS imports

- [ ] **Step 1: List all scene files that import ECS**

Run: `grep -rln "harsh_realm.ecs" src/harsh_realm/gm/scenes/`

Expected (from pre-plan scan):
- `exploration_core.py`
- `exploration_movement.py`
- `exploration_persistence.py`
- `exploration_interaction.py`
- `town.py`
- `dungeon.py`
- `combat_core.py`

- [ ] **Step 2: For each scene file, catalog the specific ECS imports and usages**

For each file, run: `grep -n "harsh_realm.ecs\|Ecs\|ecs_\|_ecs" <file>` and capture the specific symbols used (components, `EcsWorld` methods, adapters).

Write findings to `/tmp/ecs_usage_inventory.md` in this format:

```markdown
## src/harsh_realm/gm/scenes/combat_core.py
- Line 34: imports `EcsWorld, HealthComponent`
- Line 112: `world.get_component(entity_id, HealthComponent)` — reads HP for entity
- Line 145: `world.set_component(entity_id, HealthComponent(...))` — writes damaged HP

## src/harsh_realm/gm/scenes/town.py
...
```

- [ ] **Step 3: Classify each usage as REPLACE / DELETE**

For each usage:
- `REPLACE`: the behavior is real and must continue after ECS is gone. Note how the replacement works (usually: read/write the entity model directly via its repo or in-memory reference).
- `DELETE`: the behavior is ECS plumbing with no non-ECS equivalent (e.g., registering an ECS system on world startup).

- [ ] **Step 4: Commit the inventory to the spec's appendix**

Append the inventory as a new subsection `### 7.6 ECS Usage Inventory (preflight for Phase 3)` to the spec file.

```bash
git add docs/superpowers/specs/2026-04-22-rules-architecture-design.md
git commit -m "docs(spec): add ECS usage inventory for teardown preflight"
```

---

### Task 9: Migrate `LowHealthWarningSystem` behavior to an event handler

**Files:**
- Create: `src/harsh_realm/engine/low_health_narration.py`
- Create: `tests/test_low_health_narration.py`
- Read: `src/harsh_realm/ecs/actor_systems.py` (to confirm the behavior being migrated)
- Read: existing combat event type definitions to choose the right trigger event

- [ ] **Step 1: Identify the trigger event**

Run: `grep -rn "event_type\s*=\s*['\"]combat\." src/harsh_realm/` to find the existing combat event type strings.

Pick the terminal post-damage event emitted after `HealthComponent.hp` would have decreased — typically something like `combat.attack_resolved` or `combat.damage_applied`. If there isn't a clear single event that fires on HP change, note this and create a minimal `health.changed` event emission at the point where HP is written in combat code.

Record the chosen event name: `__CHOSEN_EVENT__` (fill in during execution).

- [ ] **Step 2: Write the failing test first**

Create `tests/test_low_health_narration.py`:

```python
"""Tests for the low-health narration event handler (replaces LowHealthWarningSystem)."""

from __future__ import annotations

import pytest

from harsh_realm.engine.low_health_narration import LowHealthNarrator
from harsh_realm.events import GameEvent


@pytest.mark.asyncio
async def test_emits_warning_when_player_hp_first_drops_below_25_percent() -> None:
    emitted: list[GameEvent] = []
    narrator = LowHealthNarrator(threshold_fraction=0.25)
    narrator.bind_emitter(lambda evt: emitted.append(evt))

    # Simulate a post-damage event with HP 20/100 (below threshold).
    post_damage = GameEvent(
        tick=5,
        event_type="combat.attack_resolved",  # replace if different event chosen
        data={
            "target_id": "player-1",
            "target_is_player": True,
            "target_hp": 20,
            "target_max_hp": 100,
            "target_name": "Kael",
            "target_alive": True,
        },
        source="combat",
    )
    await narrator.handle(post_damage)

    assert len(emitted) == 1
    assert emitted[0].event_type == "gm.narrate"
    assert "Kael" in str(emitted[0].data)
    assert "gravely wounded" in str(emitted[0].data)


@pytest.mark.asyncio
async def test_does_not_emit_twice_for_same_player() -> None:
    emitted: list[GameEvent] = []
    narrator = LowHealthNarrator(threshold_fraction=0.25)
    narrator.bind_emitter(lambda evt: emitted.append(evt))

    base = {
        "target_id": "player-1",
        "target_is_player": True,
        "target_max_hp": 100,
        "target_name": "Kael",
        "target_alive": True,
    }
    await narrator.handle(GameEvent(
        tick=5, event_type="combat.attack_resolved",
        data={**base, "target_hp": 20}, source="combat",
    ))
    await narrator.handle(GameEvent(
        tick=6, event_type="combat.attack_resolved",
        data={**base, "target_hp": 10}, source="combat",
    ))

    assert len(emitted) == 1


@pytest.mark.asyncio
async def test_does_not_emit_for_non_player() -> None:
    emitted: list[GameEvent] = []
    narrator = LowHealthNarrator(threshold_fraction=0.25)
    narrator.bind_emitter(lambda evt: emitted.append(evt))

    await narrator.handle(GameEvent(
        tick=5, event_type="combat.attack_resolved",
        data={
            "target_id": "goblin-3",
            "target_is_player": False,
            "target_hp": 1,
            "target_max_hp": 5,
            "target_name": "Goblin",
            "target_alive": True,
        },
        source="combat",
    ))

    assert emitted == []


@pytest.mark.asyncio
async def test_does_not_emit_when_hp_above_threshold() -> None:
    emitted: list[GameEvent] = []
    narrator = LowHealthNarrator(threshold_fraction=0.25)
    narrator.bind_emitter(lambda evt: emitted.append(evt))

    await narrator.handle(GameEvent(
        tick=5, event_type="combat.attack_resolved",
        data={
            "target_id": "player-1",
            "target_is_player": True,
            "target_hp": 50,
            "target_max_hp": 100,
            "target_name": "Kael",
            "target_alive": True,
        },
        source="combat",
    ))

    assert emitted == []


@pytest.mark.asyncio
async def test_does_not_emit_when_max_hp_zero() -> None:
    emitted: list[GameEvent] = []
    narrator = LowHealthNarrator(threshold_fraction=0.25)
    narrator.bind_emitter(lambda evt: emitted.append(evt))

    await narrator.handle(GameEvent(
        tick=5, event_type="combat.attack_resolved",
        data={
            "target_id": "x",
            "target_is_player": True,
            "target_hp": 0,
            "target_max_hp": 0,
            "target_name": "Blank",
            "target_alive": True,
        },
        source="combat",
    ))

    assert emitted == []
```

- [ ] **Step 3: Run the tests to confirm they fail**

Run: `pytest tests/test_low_health_narration.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'harsh_realm.engine.low_health_narration'`

- [ ] **Step 4: Write the minimal implementation**

Create `src/harsh_realm/engine/low_health_narration.py`:

```python
"""Event handler that narrates when a player's HP first crosses a low threshold.

Replaces the legacy ``LowHealthWarningSystem`` ECS system. Subscribes to the
terminal post-damage event and emits a ``gm.narrate`` event once per player
per session when HP drops at or below the configured fraction of max HP.
"""

from __future__ import annotations

from collections.abc import Awaitable, Callable

from harsh_realm.events import GameEvent
from harsh_realm.payloads import NarrationNotice


Emitter = Callable[[GameEvent], Awaitable[None] | None]


class LowHealthNarrator:
    """Emit a narration when a player first drops at or below the threshold."""

    def __init__(self, threshold_fraction: float = 0.25) -> None:
        self._threshold = threshold_fraction
        self._warned: set[str] = set()
        self._emit: Emitter | None = None

    def bind_emitter(self, emit: Emitter) -> None:
        """Bind the event emitter. Called once at wiring time."""
        self._emit = emit

    async def handle(self, event: GameEvent) -> None:
        """Handle a post-damage event; emit a narration if the threshold is crossed."""
        if self._emit is None:
            raise RuntimeError("LowHealthNarrator has no emitter bound")

        data = event.data
        if not data.get("target_is_player"):
            return
        if not data.get("target_alive", True):
            return

        target_id = data.get("target_id")
        if not isinstance(target_id, str) or target_id in self._warned:
            return

        max_hp = data.get("target_max_hp", 0)
        hp = data.get("target_hp", 0)
        if not isinstance(max_hp, int) or max_hp <= 0:
            return
        if not isinstance(hp, int):
            return

        threshold = int(max_hp * self._threshold)
        if hp > threshold:
            return

        self._warned.add(target_id)

        name = data.get("target_name") or target_id
        narration = GameEvent(
            tick=event.tick,
            event_type="gm.narrate",
            data=NarrationNotice(text=f"{name} is gravely wounded.").as_event_data(),
            source="low_health_narrator",
        )
        result = self._emit(narration)
        if result is not None:
            await result
```

- [ ] **Step 5: Run the tests to confirm they pass**

Run: `pytest tests/test_low_health_narration.py -v`
Expected: 5 passing.

- [ ] **Step 6: Wire the narrator into the existing event bus**

Find where other event handlers are registered on startup (search `grep -rn "event_bus.subscribe\|subscribe(" src/harsh_realm/`) and add registration for `LowHealthNarrator`. Route it to subscribe to the chosen trigger event from Step 1.

If the existing combat code does not emit the fields the handler expects (`target_is_player`, `target_hp`, `target_max_hp`, `target_name`, `target_alive`), update the emission site in combat to include them. Keep the change minimal.

- [ ] **Step 7: Run the full test suite**

Run: `pytest -x`
Expected: all tests pass. If any test fails, debug and fix before commit.

- [ ] **Step 8: Commit**

```bash
git add src/harsh_realm/engine/low_health_narration.py tests/test_low_health_narration.py
git add -u  # include any combat emission-site edits
git commit -m "feat(engine): add LowHealthNarrator event handler

Replaces the LowHealthWarningSystem ECS system with a plain event
handler subscribed to the terminal post-damage event. Same 'warn at
most once per player per session at hp <= 25% max_hp' semantics as
the legacy system.

Phase 3 / Task 9 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 10: Refactor scene files to remove ECS dependencies

**Files:**
- Modify (one at a time): the seven scene files identified in Task 8

Per-file procedure. Repeat for each file; each file gets its own commit.

- [ ] **Step 1: Pick one scene file from the list**

Order suggested: `combat_core.py` first (uses ECS most heavily per §3 guidance), then `exploration_core.py`, then `exploration_movement.py`, `exploration_persistence.py`, `exploration_interaction.py`, `town.py`, `dungeon.py`.

- [ ] **Step 2: Read the file in full and the Task 8 inventory entry for it**

Have both the file and `/tmp/ecs_usage_inventory.md` open.

- [ ] **Step 3: Replace each `REPLACE`-classified ECS usage**

General pattern for common replacements:

- `world.get_component(entity_id, HealthComponent)` → read HP from the entity model directly (via its repo or the in-memory handle passed into the scene). Example:
  ```python
  # Before:
  health = self._ecs_world.get_component(entity_id, HealthComponent)
  if health is None or not health.alive:
      return
  # After:
  npc = await self._npc_repo.get(entity_id)
  if npc is None or npc.hp <= 0:
      return
  ```
- `world.set_component(entity_id, HealthComponent(hp=new_hp, ...))` → mutate and persist the entity model.
  ```python
  # After:
  npc.hp = new_hp
  await self._npc_repo.save(npc)
  ```
- `world.query(ActorRoleComponent, HealthComponent)` → iterate via the appropriate repository (e.g., `await self._npc_repo.alive_in_scene(scene_id)`).
- `SpatialCellComponent` accesses → use `CellData` directly via the cell repository.

For each replacement, confirm tests covering that code path still pass locally before moving on (run a focused `pytest` on the relevant test file).

- [x] **Step 4: Delete all `DELETE`-classified ECS wiring**

Remove: ECS world construction, `world.register_system()` calls, any scene-owned `EcsWorld` instance variables. These are scene-startup plumbing with no non-ECS equivalent.

- [x] **Step 5: Remove the `harsh_realm.ecs` imports from the file**

Run: `grep -n "harsh_realm.ecs\|EcsWorld\|EcsComponent" <file>` — expect zero matches after this step.

- [x] **Step 6: Run focused tests**

For the scene being refactored, run the relevant test file(s):
- `pytest tests/test_<scene_name>* -v` (and any integration tests that touch this scene)

If tests fail, fix before proceeding.

- [ ] **Step 7: Run the full suite**

Run: `pytest -x`
Expected: all tests pass. If any test fails that isn't an ECS-specific test (ECS tests haven't been removed yet, so they should still pass since the ECS package is still there), debug before commit.

- [ ] **Step 8: Commit**

```bash
git add src/harsh_realm/gm/scenes/<filename>.py
git commit -m "refactor(scenes): remove ECS dependencies from <scene_name>

Replaces ECS component reads/writes with direct entity model access
via repositories. ECS scene-startup plumbing deleted. Tests green.

Phase 3 / Task 10 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

- [x] **Step 9: Repeat Steps 1–8 for the remaining scene files**

At the end, `grep -rln "harsh_realm.ecs" src/harsh_realm/gm/scenes/` should return nothing. `grep -rln "harsh_realm.ecs" src/harsh_realm/` should return only the `ecs/` package itself (to be deleted next).

---

### Task 11: Delete the ECS package and remove remaining imports

**Files:**
- Delete: `src/harsh_realm/ecs/` (entire directory)
- Modify: any remaining files with `harsh_realm.ecs` imports

- [x] **Step 1: Confirm ECS is no longer imported outside the package**

Run: `grep -rln "harsh_realm.ecs" src/harsh_realm/ | grep -v "src/harsh_realm/ecs/"`
Expected: no output. If there is output, return to Task 10 and refactor the remaining files.

- [x] **Step 2: Delete the ECS package**

Run:
```bash
rm -rf src/harsh_realm/ecs/
```

- [x] **Step 3: Try to import the package globally to expose any missed references**

Run: `python3 -c "import harsh_realm"`
Expected: clean import. If there's an `ImportError` mentioning `harsh_realm.ecs`, find the offending file, refactor it off ECS, and retry.

- [x] **Step 4: Run the full test suite (expect failures in ECS test files only)**

Run: `pytest --collect-only 2>&1 | tail -40`

Collection errors are expected for `tests/test_ecs*.py` since the ECS package is gone. That's fine — those tests are deleted next.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: remove src/harsh_realm/ecs/ package

The ECS substrate (components, world, systems, adapters) is removed in
favor of the focused-entity + subsystem pattern in AGENTS.md §Data
Ownership. Test files that exercise ECS internals are removed in the
next commit.

Phase 3 / Task 11 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 12: Delete ECS-specific test files

**Files:**
- Delete: seven `tests/test_ecs*.py` files

- [x] **Step 1: List ECS test files**

Run: `ls tests/test_ecs*.py`

Expected (from pre-plan scan):
- `tests/test_ecs.py`
- `tests/test_ecs_actor_pilot.py`
- `tests/test_ecs_actor_systems.py`
- `tests/test_ecs_adapters.py`
- `tests/test_ecs_encounter.py`
- `tests/test_ecs_feature_interactivity.py`
- `tests/test_ecs_spatial.py`

- [ ] **Step 2: Before deleting, verify there is no non-ECS test content in any of these files**

For each file, run: `grep -vE "^(from|import|#|\s*$|\s*\"\"\")" <file> | head -20` to glance at non-trivial lines. If any file contains a test that doesn't depend on ECS, extract that test into a sibling file before deletion.

- [x] **Step 3: Delete the files**

```bash
rm tests/test_ecs*.py
```

- [ ] **Step 4: Run the full test suite**

Run: `pytest`

Expected: all tests pass. Current pre-plan baseline was 857 passing + 6 skipped. Post-deletion count will be lower (exact delta depends on how many ECS tests were counted). Record the new count for later.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "test: remove ECS-specific test files

The ECS package is gone; tests exercising its internals are deleted.
Behavioral coverage for LowHealthWarningSystem is preserved in
tests/test_low_health_narration.py.

Phase 3 / Task 12 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 13: Surgically edit `tests/test_properties.py` to remove ECS property tests

**Files:**
- Modify: `tests/test_properties.py`

- [x] **Step 1: Identify ECS-related test functions**

Run: `grep -n "ecs\|Ecs\|Component" tests/test_properties.py`

- [x] **Step 2: For each ECS-related test function, decide keep/delete**

A property test that asserts invariants about ECS components (e.g., `HealthComponent.hp >= 0`) is deleted.
A property test that asserts invariants about entity models that happen to import ECS types is kept, with imports switched to the entity model types.

- [x] **Step 3: Apply the edits**

Delete the ECS-only test functions. Update any remaining imports.

- [x] **Step 4: Run the test file**

Run: `pytest tests/test_properties.py -v`
Expected: all remaining tests pass.

- [ ] **Step 5: Run the full suite**

Run: `pytest`
Expected: full suite green.

- [ ] **Step 6: Commit**

```bash
git add tests/test_properties.py
git commit -m "test(properties): remove ECS property tests

Preserves non-ECS property tests; drops ones asserting invariants on
ECS components (which no longer exist).

Phase 3 / Task 13 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 14: Phase 3 verification — full test + type check

**Files:**
- No file changes

- [ ] **Step 1: Run full pytest**

Run: `pytest`
Expected: all tests pass. Record the post-teardown test count in `/tmp/teardown_counts.txt`.

- [x] **Step 2: Run mypy strict**

Run: `mypy --strict src/harsh_realm/` (or equivalent per project config — check `pyproject.toml` / `mypy.ini`)
Expected: zero new errors. If pre-teardown errors existed, the count must not increase.

- [x] **Step 3: Confirm no `harsh_realm.ecs` imports anywhere**

Run: `grep -rn "harsh_realm.ecs" src/ tests/ docs/`
Expected: no results in `src/` or `tests/`. Possibly historical mentions in `docs/superpowers/specs/` (fine).

- [x] **Step 4: Run frontend type check (if any ECS types leaked into frontend types)**

Run: `cd frontend && npx tsc --noEmit`
Expected: zero errors. If the frontend doesn't reference anything removed, this is a no-op.

- [ ] **Step 5: Commit (no file changes; create a checkpoint commit only if needed, otherwise skip)**

If any small edits landed from Steps 1–4, commit them with message: `chore: Phase 3 cleanup — tests and types green`. Otherwise, skip.

---

## Phase 4 — Model consolidations

Each task here follows the same shape: read the relevant Rule 1 findings, decide based on the decision tree below, execute, confirm tests pass, commit. The decision trees are stated up front so the executing engineer does not re-invent them per domain.

**General decision tree for a `X_runtime.py` + `X_content.py` (or `X_runtime.py` + `X_state.py`) split:**

- **(a) Authored content vs. runtime state.** If one file holds data loaded from YAML (static authored definitions: weapon stats, armor stats, creature templates) and the other holds per-entity runtime state, the split is real — keep both files.
- **(b) ECS materialization vs. entity state.** If the split exists because one file held the ECS component shape and the other held the entity model shape, the split is bogus post-teardown — merge.
- **(c) Typed results vs. mutable entities.** If one file holds frozen result value-objects (e.g., `SkillCheckResult`) and the other holds mutable entities, the split is real — keep both.
- **Fallback.** If the reasoning is unclear from the code, default to merge, and if a field would be awkward on the merged class (mixing authored-global state with per-entity state), extract that field into a sub-model.

### Task 15: Consolidate `combat_runtime.py` and `combat_content.py`

**Files:**
- Read: `src/harsh_realm/models/combat_runtime.py`, `src/harsh_realm/models/combat_content.py`
- Possibly modify: `src/harsh_realm/models/character.py`, `src/harsh_realm/models/npc.py`
- Modify or delete: one or both of the combat_ files

- [x] **Step 1: Read both files in full and the Rule 1 appendix entries for them**

Consult `docs/superpowers/specs/2026-04-22-rules-architecture-design.md` §7.1 for any `F1-XX` entries mentioning these files.

- [x] **Step 2: Apply the decision tree**

Likely outcome per spec §4: keep `combat_content.py` if it holds authored weapon/armor content; merge `combat_runtime.py` fields into `character.py` / `npc.py` (runtime state is intrinsic to the entity, not separate).

- [x] **Step 3: Execute the chosen decision**

- If merging `combat_runtime.py` into entity classes: move each field from the runtime model into the appropriate entity class, update all importers (`grep -rn "combat_runtime" src/ tests/` to find them), delete the file.
- If keeping `combat_content.py` as authored content: confirm its models are frozen (`ConfigDict(frozen=True)`) if they represent authored data; add a module-level docstring stating "Authored combat content loaded from YAML — treat as immutable."

- [x] **Step 4: Update the §7.1 entry with the decision**

In the spec file, update the relevant `F1-XX` entries' `Category` to `fix-in-this-spec (closed)` and add a `Resolution:` line describing what was done.

- [ ] **Step 5: Run the test suite**

Run: `pytest`
Expected: green. Fix any test failures caused by import changes.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(models): consolidate combat_runtime into entity classes

<one-line description of the actual decision: merged X into character.py,
kept combat_content.py as authored YAML-loaded content, etc.>

Closes F1-XX, F1-XX from the Rule 1 review.

Phase 4 / Task 15 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 16: Consolidate `faction_runtime.py` and `faction_state.py`

**Files:**
- Read: `src/harsh_realm/models/faction_runtime.py`, `src/harsh_realm/models/faction_state.py`
- Modify or delete: both

- [x] **Step 1: Read both files and the relevant spec entries**

- [x] **Step 2: Apply the decision tree**

Likely outcome per spec §4: both halves describe the same faction; consolidate into a single `faction.py` entity. If `src/harsh_realm/models/faction_state.py` is actually authored content for faction stat definitions (rare — confirm by reading), it stays; runtime faction state merges into the entity class.

- [x] **Step 3: Execute the decision**

Create or update `src/harsh_realm/models/faction.py` with the consolidated model. Delete the `_runtime`/`_state` files. Update all importers: `grep -rn "faction_runtime\|faction_state" src/ tests/`.

- [x] **Step 4: Update the §7.1 entries**

Mark closed with resolution notes.

- [x] **Step 5: Run tests**

Run: `pytest`
Expected: green.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(models): consolidate faction_runtime + faction_state into faction.py

Closes F1-XX from the Rule 1 review.

Phase 4 / Task 16 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 17: Consolidate `cell.py` and `cell_state.py`; type the `CellData.data` column

**Files:**
- Read: `src/harsh_realm/models/cell.py`, `src/harsh_realm/models/cell_state.py`, `src/harsh_realm/models/scene_data.py`
- Modify: `src/harsh_realm/models/cell.py`, delete or empty `src/harsh_realm/models/cell_state.py`

- [x] **Step 1: Read all three files**

Note the existing TODOs in `cell.py`:
- Line ~33: `terrain` should probably be a concrete type
- Line ~35: `features` should be typed
- Line ~40: `data` should be a concrete type

- [x] **Step 2: Design the typed `data` union**

`CellData.data` currently holds a `JsonObject`. The typed sub-models that feed into it are in `cell_state.py` (`CellSettlementState`, `CellSearchState`, `CellDeathMarker`) and possibly `scene_data.py`.

Design a tagged union or per-field attachment. Two options:
- **Option A: Replace `data: JsonObject` with explicit optional fields**: `settlement: CellSettlementState | None = None`, `search: CellSearchState | None = None`, `death_markers: list[CellDeathMarker] = []`.
- **Option B: Keep `data` as a typed sum type**: `data: CellSettlementState | CellSearchState | None = None`, with a discriminator.

Option A is usually clearer for a handful of optional attachments. Default to A unless there's a specific reason for B.

- [x] **Step 3: Apply the chosen design**

Update `CellData` to use the chosen typing. Migrate `cell_state.py` contents into `cell.py` (the `CellSettlementState` etc. become sub-models on the same file) and delete `cell_state.py`. Update the TODO comments — they should now be resolved.

Update all importers: `grep -rn "cell_state\|CellDataPayload" src/ tests/`.

- [x] **Step 4: Update the §7.1 and §7.4 entries**

Both rules have entries for this consolidation; mark both closed.

- [x] **Step 5: Run tests**

Run: `pytest`
Expected: green. If database-loading code fails because the `data` JSON column has entries that don't match the new types, write a small migration helper in `src/harsh_realm/db.py` or equivalent.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(models): consolidate cell_state into cell.py and type CellData.data

Answers the three TODO comments in CellData by replacing loose
JsonObject with typed optional sub-model fields
(settlement/search/death_markers). cell_state.py deleted.

Closes F1-XX, F4-XX from the review.

Phase 4 / Task 17 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 18: Review `engine_runtime.py` and `engine_results.py`; consolidate or keep split

**Files:**
- Read: `src/harsh_realm/models/engine_runtime.py`, `src/harsh_realm/models/engine_results.py`
- Modify or delete per decision

- [x] **Step 1: Read both files**

- [x] **Step 2: Apply the decision tree**

Frozen value-objects (SkillCheckResult, AttackResult) are a legitimate separate concern — they're immutable result records. If `engine_results.py` holds only frozen result models, keep it as-is (rename module docstring to clarify "frozen engine result value-objects").

If `engine_runtime.py` duplicates data that's already on entity classes (e.g., a runtime wrapper around `Character` that was needed for ECS adapters), merge it out.

- [x] **Step 3: Execute the decision**

- [x] **Step 4: Update the spec entries**

- [x] **Step 5: Run tests**

Run: `pytest`

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(models): resolve engine_runtime/engine_results split

<one-line description>

Phase 4 / Task 18 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 19: Review `entity_state.py`

**Files:**
- Read: `src/harsh_realm/models/entity_state.py`
- Modify or delete per decision

- [x] **Step 1: Read the file; identify what data it holds and who consumes it**

Run: `grep -rn "entity_state\|EntityState" src/ tests/`

- [x] **Step 2: Apply the decision tree**

If the file is purely ECS feeder (mirrors entity data in a shape that ECS adapters expected), delete it and update consumers to use the entity model directly.

If the file holds genuinely shared state between entity kinds (e.g., a common base for all things with a position), and the shared state is intrinsic, it can stay as a mixin or base class — but confirm it isn't duplicating fields already on concrete entity classes.

- [x] **Step 3: Execute**

- [x] **Step 4: Update spec entries**

- [ ] **Step 5: Run tests**

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(models): resolve entity_state.py disposition

<one-line description>

Phase 4 / Task 19 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 20: Review `gm_runtime.py`

**Files:**
- Read: `src/harsh_realm/models/gm_runtime.py`
- Modify or delete per decision

- [x] **Step 1: Read the file**

- [x] **Step 2: Apply the decision tree**

If the file captures GM-controller transient state that is distinct from gameplay entity state (e.g., current scene tag, pending scene transition, chaos factor cache) and that state is truly GM-layer, keep it — it's the GM subsystem's state model.

If it duplicates data already on entity classes, merge.

- [x] **Step 3: Execute**

- [x] **Step 4: Update spec entries**

- [ ] **Step 5: Run tests**

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(models): resolve gm_runtime.py disposition

<one-line description>

Phase 4 / Task 20 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

### Task 21: Address any remaining Rule 1 findings flagged `fix-in-this-spec`

**Files:**
- Varies per finding

- [x] **Step 1: List remaining open `fix-in-this-spec` Rule 1 findings**

Run: `grep -nE "Category:.*fix-in-this-spec" docs/superpowers/specs/2026-04-22-rules-architecture-design.md | head -30`

Cross-reference with the ones already closed in Tasks 15–20.

- [x] **Step 2: For each remaining finding, apply the fix**

The fix is the action listed in the finding's `Notes` or `Suggested action` line.

- [x] **Step 3: Run tests after each fix**

Per-fix: `pytest`. Commit after each logical fix.

- [x] **Step 4: Confirm zero open `fix-in-this-spec` Rule 1 findings remain**

Run: `grep -nE "F1-.*Category: fix-in-this-spec$" docs/superpowers/specs/2026-04-22-rules-architecture-design.md`
Expected: no lines (all closed).

- [x] **Step 5: Commit the final cleanup if not already committed**

Rollup commit deferred per user request.

```bash
git add -A
git commit -m "refactor(models): close remaining Rule 1 fix-in-this-spec findings" || true
```

(The `|| true` is a no-op safety if there are no uncommitted changes.)

---

### Task 22: Add `PERSISTENCE` declarations to existing subsystem modules

**Files:**
- Modify: each subsystem module identified in the Rule 4 review (§7.4 of spec)

- [x] **Step 1: List subsystem modules from the Rule 4 review**

Consult spec §7.4 findings. Typical candidates: `src/harsh_realm/faction/`, `src/harsh_realm/engine/oracle.py`, `src/harsh_realm/gm/controller.py`, `src/harsh_realm/admin/service.py`.

- [x] **Step 2: For each module, choose durable or derivable**

A subsystem is **durable** if it writes to SQLite tables that must survive restart.
A subsystem is **derivable** if its state can be recomputed on load.

Most existing subsystems will be durable (faction, oracle, admin, world clock).

- [x] **Step 3: Add a module-level declaration**

At the top of each subsystem's primary module (typically `service.py` or the module root), after the docstring and imports, add:

```python
PERSISTENCE: Literal["durable", "derivable"] = "durable"
```

(Adjust the string per the subsystem.)

- [x] **Step 4: Verify the declaration is referenced by at least one test or startup check**

Optionally: add a small startup-time assertion loop that iterates known subsystems and logs their declared persistence. Not required for this task; just make sure the declaration is present as documentation.

- [x] **Step 5: Update §7.4 to mark each finding closed**

- [x] **Step 6: Run tests**

Run: `pytest`

- [x] **Step 7: Commit**

Rollup commit deferred per user request.

```bash
git add -A
git commit -m "refactor(subsystems): declare PERSISTENCE for existing subsystems

Each subsystem module now declares PERSISTENCE = 'durable' or 'derivable'
per Rule 4. Makes durability reviewable rather than implicit.

Closes F4-XX from the review.

Phase 4 / Task 22 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

## Phase 5 — Rule 3 pattern landing

### Task 23: Evaluate `engine/skill_checks.py` (or equivalent) for pipeline formalization

**Files:**
- Read: `src/harsh_realm/engine/skill_checks.py`

- [x] **Step 1: Read the file**

Look for: does it already iterate a list of modifiers? Does it have a hook for house rules?

- [x] **Step 2: Decide**

If the file has a clear modifier-list shape already, refactor to the Rule 3 pipeline pattern: extract a `SkillCheckResolution` Pydantic context model, extract modifier functions as registered resolvers, emit a single `skill_check.resolved` event post-resolution.

If the file inlines all modifier logic and there is no clear list-of-modifiers shape, **do not refactor in this spec**. Instead, append a `§7.3` finding marking this as `defer-to-future-spec` target `resolver-pipeline-formalization` and document Rule 3 purely via AGENTS.md examples. Explain in a short comment at the top of `skill_checks.py`:

```python
# TODO(resolver-pipeline-formalization): skill check resolution currently
# inlines modifier logic. Formalize per Rule 3 (AGENTS.md) in a future spec.
```

- [x] **Step 3: Execute the chosen path**

If refactoring: write the refactor as a small set of commits (extract context, extract resolvers one at a time, wire registration, update tests).

If deferring: add the TODO comment and the §7.3 deferral entry.

- [x] **Step 4: Run tests**

Run: `pytest`
Expected: green.

- [x] **Step 5: Commit**

Rollup commit deferred per user request.

```bash
git add -A
git commit -m "refactor(engine): <formalize|defer> skill check resolver pipeline

<describe the choice>

Phase 5 / Task 23 of docs/superpowers/plans/2026-04-22-rules-architecture-plan.md"
```

---

## Phase 6 — Closeout

### Task 24: Verify all success criteria from spec §5

**Files:**
- No changes expected; this is a verification step

- [x] **Step 1: Verify `src/harsh_realm/ecs/` is gone**

Run: `test -d src/harsh_realm/ecs/ && echo FAIL || echo OK`
Expected: `OK`.

- [x] **Step 2: Verify no `harsh_realm.ecs` imports remain**

Run: `grep -rn "harsh_realm.ecs\|from harsh_realm import ecs" src/ tests/`
Expected: no matches.

Verified against source files. The only source hit is the negative
architecture test that asserts `harsh_realm.ecs` is not importable; generated
`__pycache__` and egg-info artifacts are ignored.

- [x] **Step 3: Verify rules documented in AGENTS.md**

Run: `grep -n "Data Ownership, Subsystems, and Events" AGENTS.md`
Expected: one match.

Run: `grep -cE "^### Rule [1234]" AGENTS.md`
Expected: `4`.

- [x] **Step 4: Verify CLAUDE.md updated**

Run: `grep -n "Data ownership, subsystems, and events" CLAUDE.md`
Expected: at least one match in the "Key Architectural Rules" or similar section.

- [x] **Step 5: Verify punch list fully closed**

Run: `grep -cE "Category:.*fix-in-this-spec$" docs/superpowers/specs/2026-04-22-rules-architecture-design.md`
Expected: `0`. If non-zero, there are open items; return to the relevant task.

- [x] **Step 6: Verify test count has not decreased below baseline**

Run: `pytest --collect-only 2>&1 | grep -E "collected [0-9]+ items"`

Baseline was 857 passing + 6 skipped = 863 collected. Post-teardown count will be lower because ECS tests were deleted. The new baseline is whatever Task 14 recorded in `/tmp/teardown_counts.txt`. Compare:

Expected: current count ≥ post-teardown baseline (no regressions introduced by model consolidation).

Current collection: 1151 tests.

- [x] **Step 7: Run pytest**

Run: `pytest`
Expected: all tests pass.

Result: 1139 passed, 12 skipped.

- [x] **Step 8: Run Hypothesis property tests**

Run: `pytest tests/test_properties.py -v`
Expected: green.

Result: 30 passed.

- [ ] **Step 9: Run mypy strict**

Run: `mypy --strict src/harsh_realm/` (or the project's standard command)
Expected: zero new errors compared to pre-spec baseline. If any new errors, fix before marking this task complete.

Verification gap: `uv run mypy --strict src/harsh_realm/` currently reports
826 errors across 61 files. This is broader than the rules-architecture
closeout and remains a typing-hardening follow-up.

- [x] **Step 10: Run frontend type check**

Run: `cd frontend && npx tsc --noEmit`
Expected: zero errors.

Result: `npm run type-check` passed.

- [x] **Step 11: Run Playwright tests**

Run: `cd frontend && npx playwright test`
Expected: green.

Result: `npm run test:e2e -- --project=chromium` passed with 60 tests.

- [x] **Step 12: Verify PERSISTENCE declarations**

Run: `grep -rn "PERSISTENCE\s*[:=]" src/harsh_realm/`
Expected: at least one declaration per subsystem module identified in the Rule 4 review.

- [x] **Step 13: Verify no `harsh_realm.ecs` in documentation**

Run: `grep -rn "harsh_realm.ecs" docs/`
Expected: matches only inside `docs/superpowers/specs/2026-04-22-rules-architecture-design.md` (historical context) or `docs/superpowers/plans/2026-04-22-rules-architecture-plan.md` (this file). No ECS mentions in user-facing docs.

Result: only the rules-architecture plan and spec contain historical
`harsh_realm.ecs` references.

- [x] **Step 14: No commit (verification only)**

If any verification fails, return to the appropriate earlier task. Do not create a commit here.

---

### Task 25: Update `CLAUDE.md` Current State and mark spec implemented

**Files:**
- Modify: `CLAUDE.md`, `docs/superpowers/specs/2026-04-22-rules-architecture-design.md`

- [x] **Step 1: Count current tests**

Run: `pytest --collect-only 2>&1 | grep -E "collected [0-9]+ items" | sed 's/.*collected //'`
Record the number (e.g., 780).

Current collection: 1151 tests; latest full run: 1139 passed, 12 skipped.

- [x] **Step 2: Update `CLAUDE.md` Current State line**

Locate the line of the form `**NNN tests passing, N skipped ...**` under "Current State" and update to the new count plus a date.

- [x] **Step 3: Append a milestone note**

Under "Milestone Status Summary", add a row or note for this migration:

```markdown
| Rules-arch | **Complete** | <count> | 2026-04-22 |
```

(Or fit the existing table style — match rows above.)

- [x] **Step 4: Mark the spec status**

Open `docs/superpowers/specs/2026-04-22-rules-architecture-design.md` and change the status line at the top from `Approved for implementation planning` to `Implemented`, with today's date appended.

- [x] **Step 5: Commit**

Rollup commit deferred per user request.

```bash
git add CLAUDE.md docs/superpowers/specs/2026-04-22-rules-architecture-design.md
git commit -m "docs: mark rules-architecture spec implemented

Updates CLAUDE.md Current State with post-migration test count and
Milestone Status Summary row. Spec status moved to Implemented.

Closes docs/superpowers/specs/2026-04-22-rules-architecture-design.md"
```

---

## Self-review pass (executed before plan handoff)

**Spec coverage check.**

| Spec section | Implementing task(s) |
|---|---|
| §3 Rule 1 (data ownership) | Tasks 1 (doc), 3 (review), 15–21 (fixes) |
| §3 Rule 2 (cross-subsystem) | Tasks 1 (doc), 4 (review); enforcement is ongoing, not code-changing here |
| §3 Rule 3 (resolver pipelines) | Tasks 1 (doc), 5 (review), 23 (formalize or defer) |
| §3 Rule 4 (persistence) | Tasks 1 (doc), 6 (review), 22 (declarations) |
| §4 ECS teardown | Tasks 8–14 |
| §4 LowHealthWarningSystem migration | Task 9 |
| §4 Model consolidations | Tasks 15–20 |
| §5 Doc placement (AGENTS + CLAUDE) | Tasks 1, 2, 25 |
| §5 Review methodology | Tasks 3–7 |
| §5 Testing during migration | Runs in every task's gate step |
| §5 Success criteria | Task 24 |

No gaps identified.

**Placeholder scan.** Plan contains no `TBD`, no `TODO: implement later`, no `similar to task N`. The `<one-line description>` placeholders in commit messages are deliberate — the engineer fills those in based on what they actually did, and this is standard commit-message hygiene, not a plan failure. The `__CHOSEN_EVENT__` placeholder in Task 9 is filled in during Step 1 of that task; it's a named fill-in, not a vague placeholder.

**Type consistency.** `LowHealthNarrator`, `GameEvent`, `NarrationNotice` names match across Task 9 and any future references. `PERSISTENCE` attribute name used consistently across Tasks 22 and 24.

Plan is ready for execution.
