# Modular Rules Architecture — Phase 0: Foundation

**Date:** 2026-04-26
**Status:** Draft
**Cycle:** Modular Rules Architecture
**Overview:** `2026-04-26-modular-rules-architecture-overview.md`
**Prerequisites for tasks:** Read the overview spec first. Read `AGENTS.md` for coding standards.

---

## 1. Phase scope

Phase 0 establishes the pack system itself — the format, the loader, the world binding, the per-world override layer — and refactors all existing XWN content into `packs/xwn-core/`. After Phase 0, the engine has no hardcoded content; everything goes through the pack system.

This phase is foundational. Phases 1–3 depend on its completion. It does *not* add any new mechanic frameworks (those are Phases 1–3). It does not change game mechanics. The user-visible behavior after Phase 0 should be identical to before, except the world creation flow now includes pack selection.

### What Phase 0 produces

- A defined pack format with manifest, content layout, and code layout conventions.
- A pack loader that reads packs from disk, validates manifests, resolves dependencies, detects conflicts, and presents a unified read API for content.
- World-pack binding: worlds record their pack list at creation; world load reconstitutes pack data correctly.
- A per-world override layer: edits in admin create overrides, "revert to default" removes them.
- `packs/xwn-core/` containing all current XWN content. The previous `data/` directory at repo root is removed (or empty of game content).
- Admin UI shows pack-vs-override state and offers revert.
- Migration scaffolding (placeholder; the actual migration runner can be skeletal in Phase 0 since no pack updates exist yet).
- `CLAUDE.md` and `AGENTS.md` updated to document the pack system as canonical.

### What Phase 0 does not do

- No new mechanic frameworks (see Phases 1–3).
- No content imports from sources outside XWN (those are future cycles consuming this work).
- No code-bearing pack code beyond the existing `house_rules/practice_skills.py`, which is moved into `xwn-core`.
- No pack archive (`.zip`) loading at runtime in production. Directory loading is the v1 default. Archive loading is a stretch goal at the end of this phase if time permits.

## 2. Decisions locked in this phase

These were reviewed and confirmed before writing this spec. They appear here so future readers can find them in one place.

- **Pack archive format:** directory by default during dev. `.zip` archives optional in production (stretch goal; if not implemented in Phase 0, deferred).
- **Manifest filename:** `pack.yaml` at pack root.
- **Manifest top-level fields:** `id`, `version`, `name`, `description`, `authors`, `depends`, `conflicts`, `provides`. Format detailed in Task 0.2.
- **Pack version semantics:** SemVer-like `MAJOR.MINOR.PATCH`. Major version bumps may include schema migrations; minor and patch may not.
- **Override storage:** SQLite table `pack_overrides` per-world. Schema in Task 0.14.
- **Code-bearing pack code location:** `packs/<pack-id>/code/` directory. The `house_rules/` subdirectory naming is preserved inside the pack: `packs/xwn-core/code/house_rules/practice_skills.py`.
- **`xwn-core` initial version:** `1.0.0` at refactor completion.
- **Task numbering:** `0.1, 0.2, ...` matching existing milestone task conventions.

## 3. Tasks

Tasks are sized aggressively — 1, 2, or 3 story points each. Most are 1 or 2 points. A 3-point task is the largest a single session should attempt. Dependencies between tasks are declared in each task's header; an agent picks any task whose dependencies are complete.

Test layer notation in each task header: **[U]** = pytest unit tests required, **[P]** = Hypothesis property tests required, **[M]** = mutmut mutation tests required (≥85% kill rate), **[E2E]** = Playwright E2E tests required, **[V]** = Vitest unit tests required (frontend), **[FC]** = fast-check property tests required (frontend), **[S]** = Stryker mutation tests required (frontend).

---

### Task 0.1 — Codebase audit and pack-target inventory

**Points:** 2
**Dependencies:** none
**Test layers:** none (pure investigation, output is a markdown file)

**What:** Before any code changes, examine the current state of the codebase and produce an inventory of everything that becomes pack content. This is the agent's grounding step.

**Procedure:**
1. List every file under `data/` recursively. For each file, classify as: `xwn-core content`, `engine config (stays out of packs)`, or `unclear (flag for review)`.
2. List every YAML file referenced by Python code at module load time (skill mappings, classes, weapons, armor, creatures, tables, etc.) and trace where it's loaded.
3. List every file under `src/harsh_realm/house_rules/`. Classify as `xwn-core code` or `unclear`.
4. Identify all places in `src/harsh_realm/` where `data/...` paths are hardcoded.
5. Identify the world creation flow: where in code does world creation happen, and where would a pack-list parameter need to be threaded through?
6. Identify the world load flow: where in code does world load happen, and where would pack reconstitution need to hook in?
7. Identify the admin service's read paths (where does it serve content records to the admin UI?). These will need override-aware wrapping in Task 0.16.

**Deliverable:** `docs/superpowers/specs/2026-04-26-phase-0-codebase-audit.md` — a markdown report capturing all of the above. This document is consumed by subsequent tasks; it is not user-facing documentation.

**Acceptance:** The audit document exists, contains all six sections, and is concrete enough that subsequent tasks can be done without re-reading the entire codebase.

---

### Task 0.2 — Pack manifest schema

**Points:** 1
**Dependencies:** 0.1
**Test layers:** [U]

**What:** Define the Pydantic model for `pack.yaml` and a schema validator. No I/O yet — just the model and validation rules.

**File:** `src/harsh_realm/packs/manifest.py` (new)

**Manifest fields:**

```python
class PackManifest(BaseModel):
    """Pack manifest, parsed from pack.yaml."""
    model_config = ConfigDict(frozen=True)

    id: str = Field(pattern=r"^[a-z][a-z0-9-]*$",
                    description="Lowercase kebab-case unique pack ID")
    version: str = Field(pattern=r"^\d+\.\d+\.\d+$",
                         description="SemVer MAJOR.MINOR.PATCH")
    name: str = Field(min_length=1, description="Human-readable pack name")
    description: str = Field(default="", description="Brief description")
    authors: list[str] = Field(default_factory=list)
    depends: list[PackDependency] = Field(default_factory=list)
    conflicts: list[str] = Field(default_factory=list,
                                  description="Pack IDs that cannot be loaded with this one")
    provides: list[str] = Field(default_factory=list,
                                description="Capability tags this pack provides")


class PackDependency(BaseModel):
    """A declared dependency on another pack."""
    model_config = ConfigDict(frozen=True)

    id: str
    version: str = Field(default=">=0.0.0",
                         description="Version constraint, e.g. '>=1.0.0', '==1.2.3'")
```

**Tests:** `tests/packs/test_manifest.py`
- Valid manifest parses correctly.
- Invalid pack ID (uppercase, special chars) raises `ValidationError`.
- Invalid version format raises `ValidationError`.
- Empty name raises `ValidationError`.
- Dependency with constraint string parses; missing constraint defaults to `>=0.0.0`.

**Acceptance:** Tests pass. Manifest model is importable as `from harsh_realm.packs.manifest import PackManifest`.

---

### Task 0.3 — Version constraint parser

**Points:** 2
**Dependencies:** 0.2
**Test layers:** [U] [P]

**What:** A small parser for version constraints used in `PackDependency.version`. Supports `>=`, `<=`, `==`, `<`, `>`, `~=` (compatible release, e.g., `~=1.2.3` matches `>=1.2.3, <1.3.0`).

**File:** `src/harsh_realm/packs/version.py` (new)

**API:**
```python
def satisfies(version: str, constraint: str) -> bool: ...
def parse_version(version: str) -> tuple[int, int, int]: ...
```

**Tests:** `tests/packs/test_version.py`
- `satisfies("1.2.3", ">=1.0.0")` → True
- `satisfies("1.2.3", "<1.0.0")` → False
- `satisfies("1.2.3", "==1.2.3")` → True
- `satisfies("1.2.3", "~=1.2.0")` → True
- `satisfies("1.3.0", "~=1.2.0")` → False
- Property test: any valid SemVer satisfies `>=0.0.0`.
- Property test: `version` always satisfies `==version`.
- Invalid constraint format raises `ValueError`.
- Invalid version format raises `ValueError`.

**Acceptance:** Tests pass.

---

### Task 0.4 — Pack directory structure and content loader

**Points:** 3
**Dependencies:** 0.2
**Test layers:** [U] [P]

**What:** A loader that reads a pack directory from disk, parses its `pack.yaml`, and lazily reads content YAML files into a content record map keyed by namespaced ID.

**File:** `src/harsh_realm/packs/loader.py` (new)

**Pack directory convention:**

```
packs/<pack-id>/
  pack.yaml
  content/
    <category>/
      <slug>.yaml
    <category-with-many-files>/
      <slug>.yaml
      ...
    <single-file-category>.yaml      # Single-file format also accepted
  code/                                # Optional, for code-bearing packs
    __init__.py
    ...
  migrations/
    data/
      v<old>_to_v<new>.py
    schema/
      v<old>_to_v<new>.sql
```

**Two YAML layouts supported:**
- **One record per file:** `content/weapons/short_sword.yaml` contains a single record.
- **Many records per file:** `content/skills.yaml` contains a top-level dict `records: [...]` listing many records.

The loader handles both.

**API:**
```python
class Pack:
    """A loaded pack — manifest plus content access."""
    manifest: PackManifest
    root_path: Path
    def get_record(self, category: str, slug: str) -> dict | None: ...
    def list_records(self, category: str) -> list[tuple[str, dict]]: ...
    def list_categories(self) -> list[str]: ...

def load_pack(path: Path) -> Pack: ...
```

Each record dict gets injected fields `_pack_id`, `_category`, `_slug`, `_qualified_id` (`<pack-id>:<category>.<slug>`). These are read-only and exist for downstream code to know provenance.

**Tests:** `tests/packs/test_loader.py`
- Load a fixture pack with a manifest and one weapon record.
- Manifest parses correctly.
- `get_record("weapons", "short_sword")` returns the record with `_qualified_id = "test-pack:weapons.short_sword"`.
- `list_records("weapons")` returns all weapons.
- Both YAML layouts (single-record file, multi-record file with `records:` list) load identically.
- Missing `pack.yaml` raises `PackLoadError`.
- Missing `content/` directory: pack loads but has no records.
- Property test: any record retrieved by `get_record` has matching `_pack_id`, `_category`, `_slug` fields.

**Acceptance:** Tests pass. Loader handles both directory layouts. Records carry provenance.

---

### Task 0.5 — Pack registry and dependency resolution

**Points:** 3
**Dependencies:** 0.3, 0.4
**Test layers:** [U] [P]

**What:** A registry that holds multiple loaded packs, validates dependency satisfaction, detects conflicts, and produces a final ordered pack list ready for content reads.

**File:** `src/harsh_realm/packs/registry.py` (new)

**API:**
```python
class PackRegistry:
    def __init__(self, packs: list[Pack]) -> None: ...

    @classmethod
    def from_directory(cls, packs_root: Path, pack_ids: list[str]) -> "PackRegistry":
        """Load specified packs from packs_root. Validate dependencies and conflicts."""

    def packs_in_load_order(self) -> list[Pack]: ...
    def get_pack(self, pack_id: str) -> Pack | None: ...
```

**Validation rules:**
1. Every dependency in every loaded pack must resolve to a loaded pack at a satisfying version.
2. No pair of loaded packs may declare each other in `conflicts`.
3. Load order respects dependency order: if A depends on B, B comes before A in `packs_in_load_order()`. The user-supplied `pack_ids` order is used as a tiebreaker for independent packs.
4. If validation fails, `PackRegistry.from_directory` raises `PackResolutionError` with a message naming the missing/conflicting packs.

**Tests:** `tests/packs/test_registry.py`
- Two-pack registry with valid dependency loads in correct order.
- Missing dependency raises `PackResolutionError` naming the missing pack.
- Conflicting packs raise `PackResolutionError` naming both.
- Circular dependency (A→B, B→A) raises `PackResolutionError`.
- Diamond dependency (A→B, A→C, B→D, C→D) loads with D before B and C, both before A.
- Property test: in any valid registry, for every pack P and every dependency D of P, D's index in `packs_in_load_order()` is less than P's index.

**Acceptance:** Tests pass.

---

### Task 0.6 — Conflict detection at the record level

**Points:** 2
**Dependencies:** 0.5
**Test layers:** [U]

**What:** Detect when two packs in a registry register conflicting *content* (same fully-qualified ID, *or* both registering a behavior that targets the same key like a status effect resolver name).

This task implements only the *content ID* collision detection. Behavior-key collision detection happens in later phases when the relevant frameworks exist.

**File:** `src/harsh_realm/packs/registry.py` (extend)

**API additions:**
```python
class PackRegistry:
    def detect_id_conflicts(self) -> list[IdConflict]: ...

class IdConflict(BaseModel):
    qualified_id: str
    pack_ids: list[str]   # Packs that all define this ID
```

**Behavior:** For a record to appear in two packs with the same `_qualified_id`, the second pack would have to use the first pack's ID as its namespace. This is intentional ("override pack") and not necessarily an error; the registry returns these as informational `IdConflict` entries. The world creation flow (Task 0.13) decides whether to surface them as errors or accept the later-pack-wins resolution.

For Phase 0 scope: surface them as errors at world creation. Override packs are out of scope for Phase 0; an explicit override mechanism can be added in a future cycle if needed.

**Tests:** `tests/packs/test_conflict_detection.py`
- Two packs with disjoint records: no conflicts.
- Two packs both defining `xwn-core:weapons.short_sword`: one conflict reported with both pack IDs.
- A pack defining its own record (`my-pack:weapons.club`): no conflict.

**Acceptance:** Tests pass.

---

### Task 0.7 — Unified content read API across registry

**Points:** 2
**Dependencies:** 0.5
**Test layers:** [U] [P]

**What:** A read API on the `PackRegistry` that resolves a qualified ID to its record across all loaded packs.

**File:** `src/harsh_realm/packs/registry.py` (extend)

**API additions:**
```python
class PackRegistry:
    def get_record(self, qualified_id: str) -> dict | None: ...
    def list_records(self, category: str, pack_id: str | None = None) -> list[dict]: ...
    def list_qualified_ids(self, category: str) -> list[str]: ...
```

`get_record("xwn-core:weapons.short_sword")` returns the record from the `xwn-core` pack. If no such record exists, returns `None`. (No fallback or ambiguous resolution at this layer; callers who want override-layered reads use the world API in Task 0.16.)

`list_records("weapons")` returns every weapon record from every pack, in pack load order.

**Tests:** `tests/packs/test_registry_reads.py`
- `get_record` returns the correct record.
- `get_record` returns `None` for unknown qualified ID.
- `list_records` aggregates across all packs in load order.
- `list_records(category, pack_id="xwn-core")` filters to one pack.
- Property test: every record returned by `list_records(c)` has `_category == c`.

**Acceptance:** Tests pass.

---

### Task 0.8 — Pack file system layout and `xwn-core` skeleton

**Points:** 1
**Dependencies:** 0.4
**Test layers:** none (file structure task)

**What:** Create the `packs/` directory at the repo root and the `packs/xwn-core/` skeleton with an empty manifest. Content files are *not* moved yet; that's Task 0.18.

**Files created:**
- `packs/xwn-core/pack.yaml`
- `packs/xwn-core/content/.gitkeep`
- `packs/xwn-core/code/.gitkeep`
- `packs/xwn-core/migrations/data/.gitkeep`
- `packs/xwn-core/migrations/schema/.gitkeep`

**`pack.yaml` initial contents:**
```yaml
id: xwn-core
version: 1.0.0
name: XWN Core
description: |
  Default Harsh Realm content pack. Provides XWN (Worlds Without Number /
  Stars Without Number) rules content: skills, classes, weapons, armor,
  creatures, encounter tables, social mechanics, faction system.
authors:
  - Harsh Realm Project
depends: []
conflicts: []
provides:
  - xwn-core
```

**Acceptance:** Directory exists. `load_pack(Path("packs/xwn-core"))` from Task 0.4 returns a valid empty `Pack`.

---

### Task 0.9 — Default pack-root configuration

**Points:** 1
**Dependencies:** 0.5, 0.8
**Test layers:** [U]

**What:** Add a configuration entry for the packs root directory and wire it into application config.

**File:** `src/harsh_realm/config.py` (extend)

**Config field added:**
```python
class StorageConfig(BaseModel):
    # existing fields...
    packs_root: Path = Field(default=Path("packs"),
                              description="Filesystem root for pack directories")
```

**Acceptance:** Config loads with default `packs_root`. Override via env var or config file works. Existing config tests still pass; new test covers the field.

---

### Task 0.10 — Pack discovery: list available packs

**Points:** 1
**Dependencies:** 0.4, 0.9
**Test layers:** [U]

**What:** A function that scans the packs root and returns metadata for every available pack (without fully loading their content).

**File:** `src/harsh_realm/packs/discovery.py` (new)

**API:**
```python
def discover_packs(packs_root: Path) -> list[PackManifest]:
    """Return manifests of all packs in the packs root, sorted by ID."""
```

This is what the admin UI calls to populate the pack-selection list at world creation.

**Tests:** `tests/packs/test_discovery.py`
- Two packs in fixture root → both manifests returned, sorted by ID.
- Empty root → empty list.
- Non-existent root → empty list (not an error).
- Directory without `pack.yaml` is skipped (not an error).

**Acceptance:** Tests pass.

---

### Task 0.11 — World schema: pack list and override table

**Points:** 2
**Dependencies:** none (only edits `db.py`)
**Test layers:** [U]

**What:** Add SQLite tables to the world database schema for pack tracking and per-world overrides.

**File:** `src/harsh_realm/db.py` (extend `_init_schema`)

**New tables:**
```sql
CREATE TABLE world_packs (
    pack_id    TEXT NOT NULL,
    version    TEXT NOT NULL,
    load_order INTEGER NOT NULL,
    PRIMARY KEY (pack_id),
    UNIQUE (load_order)
);

CREATE TABLE pack_overrides (
    pack_id      TEXT NOT NULL,
    qualified_id TEXT NOT NULL,
    data_json    TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (pack_id, qualified_id)
);
```

`world_packs` records the exact pack list at world creation. `pack_overrides` stores per-world edits keyed by pack ID and qualified record ID.

**Tests:** `tests/test_db.py` (extend)
- New world has empty `world_packs` and `pack_overrides`.
- Insert into both tables and read back.
- Schema contains both tables.

**Acceptance:** Tests pass. Existing world creation still succeeds (with empty pack tables — Task 0.13 wires up the actual pack list write).

---

### Task 0.12 — World-pack repository

**Points:** 2
**Dependencies:** 0.11
**Test layers:** [U]

**What:** A repository module that owns reads/writes to `world_packs` and `pack_overrides`. Per AGENTS.md Rule 4, this is the only place that should issue SQL against these tables.

**File:** `src/harsh_realm/packs/world_repository.py` (new)

**API:**
```python
class WorldPackRepository:
    """Persistence for world_packs and pack_overrides."""
    PERSISTENCE = "durable"

    def __init__(self, db: WorldDatabase) -> None: ...
    async def set_pack_list(self, packs: list[PackBinding]) -> None: ...
    async def get_pack_list(self) -> list[PackBinding]: ...
    async def get_override(self, pack_id: str, qualified_id: str) -> dict | None: ...
    async def set_override(self, pack_id: str, qualified_id: str, data: dict) -> None: ...
    async def delete_override(self, pack_id: str, qualified_id: str) -> bool: ...
    async def list_overrides(self) -> list[OverrideRecord]: ...

class PackBinding(BaseModel):
    pack_id: str
    version: str
    load_order: int

class OverrideRecord(BaseModel):
    pack_id: str
    qualified_id: str
    data: dict
    updated_at: str
```

**Tests:** `tests/packs/test_world_repository.py`
- Set pack list and read back; load order preserved.
- Set, get, delete override; list shows expected entries.
- `get_override` returns `None` for non-existent override.
- `delete_override` returns `True` if deleted, `False` if no row.

**Acceptance:** Tests pass.

---

### Task 0.13 — World creation accepts a pack list

**Points:** 3
**Dependencies:** 0.5, 0.10, 0.12
**Test layers:** [U] [E2E]

**What:** Extend the world creation flow (`POST /api/worlds`) to accept a list of pack IDs and persist them.

**Files:**
- `src/harsh_realm/api/routes.py` (extend world creation route)
- `src/harsh_realm/packs/registry.py` (no changes; reuse from 0.5)
- `frontend/src/views/...` (whichever component owns world creation; identified during Task 0.1 audit)

**Backend:**
- `POST /api/worlds` request body gains `pack_ids: list[str]` (default: `["xwn-core"]` if omitted, for backward compatibility during transition).
- Server constructs a `PackRegistry.from_directory(packs_root, pack_ids)`. If resolution fails, return 400 with the error.
- Server constructs the world, persists pack bindings via `WorldPackRepository.set_pack_list()`.
- Response includes the resolved pack list with versions.

**Frontend:**
- World creation form gains a pack picker that calls `GET /api/packs` (Task 0.10's `discover_packs` exposed via a new route).
- User can multi-select packs. Order is preserved as user-specified.
- Default selection: `xwn-core` checked.
- Submitting the form sends `pack_ids`.

**Tests:**
- Backend unit: world creation with valid pack list persists bindings.
- Backend unit: world creation with invalid pack list returns 400.
- Backend unit: world creation with empty pack list defaults to `["xwn-core"]`.
- Playwright: user creates a new world, selects packs, sees the new world in the world list.

**Acceptance:** A new world is created with a recorded pack list visible via SQL inspection.

---

### Task 0.14 — World load reconstitutes pack registry

**Points:** 2
**Dependencies:** 0.5, 0.12
**Test layers:** [U]

**What:** When a world is loaded (`POST /api/worlds/load`), read its pack list and construct a `PackRegistry`. Attach the registry to app state so other modules can read content through it.

**Files:**
- `src/harsh_realm/api/routes.py` (extend world load route)
- `src/harsh_realm/main.py` (add `pack_registry` to app state)

**Behavior:**
- After opening the world DB, read `world_packs` via `WorldPackRepository.get_pack_list()`.
- Construct `PackRegistry.from_directory(packs_root, [b.pack_id for b in bindings])`.
- Verify each binding's recorded version satisfies the loaded pack's version: if the loaded version is newer, log a warning (migration may be needed; Phase 0 only logs). If older, error (cannot run a world against a downgraded pack).
- Attach to `app.state.pack_registry`.
- On world unload, clear `app.state.pack_registry`.

**Tests:** `tests/packs/test_world_load.py`
- Load a world with a recorded pack list → registry is attached to app state.
- Load a world whose recorded pack list references a missing pack → error.
- Load a world whose recorded version is newer than installed → warning logged, world still loads.
- Load a world whose recorded version is older than installed → error.

**Acceptance:** Tests pass.

---

### Task 0.15 — Override-aware content read API

**Points:** 2
**Dependencies:** 0.7, 0.12, 0.14
**Test layers:** [U] [P]

**What:** A read API that resolves a qualified ID through the pack registry but checks the world's override table first.

**File:** `src/harsh_realm/packs/content_service.py` (new)

**API:**
```python
class ContentService:
    """Override-aware content reads for the active world."""
    def __init__(self, registry: PackRegistry, repo: WorldPackRepository) -> None: ...
    async def get(self, qualified_id: str) -> dict | None: ...
    async def list_records(self, category: str) -> list[dict]: ...
    async def has_override(self, qualified_id: str) -> bool: ...
```

`get()` checks `pack_overrides` first; if present, deserializes the override JSON and returns it (with `_pack_id`, `_category`, `_slug`, `_qualified_id` re-injected and a new `_overridden = True` flag). Otherwise falls back to `registry.get_record()`.

`list_records()` returns the registry's records, with each record's data merged with its override if any.

**Tests:** `tests/packs/test_content_service.py`
- `get()` returns pack data when no override exists.
- `get()` returns override data when override exists; record carries `_overridden = True`.
- `list_records` includes overrides correctly.
- Property test: `has_override(id) == True ⇔ get(id) returns _overridden=True record`.

**Acceptance:** Tests pass.

---

### Task 0.16 — Existing engine reads route through ContentService

**Points:** 3
**Dependencies:** 0.15
**Test layers:** [U]

**What:** Find every place in `src/harsh_realm/` that reads from `data/...` files or assumes hardcoded content keys, and refactor it to read via `ContentService`. The audit from Task 0.1 enumerates these.

This task is **the most error-prone** in Phase 0. Tackle methodically:

1. Skill mappings, difficulty targets, disposition outcomes, encounter weights, faction asset stats: these are seeded from YAML into SQLite at world creation. The seeding step (`AdminService.seed_all_from_yaml`) becomes a seeding-from-pack step.
2. Weapons, armor, equipment kits, classes, skills: read at runtime from YAML by various engine modules. These callers switch to `ContentService`.
3. Random tables: loaded once at world creation into `random_tables` SQLite table. The loading step iterates packs in load order.
4. Creatures: loaded at runtime by encounter generation. Switches to `ContentService`.
5. Templates (combat narration, social narration): loaded at runtime. Switches to `ContentService`.

**Tests:** existing test suite must continue to pass. Specific new tests:
- A world loaded with `xwn-core` only behaves identically to the pre-Phase-0 codebase for skill check, combat, encounter generation, social interaction, faction turn (smoke tests, not exhaustive — existing tests cover behavior).
- An override on a weapon record changes the weapon's stats in subsequent reads.

**Acceptance:** Full existing test suite passes. Manual smoke test: create a world, play through one encounter, verify nothing has visibly changed.

---

### Task 0.17 — Override CRUD API endpoints

**Points:** 2
**Dependencies:** 0.15
**Test layers:** [U]

**What:** REST endpoints for the admin UI to read, write, and delete overrides.

**File:** `src/harsh_realm/api/admin_routes.py` (extend) or `src/harsh_realm/api/editor_routes.py` (whichever fits the existing admin/editor separation per Task 0.1 audit)

**Endpoints:**
- `GET /api/world/content/<qualified_id>` → returns the resolved record (with `_overridden` flag).
- `PUT /api/world/content/<qualified_id>` body `{"data": {...}}` → upserts an override.
- `DELETE /api/world/content/<qualified_id>` → removes the override (record reverts to pack default).
- `GET /api/world/overrides` → returns list of all overrides for the active world.

**Tests:** `tests/packs/test_override_routes.py`
- GET returns pack data when no override.
- PUT creates override; subsequent GET returns override.
- DELETE removes override; subsequent GET returns pack data.
- GET list returns expected entries.

**Acceptance:** Tests pass.

---

### Task 0.18 — Move `data/` into `packs/xwn-core/content/`

**Points:** 3
**Dependencies:** 0.16, 0.17
**Test layers:** [U] [E2E]

**What:** Physically move all current XWN content from `data/` into `packs/xwn-core/content/`. Update all references. Remove the old `data/` directory.

**Procedure:**
1. For each subdirectory and file in `data/`, decide:
   - Is this content (game records)? → moves to `packs/xwn-core/content/<category>/`.
   - Is this engine config (schemas, etc.) that doesn't belong in a pack? → moves to a new location, e.g., `src/harsh_realm/schemas/` for engine-internal schemas.
   - Is this *both* a game schema and engine-internal? → flag for review, default to engine-internal.
2. Update every code reference to `data/...` paths to use `ContentService` or, if the file is not pack content, the new engine-internal location.
3. Update `data/skills.yaml` (top-level file) to `packs/xwn-core/content/skills.yaml` (top-level in the pack's content). Same for other top-level files.
4. Remove the `data/` directory (or leave it empty with a `.gitkeep` and a README pointing to packs).
5. Run the full existing test suite and fix any remaining references.

**Tests:**
- Full existing test suite passes.
- Playwright smoke: world creation, character creation, exploration, an encounter, all work end-to-end.

**Acceptance:** No code references `data/...` for game content. The `xwn-core` pack contains all current game content. Existing tests all pass.

---

### Task 0.19 — Move `house_rules/practice_skills.py` into `xwn-core`

**Points:** 1
**Dependencies:** 0.18
**Test layers:** [U]

**What:** Move the existing house rule code into the `xwn-core` pack's code directory.

**Procedure:**
1. Create `packs/xwn-core/code/__init__.py` (empty).
2. Create `packs/xwn-core/code/house_rules/__init__.py`.
3. Move `src/harsh_realm/house_rules/practice_skills.py` → `packs/xwn-core/code/house_rules/practice_skills.py`.
4. Update imports across the codebase. The pack's `code/` directory needs to be importable; this can be done by making `packs/` a regular Python package (add `packs/__init__.py`) or by registering pack code paths in `sys.path` at pack-load time. Pick one approach and document it. **Recommended: register in `sys.path` at pack-load time** — keeps pack directories from being implicit Python packages.
5. Update `src/harsh_realm/house_rules/__init__.py` to either be empty (if no remaining house rules) or removed entirely.

**Tests:**
- Existing tests for practice skills still pass.
- A unit test confirms `packs/xwn-core/code/house_rules/practice_skills.py` is importable after pack load.

**Acceptance:** Tests pass. The `xwn-core` pack is self-contained: removing it from `packs/` and the world's pack list would remove all XWN content and code from the active engine.

---

### Task 0.20 — Code-bearing pack registration hook

**Points:** 2
**Dependencies:** 0.19
**Test layers:** [U]

**What:** Define how a code-bearing pack registers itself with the engine at load time. This is the first piece of the API surface mentioned in the overview's §5.6.

**File:** `src/harsh_realm/packs/code_loader.py` (new)

**API:**
```python
def load_pack_code(pack: Pack, app_state: AppState) -> None:
    """If pack has code/, import its __init__.py and call its register() function."""
```

A pack's `code/__init__.py` may define a `register(app_state)` function that the engine calls after the pack is loaded. The function receives the app state (event bus, registries, etc.) and may register handlers, resolvers, or other extensions.

For Phase 0, the only thing pack code can do is register house rule resolvers (since that's all that exists in `xwn-core`). The `xwn-core` `register()` function calls into the existing house-rules machinery.

**Tests:**
- Pack with no `code/` directory loads with no error.
- Pack with `code/__init__.py` defining `register()` is called at pack-load time.
- Pack code register failure surfaces as an error at world load (do not silently swallow).

**Acceptance:** Tests pass. The `xwn-core` pack's house rules are registered via this mechanism on world load.

---

### Task 0.21 — Migration scaffolding (skeletal)

**Points:** 2
**Dependencies:** 0.14
**Test layers:** [U]

**What:** Define the migration interface and runner. The runner is *skeletal* — it knows how to discover migration files and run them, but Phase 0 has no migrations to run (since `xwn-core` is at v1.0.0 with no prior versions). This sets the API for future use.

**File:** `src/harsh_realm/packs/migrations.py` (new)

**API:**
```python
def get_pending_migrations(
    pack: Pack,
    current_version: str,
) -> list[Migration]:
    """List migrations needed to bring a world from current_version to pack.manifest.version."""

class Migration(BaseModel):
    pack_id: str
    from_version: str
    to_version: str
    kind: Literal["data", "schema"]
    path: Path

async def run_migration(migration: Migration, db: WorldDatabase, registry: PackRegistry) -> None:
    """Execute a single migration."""
```

Migration discovery:
- Schema migrations: `packs/<pack-id>/migrations/schema/v<old>_to_v<new>.sql` (executed as SQL).
- Data migrations: `packs/<pack-id>/migrations/data/v<old>_to_v<new>.py` (a Python module exposing an `async def migrate(db, registry)` function).

Phase 0 produces the discovery and runner code with tests using fixture migrations. No real migrations run because no real version mismatches occur.

**Tests:**
- Discovery returns empty list when current_version equals pack version.
- Discovery returns ordered list when multiple version steps separate current from target.
- Schema migration fixture executes against an in-memory SQLite.
- Data migration fixture executes and can read/write through the registry.
- Migration failure raises `MigrationError`.

**Acceptance:** Tests pass.

---

### Task 0.22 — Frontend: world creation pack picker

**Points:** 2
**Dependencies:** 0.13
**Test layers:** [V] [E2E]

**What:** Implement the pack picker UI in the world creation form.

**Files:**
- `frontend/src/components/world/PackPicker.vue` (new)
- `frontend/src/views/...` (existing world creation view)
- `frontend/src/types/api.ts` (extend with `PackManifest` type)
- `frontend/src/api/packs.ts` (new — calls `GET /api/packs`)

**Behavior:**
- On mount, fetch available packs from `GET /api/packs`.
- Display each pack with its name, version, description, and a checkbox.
- `xwn-core` is checked by default and can be unchecked, but doing so requires a confirmation ("This will create a world with no XWN content. You may need code-bearing alternative packs.").
- Order is established by user check sequence; user can reorder via drag-and-drop or up/down buttons.
- Submit sends pack IDs in chosen order.

**Tests:**
- Vitest: PackPicker component renders pack list, default selection works, reorder updates state.
- Playwright: user creates a world with custom pack selection.

**Acceptance:** Tests pass.

---

### Task 0.23 — Frontend: override indicators in admin

**Points:** 2
**Dependencies:** 0.17
**Test layers:** [V] [E2E]

**What:** In the admin UI, show which records are overridden and offer a "revert to pack default" action.

**Files:**
- `frontend/src/components/admin/OverrideIndicator.vue` (new)
- Existing admin tab components (extended to show override badge + revert button on each editable record)

**Behavior:**
- A record shown in the admin UI displays a small badge if `_overridden = True`.
- Hovering or expanding the badge shows the pack default value alongside the overridden value.
- A "Revert to default" button calls `DELETE /api/world/content/<qualified_id>` and refreshes the record.
- An "Edit" action that submits a change calls `PUT /api/world/content/<qualified_id>` and creates an override.

**Tests:**
- Vitest: OverrideIndicator renders correctly for overridden vs. non-overridden records.
- Playwright: user edits a weapon, sees override badge, reverts, badge disappears.

**Acceptance:** Tests pass.

---

### Task 0.24 — Documentation updates

**Points:** 2
**Dependencies:** 0.18, 0.19
**Test layers:** none (documentation task)

**What:** Update `CLAUDE.md` and `AGENTS.md` to reflect the new pack architecture.

**`CLAUDE.md` updates:**
- Add a "Packs and Modular Rules" section near the top of "Architecture Summary" describing the four-layer model (kernel / frameworks / packs / worlds).
- Update "Key Architectural Rules" to add a rule about pack-vs-engine separation: "Game content (records, tables, data values) lives in packs. Engine code is content-free."
- Update "File Map" to show `packs/xwn-core/` and remove `data/`.
- Update "Tech Stack" if needed (no new dependencies expected).

**`AGENTS.md` updates:**
- Add a "Pack-aware code" section under "Database Access" or "Data Models" describing:
  - Reads of game content go through `ContentService`, never through hardcoded paths.
  - New record types added in future cycles ship as pack content, not hardcoded YAML in engine.
  - Code-bearing pack code lives in `packs/<pack-id>/code/` and registers via `register(app_state)`.
- Add to "What NOT to Do":
  - "No hardcoded `data/...` paths in engine code. Use `ContentService`."
  - "No game records in engine code. New record types go in packs."

**Acceptance:** Both files updated. A coding agent reading them after the update can build pack-aware features without needing to consult Phase 0's spec.

---

### Task 0.25 — Acceptance criteria document update

**Points:** 1
**Dependencies:** all preceding tasks
**Test layers:** none (documentation task)

**What:** Append Phase 0 entries to `docs/acceptance_criteria.md`.

**Per the convention:** each task's acceptance criteria are summarized in `docs/acceptance_criteria.md` after the phase completes. This task adds a new section "Modular Rules Architecture — Phase 0 Foundation" with one entry per task summarizing what was delivered.

**Acceptance:** Document updated with Phase 0 entries.

---

## 4. Phase completion criteria

Phase 0 is complete when *all* of the following hold:

1. All 25 tasks above are implemented and committed.
2. Full existing test suite passes (current count: 1139). New tests added by Phase 0 raise the total.
3. `pip install -e .` followed by full `pytest` shows zero failures.
4. `npx tsc --noEmit` in `frontend/` shows zero errors.
5. A user can create a world via the UI with default pack selection (`xwn-core`) and play through character creation + an exploration scene + an encounter without observing any behavioral difference vs. the pre-Phase-0 build.
6. A user can edit a weapon record in admin, see the override indicator, revert it, and see the indicator disappear.
7. `data/` directory does not contain any game content. All game content is in `packs/xwn-core/content/`.
8. `src/harsh_realm/house_rules/` either does not exist or is empty.
9. `CLAUDE.md` and `AGENTS.md` are updated.
10. `docs/acceptance_criteria.md` includes Phase 0 entries.

## 5. Phase 0 deferrals (append to overview §11)

Items deferred from Phase 0:

- **Pack archive (`.zip`) loading.** Stretch goal not implemented; only directory loading works. Add in a future cycle if pack distribution becomes a use case.
- **Override packs (packs that intentionally redefine another pack's records).** Detected as conflicts at world creation in Phase 0. A future cycle could add explicit override-pack semantics.
- **Pack signing, trust, and sandboxing.** Not relevant while Harsh Realm is single-user.
- **Pack hot-reload during development.** Out of scope; restart the engine to pick up pack changes.
- **Mid-game pack list changes.** Frozen at world creation per overview §5.4.

## 6. Notes for the coding agent

- Tasks declare explicit dependencies. Always pick a task whose dependencies are complete.
- Task 0.1 (audit) is the very first task and produces a document that subsequent tasks consume. Do not skip it; the audit doc prevents wasted work in later tasks.
- Task 0.16 (engine reads route through `ContentService`) is the most error-prone and the largest single behavior change. Tackle methodically with frequent test runs; do not batch unrelated refactors.
- Task 0.18 (move `data/`) is destructive. Make a clean commit *before* starting it. If anything goes wrong, revert is one command.
- Task 0.19 (move house rules) requires deciding on the pack-code import strategy. The recommended approach is `sys.path` registration at pack-load time. Document the decision in the commit message.
- After every task, run the full test suite. A green test suite is the gating condition for moving to the next task.
- Commit messages should follow the project convention (imperative mood, concise) and append `[Phase 0 / Task 0.N]` for traceability.
