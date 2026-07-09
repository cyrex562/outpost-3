# Modular Rules Architecture — Phase 3: Trigger / Effect Engine

**Date:** 2026-04-26
**Status:** Draft
**Cycle:** Modular Rules Architecture
**Overview:** `2026-04-26-modular-rules-architecture-overview.md`
**Depends on:** Phase 2 complete (modifier framework, trait service, resource service). Phase 1 should be complete (status effect service); the Phase 1→Phase 3 integration relies on it.
**Prerequisites for tasks:** Read overview, Phase 0, Phase 1, Phase 2 specs. Read `AGENTS.md` for coding standards.

---

## 1. Phase scope

Phase 3 ships **the** declarative DSL — the trigger/effect engine. After Phase 3, content packs can express event-driven behaviors entirely in YAML: "when X happens, if Y is true, do Z." This is the layer that takes traits and status effects from "passive modifiers only" to "real working mechanics."

Phase 3 is also where the **condition language** ships in full. Phase 2's limited predicates (`always`, `entity_has_tag`, etc.) get replaced by a richer expression DSL with comparisons, boolean combinators, path access, and arithmetic. Phase 2's predicates continue to work — they desugar into the new DSL — so existing Phase 2 content doesn't break.

Phase 3 is the second-largest framework phase. It's smaller than Phase 2 in line count but harder per-line: parsers, evaluators, and event-routing have real implementation depth and demand careful testing.

### What Phase 3 produces

- **Condition language.** A small expression DSL covering literals, path access (`entity.hp`, `event.target_id`, `world.tick`), comparison operators (symbolic and keyword forms), boolean combinators, arithmetic, and a small whitelist of helper functions. Hand-written recursive-descent parser; no new dependencies.
- **Trigger schema.** A trigger is `{ on: <event-type>, when: <condition>, do: <effect-list> }`. Triggers attach to traits, status effects, items (in a future cycle), and standalone trigger records.
- **Effect verbs.** Initial set of nine: `apply_modifier`, `remove_modifier`, `change_resource`, `apply_status`, `remove_status`, `emit_event`, `roll_dice`, `run_procedure`, `log`. Effect verbs are extensible; code-bearing packs can register new verbs through the existing `register(app_state)` hook.
- **Trigger registry and event-driven execution.** When an event fires, the registry finds matching triggers, evaluates their conditions, and runs their effects. Execution is in-process and synchronous-style (`async def` is allowed, but no scheduling).
- **Phase 2 condition predicates** continue to work via desugaring.
- **Status effect ↔ trigger/modifier integration.** The Phase 2 deferral picked up here: status effects can declare modifier lists and trigger lists. A poisoned character can have triggers that fire on tick (deal damage) and modifier contributions (penalty to attribute checks) — both expressed in YAML.
- **Test case content:**
  - "Poisoned" status effect upgraded with a real triggered behavior (1 damage per N ticks).
  - One non-trivial Godbound Gift expressed entirely declaratively — modifier list + trigger list + condition expressions, no Python code.

### What Phase 3 does not do

- **No world-mutation verbs.** `create_settlement`, `shift_weather_pattern`, `transform_terrain`, `establish_dominion` — these wait until their target subsystems exist (future cycle). The Phase 3 verb registry is extensible, so when those subsystems ship, they register their verbs without engine changes.
- **No conditional effect steps.** Effects run linearly. If you need "do A if condition, else B," attach two triggers with mutually exclusive conditions to the same event. This keeps the verb list small and the runtime simple.
- **No loops or user-defined functions in the DSL.** This is non-negotiable. Adding either turns the DSL into a programming language, which is exactly the trap the overview warned against (§3, design tension).
- **No items refactored into the trigger framework.** Items still flow through existing combat code. A future cycle migrates them.
- **No NPC behavior using triggers.** Phase 4 (NPC scheduled routines) is documented but not implemented this cycle. Phase 4's triggers use the same engine but are spec'd separately.
- **No DSL versioning across pack updates.** Phase 3 ships v1 of the DSL. Future DSL changes are migration concerns; for now, all packs target v1.
- **No sandbox or trust model on DSL evaluation.** The DSL evaluator can read entity/event/world state and call subsystem APIs — same capability as a code-bearing pack. Single-user deployment makes this acceptable.

## 2. Decisions locked in this phase

These are the high-stakes calls flagged in the overview's §10. Defaults stand unless changed during this phase.

- **Condition syntax:** string-form, always quoted in YAML. Internal representation is an AST. Both symbolic (`==`, `!=`, `<`, `<=`, `>`, `>=`) and keyword (`eq`, `neq`, `lt`, `lte`, `gt`, `gte`) operators parse to the same AST nodes. Keyword forms are documented as the YAML-preferred style.
- **Path roots:** four roots are recognized — `entity` (the entity the trigger is attached to), `event` (the triggering event payload), `world` (world clock and global state), and `target` (event-specific; the entity being acted upon if different from `entity`). Future roots can be added without a DSL version bump.
- **Helper functions in conditions:** `has_tag(entity, tag)`, `has_trait(entity, qualified_id)`, `has_status(entity, qualified_id)`, `len(list)`, `min(a, b)`, `max(a, b)`, `abs(n)`. No user-defined functions.
- **Effect verb set (v1):** `apply_modifier`, `remove_modifier`, `change_resource`, `apply_status`, `remove_status`, `emit_event`, `roll_dice`, `run_procedure`, `log`. New verbs are added through pack registration; engine code does not need changes to support new verbs.
- **Trigger ordering when multiple triggers match an event:** registration order, with stable tiebreak by `(pack_id, trigger_id)`. No priority field in v1.
- **Cascade limit:** trigger effects can emit events that fire other triggers. The cascade depth limit is the existing event bus limit (default 10 from `EventBus`). Phase 3 does not add a separate trigger-specific limit.

## 3. Tasks

Test layer notation: **[U]** pytest unit, **[P]** Hypothesis property, **[M]** mutmut mutation, **[E2E]** Playwright, **[V]** Vitest unit, **[FC]** fast-check property, **[S]** Stryker mutation.

---

### Task 3.1 — Codebase audit for trigger candidates and event taxonomy

**Points:** 1
**Dependencies:** Phase 2 complete
**Test layers:** none (investigation)

**What:** Examine the codebase to inventory event types currently emitted, places where hardcoded "if X happens, do Y" logic lives, and Phase 2 places that referenced "Phase 3 will handle this."

**Procedure:**
1. List every event type currently emitted by the engine. Use `grep` on `event_type=` literals in `src/harsh_realm/`. Categorize as command-intent / domain-result / presentation per the existing event taxonomy. The DSL primarily subscribes to domain-result events.
2. Find every `gm/scenes/` and `engine/` site where reactive logic exists ("when X happens, then Y"). Examples likely include: combat HP-zero handling, status effect tick processing (Phase 1 added the bare service; any Python-side tick logic is a candidate), faction reputation cascades, social skill check side effects.
3. Re-list the Phase 2 placeholders: every `Trait.triggers: list[dict]` Phase 2 left as forward-compat. Every status effect Phase 1 created without active mechanics.
4. Identify the existing `LowHealthWarningSystem` / "warn at low HP" handler from the rules-arch spec. Document its behavior; it becomes a candidate for re-expression as a trigger record.
5. Enumerate the Phase 2 modifier conditions that need desugaring: every existing `ModifierCondition(predicate=...)` in pack content.

**Deliverable:** `docs/superpowers/specs/2026-04-26-phase-3-codebase-audit.md` covering all five sections.

**Acceptance:** Audit document exists.

---

### Task 3.2 — Condition AST schema

**Points:** 2
**Dependencies:** 3.1
**Test layers:** [U]

**What:** Pydantic models for the condition expression AST. The parser (Task 3.3) produces these; the evaluator (Task 3.5) consumes them.

**File:** `src/harsh_realm/dsl/ast.py` (new)

**AST node types:**

```python
class Literal(BaseModel):
    """An int, float, str, bool, or None literal."""
    model_config = ConfigDict(frozen=True)
    kind: Literal["int", "float", "str", "bool", "null"]
    value: int | float | str | bool | None


class PathRef(BaseModel):
    """A dotted path access like entity.hp or event.target_id."""
    model_config = ConfigDict(frozen=True)
    root: Literal["entity", "event", "world", "target", "self"]
    parts: list[str]


class BinaryOp(BaseModel):
    """A two-operand operation: arithmetic, comparison, or logical."""
    model_config = ConfigDict(frozen=True)
    op: str  # one of "==", "!=", "<", "<=", ">", ">=", "+", "-", "*", "/", "and", "or", "in"
    left: Expr
    right: Expr


class UnaryOp(BaseModel):
    """A one-operand operation: not, negation."""
    model_config = ConfigDict(frozen=True)
    op: Literal["not", "-"]
    operand: Expr


class FuncCall(BaseModel):
    """A whitelisted helper function call."""
    model_config = ConfigDict(frozen=True)
    name: str  # one of has_tag, has_trait, has_status, len, min, max, abs
    args: list[Expr]


class ListLiteral(BaseModel):
    """A bracketed list expression: [1, 2, 3] or [entity.hp, entity.max_hp]."""
    model_config = ConfigDict(frozen=True)
    items: list[Expr]


Expr = Literal | PathRef | BinaryOp | UnaryOp | FuncCall | ListLiteral
```

`Expr` is a discriminated union via Pydantic's standard mechanism. Recursive references resolved with forward declarations.

**Tests:** `tests/dsl/test_ast.py`
- Each node type constructs and serializes correctly.
- A tree of `BinaryOp` containing `PathRef` and `Literal` validates.
- Round-trip: `ast.model_dump()` then `Expr.model_validate(...)` produces the same tree.

**Acceptance:** Tests pass.

---

### Task 3.3 — Condition string parser

**Points:** 3
**Dependencies:** 3.2
**Test layers:** [U] [P]

**What:** A hand-written recursive-descent parser that turns a quoted string into the AST. Hand-written, no new dependencies.

**File:** `src/harsh_realm/dsl/parser.py` (new)

**Grammar (informal):**

```
expr        := or_expr
or_expr     := and_expr ( ("or" | "||") and_expr )*
and_expr    := not_expr ( ("and" | "&&") not_expr )*
not_expr    := ("not" | "!") not_expr | comparison
comparison  := arith ( comparison_op arith )?
arith       := term ( ("+" | "-") term )*
term        := factor ( ("*" | "/") factor )*
factor      := unary
unary       := "-" unary | primary
primary     := literal | path | func_call | list_literal | "(" expr ")"
literal     := int | float | str | "true" | "false" | "null"
path        := root ("." identifier)*
root        := "entity" | "event" | "world" | "target" | "self"
func_call   := identifier "(" arg_list? ")"
list_literal:= "[" expr_list? "]"
identifier  := [a-zA-Z_][a-zA-Z0-9_]*
comparison_op := "==" | "!=" | "<" | "<=" | ">" | ">=" | "eq" | "neq" | "lt" | "lte" | "gt" | "gte" | "in"
```

**API:**

```python
def parse_condition(source: str) -> Expr: ...

class ParseError(Exception): ...
```

**Implementation notes:**
- Tokenizer first, parser second. Tokenizer recognizes operators (both symbolic and keyword forms), identifiers, integers, floats, strings (single and double quotes), parens, brackets, commas, dots.
- Keyword operators (`and`, `or`, `not`, `eq`, etc.) are not reserved words; they're recognized contextually. So `entity.eq` is a valid path access (not a parse error), but `entity eq other` is a comparison.
- Error messages include the position (line and column) and a snippet of the source.

**Tests:** `tests/dsl/test_parser.py`
- All comparison ops parse, both forms.
- Boolean combinators parse correctly with precedence: `a and b or c` → `(a and b) or c`.
- `not` has higher precedence than `and`/`or`.
- Path access: `entity.attributes.str` parses to `PathRef(root="entity", parts=["attributes", "str"])`.
- Function calls: `has_tag(entity, "undead")` parses correctly.
- Nested expressions: `entity.hp < entity.max_hp * 0.25 and not has_status(entity, "xwn-core:status.unconscious")`.
- Property test: any AST node serializes to a string that re-parses to an equivalent AST.
- Malformed input: missing close paren → `ParseError`. Trailing tokens → `ParseError`. Unknown function name does *not* parse-error (validation happens at evaluation).

**Acceptance:** Tests pass.

---

### Task 3.4 — Path resolver and evaluation context

**Points:** 3
**Dependencies:** 3.2
**Test layers:** [U] [P]

**What:** The runtime that resolves `PathRef` nodes against a typed evaluation context, and the model for the context itself.

**File:** `src/harsh_realm/dsl/context.py` (new)

**Models:**

```python
class EvalContext(BaseModel):
    """The state available to a DSL evaluation."""
    model_config = ConfigDict(frozen=True, arbitrary_types_allowed=True)

    self_entity: Character | NPC                # the entity that owns the trigger
    target_entity: Character | NPC | None       # the event target if different
    event_payload: dict[str, ProcedureValue] = Field(default_factory=dict)
    world_state: WorldStateSnapshot
    services: ServiceBundle  # injected runtime services for tag/trait/resource queries


class WorldStateSnapshot(BaseModel):
    model_config = ConfigDict(frozen=True)
    tick: int
    chaos_factor: int
    # additional global state added as needed


class ServiceBundle(BaseModel):
    """Read-only handles to subsystems for DSL evaluation."""
    model_config = ConfigDict(arbitrary_types_allowed=True, frozen=True)
    tag_service: TagService
    trait_service: TraitService
    status_service: StatusEffectService
    resource_service: ResourceService
```

**Path resolution:**

```python
async def resolve_path(path: PathRef, ctx: EvalContext) -> ProcedureValue:
    """Resolve a path against the eval context."""
```

Resolution semantics:
- `entity.<field>` → `ctx.self_entity.<field>`. Supports nested dots into attribute dicts: `entity.attributes.str`.
- `target.<field>` → `ctx.target_entity.<field>` (raises if `target_entity` is None and accessed).
- `self` is an alias for `entity`.
- `event.<field>` → `ctx.event_payload[<field>]`. Nested supported for nested dicts in payload.
- `world.<field>` → `ctx.world_state.<field>`.
- A path that doesn't resolve raises `PathResolutionError` with the path string.

For convenience, a few derived paths are special-cased:
- `entity.hp` resolves to `ResourceService.get(entity_id, "xwn-core:resource.hp").current` (live read).
- `entity.max_hp` → `.max`.
- `entity.gold` → resource for gold similarly.
- `entity.tags` → `tag_service.get_tags(entity_id)` (a frozenset).
- `entity.traits` → `trait_service.get_traits(entity_id)` (returns a list of qualified IDs).

This bridges the DSL across the modifier/trait/resource frameworks without users having to know the qualified IDs.

**Tests:** `tests/dsl/test_path_resolver.py`
- Each path root resolves a sample field correctly.
- Nested path access works.
- Missing path raises descriptive error.
- `entity.hp` reads live HP via ResourceService.
- `entity.tags` returns dynamic tags.
- Property test: `entity.<existing_field>` is never None for a valid character.

**Acceptance:** Tests pass.

---

### Task 3.5 — Condition evaluator

**Points:** 2
**Dependencies:** 3.4
**Test layers:** [U] [P]

**What:** The evaluator that takes an AST and an `EvalContext` and returns a typed value (bool, int, float, str, or list).

**File:** `src/harsh_realm/dsl/evaluator.py` (new)

**API:**

```python
async def evaluate(expr: Expr, ctx: EvalContext) -> ProcedureValue: ...

async def evaluate_to_bool(expr: Expr, ctx: EvalContext) -> bool:
    """Convenience: evaluate, raise EvalError if result is not a bool."""
```

**Semantics:**
- Literal evaluates to its value.
- PathRef evaluates via `resolve_path`.
- BinaryOp: evaluate both operands, then apply operator. Type checking:
  - Comparison ops require comparable operands; mixing int and float ok; mixing str and int errors.
  - Arithmetic requires numeric.
  - `and`/`or` short-circuit; require boolean operands.
  - `in` requires right operand to be a list, set, or string; checks membership.
- UnaryOp: standard `not` (bool) and `-` (numeric).
- FuncCall: dispatch to the helper function whitelist. Unknown functions error.
- ListLiteral: evaluate each item.

Type errors at evaluation raise `EvalError` with the expression substring.

**Tests:** `tests/dsl/test_evaluator.py`
- Each op produces correct results on valid inputs.
- `and`/`or` short-circuit: `false and X` doesn't evaluate X.
- Type mismatch raises `EvalError`.
- Helper function calls produce expected results.
- Property test: for any AST that uses only literals and arithmetic on integers, evaluation produces the same result as Python's eval on the equivalent expression.

**Acceptance:** Tests pass.

---

### Task 3.6 — Trigger schema

**Points:** 1
**Dependencies:** 3.3
**Test layers:** [U]

**What:** Pydantic model for trigger records.

**File:** `src/harsh_realm/triggers/schema.py` (new)

**Model:**

```python
class Trigger(BaseModel):
    model_config = ConfigDict(frozen=True)

    id: str = Field(description="Unique within the owning record (trait/status/etc)")
    on: str = Field(description="Event type to subscribe to")
    when: str = Field(
        default="true",
        description="Condition expression (string form). Default: always fires.",
    )
    do: list[Effect]
    description: str = ""

    @field_validator("when")
    @classmethod
    def _condition_parses(cls, v: str) -> str:
        # Ensure the condition is parseable at validation time
        from harsh_realm.dsl.parser import parse_condition, ParseError
        try:
            parse_condition(v)
        except ParseError as e:
            raise ValueError(f"Invalid condition expression: {e}")
        return v
```

`Effect` is defined in Task 3.7.

**Tests:**
- Valid trigger record parses.
- Invalid condition fails validation with parser error.
- Trigger with empty `do` list parses (no-op trigger).

**Acceptance:** Tests pass.

---

### Task 3.7 — Effect schema

**Points:** 1
**Dependencies:** 3.6
**Test layers:** [U]

**What:** Pydantic model for effect records — discriminated union over verb types.

**File:** `src/harsh_realm/triggers/effects.py` (new)

**Models:**

```python
class Effect(BaseModel):
    """Discriminated union over effect verbs. Concrete fields per kind."""
    model_config = ConfigDict(frozen=True)
    kind: str  # one of the registered verb names
    params: dict[str, str | int | float | bool | list | dict] = Field(default_factory=dict)


# For static type clarity, also define typed sub-models for each v1 verb:

class ApplyModifierEffect(Effect):
    kind: Literal["apply_modifier"]
    # params shape:
    #   target: path expression resolving to entity
    #   modifier: Modifier dict
    #   duration_ticks: int (optional, default 0 = permanent until removed)


class ChangeResourceEffect(Effect):
    kind: Literal["change_resource"]
    # params shape:
    #   target: path expression
    #   resource: qualified ID
    #   delta: int or path expression


# ... and so on for the other v1 verbs.
```

For Phase 3, the runtime accepts the loose `Effect` form (kind + params dict). Validation happens when the verb handler unpacks the params. This keeps the engine open for new verbs without modifying schema code.

**Tests:** `tests/triggers/test_effect_schema.py`
- Each v1 verb's params shape parses.
- Unknown `kind` parses but warns at registration.

**Acceptance:** Tests pass.

---

### Task 3.8 — Effect verb registry and dispatcher

**Points:** 2
**Dependencies:** 3.7
**Test layers:** [U]

**What:** A registry that holds effect-verb handlers keyed by name, and a dispatcher that runs an effect list.

**File:** `src/harsh_realm/triggers/dispatcher.py` (new)

**API:**

```python
EffectHandler = Callable[[dict, EvalContext], Awaitable[None]]


class EffectVerbRegistry:
    def register(self, verb_name: str, handler: EffectHandler) -> None: ...
    def get(self, verb_name: str) -> EffectHandler | None: ...
    def list_registered(self) -> list[str]: ...


class EffectDispatcher:
    def __init__(self, registry: EffectVerbRegistry) -> None: ...

    async def run(self, effects: list[Effect], ctx: EvalContext) -> None:
        """Run effects in order. Each effect resolves its params against ctx,
        then calls the registered handler. Failures of one effect do not stop
        subsequent effects (logged but not raised), unless the failure is a
        registry miss (unknown verb), which raises immediately."""
```

**Tests:** `tests/triggers/test_dispatcher.py`
- A registered handler runs when an effect of that kind is dispatched.
- Unknown verb raises `UnknownEffectVerbError`.
- Handler exception is logged, dispatcher continues with remaining effects.

**Acceptance:** Tests pass.

---

### Task 3.9 — Simple effect verbs (emit_event, log, change_resource)

**Points:** 2
**Dependencies:** 3.8, Phase 2 ResourceService
**Test layers:** [U]

**What:** Implement and register the three simplest verbs.

**File:** `src/harsh_realm/triggers/verbs/simple.py` (new)

**`emit_event`:** publishes an event to the bus. Params: `event_type` (str), `data` (dict, with path expressions resolved).

**`log`:** writes a narrative log message. Params: `message` (str, with `{path}` interpolation).

**`change_resource`:** calls `ResourceService.change`. Params: `target` (path → entity_id), `resource` (qualified ID), `delta` (int or path).

Each handler resolves its params against the context (paths get resolved, plain values pass through), then calls the underlying service.

**Tests:** `tests/triggers/test_simple_verbs.py`
- `emit_event` publishes the right event type with resolved data.
- `log` writes the message via the standard logging module.
- `change_resource` calls ResourceService with resolved arguments.

**Acceptance:** Tests pass.

---

### Task 3.10 — Status effect verbs (apply_status, remove_status)

**Points:** 1
**Dependencies:** 3.9, Phase 1 StatusEffectService
**Test layers:** [U]

**What:** Implement `apply_status` and `remove_status` verbs.

**File:** `src/harsh_realm/triggers/verbs/status.py` (new)

`apply_status` params: `target` (path), `effect_id` (qualified ID), `duration_ticks` (int, optional), `source` (str, optional). Calls `StatusEffectService.apply` (via emitting `status.apply_requested` event per Phase 1's pattern).

`remove_status` params: `target`, `effect_id`. Same pattern.

**Tests:**
- Both verbs trigger the corresponding service action.

**Acceptance:** Tests pass.

---

### Task 3.11 — Modifier verbs (apply_modifier, remove_modifier)

**Points:** 2
**Dependencies:** 3.9, Phase 2 ModifierService
**Test layers:** [U]

**What:** Implement `apply_modifier` and `remove_modifier`. These add a *transient* modifier to an entity outside of trait/status sources — useful for one-shot effects ("attacker takes -1 to attacks for the rest of the scene after using this Gift").

This requires a small extension to the modifier framework: a "transient modifier source" that's neither a trait nor a status effect. Phase 2's `ModifierService.collect` walked traits and status effects; Phase 3 extends it to also walk a transient-modifiers table.

**Files:**
- `src/harsh_realm/triggers/verbs/modifiers.py` (new — verb handlers)
- `src/harsh_realm/modifiers/transient.py` (new — transient modifier registry, possibly with persistence)
- `src/harsh_realm/db.py` (new table `entity_transient_modifiers`)

**Schema for transient modifiers:**

```sql
CREATE TABLE entity_transient_modifiers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_id       TEXT NOT NULL,
    modifier_json   TEXT NOT NULL,
    applied_at_tick INTEGER NOT NULL,
    expires_at_tick INTEGER,
    source          TEXT
);
```

`ModifierService.collect` extended to read this table for the entity.

**Tests:**
- `apply_modifier` adds a row; `ModifierService.collect` returns it.
- `remove_modifier` deletes the row.
- A transient modifier with `expires_at_tick` past current tick is filtered out (or eagerly removed by tick handler).

**Acceptance:** Tests pass.

---

### Task 3.12 — Composite verbs (roll_dice, run_procedure)

**Points:** 2
**Dependencies:** 3.9, Phase 1 ProcedureRunner
**Test layers:** [U]

**What:** Implement `roll_dice` and `run_procedure`.

**File:** `src/harsh_realm/triggers/verbs/composite.py` (new)

**`roll_dice`** params: `expr` (dice notation string like `"2d6+3"`), `assign` (str, optional — name to bind result in subsequent effects' param resolution). Roll happens via existing `engine/dice.py`.

**`run_procedure`** params: `procedure_id` (qualified), `inputs` (dict, paths resolved), `assign` (str, optional). Calls `ProcedureRunner.run`.

The `assign` mechanism requires a small extension to the dispatcher: an effect-list-local variable map that subsequent effects can reference via `local.<name>` in their param paths. This is Phase 3's only "shared state across effects" feature; without it, a damage trigger that rolls dice can't apply the rolled value to a target.

**Path root extension:** add `local` to the recognized path roots (so `entity`, `event`, `world`, `target`, `self`, and now `local`). Update the parser and resolver.

**Tests:**
- `roll_dice` produces a result; subsequent effect with `local.<assign_name>` reads it.
- `run_procedure` runs the procedure and binds output to local.
- A trigger with `roll_dice` then `change_resource(... delta: -local.damage ...)` deals rolled damage.

**Acceptance:** Tests pass.

---

### Task 3.13 — Trigger registry and event subscription

**Points:** 3
**Dependencies:** 3.6, 3.8
**Test layers:** [U]

**What:** The runtime that subscribes to events on the bus, finds matching triggers, evaluates conditions, and dispatches effects.

**File:** `src/harsh_realm/triggers/registry.py` (new)

**API:**

```python
class TriggerRegistry:
    """Holds active triggers indexed by event type."""

    def __init__(
        self,
        event_bus: EventBus,
        dispatcher: EffectDispatcher,
        services: ServiceBundle,
    ) -> None: ...

    def register(self, owner_entity_id: str, owner_source: str, trigger: Trigger) -> str:
        """Register a trigger. Returns a registration handle for later removal."""

    def unregister(self, handle: str) -> None: ...

    async def fire(self, event: GameEvent) -> None:
        """Called by the event bus subscription. Finds matching triggers and runs them."""
```

**Behavior:**
- `register` indexes the trigger by `trigger.on` event type.
- On engine startup or world load, the registry subscribes to all event types it has triggers for. (Or alternatively: subscribe to `*` and filter by event type. Performance is fine at current scale.)
- `fire` walks matching triggers, builds an `EvalContext` for each (with `self_entity` = the trigger's owner), evaluates `when`, and if true, dispatches `do`.
- Errors in one trigger don't stop other triggers from firing on the same event.

**Tests:**
- Register a trigger; fire a matching event; the effect runs.
- Register two triggers; both fire on the same event; both effects run.
- Trigger whose `when` evaluates false: effect does not run.
- Trigger whose effect emits another event that fires another trigger: cascade works (within depth limit).
- Unregister: trigger no longer fires.

**Acceptance:** Tests pass.

---

### Task 3.14 — Trait and status effect triggers wired into the registry

**Points:** 2
**Dependencies:** 3.13, Phase 2 TraitService, Phase 1 StatusEffectService
**Test layers:** [U]

**What:** When traits or status effects are added/removed from an entity, their triggers are registered/unregistered with the trigger registry.

**Files:**
- `src/harsh_realm/traits/service.py` (extend — `add_trait`/`remove_trait` register/unregister triggers)
- `src/harsh_realm/status_effects/service.py` (extend — `apply`/`remove` register/unregister triggers)

**Behavior:**
- When `TraitService.add_trait` is called, look up the trait's `triggers` list, register each one with `TriggerRegistry`. Store the registration handles in entity data so `remove_trait` can unregister them.
- Same for status effects: `StatusEffectService.apply` registers triggers; `remove`/`expire_due` unregisters them.

**Tests:**
- Adding a trait with a trigger: trigger is active; firing matching event runs the effect.
- Removing the trait: trigger is no longer active.
- Status effect lifecycle similarly.

**Acceptance:** Tests pass.

---

### Task 3.15 — Phase 2 modifier conditions desugared into DSL

**Points:** 1
**Dependencies:** 3.5
**Test layers:** [U] [P]

**What:** The Phase 2 `ModifierCondition` predicates (`always`, `entity_has_tag`, `target_has_tag`, `entity_has_trait`) continue to work, internally desugaring to DSL expressions.

**File:** `src/harsh_realm/modifiers/context.py` (extend `evaluate_condition` from Phase 2 Task 2.3)

**Behavior:**
- `always` → `true`
- `entity_has_tag(X)` → `has_tag(self, "X")`
- `target_has_tag(X)` → `has_tag(target, "X")`
- `entity_has_trait(X)` → `has_trait(self, "X")`

The Phase 2 `evaluate_condition` function's body becomes: parse the desugared DSL string, evaluate it. Phase 2 modifier records continue to validate and resolve correctly.

**Modifier records can also use `condition_expr: str`** as an alternative to the discrete predicate form. If both are present, `condition_expr` wins. Schema gains a model_validator that allows either form.

**Tests:**
- Phase 2 condition predicate `entity_has_tag("undead")` evaluates correctly via desugaring.
- New `condition_expr: "entity.hp < entity.max_hp * 0.25"` works for "low HP" modifier conditions.
- Property test: a Phase 2 condition and its desugared expression always produce the same result for a given context.

**Acceptance:** Tests pass.

---

### Task 3.16 — Status effect modifier integration (Phase 2 deferral)

**Points:** 2
**Dependencies:** 3.15
**Test layers:** [U]

**What:** The Phase 2 deferral picked up here. Status effect content schema gains a `modifiers: list[Modifier]` field. `ModifierService.collect` walks active status effects (already in the Phase 2 spec; this task wires it up).

**Files:**
- `src/harsh_realm/status_effects/schema.py` (extend with `modifiers` and `triggers` fields)
- `src/harsh_realm/modifiers/service.py` (extend `collect` to walk status effects via Phase 1's `StatusEffectService.list_for_entity`)

**Status effect schema after this task:**

```python
class StatusEffect(BaseModel):
    # Phase 1 fields ...
    modifiers: list[Modifier] = Field(default_factory=list)
    triggers: list[Trigger] = Field(default_factory=list)
    provides_tags: list[str] = Field(default_factory=list)
```

**Tests:**
- A status effect with a modifier list applied to an entity contributes to `ModifierService.collect`.
- Removing the status effect removes the contribution.

**Acceptance:** Tests pass.

---

### Task 3.17 — Test case: Poisoned with triggered tick damage

**Points:** 2
**Dependencies:** 3.14, 3.16
**Test layers:** [U]

**What:** Phase 3's first content test case. Upgrade the Phase 1 "Poisoned" status effect with a real triggered behavior and a modifier.

**File:** `packs/xwn-core/content/status_effects/poisoned.yaml` (extend)

```yaml
id: poisoned
name: Poisoned
description: |
  The character has been poisoned. Takes 1 damage every 4 ticks until cleared.
  Penalty to physical actions.
default_duration_ticks: 24    # 6 damage applications before clearing
stacking: extend
provides_tags: [poisoned, debuff_active]
modifiers:
  - target: skill.stab
    value: -1
    stacking: additive
    description: Poison weakens precise actions
  - target: skill.punch
    value: -1
    stacking: additive
triggers:
  - id: tick_damage
    on: world.tick_advanced
    when: "world.tick % 4 eq 0"
    do:
      - kind: change_resource
        params:
          target: self
          resource: xwn-core:resource.hp
          delta: -1
      - kind: log
        params:
          message: "{self.name} suffers from the poison."
```

**Tests:** `tests/dsl/test_poisoned_endtoend.py`
- Apply poisoned to a test character. Advance the world clock by 24 ticks.
- HP decreased by 6 (one per 4 ticks).
- During the affliction, skill check rolls for stab/punch include the −1 modifier.
- After 24 ticks, the status auto-expires; modifier and trigger no longer apply.

**Acceptance:** Tests pass. Poisoned now does what its name implies — entirely declaratively.

---

### Task 3.18 — Test case: Godbound Gift expressed declaratively

**Points:** 3
**Dependencies:** 3.14, 3.16
**Test layers:** [U]

**What:** Phase 3's second content test case and the cycle-level proof. Pick one Godbound Gift with both modifier and triggered components, encode it as a trait record entirely in YAML, and verify it works.

**Suggested Gift:** something like "Knight of the Sword" or "Word-Strike" or another Gift whose mechanics fit the Phase 3 verb set. Selection criteria:
- Has at least one passive modifier component.
- Has at least one event-driven triggered component.
- Doesn't require world-mutation verbs (no creating settlements, shifting weather, etc.).
- Doesn't require Effort commitment (Effort spending is a future cycle if it requires custom resolution; for now, treat Effort like any other resource via `change_resource`).

The chosen Gift goes in `packs/godbound-base/content/traits/gifts/<word>.<gift>.yaml`.

**Test case acceptance:**
- A character with the Gift demonstrates the passive modifier in a relevant skill check or combat math.
- A character with the Gift demonstrates the triggered behavior on the relevant event.
- No Python is added to make this Gift work; it's pure pack content.

**Tests:** integration test that applies the Gift to a character and exercises both pathways.

**Acceptance:** Tests pass. The Gift is committed to `godbound-base`. This is the cycle-level proof that the framework works.

---

### Task 3.19 — Trigger admin UI

**Points:** 2
**Dependencies:** 3.13
**Test layers:** [V] [E2E]

**What:** Admin UI for inspecting active triggers per entity, and a "trigger tester" that lets the user fire a synthetic event to see what triggers match.

**Files:**
- `frontend/src/components/admin/TriggerInspector.vue` (new)
- `frontend/src/components/admin/TriggerTester.vue` (new)
- backend endpoints: `GET /api/entities/<id>/triggers`, `POST /api/admin/test_trigger` (admin-mode-gated)

**Behavior:**
- Inspector lists all active triggers on a selected entity, showing source (which trait/status), event subscription, condition expression, and effect summary.
- Tester lets the user pick an event type, supply payload fields, and see which triggers would match (condition evaluated against the entity at that moment).

**Tests:**
- Vitest: components render correctly with mocked data.
- Playwright: navigate to admin, select an entity, see its triggers; test-fire an event, see matches.

**Acceptance:** Tests pass.

---

### Task 3.20 — Documentation and acceptance criteria

**Points:** 2
**Dependencies:** all preceding tasks
**Test layers:** none

**What:**
- Create `docs/dsl_reference.md` — full reference for the condition language and effect verb list. Includes:
  - Grammar (informal).
  - Path roots and their semantics.
  - Operator precedence and associativity.
  - Helper function reference.
  - Effect verb reference: each v1 verb's params and behavior.
  - Examples for common patterns ("trigger on damage taken", "tick-based effect", "conditional bonus").
- Update `AGENTS.md`: add "Trigger / Effect DSL" section under "Data Models". Add to "What NOT to Do":
  - "No new hardcoded reactive logic in scenes/engine. Express it as a trigger."
  - "No new event types without considering whether triggers should subscribe."
- Update `CLAUDE.md` "Completed Subsystems" with trigger/effect engine.
- Append Phase 3 entries to `docs/acceptance_criteria.md`.

**Acceptance:** All documents updated.

---

## 4. Phase completion criteria

Phase 3 is complete when *all* of the following hold:

1. All 20 tasks above are implemented and committed.
2. Full existing test suite passes; new tests added by Phase 3 raise the total.
3. The condition DSL parses all documented forms; the parser has no third-party dependencies.
4. The effect verb registry contains the nine v1 verbs and accepts new verbs from code-bearing pack registration.
5. Triggers attached to traits and status effects fire correctly when the relevant events occur.
6. Phase 2 modifier conditions continue to work via desugaring; new `condition_expr` form also works.
7. The "Poisoned" status effect ticks damage and applies a stat penalty entirely declaratively.
8. The chosen Godbound Gift demonstrates declarative authoring of a non-trivial Gift.
9. `docs/dsl_reference.md` exists and is comprehensive enough to author content without reading source.
10. `CLAUDE.md`, `AGENTS.md`, and `docs/acceptance_criteria.md` are updated.

## 5. Phase 3 deferrals (append to overview §11)

- **Conditional effect steps.** "If condition do A else do B" within a single effect list. Explicitly out of scope per §1; if real content needs it, a future cycle adds an `if`/`else` effect verb that takes a condition string and two sub-effect-lists.
- **Loops or user-defined functions in DSL.** Will not be added.
- **DSL versioning.** Phase 3 ships v1. Future DSL changes need migration; Phase 3 doesn't pre-build that.
- **Trigger priority.** v1 fires triggers in registration order with a stable tiebreak. Future cycle could add explicit priority if needed.
- **Pre-commit modifier triggers.** Triggers fire on post-commit events only. Phase 2's modifier framework already handles "contribute a modifier to a resolution"; Phase 3 doesn't add a parallel pre-commit trigger system. (Per AGENTS.md Rule 3 — multi-contributor resolutions go through resolver pipelines, not pre-commit events.)
- **Items as trigger sources.** Items continue to flow through existing combat code. Future cycle migrates them.
- **World-mutation verbs.** `create_settlement`, `shift_weather_pattern`, etc. — wait until target subsystems exist.
- **Effort commitment mechanics.** "Commit Effort for the scene" requires a per-scene resource lifetime concept. Currently `change_resource` works for scene-end full release, but per-scene commitment vs. per-day vs. permanent commitment as distinct resource states needs more thought. Future cycle.
- **DSL debugging tools beyond the trigger inspector.** Step-through trigger execution, condition evaluation traces, effect execution logs as a UI feature. Future cycle.

## 6. Notes for the coding agent

- Phase 3 has the highest implementation depth per task in the cycle. Tasks 3.3 (parser), 3.4 (path resolver), and 3.13 (registry) are the meaty ones. Don't rush them.
- The condition DSL parser is hand-written. Resist the urge to add a parser library unless something genuinely intractable comes up. The grammar is small and a recursive-descent implementation is well within the scope of a single session.
- The effect verb set is *deliberately small*. Resist content-pressure to add new verbs during Phase 3 implementation. New verbs from real content needs are normal; new verbs from "this would be cool" are scope creep. Each new verb is a new task, not a quick add.
- The `local.<name>` path root in Task 3.12 is the only "shared state across effects" mechanism. It's deliberately limited — values bind once, can be read by subsequent effects in the same `do:` list, and don't persist beyond the trigger execution. Don't generalize this into a full variable-binding-with-scope feature.
- Status effect upgrade (Task 3.17) and Godbound Gift (Task 3.18) are the cycle-level proofs. If either feels forced — if you're inventing pack-content gymnastics to fit the framework rather than expressing real source content — that's a framework gap. Surface it explicitly; don't paper over it.
- Per AGENTS.md, every task uses the four-layer test rule. Mark task layer requirements in commits.
- After every task, run the full test suite. Commit with `[Phase 3 / Task 3.N]` for traceability.
