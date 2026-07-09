# ECS ADR

## Status

Accepted

## Context

Harsh Realm now has much stronger typed persistence and clearer repository
boundaries, but runtime simulation logic is still spread across scene handlers,
runtime payload models, and feature-specific helpers.

The original ECS note started from cells, because cells are accumulating more
behavior over time. That is directionally correct, but too narrow. The real
problem is broader: multiple runtime domains now have overlapping mutable state
and composable behaviors that do not fit cleanly into one inheritance tree or a
single scene-state object.

Examples:

- actors share position, health, faction, inventory, AI, and interaction state
- cells and feature instances share search, enter, block, loot, and trigger
  behavior
- encounters, hazards, and temporary effects need lifecycle-driven runtime state
- future faction patrols or mobile threats will likely behave like world actors

At the same time, not all structured data in the system should become ECS.
Persistence records, API payloads, editor models, and static content definitions
serve different purposes and already have stronger boundaries.

## Decision

Harsh Realm will treat ECS as a runtime-simulation architecture, not as a
general-purpose modeling pattern.

The intended use of ECS is:

- in-memory runtime entities
- typed components for orthogonal behavior
- explicit systems that operate over queried component sets
- adapters that materialize ECS entities from repositories and write outcomes
  back through the existing repository and event architecture

ECS is not the source of truth for persistence, transport, or admin data
management.

## Scope

ECS is in scope for runtime domains where all of the following are true:

- state changes frequently during scene execution
- multiple entity types share overlapping behavior
- behavior is compositional rather than hierarchy-driven
- future mechanics are likely to add more independent flags, timers, or traits

Initial in-scope entity families:

1. Actors
   - player character
   - NPCs
   - combat enemies
   - later summoned or temporary actors

2. Spatial runtime entities
   - world hexes
   - town cells
   - dungeon cells or room nodes
   - feature instances such as lairs, ruins, landmarks, exits, and searchable
     nodes

3. Optional later runtime entities
   - dropped loot and containers
   - encounter groups and hazards
   - faction patrols, expeditions, and other ambient world presences

The initial ECS pilot should start with actor runtime state in combat and
exploration, then expand to spatial cells and feature instances only after the
actor slice proves its value.

## Non-Goals

The following are explicitly out of scope for ECS unless a later ADR changes the
decision:

- admin CRUD models and services
- editor CRUD models and services
- repository storage and database row definitions
- SQLite schema ownership
- API request and response schemas
- websocket transport envelopes
- event-log storage
- static YAML content documents
- replacing the existing repository layer with ECS storage
- replacing Pydantic domain models with ECS-only models

## Architectural Boundaries

The boundaries are:

1. Persistence remains repository-driven.
   Repositories and adapters continue to own reads and writes to SQLite.

2. ECS remains runtime-only.
   ECS entities are materialized from repository/domain models when a runtime
   simulation needs them, then reduced back into typed write operations or
   domain events.

3. Pydantic remains the structured model layer.
   Components should be typed Pydantic models. ECS does not replace the existing
   requirement that structured data be represented as Pydantic models.

4. The event architecture remains the integration seam.
   ECS systems should emit typed gameplay events or structured outcome objects,
   not directly mutate transport/UI layers.

## Consequences

Positive:

- cleaner separation between persistent state and runtime simulation state
- easier extension of actor, cell, encounter, and hazard behavior
- less scene-specific branching for orthogonal runtime behavior
- stronger fit for temporary effects, AI intent, and trigger-driven mechanics

Costs:

- another runtime abstraction to maintain
- adapter code between repositories/domain models and ECS entities
- migration complexity while scenes still support legacy runtime paths
- need for clear rules so ECS does not sprawl into unrelated layers

## Rollout Guidance

The rollout should be staged:

1. define entity families, components, and system protocol
2. implement a minimal in-memory ECS runtime package
3. pilot actor ECS in combat and exploration
4. expand to spatial cells and feature instances if the pilot reduces complexity
5. evaluate loot, hazards, and faction-presence entities later

## Revisit Triggers

This ADR should be revisited only if one of these becomes true:

- ECS starts being pushed toward persistence or API modeling
- the actor pilot fails to reduce runtime complexity
- a different runtime architecture proves simpler for scene simulation
- later mechanics demand a broader or narrower ECS scope than defined here
