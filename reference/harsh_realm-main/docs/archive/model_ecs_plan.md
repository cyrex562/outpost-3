# Model And ECS Plan

This document defines the staged plan for two structural changes:

1. finish the remaining Pydantic migration for structured data
2. introduce a general ECS runtime where it reduces behavioral complexity

The goal is not to replace every class with ECS. The goal is to remove anonymous
state, isolate orthogonal behaviors, and make the most complex runtime flows
easier to extend.

## Principles

- Use Pydantic for structured payloads, stored records, and result objects.
- Keep repositories, services, registries, and controllers as normal classes.
- Treat plain `dict` as a compatibility boundary, not the internal source of
  truth.
- Use ECS only for runtime entity state with many orthogonal behaviors.
- Do not move API request/response models, repositories, or admin CRUD into ECS.

The scope boundary for ECS is defined in
[docs/ecs_adr.md](/home/cyrex/Projects/harsh_realm/docs/ecs_adr.md).

The candidate entity-family ranking is documented in
[docs/ecs_entity_inventory.md](/home/cyrex/Projects/harsh_realm/docs/ecs_entity_inventory.md).

The initial runtime architecture is documented in
[docs/ecs_runtime_architecture.md](/home/cyrex/Projects/harsh_realm/docs/ecs_runtime_architecture.md).

The first component catalog is documented in
[docs/ecs_component_catalog.md](/home/cyrex/Projects/harsh_realm/docs/ecs_component_catalog.md).

The item and loot ECS decision is documented in
[docs/ecs_item_evaluation.md](/home/cyrex/Projects/harsh_realm/docs/ecs_item_evaluation.md).

The ambient-world ECS decision is documented in
[docs/ecs_ambient_world_evaluation.md](/home/cyrex/Projects/harsh_realm/docs/ecs_ambient_world_evaluation.md).

## Status

Completed:

- most engine result models and API payloads are already Pydantic-backed
- `GameEvent` and gameplay request payloads are Pydantic-backed
- `ShopItem` is now a Pydantic value object
- UNE motivation, bearing, and personality payloads are now typed Pydantic
  models under `models/npc.py`

Still high priority:

- remaining ad hoc structured dict payloads in content loading and generators
- nested JSON blobs inside models that should become typed submodels
- scene/runtime state that still mixes transition state, actor state, and
  behavioral flags in one object graph

## Phase 1: Complete Pydantic Coverage

The purpose of this phase is to eliminate remaining anonymous structured payloads
 before any ECS work begins.

### Target areas

1. `engine/shop_inventory.py`
   - done for `ShopItem`
   - next: model shop YAML tiers and entries instead of caching raw dicts

2. `engine/npc_personality.py`
   - done for UNE result payloads
   - next: replace cached raw table rows with typed table-entry models where useful

3. `engine/adventure_crafter.py`
   - replace `Plotline.scenes: list[dict[str, Any]]` with a typed scene model
   - make `_generate_scene()` return that typed model

4. `generators/square_gen.py`
   - replace `rooms`, `connections`, and `buildings` raw dict lists in result
     models with `DungeonRoom`, `DungeonConnection`, and `SettlementBuilding`

5. `generators/settlement_gen.py`
   - replace `dict[str, Any]` settlement and establishment payloads with
     `SettlementData` and `BuildingData`

6. `engine/tables.py`, `engine/oracle.py`, `engine/loot.py`
   - type generator params/results and cached table payloads where they represent
     stable shapes rather than arbitrary YAML

7. `api/menu_handler.py`, `api/websocket.py`, `api/routes.py`
   - replace list/dict message payload assembly with existing message models
   - keep raw dict only at the final serialization boundary

8. `admin/content_mixin.py` and editor YAML status routes
   - introduce row/result models for named JSON content instead of broad
     `dict[str, object]`

### Exit criteria

- no remaining plain structured data holder classes in `src/harsh_realm`
- stable nested JSON payloads are represented by Pydantic models
- raw `dict` remains only for:
  - truly free-form YAML content
  - JSON serialization boundaries
  - intentionally open-ended extension fields

## Phase 2: General ECS Design

ECS should be introduced only after Phase 1, because ECS built on anonymous dict
payloads just relocates the ambiguity.

The intended ECS scope is general runtime simulation, not just cells.

### Where ECS helps most

ECS is most useful in Harsh Realm where all of these are true:

- runtime state changes frequently
- different entity types share overlapping behavior
- behavior is compositional rather than inheritance-driven
- new mechanics are likely to add orthogonal flags or temporary state

That points to the following entity families.

### Primary ECS entity families

1. Actors
   - player character
   - NPCs
   - combat enemies
   - summoned or temporary actors later

Why:

- shared position, identity, faction, inventory, and health state
- combat, movement, dialogue, AI, and status effects cut across actor type
- this is the highest-value first ECS slice

2. Spatial cells and map features
   - world hexes
   - dungeon cells / rooms
   - town cells
   - feature instances such as lairs, ruins, landmarks, exits, and searchable nodes

Why:

- cells increasingly carry composable behaviors such as enterable, searchable,
  lootable, hostile, blocking, settlement-linked, or dungeon-linked
- town and dungeon scenes already behave like grids with layered feature rules
- this is the right long-term evolution of the original “cells as ECS” idea

3. Item and loot entities
   - world loot markers
   - dropped equipment
   - container contents
   - inventory items once stacking/ammo/equipment rules deepen

Why:

- items are acquiring more orthogonal behavior: stackable, equipped, consumable,
  weapon, armor, quest-tagged, container-bound, tradeable
- item-instance ECS is useful once inventory rules expand, but should not be the
  first slice

4. Encounter and hazard entities
   - encounter groups
   - traps
   - hazards
   - weather fronts or regional effects later

Why:

- these are transient runtime objects with timers, triggers, and effect payloads
- ECS is a good fit for temporary behavior bundles that should not become
  hard-coded scene flags

5. Faction-world entities
   - faction patrols
   - expeditions
   - influence nodes
   - mobile threat sources

Why:

- the current faction data model is already relational and stable
- the faction strategy layer itself does not need ECS first
- ECS becomes useful when factions gain runtime-presence entities on the map

### Things that should not become ECS entities by default

- API request and response models
- repository row models
- editor and admin CRUD records
- static YAML content definitions
- event transport envelopes
- long-lived persistence schema definitions

## Phase 3: ECS Runtime Architecture

The ECS runtime should remain in-memory and explicit.

### Core abstractions

- `EntityId`
- component models as typed Pydantic classes
- `EcsWorld` or equivalent registry/store
- query helpers for component intersections
- system protocol with explicit inputs and outputs
- adapter layer between repositories and ECS entities
- event bridge so systems emit typed domain events instead of mutating UI state

### Initial component catalog

The first component set should stay small and grow only when behavior repeats.

- `IdentityComponent`
- `PositionComponent`
- `ScenePresenceComponent`
- `MovementComponent`
- `HealthComponent`
- `CombatStatsComponent`
- `InventoryComponent`
- `EquipmentComponent`
- `DispositionComponent`
- `FactionAffiliationComponent`
- `NpcRoleComponent`
- `InteractableComponent`
- `SearchableComponent`
- `LootComponent`
- `EncounterComponent`
- `HazardComponent`
- `StatusEffectsComponent`
- `AiIntentComponent`

### Initial systems

- scene materialization
- movement and collision
- interaction dispatch
- search and discovery
- encounter activation
- combat turn setup and resolution
- flee and disengage
- loot spawn and pickup
- disposition and social reaction changes
- status effect ticking

## Phase 4: ECS Pilot And Rollout

Start with actor runtime state inside combat and exploration.

Why:

- multiple actor types already share overlapping fields
- combat and exploration already mix movement, inventory, health, AI, and
  temporary state
- this gives the cleanest proof that ECS reduces complexity before cells and
  map features are moved over

### Pilot sequence

1. actor ECS in combat and exploration
2. cell/feature ECS in town and dungeon runtime
3. loot and hazard ECS
4. optional faction patrol and ambient-world ECS

### Non-goals for ECS

- replacing SQLite repositories with ECS storage
- replacing `Character`, `NPCData`, or API schemas with ECS-only models
- moving admin/editor CRUD into ECS
- converting the whole application at once

## Phase 5: Persistence Bridge

Once the combat/exploration ECS pilot is stable:

- add adapters between persistent Pydantic models and runtime ECS state
- ensure event handlers can materialize ECS state, run systems, and write back
  through repositories
- keep event payloads typed and independent of ECS internals

## Suggested execution order

1. type `adventure_crafter` scene payloads
2. type `square_gen` and `settlement_gen` result payloads
3. remove dict-built websocket/menu message payloads in favor of message models
4. type admin/editor content result rows where shapes are stable
5. define the ECS entity families, component catalog, and runtime boundaries
6. implement a minimal ECS runtime package behind the current scene API
7. migrate combat and exploration actors to the ECS pilot
8. expand ECS to town and dungeon cells/features if the actor pilot reduces
   complexity
9. add optional loot, hazard, and faction-presence ECS slices only when those
   mechanics need them

## Guardrails

- every migration slice should be independently testable
- preserve current API contracts unless a coordinated frontend change is made
- prefer adapters over flag days
- if a dict shape is intentionally open-ended, keep it and document why
