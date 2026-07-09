# Modular Rules Architecture — Phase 2: Modifiers, Traits, and Resources

**Date:** 2026-04-26
**Status:** Draft
**Cycle:** Modular Rules Architecture
**Overview:** `2026-04-26-modular-rules-architecture-overview.md`
**Depends on:** Phase 0 complete. Phase 1 recommended (provides ContentService usage patterns and status effect framework that Phase 2 builds on), but not strictly required for the modifier/trait/resource frameworks themselves.
**Prerequisites for tasks:** Read overview, Phase 0, and Phase 1 specs. Read `AGENTS.md` for coding standards.

---

## 1. Phase scope

Phase 2 builds the three mechanic frameworks that unlock the bulk of source-system content imports: modifiers, traits, and resources. After Phase 2, content packs can express "the character has X feature, which grants +Y bonus to Z under condition W" and "the character has resource pool Q with these rules" entirely as data. This is the largest single content-unlock in the cycle.

The frameworks are interlocked:

- **Modifier framework** is the foundation. Anything that says "+/− N to some quantity under some condition" goes here. Used by traits, status effects (in a future phase), items (in a future phase), and any other source.
- **Trait/feature framework** sits on top of modifiers. A trait is "a thing an entity has" that contributes modifiers (and, in Phase 3, triggers). Edges, Advantages, Disadvantages, Feats, Class Features, racial abilities, and Godbound Gifts all map to traits.
- **Resource service** generalizes "a quantity an entity has with a max and rules for changing." HP, gold, Bennies, Effort, future Dominion — all instances of a single subsystem.

Phase 2 also performs the **HP and gold migration**: the existing `Character.hp`, `Character.max_hp`, and `Character.gold` fields are replaced (or backed) by `ResourceService` instances. This is the most error-prone task in the phase and is staged carefully.

### What Phase 2 produces

- Modifier schema, registry, and resolver. Modifiers can be unconditional or conditional on a small set of supported predicates (entity tag, target tag, trait possession). Phase 3 extends the condition language to the full trigger/effect DSL.
- Tag system: entities carry tags; traits and status effects can add tags dynamically; modifier conditions can check tags.
- Trait/feature schema, service, and persistence. Traits attach to entities, contribute modifiers, declare prerequisites and conflicts.
- Resource schema, service, and persistence. Resources are content records; per-entity instances live in `entity_resources`.
- HP, max_hp, and gold refactored as resource instances. Combat reads/writes HP through `ResourceService`. Shopping reads/writes gold through `ResourceService`.
- Test-case content imports demonstrating the frameworks work:
  - A representative slice of GURPS Advantages and Disadvantages as trait records.
  - A representative Godbound Gift as a trait record (modifier-only; triggered behavior lands in Phase 3).
  - Bennies defined as a content-only resource (no Python required).
  - Effort defined as a resource (the scene-day-permanent commitment mechanic stub; full Effort behavior is Phase 3 work).
- Frontend: a traits panel on the character sidebar; resource bars driven by `ResourceService`.

### What Phase 2 does not do

- **No full condition DSL.** Phase 2 conditions are limited to: `always`, `entity_has_tag(X)`, `target_has_tag(X)`, `entity_has_trait(X)`. Anything richer (numeric comparison, boolean combinators with arbitrary depth, event payload access) waits for Phase 3.
- **No triggered behavior on traits.** Traits in Phase 2 ship with modifier lists only. The `triggers` field exists on the schema as a forward-compat stub, but the runner that executes triggers is Phase 3.
- **No active resource mechanics.** Bennies are defined; "spend a Benny to reroll" is not implemented. Effort is defined; "commit Effort for the scene" is not implemented. Both wait for Phase 3.
- **No item-as-modifier-source.** Items currently grant equipment effects through the existing combat code path; Phase 2 doesn't refactor items into the modifier framework. That's a future cycle. (HP and gold get the migration treatment because they're cleaner; items are a larger refactor.)
- **No status-effect-to-modifier integration end-to-end.** Status effects from Phase 1 don't yet contribute modifiers in Phase 2. That integration is small, but it's listed as a Phase 2 deferred item to keep this phase focused. Phase 3 picks it up alongside trigger integration.

## 2. Decisions locked in this phase

- **Modifier targets are namespaced strings.** Format: `<domain>.<key>`. Examples: `attribute.str`, `skill.stab`, `resource.hp.max`, `combat.attack_roll`, `combat.ac`. The exact target taxonomy is curated centrally and grows as needed.
- **Modifier stacking modes:** `additive` (default), `multiplicative`, `replace`, `max`, `min`. Resolution combines additives by sum, multiplicatives by product (compounding), replace by priority order, max/min by the corresponding aggregation.
- **Phase 2 condition predicates:** `always`, `entity_has_tag(tag)`, `target_has_tag(tag)`, `entity_has_trait(qualified_id)`. No boolean combinators in Phase 2 (a future phase adds `and`/`or`/`not`).
- **Tags on entities:** stored as a `tags: list[str]` field on Character/NPC entities, plus a dynamic-tags mechanism (status effects and traits can contribute tags to an entity at runtime via the modifier resolution context).
- **Trait storage:** entity has `traits: list[str]` field — list of qualified trait IDs. Trait records live at `packs/<pack-id>/content/traits/<category>/<slug>.yaml`.
- **Resource storage:** content records at `packs/<pack-id>/content/resources/<slug>.yaml`. Per-entity instances in new SQLite table `entity_resources`.
- **HP and gold migration approach:** *both* the existing `Character.hp`/`max_hp`/`gold` fields *and* the new `entity_resources` rows are maintained in parallel during the migration. A future cycle removes the legacy fields. This is the safer staging — readers can switch to `ResourceService` incrementally.

## 3. Tasks

Test layer notation: **[U]** pytest unit, **[P]** Hypothesis property, **[M]** mutmut mutation, **[E2E]** Playwright, **[V]** Vitest unit, **[FC]** fast-check property, **[S]** Stryker mutation.

---

### Task 2.1 — Codebase audit for HP, gold, traits-shaped state, and modifier-shaped logic

**Points:** 2
**Dependencies:** Phase 0 complete
**Test layers:** none (investigation)

**What:** Examine the codebase to inventory what becomes resource-managed, what becomes trait-managed, and where modifier-shaped logic currently lives in ad-hoc form.

**Procedure:**
1. Find every read of `character.hp`, `character.max_hp`, `character.gold` in `src/harsh_realm/`. Classify each as: combat damage write, healing read/write, shopping read/write, sidebar display read, save-throw read, encumbrance read.
2. Find every write to those fields. Classify as: damage application, healing, level-up, character creation, shopping transaction, debug/admin command.
3. Find every "if entity has X then bonus Y" pattern in engine code. These are candidate modifiers. Examples: class abilities, Veteran's Luck (Warrior), expert reroll (Expert), house-rule practice skills.
4. Find every "feature/ability/talent" data structure. Currently, character class abilities are likely encoded directly in class YAML. List them as candidate trait records.
5. Identify the existing resource-shaped state beyond HP and gold: XP, encumbrance load, ammo (M4.6), system strain (if any). For each, decide whether Phase 2 migrates it or defers. Recommendation: only HP and gold migrate in Phase 2. XP, encumbrance, and ammo defer.
6. Identify the world-clock tick subscribers that would benefit from resource regeneration (e.g., HP regeneration on rest). Currently rest-based healing exists in `engine/healing.py`; document its current behavior so the resource service can preserve it.

**Deliverable:** `docs/superpowers/specs/2026-04-26-phase-2-codebase-audit.md` — markdown report with the above six sections.

**Acceptance:** Audit document exists, lists every HP/gold mutation site, lists every modifier-shaped pattern, identifies regeneration hooks.

---

### Task 2.2 — Modifier schema and condition predicates

**Points:** 2
**Dependencies:** 2.1
**Test layers:** [U] [P]

**What:** Pydantic models for `Modifier`, `ModifierSource`, and the small set of Phase 2 condition predicates.

**File:** `src/harsh_realm/modifiers/schema.py` (new)

**Models:**

```python
class Modifier(BaseModel):
    """A single modifier contribution from some source."""
    model_config = ConfigDict(frozen=True)

    target: str = Field(description="Namespaced target, e.g. 'attribute.str'")
    value: int = Field(description="Magnitude; sign indicates bonus or penalty")
    stacking: Literal[
        "additive", "multiplicative", "replace", "max", "min"
    ] = "additive"
    priority: int = Field(default=0, description="For 'replace' mode; higher wins")
    condition: ModifierCondition = Field(
        default_factory=lambda: ModifierCondition(predicate="always"),
    )
    description: str = ""


class ModifierCondition(BaseModel):
    """A Phase 2 condition predicate. Phase 3 expands the language."""
    model_config = ConfigDict(frozen=True)

    predicate: Literal[
        "always",
        "entity_has_tag",
        "target_has_tag",
        "entity_has_trait",
    ]
    arg: str | None = None  # required for non-'always' predicates


class ModifierContribution(BaseModel):
    """Resolved-time view of a modifier with its source identified."""
    model_config = ConfigDict(frozen=True)

    modifier: Modifier
    source_type: Literal["trait", "status_effect", "item", "situational"]
    source_id: str   # qualified ID of the source content record or runtime instance
```

**Validation rules:**
- Non-`always` predicates require non-empty `arg`.
- `target` field non-empty.
- `priority` only meaningful for `replace`; document but don't error if used elsewhere.

**Tests:** `tests/modifiers/test_schema.py`
- Valid modifier records parse for each stacking mode.
- Condition without required `arg` raises `ValidationError`.
- Property test: any valid modifier with `predicate="always"` accepts arbitrary contexts.

**Acceptance:** Tests pass.

---

### Task 2.3 — Modifier resolution context

**Points:** 1
**Dependencies:** 2.2
**Test layers:** [U]

**What:** A typed resolution context that condition predicates evaluate against.

**File:** `src/harsh_realm/modifiers/context.py` (new)

**Model:**

```python
class ResolutionContext(BaseModel):
    """The state needed to evaluate a modifier query."""
    model_config = ConfigDict(frozen=True)

    target: str = Field(description="What's being modified, e.g. 'skill.stab'")
    entity_id: str
    entity_tags: frozenset[str] = Field(default_factory=frozenset)
    entity_traits: frozenset[str] = Field(default_factory=frozenset)
    target_tags: frozenset[str] = Field(default_factory=frozenset)
    extra: dict[str, str | int | bool] = Field(default_factory=dict)


def evaluate_condition(
    condition: ModifierCondition,
    ctx: ResolutionContext,
) -> bool: ...
```

`evaluate_condition` is pure; it takes a condition and a context and returns whether the condition holds. It is the only place predicate semantics live in Phase 2 — Phase 3 expands this function (or replaces it with a richer evaluator) without changing the modifier model.

**Tests:** `tests/modifiers/test_context.py`
- `always` predicate: always true.
- `entity_has_tag("undead")`: true iff context has the tag.
- `target_has_tag` and `entity_has_trait`: same pattern.
- Each predicate with missing `arg` raises `ValueError` (defensive, even though schema validation should prevent it).

**Acceptance:** Tests pass.

---

### Task 2.4 — Modifier service and resolver

**Points:** 3
**Dependencies:** 2.3, Phase 0 ContentService, Phase 1 StatusEffectService (if Phase 1 is complete)
**Test layers:** [U] [P]

**What:** The service that collects modifiers from all sources and resolves a modifier query against a context.

**File:** `src/harsh_realm/modifiers/service.py` (new)

**API:**

```python
class ModifierService:
    """Collects modifiers from sources and resolves queries."""
    PERSISTENCE = "derivable"

    def __init__(
        self,
        content: ContentService,
        trait_service: TraitService,
        status_service: StatusEffectService | None,
    ) -> None: ...

    async def collect(
        self,
        ctx: ResolutionContext,
    ) -> list[ModifierContribution]:
        """Gather all modifiers from all sources whose conditions hold for ctx.
        Filters by target match. Does not aggregate — caller decides how."""

    async def resolve(
        self,
        ctx: ResolutionContext,
        base_value: int = 0,
    ) -> ResolvedModifierResult:
        """Convenience: collect, group by stacking mode, apply, return final."""


class ResolvedModifierResult(BaseModel):
    model_config = ConfigDict(frozen=True)
    base_value: int
    contributions: list[ModifierContribution]
    final_value: int
```

**Resolution algorithm:**
1. `collect` walks the entity's traits (via `TraitService`) and status effects (via `StatusEffectService` if available), filters to those targeting `ctx.target`, evaluates each modifier's condition, and returns the list.
2. `resolve` groups contributions by stacking mode:
   - `additive`: sum values, add to base.
   - `multiplicative`: multiply running total by `(1 + value/100)` (treating value as a percent) — confirm semantics in audit; alternative is direct multiplier. **Decision: value is treated as +N percent. So `value=10` means ×1.10.** Document in code comments.
   - `replace`: take highest-priority replacement; if any replace contributes, the final value is the replace value (others ignored).
   - `max`: final at least the highest `max` modifier value.
   - `min`: final at most the lowest `min` modifier value.
3. Order of application: replace → additive → multiplicative → max/min clamps. Documented and tested.

**Tests:** `tests/modifiers/test_service.py`
- Empty trait + status set → `resolve` returns base value with no contributions.
- One trait with one additive modifier → final = base + value.
- Two additives sum.
- Multiplicative compounds.
- Replace overrides additive.
- Max clamp: additive of +5, max modifier of 3 → final = 3.
- Property test: `resolve` with no replace/max/min modifiers gives `base + sum(additives) * product(multipliers)`.

**Acceptance:** Tests pass. The service marks itself `PERSISTENCE = "derivable"` per Rule 4 — it doesn't persist; it computes on demand.

---

### Task 2.5 — Tag system on entities

**Points:** 2
**Dependencies:** 2.3
**Test layers:** [U]

**What:** Entities carry tags. Tags are stored on Character/NPC and contributed dynamically by status effects and traits.

**Files:**
- `src/harsh_realm/models/character.py` (extend)
- `src/harsh_realm/models/npc.py` (extend)
- `src/harsh_realm/tags/service.py` (new)

**Model changes:**
- `Character` and `NPC` gain `tags: list[str] = Field(default_factory=list)`.

**Service:**

```python
class TagService:
    """Resolves the full tag set for an entity at a moment in time."""
    PERSISTENCE = "derivable"

    def __init__(
        self,
        entity_repo: EntityRepository,
        trait_service: TraitService,
        status_service: StatusEffectService | None,
    ) -> None: ...

    async def get_tags(self, entity_id: str) -> frozenset[str]:
        """Union of: entity's static tags, tags from active traits,
        tags from active status effects."""
```

For Phase 2: a trait can carry a `provides_tags: list[str]` field; a status effect's content schema gains the same field (small addition to the Phase 1 status effect schema).

**Tests:**
- Entity with static tags returns those tags.
- Entity with a trait that provides tags returns the union.
- Entity with a status effect that provides tags returns the union.
- Removing a status effect removes its tags from subsequent queries.

**Acceptance:** Tests pass. `ResolutionContext.entity_tags` is populated by `TagService.get_tags()` at the call site.

---

### Task 2.6 — Trait content schema

**Points:** 1
**Dependencies:** 2.2
**Test layers:** [U]

**What:** Pydantic model for a trait record.

**File:** `src/harsh_realm/traits/schema.py` (new)

**Model:**

```python
class Trait(BaseModel):
    """A trait/feature/edge/advantage/gift/etc. content record."""
    model_config = ConfigDict(frozen=True)

    id: str
    name: str
    description: str = ""
    category: str = Field(
        description="advantage, disadvantage, edge, hindrance, gift, feat, talent, "
                    "class_feature, racial, ..."
    )
    modifiers: list[Modifier] = Field(default_factory=list)
    triggers: list[dict] = Field(
        default_factory=list,
        description="Trigger records — Phase 2 stores them but does not execute. "
                    "Phase 3 adds the runner.",
    )
    provides_tags: list[str] = Field(default_factory=list)
    prerequisites: list[Prerequisite] = Field(default_factory=list)
    conflicts: list[str] = Field(
        default_factory=list,
        description="Qualified trait IDs that cannot coexist with this trait.",
    )
    cost: TraitCost | None = None
    tags: list[str] = Field(default_factory=list)


class Prerequisite(BaseModel):
    model_config = ConfigDict(frozen=True)
    kind: Literal["trait", "attribute_min", "level_min", "skill_min"]
    arg: str            # trait ID, attribute name, etc.
    value: int = 0      # min level for attribute/level/skill prerequisites


class TraitCost(BaseModel):
    """How acquiring this trait costs the character. Game-system-specific."""
    model_config = ConfigDict(frozen=True)
    points: int = 0
    slot: str | None = None    # e.g., "edge_slot", "gift_slot"
    description: str = ""
```

**Tests:**
- Valid GURPS Advantage parses (e.g., one with attribute_min prerequisite and additive modifier).
- Valid Godbound Gift parses (modifier list, optional triggers stub).
- Trait with prerequisite of unknown `kind` fails validation.

**Acceptance:** Tests pass.

---

### Task 2.7 — Trait service and entity-trait persistence

**Points:** 2
**Dependencies:** 2.6
**Test layers:** [U] [P]

**What:** A service for managing an entity's traits.

**Files:**
- `src/harsh_realm/traits/service.py` (new)
- `src/harsh_realm/models/character.py` (extend with `traits: list[str]` field)
- `src/harsh_realm/models/npc.py` (extend with `traits: list[str]` field)

**API:**

```python
class TraitService:
    """Per-entity trait lookup and management."""
    PERSISTENCE = "durable"   # via entity data JSON

    def __init__(
        self,
        content: ContentService,
        entity_repo: EntityRepository,
    ) -> None: ...

    async def get_traits(self, entity_id: str) -> list[Trait]:
        """Resolved trait records for an entity."""

    async def add_trait(
        self,
        entity_id: str,
        trait_id: str,
        check_prerequisites: bool = True,
    ) -> None: ...

    async def remove_trait(self, entity_id: str, trait_id: str) -> None: ...

    async def list_modifiers(self, entity_id: str) -> list[ModifierContribution]:
        """Flat list of all modifiers from all of the entity's traits."""

    async def check_prerequisites(
        self,
        entity_id: str,
        trait: Trait,
    ) -> list[Prerequisite]:
        """Returns list of unmet prerequisites; empty if all met."""
```

**Persistence note:** trait IDs live in the entity's `data` JSON column under `traits`. This is intrinsic per AGENTS.md Rule 1 — a trait is part of "what the entity is." The trait *records* (definitions) live in pack content; the entity stores only the IDs.

**Tests:**
- `add_trait` with met prerequisites succeeds; entity's `traits` list now contains the ID.
- `add_trait` with unmet prerequisites raises `PrerequisiteNotMetError`.
- `add_trait` with a conflicting existing trait raises `TraitConflictError`.
- `remove_trait` removes from entity.
- `get_traits` resolves all IDs through `ContentService`.
- Property test: `add_trait` followed by `remove_trait` for the same ID leaves the entity unchanged.

**Acceptance:** Tests pass.

---

### Task 2.8 — Trait CRUD admin endpoints

**Points:** 2
**Dependencies:** 2.7, Phase 0 override routes
**Test layers:** [U]

**What:** REST endpoints for viewing and editing trait records (with override semantics) and for managing entity-trait assignments.

**Endpoints:**
- `GET /api/world/traits` → list all trait records.
- `GET /api/world/traits/<qualified_id>` → single trait.
- `PUT /api/world/traits/<qualified_id>` → upsert override.
- `DELETE /api/world/traits/<qualified_id>` → revert.
- `GET /api/entities/<id>/traits` → list entity's traits (resolved).
- `POST /api/entities/<id>/traits/<qualified_id>` → add trait to entity (validates prerequisites).
- `DELETE /api/entities/<id>/traits/<qualified_id>` → remove trait from entity.

**Tests:**
- All endpoints round-trip correctly.
- Adding a trait with unmet prerequisites returns 400 with the prerequisite list.

**Acceptance:** Tests pass.

---

### Task 2.9 — GURPS Advantages/Disadvantages slice imported

**Points:** 2
**Dependencies:** 2.8
**Test layers:** [U]

**What:** Phase 2's first content test case. Create a `gurps-traits` pack containing a representative slice of GURPS Advantages and Disadvantages — modifier-shaped only (no triggered behaviors).

**Suggested slice (10–15 traits):**
- Advantages: Acute Hearing, Acute Vision, Combat Reflexes, High Pain Threshold, Toughness, Strong Will, Hard to Kill (passive form), Reputation (positive).
- Disadvantages: Bad Sight, Hard of Hearing, Low Pain Threshold, Bad Temper (modifier form), Code of Honor (modifier-only stub).

These are the GURPS Advantages whose mechanics are pure modifiers — no event-driven behavior. Active or triggered Advantages (Combat Reflexes' "never surprised on a tie", Luck's "reroll once per hour") wait for Phase 3.

**Files:**
- `packs/gurps-traits/pack.yaml`
- `packs/gurps-traits/content/traits/advantages/<slug>.yaml` × N
- `packs/gurps-traits/content/traits/disadvantages/<slug>.yaml` × N

**Pack manifest:**

```yaml
id: gurps-traits
version: 0.1.0
name: GURPS Traits (Modifier Subset)
description: |
  A representative slice of GURPS Advantages and Disadvantages, restricted to
  passive modifier mechanics. Triggered and active variants will be added once
  the trigger/effect engine ships.
authors: [Harsh Realm Project]
depends:
  - id: xwn-core
    version: ">=1.0.0"
provides: [gurps-traits-modifier-subset]
```

**Source-encoding note:** trait records cite their source page reference in a `source` field on each YAML record (recommended; not required by schema). Original GURPS material is intellectual property; encoded versions for personal use within Harsh Realm are fine for a single-user system, but document that pack distribution outside personal use is a future concern.

**Tests:**
- Pack loads cleanly.
- A character with `gurps-traits:advantages.combat_reflexes` resolves to the expected modifier contribution when queried.
- A character with `gurps-traits:disadvantages.bad_sight` resolves to a vision-related penalty when queried.

**Acceptance:** Tests pass.

---

### Task 2.10 — Godbound Gift imported as a trait

**Points:** 2
**Dependencies:** 2.8
**Test layers:** [U]

**What:** Phase 2's second content test case. Create a `godbound-base` pack stub with at least one Gift expressed as a trait. Phase 2 supports the modifier portion only; triggers are added in Phase 3 and downstream Words/Dominion content lands in a future cycle.

**Suggested Gift:** "The Sun's Authority" or similar — a Gift whose Phase-2-expressible portion is "+N to skill checks against creatures with tag `darkness`" or equivalent. Pick one whose pure-modifier subset is meaningful even without triggered behavior.

**Files:**
- `packs/godbound-base/pack.yaml`
- `packs/godbound-base/content/traits/gifts/<word>.<gift>.yaml`
- (Note: `godbound-base` is a stub pack here. It only contains the test trait. Future cycles fill it in with Words, full Gift listings, Dominion mechanics, etc.)

**Tests:**
- Pack loads.
- A character with the Gift gets the expected modifier when querying against a darkness-tagged target.

**Acceptance:** Tests pass.

---

### Task 2.11 — Resource content schema

**Points:** 1
**Dependencies:** Phase 0 Pack loader
**Test layers:** [U]

**What:** Pydantic model for resource records.

**File:** `src/harsh_realm/resources/schema.py` (new)

**Model:**

```python
class Resource(BaseModel):
    """A resource type definition (HP, gold, Bennies, Effort, etc.)."""
    model_config = ConfigDict(frozen=True)

    id: str
    name: str
    description: str = ""
    default_max: int | None = Field(
        default=None,
        description="Default max for new instances. None means no max (e.g., gold).",
    )
    default_current: int | str = Field(
        default="max",
        description="'max' means start full; an int means start at that value.",
    )
    can_go_negative: bool = False
    regeneration: ResourceRegeneration | None = None
    zero_event: str | None = Field(
        default=None,
        description="Event type to emit when current reaches 0.",
    )
    full_event: str | None = Field(
        default=None,
        description="Event type to emit when current reaches max.",
    )
    tags: list[str] = Field(default_factory=list)


class ResourceRegeneration(BaseModel):
    """Passive regeneration rule for a resource."""
    model_config = ConfigDict(frozen=True)
    rate: int = Field(description="Amount per tick interval")
    interval_ticks: int = Field(ge=1, default=1)
    condition_predicate: ModifierCondition = Field(
        default_factory=lambda: ModifierCondition(predicate="always"),
    )
```

**Tests:**
- Valid HP-shaped resource parses.
- Valid gold-shaped resource (no max, no regen) parses.
- Negative `interval_ticks` raises validation error.

**Acceptance:** Tests pass.

---

### Task 2.12 — Resource database table and repository

**Points:** 2
**Dependencies:** 2.11
**Test layers:** [U]

**What:** SQLite table and repository for per-entity resource instances.

**Files:**
- `src/harsh_realm/db.py` (extend `_init_schema`)
- `src/harsh_realm/resources/repository.py` (new)

**Schema:**

```sql
CREATE TABLE entity_resources (
    entity_id           TEXT NOT NULL,
    resource_id         TEXT NOT NULL,           -- qualified ID, e.g. "xwn-core:resource.hp"
    current             INTEGER NOT NULL,
    max                 INTEGER,                 -- NULL means no max
    last_regen_tick     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_id, resource_id)
);

CREATE INDEX idx_entity_resources_entity ON entity_resources(entity_id);
```

**Repository API:**

```python
class ResourceRepository:
    PERSISTENCE = "durable"

    async def get(self, entity_id: str, resource_id: str) -> ResourceInstance | None: ...
    async def set(self, instance: ResourceInstance) -> None: ...
    async def list_for_entity(self, entity_id: str) -> list[ResourceInstance]: ...
    async def delete(self, entity_id: str, resource_id: str) -> bool: ...


class ResourceInstance(BaseModel):
    model_config = ConfigDict(frozen=True)
    entity_id: str
    resource_id: str
    current: int
    max: int | None
    last_regen_tick: int = 0
```

**Tests:**
- Insert, read, update, delete round-trip.
- `list_for_entity` returns all resources for that entity.

**Acceptance:** Tests pass.

---

### Task 2.13 — ResourceService

**Points:** 3
**Dependencies:** 2.12, Phase 0 ContentService
**Test layers:** [U] [P]

**What:** The service that owns resource lifecycle: initialization, change, regeneration, zero/full events.

**File:** `src/harsh_realm/resources/service.py` (new)

**API:**

```python
class ResourceService:
    PERSISTENCE = "durable"

    def __init__(
        self,
        repo: ResourceRepository,
        content: ContentService,
        clock: WorldClock,
        event_bus: EventBus,
    ) -> None: ...

    async def ensure_instance(
        self,
        entity_id: str,
        resource_id: str,
        max_override: int | None = None,
        current_override: int | None = None,
    ) -> ResourceInstance:
        """Create the instance if it doesn't exist, using resource defaults."""

    async def get(self, entity_id: str, resource_id: str) -> ResourceInstance | None: ...

    async def change(
        self,
        entity_id: str,
        resource_id: str,
        delta: int,
        source: str | None = None,
    ) -> ResourceChangeResult:
        """Apply a delta. Clamps to [0 or -inf, max]. Emits zero/full events
        if thresholds crossed."""

    async def set_current(
        self,
        entity_id: str,
        resource_id: str,
        current: int,
    ) -> ResourceInstance: ...

    async def set_max(
        self,
        entity_id: str,
        resource_id: str,
        max: int | None,
    ) -> ResourceInstance: ...

    async def tick_regeneration(self, current_tick: int) -> list[ResourceChangeResult]:
        """For every instance with regeneration, apply the regen rule if due."""

    async def list_for_entity(self, entity_id: str) -> list[ResourceInstance]: ...


class ResourceChangeResult(BaseModel):
    model_config = ConfigDict(frozen=True)
    entity_id: str
    resource_id: str
    previous_current: int
    new_current: int
    delta_applied: int
    events_emitted: list[str]
```

**Behavior notes:**
- `change` clamps to `[0, max]` by default; if `can_go_negative=True` on the resource, the lower bound is `-inf`. If `max=None`, the upper bound is `+inf`.
- Zero event emitted when `previous_current > 0` and `new_current <= 0`. Full event emitted when `previous_current < max` and `new_current >= max`.
- Regeneration: a tick handler subscribes to `world.tick_advanced` and calls `tick_regeneration`.

**Tests:**
- `ensure_instance` creates new instance with default max and current=max.
- `change` with delta past max clamps; emits full event.
- `change` with delta past 0 clamps; emits zero event.
- `tick_regeneration` advances regenerable resources.
- A resource with no regen is unaffected by tick.
- Property test: for a non-negative resource, `change` results never go below 0 regardless of delta.

**Acceptance:** Tests pass.

---

### Task 2.14 — HP migration: define HP as a resource and wire combat reads

**Points:** 3
**Dependencies:** 2.13, Task 2.1 audit
**Test layers:** [U]

**What:** The first half of the HP migration. HP is defined as a resource record in `xwn-core`. The `ResourceService` becomes the canonical write path for HP. Existing `Character.hp` and `max_hp` fields are preserved for backward compatibility but are now *projections* of the resource state — they are kept in sync but no longer the source of truth.

**Files:**
- `packs/xwn-core/content/resources/hp.yaml` (new)
- `src/harsh_realm/resources/sync.py` (new — keeps Character.hp/max_hp synced with ResourceService for now)
- Combat write paths updated to call `ResourceService.change` (which then syncs back to `Character.hp` via the sync layer)
- Combat read paths can stay unchanged (they read `Character.hp`, which the sync layer keeps current)

**HP resource record:**

```yaml
# packs/xwn-core/content/resources/hp.yaml
id: hp
name: Hit Points
description: |
  The character's resilience. Combat damage reduces HP. Reaching 0 triggers
  death or last-stand mechanics.
default_max: null   # Set per-character at creation from class HD
default_current: max
can_go_negative: false
zero_event: character.hp_zero
full_event: character.hp_full
regeneration: null    # Rest-based healing applied via separate mechanism
tags: [vital, combat]
```

**Sync layer:** a small adapter that subscribes to resource change events for `xwn-core:resource.hp` and updates the corresponding `Character.hp` / `Character.max_hp` field in the entity's data JSON. Reads continue to work against the entity model. This is a temporary measure; a future cycle removes the legacy fields entirely.

**Migration on world load:** for every existing character entity, ensure an HP resource instance exists. If the character has `hp` and `max_hp` in their data and no resource row, create the row from those values. This is a one-time backfill per world.

**Tests:**
- After migration, every character has an `entity_resources` row for HP.
- Combat damage call: `ResourceService.change(...)` updates DB, sync layer updates `Character.hp`.
- Combat reads `character.hp` and gets the same value as `ResourceService.get(...).current`.
- HP reaches 0 → `character.hp_zero` event emitted.

**Acceptance:** Existing combat tests pass without modification. New tests verify dual-write consistency.

---

### Task 2.15 — Gold migration

**Points:** 2
**Dependencies:** 2.13
**Test layers:** [U]

**What:** Same pattern as HP: define gold as a resource, route shopping writes through `ResourceService`, sync `Character.gold` for backward compatibility.

**Files:**
- `packs/xwn-core/content/resources/gold.yaml`
- Shopping handler updated to use `ResourceService.change`
- Sync extends to gold

**Gold resource record:**

```yaml
id: gold
name: Gold
description: Currency. The character's wealth in coin.
default_max: null
default_current: 0
can_go_negative: false
tags: [currency]
```

**Tests:**
- Shopping purchase: `ResourceService.change(... delta=-100 ...)` updates DB, sync updates `Character.gold`.
- Shopping sale: positive delta, similarly.
- Insufficient gold (would go negative): `change` raises `ResourceInsufficientError`.

**Acceptance:** Existing shopping tests pass.

---

### Task 2.16 — Bennies as a content-only resource

**Points:** 1
**Dependencies:** 2.13
**Test layers:** [U]

**What:** Phase 2's third content test case. Define Bennies as a resource record in a new pack with no Python — purely YAML. Demonstrates the resource service supports content-only definitions.

**Files:**
- `packs/savage-trappings/pack.yaml`
- `packs/savage-trappings/content/resources/bennies.yaml`

**Bennies resource:**

```yaml
id: bennies
name: Bennies
description: |
  Narrative currency, reroll fuel. Awarded for good play; spent to influence
  outcomes.
default_max: 5
default_current: 3
can_go_negative: false
zero_event: bennies.depleted
full_event: bennies.full
tags: [narrative, savage]
```

(The actual *use* of Bennies — "spend to reroll" — requires the trigger/effect engine in Phase 3. Phase 2 just defines the resource; characters in worlds with `savage-trappings` enabled have a Benny pool, but nothing changes it yet beyond admin commands.)

**Pack manifest:** depends on `xwn-core@>=1.0.0`.

**Tests:**
- Pack loads.
- A character in a world with `savage-trappings` gets a Bennies resource instance with the right defaults.
- `ResourceService.change(... bennies, delta=-1 ...)` reduces the pool.

**Acceptance:** Tests pass.

---

### Task 2.17 — Effort as a content resource (Godbound prep)

**Points:** 1
**Dependencies:** 2.13
**Test layers:** [U]

**What:** Define Effort as a resource in `godbound-base` (the same pack that Task 2.10 stubbed). Phase 2 only defines the resource; the commitment/release mechanics (Effort committed for the scene, day, or permanently) are Phase 3 work via the trigger/effect engine.

**File:** `packs/godbound-base/content/resources/effort.yaml`

```yaml
id: effort
name: Effort
description: |
  The fuel of divine action. Committed for the scene, day, or permanently to
  empower Gifts. Recovers based on commitment type.
default_max: null   # Set per-character at creation: 1 + bonus from Words
default_current: max
can_go_negative: false
tags: [divine, godbound]
```

**Tests:**
- Resource loads.
- Character with Effort defined has the resource instance.

**Acceptance:** Tests pass.

---

### Task 2.18 — Frontend: traits panel on character sidebar

**Points:** 2
**Dependencies:** 2.8
**Test layers:** [V] [E2E]

**What:** Add a traits section to the character sidebar showing the player character's traits.

**Files:**
- `frontend/src/components/character/TraitsList.vue` (new)
- `frontend/src/components/StatusSidebar.vue` (extend to mount traits list)
- `frontend/src/types/api.ts` (add `Trait` type)
- `frontend/src/stores/game.ts` (extend with traits state)

**Behavior:**
- Fetch traits via `GET /api/entities/<player_id>/traits` on character load.
- Display each trait with name, category badge, and a tooltip showing the description.
- Click a trait → modal showing full description, modifiers, prerequisites, conflicts.

**Tests:**
- Vitest: traits list renders correctly with mocked data.
- Playwright: a test character with two traits shows both; clicking opens modal.

**Acceptance:** Tests pass.

---

### Task 2.19 — Frontend: resource bars driven by ResourceService

**Points:** 2
**Dependencies:** 2.14, 2.15
**Test layers:** [V] [E2E]

**What:** Refactor the existing HP and gold display in the sidebar to read from `ResourceService` data via a new `GET /api/entities/<id>/resources` endpoint. Add support for additional resources (Bennies, Effort) when the relevant packs are enabled.

**Files:**
- `frontend/src/components/character/ResourceBars.vue` (new — generic resource bar component)
- `frontend/src/components/StatusSidebar.vue` (replace HP/gold inline displays)
- `frontend/src/types/api.ts` (add `Resource` and `ResourceInstance` types)
- `frontend/src/stores/game.ts` (extend)

**Behavior:**
- Fetch resources via `GET /api/entities/<player_id>/resources`.
- For each resource, render a labeled bar (current / max) with appropriate visual styling.
- Bars without a max (gold) display as a number, not a bar.
- WebSocket events (resource changes from combat, shopping, etc.) update bars in real time.

**Tests:**
- Vitest: bar component renders correctly for various current/max combinations.
- Playwright: combat damages player, HP bar updates; shopping reduces gold display.

**Acceptance:** Tests pass.

---

### Task 2.20 — Documentation updates

**Points:** 1
**Dependencies:** all preceding tasks
**Test layers:** none

**What:**
- Update `AGENTS.md`: add a "Modifiers, Traits, and Resources" section describing the frameworks. Add to "What NOT to Do":
  - "No new feature/ability code in engine modules. New character abilities go in pack `traits/`."
  - "No new resource-shaped state on entity classes. Define a resource record."
- Update `CLAUDE.md` "Completed Subsystems" with modifier framework, trait service, resource service.
- Update `CLAUDE.md` "Known PLACEHOLDERs" to remove items resolved by this phase.

**Acceptance:** Documents updated.

---

### Task 2.21 — Acceptance criteria document update

**Points:** 1
**Dependencies:** all preceding tasks
**Test layers:** none

**What:** Append Phase 2 entries to `docs/acceptance_criteria.md`.

**Acceptance:** Document updated.

---

## 4. Phase completion criteria

Phase 2 is complete when *all* of the following hold:

1. All 21 tasks above are implemented and committed.
2. Full existing test suite passes; new tests added by Phase 2 raise the total.
3. The modifier framework can be queried for any entity and target, returning correct contributions from traits and (if Phase 1 is complete) status effects.
4. The trait framework supports add/remove with prerequisite and conflict checking. The `gurps-traits` pack and a `godbound-base` Gift demonstrate working trait content.
5. The resource service is the canonical write path for HP and gold. Combat damage, healing, shopping, and admin commands all route through it. `Character.hp`, `max_hp`, and `gold` remain as backward-compat projections.
6. Bennies and Effort are defined as content resources. Worlds with the appropriate packs gain those resource instances per character.
7. The character sidebar shows traits and resource bars driven by the new services.
8. `AGENTS.md`, `CLAUDE.md`, and `docs/acceptance_criteria.md` are updated.

## 5. Phase 2 deferrals (append to overview §11)

- **Status effect modifier integration.** Phase 1 status effects don't yet contribute modifiers in Phase 2. The integration is small but cross-phase. Picked up in Phase 3 alongside trigger integration: status effects with both modifier and trigger fields make sense as one cohesive change.
- **Items as modifier sources.** Equipped items currently route their stat effects through the existing combat code. Refactoring items into the modifier framework is a substantial change; deferred to a future cycle.
- **Boolean condition combinators.** Phase 2 conditions are flat predicates. Phase 3's full DSL adds `and`/`or`/`not` and richer comparisons.
- **Triggered behaviors on traits.** The `triggers` field on `Trait` is stored but not executed in Phase 2. Phase 3 adds the runner.
- **Active resource mechanics.** Spending Bennies, committing Effort — these are Phase 3+ via the trigger/effect engine.
- **XP, encumbrance, ammo migrations to ResourceService.** Listed in audit; deferred. HP and gold are the highest-traffic and best-defined; XP/encumbrance/ammo can migrate when their owning subsystems are touched for other reasons.
- **Removing legacy `Character.hp`/`max_hp`/`gold` fields.** Currently kept as backward-compat projections. A future cycle removes them entirely; Phase 2 adds the sync layer rather than pulling the rip cord.
- **Cross-pack trait conflict resolution beyond ID matching.** A future feature could allow packs to declare semantic conflicts ("don't let a character have both Tough and Frail").

## 6. Notes for the coding agent

- Phase 2 is the largest framework phase. Tackle it in the natural clusters: modifier (2.2–2.5) → traits (2.6–2.10) → resources (2.11–2.17) → frontend (2.18–2.19) → docs (2.20–2.21). Don't interleave clusters; finish one before starting the next.
- Task 2.14 (HP migration) is the highest-blast-radius task in Phase 2. Make a clean commit before starting; the dual-write approach (write through ResourceService, sync legacy field) is intentionally conservative. Resist the temptation to remove legacy fields in this phase — that's a separate task in a future cycle.
- The modifier service (2.4) is shaped like a resolver pipeline. This is intentional. When Phase 3's trigger/effect engine ships, the same pipeline pattern extends to event-driven modifiers and triggered effects.
- Conditions in Phase 2 are deliberately limited. If you find yourself wanting to write `if entity_has_tag(X) and resource(hp).current < 0.25 * resource(hp).max`, stop — that's Phase 3 territory. For Phase 2, encode the modifier as conditional on a single tag and let a (future) status effect or hand-coded handler manage the dynamic part.
- Trait content imports (Tasks 2.9, 2.10) are the proof that the framework is general. If a trait you want to import doesn't fit cleanly into the schema, surface it as a framework gap, not as content gymnastics. The schema should evolve to handle real content, not the content distort to fit a thin schema.
- Per AGENTS.md, every task uses the four-layer test rule. Mark task layer requirements in commits.
- After every task, run the full test suite. Commit with `[Phase 2 / Task 2.N]` for traceability.
