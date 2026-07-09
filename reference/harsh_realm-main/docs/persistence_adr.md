# Persistence ADR

## Status

Accepted for implementation planning.

## Context

Harsh Realm currently uses per-world SQLite databases as the authoritative
runtime store. The schema already has a useful relational core, but several
important gameplay aggregates are still persisted as opaque JSON blobs inside
`TEXT` columns.

This shows up most clearly in:

- `entities.data`
- `cells.data`
- `factions.data`
- `faction_assets.data`
- `dungeons.rooms`
- `dungeons.connections`
- `dungeons.data`
- `threads.data`
- `plotlines.data`
- `items.data`
- `creature_templates.data`

There are also list-like JSON columns already mixed into otherwise relational
tables:

- `factions.goals`
- `factions.tags`
- `faction_relations.history`
- `random_tables.entries`
- `random_tables.tags`
- `plotlines.scenes`

The result is that too much authoritative state is hidden behind generic
`json.dumps()` / `json.loads()` code paths. That makes migrations harder,
prevents meaningful database constraints, weakens discoverability, and causes
repository code to revolve around raw dict payloads instead of typed aggregates.

At the same time, not all JSON usage is wrong. Some data is naturally
document-shaped or transport-shaped and does not benefit from immediate
relational normalization.

This ADR defines what must become relational, what may remain JSON, and which
legacy JSON columns are only compatibility shims during migration.

## Decision

### 1. Gameplay-authoritative state must be relational

Any state that directly affects gameplay, progression, map state, combat, NPC
behavior, faction turns, or repeatable world simulation must not live
authoritatively in opaque JSON blobs.

This includes:

- character state
- NPC state
- inventory and equipment state
- authoritative cell/world state
- faction state and faction asset state
- dungeon structure
- any state that must support validation, constraints, querying, or migration

The long-term target is that repositories load and save typed aggregates backed
by explicit columns and child tables rather than raw JSON payloads.

### 2. JSON remains allowed in narrow, explicit cases

JSON may remain in SQLite only when one of these is true:

- the data is an event payload or transport snapshot rather than canonical
  runtime state
- the data is editor-oriented, schema-flexible content where relational
  modeling would add more complexity than value
- the data is a temporary compatibility shim during an additive migration

Allowed JSON categories:

- `event_log.data`
- temporary compatibility payloads during staged migrations
- narrowly scoped editor/config documents that are intentionally document-like

### 3. `event_log.data` is intentionally JSON

`event_log.data` is not in scope for relational normalization. It stores event
payloads, not canonical aggregate state. Those payloads are transport- and
debug-oriented and are expected to vary by event type.

We may tighten payload typing at the model layer, but the database column
remains JSON.

This decision should only be revisited if concrete replay, audit, or debugging
requirements prove that selected event fields need relational indexing or
structured query support beyond the current event taxonomy metadata.

### 4. Legacy JSON columns are compatibility shims, not permanent targets

The following columns are considered temporary compatibility shims until their
target relational migrations are completed:

- `entities.data`
- `cells.data`
- `factions.data`
- `faction_assets.data`
- `dungeons.rooms`
- `dungeons.connections`
- `dungeons.data`
- `threads.data`
- `plotlines.data`
- `items.data`
- `creature_templates.data`

The following JSON-list fields should also be treated as migration targets, not
final design:

- `factions.goals`
- `factions.tags`
- `faction_relations.history`
- `plotlines.scenes`

`random_tables.entries` and `random_tables.tags` are a special case. They are
content-table structures rather than mutable gameplay entity state, so they may
remain JSON unless a concrete need for relational querying or constraints
emerges.

## Classification Rules

### Must be relational

Use relational tables/columns when the data:

- is authoritative gameplay state
- is updated repeatedly during play
- needs partial updates or joins
- needs constraints or referential integrity
- needs stable migrations across world versions
- is shared across multiple runtime flows

### May remain JSON

Use JSON when the data:

- is event payload data
- is a document-shaped editor/config object with low query needs
- is intentionally polymorphic and not gameplay-authoritative
- is temporary dual-write/backfill compatibility state

### Temporary compatibility only

If a JSON field currently stores authoritative state but does so only because
the relational migration is incomplete, it must be marked as a compatibility
shim and scheduled for removal from active gameplay reads.

## Migration Rules

All persistence normalization work follows this sequence:

1. Add new columns/tables without removing old JSON columns.
2. Introduce typed repositories or typed read/write methods for the target
   aggregate.
3. Dual-write new relational state and old JSON state for a temporary period.
4. Backfill existing worlds from legacy JSON payloads.
5. Switch reads to the typed relational source of truth.
6. Remove legacy JSON reads from gameplay paths.
7. Drop or deprecate old JSON columns only after compatibility requirements are
   satisfied.

No big-bang destructive migration is allowed for active gameplay data.

### Migration Standard

Each persistence refactor should satisfy the following implementation rules:

- Schema additions must be additive and forward-compatible.
- New relational writes must be introduced before old JSON writes are removed.
- Dual-write must remain in place until both new worlds and migrated legacy
  worlds can be read from the relational source without loss of behavior.
- Backfill must be explicit and testable; do not rely on opportunistic lazy
  migration during random gameplay reads.
- Read cutover should happen behind repository boundaries so callers do not need
  to know whether a world is pre- or post-migration.
- Legacy JSON reads must be removed from gameplay-authoritative paths once the
  relational source is proven correct.
- Old JSON columns should be treated as deprecated after read cutover, and only
  dropped in a later cleanup step after compatibility needs are satisfied.

### Required Verification Per Migration Slice

Before a migration slice is considered complete, it should have:

- a schema migration or bootstrap path for new worlds
- a backfill path for existing worlds
- repository tests for typed read/write behavior
- migration tests proving legacy JSON worlds upgrade correctly
- integration coverage for the gameplay/editor flows that depend on the slice

## Initial Migration Order

The recommended order is:

1. `entities.data`
2. `cells.data`
3. `factions.data` / `faction_assets.data` / relation history
4. `dungeons.rooms` / `dungeons.connections` / `dungeons.data`
5. remaining editor/runtime document blobs such as threads, plotlines, items,
   and creature templates

This order prioritizes the most frequently mutated gameplay state first.

## SQLModel Position

`SQLModel` was not used as the first step of this migration.

The first step was deciding the target relational shape and migrating active
gameplay state into typed tables behind repositories. That work is now stable
enough that a narrow `SQLModel` spike was completed and documented in
[docs/sqlmodel_spike.md](/home/cyrex/Projects/harsh_realm/docs/sqlmodel_spike.md).

Decision:

- `SQLModel` is rejected for the current persistence architecture
- repository boundaries remain the preferred integration surface
- Pydantic remains the application model layer
- new persistence work should continue with explicit schema SQL plus repository
  mapping

The decision may be revisited only if the project later adopts SQLAlchemy for a
stronger reason than row-class convenience.

## Consequences

Positive:

- stronger typing at the persistence boundary
- better constraints and queryability
- cleaner repository APIs
- safer migrations for existing worlds
- less gameplay logic built around raw dict payloads

Costs:

- additive migration complexity
- temporary dual-write behavior
- more schema and repository code during the transition
- possible follow-up work if SQLModel is later adopted

## Out of Scope

This ADR does not define the exact schema for each aggregate. That is handled by
the migration matrix and implementation plans for each vertical slice.
