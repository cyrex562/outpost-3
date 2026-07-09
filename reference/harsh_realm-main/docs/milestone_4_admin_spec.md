# Harsh Realm — Skill Mapping System & Admin Panel
**Milestone:** 4 (alongside social scene work)  
**Estimated effort:** ~16 hours  
**Date:** 2026-03-27

---

## Design Principle

**YAML is a seed. SQLite is the source of truth for a world.**

At world creation, all editable config tables are populated from YAML defaults. From that point on, SQLite owns the data for that world. The YAML is never read again during normal play. A "reset to default" admin action re-reads the YAML and overwrites the SQLite row — explicit, intentional, reversible only if the player saved a snapshot first.

This means:
- Each world can diverge from the defaults independently
- No merge logic, no delta tracking, no versioning headaches
- New YAML entries added to the repo do NOT auto-appear in existing worlds (intentional — worlds are stable)
- If you want a new default in an old world, you add it manually via admin

---

## Section A — Editable Config Tables (SQLite)

All tables below are created at world creation and seeded from YAML. They live in the world `.db` file alongside game state.

### A.1 `skill_mappings`

Maps social/action verbs to XWN skill + attribute + difficulty.

```sql
CREATE TABLE skill_mappings (
    verb            TEXT PRIMARY KEY,
    skill           TEXT NOT NULL,       -- XWN skill name: "Talk", "Connect", etc.
    attribute       TEXT NOT NULL,       -- "STR" | "DEX" | "CON" | "INT" | "WIS" | "CHA"
    base_difficulty INTEGER NOT NULL DEFAULT 8,
    opposed         INTEGER DEFAULT 0,   -- 1 = opposed check (target resists)
    description     TEXT,               -- shown in help/admin UI
    notes           TEXT                -- designer notes, not shown to player
);
```

**Seed YAML:** `data/skill_mappings.yaml`

```yaml
# data/skill_mappings.yaml
# verb: the command the player types
# skill: XWN skill used
# attribute: attribute modifier applied
# base_difficulty: default target number (8 = routine, 10 = hard, 12 = very hard)
# opposed: if true, target rolls to resist (WIS save or relevant skill)

- verb: convince
  skill: Talk
  attribute: CHA
  base_difficulty: 8
  opposed: true
  description: "Persuade an NPC to agree with you or do something."
  notes: "Target resists with WIS modifier. Friendly NPCs are easier to convince."

- verb: intimidate
  skill: Talk
  attribute: STR
  base_difficulty: 10
  opposed: true
  description: "Coerce an NPC through threat of force."
  notes: "Uses STR not CHA — physical presence matters. Always costs disposition on success."

- verb: deceive
  skill: Talk
  attribute: CHA
  base_difficulty: 10
  opposed: true
  description: "Lie to an NPC."
  notes: "If failed by 3+, NPC knows they were lied to. Disposition penalty scales with margin."

- verb: bribe
  skill: Trade
  attribute: CHA
  base_difficulty: 8
  opposed: false
  description: "Offer payment to get what you want."
  notes: "Requires gold/item in hand. Amount offered can modify difficulty (-2 to +2)."

- verb: connect
  skill: Connect
  attribute: CHA
  base_difficulty: 8
  opposed: false
  description: "Leverage existing social network or contacts."
  notes: "Only works if NPC has a plausible connection to the character's background."

- verb: ask
  skill: Talk
  attribute: CHA
  base_difficulty: 6
  opposed: false
  description: "Ask an NPC a direct question."
  notes: "Free unless the topic is sensitive. Sensitive topics use full difficulty."

- verb: perform
  skill: Perform
  attribute: CHA
  base_difficulty: 8
  opposed: false
  description: "Entertain or impress an NPC through performance."
  notes: "Deferred from M3. Included here for completeness — social skill check only."
```

---

### A.2 `difficulty_targets`

Named difficulty levels used throughout the rules engine.

```sql
CREATE TABLE difficulty_targets (
    name        TEXT PRIMARY KEY,   -- "routine", "challenging", "hard", "formidable", "heroic"
    target      INTEGER NOT NULL,
    description TEXT
);
```

**Seed YAML:** `data/difficulty_targets.yaml`

```yaml
- name: trivial
  target: 4
  description: "Anyone could do this under normal circumstances."

- name: routine
  target: 8
  description: "Standard difficulty. A competent person succeeds more often than not."

- name: challenging
  target: 10
  description: "Requires real skill or good luck."

- name: hard
  target: 12
  description: "Even skilled characters will fail sometimes."

- name: formidable
  target: 14
  description: "Near the edge of human capability."

- name: heroic
  target: 16
  description: "Only the best have a realistic chance."
```

---

### A.3 `disposition_outcomes`

How much disposition changes per skill check outcome band.

```sql
CREATE TABLE disposition_outcomes (
    outcome_key     TEXT PRIMARY KEY,   -- "large_failure", "small_failure", etc.
    delta           INTEGER NOT NULL,
    description     TEXT
);
```

**Seed YAML:** `data/disposition_outcomes.yaml`

```yaml
- outcome_key: exceptional_failure   # failed by 4+
  delta: -2
  description: "Large disposition penalty. NPC is insulted or threatened."

- outcome_key: failure               # failed by 1-3
  delta: -1
  description: "Small disposition penalty. NPC is unimpressed."

- outcome_key: bare_success          # success by 0-1
  delta: 0
  description: "Goal achieved. No disposition change."

- outcome_key: solid_success         # success by 2-3
  delta: 1
  description: "Goal achieved. NPC warms slightly."

- outcome_key: exceptional_success   # success by 4+
  delta: 2
  description: "Goal achieved. NPC is impressed. May volunteer extra info."

# Special overrides for specific verbs
- outcome_key: intimidate_success    # any successful intimidate
  delta: -1
  description: "Intimidation always costs goodwill, even when it works."

- outcome_key: deceive_caught        # failed deceive by 3+
  delta: -3
  description: "NPC knows they were lied to. Major trust damage."
```

---

### A.4 `encounter_weights`

Modifier weights applied to encounter table rolls based on faction disposition.

```sql
CREATE TABLE encounter_weights (
    faction_disposition TEXT NOT NULL,   -- "allied", "friendly", "neutral", etc.
    encounter_tag       TEXT NOT NULL,   -- "patrol_hostile", "trade_opportunity", etc.
    weight_modifier     INTEGER NOT NULL,
    PRIMARY KEY (faction_disposition, encounter_tag)
);
```

**Seed YAML:** `data/encounter_weights.yaml`

```yaml
- faction_disposition: allied
  encounter_tag: patrol_friendly
  weight_modifier: 3

- faction_disposition: allied
  encounter_tag: patrol_hostile
  weight_modifier: -999   # effectively impossible

- faction_disposition: friendly
  encounter_tag: patrol_hostile
  weight_modifier: -2

- faction_disposition: friendly
  encounter_tag: trade_opportunity
  weight_modifier: 2

- faction_disposition: unfriendly
  encounter_tag: patrol_hostile
  weight_modifier: 2

- faction_disposition: unfriendly
  encounter_tag: spy_encounter
  weight_modifier: 1

- faction_disposition: hostile
  encounter_tag: patrol_hostile
  weight_modifier: 4

- faction_disposition: hostile
  encounter_tag: bounty_hunter
  weight_modifier: 2

- faction_disposition: hostile
  encounter_tag: ambush
  weight_modifier: 1
```

---

### A.5 `faction_asset_stats`

WWN faction asset definitions. Editable per-world.

```sql
CREATE TABLE faction_asset_stats (
    asset_type      TEXT PRIMARY KEY,
    category        TEXT NOT NULL,       -- "force" | "cunning" | "wealth"
    min_attribute   INTEGER NOT NULL,    -- minimum faction attribute to purchase
    cost            INTEGER NOT NULL,    -- in faction XP
    upkeep          INTEGER DEFAULT 0,   -- per faction turn
    max_hp          INTEGER NOT NULL,
    attack_stat     TEXT,               -- e.g. "force_vs_force", "cunning_vs_wealth"
    counter_stat    TEXT,
    attack_roll     TEXT,               -- dice expression e.g. "1d6"
    special         TEXT,               -- JSON: special abilities
    description     TEXT
);
```

**Seed YAML:** `data/faction_assets.yaml` — full WWN asset list (Warriors, Informers, Smugglers, etc.)

---

### A.6 `random_tables` (already exists — admin makes it editable)

The `random_tables` table already exists in the schema from M2. The admin system adds read/write access to it via UI and CLI. No schema change needed.

---

## Section B — Backend Admin Service

**File:** `src/harsh_realm/admin/service.py`

A single `AdminService` class used by both the REST API and CLI. All mutation goes through here — no direct DB writes from routes or CLI.

```python
class AdminService:
    def __init__(self, db: WorldDatabase): ...

    # Skill mappings
    def list_skill_mappings(self) -> list[SkillMapping]: ...
    def get_skill_mapping(self, verb: str) -> SkillMapping | None: ...
    def set_skill_mapping(self, verb: str, skill: str, attribute: str,
                          base_difficulty: int, opposed: bool,
                          description: str = "") -> SkillMapping: ...
    def reset_skill_mapping(self, verb: str) -> SkillMapping: ...  # re-reads YAML default
    def reset_all_skill_mappings(self) -> list[SkillMapping]: ...

    # Difficulty targets
    def list_difficulty_targets(self) -> list[DifficultyTarget]: ...
    def set_difficulty_target(self, name: str, target: int) -> DifficultyTarget: ...
    def reset_difficulty_target(self, name: str) -> DifficultyTarget: ...

    # Disposition outcomes
    def list_disposition_outcomes(self) -> list[DispositionOutcome]: ...
    def set_disposition_outcome(self, outcome_key: str, delta: int) -> DispositionOutcome: ...
    def reset_disposition_outcome(self, outcome_key: str) -> DispositionOutcome: ...

    # Encounter weights
    def list_encounter_weights(self) -> list[EncounterWeight]: ...
    def set_encounter_weight(self, disposition: str, tag: str,
                             modifier: int) -> EncounterWeight: ...
    def reset_encounter_weights(self, disposition: str | None = None) -> list[EncounterWeight]: ...

    # Faction asset stats
    def list_faction_assets(self) -> list[FactionAsset]: ...
    def set_faction_asset(self, asset_type: str, **fields) -> FactionAsset: ...
    def reset_faction_asset(self, asset_type: str) -> FactionAsset: ...

    # Random tables
    def list_random_tables(self, category: str | None = None) -> list[RandomTableMeta]: ...
    def get_random_table(self, table_id: str) -> RandomTable: ...
    def set_table_entry(self, table_id: str, index: int,
                        weight: int, result: dict) -> None: ...
    def add_table_entry(self, table_id: str, weight: int, result: dict) -> None: ...
    def remove_table_entry(self, table_id: str, index: int) -> None: ...
    def reset_random_table(self, table_id: str) -> RandomTable: ...

    # Utility
    def seed_all_from_yaml(self) -> SeedResult: ...  # used at world creation
    def export_world_config(self) -> dict: ...        # full config snapshot as dict
```

**Tests:**
- `test_admin_service.py` — unit tests for all methods
- Set a skill mapping → get it back → verify SQLite row changed
- Reset a skill mapping → verify it matches YAML default exactly
- Reset all → all rows match YAML
- Set faction asset stat below valid range → raises `ValidationError`

---

## Section C — REST API (`/api/admin`)

**File:** `src/harsh_realm/api/admin_routes.py`

Separate router mounted at `/api/admin`. No auth (single-player, local). Could add a simple `?admin_key=` query param in future if needed.

```
GET    /api/admin/skill-mappings              → list all
GET    /api/admin/skill-mappings/{verb}       → get one
PUT    /api/admin/skill-mappings/{verb}       → set (full replace)
POST   /api/admin/skill-mappings/{verb}/reset → reset to YAML default
POST   /api/admin/skill-mappings/reset-all    → reset all

GET    /api/admin/difficulty-targets
PUT    /api/admin/difficulty-targets/{name}
POST   /api/admin/difficulty-targets/{name}/reset

GET    /api/admin/disposition-outcomes
PUT    /api/admin/disposition-outcomes/{key}
POST   /api/admin/disposition-outcomes/{key}/reset

GET    /api/admin/encounter-weights
PUT    /api/admin/encounter-weights/{disposition}/{tag}
POST   /api/admin/encounter-weights/reset

GET    /api/admin/faction-assets
GET    /api/admin/faction-assets/{asset_type}
PUT    /api/admin/faction-assets/{asset_type}
POST   /api/admin/faction-assets/{asset_type}/reset

GET    /api/admin/random-tables
GET    /api/admin/random-tables/{table_id}
PUT    /api/admin/random-tables/{table_id}/entries/{index}
POST   /api/admin/random-tables/{table_id}/entries
DELETE /api/admin/random-tables/{table_id}/entries/{index}
POST   /api/admin/random-tables/{table_id}/reset

POST   /api/admin/export-config             → download full config as JSON
```

**Tests:**
- `test_admin_routes.py` — integration tests using FastAPI `TestClient`
- PUT skill mapping → GET → verify response matches
- POST reset → GET → verify matches YAML seed

---

## Section D — CLI Admin Script

**File:** `src/harsh_realm/admin/cli.py`  
**Entry point:** `python -m harsh_realm.admin <world_path> <command> [args]`

```bash
# Skill mappings
python -m harsh_realm.admin worlds/ashfall.db skill-mappings list
python -m harsh_realm.admin worlds/ashfall.db skill-mappings get convince
python -m harsh_realm.admin worlds/ashfall.db skill-mappings set convince --skill Talk --attr CHA --difficulty 8 --opposed
python -m harsh_realm.admin worlds/ashfall.db skill-mappings reset convince
python -m harsh_realm.admin worlds/ashfall.db skill-mappings reset-all

# Difficulty targets
python -m harsh_realm.admin worlds/ashfall.db difficulties list
python -m harsh_realm.admin worlds/ashfall.db difficulties set hard --target 11

# Disposition outcomes
python -m harsh_realm.admin worlds/ashfall.db disposition list
python -m harsh_realm.admin worlds/ashfall.db disposition set exceptional_failure --delta -3

# Encounter weights
python -m harsh_realm.admin worlds/ashfall.db encounter-weights list
python -m harsh_realm.admin worlds/ashfall.db encounter-weights set hostile patrol_hostile --modifier 6
python -m harsh_realm.admin worlds/ashfall.db encounter-weights reset hostile

# Faction assets
python -m harsh_realm.admin worlds/ashfall.db faction-assets list
python -m harsh_realm.admin worlds/ashfall.db faction-assets get Warriors
python -m harsh_realm.admin worlds/ashfall.db faction-assets set Warriors --max-hp 6

# Random tables
python -m harsh_realm.admin worlds/ashfall.db tables list
python -m harsh_realm.admin worlds/ashfall.db tables list --category encounter
python -m harsh_realm.admin worlds/ashfall.db tables show encounters_forest
python -m harsh_realm.admin worlds/ashfall.db tables reset encounters_forest

# Utility
python -m harsh_realm.admin worlds/ashfall.db export-config > my_world_config.json
python -m harsh_realm.admin worlds/ashfall.db seed-from-yaml   # re-seed all (DANGER: overwrites)
```

**Output format:** Tables for `list` commands, pretty-printed JSON for `get`/`show`. Confirmation prompt before destructive operations (`reset-all`, `seed-from-yaml`).

**Tests:**
- `test_admin_cli.py` — invoke CLI via `subprocess` or `click.testing.CliRunner`
- `list` commands produce correct column headers
- `set` followed by `get` returns updated value
- `reset` followed by `get` returns YAML default
- Missing world file → clear error message

---

## Section E — In-Game Admin Commands

**File:** `src/harsh_realm/parser/commands.py` (extend)

Admin commands available in-game. Gated behind `config.admin_mode = true` in `config.yaml` — off by default so they can't fire accidentally during play.

```
admin list mappings
admin show mapping convince
admin set mapping convince Talk CHA 8
admin reset mapping convince
admin reset all mappings

admin list difficulties
admin set difficulty hard 11

admin list dispositions
admin set disposition exceptional_failure -3

admin list encounter-weights
admin set encounter-weight hostile patrol_hostile 6

admin list tables
admin show table encounters_forest
admin reset table encounters_forest

admin export config
```

**GM responses:** Admin commands produce formatted table output in the game log (same channel as normal output, but prefixed with `[ADMIN]` in a distinct color/style). They do NOT fire game events — they are out-of-band.

**Tests:**
- Admin commands rejected when `admin_mode = false` with message "Admin mode is not enabled."
- Admin commands fire no `GameEvent` entries
- `admin set mapping` → subsequent social check uses updated mapping

---

## Section F — Vue `/admin` Panel

**Files:** `frontend/src/views/AdminView.vue` + component files  
**Route:** `/admin` (separate from main game at `/`)

### World Selection

The admin panel defaults to the currently loaded game world but can target any world file independently. This is a first-class feature — not an afterthought.

**`admin.ts` Pinia store:**
```typescript
interface AdminStore {
  activeWorldPath: string        // defaults to gameStore.currentWorldPath on mount
  availableWorlds: WorldMeta[]   // fetched from GET /api/worlds on mount
  isDirty: boolean               // unsaved changes in current tab
}
```

On mount: `activeWorldPath` is set from `gameStore.currentWorldPath`. If no game world is loaded, defaults to the first world in `availableWorlds`, or shows an empty state.

Changing the world selector triggers a full reload of all tab data. If `isDirty` is true, show a "You have unsaved changes — leave anyway?" confirmation before switching.

**API convention:** Every `/api/admin/*` endpoint accepts a `?world=<path>` query parameter specifying which `.db` file to operate on. If omitted, falls back to the currently loaded world. The backend opens the specified world file for that request only — no persistent world switching on the backend.

```python
# Example route signature
@router.get("/skill-mappings")
async def list_skill_mappings(world: str = Query(default=None)):
    db = get_world_db(world or get_current_world())
    ...
```

### Layout

```
┌──────────────────────────────────────────────────────────────┐
│  ⚙ Admin   World: [ashfall.db          ▼]   [Export Config]  │
│            ● Currently loaded world                          │
├──────────────────────────────────────────────────────────────┤
│  Skill Mappings | Difficulties | Disposition |               │
│  Encounter Weights | Faction Assets | Tables                 │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  [Tab content — see below]                                   │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

The "● Currently loaded world" indicator appears when the selected world matches the active game world. Disappears (no indicator) when editing a different world — makes it visually obvious you're editing a world that isn't currently running.

### Tab: Skill Mappings

Inline-editable table. Each row has Save and Reset buttons.

```
Verb         Skill    Attr   Difficulty  Opposed   Actions
──────────────────────────────────────────────────────────
convince     [Talk▼]  [CHA▼] [8      ]  [✓]       [Save] [Reset]
intimidate   [Talk▼]  [STR▼] [10     ]  [✓]       [Save] [Reset]
deceive      [Talk▼]  [CHA▼] [10     ]  [✓]       [Save] [Reset]
bribe        [Trade▼] [CHA▼] [8      ]  [ ]       [Save] [Reset]
connect      [Conn▼]  [CHA▼] [8      ]  [ ]       [Save] [Reset]
                                                [Reset All]
```

- Skill and Attribute are `<select>` dropdowns populated from `skills.yaml` / attribute list
- Difficulty is a number input (min 4, max 20)
- Opposed is a checkbox
- Save button calls `PUT /api/admin/skill-mappings/{verb}`
- Reset button calls `POST /api/admin/skill-mappings/{verb}/reset` with confirmation dialog
- "Reset All" button with double-confirm

### Tab: Difficulties

```
Name         Target   Description                        Actions
──────────────────────────────────────────────────────────────────
trivial      [4    ]  Anyone could do this...            [Save] [Reset]
routine      [8    ]  Standard difficulty...             [Save] [Reset]
challenging  [10   ]  Requires real skill...             [Save] [Reset]
hard         [12   ]  Even skilled characters...         [Save] [Reset]
```

### Tab: Disposition Outcomes

```
Outcome Key          Delta    Description                 Actions
────────────────────────────────────────────────────────────────────
exceptional_failure  [-2   ]  Large disposition penalty   [Save] [Reset]
failure              [-1   ]  Small disposition penalty   [Save] [Reset]
bare_success         [0    ]  Goal achieved...            [Save] [Reset]
solid_success        [+1   ]  Goal achieved, NPC warms    [Save] [Reset]
exceptional_success  [+2   ]  Goal achieved, impressed    [Save] [Reset]
intimidate_success   [-1   ]  Always costs goodwill       [Save] [Reset]
deceive_caught       [-3   ]  NPC knows they were lied to [Save] [Reset]
```

### Tab: Encounter Weights

Matrix display: rows = dispositions, columns = encounter tags. Cell = weight modifier.

```
                    patrol_friendly  patrol_hostile  bounty_hunter  ambush  ...
────────────────────────────────────────────────────────────────────────────────
allied              [+3]             [-999]           —              —
friendly            —                [-2]             —              —
neutral             —                —                —              —
unfriendly          —                [+2]             —              —
hostile             —                [+4]             [+2]           [+1]
```

Click a cell to edit inline. Empty = no modifier (not stored in DB). Save on blur.

### Tab: Faction Assets

Table matching `faction_asset_stats` schema. Sortable by category. Each row inline-editable with Save/Reset.

### Tab: Random Tables

Two-panel layout:
- Left: table list (filterable by category)
- Right: selected table entries with weight + result JSON

Entry editing: click a row to edit weight or result. Result is a raw JSON textarea (simple, not a form). Add/remove entries with +/− buttons.

"Reset Table" button re-seeds from YAML with confirmation.

### Frontend Components

```
frontend/src/views/AdminView.vue          # top-level tab container
frontend/src/components/admin/
  SkillMappingsTab.vue
  DifficultiesTab.vue
  DispositionOutcomesTab.vue
  EncounterWeightsTab.vue
  FactionAssetsTab.vue
  RandomTablesTab.vue
  AdminTable.vue                          # shared reusable inline-edit table
  ConfirmResetDialog.vue                  # shared reset confirmation modal
frontend/src/stores/admin.ts              # Pinia store for admin state
```

**Tests (Playwright):**
- Navigate to `/admin` → all tabs visible
- Edit skill mapping → save → refresh page → value persists
- Reset skill mapping → value returns to YAML default
- Edit table entry weight → save → table entry weight updated

---

## Section G — World Creation Seeding

**File:** `src/harsh_realm/db.py` (extend `WorldDatabase.create()`)

At world creation, `AdminService.seed_all_from_yaml()` is called automatically:

```python
@classmethod
def create(cls, path: str, name: str, settings: dict) -> "WorldDatabase":
    db = cls._init_schema(path, name, settings)
    admin = AdminService(db)
    admin.seed_all_from_yaml()   # populate all config tables from YAML
    return db
```

**Tests:**
- Fresh world creation → all config tables populated
- Row count in `skill_mappings` matches YAML entry count
- `difficulty_targets` values match YAML exactly

---

## Estimated Effort

| Section | Hours |
|---|---|
| A. Config tables + YAML seed files | 3 |
| B. AdminService + tests | 3 |
| C. REST API routes + tests | 2 |
| D. CLI script + tests | 2 |
| E. In-game commands | 1 |
| F. Vue /admin panel | 4 |
| G. World creation seeding | 1 |
| **Total** | **~16 hours** |

---

## Acceptance Tests

1. **Fresh world:** Create world → `admin list mappings` in CLI → all 7 verbs present with YAML defaults
2. **Set via CLI:** `admin set mapping convince Talk CHA 10` → start game → convince NPC → check uses difficulty 10
3. **Set via in-game:** `admin set mapping intimidate Talk STR 12` → subsequent intimidate uses updated values
4. **Reset via CLI:** `admin reset mapping convince` → value returns to YAML default (difficulty 8)
5. **Vue panel:** Open `/admin` → edit `deceive` difficulty to 14 → save → refresh → value is 14
6. **Vue reset:** Reset `deceive` → value returns to 10 (YAML default)
7. **Export:** `admin export-config` → JSON file contains all 6 categories with current world values
8. **Isolation:** Change skill mapping in `worlds/ashfall.db` → open `worlds/other_world.db` → mapping unchanged
9. **World selector:** Open `/admin` with `ashfall.db` loaded → "● Currently loaded world" indicator visible → switch selector to `other_world.db` → indicator disappears → tab data reloads showing other world's values → edit + save → switch back to `ashfall.db` → original values unchanged
10. **Dirty guard:** Edit a value without saving → change world selector → "unsaved changes" confirmation appears
9. **Admin mode gate:** `admin_mode = false` in config → `admin set mapping convince...` → "Admin mode is not enabled"
10. **Seeding:** Delete `skill_mappings` table rows → call `seed_all_from_yaml()` → table fully repopulated
