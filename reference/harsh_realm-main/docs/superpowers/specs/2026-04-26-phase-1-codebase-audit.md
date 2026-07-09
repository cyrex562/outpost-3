# Modular Rules Architecture — Phase 1 Codebase Audit

**Date:** 2026-04-27
**Phase:** Modular Rules Architecture / Phase 1 Procedures and Status Effects
**Task:** 1.1 — Codebase audit for procedures and status-effect-shaped state
**Status:** Complete

This audit inventories the current hardcoded generator/procedure-like flows,
status-effect-shaped state, and tick/event seams before implementing the Phase 1
procedure framework and status effect service.

## 1. UNE Personality Generator

Current implementation: `src/harsh_realm/engine/npc_personality.py`.

`UNEGenerator` is a Python-only generator that lazily loads YAML tables from
`default_content_dir() / "tables" / "npc"`. It caches scalar table results in
`_scalar_cache` and bearing results in `_bearing_cache`.

### Table Inputs

The generator reads these table files:

| Behavior | Table file | Shape |
| --- | --- | --- |
| Power level | `tables/npc/une_power_level.yaml` | scalar `entries[].result` |
| Descriptor | `tables/npc/une_descriptors.yaml` | scalar `entries[].result` |
| Motivation verb | `tables/npc/une_motivation_verbs.yaml` | scalar `entries[].result` |
| Motivation noun | `tables/npc/une_motivation_nouns.yaml` | scalar `entries[].result` |
| Bearing/focus | `tables/npc/une_bearings.yaml` | `entries[].result.{bearing, focus}` |

Missing table files are tolerated by returning empty lists; empty rolls return
empty strings. This behavior should be preserved or intentionally tightened in
Task 1.6.

### Public Behavior

`generate_personality(power_level=None)`:

- If `power_level` is `None`, rolls `une_power_level`.
- Always rolls one descriptor.
- Calls `generate_motivation()` for verb/noun.
- Calls `generate_bearing()` with its default chaos factor of `5`.
- Returns `UNEPersonality` with `base_disposition=0`.

`generate_motivation()`:

- Rolls `une_motivation_verbs`.
- Rolls `une_motivation_nouns`.
- Returns `UNEMotivation(verb=..., noun=...)`.

`generate_bearing(chaos_factor=5)`:

- Rolls two choices from `une_bearings`.
- Current logic chooses `pick2` only when `chaos_factor >= 7`.
- It chooses `pick1` when `chaos_factor <= 3` and also when chaos is 4-6.
- The docstring says low chaos keeps the earlier/milder result and high chaos
  keeps the later/more extreme result, but the implementation does not compare
  table order; it simply uses the first or second random choice.
- Empty table returns `UNEBearing(bearing="", focus="")`.

`disposition_label(score)`:

- Clamps score to `[-3, 3]`.
- Maps `-3 hostile`, `-2 unsteady`, `-1 guarded`, `0 indifferent`,
  `1 sociable`, `2 friendly`, `3 helpful`.
- Unknown clamped score falls back to `"indifferent"`.

`chaos_modified_disposition(base_score, chaos_factor)`:

- Starts with modifier `0`.
- Chaos `>= 7` applies `-1`.
- Chaos `<= 3` applies `+1`.
- Result is clamped to `[-3, 3]`.

### Procedure Migration Notes

UNE is a good Task 1.6 test because it needs all of Phase 1's basic step kinds:

- `roll` for scalar table draws.
- `compute` for `chaos_modified_disposition`, `disposition_label`, and possibly
  bearing chaos selection.
- `format` or structured output mapping for the final `UNEPersonality`.

The current `generate_personality()` does not thread the controller chaos factor
into `generate_bearing()`. If Phase 1 preserves exact behavior, the procedure
should default bearing chaos to `5` unless callers explicitly pass a value.

## 2. Current TableEngine And Generator API

Current implementation: `src/harsh_realm/engine/tables.py`.

`TableEngine` has two related responsibilities:

- Loading random table YAML under `data_dir / "tables"` into SQLite through
  `RandomTableRepository`.
- Rolling on loaded tables with weighted entries and simple subtable expansion.

### Random Table Roll API

`roll_on(table_id, context=None, _chain=None) -> TableResult`:

- Loads a `RandomTableRow` from SQLite.
- Uses `random.choices(entries, weights=entry.weight, k=1)`.
- Supports three result forms:
  - plain string -> `result_type="text"`, `result={"text": value}`,
    `raw_text=value`
  - dict with `table` -> recursively roll on that subtable
  - any other dict -> `result_type=result.get("type", "unknown")`,
    `raw_text=result.get("name") or result.get("text")`
- `context` is accepted but currently not used for filtering or substitution.

`roll_with_tags(category, tags) -> TableResult`:

- Lists tables by category.
- Picks a table with any overlapping tag.
- Rolls on that table.

### Existing Generator API

`generate(generator_id, params=None) -> GeneratorExecutionResult`:

- Requires `load_tables(data_dir)` to have been called first so `_data_dir` is
  known.
- Loads a YAML file from `_data_dir / "generators" / f"{generator_id}.yaml"`.
- Validates it as `GeneratorDefinition`.
- Supports only steps shaped as `{roll: table_id, assign: variable_name}`.
- For each step, calls `roll_on(step.roll)`.
- Stores `raw_text` for text table results, otherwise stores the result dict.
- Returns `GeneratorExecutionResult(assignments={...})`.
- `params` is reserved and currently unused.

Current generator schema is in `src/harsh_realm/models/generation.py` and
`src/harsh_realm/models/scene_data.py`:

- `GeneratorDefinition`: `id`, `name`, `steps`.
- `GeneratorStep`: `roll`, `assign`.
- `GeneratorExecutionResult`: `assignments: dict[str, JsonValue]`.

The only shipped generator content is
`packs/xwn-core/content/generators/npc_basic.yaml`, which rolls first name,
surname, occupation, two traits, motivation, and appearance.

### Phase 1 Recommendation

Use `TableEngine.roll_on()` or `RandomTableRepository` as the table-roll
foundation. Replace or supersede `TableEngine.generate()` with the new procedure
runner rather than expanding it in place, because Phase 1 procedure records need:

- Pack content records under `content/procedures/`, not filesystem-only
  `content/generators/` reads.
- Step kinds beyond `roll`: `compute`, `procedure`, and `format`.
- Input validation/defaults.
- Compute callable registry.
- Structured outputs rather than assignment maps only.
- Qualified content IDs such as `xwn-core:tables.une_descriptors`.

An adapter can keep `npc_basic` working during migration by treating old
generator YAML as a simple procedure or by leaving `TableEngine.generate()` as a
compatibility path until callers are moved.

## 3. Other Procedure Candidates

### NPCGenerator

Current implementation: `src/harsh_realm/generators/npc_gen.py`.

`NPCGenerator.generate_npc()` is a multi-step generation flow:

1. Validates optional `NPCGenerationContext`.
2. Executes `TableEngine.generate("npc_basic")`.
3. Formats first name + surname.
4. Extracts occupation, including dict-table entries with a `name` field.
5. Applies `occupation_override` from context.
6. Builds a trait list from `trait_1` and `trait_2`.
7. Copies motivation and appearance.
8. Computes disposition with `_derive_disposition(traits)`.
9. Rolls a matching greeting by trying `npcs_greetings` up to 10 times.
10. Returns `GeneratedNPC`.

Good procedure candidate after UNE. It needs `roll`, `compute`, `format`, and
probably a future conditional/filtering step for greetings.

### SettlementGenerator

Current implementation: `src/harsh_realm/generators/settlement_gen.py`.

`SettlementGenerator.generate_settlement()` mixes generation and persistence:

- Rolls settlement name and description.
- Chooses required buildings from settlement size.
- Rolls building names.
- Generates operator NPCs and resident NPCs.
- Persists settlement, building, and NPC entities.
- Updates cell settlement/town data.
- Calls `SquareWorldGenerator.generate_town()`.

Procedure candidate only for the content-generation subparts. Persistence should
remain repository/service-owned rather than inside a generic procedure runner.

### SquareWorldGenerator

Current implementation: `src/harsh_realm/generators/square_gen.py`.

`generate_dungeon()` and `generate_town()` are algorithmic map generators:

- Dungeon: wall fill, random room placement, corridor carving, connectivity
  repair, entrance selection.
- Town: road/plaza layout, quadrant building placement, terrain assignment.

These are better represented as registered `compute` callables or future
procedure step types, not YAML-only linear procedures.

### WorldGenerator

Current implementation: `src/harsh_realm/generators/world_gen.py`.

`generate_region()` is a large algorithmic generation flow with persistence:

- Loads terrain weights and name lists.
- Chooses bounded/open edges.
- Seeds and fills terrain using adjacency modifiers.
- Guarantees passable terrain variety.
- Places settlements, ruins, landmarks, lairs, and camps.
- Writes cells to SQLite.
- Optionally enhances settlements via `SettlementGenerator`.

The feature/name/terrain table parts could eventually become procedure content,
but the world-generation algorithm and cell writes should remain a dedicated
generator/service.

### AdventureCrafter

Current implementation: `src/harsh_realm/engine/adventure_crafter.py`.

`AdventureCrafter` is a strong future procedure candidate:

- Loads theme, character, and plot tables from oracle YAML.
- Creates/list/resolves plotlines via `OracleRepository`.
- `advance_plotline()` generates one `AdventureScene` by choosing a theme
  element, character, and plot, then formats narration.

The scene-generation part maps cleanly to `roll`/`compute`/`format`. Plotline
CRUD and persistence should remain outside the procedure runner.

### DiscoverySystem

Current implementation: `src/harsh_realm/engine/discovery.py`.

`search_hex()` is partly procedural but has gameplay side effects:

- Enforces `last_searched_tick` cooldown.
- Rolls discovery probability.
- Rolls terrain-specific table with common fallback.
- Performs a skill check when required.
- Updates cell data/features through an event-backed persistence path.

It is not a Phase 1 procedure migration target, but it is a later candidate for
declarative table selection and result formatting.

## 4. Status-Effect-Shaped State

There is no dedicated status effect subsystem, no `entity_status_effects` table,
and no active tick-expiring condition model.

### Existing Nearby Shapes

`src/harsh_realm/models/item.py`:

- `SaveBonusEffect` has `save_type`, `bonus`, and `duration`.
- Its docstring describes a temporary saving throw bonus.
- This is status-effect-shaped content, but there is no runtime application,
  persistence, or expiration path found in the current code.
- Phase 1 should avoid implementing the modifier side of this effect; Phase 2
  owns mechanical modifiers.

`src/harsh_realm/engine/discovery.py` and `gm/cell_repository.py`:

- `last_searched_tick` is durable timed state in `cell_search_state`.
- It is not an entity status effect, but it is the current best example of a
  tick-based cooldown stored relationally.

`src/harsh_realm/engine/healing.py` and
`src/harsh_realm/gm/scenes/exploration_movement.py`:

- Rest accepts tick counts (`10` for short rest, `50+` for full rest).
- The scene may repeat long rests and totals elapsed ticks for narration.
- These rest ticks do not currently advance `GMController._tick` by the rest
  amount; they are local healing inputs.

`src/harsh_realm/models/character.py` and NPC models:

- Character/NPC intrinsic state includes HP, class abilities, skills, and
  disposition.
- No active condition/effect list was found on character or NPC state.

`src/harsh_realm/models/scene_data.py`:

- `DungeonRoom.encounter`, `DungeonRoom.loot`, and `DungeonRoom.data` can hold
  flexible payloads, but no status-effect convention was found.

### Non-Candidates

- `status` commands in scenes are presentation commands, not active conditions.
- Thread/plotline `status` fields are oracle workflow state, not entity
  conditions.
- `event_log.data` is intentionally JSON event payload storage, not status
  storage.

### Phase 1 Recommendation

Create the new durable status effect path exactly as the Phase 1 spec describes:

- Pack content schema for status effect definitions.
- `entity_status_effects` table owned by a status effect repository/service.
- Event-requested apply/remove/expire writes.
- No mechanical modifier fields in Phase 1.

The existing `SaveBonusEffect.duration` should be listed as a future integration
point after the status effect service and Phase 2 modifier framework exist.

## 5. Current Tick And Event Flow

There is no standalone `WorldClock` service in source. The active runtime clock
is `GMController._tick`.

### Tick Ownership

Current implementation: `src/harsh_realm/gm/controller.py` and
`src/harsh_realm/gm/controller_support.py`.

- `GMController.initialize()` loads `tick` from `gm_state`.
- `GMController.handle_input()` increments `_tick` by exactly `1` after each
  player input.
- `_save_state()` persists `scene` and `tick` through
  `gm.state_persist_requested` domain events.
- `GMStateEventHandler` writes those values into `gm_state`.
- `GameEvent.tick` is set from the controller or scene's current tick.
- Scene objects receive a tick value at construction and use it when creating
  events, but the controller owns persistence of the canonical tick.

### Event Layers

There are two event dispatch layers:

- `DomainEventDispatcher` handles async command/request events and persistence
  handlers, then returns cascaded events.
- `EventBus` publishes public events, handles synchronous cascades, feeds
  websocket subscribers, and logs through `EventLogger`.

Relevant existing request/result pattern:

- `exploration.rest_requested` -> `ExplorationEventHandler` persists character
  HP -> emits `character.hp_changed`.
- `exploration.search_requested` -> persists cell data/features -> emits
  `exploration.search_completed`.
- `gm.state_persist_requested` -> persists `gm_state` with no public result.

### Tick Consumers

- Faction turns run from `GMController._run_faction_turns()` after state save.
  `FactionTurnEngine.check_and_run_weekly(current_tick)` reads
  `faction_last_turn_tick` from `gm_state`.
- Discovery search cooldown compares current tick to durable
  `cell_search_state.last_searched_tick`.
- Event log records event ticks.

### Status Effect Hook Point

Phase 1 needs a reliable expiration trigger. The spec says world tick
advancement publishes `world.tick_advanced`, but no such event currently exists.
Recommended Task 1.12 approach:

1. Add a terminal public event such as `world.tick_advanced` after `_tick`
   increments and the tick is persisted, with `previous_tick` and
   `current_tick`.
2. Register a status effect handler/subscriber that calls
   `StatusEffectService.expire_due(current_tick)`.
3. Emit `status.expired` for each removed active effect.

If rest should advance status durations by more than one tick, Phase 1 must
decide whether to keep current behavior (one controller tick per command) or add
an explicit multi-tick advancement command/event. The current code treats rest
tick counts as healing math, not controller clock advancement.

## 6. Oracle Procedure Candidates

Current implementation: `src/harsh_realm/engine/oracle.py`.

The Mythic oracle should remain Python in Phase 1 per the spec, but several
parts are procedure-shaped and worth listing for future migration.

### FateChecker

`FateChecker.check(likelihood, chaos_factor, rng=None)`:

- Loads `tables/oracle/fate_chart.yaml`.
- Normalizes likelihood enum/string.
- Clamps chaos to `[1, 9]`.
- Looks up yes/exceptional thresholds.
- Rolls d100.
- Classifies exceptional yes, yes, exceptional no, or no.
- Formats narration.

This is a table lookup plus compute-heavy resolver. It is better as a registered
oracle resolver or compute callable than as a simple linear procedure.

### ChaosTracker

`ChaosTracker` is stateful, clamped, and event-callback aware. It should not be
migrated to procedure content. It is a service/state helper.

### SceneChecker

`SceneChecker.check(chaos_factor, rng=None)`:

- Rolls d10.
- Compares against chaos factor.
- Odd in-chaos roll means interrupt.
- Even in-chaos roll means altered.
- Otherwise normal scene.
- Formats narration.

This can become a future procedure or registered scene-check resolver.

### RandomEventGenerator

`RandomEventGenerator.generate(rng=None)`:

- Rolls on event focus, event action, and event subject tables.
- Formats narration from the three results.

This is the cleanest future oracle procedure candidate: `roll`, `roll`, `roll`,
then `format`.

### Adventure Crafter

`AdventureCrafter._generate_scene()` is also oracle-adjacent and procedure-like:
choose a theme element, choose character/plot entries, and format an
`AdventureScene`. See section 3.

## 7. Follow-Up Notes For Phase 1

- The current table IDs are unqualified (`une_descriptors`,
  `npcs_greetings`, `discoveries_forest`). Procedure records should use the
  Phase 1 qualified ID convention, so Task 1.2/1.3 needs a resolver from
  qualified content IDs to current table IDs or a table-loading migration.
- `TableEngine.generate()` loads generator YAML directly from disk and does not
  use `ContentService`. New procedure loading should be pack/override aware.
- `params` on `TableEngine.generate()` is unused. Procedure inputs should be
  validated and available to compute/format steps from the start.
- Existing generation flows often mix pure generation with persistence. The
  Phase 1 runner should stay pure where possible; services/scenes should own
  writes through existing request/result event patterns.
- There is no current status effect storage to migrate, so Phase 1 can add the
  table/service without backfilling legacy active effects.
