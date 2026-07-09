# Persistence Inventory

This document inventories the current JSON-backed persistence surfaces in the
world SQLite schema. It is the concrete follow-up to
[docs/persistence_adr.md](/home/cyrex/Projects/harsh_realm/docs/persistence_adr.md).

The goal here is not to define the final schema yet. The goal is to make the
current state explicit: where each JSON-backed column lives, what shape it
stores, which code reads it, which code writes it, and whether it is expected
to remain JSON or be normalized later.

## Summary

| Surface | Table / Column | Current shape | Current role | ADR classification |
| --- | --- | --- | --- | --- |
| Entity payload | `entities.data` | JSON object | Character/NPC runtime state | Temporary compatibility shim |
| Cell payload | `cells.data` | JSON object | Settlement, discovery, loot, markers, misc world state | Temporary compatibility shim |
| Faction payload | `factions.data` | JSON object | Extra faction runtime/editor state | Temporary compatibility shim |
| Faction asset payload | `faction_assets.data` | JSON object | Extra asset runtime/editor state | Temporary compatibility shim |
| Relation history | `faction_relations.history` | JSON array | Faction relationship history log | Temporary compatibility shim |
| Dungeon rooms | `dungeons.rooms` | JSON array | Dungeon room graph nodes | Temporary compatibility shim |
| Dungeon connections | `dungeons.connections` | JSON array | Dungeon room graph edges | Temporary compatibility shim |
| Dungeon payload | `dungeons.data` | JSON object | Extra dungeon/editor state | Temporary compatibility shim |
| Thread payload | `threads.data` | JSON object | Reserved thread-side metadata | Temporary compatibility shim |
| Plotline payload | `plotlines.data` | JSON object | Reserved plotline-side metadata | Temporary compatibility shim |
| Plotline scenes | `plotlines.scenes` | JSON array | Ordered adventure scene list | Temporary compatibility shim |
| Item payload | `items.data` | JSON object | Canonical item document in world DB | Review case; may remain JSON if editor-content only |
| Creature template payload | `creature_templates.data` | JSON object | Canonical creature document in world DB | Review case; may remain JSON if editor-content only |
| Event payload | `event_log.data` | JSON object | Logged event payload | Intentionally JSON |

## Detailed Inventory

### `entities.data`

- Table / column:
  `entities.data`
- Shape:
  JSON object
- Primary stored fields today:
  character data, NPC data, inventory/equipment, class abilities, attr mods,
  social disposition, position mirrors, generated NPC personality, misc runtime
  state
- Main readers:
  [src/harsh_realm/gm/entity_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/entity_repository.py),
  [src/harsh_realm/api/editor/characters.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/characters.py),
  social/town/exploration scene flows via the entity repository
- Main writers:
  [src/harsh_realm/gm/entity_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/entity_repository.py),
  [src/harsh_realm/api/editor/characters.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/characters.py),
  settlement generation in
  [src/harsh_realm/generators/settlement_gen.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/generators/settlement_gen.py)
- Notes:
  This is the largest gameplay-authoritative JSON surface in the system. It is
  the first migration target.
- ADR classification:
  Temporary compatibility shim

### `cells.data`

- Table / column:
  `cells.data`
- Shape:
  JSON object
- Primary stored fields today:
  settlement payloads, discovery/search timestamps, loot markers, death markers,
  generated feature metadata, misc exploration state
- Main readers:
  [src/harsh_realm/gm/cell_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/cell_repository.py),
  [src/harsh_realm/engine/discovery_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/engine/discovery_repository.py),
  exploration interaction/persistence flows, narrator/town settlement reads
- Main writers:
  [src/harsh_realm/gm/cell_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/cell_repository.py),
  [src/harsh_realm/engine/discovery_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/engine/discovery_repository.py),
  world/settlement generation,
  respawn/death marker flows
- Notes:
  This is the second major gameplay-authoritative JSON surface and should follow
  `entities.data` in migration order.
- ADR classification:
  Temporary compatibility shim

### `factions.data`

- Table / column:
  `factions.data`
- Shape:
  JSON object
- Primary stored fields today:
  extra faction state not represented by scalar columns
- Main readers:
  [src/harsh_realm/faction/repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/faction/repository.py),
  [src/harsh_realm/api/editor/factions.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/factions.py)
- Main writers:
  [src/harsh_realm/faction/repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/faction/repository.py),
  [src/harsh_realm/api/editor/factions.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/factions.py)
- Notes:
  The column exists as an overflow bucket for faction state. It should be
  normalized together with goals/tags and asset-side state.
- ADR classification:
  Temporary compatibility shim

### `faction_assets.data`

- Table / column:
  `faction_assets.data`
- Shape:
  JSON object
- Primary stored fields today:
  asset expansion flags and asset-specific overflow data
- Main readers:
  [src/harsh_realm/faction/repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/faction/repository.py),
  [src/harsh_realm/faction/faction_ai.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/faction/faction_ai.py),
  [src/harsh_realm/api/editor/factions.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/factions.py)
- Main writers:
  [src/harsh_realm/faction/repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/faction/repository.py),
  faction turn logic in
  [src/harsh_realm/faction/faction_turn.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/faction/faction_turn.py)
- Notes:
  This should migrate with faction persistence rather than independently.
- ADR classification:
  Temporary compatibility shim

### `faction_relations.history`

- Table / column:
  `faction_relations.history`
- Shape:
  JSON array
- Primary stored fields today:
  historical relation entries
- Main readers:
  No significant active readers found in current gameplay paths
- Main writers:
  No significant active writers found in current gameplay paths
- Notes:
  The column exists in schema but appears underused in the current runtime. It
  still belongs in the inventory because it is JSON-backed persistence and a
  likely future normalization target.
- ADR classification:
  Temporary compatibility shim

### `dungeons.rooms`

- Table / column:
  `dungeons.rooms`
- Shape:
  JSON array of room records
- Primary stored fields today:
  room id, name, type, description, editor position, loot/search info, room data
- Main readers:
  [src/harsh_realm/api/editor/dungeons.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/dungeons.py),
  [src/harsh_realm/gm/scenes/dungeon.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/scenes/dungeon.py),
  exploration-to-dungeon transition logic
- Main writers:
  [src/harsh_realm/api/editor/dungeons.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/dungeons.py),
  dungeon generation flows
- Notes:
  This is authoritative structural gameplay state and should become a child
  table.
- ADR classification:
  Temporary compatibility shim

### `dungeons.connections`

- Table / column:
  `dungeons.connections`
- Shape:
  JSON array of connection records
- Primary stored fields today:
  from/to room ids, direction, edge metadata
- Main readers:
  [src/harsh_realm/api/editor/dungeons.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/dungeons.py),
  [src/harsh_realm/gm/scenes/dungeon.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/gm/scenes/dungeon.py)
- Main writers:
  [src/harsh_realm/api/editor/dungeons.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/dungeons.py),
  dungeon generation flows
- Notes:
  This is authoritative dungeon graph data and should become a child table.
- ADR classification:
  Temporary compatibility shim

### `dungeons.data`

- Table / column:
  `dungeons.data`
- Shape:
  JSON object
- Primary stored fields today:
  editor/runtime overflow data
- Main readers:
  [src/harsh_realm/api/editor/dungeons.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/dungeons.py)
- Main writers:
  [src/harsh_realm/api/editor/dungeons.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/dungeons.py)
- Notes:
  Lower priority than rooms/connections, but still part of the dungeon
  normalization slice.
- ADR classification:
  Temporary compatibility shim

### `threads.data`

- Table / column:
  `threads.data`
- Shape:
  JSON object
- Primary stored fields today:
  effectively unused placeholder metadata; core thread state lives in scalar
  columns
- Main readers:
  No active gameplay/editor readers found
- Main writers:
  [src/harsh_realm/engine/oracle_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/engine/oracle_repository.py),
  [src/harsh_realm/api/editor/oracle.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/api/editor/oracle.py)
  both initialize it to `{}`
- Notes:
  This is a low-risk cleanup candidate because the column appears reserved
  rather than meaningfully used.
- ADR classification:
  Temporary compatibility shim

### `plotlines.data`

- Table / column:
  `plotlines.data`
- Shape:
  JSON object
- Primary stored fields today:
  effectively unused placeholder metadata; plotline state currently lives in
  scalar columns plus `scenes`
- Main readers:
  No active gameplay/editor readers found
- Main writers:
  [src/harsh_realm/engine/oracle_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/engine/oracle_repository.py)
  initializes it to `{}`
- Notes:
  This appears reserved like `threads.data`.
- ADR classification:
  Temporary compatibility shim

### `plotlines.scenes`

- Table / column:
  `plotlines.scenes`
- Shape:
  JSON array of `AdventureScene` records
- Primary stored fields today:
  ordered plotline scene list
- Main readers:
  [src/harsh_realm/engine/oracle_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/engine/oracle_repository.py)
- Main writers:
  [src/harsh_realm/engine/oracle_repository.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/engine/oracle_repository.py)
- Notes:
  This is structured runtime state and should be reviewed alongside plotline
  persistence if plotlines become a richer gameplay system.
- ADR classification:
  Temporary compatibility shim

### `items.data`

- Table / column:
  `items.data`
- Shape:
  JSON object
- Primary stored fields today:
  canonical item document loaded from YAML and stored in world DB
- Main readers:
  [src/harsh_realm/admin/content_mixin.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/admin/content_mixin.py),
  admin/editor item CRUD, item seeding
- Main writers:
  [src/harsh_realm/admin/content_mixin.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/admin/content_mixin.py)
- Notes:
  This is not the same as per-character inventory. It is closer to editor-owned
  content. It may be acceptable to leave as JSON if it remains document-shaped
  content rather than mutable gameplay state.
- ADR classification:
  Review case; may remain JSON if editor-content only

### `creature_templates.data`

- Table / column:
  `creature_templates.data`
- Shape:
  JSON object
- Primary stored fields today:
  canonical creature document loaded from YAML and stored in world DB
- Main readers:
  [src/harsh_realm/admin/content_mixin.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/admin/content_mixin.py),
  admin/editor creature CRUD, creature seeding
- Main writers:
  [src/harsh_realm/admin/content_mixin.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/admin/content_mixin.py)
- Notes:
  Like `items.data`, this is content-table storage rather than an obvious
  gameplay aggregate. It should be evaluated separately from entity/cell/faction
  normalization.
- ADR classification:
  Review case; may remain JSON if editor-content only

### `event_log.data`

- Table / column:
  `event_log.data`
- Shape:
  JSON object
- Primary stored fields today:
  serialized `GameEvent.data` payloads
- Main readers:
  event-log consumers, debugging, integration checks, future replay/debug tools
- Main writers:
  [src/harsh_realm/events.py](/home/cyrex/Projects/harsh_realm/src/harsh_realm/events.py)
  via `EventLogger._write()`
- Notes:
  This is transport/debug data, not canonical aggregate state.
- ADR classification:
  Intentionally JSON

## Immediate Follow-up

The inventory confirms the migration order already proposed in the ADR:

1. `entities.data`
2. `cells.data`
3. `factions.data` / `faction_assets.data` / `faction_relations.history`
4. `dungeons.rooms` / `dungeons.connections` / `dungeons.data`
5. `threads.data`, `plotlines.data`, `plotlines.scenes`, `items.data`,
   `creature_templates.data`

The next document should be a migration matrix that adds target tables/columns,
reader/writer ownership, and dual-write/backfill requirements for each surface.
