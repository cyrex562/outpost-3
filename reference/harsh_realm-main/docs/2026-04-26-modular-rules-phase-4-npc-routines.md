# Modular Rules Architecture — Phase 4: NPC Scheduled Routines (Deferred)

**Date:** 2026-04-26
**Status:** Draft — **NOT IMPLEMENTED THIS CYCLE**
**Cycle:** Modular Rules Architecture
**Overview:** `2026-04-26-modular-rules-architecture-overview.md`
**Depends on:** Phase 0 complete. Benefits from but does not require Phases 1–3.

> ⚠ **This phase is documented but not implemented in this cycle.**
> Phase 4 specs the FSM-with-scheduled-routines pattern for NPC behavior. Writing the spec now ensures (a) framework decisions in Phases 0–3 don't accidentally close off this direction, and (b) the eventual implementation cycle has a concrete starting point. **No tasks in this spec are executed in the current cycle.** When a future cycle picks up Phase 4, the agent should re-audit the codebase first (Task 4.1) since by then Phases 0–3 will have shipped and the surface area will have changed.

---

## 1. Phase scope

Phase 4 replaces the current "NPCs are static records that respond to player commands" model with NPCs that have *behaviors*: they follow daily routines, perceive events around them, and react to stimuli (interrupts) by transitioning between behavioral states. After Phase 4, a shopkeeper opens their shop in the morning, tends it during the day, drinks at the tavern in the evening, and goes home to sleep — all without player interaction. When the player walks in, the NPC interrupts the routine to interact, then returns to schedule afterward.

The mental model is **finite state machine with hierarchical states and a content-driven default routine**. Top-level states cover broad behavioral modes (Active / Engaged / Disabled). Sub-states cover specific activities. The default top-level state is Active, where the NPC follows a routine — a content-driven schedule of activities. Stimuli interrupt the routine to push the NPC into Engaged states (Social, Combat, Flee). Resolution returns to Active and the NPC re-anchors to where their schedule says they should be.

Phase 4 is intentionally **FSM-now, BT-later**. Behavior trees are a more flexible eventual destination, and the FSM design here is structured to migrate cleanly. Building a BT framework on day one is a much larger commitment with returns proportional to content volume that doesn't yet exist.

### What Phase 4 produces (when implemented)

- **NPC FSM framework:** state machine with hierarchical states, transition rules, and per-state behavior dispatch. Implemented in Python; no new DSL.
- **Routine schema and content:** YAML records describing daily schedules. A routine is a sequence of time-slotted activities. Activities are content-named handles to Python behavior implementations.
- **Activity implementations:** small Python modules that implement what an NPC actually does in each activity (tend_shop, drink_at_tavern, sleep, patrol, etc.). Activities live in `xwn-core` initially; new packs can register their own.
- **Perception model:** NPCs subscribe to a filtered subset of world events. Filters consider proximity (in-range only), awareness (line of sight, hearing), and per-state perception masks (a sleeping NPC has narrow perception).
- **Interrupt system:** stimuli pushed into the FSM cause state transitions per declared rules. Interrupts have priority ordering (combat outranks social).
- **Routine re-anchoring:** when an interrupt resolves, the FSM returns to the Active state and asks the routine system "where should I be right now?" The NPC may catch up, skip ahead, or transition to a fallback activity (e.g., if the shop's open hours are over, go home instead of resuming tend_shop).
- **Combat and Social scene integration:** the existing scenes read NPC state to determine how the NPC behaves (a routine-engaged shopkeeper interrupting to talk uses the Social scene; an aggrieved guard pursuing a player uses the Combat scene).
- **Test case:** a shopkeeper NPC follows a complete daily schedule. The player walks in mid-day, the shopkeeper interrupts to interact, the player leaves, and the shopkeeper resumes tending the shop. The player returns at night and finds the shop closed and the shopkeeper at home asleep.
- **Migration path documented to behavior trees** so the eventual move from FSM to BT doesn't require rewriting the routine/perception/interrupt model.

### What Phase 4 does not do (when implemented)

- **No utility AI or full goal-oriented action planning (GOAP).** NPCs follow scheduled routines, not dynamically scored goal selections. Drives, needs, and utility scoring are a future cycle.
- **No NPC-to-NPC interactions beyond pre-scripted ones.** Two shopkeepers don't dynamically socialize at the tavern; if the routine says "drink at tavern," the NPC just performs the drink_at_tavern activity. NPC-NPC dynamics are a future cycle.
- **No persistent memory beyond the existing entity data JSON.** "The NPC remembers you killed their cousin" is out of scope. Phase 4 NPCs have current state and the immediate interrupt context; long-term memory is a future cycle.
- **No procedural routine generation.** Routines are authored content, not rolled at NPC creation. A future cycle could add procedural routines tied to occupation tags.
- **No animation, pathfinding visualization, or detailed movement simulation.** NPCs occupy cells; movement between cells is via existing pathfinding (A* over the square grid). Within a cell, NPC position is abstract.
- **No DSL-authored NPC behavior.** Activities stay Python. The routine schedule is data, but what each activity *does* is code. (This was confirmed in design conversation — NPC behavior stays mostly Python.)
- **No routines for the player character.** Routines are an NPC concept. PC behavior is driven by player commands, not schedule.

## 2. Decisions locked in this phase

These were discussed before this spec was written and reflect the design intent. Future implementation work should hold these unless a strong reason to revise emerges during Task 4.1 (audit).

- **FSM, not BT, for v1.** Behavior trees are documented as the eventual evolution but not built in this phase.
- **Hierarchical states.** Top-level: Active, Engaged, Disabled. Active sub-states: Routine, Idle. Engaged sub-states: Social, Combat, Flee. Disabled sub-states: Unconscious, Dead, Removed.
- **Schedule unit:** ticks. The world clock already advances in ticks; routines are time-slotted in the same unit. A "day" is a configurable number of ticks (default: 24 representing one tick per hour, but the value lives in world config).
- **Routine assignment:** each NPC has a `routine_id` field referencing a routine record. Routines are pack content. Multiple NPCs can share a routine.
- **Activity dispatch:** an activity's name (e.g., `tend_shop`) is a key into a Python registry. Code-bearing packs register activity handlers via the existing `register(app_state)` hook.
- **Interrupt priority:** Combat > Flee > Social > routine. A higher-priority interrupt can override a lower-priority engagement (a shopkeeper in conversation switches to Combat state if attacked).
- **Re-anchoring policy:** after interrupt resolution, the NPC consults its routine for the current world tick and transitions to the matching activity. If the matching activity's location differs from the NPC's current cell, the NPC pathfinds toward that location as part of the activity.
- **Persistence per AGENTS.md Rule 4:** NPC FSM state is intrinsic ("what the NPC is currently doing") and lives on the NPC entity model. Routine *records* are pack content. Per-NPC routine progress (current activity, time-since-activity-started) is intrinsic and lives on the entity. No separate subsystem table needed for state itself; perception subscriptions and interrupt queue do justify their own tables.
- **Perception range:** measured in cells (Chebyshev distance on the square grid). Default range is 1 cell for sight, 2 for hearing. Per-NPC overrides via NPC traits.
- **NPC behavior is mostly Python; routines and perception filters are pack content.** The "NPC" content surface stays narrow: routines, activities (Python registrations), and trait/status content (already covered by Phases 2–3).

## 3. Tasks

> All tasks below are **deferred to a future implementation cycle**. They are sequenced and sized as if Phase 4 were active so the future work has a concrete plan. Story points and dependencies reflect intent; both may be revised at implementation time once Phases 0–3 have actually shipped.

Test layer notation: **[U]** pytest unit, **[P]** Hypothesis property, **[M]** mutmut mutation, **[E2E]** Playwright, **[V]** Vitest unit, **[FC]** fast-check property, **[S]** Stryker mutation.

---

### Task 4.1 — Codebase audit (when implementation starts)

**Points:** 2
**Dependencies:** Phase 0–3 complete (when implementation begins)
**Test layers:** none (investigation)

**What:** When this phase becomes active, the agent re-audits the codebase. Phases 0–3 will have changed substantial portions of the engine; this audit grounds the work in then-current reality.

**Procedure:**
1. List current NPC behavior surface: every place NPC state is read or written outside the entity repository, every place NPC actions are taken in scenes, every event NPCs currently emit or consume.
2. Identify the world-clock tick advancement: which subsystems already subscribe to `world.tick_advanced`. Phase 4's routine system becomes another subscriber.
3. Re-check the perception-shaped state in the codebase: the existing awareness check in combat (`AwarenessCheckResult`), the search system's discovery checks, any sight/hearing logic. Decide whether Phase 4 generalizes these or keeps them local.
4. Re-check Phase 3's trigger system. Determine whether NPC interrupts should be expressed as triggers (event-driven, condition-checked) or as a parallel system. Recommendation in §4: parallel system; triggers are for content reactions, perception/interrupts are for NPC state machinery.
5. Identify the Combat and Social scene entry/exit points. Routines that get interrupted into these scenes need clean handoff. The scenes already have entry/exit hooks; the audit confirms.
6. Verify that NPC entity data still has room for new fields (current_state, current_activity, activity_started_tick, interrupt_queue) without bloating the JSON column unreasonably. If it has, those fields land here. If not, an `npc_behavior_state` table may be cleaner.

**Deliverable:** Audit document at the path consistent with prior audits.

**Acceptance:** Audit document exists.

---

### Task 4.2 — FSM framework: state hierarchy and runtime

**Points:** 3
**Dependencies:** 4.1
**Test layers:** [U] [P]

**What:** A small FSM library scoped to NPC behavior. Hierarchical states, transition rules, and a runtime that drives transitions on tick or stimulus.

**File:** `src/harsh_realm/npc_behavior/fsm.py` (new)

**Design:**

```python
class NPCState(BaseModel):
    """A state in the NPC FSM. Hierarchical via parent_state."""
    model_config = ConfigDict(frozen=True)
    id: str                       # e.g., "active.routine.tend_shop"
    parent_state: str | None
    on_enter: str | None          # activity handler name to invoke on enter
    on_exit: str | None           # activity handler name to invoke on exit
    on_tick: str | None           # activity handler name to invoke each tick


class FSMTransition(BaseModel):
    """A transition rule."""
    model_config = ConfigDict(frozen=True)
    from_state: str | None        # None = any state
    to_state: str
    on_event: str | None          # event type that triggers the transition
    condition: str | None         # DSL condition expression (Phase 3 DSL, evaluated)
    priority: int = 0


class NPCFSMRuntime:
    """Drives one NPC through state transitions."""

    def __init__(
        self,
        npc_id: str,
        states: dict[str, NPCState],
        transitions: list[FSMTransition],
        activity_registry: ActivityRegistry,
        services: ServiceBundle,
    ) -> None: ...

    async def tick(self, current_tick: int) -> None:
        """Advance the FSM one tick. Calls on_tick handlers, evaluates
        time-based transitions."""

    async def handle_event(self, event: GameEvent) -> None:
        """Evaluate event-triggered transitions. May change state."""

    def current_state(self) -> str: ...
    def current_state_path(self) -> list[str]:
        """Returns [top, mid, leaf] for hierarchical state."""
```

**Hierarchy semantics:**
- A state path is dotted: `active.routine.tend_shop`.
- Entering a sub-state implies entering its parent if not already there. Same for on_exit unwinding.
- Transitions can target any state in the hierarchy; the runtime computes the entry/exit chain.
- Default transitions on `world.tick_advanced` use `condition` to express time-based logic (e.g., "current world hour ≥ 8" via the DSL).

**Tests:**
- Single-level FSM: transitions fire correctly on events.
- Hierarchical: entering a sub-state enters its parent first.
- Transition with condition: only fires when condition holds.
- Property test: for any sequence of valid events and ticks, the runtime ends in a reachable state.

**Acceptance:** Tests pass.

---

### Task 4.3 — Routine schema and registry

**Points:** 2
**Dependencies:** 4.2, Phase 0 ContentService
**Test layers:** [U]

**What:** Pydantic models for routine records and the registry that loads them from packs.

**File:** `src/harsh_realm/npc_behavior/routine.py` (new)

**Models:**

```python
class Routine(BaseModel):
    """A daily schedule for an NPC."""
    model_config = ConfigDict(frozen=True)
    id: str
    name: str
    description: str = ""
    day_length_ticks: int = 24    # override world default if needed
    schedule: list[ScheduleSlot]
    fallback_activity: str = "idle"   # used if schedule is empty or all slots skipped
    tags: list[str] = Field(default_factory=list)


class ScheduleSlot(BaseModel):
    model_config = ConfigDict(frozen=True)
    start_tick_offset: int        # within a day, when the slot begins
    duration_ticks: int           # how long the slot lasts
    activity: str                 # activity registry name
    location: str | None = None   # optional location tag (e.g. "shop", "home", "tavern")
    params: dict[str, str | int | bool] = Field(default_factory=dict)
```

**Routine registry:** loads Routine records from `ContentService` and resolves an NPC's `routine_id` to a Routine.

**Tests:**
- Valid routine parses.
- Schedule slots may overlap (last-wins) or be flagged invalid; spec ships with invalid (overlapping slots fail validation). Decision in implementation.
- Slot with `start_tick_offset >= day_length_ticks` fails validation.

**Acceptance:** Tests pass.

---

### Task 4.4 — NPC entity fields for behavior state

**Points:** 1
**Dependencies:** 4.1
**Test layers:** [U]

**What:** Add fields to the NPC entity model to track FSM state.

**File:** `src/harsh_realm/models/npc.py` (extend)

**New fields on `NPC`:**

```python
class NPC(BaseModel):
    # existing fields ...
    routine_id: str | None = None
    current_state: str = "active.routine"
    current_activity: str | None = None
    activity_started_tick: int | None = None
    last_interrupt: dict | None = None    # context from last interrupt for re-anchoring
```

These are intrinsic per Rule 1 — they describe what the NPC currently *is* doing.

**Tests:**
- New NPC defaults: state = active.routine, no activity, etc.
- Round-trip through SQLite preserves all behavior state.

**Acceptance:** Tests pass.

---

### Task 4.5 — Activity registry and core activity implementations

**Points:** 3
**Dependencies:** 4.4
**Test layers:** [U]

**What:** The Python registry for activity handlers, plus implementations of the core activities for the test-case shopkeeper.

**Files:**
- `src/harsh_realm/npc_behavior/activities/__init__.py` (registry)
- `src/harsh_realm/npc_behavior/activities/movement.py` (walk_to, walk_home — pathfinding wrappers)
- `src/harsh_realm/npc_behavior/activities/work.py` (tend_shop, open_shop, close_shop)
- `src/harsh_realm/npc_behavior/activities/leisure.py` (drink_at_tavern, eat_meal)
- `src/harsh_realm/npc_behavior/activities/rest.py` (sleep, idle)
- `packs/xwn-core/code/__init__.py` (extended to register the activities)

**Activity handler signature:**

```python
ActivityHandler = Callable[[NPCActivityContext], Awaitable[ActivityResult]]


class NPCActivityContext(BaseModel):
    model_config = ConfigDict(arbitrary_types_allowed=True, frozen=True)
    npc_id: str
    npc_state: NPCState
    schedule_slot: ScheduleSlot
    services: ServiceBundle
    world_state: WorldStateSnapshot
    tick_within_slot: int  # how far into the slot we are


class ActivityResult(BaseModel):
    model_config = ConfigDict(frozen=True)
    events_to_emit: list[GameEvent] = Field(default_factory=list)
    log_messages: list[str] = Field(default_factory=list)
    request_state_change: str | None = None  # if the activity wants to transition out
```

Activities are deliberately small. `tend_shop`'s implementation might be: "if any customer is in the shop and asking, hand off to Social scene by emitting `npc.engagement_requested`. Otherwise, log occasional flavor messages." `sleep` might be: "do nothing; respond to no perceivable events except direct attack." `walk_to` might be: "advance one cell along a pathfinding route toward the target location."

The activity registry has `register(name: str, handler: ActivityHandler)` and `get(name: str)`. Code-bearing packs register their own activities through this.

**Tests:**
- Each core activity executes without error in a controlled context.
- `tend_shop` recognizes a customer-in-shop event and emits engagement request.
- `sleep` ignores most events but responds to attack.

**Acceptance:** Tests pass.

---

### Task 4.6 — Perception system

**Points:** 3
**Dependencies:** 4.4
**Test layers:** [U]

**What:** A subsystem that decides which events an NPC perceives and queues them for the FSM runtime.

**File:** `src/harsh_realm/npc_behavior/perception.py` (new)

**Design:**

```python
class PerceptionFilter(BaseModel):
    """Per-state perception rules."""
    model_config = ConfigDict(frozen=True)
    sight_range_cells: int = 1
    hearing_range_cells: int = 2
    perceives_event_types: list[str] = Field(default_factory=list)


class PerceptionService:
    PERSISTENCE = "derivable"

    def __init__(
        self,
        npc_repo: NPCRepository,
        cell_repo: CellRepository,
        services: ServiceBundle,
    ) -> None: ...

    async def filter_for_npc(self, event: GameEvent, npc_id: str) -> bool:
        """Returns True if the NPC perceives this event."""
```

**Filtering rules:**
- Look up the NPC's current state.
- Look up the perception filter for that state (e.g., `sleep` has narrow perception, `patrol` has wider).
- Check event type against the state's `perceives_event_types`.
- If the event has spatial coordinates, check distance from NPC's cell against sight or hearing range (depending on event type).
- Return True/False.

Per-state perception filters are content (in `xwn-core/content/perception/`) keyed by state path.

**Tests:**
- A combat event in the same cell is perceived.
- A combat event two cells away is heard but not seen (depending on distance and ranges).
- An event the state doesn't include in `perceives_event_types` is ignored.
- A sleeping NPC perceives only direct attack events.

**Acceptance:** Tests pass.

---

### Task 4.7 — Interrupt system and queue

**Points:** 2
**Dependencies:** 4.6
**Test layers:** [U]

**What:** When perceived events warrant interruption, push them into the NPC's interrupt queue and trigger FSM transitions per declared rules.

**File:** `src/harsh_realm/npc_behavior/interrupts.py` (new)

**Design:**

```python
class InterruptRule(BaseModel):
    """When to interrupt and what to transition to."""
    model_config = ConfigDict(frozen=True)
    triggering_event_type: str
    condition: str | None         # DSL expression
    priority: int                 # higher = preempts lower
    target_state: str             # e.g. "engaged.combat"


class InterruptManager:
    """Owns the per-NPC interrupt queue and routes events to FSM transitions."""

    async def on_perceived_event(self, npc_id: str, event: GameEvent) -> None:
        """Check interrupt rules; if any matches, queue and trigger."""
```

Interrupt rules are pack content (`packs/xwn-core/content/interrupts/<state>.yaml`). They map event types to state transitions, with optional conditions and priority.

**Behavior:**
- Higher-priority interrupts can preempt active engagements (combat preempts social).
- The interrupt context is stored on the NPC's `last_interrupt` field for the activity-level handler to read (e.g., who attacked me, who initiated the conversation).

**Tests:**
- An attack event interrupts a routine state into combat.
- An attack event during social engagement preempts to combat.
- An event without a matching interrupt rule does not change state.

**Acceptance:** Tests pass.

---

### Task 4.8 — Routine re-anchoring

**Points:** 2
**Dependencies:** 4.3, 4.7
**Test layers:** [U]

**What:** After an interrupt resolves, decide which routine activity the NPC should pick up.

**File:** `src/harsh_realm/npc_behavior/reanchor.py` (new)

**API:**

```python
async def reanchor_to_routine(
    npc_id: str,
    routine: Routine,
    current_tick: int,
    services: ServiceBundle,
) -> ScheduleSlot | None:
    """Find the schedule slot active at current_tick. None if none active."""
```

**Behavior:**
- Compute `tick_within_day = current_tick % routine.day_length_ticks`.
- Find the schedule slot whose `start_tick_offset <= tick_within_day < start_tick_offset + duration_ticks`.
- If none: NPC enters `active.idle` (fallback).
- If found but the NPC is at the wrong location: the activity itself decides whether to walk there or skip ahead. Common case: walk to the location as part of the activity logic.

**Tests:**
- Re-anchor at a tick mid-slot: returns that slot.
- Re-anchor at a tick with no slot active: returns None.
- Re-anchor across day boundary: works correctly.

**Acceptance:** Tests pass.

---

### Task 4.9 — World tick subscription and per-NPC tick processing

**Points:** 3
**Dependencies:** 4.2, 4.5, 4.6, 4.7, 4.8
**Test layers:** [U]

**What:** The integration point. Subscribe to `world.tick_advanced`. For each NPC in the loaded world (or in a relevant cell range — see scaling note below), advance their FSM, run their on_tick activity handler, process pending interrupts.

**File:** `src/harsh_realm/npc_behavior/scheduler.py` (new)

**API:**

```python
class NPCScheduler:
    def __init__(
        self,
        npc_repo: NPCRepository,
        fsm_runtimes: dict[str, NPCFSMRuntime],
        services: ServiceBundle,
    ) -> None: ...

    async def on_world_tick(self, current_tick: int) -> None:
        """Advance all relevant NPCs by one tick."""

    async def on_event(self, event: GameEvent) -> None:
        """Route an event to NPCs that perceive it."""
```

**Scaling note:** with hundreds of NPCs in a world, ticking every NPC every tick is wasteful. The scheduler implements a relevance filter: NPCs in the player's cell range get full ticks; off-screen NPCs get coarser updates (every N ticks, simulated by running N ticks worth of routine in one pass). The exact relevance policy is decided during implementation. Phase 4 ships with full ticks for all NPCs and a TODO to optimize once N-NPC performance is measured.

**Tests:**
- World tick advances all NPCs by one tick.
- An NPC whose schedule says they should be tending_shop at this hour transitions to that activity.
- An event perceived by an NPC routes through the scheduler to their FSM.

**Acceptance:** Tests pass.

---

### Task 4.10 — Existing static NPC migration

**Points:** 2
**Dependencies:** 4.9
**Test layers:** [U]

**What:** Existing NPCs in the world (created before Phase 4) need migration. Default behavior: assign them the `idle` routine, which is a no-op routine that keeps them in `active.idle` perpetually. They behave exactly as they did pre-Phase 4 (respond to player commands, no autonomous behavior).

**Files:**
- `packs/xwn-core/content/routines/idle.yaml` (new — empty schedule)
- `packs/xwn-core/content/routines/shopkeeper_general.yaml` (new — for the test case)
- migration script for the `entity_state` table to set `routine_id` defaults

**Behavior:**
- Migration on world load: any NPC without a `routine_id` gets `xwn-core:routine.idle`.
- Specific named NPCs can be assigned non-idle routines via admin or generator.

**Tests:**
- Pre-Phase-4 worlds load with all NPCs in idle routine; behavior unchanged.
- A test NPC assigned shopkeeper_general routine follows the schedule.

**Acceptance:** Tests pass.

---

### Task 4.11 — Combat and Social scene integration

**Points:** 2
**Dependencies:** 4.7, 4.10
**Test layers:** [U] [E2E]

**What:** When an NPC's FSM enters `engaged.combat` or `engaged.social`, the existing scenes pick up. When the scene exits, the FSM gets a notification to return to `active`.

**Files:**
- `src/harsh_realm/gm/scenes/combat.py` (extend with NPC FSM-aware entry/exit hooks)
- `src/harsh_realm/gm/scenes/social.py` (same)
- `src/harsh_realm/npc_behavior/scheduler.py` (handlers for scene exit notifications)

**Behavior:**
- Scene entry: when player initiates `talk to <npc>`, the social scene asks the FSM to transition to `engaged.social` for that NPC. The interrupt context is "player initiated conversation."
- Scene exit: the social scene emits `social.scene_ended`; the scheduler's handler transitions the NPC's FSM back to `active`, which re-anchors to the routine.
- Combat scene handles similarly: combat entry transitions all participating NPCs to `engaged.combat`. Resolution returns survivors to `active`.

**Tests:**
- Routine-tending shopkeeper with player walking in: transitions to `engaged.social`.
- Player leaving conversation: transitions back to `active.routine.tend_shop` (or whatever the schedule says).
- Combat: similar flow for an attacked NPC.

**Acceptance:** Tests pass.

---

### Task 4.12 — End-to-end test case: shopkeeper following daily schedule

**Points:** 3
**Dependencies:** 4.10, 4.11
**Test layers:** [U] [E2E]

**What:** The cycle-level proof. A shopkeeper NPC in a settlement follows the `shopkeeper_general` routine across a full simulated day. Player interactions interrupt; resolution returns to schedule.

**Test scenarios:**
- Player walks to the shop at hour 10: shopkeeper is tending. Conversation works.
- Player walks to the shop at hour 17: shopkeeper is closing. Conversation may or may not be available depending on activity logic.
- Player walks to the shop at hour 22: shopkeeper is at the tavern. Shop is empty.
- Player walks to the shopkeeper's home at hour 23: shopkeeper is home. Player can wake them up (separate interrupt rule), with social consequences.
- Player attacks the shopkeeper while sleeping: transition to combat.
- Combat resolves, shopkeeper survives: returns to schedule (likely picks up from sleep or recovers).

**Tests:** integration tests covering each scenario.

**Acceptance:** Tests pass. The shopkeeper's behavior is observable in admin logs and via player commands.

---

### Task 4.13 — Behavior tree migration path documented

**Points:** 1
**Dependencies:** 4.12
**Test layers:** none (documentation)

**What:** A design note describing how the FSM model can migrate to a behavior tree model in a future cycle, without rewriting the routine, perception, interrupt, or activity systems.

**File:** `docs/superpowers/specs/2026-04-26-phase-4-bt-migration-notes.md` (new)

**Outline:**
- An FSM is a degenerate behavior tree where every node is either a state or a transition.
- The activity registry already abstracts "what the NPC is doing right now" — this maps cleanly to BT leaves.
- The transition rules already abstract "when to switch" — these map to BT condition nodes.
- Hierarchical states map to BT subtrees.
- A future migration replaces `NPCFSMRuntime` with `NPCBehaviorTreeRuntime`. State nodes become BT leaf nodes; transitions become BT priority/sequence/selector parents. The activity, perception, interrupt, and routine systems do not need to change.
- The BT format itself is content-driven (YAML behavior tree definitions). A small BT runtime + visualizer in admin can replace the current FSM debugger.

**Acceptance:** Document exists.

---

### Task 4.14 — Frontend: NPC state visualization in admin

**Points:** 2
**Dependencies:** 4.10
**Test layers:** [V] [E2E]

**What:** An admin panel showing per-NPC current state, current activity, time-into-activity, and recent interrupt history.

**Files:**
- `frontend/src/components/admin/NPCBehaviorPanel.vue` (new)
- backend endpoint: `GET /api/entities/<id>/behavior` returns FSM state + recent interrupt log

**Behavior:**
- A user selects an NPC in admin, sees their current state, scheduled activity, time remaining in activity.
- A live update via WebSocket keeps the panel current.
- An "advance world clock by N ticks" button (admin-mode-gated) lets the user fast-forward to test routines.

**Tests:**
- Vitest: panel renders correctly with mocked state.
- Playwright: navigate admin, select shopkeeper, advance world clock, watch state transitions.

**Acceptance:** Tests pass.

---

### Task 4.15 — Documentation

**Points:** 2
**Dependencies:** 4.12
**Test layers:** none

**What:**
- Update `AGENTS.md` with an "NPC Behavior" section under "Data Models." Describe the FSM, routine, activity, perception, and interrupt systems. Add to "What NOT to Do":
  - "No new ad-hoc NPC behavior code in scenes. Use an activity handler."
  - "No new NPC state fields outside the FSM-managed set."
- Update `CLAUDE.md` "Completed Subsystems" with NPC behavior framework.
- Author a content guide: `docs/authoring/npc_routines.md` describing how to write a routine and an activity handler.
- Author the behavior-tree migration notes (Task 4.13).

**Acceptance:** All documents exist.

---

### Task 4.16 — Acceptance criteria document update

**Points:** 1
**Dependencies:** all preceding
**Test layers:** none

**What:** Append Phase 4 entries to `docs/acceptance_criteria.md`.

**Acceptance:** Document updated.

---

## 4. Phase completion criteria (when implemented)

Phase 4 is complete when *all* of the following hold:

1. All 16 tasks are implemented and committed.
2. Full existing test suite passes; new tests added by Phase 4 raise the total.
3. The shopkeeper test case (Task 4.12) demonstrates end-to-end scheduled behavior with player interrupts and routine re-anchoring.
4. Existing static NPCs migrated to the `idle` routine still respond identically to the pre-Phase-4 behavior. No regressions in social, combat, or shopping scenes.
5. The admin UI shows live NPC state and supports fast-forward testing.
6. `AGENTS.md`, `CLAUDE.md`, `docs/acceptance_criteria.md`, and the new `docs/authoring/npc_routines.md` all exist or are updated.
7. The behavior-tree migration notes document exists.

## 5. Phase 4 deferrals (append to overview §11)

Items deferred from this phase, when implemented, to future cycles:

- **Behavior trees as the runtime.** Documented as the eventual evolution; not built.
- **Utility AI / GOAP / dynamic goal selection.** NPCs follow scheduled routines, not utility-scored goal pursuits.
- **NPC-NPC dynamic interactions.** Two NPCs at the tavern don't socialize beyond what their individual activities specify.
- **Persistent NPC memory.** "The NPC remembers events from past sessions" — out of scope. Per-NPC short-term context only.
- **Procedurally-generated routines from occupation tags.** Routines are authored content. A future cycle could roll a routine for a new NPC based on occupation/personality/faction.
- **Detailed within-cell movement and animation.** NPCs occupy cells; finer positioning is abstract.
- **Off-screen scaling.** Initial implementation full-ticks all NPCs. A future optimization adds proximity-based update frequency.
- **Player character routines.** PCs are driven by player commands.
- **DSL-authored NPC behavior.** Activities stay in Python.

## 6. Notes for the future implementation cycle

- This spec was written during the Modular Rules Architecture cycle without implementing the work. When the implementation cycle activates, **start with Task 4.1 (audit)** — Phases 0–3 will have changed the codebase substantially.
- The framework decisions in Phases 0–3 were made knowing Phase 4 was coming. Specifically: the world clock tick mechanism is the routine driver; the perception system is parallel to (not built on) the trigger system; the activity registry pattern mirrors the compute registry from Phase 1 and the effect verb registry from Phase 3 — all three are versions of the same "named-handler" pattern, and lessons from Phases 1 and 3 apply.
- Resist combining the perception/interrupt system with the trigger/effect engine from Phase 3. They look superficially similar but serve different purposes: triggers are *content reactions* (a status effect's tick damage), perception/interrupts are *NPC state machinery* (an attacked guard switches to combat). Mixing them produces a confused architecture where simple things are hard.
- Resist building behavior trees in this cycle. The FSM is right for v1. The migration notes (Task 4.13) describe how to evolve later without throwing away this work.
- Resist procedural routine generation. Authored routines are the content lever. Routines for a new NPC type are a content task, not an engine task.
- Test the shopkeeper scenario end-to-end early (don't leave the integration test for last). The cross-system interaction is the real risk.
- Per AGENTS.md, every task uses the four-layer test rule. Mark task layer requirements in commits.
- After every task, run the full test suite. Commit with `[Phase 4 / Task 4.N]` for traceability.
