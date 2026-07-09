# Modular Rules Architecture — Phase 0 Codebase Audit

**Date:** 2026-04-26
**Phase:** Modular Rules Architecture / Phase 0 Foundation
**Task:** 0.1 — Codebase audit and pack-target inventory
**Status:** Complete

This audit inventories current game content, hardcoded `data/` access, world
creation/load seams, and admin/editor read paths before implementing the pack
system. It is a working document for Phase 0 tasks, not user-facing product
documentation.

## 1. Data File Classification

Current `data/` has 91 YAML files. Most files are game content and should move
to `packs/xwn-core/content/`. Editor schema files are tooling metadata and
should not be treated as game records, although they may need to become
pack-aware because they describe pack-editable content.

### XWN Core Content

These are authored game records, rules defaults, random tables, generators, or
narration templates. They should move into `packs/xwn-core/content/` during
Task 0.18.

| Current path/group | Shape | Pack target |
| --- | --- | --- |
| `data/classes.yaml` | `classes` document | `content/classes.yaml` |
| `data/skills.yaml` | `skills` document | `content/skills.yaml` |
| `data/skill_mappings.yaml` | list of skill mapping records | `content/admin_defaults/skill_mappings.yaml` or `content/skill_mappings.yaml` |
| `data/difficulty_targets.yaml` | list of target records | `content/admin_defaults/difficulty_targets.yaml` |
| `data/disposition_outcomes.yaml` | list of social disposition results | `content/admin_defaults/disposition_outcomes.yaml` |
| `data/encounter_weights.yaml` | list of encounter-weight modifiers | `content/admin_defaults/encounter_weights.yaml` |
| `data/faction_assets.yaml` | list of faction asset stat records | `content/faction_assets.yaml` |
| `data/factions/starting_factions.yaml` | starting factions and relations | `content/factions/starting_factions.yaml` |
| `data/equipment_kits.yaml` | `kits` document | `content/equipment_kits.yaml` |
| `data/items/*.yaml` | item catalogs | `content/items/*.yaml` |
| `data/creatures/*.yaml` | creature catalogs | `content/creatures/*.yaml` |
| `data/shops/*.yaml` | shop tier inventories | `content/shops/*.yaml` |
| `data/terrain.yaml` | hex terrain registry | `content/terrain.yaml` |
| `data/terrain_square.yaml` | square terrain registry | `content/terrain_square.yaml` |
| `data/templates/*.yaml` | narration/template content | `content/templates/*.yaml` |
| `data/generators/npc_basic.yaml` | generator definition | `content/generators/npc_basic.yaml` |
| `data/tables/discoveries/*.yaml` | random tables | `content/tables/discoveries/*.yaml` |
| `data/tables/encounters/*.yaml` | random tables | `content/tables/encounters/*.yaml` |
| `data/tables/factions/faction_actions.yaml` | faction action table/config | `content/tables/factions/faction_actions.yaml` |
| `data/tables/loot/*.yaml` | loot tables | `content/tables/loot/*.yaml` |
| `data/tables/names/*.yaml` | name lists | `content/tables/names/*.yaml` |
| `data/tables/npc/une_*.yaml` | UNE personality tables | `content/tables/npc/*.yaml` |
| `data/tables/npcs/*.yaml` | NPC tables | `content/tables/npcs/*.yaml` |
| `data/tables/oracle/*.yaml` | Mythic/Adventure Crafter oracle tables | `content/tables/oracle/*.yaml` |
| `data/tables/settlements/*.yaml` | settlement tables | `content/tables/settlements/*.yaml` |
| `data/tables/terrain/*.yaml` | terrain generation content | `content/tables/terrain/*.yaml` |

### Engine Or Editor Tooling Metadata

These files describe editor widgets or validation references. They should stay
outside the content-record layer unless Task 0.18 deliberately makes editor
schemas pack-provided. Recommended default: move them to an engine/tooling path
such as `src/harsh_realm/schemas/` or `frontend`/admin assets, then make the
editor schema API read from that new tooling root.

| Current path/group | Reason |
| --- | --- |
| `data/schemas/editors/*.yaml` | Admin/editor form schemas, not game-world records. |
| `data/schemas/table_schema.yaml` | Documentation/reference schema for table YAML; parsed as `None` because it is comment-only. |

### Unclear Or Needs Review

No files are truly unknown, but two groups deserve explicit design decisions:

- `data/schemas/editors/*.yaml`: if pack authors can ship custom editor schemas,
  these may become pack metadata later. For Phase 0, keep engine/editor schemas
  separate from `xwn-core` content to avoid coupling pack records to the current
  admin UI implementation.
- `data/templates/*.yaml`: these are authored game text, not engine config. They
  should move into `xwn-core`, but callers currently expect a filesystem
  directory. `Narrator` and combat narration need a content-service-friendly
  adapter.

## 2. YAML References And Load Sites

The codebase still has many direct `data/` assumptions. These are the primary
targets for Tasks 0.16 and 0.18.

### Central Data Directory Helpers

| Module | Current behavior | Phase 0 action |
| --- | --- | --- |
| `admin/data_access.py` | `find_data_dir()` and `load_yaml(rel_path)` read repo `data/` | Replace admin defaults with pack/content service reads. |
| `api/editor/common.py` | `find_data_dir()` and traversal guard for YAML editor | Split editor tooling root from pack content root; update YAML editor semantics. |
| `api/menu_handler.py` | `_find_data_dir()` for text-mode world creation | Thread pack list and pack content through menu world creation. |
| `gm/gm_factory.py` | `_find_data_dir()` loads terrain and templates for controller | Load terrain/templates from active content service or pack registry. |
| `generators/world_gen.py` | `_find_data_dir()` loads terrain weights and names | Route generation tables/name lists through pack content. |

### Runtime Content Loaders

| Module | Current direct content reads | Notes |
| --- | --- | --- |
| `models/cell.py` | `TerrainRegistry.load(data/terrain.yaml)` and `load_all(data/)` | Needs terrain content adapter. |
| `engine/item_registry.py` | loads all `data/items/*.yaml` | Should load item records from `xwn-core` through `ContentService`. |
| `models/creature.py` | `CreatureRegistry.load(Path("data"))`; loads `data/creatures/*.yaml` | Also used by exploration encounters. |
| `engine/tables.py` | `load_tables(data_dir)` scans `data/tables`; `generate()` reads `data/generators` | Pack table loading should iterate loaded packs in order. |
| `engine/shop_inventory.py` | loads `data/shops/<building>.yaml` | Shop inventory should be pack content. |
| `engine/loot.py` | loads `data/tables/loot/*.yaml` | Loot tables are pack content; current format is not the same as `random_tables`. |
| `engine/npc_personality.py` | UNE tables under `data/tables/npc/` | Phase 1 will migrate to procedures; Phase 0 can keep via content reads. |
| `engine/oracle.py` | fate/event tables under `data/tables/oracle/` | Pack content; may remain Python logic for now. |
| `engine/adventure_crafter.py` | Adventure Crafter tables under `data/tables/oracle/` | Pack content; cached module globals need invalidation/pack awareness. |
| `engine/combat.py` | `data/templates/combat_narration.yaml` | Template content; current module-level path. |
| `gm/narrator.py` | templates directory with terrain/movement/adjacent hints YAML | Needs adapter or content-backed template loader. |
| `gm/scenes/character_creation_support.py` | `skills.yaml`, `equipment_kits.yaml` | Character creation should read pack records. |
| `gm/scenes/exploration_combat.py` | `CreatureRegistry.load(_DATA_DIR)`, `TableEngine.load_tables(_DATA_DIR)` | Important gameplay smoke-test target. |
| `gm/scenes/shopping.py` | item registry and shop inventories from `data/` | Shopping is a high-risk Task 0.16 target because it mixes items, gold, and scenes. |

### World Creation And Seeding Reads

| Module | Current direct content reads | Notes |
| --- | --- | --- |
| `api/routes.py:create_world` | resolves `data/`, loads terrain, passes `data_dir` to `WorldGenerator`, then calls `AdminService.seed_all_from_yaml()` | Main seam for adding `pack_ids` and initial pack registry. |
| `admin/service.py` | loads skill mappings, difficulties, disposition, encounter weights, faction assets from YAML | These are currently world-scoped SQLite defaults seeded from YAML. |
| `admin/content_mixin.py` | seeds `random_tables`, `items`, `creature_templates`; reset paths re-read YAML | Needs override-aware pack/default semantics. |
| `generators/world_gen.py` | loads terrain weights, settlement/ruin/landmark names; loads random tables for settlement enhancement | Important: world generation needs pack access before the world is fully loaded. |

### Tests With Direct Data Assumptions

Many tests use `Path(__file__).parent.parent / "data"` or literal
`"data/..."`. These should be adjusted after pack content lands, not before.
Representative files:

- `tests/test_cell.py`, `tests/test_world_gen.py`, `tests/test_world_map_api.py`
- `tests/test_item_registry.py`, `tests/test_shop_inventory.py`, `tests/test_creatures.py`
- `tests/test_tables.py`, `tests/test_discovery.py`, `tests/test_encounters.py`
- `tests/test_admin_schema.py`, `tests/test_editor_routes.py`
- `tests/test_gm_controller.py`, `tests/test_npc_gen.py`, `tests/test_loot.py`

## 3. House Rules And Code-Bearing Pack Inputs

The Phase 0 spec mentions `src/harsh_realm/house_rules/practice_skills.py`, but
the current repository has no `src/harsh_realm/house_rules/` package and no
`practice_skills.py` file.

Audit command returned no matches for:

- `src/harsh_realm/**/house_rules*`
- `src/harsh_realm/**/practice_skills*`

Phase 0 Tasks 0.19 and 0.20 should therefore be revised slightly:

- Do not move a non-existent `practice_skills.py`.
- Still implement the code-bearing pack registration hook.
- Add a fixture/test pack with `code/__init__.py register(app_state)` to prove
  the mechanism.
- Leave a note in `xwn-core` that it currently has no code-bearing house-rule
  modules unless a later task creates one.

## 4. Hardcoded `data/` Paths To Remove

The following source files contain `data/` path assumptions or data-dir helper
logic and should be revisited before Task 0.18 can remove or empty root `data/`:

- `src/harsh_realm/admin/content_mixin.py`
- `src/harsh_realm/admin/data_access.py`
- `src/harsh_realm/admin/service.py`
- `src/harsh_realm/api/admin_routes.py`
- `src/harsh_realm/api/editor/common.py`
- `src/harsh_realm/api/editor/yaml_files.py`
- `src/harsh_realm/api/editor_routes.py`
- `src/harsh_realm/api/menu_handler.py`
- `src/harsh_realm/api/routes.py`
- `src/harsh_realm/engine/adventure_crafter.py`
- `src/harsh_realm/engine/combat.py`
- `src/harsh_realm/engine/item_registry.py`
- `src/harsh_realm/engine/loot.py`
- `src/harsh_realm/engine/npc_personality.py`
- `src/harsh_realm/engine/oracle.py`
- `src/harsh_realm/engine/shop_inventory.py`
- `src/harsh_realm/engine/tables.py`
- `src/harsh_realm/generators/world_gen.py`
- `src/harsh_realm/gm/controller.py`
- `src/harsh_realm/gm/gm_factory.py`
- `src/harsh_realm/gm/scenes/character_creation_support.py`
- `src/harsh_realm/gm/scenes/exploration_combat.py`
- `src/harsh_realm/gm/scenes/exploration_support.py`
- `src/harsh_realm/models/creature.py`
- `src/harsh_realm/models/item.py`

Recommended sequencing:

1. Introduce pack and content services without removing current `data/`.
2. Add compatibility adapters so existing registries can be backed by pack
   records.
3. Switch runtime call sites to the adapters one domain at a time.
4. Move physical files only after tests no longer require direct `data/` paths.

## 5. World Creation Flow

### Current Backend Flow

`POST /api/worlds` in `api/routes.py` currently:

1. Resolves `worlds_dir` from app config.
2. Creates a new SQLite database with `WorldDatabase.create(filepath, name)`.
3. Resolves a filesystem `data_dir`.
4. Loads `TerrainRegistry` from `data/terrain.yaml`.
5. Constructs `WorldGenerator(db, registry, data_dir=data_dir)`.
6. Generates cells with `generate_region(width, height, seed)`.
7. Calls `AdminService(db).seed_all_from_yaml()`.
8. Inserts initial `gm_state` scene `char_create`.
9. Closes the DB and returns `WorldCreateResult`.

### Pack Binding Seam

Task 0.13 should extend `CreateWorldRequest` in `models/public_api.py` with:

- `pack_ids: list[str] = Field(default_factory=lambda: ["xwn-core"])`

Then `create_world` should:

1. Discover/resolve packs before creating or mutating the world DB.
2. Fail with 400 before DB creation if pack resolution fails.
3. Create DB.
4. Persist `world_packs` via `WorldPackRepository`.
5. Create a temporary content service/registry for world generation and seeding.
6. Generate terrain and seed admin/config/content tables from packs.

### Frontend Flow

`WorldManager.vue` calls `worldStore.createWorld(name, width, height, seed)`.
`frontend/src/stores/world.ts` sends `{ name, width, height, seed? }` to
`POST /api/worlds`, then immediately calls `/api/worlds/load`.

Task 0.22 should add a pack picker and thread `pack_ids` through:

- `WorldManager.vue`
- `useWorldStore.createWorld(...)`
- frontend API types once `WorldCreateResult` includes pack metadata

## 6. World Load Flow

### Current Backend Flow

`POST /api/worlds/load` in `api/routes.py` currently:

1. Resolves the DB path from `state.config.worlds.directory`.
2. Clears old `state.gm_controller`.
3. Closes previous `state.active_world`, if any.
4. Clears event bus subscriptions.
5. Opens the selected `WorldDatabase`.
6. Stores it as `state.active_world`.
7. Reattaches WebSocket event broadcaster and `EventLogger`.
8. Creates GM controller with `make_gm_controller(db, state.event_bus)`.
9. Returns `{ name, file }`.

`make_gm_controller` currently loads `TerrainRegistry` and `Narrator` from
filesystem `data/`.

### Pack Reconstitution Seam

Task 0.14 should insert pack registry construction after opening the world and
before creating the GM controller:

1. Read `world_packs`.
2. Build `PackRegistry` from configured `packs_root`.
3. Validate installed pack versions against recorded versions.
4. Attach `state.pack_registry`.
5. Attach `state.content_service` or an equivalent content-service factory.
6. Pass content dependencies into `make_gm_controller` so it no longer resolves
   repo `data/` directly.

On unload/delete/failed load, clear `state.pack_registry` and any content
service/procedure/registry state associated with the old world.

## 7. Admin And Editor Read Paths

### World-Backed Admin Tables

`AdminService` and `AdminContentMixin` already centralize most config/content
CRUD. This is the right place to introduce pack-aware defaults and override
semantics.

Current world-backed admin/config surfaces:

- `skill_mappings`
- `difficulty_targets`
- `disposition_outcomes`
- `encounter_weights`
- `faction_asset_stats`
- `random_tables`
- `items`
- `creature_templates`

Current admin routes live in `api/admin_routes.py`:

- `/api/admin/skill-mappings[...]`
- `/api/admin/difficulty-targets[...]`
- `/api/admin/disposition-outcomes[...]`
- `/api/admin/encounter-weights[...]`
- `/api/admin/faction-assets[...]`
- `/api/admin/items-data[...]`
- `/api/admin/creature-templates[...]`

These are currently world-scoped SQLite CRUD operations, with reset operations
returning to root YAML defaults. Under Phase 0, reset should mean "delete or
replace the per-world override and fall back to pack default" where the record
has a pack-backed source. Existing SQLite tables may remain as world-scoped
materialized/config tables during Phase 0, but their seed/reset source should
be pack data rather than `data/`.

### YAML Editor Routes

`api/editor/yaml_files.py` currently exposes raw filesystem editing of
`data/`:

- list/read/write/delete arbitrary YAML under `data/`
- list editor schemas from `data/schemas/editors`
- table status for `data/tables`
- table zip download/upload

Phase 0 needs an explicit policy decision for this surface:

- Recommended Phase 0 default: keep YAML editor as an engine/developer tooling
  route, but repoint it away from root `data/` before `data/` is removed.
- Pack-aware editing should use override APIs (`/api/world/content/...`) rather
  than writing directly into `packs/xwn-core/content`.
- Bulk table zip import/export can remain as tooling for pack authoring, but it
  should not mutate installed read-only pack data for active worlds unless a
  future "pack development mode" is introduced.

### Editor World-State Routes

The split `api/editor/` modules for cells, characters/entities, factions,
dungeons, oracle state, and worlds operate on world SQLite state. They are not
pack content reads, except where they depend on editor schemas or item/creature
definitions. They should not be refactored during early Phase 0 unless a test
reveals a direct `data/` dependency.

## 8. Schema And Migration Note

The Phase 0 spec currently says to extend `db.py` / `_init_schema`, but the
current repository uses:

- `src/harsh_realm/db_schema.py` as the canonical `SCHEMA_SQL` and
  `REQUIRED_TABLES` source for new worlds.
- `src/harsh_realm/db.py::_migrate_schema()` for forward-compatible migrations
  of existing world DBs.

Tasks 0.11, 1.10, 2.12, and 3.11 should update both places as appropriate:

- Add new tables to `db_schema.py`.
- Add migration DDL in `db.py` for existing worlds.
- Add table names to `REQUIRED_TABLES` when they are required for a valid world.

## 9. Immediate Follow-Up For Phase 0

Recommended next implementation slice:

1. Task 0.2: add `harsh_realm.packs.manifest.PackManifest`.
2. Task 0.3: add version parsing/constraints.
3. Task 0.4: add directory pack loader.
4. Task 0.5/0.7: add registry and read API.

Do not move `data/` or touch world creation until the pack model, loader, and
registry have unit/property coverage.
