# Persistence Migration Matrix

This document turns the JSON persistence inventory into a migration plan. For
each JSON-backed surface, it records:

- current readers and writers
- authoritative fields
- target relational columns/tables
- migration priority
- whether dual-write is required
- whether backfill is required

This is the planning artifact that follows:

- [docs/persistence_adr.md](/home/cyrex/Projects/harsh_realm/docs/persistence_adr.md)
- [docs/persistence_inventory.md](/home/cyrex/Projects/harsh_realm/docs/persistence_inventory.md)

## Priority Scale

- `P0`: highest-value gameplay migration; start here
- `P1`: important authoritative state; follow the first slice
- `P2`: structured state that should be normalized after core gameplay flows
- `P3`: low-risk cleanup or review case
- `JSON`: intentionally remains JSON

## Matrix

| Surface | Current readers / writers | Authoritative fields today | Proposed target columns / tables | Priority | Dual-write | Backfill |
| --- | --- | --- | --- | --- | --- | --- |
| `entities.data` | Readers: `gm/entity_repository.py`, `api/editor/characters.py`, social/town/exploration flows. Writers: same plus `generators/settlement_gen.py`. | Character attributes, skills, HP/max HP, saves, attack bonus, inventory/equipment, class abilities, position mirrors, NPC occupation/personality/disposition/building links. | Keep `entities` as identity root. Add `character_stats`, `character_attributes`, `character_skills`, `character_inventory`, `character_abilities`, `npc_state`, `npc_personality`, and typed relationship tables for building/shop links if still needed. | P0 | Yes | Yes |
| `cells.data` | Readers: `gm/cell_repository.py`, `engine/discovery_repository.py`, exploration interaction, narrator/town reads. Writers: same plus world/settlement generation and respawn flows. | Settlement payloads, search timestamps, loot/death markers, feature instance state, miscellaneous exploration state. | Keep `cells` for scalar fields. Add `cell_settlements`, `cell_discoveries`, `cell_loot`, `cell_markers`, and `cell_feature_state` or equivalent typed child tables. | P0 | Yes | Yes |
| `factions.data` | Readers/writers: `faction/repository.py`, `api/editor/factions.py`. | Extra faction state not already in scalar columns. | Keep `factions` root. Add typed child tables such as `faction_state`, `faction_traits`, or more specific tables once concrete fields are enumerated. | P1 | Yes | Yes |
| `faction_assets.data` | Readers: `faction/repository.py`, `faction/faction_ai.py`, `api/editor/factions.py`. Writers: `faction/repository.py`, `faction/faction_turn.py`. | Asset expansion flags and asset-specific runtime state. | Keep `faction_assets` root. Add `faction_asset_state` and any specialized asset-detail child tables required by asset behavior. | P1 | Yes | Yes |
| `faction_relations.history` | No meaningful active readers/writers found. | Historical relation entries between factions. | Add `faction_relation_history` with one row per historical change/event. | P2 | No | Optional yes |
| `dungeons.rooms` | Readers: `api/editor/dungeons.py`, `gm/scenes/dungeon.py`, exploration-to-dungeon transition. Writers: same plus generators. | Room identity, type, description, editor position, loot/search state, encounter payload, room-local data. | Add `dungeon_rooms`, `dungeon_room_loot`, `dungeon_room_encounters`, and possibly `dungeon_room_state` for overflow fields. | P1 | Yes | Yes |
| `dungeons.connections` | Readers: `api/editor/dungeons.py`, `gm/scenes/dungeon.py`. Writers: same plus generators. | Room graph edges and direction metadata. | Add `dungeon_connections` with foreign keys to `dungeon_rooms`. | P1 | Yes | Yes |
| `dungeons.data` | Readers/writers: `api/editor/dungeons.py`. | Extra dungeon/editor state not modeled elsewhere. | Add `dungeon_state` or dedicated columns/tables for any field that becomes authoritative. Leave purely editor-only overflow deferred until concrete usage is known. | P2 | Yes | Yes |
| `threads.data` | No active readers; legacy writers previously initialized empty JSON in `engine/oracle_repository.py` and `api/editor/oracle.py`. | No meaningful authoritative fields in current runtime. | Leave column unused for compatibility; do not treat as active persistence. | P3 | No | No |
| `plotlines.data` | No active readers; legacy writer previously initialized empty JSON in `engine/oracle_repository.py`. | No meaningful authoritative fields in current runtime. | Leave column unused for compatibility; do not treat as active persistence. | P3 | No | No |
| `plotlines.scenes` | Readers/writers: `engine/oracle_repository.py`. | Ordered list of `AdventureScene` records. | `plotline_scenes` child table with explicit ordering and scene payloads. | P2 | Done | N/A |
| `items.data` | Readers/writers: `admin/content_mixin.py` through admin/editor CRUD and seeding. | Canonical item document persisted into world DB. | Keep JSON and document as editor-content storage unless runtime querying requirements change. | P3 | No | No |
| `creature_templates.data` | Readers/writers: `admin/content_mixin.py` through admin/editor CRUD and seeding. | Canonical creature document persisted into world DB. | Keep JSON and document as editor-content storage unless runtime querying requirements change. | P3 | No | No |
| `event_log.data` | Writer: `events.py` via `EventLogger`. Reader side is for debugging, replay, and inspection. | Event payload snapshot, not canonical aggregate state. | No schema migration planned. Remains JSON by ADR. | JSON | No | No |

## Notes by Surface

### `entities.data`

This is the first slice because it is both heavily mutated and widely consumed.
It also blocks cleanup in social, shopping, healing, combat, and editor flows.

The migration should preserve `entities` as the identity table and move the
payload into typed aggregate tables around it rather than replacing `entities`
entirely.

### `cells.data`

This is the second slice because it is the main world-state overflow bucket.
Normalizing it will also reduce ad hoc dict mutation in exploration, respawn,
discovery, and settlement logic.

### `factions.data` and `faction_assets.data`

These should move together. Asset-side state is already coupled to faction turn
logic, so migrating them separately would create extra compatibility work with
little benefit.

### `dungeons.rooms` and `dungeons.connections`

These are clearly structured graph data and are strong candidates for direct
relational normalization. They are less urgent than entities/cells only because
the dungeon system is less central to current runtime coverage.

### `threads.data` and `plotlines.data`

These are placeholder columns today. They are no longer written by active
runtime/editor code and should not be treated as gameplay persistence surfaces
unless real metadata is introduced later.

### `items.data` and `creature_templates.data`

These are editor/content documents rather than gameplay aggregate state. They
remain JSON by design for now and are out of the gameplay normalization wave
unless runtime querying needs become concrete enough to justify relational
projection.

### `event_log.data`

This is intentionally excluded from relational normalization. The right way to
improve it is payload typing and event taxonomy discipline, not schema
explosion.

## Recommended Execution Order

1. `entities.data`
2. `cells.data`
3. `factions.data` + `faction_assets.data`
4. `dungeons.rooms` + `dungeons.connections` + `dungeons.data`
5. `plotlines.scenes`
6. cleanup review of `threads.data`, `plotlines.data`, `items.data`,
   `creature_templates.data`

## Readiness for Implementation

This matrix is intentionally concrete enough to start implementation planning
for the first slice:

- `entities.data`

The next planning artifact should define the proposed schema and migration steps
for that slice in file-by-file terms.
