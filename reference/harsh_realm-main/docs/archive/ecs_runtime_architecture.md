# ECS Runtime Architecture

This document defines the initial ECS runtime architecture for Harsh Realm.

It is intentionally narrow. The goal is to define the core runtime pieces well
enough to build the first ECS package without making storage, query, and system
behavior up ad hoc during implementation.

This document follows the boundaries set by:

- [docs/ecs_adr.md](/home/cyrex/Projects/harsh_realm/docs/ecs_adr.md)
- [docs/ecs_entity_inventory.md](/home/cyrex/Projects/harsh_realm/docs/ecs_entity_inventory.md)

## Goals

The initial architecture must support:

- stable, opaque entity identifiers
- typed Pydantic components
- an in-memory world/registry
- efficient component-intersection queries for small to medium runtime slices
- explicit systems with deterministic ordering
- integration with the existing event bus and repository adapters

It does not need to support:

- persistence ownership
- ORM-style change tracking
- distributed execution
- arbitrary plugin loading in the first pass

## Core Types

## EntityId

`EntityId` should be an opaque runtime identifier, represented as `str`.

Requirements:

- unique within one `EcsWorld`
- stable for the lifetime of that world instance
- not required to match persistent database IDs
- may optionally encode origin during materialization for debugging, such as:
  - `actor:pc:<entity_id>`
  - `actor:npc:<entity_id>`
  - `feature:lair:<q>:<r>`
  - `cell:dungeon:<room_id>`

Design decision:

- use `str` rather than a wrapper class for the first iteration
- keep the alias local to the ECS package so a stronger type can be introduced
  later without changing every caller immediately

## Component Model

All ECS components should be typed Pydantic models.

Component rules:

- one component per orthogonal concern
- component fields must be explicit and typed
- components should be mutable `BaseModel` objects unless immutability is
  specifically useful
- components should avoid embedding repository objects, database handles, or
  transport-only payloads
- components may carry persistent IDs when needed for reconciliation back to
  repositories

Base component guidance:

- define a minimal common marker base such as `EcsComponent`
- the base should not own business logic beyond shared validation/config
- business logic belongs in systems, not component methods

## World / Registry Storage

The runtime container should be `EcsWorld`.

Responsibilities:

- create and delete entities
- attach, replace, and remove components
- retrieve one component for one entity
- query entities by component intersection
- hold deterministic system registration order
- collect domain events produced during execution

Recommended internal storage:

1. `entities: set[EntityId]`
2. `components_by_type: dict[type[EcsComponent], dict[EntityId, EcsComponent]]`
3. optional `entity_tags` or metadata only if needed later

Why this shape:

- simple and explicit
- works well for the bounded runtime slices expected in scenes
- makes intersection queries straightforward
- avoids premature optimization before real ECS load exists

Initial world API should include methods equivalent to:

- `create_entity(entity_id: EntityId | None = None) -> EntityId`
- `delete_entity(entity_id: EntityId) -> None`
- `set_component(entity_id: EntityId, component: EcsComponent) -> None`
- `get_component(entity_id: EntityId, component_type: type[T]) -> T | None`
- `require_component(entity_id: EntityId, component_type: type[T]) -> T`
- `remove_component(entity_id: EntityId, component_type: type[EcsComponent]) -> None`
- `has_component(entity_id: EntityId, component_type: type[EcsComponent]) -> bool`

## Query API

The first query API should remain explicit and small.

Required operations:

1. Iterate entities that have all requested component types.
2. Return typed component tuples for those entities.
3. Support optional exclusion filters later, but do not require them for the
   first implementation.

Recommended interface shape:

- `world.query(PositionComponent, HealthComponent)`
- yields `(entity_id, position_component, health_component)`

Implementation notes:

- choose the smallest component table as the query anchor
- intersect entity IDs from the requested component maps
- preserve deterministic ordering by sorting entity IDs or by insertion order
  if that is stable enough for tests

Design decision:

- no string-based query language
- no lazy SQL-like planner
- no hidden caching in the first pass

## System Protocol

Systems should be explicit objects or callables registered in execution order.

Each system should:

- receive the `EcsWorld`
- optionally receive a typed execution context object
- read and write components
- emit domain events or outcome records through the world/event bridge
- avoid direct repository writes

Recommended protocol:

- `name: str`
- `run(world: EcsWorld, context: EcsRunContext) -> list[GameEvent] | None`

`EcsRunContext` should be a typed Pydantic model carrying runtime-only inputs
such as:

- current scene
- current tick
- initiating command or action kind
- RNG/seed handle reference if needed
- current actor entity ID

System ordering rules:

- systems run in deterministic registered order
- systems should be grouped by concern, not by entity type
- side effects should be visible through emitted events or updated components
- systems should not call each other directly in the first design

## Event-Bus Integration

The existing `EventBus` remains the integration seam.

ECS should integrate with it in two places.

### 1. Input side

Runtime orchestration may materialize an `EcsWorld` in response to:

- scene entry
- command-intent events such as `*_requested`
- simulation steps that need multi-entity behavior resolution

That orchestration stays outside ECS core. ECS itself should not subscribe to
the application event bus directly.

### 2. Output side

Systems should emit typed gameplay `GameEvent` objects or typed outcome objects
that are converted to `GameEvent` objects at the adapter boundary.

Recommended initial approach:

- `EcsWorld` owns an internal list of pending domain events
- systems append to that list through a narrow method like `emit_event(...)`
- the adapter/orchestrator flushes those events to the app `EventBus`

Why:

- preserves current event architecture
- keeps ECS deterministic and testable without a live app
- avoids coupling core ECS storage to FastAPI or websocket state

## Repository Integration Points

Repositories remain responsible for persistence.

The bridge should be a thin adapter layer with three responsibilities:

1. materialize ECS entities from persistent/domain models
2. run one or more systems
3. translate ECS state deltas and emitted events back into repository writes and
   published `GameEvent` objects

Examples:

- `ActorEcsAdapter` loads character/NPC/enemy records into actor entities
- `SpatialEcsAdapter` loads town/dungeon/world feature runtime entities
- `CombatEcsRunner` or `ExplorationEcsRunner` coordinates system execution for a
  scene slice

Design rule:

- ECS core never imports repository modules
- adapters may import both repository and ECS packages

## Package Layout

The first ECS package should live under `src/harsh_realm/ecs/`.

Suggested initial modules:

- `types.py`: `EntityId`, generic type aliases
- `components.py`: base component types and early shared components
- `world.py`: `EcsWorld`
- `query.py`: query helpers if they need isolation
- `systems.py`: system protocol and registry helpers
- `context.py`: `EcsRunContext`
- `events.py`: ECS-local event buffer helpers
- `adapters/`: materialization and persistence bridge modules

Do not split into many packages until the first pilot proves the boundary.

## Determinism And Testing

The runtime architecture should optimize for determinism.

Required properties:

- deterministic entity iteration in tests
- deterministic system order
- no hidden background work
- no repository writes from systems
- no event publication side effects hidden inside component setters

Test surface implied by this architecture:

- entity/component lifecycle tests
- query intersection tests
- system-ordering tests
- event-buffer tests
- adapter round-trip tests

## First-Pass Constraints

To keep the first implementation controlled:

- no archetype storage
- no dynamic dependency graph between systems
- no async systems in ECS core
- no direct `EventBus` subscription inside ECS core
- no persistence writes inside ECS core
- no attempt to model every runtime domain at once

## Recommended First Pilot

The first pilot should use this architecture for actor runtime state in combat
and exploration.

That pilot should prove:

- actor entities can be materialized cleanly from current repositories
- combat/exploration behavior becomes simpler under component queries
- emitted events still flow through the existing `EventBus`
- repository writes remain outside the ECS core
