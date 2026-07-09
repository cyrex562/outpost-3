# Harsh Realm Agent Reference

This document is a working reference for future coding work in this repository.
It is based on the current implementation, but it also records explicit
direction for how new work should move the codebase forward.

## 1. What this repo is

- Backend: FastAPI app in `src/harsh_realm/`
- Persistence: per-world SQLite databases wrapped by `WorldDatabase`
- Runtime model: a GM controller routes player commands into scene handlers and
  emits `GameEvent`s
- Frontend: Vue 3 + Pinia + TypeScript app in `frontend/`
- Content/config: YAML in `data/`

The codebase is a hybrid of:

- A clear intended architecture described in `AGENTS.md`
- A pragmatic current implementation with some shortcuts and drift from that spec

When editing, preserve runtime compatibility, but bias new work toward the
architectural decisions recorded in this document.

## 1.1 Current direction

These decisions are explicit and should guide future changes:

- Prefer Pydantic models over stdlib dataclasses for structured application data
- Route more state changes through the `EventBus` to support signaling and
  subscriptions
- Enforce TypeScript strictness and correctness rather than tolerating `any`
- Introduce database write adapters/repositories so callers do not assemble SQL
  inline
- Treat the loaded world database as the source of truth

These are not just ideals. New code should follow them unless there is a strong
compatibility reason not to.

## 2. Primary runtime flow

### Backend startup

- App entrypoint: `src/harsh_realm/main.py`
- Routers mounted there:
  - `api/routes.py`
  - `api/admin_routes.py`
  - `api/editor_routes.py`
  - `api/gm_routes.py`
  - `api/websocket.py`
- Shared app state:
  - `config`
  - `event_bus`
  - `active_world`
  - `gm_controller`
  - `connection_manager`

### World lifecycle

- Worlds are `.db` files under the configured worlds directory
- `POST /api/worlds` creates a DB, generates terrain/cells, seeds admin tables
- `POST /api/worlds/load` opens a DB, clears and rewires the event bus, creates a
  `GMController`
- `GET /api/worlds/current` is how the frontend restores session state

### Player command flow

1. Frontend sends a WebSocket `{"type":"command","text":"..."}` message
2. `api/websocket.py` delegates to `GMController.handle_input()`
3. `GMController` parses the command, routes it to the active scene
4. Scene returns `GameEvent`s
5. Controller publishes them to the `EventBus`, handles transitions, saves GM state
6. WebSocket delivery uses the same resolved event cascade and the same event-to-message formatter used by the bus broadcaster

The controller now has a two-stage event path:

1. Scenes/routes emit command or internal request events
2. `gm/domain_events.py` resolves those through in-process handlers that own
   persistence
3. Only the resulting public events are published to the shared `EventBus`

That pattern is now in place for exploration, shopping, GM mutation routes, and
controller-owned `gm_state` persistence.

WebSocket delivery is now unified around that cascade:

- command responses use the exact `GMController.handle_input()` cascade
- the websocket module broadcasts that cascade with the same formatter used by
  wildcard event-bus subscriptions
- external/public events that are published outside the command-response path
  still reach clients through the event-bus broadcaster
- duplicate delivery is prevented by suppressing the wildcard broadcaster while
  the command-response cascade is being explicitly broadcast

### Editor/admin live-update policy

Editor and admin mutations now have a separate live-update policy from gameplay
domain events.

Audit/editor events still exist for broad CRUD visibility, but only selected
mutations publish subscription-friendly live updates:

- `editor.live_update`
  - only emitted when the edited DB is the currently loaded world
  - covers world-facing editor mutations:
    - cells
    - entities
    - dungeons
    - factions, faction relations, and faction assets
    - oracle threads, oracle NPCs, and oracle chaos state
- `admin.config_updated`
  - only emitted when the edited DB is the currently loaded world
  - covers runtime-relevant config mutations:
    - skill mappings
    - difficulty targets
    - disposition outcomes
    - encounter weights
    - faction asset stats

Intentional non-live surfaces for now:

- YAML file maintenance
- import/export and transfer operations
- world cloning and world meta maintenance
- generic content CRUD such as random tables unless a task explicitly needs live
  subscriptions there

## 3. Most important backend modules

### `db.py`

- Central SQLite wrapper
- Holds schema creation and general-purpose `execute`, `fetch_one`, `fetch_all`
- Intended rule in `AGENTS.md`: no raw SQL outside `db.py`
- Actual repo state: many modules still issue SQL directly through the wrapper

### `events.py`

- Contains `EventBus`, `GameEvent`, and `EventLogger`
- `GameEvent` is now a frozen Pydantic model
- Handlers are sync callables; async persistence is done by scheduling work inside
  `EventLogger`
- `EventLogger` now writes event-log taxonomy metadata:
  - `event_id`
  - `source`
  - `event_kind`
  - `authoritative`

### Event taxonomy

The repo now has an explicit event-log taxonomy:

- `command_intent`: internal request events ending in `_requested`
- `presentation`: UI/projection events such as `gm.narrate`,
  `gm.suggestions`, and `town.map`
- `domain_result`: public outcome/state events that are neither of the above

Current logging rule:

- authoritative events are currently the same as `domain_result`
- command intents and presentation events are still logged, but they are marked
  non-authoritative so replay/debug tooling can filter them cleanly

### `gm/controller.py`

- Main gameplay coordinator
- Owns:
  - parser
  - active scene state
  - tick
  - chaos tracker
  - faction turn engine
- Handles global oracle/thread/plotline commands directly
- Emits scene change events and appends suggestion events

### `gm/scenes/`

- Scene handlers implement the `SceneHandler` protocol in `gm/scenes/base.py`
- Major scenes:
  - `character_creation.py`
  - `exploration.py`
  - `combat.py`
  - `social.py`
  - `shopping.py`
  - `dungeon.py`
  - `town.py`
  - `respawn.py`

### `admin/service.py`

- Main CRUD/reset service for admin config and content seeded from YAML
- Good place for reusable admin logic
- Returns Pydantic row-backed models from `models/admin.py`

### `api/editor/`

- Split editor/admin world-state surface for cells, entities, factions, dungeons,
  oracle data, YAML, import/export, and meta operations
- Shared event and DB resolution helpers live in `api/editor/common.py`
- This package is still a high-leverage integration surface because it bridges
  active-world editing, audit events, and selective live updates

## 4. Current architecture realities to respect

These are important because they differ from the intended project rules.

### Structured data models are now Pydantic-backed

The application code under `src/harsh_realm` no longer uses stdlib
`@dataclass` for structured runtime models. Shared payloads, engine result
objects, config records, repositories' row-backed models, and scene handoff
objects now use Pydantic models.

Direction for future work:

- Keep new structured models on Pydantic `BaseModel`
- Prefer frozen Pydantic models for immutable value/result objects
- Preserve compatibility at API boundaries when tightening validation or field
  shapes

Completed migrations include:

- `events.GameEvent`
- `models.character.Character`
- `models.cell.TerrainType`
- `models.cell.CellData`
- `models.grid.GridCoord`
- `models.npc.NPCData`
- `config.AppConfig` and nested config models
- `parser.commands.ParsedCommand`
- `engine.dice.DiceResult`
- `engine.damage.DamageResult`
- `engine.damage.AttackResult`
- `engine.skill_checks.SkillCheckResult`
- `engine.loot.LootItem`
- `engine.loot.HarvestResult`
- `engine.loot.LootResult`
- `engine.healing.HealingResult`
- `engine.items.ItemUseResult`
- `engine.threads.Thread`
- `engine.threads.OracleNPC`
- `engine.oracle.FateCheckResult`
- `engine.oracle.SceneCheckResult`
- `engine.oracle.RandomEvent`
- `engine.combat.AwarenessCheckResult`
- `engine.combat.Combatant`
- `engine.combat.CombatState`
- `engine.combat.FleeResult`
- `engine.combat.LastStandResult`
- `models.creature.CreatureData`
- `engine.discovery.SkillCheckResult`
- `engine.discovery.DiscoveryResult`
- `engine.encounters.EncounterResult`
- `engine.tables.TableResult`
- `engine.enemy_ai.EnemyAction`
- `engine.advancement.XPAwardResult`
- `engine.advancement.LevelUpResult`
- `engine.character_recalc.RecalcResult`
- `engine.adventure_crafter.Plotline`
- `faction.repository.FactionData`
- `faction.repository.FactionAssetData`
- `generators.square_gen.SquareCell`
- `generators.square_gen.DungeonResult`
- `generators.square_gen.TownResult`

### Raw SQL still exists outside `db.py`

Despite the stated rule, raw SQL still appears in:

- `api/routes.py`
- `api/editor/` modules
- import/export/bootstrap paths
- repositories/adapters that intentionally own persistence
- several admin/editor modules

Direction for future work:

- New database write logic should move behind adapters/repositories/services
- Avoid introducing new inline SQL in routes, scenes, or frontend-driven
  handlers when an adapter can own that behavior
- Keep gameplay SQL concentrated in repository/adapter modules rather than
  scenes, engines, controllers, or command routes
- Existing non-gameplay inline SQL can be tolerated temporarily, but should be
  reduced over time

### Event-bus purity is partial

`AGENTS.md` says all state changes should flow through the event bus. In practice:

- gameplay mutations mostly flow through request events and persistence handlers
- editor/admin maintenance endpoints still often write directly through service
  or repository layers
- editor/admin live updates are selective by design, not universal

Direction for future work:

- New gameplay state-change paths should prefer flowing through event-driven
  logic
- Editor/admin writes should emit live updates only when the loaded world or
  runtime config benefits from subscriptions
- The goal is not only logging; it is enabling subscriptions, signaling, and
  downstream reactions without turning all CRUD into gameplay-domain events
- Preserve current behavior first, but avoid expanding non-event-driven
  gameplay writes or noisy live-update surfaces

Current migrated areas:

- exploration movement/rest/pickup/search
- exploration settlement/shop/dungeon lookup helpers
- shopping buy/sell
- `/api/gm` mutation commands
- controller-owned `gm_state` writes for scene, tick, chaos factor, and faction
  weekly-turn markers
- table/discovery/skill-check config access through dedicated repositories
- faction turn/reputation config access through the faction repository
- websocket delivery now shares one event-to-message contract for both explicit
  command responses and bus-driven live broadcasts

Current repository boundary after `ARCH-14`:

- gameplay scenes/controllers/routes should not issue SQL directly
- gameplay persistence and config access should go through repository/adapter
  modules such as:
  - `gm/entity_repository.py`
  - `gm/cell_repository.py`
  - `gm/gm_state_repository.py`
  - `engine/random_table_repository.py`
  - `engine/discovery_repository.py`
  - `engine/skill_config_repository.py`
  - `engine/oracle_repository.py`
  - `faction/repository.py`
- remaining direct SQL outside those areas is mainly editor/admin, import/export,
  or world/bootstrap plumbing

Allowed non-event/non-repository exceptions after `ARCH-18`:

- import/export routes that reshape arbitrary table data
- world/meta/bootstrap helpers
- editor/admin maintenance endpoints where emitting an audit event or selective
  live update is sufficient and a gameplay-domain event would add no value

### Typed payload boundary after `ARCH-15`

The repo now has a typed payload layer in `src/harsh_realm/payloads.py` for the
most stable gameplay contracts:

- internal `_requested` event payloads for exploration, shopping, GM commands,
  and `gm_state` persistence
- scene transition handoff state for social, shopping, dungeon, and town scenes

Current rule for new work:

- when an event or scene-handoff contract is stable and shared across modules,
  prefer a Pydantic payload model over ad hoc `dict[str, Any]`
- construct event payloads with `model_dump(mode="json")`
- validate incoming event payloads in handlers with `model_validate(...)`

What is still intentionally looser:

- combat encounter staging and other rapidly changing scene-local scratch state
- editor/admin CRUD payloads that have not been migrated yet
- legacy runtime models such as `Character` and `GameEvent`, which still need
  broader model migration work outside this pass

### Event-system test coverage after `ARCH-16`

The event architecture now has explicit tests for:

- `EventBus` cascade behavior and taxonomy metadata
- `DomainEventDispatcher` async handling, exception suppression, and deterministic
  depth-first subscriber ordering
- invalid typed payloads failing closed without mutating state
- adapter-backed mutation paths in controller/gameplay tests
- websocket command delivery staying single-shot without duplicate bus echoes
- a Hypothesis property that checks the event taxonomy rule across arbitrary
  event names

When changing the event system, the fastest confidence slice is:

- `tests/test_events.py`
- `tests/test_event_wiring.py`
- `tests/test_properties.py`
- `tests/test_websocket.py`
- `tests/test_gm_controller.py`

### Frontend strictness is aspirational, not fully enforced in code

- `frontend/tsconfig.json` has `strict: true`
- but `frontend/src/stores/admin.ts` still has extensive `any`
- some components also rely on `any`

Direction for future work:

- Use `frontend/src/types/api.ts` as the shared contract layer
- Do not add new `any`
- Prefer replacing existing `any` with concrete interfaces when touching code
- Treat TypeScript correctness as a real requirement, not cleanup for later

## 5. Frontend map

### Entry points

- `frontend/src/main.ts`
- `frontend/src/router.ts`
- views:
  - `views/GameView.vue`
  - `views/AdminView.vue`

### Important stores/composables

- `stores/game.ts`: chat log, character sidebar state, suggestions, scene, chaos
- `stores/world.ts`: world load/create state
- `stores/map.ts`: world map state
- `stores/town.ts`: town map state
- `stores/layout.ts`: panel/window layout
- `stores/admin.ts`: large admin/editor store; high churn, weak typing
- `composables/useWebSocket.ts`: central client event handling

### Frontend event contract

`useWebSocket.ts` is effectively the live UI adapter layer. If backend event
payloads change, update this file and the related stores/components together.

Common event types consumed explicitly:

- `exploration.enter_cell`
- `gm.suggestions`
- `gm.scene_change`
- `character.created`
- `character.hp_changed`
- `character.xp_gained`
- `character.level_up`
- `shopping.purchase`
- `shopping.sale`
- `social.disposition_change`
- `faction.turn_completed`
- `oracle.chaos_changed`
- combat events
- town events

## 6. Data/content layout

- Game/config YAML lives under `content/<pack-id>/content/`
- Admin/editor schemas live under `content/schemas/editors/`
- Rules/reference docs live under `docs/` and repo-root markdown files
- Existing rules reference files:
  - `docs_rules_reference_social.md`
  - `docs_rules_reference_oracle.md`
  - `docs_rules_reference_faction_turns.md`

Use YAML defaults as source-of-truth when resetting world config tables through
`AdminService`.

## 7. Testing reality

### Python

- Main command: `pytest --tb=short -q`
- There is broad test coverage in `tests/`
- Property tests exist in `tests/test_properties.py`
- Bot tests require `--run-bot`

### Frontend

- `frontend/package.json` currently exposes:
  - `npm run build`
  - `npm run type-check`
  - `npm run test:e2e`
- I do not see a configured Vitest unit-test script yet, despite `AGENTS.md`
  describing one as the desired standard

Practical rule: for frontend changes, at minimum run `npm run type-check` and the
most relevant Playwright coverage if the task touches UI behavior.

## 8. Safe change strategy for this repo

When modifying code here:

1. Read the route/store/scene pair together before changing behavior
2. Preserve event names and payload shapes unless the task includes frontend sync
3. Be careful with duplicated state:
   - entity table columns
   - entity `data` JSON
   - frontend store projections
   - event payload projections
4. Prefer small patches in large files like `api/editor_routes.py`,
   `frontend/src/stores/admin.ts`, and `gm/scenes/combat.py`
5. Add tests near the touched subsystem, even if full four-layer coverage is not
   yet consistently implemented in the current repo
6. When introducing a new persistence path, ask whether that logic belongs in a
   database adapter/repository instead of the caller
7. When introducing state changes, ask which event should represent them

Large or dramatic refactors are acceptable in this repository when they
materially improve the architecture and are backed by extensive tests that
validate the changed behavior.

## 9. High-risk areas

- `api/editor_routes.py`: very large, many responsibilities, dynamic SQL assembly
- `frontend/src/stores/admin.ts`: large store, weak typing, many fetch wrappers
- `gm/scenes/combat.py`: large and stateful
- WebSocket event shape changes: easy to break UI behavior silently
- Character state duplication between DB columns and JSON `data`

## 10. Source of truth

For a loaded world, the database is the source of truth.

That means:

- In-memory/frontend state is a projection, cache, or convenience layer
- Event payloads should describe changes, not become the canonical store
- If duplicated state exists between columns and JSON blobs, changes should be
  made deliberately and consistently, with a bias toward clear DB-owned truth
- Refactors should reduce ambiguity about where authoritative state lives

## 11. Guidance for future agents

### Event lifecycle target

The event system is now being pushed toward a stricter contract:

- Scene and route code should produce command or domain events, not perform
  the authoritative write and then emit a descriptive event afterward.
- Controller-owned publish points should return the full published cascade,
  including subscriber-generated events such as scene-change side effects,
  chaos updates, and future persistence-triggered events.
- The database remains the source of truth. Event payloads describe requested
  or completed changes; they do not replace persisted state.
- Event handlers/subscribers are intended to be synchronous and in-process for
  now. Async delivery is not required to get the extensibility benefit.
- WebSocket output, event logging, and persistence adapters should eventually
  consume the same event cascade so extensions do not need bespoke wiring.
- Event logging now records enough metadata to separate command intent,
  authoritative results, and presentation noise during debugging or replay
  analysis.

Current implementation note:

- `GMController.handle_input()` now returns the full controller-owned publish
  cascade, not just the raw events returned by the active scene.
- A new async-capable in-process domain dispatcher sits ahead of the public
  event bus. It is intended for authoritative command/result flows where
  subscribers may need to perform DB-backed work before public events are
  emitted.
- Exploration movement is the first migrated slice: the scene now emits an
  internal `exploration.move_requested` domain event, and a registered
  exploration handler performs the persistence work and emits the public
  `action.move` / `exploration.enter_cell` result events.
- Exploration rest, pickup, and search now follow the same pattern:
  `exploration.rest_requested`, `exploration.take_requested`, and
  `exploration.search_requested` are resolved by exploration domain handlers,
  which persist the authoritative DB changes and emit the public result events.
- Shopping now follows the same model: `shopping.purchase_requested` and
  `shopping.sale_requested` are resolved by shopping domain handlers, which
  persist inventory/gold changes and emit the public shopping result events.
- `/api/gm` mutation routes now dispatch request events first and persist
  through GM command handlers rather than mutating the database directly in the
  route function.
- Controller-owned `gm_state` persistence now follows the same request/result
  boundary, so scene/tick/chaos/faction-turn markers are no longer written
  directly from the controller.
- The event log now stores `event_id`, `source`, `event_kind`, and
  `authoritative` alongside the payload, which is enough to inspect command
  chains and filter replay/debug output without changing the payload contract.
- Scene modules, gameplay helpers, and several HTTP mutation routes still
  perform direct DB writes before or alongside emitting events. Those are the
  main remaining migration targets.

- Treat `AGENTS.md` as target architecture, not a literal description of current
  implementation
- Do not assume a clean Pydantic model layer exists everywhere, but move toward
  one
- Do not assume the event bus is already the sole write path, but prefer it for
  new signaling-friendly changes
- Prefer adding or extending adapters/repositories over spreading inline SQL
- Treat strict TypeScript as enforceable policy for new code
- When in doubt, search for the event name or route path and trace both backend
  producer and frontend consumer before editing

## 12. Remaining design tension

The main unresolved implementation issue is migration strategy, not direction.

The direction is now clear:

- Pydantic over dataclasses
- more event-driven state propagation
- stricter TypeScript
- DB adapters over scattered SQL
- database as source of truth

The open implementation question on future tasks is usually:

- do the smallest compatible change now, or spend scope on migrating the touched
  subsystem toward that target

Current preference:

- substantial change is acceptable if the patch also adds enough tests to make
  the new behavior trustworthy
