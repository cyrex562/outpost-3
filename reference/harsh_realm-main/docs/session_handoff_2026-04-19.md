# Session Handoff — 2026-04-19 → Follow-on

Use this document as the opening prompt for the next Harsh Realm session. It captures what was completed today, the current baseline, and the next concrete task to pick up.

## Session summary (today, 2026-04-19)

Worked through the ECS + Pydantic migration in `todo.md`. Everything below has been committed (`10ca0b5 202604192130`). 1188 tests pass; 4 pre-existing failures remain (`test_character_creation.py::TestMinHpIsOne::test_hp_minimum_1_with_con_minus2`, `test_expert_reroll.py::test_expert_reroll_consumed`, two `test_healer.py` checks) — not caused by this session's changes.

Completed in order:

1. **CLAUDE.md** — inlined Python style, Pydantic rules, `object`/`Any` priority ladder, `Protocol` vs `ABC`, event-bus rules, DB access rules, error handling, logging, file org, YAML content, frontend TypeScript + component + testing rules, git conventions, full "What NOT to Do" list.
2. **`Any`/`object` cleanup** — 3 HIGH, 6 MED, 14 LOW, 5 NEW violations fixed; `**kwargs: Any` patch APIs in `faction/repository.py` (4 methods) and `admin/service.py` (1 method) replaced with typed explicit keyword parameters.
3. **FastAPI inline Pydantic bodies** — moved `GMTeleportBody`/`GMSpawnBody`/`GMGiveItemBody`/`GMSetHPBody`/`GMSetGoldBody` from `api/gm_routes.py` into `models/api.py`. Moved `CellCoordinate`/`CellUpdateBody`/`BulkCellUpdateBody` from `api/editor/cells.py` into `models/editor_api.py`. Added `ImportAllResult(RootModel)` wrapper for `/import-all` return type.
4. **BaseModel reorganization** — resolved duplicate `SkillCheckResult` (renamed discovery's to `DiscoverySkillCheck`). Moved ~35 BaseModels + 4 enums + 1 protocol from `engine/`, `faction/`, `generators/`, `gm/`, `parser/` into new or existing `models/` files (`engine_results`, `combat_runtime`, `loot`, `oracle`, `shop`, `generation`, `faction_state`, `gm_runtime`, `parser`). Source modules keep re-exports so call sites aren't broken.
5. **Sub-model aliases** — `AttributeScores`/`AttributeMods`/`SkillLevels` in `models/character.py` shared by `Character` and `ecs/components.CharacterSheetComponent`. `TerrainWeightMap`/`AdjacencyWeightMap`/`TerrainCountMap` in `models/generation.py`. `TerrainDescriptionMap`/`FeaturePrefixMap`/`MovementDescriptionMap` in `models/narrator_content.py`.
6. **ECS runtime pilot** — first concrete system wired:
   - `LowHealthWarningSystem` in `src/harsh_realm/ecs/actor_systems.py` — emits one `gm.narrate` event when the player's HP first crosses below 25%, registered in both combat and exploration scenes. Runs via `world.run_systems()` through the scene's `_run_ecs_systems()` helper; emitted events flow back out through `handle_command`.
7. **Exploration actor lifecycle** — `ExplorationCoreMixin` now owns a persistent `_ecs_world`. `_ensure_actor_ecs(db)` (lazy, called from `handle_command`) materializes the player + NPCs at the current cell; `_refresh_ecs_cell` clears stale NPC / spatial / feature entities and re-materializes on cell change. `SpatialEcsAdapter.materialize_world_cell` is invoked for the current cell too.
8. **Encounter group ECS** — new `EncounterComponent` + `EncounterEcsAdapter` (`src/harsh_realm/ecs/encounter.py`). `ExplorationCoreMixin._pending_combat` is now a `@property` backed by the ECS entity; setter accepts `PendingEncounterState | dict | None`; getter projects a fresh `PendingEncounterState` from the ECS entity. `_handle_sneak` now re-assigns the projected state after mutation to persist the change.
9. **Feature instance interactivity** — `_handle_enter`, `_handle_explore`, `_handle_shop` dispatch via `_query_features_at_player(EnterableComponent)` → `feature_kind` set instead of `"settlement" in features` string checks. Scene ECS world now holds the current cell's `FeatureComponent` entities with `Searchable`/`Enterable`/`Interactable` attached by kind.

Tests added this session: 45 (13 for `LowHealthWarningSystem`, 3 exploration actor lifecycle, 11 encounter ECS, 5 feature interactivity, 13 miscellaneous in moved modules; full list visible via `git show 10ca0b5 --stat -- tests/`).

## Current state

```
$ pytest --tb=short -q
1188 passed, 4 failed (pre-existing), 12 skipped
```

**Clean sections of `todo.md`:**
- `Write a rule to not use Object as a type` — all HIGH/MED/LOW/NEW done except "temporary migration shim" collapses (bundled with dual-path removal; not urgent).
- `Use pydantic classes for Request Body and Response models` — all 6 sub-items done.
- `Separate class and route definition code` — all 3 sub-items done.
- `Move basemodel classes out of modules and into their own modules under the models/ package` — all 13 sub-items done; `models/api/` subpackage plan abandoned in favor of flat files (`models/api.py`, `models/editor_api.py`).
- `ECS runtime coverage expansion` — 4 of 11 top-level items done (first system, actor lifecycle, encounter group, feature interactivity).

## What to work on next

Pick up **ECS runtime coverage expansion** in `todo.md`. In priority order:

### Option A — Hazard/trap ECS (net-new gameplay mechanics)

- Decide hazard source of truth (generator-planted vs YAML-defined); document the decision in `docs/ecs_component_catalog.md`.
- Materialize hazards as ECS entities (`IdentityComponent` + `PositionComponent` + `HazardComponent` + optional `TriggerComponent`) during cell or dungeon generation.
- Add a `HazardTriggerSystem` in `src/harsh_realm/ecs/actor_systems.py` (or a new `hazard_systems.py`) that fires on player entry into a hazard cell and emits damage / save events.
- Remove any inline hazard dict handling from exploration/dungeon code (likely minimal; may mostly be net-new).
- Unit + integration tests.

### Option B — World cell runtime overlays

- Add `SpatialEcsAdapter.mark_cell_explored(world, q, r)` and `set_cell_faction(world, q, r, faction_id)` helpers that update `SpatialCellComponent` and emit cell-updated events.
- Rewrite `gm/scenes/exploration_movement._handle_move` to call these adapter methods instead of mutating `CellData` fields directly.
- Ensure writes flush through `CellRepository` via existing event handlers (no direct SQL leaking in).
- Unit tests for the adapter helpers; integration test verifying `cells.data` is persisted after a move.

### Option C — Loot entity materialization

- Convert `death_markers` (currently stored in `cells.data`) → ECS entity with `LootComponent` during cell materialization.
- Convert encounter rewards → `LootComponent` on the encounter entity, transferred to a dropped-loot entity on combat resolution.
- Wire a loot-pickup handler path that queries `LootComponent` at the player's position (replaces the dict iteration in `_handle_take`).
- Tests including loot persistence across session boundaries.

### Option D — Duplicate state hotspots consolidation

- `Combatant` vs `HealthComponent`/`CombatStatsComponent`/`InventoryComponent` — make combat read HP/AC/items from ECS components; `Combatant` becomes a thin projection for initiative ordering only.
- `CellData.features` vs `FeatureComponent` — ECS becomes runtime source; `CellData.features` drops to persistence cache only. (Largely already true after feature interactivity work; finalize by removing any remaining string-based reads in scene code.)
- `DungeonScene._adjacency` vs `ConnectivityComponent` — remove `_adjacency` dict and read connectivity from the component every time.

### Option E — Adapter round-trip tests (quick win)

- Test: materialize a character + NPC into an `EcsWorld`, project back via `project_character` / `project_npc_record`, assert round-trip equality on all carried fields.
- Test: materialize → mutate a component → persist via `persist_entities` → reload from DB → assert the mutation landed.

### Option F — Status effects (net-new)

- Add `StatusEffectsComponent` to `ecs/components.py`.
- Materialize effects on actor entities during scene init from character `class_abilities` / NPC data.
- Add a `StatusEffectTickSystem` that decrements `remaining_ticks` each tick and emits expiry events.
- Wire into combat and exploration scene tick loops.
- Unit + integration tests.

**My recommendation:** start with **Option E (adapter round-trip tests)** as a warm-up — it's a standalone <1-hour task that solidifies the work done yesterday and would catch any projection bugs before they compound. Then move to **Option A (hazards)** or **Option B (cell overlays)** for the next substantial piece.

## How to start the next session

Paste this doc (or a summary of it) into a fresh session, then:

1. **Verify baseline:**
   ```
   source .venv/bin/activate && pytest --tb=short -q 2>&1 | tail -5
   ```
   Expect: `1188 passed, 4 failed, 12 skipped`. Don't touch the 4 pre-existing failures — they aren't yours.

2. **Read the active todo section:** `todo.md`, search for "ECS runtime coverage expansion" — the completed items are marked `[x]` with a dated rationale; the unchecked items are what's left.

3. **Pick an option (A–F above)** and start. Create tasks via `TaskCreate` as you break the work down. Commit each logical chunk separately; the project's commit convention is a bare `YYYYMMDDHHMM` timestamp subject.

## Conventions reminders

- **Pydantic for all structured data** — no `@dataclass`, no `TypedDict`, no plain `dict` between functions.
- **No `Any`/`object` convenience types** — priority ladder: Pydantic → `JsonValue`/`JsonObject` → `Protocol` → concrete union → `TypeVar`.
- **ECS is runtime-only** — source of truth for simulation; persistence stays repository-driven. New components go in `src/harsh_realm/ecs/components.py`; concrete systems in `src/harsh_realm/ecs/*_systems.py`; adapters in `src/harsh_realm/ecs/adapters/` or a root-level `src/harsh_realm/ecs/*.py`.
- **Events** — dotted namespace strings, flat JSON payloads, handlers don't propagate exceptions. Typed event wrappers layer on top of `GameEvent`.
- **Testing** — every feature needs unit + property (when invariants exist) + mutmut + Playwright (for UI) per the matrix in `CLAUDE.md`. For ECS-only work, unit + integration is sufficient until mutmut is rerun on touched modules.
- **Commit style** — bare `YYYYMMDDHHMM` subject; include `Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>` trailer.

## Open questions the next session should address

- **Hazards (Option A)** — YAML or generator? Both have tradeoffs. YAML is explicit and content-driven; generator-planted is more dynamic. Ask the user before starting.
- **Duplicate state consolidation (Option D)** — `Combatant` dedup is large and touches combat flow. Do this after hazards/overlays, and split into one sub-pattern per commit.
- **Social/shopping ECS handoff** — deferred sub-item in the exploration actor lifecycle: social and shopping scenes currently re-fetch NPCs from persistence when spawned from exploration. The ECS world already has the NPCs materialized; pass them via projection. Revisit once Option D has settled actor consolidation.
- **`_handle_interact` handler** — feature catalog has `InteractableComponent` attached by kind, but no `interact` verb exists. Wait for a gameplay need.

## File index (new this session)

- `src/harsh_realm/ecs/actor_systems.py` — concrete actor-domain ECS systems.
- `src/harsh_realm/ecs/encounter.py` — `EncounterEcsAdapter` + `EncounterComponent` wiring.
- `src/harsh_realm/models/combat_runtime.py`, `engine_results.py`, `gm_runtime.py`, `loot.py`, `parser.py`, `shop.py` — relocated BaseModels.
- `tests/test_ecs_actor_systems.py`, `test_ecs_encounter.py`, `test_ecs_feature_interactivity.py` — new ECS tests.
