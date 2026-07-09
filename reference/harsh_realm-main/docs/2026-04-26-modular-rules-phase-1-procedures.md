# Modular Rules Architecture — Phase 1: Procedures and Status Effects

**Date:** 2026-04-26
**Status:** Draft
**Cycle:** Modular Rules Architecture
**Overview:** `2026-04-26-modular-rules-architecture-overview.md`
**Depends on:** Phase 0 complete
**Prerequisites for tasks:** Read the overview spec first. Read Phase 0 spec for pack architecture context. Read `AGENTS.md` for coding standards.

---

## 1. Phase scope

Phase 1 adds two mechanic frameworks on top of the pack infrastructure built in Phase 0:

- **Generator/procedure framework** — a declarative way to compose multi-step content generation: roll on table, invoke another procedure, run a registered Python function, format a result string. Generalizes the hardcoded UNE personality generator and unlocks Wickham-style generator content as data.
- **Status effect service** — a typed, duration-tracking subsystem for "things that are happening *to* an entity" (poisoned, blessed, bleeding, fatigued). Fits Rule 1 cleanly as an extrinsic subsystem. Phase 1 ships application, persistence, expiration, and event integration; the *modifier* and *triggered* aspects of status effects come in Phases 2 and 3.

Phase 1 is intentionally smaller than Phase 0. These are quick wins that prove the pack infrastructure works for non-trivial content and unlock immediate authoring value (the Wickham library, future status effect content from any source).

### What Phase 1 produces

- Procedure schema and runner. Procedures are pack content under `packs/<pack-id>/content/procedures/`.
- Compute-step registration: code-bearing packs can register Python callables that procedures invoke by name.
- UNE personality generation re-implemented as a procedure (the existing hardcoded `UNEGenerator` is removed; its behavior is preserved by an equivalent procedure in `xwn-core`).
- One Wickham generator imported as a `wickham-tables` pack (or as content within `xwn-core` for now — see Task 1.8).
- Status effect schema (pack content) and service (engine subsystem).
- Status effect persistence (per-entity, durable per Rule 4).
- Status effect lifecycle: apply, expire on tick, remove explicitly. All transitions go through events.
- One status effect imported as a content record (a representative effect like "Poisoned") that demonstrates application and expiration end-to-end.
- Frontend: status effects visible on the character sidebar.

### What Phase 1 does not do

- **No modifier integration for status effects.** Status effects in Phase 1 cannot say "+1 to attack while active" — that requires the modifier framework from Phase 2. Phase 1 status effects exist, persist, and emit events; they don't change combat math.
- **No triggered effects.** Status effects cannot say "deal 1 damage per tick" declaratively in YAML — that's the trigger/effect engine in Phase 3. If a Phase 1 status effect needs active behavior, it does so via Python code in `xwn-core`'s code surface (a code-bearing handler that subscribes to status events).
- **No conditional procedure steps.** Procedure flow is linear in Phase 1. Conditions and branching are deferred to Phase 3 (the same DSL primitives serve both procedures and triggers).
- **No migration of `engine/oracle.py` Mythic procedures.** The Mythic GME oracle stays as Python code for now. A future cycle can migrate it once the procedure framework is proven in simpler contexts.

## 2. Decisions locked in this phase

- **Procedure storage:** YAML records under `packs/<pack-id>/content/procedures/`. Each procedure is one record per file or grouped in a multi-record file under `procedures/<group>.yaml`.
- **Procedure step types in Phase 1:** `roll`, `compute`, `procedure`, `format`. Conditional steps deferred.
- **Compute-step registration:** code-bearing packs register callables in their `register(app_state)` function. Callables are invoked by namespaced name (e.g., `xwn-core:disposition_from_chaos`).
- **Status effect storage:** new SQLite table `entity_status_effects` (per-world, durable). Owned by `StatusEffectService` per AGENTS.md Rule 4.
- **Status effect duration unit:** ticks (the existing world clock unit). A duration of `0` means permanent until explicitly removed; positive integers expire on the corresponding tick.
- **Status effect content schema:** id, name, description, default_duration_ticks, tags, optional icon. No mechanical fields in Phase 1; those are added in Phases 2 and 3.

## 3. Tasks

Test layer notation: **[U]** pytest unit, **[P]** Hypothesis property, **[M]** mutmut mutation, **[E2E]** Playwright, **[V]** Vitest unit, **[FC]** fast-check property, **[S]** Stryker mutation.

---

### Task 1.1 — Codebase audit for procedures and status-effect-shaped state

**Points:** 1
**Dependencies:** Phase 0 complete
**Test layers:** none (investigation)

**What:** Examine the current state of the codebase to inventory what becomes procedure content and what status-effect-shaped state already exists in ad-hoc form.

**Procedure:**
1. Walk `src/harsh_realm/engine/npc_personality.py` and document the exact behavior of `UNEGenerator.generate_personality`, `generate_motivation`, `generate_bearing`. Note all table IDs it rolls on, all conditional logic, all output fields.
2. Walk `src/harsh_realm/engine/tables.py` (existing `TableEngine`) and document the current `generate(generator_id, params)` API. Determine whether this is the foundation Phase 1 builds on or whether it should be replaced.
3. Walk `src/harsh_realm/generators/` (npc_gen, settlement_gen, etc.) and identify any other multi-step generation flows that should eventually become procedures. Phase 1 doesn't migrate them; the audit just lists them for future cycles.
4. Search for any existing "this entity has a temporary condition" patterns: status effect-shaped fields on entities, ad-hoc `data["effects"]` JSON entries, code that decrements counters on tick. Document what you find — these are candidates for refactoring once the status effect service exists.
5. Identify the current world clock tick mechanism (where does it advance? which subsystems get notified?). The status effect service hooks into this for expiration.
6. Review `engine/oracle.py` and confirm: is anything there shaped like a procedure (oracle scene check, fate roll, event focus chain)? List them as future-cycle migration targets.

**Deliverable:** `docs/superpowers/specs/2026-04-26-phase-1-codebase-audit.md` — markdown report with the above six sections.

**Acceptance:** Audit document exists, is concrete enough that subsequent tasks can reference it without re-reading source.

---

### Task 1.2 — Procedure content schema

**Points:** 2
**Dependencies:** 1.1
**Test layers:** [U] [P]

**What:** Pydantic models for a procedure record and its step types.

**File:** `src/harsh_realm/procedures/schema.py` (new)

**Models:**

```python
class Procedure(BaseModel):
    """A multi-step procedure record."""
    model_config = ConfigDict(frozen=True)

    id: str
    name: str
    description: str = ""
    inputs: list[ProcedureInput] = Field(default_factory=list)
    steps: list[ProcedureStep]
    output: ProcedureOutput | None = None
    tags: list[str] = Field(default_factory=list)


class ProcedureInput(BaseModel):
    """A named input parameter the procedure accepts."""
    model_config = ConfigDict(frozen=True)
    name: str
    type: Literal["string", "integer", "boolean", "any"] = "any"
    required: bool = True
    default: str | int | bool | None = None


class ProcedureStep(BaseModel):
    """Discriminated union of step types."""
    model_config = ConfigDict(frozen=True)
    kind: Literal["roll", "compute", "procedure", "format"]
    assign: str = Field(description="Output variable name")
    # Step-specific fields (one of):
    table: str | None = None              # for kind="roll" — qualified table ID
    function: str | None = None           # for kind="compute" — qualified function name
    procedure: str | None = None          # for kind="procedure" — qualified procedure ID
    template: str | None = None           # for kind="format" — Python format string
    params: dict[str, str] = Field(default_factory=dict)
    count: int = 1                        # repeat the step this many times (collects into a list)


class ProcedureOutput(BaseModel):
    """Optional explicit output shape."""
    model_config = ConfigDict(frozen=True)
    fields: dict[str, str]   # output_name → variable_name from steps
```

**Validation rules:**
- A `ProcedureStep` of `kind="roll"` requires `table`. `kind="compute"` requires `function`. Etc.
- `params` values are template strings that may reference prior step outputs as `{var_name}`.
- Step `assign` names are unique within a procedure.

Add a `model_validator(mode="after")` to enforce the `kind`-to-required-field rule.

**Tests:** `tests/procedures/test_schema.py`
- Valid procedure record with two roll steps and one format step parses.
- Step missing required field for its kind raises `ValidationError`.
- Duplicate `assign` names raise `ValidationError`.
- Property test: every parsed procedure has exactly one of `table`/`function`/`procedure`/`template` set per step.

**Acceptance:** Tests pass.

---

### Task 1.3 — Procedure runner

**Points:** 3
**Dependencies:** 1.2, Phase 0 ContentService (Task 0.15)
**Test layers:** [U] [P]

**What:** A runtime that executes a `Procedure` against an input dict and returns an output dict.

**File:** `src/harsh_realm/procedures/runner.py` (new)

**API:**
```python
class ProcedureRunner:
    def __init__(
        self,
        content: ContentService,
        tables: TableEngine,
        compute_registry: ComputeRegistry,
    ) -> None: ...

    async def run(
        self,
        procedure_id: str,
        inputs: dict[str, ProcedureValue] | None = None,
    ) -> dict[str, ProcedureValue]: ...
```

**Execution semantics:**
1. Resolve the procedure by qualified ID via `ContentService`.
2. Validate inputs against `procedure.inputs`. Apply defaults for missing optional inputs. Raise `ProcedureValidationError` if a required input is missing or a type mismatch occurs.
3. Initialize a variable map from the inputs.
4. Execute steps in order. For each step:
   - Resolve template parameters in `params` against the variable map (substitute `{var}` placeholders).
   - **`roll`:** Call `tables.roll_on(step.table, params=resolved_params)`. Result goes into `vars[step.assign]`.
   - **`compute`:** Call `compute_registry.invoke(step.function, resolved_params)`. Result goes into `vars[step.assign]`.
   - **`procedure`:** Call `self.run(step.procedure, inputs=resolved_params)`. The full output dict goes into `vars[step.assign]`.
   - **`format`:** Render `step.template` against the variable map. The string goes into `vars[step.assign]`.
   - If `count > 1`, the step runs `count` times and `vars[step.assign]` is a list.
5. Build the output dict: if `procedure.output.fields` is set, output is the named projection; otherwise output is the full variable map (excluding inputs).

**Errors:**
- Missing template variable → `ProcedureExecutionError` naming the variable.
- Compute function not registered → `ProcedureExecutionError`.
- Sub-procedure recursion depth > 10 → `ProcedureExecutionError`.

**Tests:** `tests/procedures/test_runner.py`
- A procedure with one roll step returns the roll result.
- A procedure with format step and prior var: format renders correctly.
- A procedure invoking a sub-procedure passes inputs and receives outputs.
- A procedure invoking a compute function: registry returns expected value.
- Missing required input raises `ProcedureValidationError`.
- Recursion depth 11 raises `ProcedureExecutionError`.
- Property test: for any procedure with all steps assigning unique vars and using only existing tables/functions, `run` produces a dict containing every assigned variable.

**Acceptance:** Tests pass.

---

### Task 1.4 — Compute registry

**Points:** 1
**Dependencies:** Phase 0 code-bearing pack hook (Task 0.20)
**Test layers:** [U]

**What:** Registry that holds named Python callables registered by code-bearing packs.

**File:** `src/harsh_realm/procedures/compute_registry.py` (new)

**API:**
```python
class ComputeRegistry:
    def register(self, qualified_name: str, fn: Callable[..., ProcedureValue]) -> None: ...
    async def invoke(self, qualified_name: str, params: dict[str, ProcedureValue]) -> ProcedureValue: ...
    def list_registered(self) -> list[str]: ...
```

`qualified_name` is `<pack-id>:<function-name>`, e.g., `xwn-core:disposition_from_chaos`. Functions can be sync or async; `invoke` awaits if needed.

**Tests:** `tests/procedures/test_compute_registry.py`
- Register a sync function, invoke it, get expected result.
- Register an async function, invoke it, get expected result.
- Invoking unregistered name raises `ComputeNotFoundError`.
- Registering same name twice raises `ComputeAlreadyRegisteredError`.

**Acceptance:** Tests pass.

---

### Task 1.5 — Wire procedure framework into app state

**Points:** 1
**Dependencies:** 1.3, 1.4
**Test layers:** [U]

**What:** Construct `ProcedureRunner` and `ComputeRegistry` at world load and attach to app state. Code-bearing pack `register` hooks (from Phase 0 Task 0.20) gain access to the registry.

**Files:**
- `src/harsh_realm/main.py` (extend)
- `src/harsh_realm/api/routes.py` (world load extension)

**Behavior:**
- On world load, instantiate `ComputeRegistry`, then call each loaded pack's `register(app_state)` hook (already wired in Phase 0). The hook can register compute functions before the procedure runner runs.
- Then instantiate `ProcedureRunner(content, tables, compute_registry)` and attach to `app.state.procedure_runner`.
- On world unload, clear `app.state.procedure_runner`.

**Tests:**
- World load attaches a procedure runner to app state.
- Unload clears it.
- A test pack registering a compute function makes it invocable through the runner.

**Acceptance:** Tests pass.

---

### Task 1.6 — Migrate UNE personality generation to a procedure

**Points:** 3
**Dependencies:** 1.5, Task 1.1 audit
**Test layers:** [U] [P]

**What:** Replace `engine/npc_personality.py`'s `UNEGenerator` with a YAML procedure in `xwn-core`. The Python class either becomes a thin facade that calls the procedure runner, or is removed entirely with callers updated to invoke the procedure directly.

**Files:**
- `packs/xwn-core/content/procedures/une_personality.yaml` (new)
- `packs/xwn-core/content/procedures/une_motivation.yaml` (new)
- `packs/xwn-core/content/procedures/une_bearing.yaml` (new)
- `packs/xwn-core/code/__init__.py` (extend `register()` to register any compute functions UNE needs)
- `src/harsh_realm/engine/npc_personality.py` (replace internals or delete)
- callers updated

**Procedure shape (sketch):**

```yaml
# packs/xwn-core/content/procedures/une_personality.yaml
id: une_personality
name: UNE Personality Generator
description: Generates a complete UNE personality block for an NPC.
inputs:
  - name: power_level
    type: string
    required: false
  - name: chaos_factor
    type: integer
    required: false
    default: 5
  - name: relationship
    type: string
    required: false
    default: peer
steps:
  - kind: roll
    assign: power_level_rolled
    table: xwn-core:tables.une_power_level
    params:
      preset: "{power_level}"
  - kind: roll
    assign: descriptor
    table: xwn-core:tables.une_descriptors
  - kind: procedure
    assign: motivation
    procedure: xwn-core:procedures.une_motivation
  - kind: procedure
    assign: bearing
    procedure: xwn-core:procedures.une_bearing
    params:
      chaos_factor: "{chaos_factor}"
      relationship: "{relationship}"
  - kind: compute
    assign: base_disposition
    function: xwn-core:disposition_from_chaos
    params:
      chaos_factor: "{chaos_factor}"
output:
  fields:
    power_level: power_level_rolled
    descriptor: descriptor
    motivation_verb: motivation.verb
    motivation_noun: motivation.noun
    bearing: bearing.bearing
    bearing_focus: bearing.focus
    base_disposition: base_disposition
```

The single compute function `xwn-core:disposition_from_chaos` is registered in `xwn-core`'s `register()` hook. It encapsulates the small bit of logic that wasn't a table roll.

**Note on table ID format:** the `table:` field uses a qualified content ID. Existing tables loaded by `TableEngine` need to be addressable by qualified ID. If they aren't already (the audit will tell), Task 1.1 flags it and a small adapter step in this task makes them so. Conventionally, tables in YAML files at `packs/xwn-core/content/tables/npc/une_descriptors.yaml` resolve to `xwn-core:tables.une_descriptors` (the dotted slug reflecting the directory).

**Tests:** `tests/procedures/test_une_personality_procedure.py`
- Run the procedure 100 times → all results have all expected output fields with values within table bounds.
- Property test: power_level, descriptor, motivation_verb, motivation_noun, bearing fields are always non-empty.
- Behavioral parity: the new procedure-based generator produces NPCs in the same shape as the old `UNEGenerator` (same field names; existing callers in the social scene continue to work without modification).

Existing tests in `tests/test_npc_personality.py` should continue to pass; if they fail because they tested implementation details of `UNEGenerator` (rather than behavior), update them to test the procedure instead.

**Acceptance:** UNE NPCs generated through the new procedure work end-to-end in the social scene. The old `UNEGenerator` class is either deleted or thinly wraps the runner.

---

### Task 1.7 — Procedure CRUD admin endpoints

**Points:** 2
**Dependencies:** 1.5, Phase 0 override routes (Task 0.17)
**Test layers:** [U]

**What:** REST endpoints for the admin UI to view and edit procedure records (with override semantics from Phase 0).

**File:** `src/harsh_realm/api/admin_routes.py` (extend)

**Endpoints:**
- `GET /api/world/procedures` → list all procedure records (with `_overridden` flags).
- `GET /api/world/procedures/<qualified_id>` → single procedure record.
- `PUT /api/world/procedures/<qualified_id>` body `{"data": {...}}` → upsert override.
- `DELETE /api/world/procedures/<qualified_id>` → revert to pack default.
- `POST /api/world/procedures/<qualified_id>/run` body `{"inputs": {...}}` → execute the procedure and return output. *Admin-only; gated by `config.admin_mode`*. This is for testing procedures, not gameplay.

**Tests:** `tests/procedures/test_procedure_routes.py`
- List returns all procedures from `xwn-core`.
- GET returns the UNE personality procedure.
- Override and revert round-trip.
- POST run returns valid output for UNE personality.
- POST run when `admin_mode = false` returns 403.

**Acceptance:** Tests pass.

---

### Task 1.8 — Wickham generator imported as a procedure

**Points:** 2
**Dependencies:** 1.6
**Test layers:** [U]

**What:** Phase 1's "real-world content" test case. Pick one Wickham generator (recommended: a settlement/town name + character generator from Grammar Fuel — small, self-contained, exercises rolls and formatting) and import it as a procedure record, plus its supporting tables.

**Decision to make during this task:** does the Wickham content go into:
- (a) `packs/xwn-core/content/procedures/` and `packs/xwn-core/content/tables/wickham/` — i.e., extends `xwn-core` directly, or
- (b) A new pack `packs/wickham-tables/` with its own manifest that depends on `xwn-core`.

Recommendation: (b). It proves the multi-pack model works. The `wickham-tables` pack's manifest declares `depends: [xwn-core@>=1.0.0]`; its content lives in its own namespace; users can opt in/out at world creation.

**Files:**
- `packs/wickham-tables/pack.yaml` (new)
- `packs/wickham-tables/content/tables/<n>.yaml` (new — encoded from Grammar Fuel)
- `packs/wickham-tables/content/procedures/<chosen-generator>.yaml` (new)

**Source-encoding note:** Wickham's tables are author-encoded from Grammar Fuel verbatim. Mark file headers with the source page reference. Do not paraphrase entries.

**Tests:** `tests/procedures/test_wickham_generator.py`
- The pack loads cleanly with `xwn-core` enabled.
- Running the chosen generator 50 times produces structurally valid output.
- Smoke test: a world created with `[xwn-core, wickham-tables]` can run the procedure.

**Acceptance:** Tests pass. The pack is committed to version control. A user can add `wickham-tables` to a new world and use the generator from the admin UI's procedure runner.

---

### Task 1.9 — Status effect content schema

**Points:** 1
**Dependencies:** Phase 0 Pack loader (Task 0.4)
**Test layers:** [U]

**What:** Pydantic model for status effect records.

**File:** `src/harsh_realm/status_effects/schema.py` (new)

**Model:**

```python
class StatusEffect(BaseModel):
    """A status effect content record."""
    model_config = ConfigDict(frozen=True)

    id: str
    name: str
    description: str = ""
    default_duration_ticks: int = Field(
        default=0,
        ge=0,
        description="0 means permanent until explicitly removed",
    )
    tags: list[str] = Field(default_factory=list)
    icon: str | None = None
    stacking: Literal["replace", "extend", "stack"] = Field(
        default="replace",
        description=(
            "How a re-application interacts with an existing instance: "
            "replace (overwrite duration), extend (sum durations), "
            "stack (multiple parallel instances)"
        ),
    )
```

Phase 1 has *no mechanical fields* on status effects. No modifier list, no triggers — those come in Phases 2 and 3. Phase 1 just establishes "the concept of a status effect exists; here's how it's named, described, and how long it lasts."

**Tests:** `tests/status_effects/test_schema.py`
- Valid record parses.
- `default_duration_ticks < 0` raises `ValidationError`.
- Invalid `stacking` value raises `ValidationError`.

**Acceptance:** Tests pass.

---

### Task 1.10 — Status effect database table

**Points:** 1
**Dependencies:** 1.9
**Test layers:** [U]

**What:** Add SQLite table for active status effects.

**File:** `src/harsh_realm/db.py` (extend `_init_schema`)

**New table:**

```sql
CREATE TABLE entity_status_effects (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    effect_id       TEXT NOT NULL,            -- qualified ID, e.g. "xwn-core:status.poisoned"
    applied_at_tick INTEGER NOT NULL,
    expires_at_tick INTEGER,                  -- NULL means permanent
    source          TEXT,                     -- free-form: "encounter:42", "trap:cell-3-7", etc.
    data_json       TEXT NOT NULL DEFAULT '{}'  -- per-application state, e.g., stacking count
);

CREATE INDEX idx_entity_status_effects_entity ON entity_status_effects(entity_id);
CREATE INDEX idx_entity_status_effects_expiry ON entity_status_effects(expires_at_tick);
```

**Tests:** `tests/test_db.py` (extend)
- New world has empty `entity_status_effects` table.
- Insert and read back round-trips.

**Acceptance:** Tests pass.

---

### Task 1.11 — StatusEffectService and repository

**Points:** 3
**Dependencies:** 1.10, Phase 0 ContentService
**Test layers:** [U] [P]

**What:** The subsystem that owns status effect persistence and lifecycle. Compliant with AGENTS.md Rules 1, 2, 4: extrinsic data, durable persistence, owns its table.

**Files:**
- `src/harsh_realm/status_effects/service.py` (new)
- `src/harsh_realm/status_effects/repository.py` (new)
- `src/harsh_realm/status_effects/models.py` (new — runtime models for active effects)

**Models:**

```python
class ActiveStatusEffect(BaseModel):
    """An applied status effect on an entity."""
    model_config = ConfigDict(frozen=True)

    id: int
    entity_id: str
    effect_id: str
    applied_at_tick: int
    expires_at_tick: int | None
    source: str | None
    data: dict[str, ProcedureValue] = Field(default_factory=dict)
```

**Service API:**

```python
class StatusEffectService:
    PERSISTENCE = "durable"

    def __init__(
        self,
        repo: StatusEffectRepository,
        content: ContentService,
        clock: WorldClock,
    ) -> None: ...

    async def apply(
        self,
        entity_id: str,
        effect_id: str,
        duration_ticks: int | None = None,
        source: str | None = None,
        data: dict[str, ProcedureValue] | None = None,
    ) -> ActiveStatusEffect:
        """Apply an effect. Honors the effect's stacking policy."""

    async def remove(
        self,
        entity_id: str,
        effect_id: str,
    ) -> int:
        """Remove all instances of effect_id from entity. Returns count removed."""

    async def remove_by_id(self, active_effect_id: int) -> bool: ...

    async def list_for_entity(self, entity_id: str) -> list[ActiveStatusEffect]: ...

    async def expire_due(self, current_tick: int) -> list[ActiveStatusEffect]:
        """Remove all effects whose expires_at_tick <= current_tick. Returns the removed effects."""
```

**Stacking semantics (per Task 1.9 schema):**
- `replace`: `apply` removes existing instances of same `effect_id` first, then inserts new.
- `extend`: existing instance has its `expires_at_tick` increased by the new duration; no new row.
- `stack`: insert another row; multiple instances coexist.

**Repository:** standard repository pattern matching `EntityRepository` and similar from existing code. Owns all SQL against `entity_status_effects`.

**Tests:** `tests/status_effects/test_service.py`
- Apply effect, list shows it.
- Apply with `replace` stacking twice: list shows one, with second's duration.
- Apply with `extend` stacking: duration sums.
- Apply with `stack`: list shows two.
- `expire_due` removes expired, leaves unexpired.
- Permanent effect (`expires_at_tick = NULL`) never expires.
- Property test: after `apply` and `remove` of the same effect, `list_for_entity` does not include it.

**Acceptance:** Tests pass.

---

### Task 1.12 — Status effect events and event handler

**Points:** 2
**Dependencies:** 1.11
**Test layers:** [U]

**What:** Wire status effect lifecycle into the event bus per AGENTS.md Rule 2 (cross-subsystem writes go through events).

**Files:**
- `src/harsh_realm/payloads.py` (extend with status effect typed payloads)
- `src/harsh_realm/status_effects/handlers.py` (new)
- `src/harsh_realm/gm/domain_events.py` (extend dispatcher registration)

**Event types:**
- `status.apply_requested` — request to apply an effect (from any subsystem).
- `status.applied` — terminal event after successful application.
- `status.remove_requested` — request to remove.
- `status.removed` — terminal event after removal.
- `status.expired` — terminal event after auto-expiration.

**Payloads:** typed Pydantic models in `payloads.py`.

**Handler:**
- `StatusEffectEventHandler` subscribes to `*_requested` events and calls `StatusEffectService` to perform the write. Emits the corresponding terminal event with the resulting `ActiveStatusEffect`.
- Errors caught and logged; never propagate to the bus (per AGENTS.md).

**Tick integration:**
- The world clock's tick advancement publishes `world.tick_advanced` (existing event). A new subscriber calls `StatusEffectService.expire_due(current_tick)` and emits `status.expired` for each removed effect.

**Tests:** `tests/status_effects/test_handlers.py`
- Emit `status.apply_requested`, observe `status.applied`, observe service has the effect.
- Emit `status.remove_requested`, observe `status.removed`.
- Advance world tick past expiration, observe `status.expired` events.
- Handler errors are logged but do not crash the bus.

**Acceptance:** Tests pass.

---

### Task 1.13 — One status effect imported as content (test case)

**Points:** 1
**Dependencies:** 1.12
**Test layers:** [U]

**What:** The Phase 1 status-effect test case. Add at least one status effect record to `xwn-core` and verify end-to-end application/expiration.

**Suggested record:** "Poisoned" — a generic status effect representative of typical XWN content. No modifier or trigger fields (those come in Phases 2 and 3); just identity and duration.

**File:** `packs/xwn-core/content/status_effects/poisoned.yaml`

```yaml
id: poisoned
name: Poisoned
description: |
  The character has been poisoned. Without treatment or strong constitution,
  the poison runs its course and clears.
default_duration_ticks: 6
tags: [debuff, biological]
stacking: extend
```

(Mechanical effect of "poisoned" — actual damage per tick — is implemented in Phase 3 as a triggered effect. Phase 1 just tracks the condition.)

**Tests:** `tests/status_effects/test_xwn_core_effects.py`
- `xwn-core:status.poisoned` loads as a content record.
- Applying it to an entity persists in `entity_status_effects` with expected expiration.
- Advancing the world clock past expiration removes it.
- An admin UI inspector can query the effect on an entity.

**Acceptance:** Tests pass.

---

### Task 1.14 — Frontend: active status effects on character sidebar

**Points:** 2
**Dependencies:** 1.12
**Test layers:** [V] [E2E]

**What:** Display the player character's active status effects in the existing `StatusSidebar.vue` (or wherever character status currently shows).

**Files:**
- `frontend/src/components/StatusSidebar.vue` (extend)
- `frontend/src/types/api.ts` (add `ActiveStatusEffect` type)
- `frontend/src/stores/game.ts` (extend with effects state)
- `frontend/src/composables/useWebSocket.ts` (extend with status event handlers)

**Behavior:**
- On world load, fetch active effects for the player character via a new endpoint `GET /api/character/<id>/status_effects`.
- WebSocket events `status.applied`, `status.removed`, `status.expired` update the store.
- Sidebar displays each effect's name, optional icon, remaining ticks, and a tooltip with the description.

**Tests:**
- Vitest: store correctly updates on simulated WebSocket events.
- Playwright: a test world applies "poisoned" via admin command, sidebar shows the effect, advances clock, sidebar updates and eventually clears.

**Acceptance:** Tests pass.

---

### Task 1.15 — Documentation and acceptance criteria

**Points:** 1
**Dependencies:** all preceding tasks
**Test layers:** none

**What:**
- Update `AGENTS.md` to note that procedures are pack content and the procedure runner is the canonical multi-step generation framework. Add to "What NOT to Do": "No new hardcoded multi-step generators in Python — use a procedure record."
- Update `CLAUDE.md` "Completed Subsystems" section to add procedure framework and status effect service.
- Append Phase 1 entries to `docs/acceptance_criteria.md`.

**Acceptance:** Documents updated.

---

## 4. Phase completion criteria

Phase 1 is complete when *all* of the following hold:

1. All 15 tasks above are implemented and committed.
2. Full existing test suite passes; new tests added by Phase 1 raise the total.
3. UNE personality generation runs through the procedure runner. The hardcoded `UNEGenerator` is gone (or reduced to a thin facade).
4. The `wickham-tables` pack exists and a world can be created with `[xwn-core, wickham-tables]`. The chosen Wickham generator runs successfully.
5. `StatusEffectService` is operational. Applying "poisoned" to a character persists; advancing the world clock past its duration removes it; events fire at each transition.
6. Status effects appear in the character sidebar in real time.
7. `AGENTS.md`, `CLAUDE.md`, and `docs/acceptance_criteria.md` are updated.

## 5. Phase 1 deferrals (append to overview §11)

Items deferred from Phase 1:

- **Conditional procedure steps** (`if`/`when`/branching). Deferred to Phase 3, where the same DSL primitives serve both procedures and triggers.
- **Procedure debugging UI.** A future cycle could add step-by-step trace inspection in admin. Phase 1 ships only an "execute and return result" runner.
- **Modifier integration for status effects.** "Status effect grants +X to attribute Y" is Phase 2.
- **Triggered effects on status effects.** "Effect deals damage per tick" is Phase 3.
- **Migration of `engine/oracle.py` Mythic procedures to the procedure framework.** Future cycle once the framework is proven elsewhere.
- **Migration of `generators/settlement_gen.py`, `generators/encounter_gen.py`** to procedures. Listed in audit; deferred.
- **Status effect interaction rules** (e.g., "can't apply Burning while Wet is active"). Future cycle. Phase 1's stacking handles re-applications of the same effect; cross-effect interactions are out of scope.
- **Status effect categories with category-level rules** (e.g., "max 3 buffs at once"). Future cycle.

## 6. Notes for the coding agent

- Phase 1 is smaller than Phase 0. Tasks are tightly coupled within the procedure cluster (1.2–1.8) and within the status-effect cluster (1.9–1.14). Pick one cluster at a time per session for cleaner commits.
- The Wickham pack (Task 1.8) is the first non-`xwn-core` pack created in this cycle. Treat any friction as a signal that the pack format from Phase 0 needs adjustment, and surface it explicitly rather than working around it.
- UNE migration (Task 1.6) preserves behavior. The acceptance test is "social scene still works." Do not change UNE rules during the migration; the goal is to *prove* the procedure framework can express what was previously hardcoded, not to redesign UNE.
- Status effects in Phase 1 are deliberately minimal. Resist the temptation to add modifier or trigger fields here. Those are real Phase 2 / Phase 3 work and rushing them creates a bad framework. The Phase 1 status effect service is a bare bones lifecycle tracker — that's the entire goal.
- Per AGENTS.md, every task uses the four-layer test rule. The `[U]`/`[P]`/`[M]`/`[E2E]` markings are minima: add layers if a task qualifies for more.
- After every task, run the full test suite. Commit with `[Phase 1 / Task 1.N]` for traceability.
