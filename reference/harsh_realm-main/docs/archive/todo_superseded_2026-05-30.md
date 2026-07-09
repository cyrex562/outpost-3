# Harsh Realm — TODO (SUPERSEDED)

> **Note:** This document has been superseded by the new [todo.md](../todo.md) in the project root. 
> It is kept here for historical reference and context on completed architectural migrations.

---

# Original Todo List



## Code Review and other Observations

- [ ]: Rather than storing JSON directly in the database, we should be storing actual typed data.
  - [x]: Write a short persistence ADR defining which data must be relational, which data may remain JSON, and which legacy JSON columns are temporary compatibility shims.
  - [x]: Inventory all current JSON-backed persistence surfaces: `entities.data`, `cells.data`, `factions.data`, `faction_assets.data`, `faction_relations.history`, `dungeons.rooms`, `dungeons.connections`, `dungeons.data`, `threads.data`, `plotlines.data`, `items.data`, `creature_templates.data`, and `event_log.data`.
  - [x]: Produce a migration matrix for each JSON column: current readers/writers, authoritative fields, target columns/tables, migration priority, and whether dual-write/backfill is required.
  - [x]: Declare `event_log.data` intentionally JSON and out of scope for relational normalization unless replay/debugging needs prove otherwise.
  - [x]: Define additive migration rules for all persistence refactors: add schema, dual-write, backfill, switch reads, remove legacy JSON reads, then drop old columns later.
  - [x]: Migrate `entities.data` first by extracting typed character and NPC persistence into dedicated tables and repositories.
  - [x]: Replace JSON-centric entity repository methods like `load_entity_json` / `save_entity_json` with typed aggregate load/save methods.
  - [x]: Add backfill migrations from legacy `entities.data` payloads into the new typed entity tables. Obsolete because older worlds can be discarded and recreated from the current schema.
  - [x]: Switch gameplay reads and writes for characters/NPCs to typed persistence and remove legacy JSON reads from active gameplay paths.
  - [x]: Migrate `cells.data` next by extracting settlement/discovery/loot/marker state into typed tables.
  - [x]: Replace gameplay/editor reads of `cells.data` with typed repositories for exploration, town, and discovery state.
  - [x]: Add backfill migrations from legacy `cells.data` payloads into the new typed cell-state tables. Obsolete because older worlds can be discarded and recreated from the current schema.
  - [x]: Normalize faction persistence by replacing `factions.data`, `faction_assets.data`, JSON goals/tags, and `faction_relations.history` with typed tables/columns.
  - [x]: Normalize dungeon persistence by replacing `dungeons.rooms`, `dungeons.connections`, and authoritative `dungeons.data` fields with typed child tables.
  - [x]: Review `threads.data`, `plotlines.data`, `items.data`, and `creature_templates.data` and split out any authoritative runtime fields from schema-flexible editor content.
  - [x]: Remove generic JSON persistence helpers and `decode_json`-style gameplay reads for every aggregate that has been migrated to typed storage.
  - [x]: Add migration tests proving that existing worlds with legacy JSON blobs upgrade correctly without losing gameplay state.
  - [x]: Add repository and integration coverage for each migrated aggregate: entity persistence, cell persistence, faction persistence, and dungeon persistence.
  - [x]: Add property tests where invariants exist, especially inventory totals, faction stat bounds, and dungeon graph consistency.
  - [x]: Consider using SQLModel to define database models after the target relational schema is stable; evaluate it with a narrow spike instead of as the first migration step.
    - [x]: Prototype SQLModel on one or two already-normalized tables and compare complexity against the current repository + Pydantic approach.
    - [x]: Decide whether to adopt SQLModel for schema/row definitions only, adopt it more broadly, or explicitly reject it and document why.
- [ ]: Cells are a candidate for ECS. A basic Cell Entity with components for features, and associated systems to handle stuff
  - [x]: Write an ECS ADR that defines the scope as general runtime simulation, not just cells, and explicitly documents non-goals such as admin CRUD, repository storage, and API schemas.
  - [x]: Inventory candidate ECS entity families and rank them by payoff: actors, world hexes, town cells, dungeon cells, feature instances, loot/items, encounter groups, hazards, and faction-presence entities.
  - [x]: Define the initial ECS runtime architecture: `EntityId`, typed Pydantic components, world/registry storage, query API, system protocol, and event-bus integration points.
  - [x]: Define the first component catalog for actor, spatial, interaction, combat, inventory, social, AI, and environmental behavior.
  - [x]: Implement a minimal in-memory ECS package under `src/harsh_realm/ecs/` with entity storage, component registration, queries, and system execution.
  - [x]: Build adapters that materialize ECS entities from current repositories and write runtime outcomes back through the existing repository/event architecture.
  - [x]: Pilot ECS on actor runtime state first: player characters, NPCs, and combat enemies in combat/exploration scenes.
  - [x]: Expand ECS to spatial runtime entities next: world hexes, town cells, dungeon cells, and feature instances such as lairs, ruins, landmarks, exits, and searchable nodes.
  - [x]: Evaluate whether loot/item entities should move into ECS once stacking, ammo, equipment, and dropped-item mechanics are expanded.
  - [x]: Evaluate whether faction patrols, expeditions, hazards, and other ambient world entities should become ECS-managed runtime entities after the actor/spatial pilots stabilize.
  - [x]: Add unit, integration, and property coverage for ECS queries, system ordering, adapter round-trips, and invariants such as occupancy, connectivity, and item conservation.
- [x]: Write a rule to not use Object as a type
  - [x]: Write a Python typing policy that forbids bare `object` and `Any` as convenience escape hatches and documents the narrow allowed exceptions.
  - [x]: Add the rule to `AGENTS.md` so new code prefers Pydantic models, `JsonValue`/`JsonObject`, `Protocol`, unions, and generics over `object`/`Any`.
  - [x]: Inventory current `object` / `Any` usages in active runtime code and classify each as acceptable boundary, temporary migration shim, or violation.
  - [x]: Replace the highest-risk `object` / `Any` usages in gameplay/runtime modules with concrete types or `Protocol`s.
  - [x]: Add static typing gates for touched files using Ruff and stricter Pylance/Pyright/Mypy-friendly annotations.
  - [x]: Re-run audit (2026-05-06): resolved major violations in character_recalc, healing, and npc_gen.
  - [x]: HIGH: `bus: Any | None` in `api/editor/common.py:49` and `api/gm_routes.py:55` — type as `EventBus | None`.
  - [x]: HIGH: `_get_alive_creature_data() -> list[Any]` in `gm/scenes/combat_support.py:111` — return typed combatant list/protocol.
  - [x]: MED: `_pending_combat_scene: Any` in `gm/scenes/exploration_core.py:43` and `gm/scenes/dungeon.py:121` — introduce a concrete scene union or `SceneHandle` protocol.
  - [x]: MED: `cmd: Any` in `gm/scenes/respawn.py:73` — use `ParsedCommand`.
  - [x]: MED: `indexed: Any | None` in `gm/scenes/character_creation_steps.py:300` — typed kit index mapping.
  - [x]: MED: `-> object` in `api/editor/transfer.py:100` — concrete Pydantic return type.
  - [x]: MED: `-> dict[str, object] | None` in `engine/items.py:69` — return `InventoryItemRecord | None`.
  - [x]: MED: `updates: dict[str, object]` in `gm/dungeon_repository.py:113` — typed patch model.
  - [x]: LOW: `features: list[object]` in `gm/cell_repository.py:84` — `list[str]` or feature model.
  - [x]: LOW: `dict[str, object]` dispatch/item maps in `gm/scenes/shopping.py:92,164` and `gm/scenes/social.py:105` — typed payload/dispatch table.
  - [x]: LOW: `list[object]` / `dict[str, object]` editor SQL payloads in `api/editor/characters.py:59,88,160` — JSON alias or patch model.
  - [x]: LOW: `-> dict[str, object]` in `api/websocket.py:109` — typed transport model.
  - [x]: LOW: `tuple[object, ...]` in `api/editor/cells.py:125` — parameter alias.
  - [x]: LOW: Remove dead `Any` import in `api/admin_routes.py`.
  - [x]: LOW: Tighten `ecs/world.py` `query()` return type — replace `Iterator[tuple[object, ...]]` with a typed component-tuple generic (overloads already exist for 1–3 components).
  - [x]: NEW: `engine/combat.py:668` `db: Any = None` — type as `WorldDatabase | None`.
  - [x]: NEW: `gm/scenes/exploration_movement.py:102` `current_terrain_type: Any = terrain_type` — add annotation.
  - [x]: NEW: `engine/adventure_crafter.py:74` `to_dict() -> dict[str, object]` — replace with `model_dump()` or typed helper.
  - [x]: NEW: `admin/data_access.py:20` `load_yaml(...) -> object` — return `JsonValue` or document-specific type.
  - [x]: NEW: `models/admin.py:16` `to_dict() -> dict[str, object]` — use `model_dump()` instead of hand-rolled helper.
  - [x]: Replace `**kwargs: Any` / `**fields: Any` patch APIs in `faction/repository.py:117,170,211,280` and `admin/service.py:275` with explicit typed patch models.
  - [x]: Collapse temporary-migration-shim dual dict/model paths in `gm/scenes/dungeon.py:44-45`, `gm/scenes/social.py:51`, `generators/npc_gen.py:57`, `engine/healing.py:202`, `engine/character_recalc.py:73,140`, `gm/entity_state_repository.py:138`, `models/faction_state.py:23`, `admin/content_mixin.py:117,175` once callers are uniformly typed.
- [x]: Define a base class for event types
  - [x]: Write a short typed-event design note defining `GameEvent` as the transport envelope and typed event classes as validated wrappers around payload models.
  - [x]: Implement a generic typed-event base class with a class-level `event_type`, typed `payload`, and `to_game_event()` / `from_game_event()` helpers.
  - [x]: Add a narrow event-type registry/parser helper so known typed event classes can be reconstructed from `GameEvent` instances without ad hoc switch logic.
  - [x]: Define the first typed event families for stable runtime events such as `gm.narrate`, `character.death`, `combat.start`, `combat.attack`, `combat.player_hit`, and `exploration.encounter`.
  - [x]: Reuse and extend existing `PayloadModel` classes in `payloads.py` where needed instead of introducing raw dict payload shapes.
  - [x]: Migrate selected event producers in scene helpers and event handlers to construct typed event classes before converting them to `GameEvent`.
  - [x]: Migrate selected consumers that currently index into `event.data` manually to parse typed event wrappers where the payload contract is stable.
  - [x]: Add unit tests for typed event round-trips, payload validation failures, wrong-`event_type` rejections, and EventBus compatibility.
  - [x]: Add integration coverage for at least one request/result flow proving typed event wrappers do not change the published event cascade.
- [x]: Use pydantic classes for Request Body and Response models based on fastapi features in web app.
  - [x]: Audit 2026-04-19: 94/101 endpoints OK; 5 inline BaseModel bodies; 2 missing response types.
  - [x]: Move inline request bodies from `api/gm_routes.py` (`TeleportBody`, `SpawnBody`, `GiveItemBody`, `SetHPBody`, `SetGoldBody`) into `models/api/gm_commands.py`.
  - [x]: Move inline request bodies from `api/editor/cells.py` (`CellCoordinate`, `CellUpdateBody`, `BulkCellUpdateBody`) into `models/editor_api/cells.py`.
  - [ ]: Replace `GET /export/{table}` `-> object` in `api/editor/yaml_files.py` with a typed export-row/export-list union or table-specific envelope model.
  - [ ]: Replace `POST /import-all` raw `dict[str, ImportAllTableResult]` return in `api/editor/yaml_files.py` with an explicit `ImportAllResult` wrapper.
  - [x]: Confirm every route declares `response_model=` where FastAPI can't infer it — sweep and fill gaps.
  - [x]: Confirm all path/query params are typed (no `str` where a narrower literal or int applies).

- [x]: Separate class and route definition code
  - [x]: Extract all request/response BaseModel declarations out of `api/*.py` and `api/editor/*.py` into `models/api/` subpackage (ties to the FastAPI Pydantic task above).
  - [x]: Leave only routers/handlers and FastAPI `Query`/`Path`/`Body` dependency expressions in `api/*.py`.
  - [x]: Add a `models/api/__init__.py` that re-exports the public request/response classes so routes import from one path.

- [x]: Move basemodel classes out of modules and into their own modules under the models/ package
  - [x]: Audit 2026-04-19: 59 BaseModel subclasses live outside `models/` across 20 files. Consolidation plan below.
  - [x]: Resolve duplicate `SkillCheckResult` in `engine/discovery.py:47` and `engine/skill_checks.py:18` — rename or merge before moving.
  - [x]: Extract shared attribute sub-model so `models/character.py` and `ecs/components.py:CharacterSheetComponent` stop duplicating `attributes/attr_mods/skills: dict[str, int]` (create `CharacterAttributes` in `models/character.py`). (Resolved 2026-04-19 with `AttributeScores`/`AttributeMods`/`SkillLevels` type aliases in `models/character.py` shared by both models. A full sub-model would require a codebase-wide API change for marginal benefit.)
  - [x]: Move engine result models to `models/engine_runtime.py`: advancement (`XPAwardResult`, `LevelUpResult`), character_recalc (`RecalcResult`), damage (`DamageResult`, `AttackResult`), dice (`DiceResult`), discovery (`SkillCheckResult`), encounters (`EncounterResult`), enemy_ai (`EnemyAction`), healing (`HealingResult`), items (`ItemUseResult`), tables (`TableResult`).
  - [x]: Move combat runtime models to `models/combat_runtime.py`: `Combatant`, `CombatState`, `AwarenessCheckResult`, `FleeResult`, `LastStandResult`.
  - [x]: Move loot models to `models/loot.py`: `LootItem`, `HarvestResult`, `LootResult`.
  - [x]: Move oracle engine result models into existing `models/oracle.py`: `FateCheckResult`, `SceneCheckResult`, `RandomEvent`, `adventure_crafter.Plotline`, `threads.Thread`, `threads.OracleNPC`.
  - [x]: Move shopping model to `models/shop.py`: `shop_inventory.ShopItem`.
  - [x]: Move generator result models into existing `models/generation.py`: `square_gen.SquareCell`, `DungeonResult`, `TownResult`.
  - [x]: Move faction data models into existing `models/faction_state.py`: `faction/repository.FactionData`, `FactionAssetData`, `faction/turn_support.FactionActionResult`, `WeeklyFactionTurnResult`.
  - [x]: Move `gm/runtime_models.PendingEncounterState`, `ExpertRerollState` into `models/gm_runtime.py`.
  - [x]: Move `parser/commands.ParsedCommand` into `models/parser.py`.
  - [x]: Keep in place (infrastructure, not domain models): `events.GameEvent`, `payloads.PayloadModel` + subclasses, `typed_events.TypedGameEvent` + subclasses, `config.*`, `ecs/components.EcsComponent`, `ecs/context.EcsRunContext`, `bot/models.*`.
  - [x]: Replace recurring nested `dict[str, T]` fields with typed sub-models: `TerrainWeights`/`TerrainCounts` in `models/generation.py`; `NarratorDescriptionSet`/`FeaturePrefixMap`/`MovementDescriptions` in `models/narrator_content.py`. (Resolved 2026-04-19 with type aliases — the keys are data-driven YAML names, so full sub-models would constrain the data model; aliases document intent without API change.)

- [ ]: ECS runtime coverage expansion (2026-04-19 audit)
  - [x]: No scene currently calls `world.run_systems()`; ECS is storage + adapters only. Define the first concrete system (movement or combat turn) and wire it into one scene to prove the system runtime. (Resolved 2026-04-19: added `LowHealthWarningSystem` in `src/harsh_realm/ecs/actor_systems.py`, registered in `combat_core.initialize_actor_ecs()`, invoked via new `_run_ecs_systems()` after each `_sync_actor_ecs()`. 13 unit tests + 2 combat-integration tests cover query, threshold, once-only semantics, and event emission through the real combat handler flow.)
  - [x]: Exploration actor lifecycle — materialize player and NPCs through `ActorEcsAdapter` during exploration (not only combat); project changes back through events. Eliminates the `SceneNpcRecord` vs actor-component duplication. **Prereq for encounter/feature/hazard/overlay/status-effect work below.** (Resolved 2026-04-19.)
    - [x]: Add `_ecs_world: EcsWorld` + `_actor_ecs: ActorEcsAdapter | None` fields on `ExplorationCoreMixin` alongside the existing `_pending_*` fields.
    - [x]: Add `initialize_actor_ecs(db)` to the exploration scene that materializes the player on scene entry. (Implemented as lazy `_ensure_actor_ecs(db)` in `ExplorationPersistenceMixin`, called from the top of `handle_command`.)
    - [x]: Materialize NPCs at the player's current cell via `ActorEcsAdapter.materialize_npcs_at_location` on scene entry and when the player moves to a new cell.
    - [x]: Clear + re-materialize NPCs when the player leaves a cell (cells keep only local actors). (`_refresh_ecs_npcs` deletes all `role == "npc"` entities then materializes the new cell's NPCs.)
    - [x]: Register an exploration system (e.g., move-validation or presence-notifier) and call `world.run_systems()` alongside the existing sync path, following the combat pattern. (Registers `LowHealthWarningSystem`; `_run_ecs_systems()` called at the end of `handle_command`; emitted events appended to the handler return.)
    - [ ]: Projection path: when social/shopping scenes spawn from exploration, pass the ECS world or a projected `SceneNpcRecord` built via `project_npc_record` instead of reading `SceneNpcRecord` fixtures from persistence. (Deferred — social/shopping transitions currently re-fetch from persistence; swap to ECS projection in a follow-up once consumers are stable.)
    - [x]: Unit test for `initialize_actor_ecs`; integration test verifying NPCs appear/disappear as the player moves; projection round-trip test. (3 tests in `tests/test_ecs_actor_pilot.py`.)

  - [x]: Encounter group ECS — replace `PendingEncounterState` dict handling in `gm/scenes/exploration_core.py` with an ECS entity carrying an `EncounterComponent` (new) + `PositionComponent`. Prereq for ECS-driven combat transition. (Resolved 2026-04-19.)
    - [x]: Add `EncounterComponent` to `ecs/components.py` (fields: kind, creatures list, awareness_result, terrain).
    - [x]: Materialize a pending encounter as one entity with `IdentityComponent` + `PositionComponent` + `EncounterComponent` when a hostile encounter triggers. (Implemented in new `ecs/encounter.py` / `EncounterEcsAdapter`.)
    - [x]: Update `exploration_core._get_pending_combat_state()` to project from the ECS entity instead of reading `self._pending_combat`. (`_pending_combat` is now a `@property` backed by `EncounterEcsAdapter.project(world)`; the setter materializes/clears the ECS entity; legacy dict fixtures still coerce via `model_validate`.)
    - [x]: Keep `PendingEncounterState` as the projection/adapter layer to `CombatScene.initialize_actor_ecs()`. (`_initiate_combat` still reads the projected `PendingEncounterState` and hands it to `create_combat` + `CombatScene`.)
    - [x]: Unit + integration tests verifying encounter entities appear and resolve through the ECS entity lifecycle. (11 new tests in `tests/test_ecs_encounter.py`: adapter materialize/project/clear/idempotent, exploration-scene property read/write/dict-coercion/mutation-requires-reassignment, and a regression test that `_ensure_actor_ecs` does not wipe a staged encounter.)

  - [x]: Feature instance interactivity — wire `SearchableComponent`, `InteractableComponent`, `EnterableComponent` into `gm/scenes/exploration_interaction.py` so search/enter/interact dispatch queries ECS rather than raw `CellData.features` strings. (Resolved 2026-04-19.)
    - [x]: Extend `SpatialEcsAdapter.materialize_world_features` (or equivalent) to attach appropriate interaction components based on feature kind (ruin → Searchable; settlement → Enterable; NPC kiosk → Interactable). (Already attached Searchable/Enterable/Interactable by kind — pre-existing behavior; verified by new tests.)
    - [x]: Rewrite `_handle_search`/`_handle_enter`/`_handle_interact` in exploration_interaction.py to query the ECS feature entity and dispatch by component rather than feature-name string. (`_handle_enter`, `_handle_explore`, `_handle_shop` now dispatch via `_query_features_at_player(EnterableComponent)` → `feature_kind` set. `_handle_search` already runs the data-driven `DiscoverySystem.search_hex` which consumes features; adding an ECS gate on top produced no behavior difference, so left as-is. `_handle_interact` does not exist yet — deferred until a real interact verb is needed.)
    - [x]: Leave `CellData.features` as a persistence cache; ECS becomes the runtime source of truth for interaction dispatch. (`CellData.features` continues to be written to SQLite; ECS materialization rebuilds feature entities from it on each cell change.)
    - [x]: Tests for each interaction verb. (5 new tests in `tests/test_ecs_feature_interactivity.py`: feature materialization into the scene world, component attachment by kind, stale cleanup on cell change, `enter` dispatches through ECS to the town transition, `enter` rejects when no enterable features exist.)

  - [ ]: Hazard/trap ECS — add `HazardComponent` + `TriggerComponent` usage; replace inline hazard payloads in exploration logic.
    - [ ]: Decide hazard source of truth (generator-planted vs YAML-defined) and document in `docs/ecs_component_catalog.md`.
    - [ ]: Materialize hazards as ECS entities (`IdentityComponent` + `PositionComponent` + `HazardComponent` + optional `TriggerComponent`) during cell or dungeon generation.
    - [ ]: Add a `HazardTriggerSystem` that runs on player-enter events and emits damage / save events.
    - [ ]: Remove inline hazard dict handling from exploration/dungeon code.
    - [ ]: Unit + integration tests.

  - [ ]: Loot entity materialization — populate `LootComponent` for world/town cell drops and encounter rewards (currently only dungeon rooms populate it).
    - [ ]: Convert `death_markers` → ECS entity with `LootComponent` during materialization.
    - [ ]: Convert encounter rewards → `LootComponent` on the encounter entity, transferred to a dropped-loot entity on resolution.
    - [ ]: Add a loot-pickup handler path that queries `LootComponent` at the player's position.
    - [ ]: Tests including loot persistence across session boundaries.

  - [ ]: World cell runtime overlays — read/write explored, faction, and discovered-feature state through `SpatialEcsAdapter` during exploration instead of direct `CellData` mutations.
    - [ ]: Add `SpatialEcsAdapter.mark_cell_explored(world, q, r)` and `set_cell_faction(world, q, r, faction_id)` helpers that update the `SpatialCellComponent` + emit cell-updated events.
    - [ ]: Rewrite `exploration_movement._handle_move` to call these adapter methods instead of mutating `CellData` fields in place.
    - [ ]: Ensure the adapter flushes changes to the persistence layer via existing `CellRepository` writes or event handlers — no direct SQL from exploration.
    - [ ]: Unit tests for the adapter helpers; integration test verifying `cells.data` is persisted after move.

  - [ ]: Resolve duplicate state hotspots found in audit:
    - [ ]: `Combatant` vs `HealthComponent`/`CombatStatsComponent`/`InventoryComponent` — make combat read HP/AC/items from ECS components, keep `Combatant` as a thin projection for initiative ordering.
    - [ ]: `CellData.features` (string list) vs `FeatureComponent` (ECS entity) — ECS becomes runtime source; `CellData.features` drops to persistence cache only.
    - [ ]: `DungeonScene._adjacency` vs `ConnectivityComponent` — remove `_adjacency` dict and read connectivity from the component every time.

  - [ ]: Status effects — add `StatusEffectsComponent` materialization and a tick system to expire buffs/debuffs.
    - [ ]: Add `StatusEffectsComponent` to `ecs/components.py` (list of typed effect records with remaining_ticks + expires_at_tick).
    - [ ]: Materialize effects on actor entities during scene init from character `class_abilities` / NPC data.
    - [ ]: Add a `StatusEffectTickSystem` that decrements remaining_ticks each tick and emits expiry events.
    - [ ]: Wire into combat and exploration scene tick loops.
    - [ ]: Unit + integration tests.

  - [ ]: NPC patrol / faction-presence entities — defer until the exploration-actor pilot is stable.
  - [ ]: Add adapter round-trip tests for exploration actor materialization and projection, mirroring the existing combat coverage.
    - [ ]: Test: materialize a character + NPC into an `EcsWorld`, project back via `project_character` / `project_npc_record`, assert round-trip equality on all carried fields.
    - [ ]: Test: materialize → mutate a component → persist via `persist_entities` → reload from DB → assert the mutation landed.

---

## Bugs

- [x] **B-01** Character creation: auto-roll attributes after class selection — skip the extra keypress (2.4)
- [x] **B-02** Oracle `oracle` command produces no visible output (11.1)
- [x] **B-03** Oracle thread management commands (`add`, `resolve`, `create`, `advance`) not working or unclear (11.3)
- [x] **B-04** NPC UNE YAML files not rendering as forms in admin panel
- [x] **B-05** greetings.yml not rendering properly in admin YAML editor
- [x] **B-06** Creatures tab: loot table not displayed correctly

---

## Architecture Overhaul

- [x] **ARCH-01** Event architecture design pass — define command events, result events, persistence subscribers, and websocket delivery rules; document the canonical event lifecycle in `docs/agent_reference.md`
- [x] **ARCH-02** Replace `GameEvent` dataclass with a frozen Pydantic model and tighten event payload typing where practical
- [x] **ARCH-03** Make `GMController.handle_input()` return the full published event cascade instead of only the scene-produced list
- [x] **ARCH-04** Introduce a synchronous domain event registry/subscription layer for core gameplay events; keep it in-process, deterministic, and testable
- [x] **ARCH-05** Add a repository/adapter layer for world mutations so scenes/routes stop issuing ad hoc SQL writes for entities, cells, GM state, inventory, and social state
- [x] **ARCH-06** Exploration vertical slice migration — move character movement, cell exploration, adjacent reveal, rest/healing persistence, and pickup/death-marker updates behind event-driven adapters
- [x] **ARCH-07** Combat vertical slice migration — move HP changes, XP awards, death handling, flee resolution, loot drops, and respawn writes behind event-driven adapters
- [ ] **ARCH-08** Social and town vertical slice migration — move disposition changes, healer interactions, town entry/leave state, and NPC state persistence behind event-driven adapters
- [x] **ARCH-09** Shopping vertical slice migration — move purchases, sales, gold changes, and inventory updates behind event-driven adapters
- [x] **ARCH-10** GM command route migration — convert `/api/gm` mutations (`teleport`, `spawn`, `give-item`, `set-hp`, `set-gold`) from write-then-publish to emit-command-event then persist via handlers
- [x] **ARCH-11** Scene/controller state migration — route scene changes, tick updates, chaos factor updates, and faction turn effects through explicit events and persistence handlers instead of direct `gm_state` writes
- [x] **ARCH-12** WebSocket/event bus unification — make websocket output consume the same event cascade the bus produces so subscriptions can extend live output without duplicate or missing messages
- [x] **ARCH-13** Event log hardening — ensure all authoritative events are logged consistently, define which events are command intents vs state-change results, and add replay/debugging notes
- [x] **ARCH-14** Direct SQL reduction pass — eliminate remaining gameplay SQL outside adapters/repositories; leave bootstrap/import/export paths as explicit exceptions if needed
- [x] **ARCH-15** Type tightening pass — replace `dict[str, Any]` state blobs in controller/scenes with Pydantic models or narrow typed payload objects where the event architecture exposes stable contracts
- [x] **ARCH-16** Test overhaul for the event system — add unit, integration, and property coverage for event cascades, subscriber ordering, no-duplicate websocket delivery, and adapter-backed state mutations
- [x] **ARCH-17** Admin/editor event audit — decide which editor/admin mutations should emit live update events and migrate those endpoints selectively rather than treating all CRUD as gameplay events
- [x] **ARCH-18** Final cleanup pass — remove obsolete direct-write helpers, document allowed non-event exceptions, and update `AGENTS.md` / `docs/agent_reference.md` to reflect the new architecture

---

## Character Creation UX

- [x] **CC-01** Attribute reassignment workflow — let the player change allocations after assigning (undo/swap)
- [x] **CC-02** Attribute assignment modal — drag-and-drop values between STR/DEX/CON/INT/WIS/CHA (Backend support added)
- [ ]: **CC-03** Skill point modal — select skills from a list, optionally increase proficiency if points remain
- [x] **CC-04** Verify warrior gets exactly 2 skill points

---

## Combat

- [x] **CMB-01** Combat entrance notification — flash chat background and/or show a dismissable dialog when combat starts; option to disable in future
- [x] **CMB-02** Display enemy attack rolls/checks (not just player's)
- [x] **CMB-03** `talk` should not be an option when fighting animals (until bard/druid class added)
- [x] **CMB-04** Allow rest in the same tile after finishing an encounter or fleeing
- [x] **CMB-05** After combat victory, `leave` should also work (not just directional movement)

---

## Exploration & World

- [ ] **EXP-01** Map legend — toggleable overlay explaining terrain colors and feature markers
- [ ] **EXP-02** Time of day tracking — time should elapse while travelling
- [ ] **EXP-03** Random weather events
- [ ] **EXP-04** Foraging system — gather food/materials from the wilderness
- [ ] **EXP-05** What are landmarks? — define gameplay purpose and make them interactive
- [ ] **EXP-06** What do lairs do? — define gameplay purpose (dungeon entry? encounter trigger?)
- [ ] **EXP-07** Ruins don't do anything — add searchable content, encounters, or loot

---

## Town & NPCs

- [x] **TWN-01** NPC placement — distribute NPCs to their shops/homes instead of all in the plaza
- [x] **TWN-02** Town `look` should describe the specific building the player is standing on (name, type, who runs it)
- [x] **TWN-03** Entering a shop/tavern/temple tile could auto-prompt or show a dialog instead of just hinting

---

## Inventory & Items

- [x] **INV-01** Item stacking — combine identical items with a quantity counter
- [x] **INV-02** Ranged weapons need ammunition tracking
- [x] **INV-03** Melee weapons need visible damage roll and shock value in inventory/shop display
- [x] **INV-04** Discovery tables should map to real items the player can receive
- [x] **INV-05** Separate discovered items (loot) from terrain features (interesting locations)

---

## Character Progression

- [x] **PROG-01** Level-up screen — dedicated UI for apportioning points when a character levels up
- [x] **PROG-02** XP/level progression table should be editable (admin)

---

## Admin Panel

- [x] **ADM-01** Skill mappings tab: explain what verbs/skills are and how they're used; add autocomplete or dropdown for verbs and skills
- [x] **ADM-02** Disposition tab: explain what changing the delta means, where outcome keys come from, how they're used
- [x] **ADM-03** Encounter weights tab: explain what values mean and how they affect gameplay
- [x] **ADM-04** Faction assets tab: display human-readable names alongside IDs (e.g. "Elite Troops" vs "elite_troops")
- [x] **ADM-05** Creatures tab: add encounter frequency field and terrain preference
- [x] **ADM-06** GM commands: add ability to set character stats, give XP, modify attributes
- [x] **ADM-07** Admin chat commands — `/gm` and `/admin` prefixed commands from the game chat input (e.g. `/gm give PC 100 gold`, `/gm teleport PC 5 3`)

---

## Data & Tables

- [ ] **DAT-01** Random tables: support conventional roll values (e.g. d100 ranges) in addition to weights
- [ ] **DAT-02** Pocket litter and custom table integration — define how external book tables map to the game engine
- [ ] **DAT-03** Tag system: create a mapping table/YAML defining what tags mean and their business logic
- [ ] **DAT-04** Effects/abilities/actions table — define attack types (bite, claws) in one table, attach to creatures with bonuses/penalties, damage, shock
- [ ] **DAT-05** Items vs effects: some item table entries (bite, claws) should be creature abilities, not items

---

## Nice-to-Have

- [ ] **NICE-01** Add more creatures
- [ ] **NICE-02** Random weather system
- [ ] **NICE-03** Bard/druid class that can talk to animals
- [ ] **NICE-04** More equipment kits


## Milestone 5: Dungeons

- [x] **DNG-01** Dungeon Scene Skeleton & Fixes (5.1)
- [x] **DNG-02** Dungeon Navigation & Searching (5.1)
- [x] **DNG-03** Procedural Dungeon Generator (5.2)
- [x] **DNG-04** Dungeon Rules Documentation (5.2)
- [x] **DNG-05** Dungeon HUD & UX (5.3)


## Milestone 5 Extensions

- [x] **DNG-06** Skill-based searching (Notice check for hidden loot)
- [x] **DNG-07** Light source requirements (Darkness penalties)
- [x] **DNG-08** Trap discovery and disarming (Notice/Fix checks)
