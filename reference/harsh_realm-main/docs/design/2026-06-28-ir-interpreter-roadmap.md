# IR Interpreter Roadmap — making authored content drive the whole game

> Status: planning. Created 2026-06-28.
> North star: the "Design Vision — IR as the Game's Interpreter" section in
> [AGENTS.md](../../AGENTS.md). This doc is the concrete plan that section's
> closing paragraph points to.
> Task tracking: HR-756 … HR-764 in [todo.md](../../todo.md).

## 1. Purpose

The IR (intermediate representation) machinery — compiler, DSL, dispatch,
intents, status/modifier services, and the `runtime_content` spine — is assembled
and **drives combat today**. This document plans the work that turns it from a
combat status engine into the game's general **content interpreter**: authored
YAML reacting to every event in every scene, with the engine providing only
primitives. It exists so the work is reviewable as a sequence of small,
well-scoped PRs rather than one undifferentiated push, and so each step has clear
acceptance criteria.

This is the same shape as Zork's Z-machine — authored content run by a small
stable VM — but more detailed: typed damage pools, named defenses, statuses,
modifiers, traits, tables, and procedures instead of flag bits.

## 2. Where the interpreter is today (2026-06-28)

The live loop is [`runtime_content::TriggerRuntime`](../../crates/harsh-core/src/runtime_content/trigger_runtime.rs):

```text
event ──▶ gather triggers the ACTING entity carries (active statuses,
          equipped items' granted traits, intrinsic traits)
      ──▶ evaluate each trigger's `when` against an EvalContext (pure DSL)
      ──▶ lower fired `do` effects to typed Intents   (dispatch::lower_effect)
      ──▶ apply Intents to state                       (IntentApplier)
      ──▶ cascade emitted events (bounded depth) ──▶ (loop)
```

What works: combat triggers from statuses + items + intrinsic traits (creature
and character), the shard-lance/laceration and ash-crawler/caustic-bite starter
examples, pack→`ir_records` compile at world creation, and the Content Studio
demo harness.

The limits that this roadmap removes:

- **Combat-only gate.** [`controller.rs::run_ir_triggers`](../../crates/harsh-core/src/gm/controller.rs)
  returns early unless `state == SceneState::Combat`. Nothing outside combat ever
  reaches the interpreter.
- **Acting-entity-only sourcing.** Triggers fire only for the event's `self`
  entity, and only entity-carried triggers exist — no world/room/object
  subscriptions. `index.rs` explicitly defers standalone/global triggers.
- **Combat-specific resolver.** `CombatantResolver` (reads `Combatant`, writes hp
  deltas) is the only way to feed the loop; there is no general entity resolver.
- **No time heartbeat.** Over-time effects are faked: a poison ticks "when the
  poisoned entity acts" in combat, not on the world clock.
- **Compute effects unimplemented.** `dispatch::lower_effect` errors on
  `roll_dice` / `run_procedure` (explicit, not silent).
- **Action model dropped.** The IR→`CreatureData` adapter discards
  `actions`/`pools`/`defenses`; combat uses legacy single-pool stats.
- **Two content models.** Legacy `CreatureData`/`ItemData`/`TableEngine`
  registries coexist with IR records. This split is debt, not design.

## 3. Plan

Phased so each phase is independently shippable and testable. Tasks reference
the HR ids in todo.md.

### Phase 1 — the interpreter fires everywhere (highest leverage)

This is the change that makes IR *the engine* rather than a combat feature.

- **HR-756 — General `EntityResolver` abstraction.** Extract a resolver trait
  (read an entity view, apply a resource delta) that both combat and a new
  world/exploration resolver implement. `CombatantResolver` becomes one
  implementation. `TriggerRuntime` is generic over the resolver.
  *Acceptance:* combat path unchanged (all existing tests green); a non-combat
  resolver can supply the player + nearby entities to the loop.

- **HR-757 — Ungate `run_ir_triggers` to all scenes.** Drop the
  `state == Combat` guard so any published event runs triggers in any scene,
  still guarded by `ir_triggers_enabled` (empty worlds unchanged). Wire the
  active scene's resolver.
  *Proof/acceptance:* a starter-pack `on: exploration.enter_hex` trigger fires
  when the player enters a tagged hex (e.g. a ruins hex applies a "dread"
  status); integration test pins it. Combat behavior unchanged.

- **HR-758 — World-clock `time.tick` heartbeat.** Emit a `time.tick` event when
  the world clock advances and evaluate `on: time.tick` triggers for *all*
  entities carrying them (not just an acting entity), so over-time effects tick
  on time: poison damages each tick, buffs decay, wounds fester — in any scene.
  Replace the combat "ticks when the entity acts" hack.
  *Acceptance:* a poisoned entity loses hp on clock advance with no action taken;
  status expiry is driven by tick count; test covers multi-tick decay.

### Phase 2 — author-facing expressiveness

- **HR-759 — Compute effects (`roll_dice`, `run_procedure`).** Implement these in
  `dispatch::lower_effect`, feeding results to subsequent effects via the eval
  context's `local` map (e.g. roll damage, then apply it). Uses the procedure
  runner for `run_procedure`.
  *Acceptance:* a trigger can roll dice and use the result in a later effect;
  the previous explicit-error path is replaced; tests cover binding + use.

- **HR-760 — Standalone & global trigger subscriptions.** Let authored
  `trigger` records (and room/object/world-owned triggers) fire by event type
  regardless of which entity carries them, alongside the entity-carried sources.
  Extend the `index.rs` sourcing model and the runtime to evaluate them with an
  appropriate `self`/context.
  *Acceptance:* a world/room trigger ("when the player enters the crypt", "when
  this lever is pulled") fires without being attached to the acting entity; test
  pins ordering vs entity-carried triggers.

### Phase 3 — "more detailed than Zork" (the action model)

- **HR-761 — Consume `actions` / `pools` / `defenses`.** Stop dropping the
  action-model fields in the adapter. Support multi-pool `EmitDamage`, named
  defenses, and authored creature/character actions so combat resolves through IR
  rather than legacy single-pool stats.
  *Acceptance:* a creature authored with typed pools + named defenses fights
  correctly through the IR damage pipeline; adapter no longer warns about dropped
  fields for these; tests cover multi-pool damage + a named defense.

### Phase 4 — collapse the two content models (anti-rot)

- **HR-762 — Migrate the legacy catalog to IR.** Convert existing `creatures:`
  lists and item lists (NOT IR-format) into IR records so there is one content
  model. Provide a migration path/compat shim so existing worlds keep working.
  *Acceptance:* the legacy registries are sourced from (or replaced by) IR; no
  new content requires the legacy shape; documented migration for existing data.

### Cross-cutting

- **HR-763 — Non-combat demo harnesses.** Mirror `DemoCombatRunner` for NPC/
  social, dungeon, and map scenarios so authored non-combat IR is exercisable in
  isolation (and in Content Studio).
  *Acceptance:* at least one non-combat demo runner + endpoint + test.

- **HR-764 — Trigger/effect authoring guide.** Extend
  [content-authoring-guide](2026-06-18-content-authoring-guide.md) with the full
  trigger/effect/intent vocabulary, the event catalog, and worked examples for
  each phase's new capability.
  *Acceptance:* a content author can write each supported effect from the docs
  without reading engine source.

## 4. Sequencing & dependencies

- Phase 1 is prerequisite for everything else (the loop must run outside combat
  before non-combat content is worth authoring). Within Phase 1: HR-756 →
  HR-757 → HR-758.
- Phase 2 and Phase 3 are independent of each other; both want Phase 1.
- Phase 4 (HR-762) is best **after** the action model (HR-761) so migrated
  content targets the final shape, not an interim one.
- HR-764 trails each phase (document capabilities as they land); HR-763 can start
  any time after HR-757.

## 5. Invariants to preserve (see AGENTS.md vision)

Every task above must keep: the engine pure (output is `Intent`s, a host
applies them); events as the only trigger; convergence on one content model (add
to IR, never deepen the legacy model or add a third); and primitives-in-kernel /
behavior-in-content (new engine code only for new primitives, authored content
for behavior). Resist scene-specific special-casing in the controller — that is
the growth pattern that calcifies a simulation.
