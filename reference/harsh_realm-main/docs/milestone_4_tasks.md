# Milestone 4: People — Task Specification

> **Goal:** Social encounters, full WWN faction system, Mythic GME oracle (including Adventure Crafter), shopping, Expert class ability, and a data-driven admin system for game configuration.
> **Estimated time:** 4–5 weeks (AI-assisted development)
> **Prerequisite:** Milestone 3 complete. Read CLAUDE.md, AGENTS.md, and all docs in `docs/rules_reference/` including the three new M4 docs before starting.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green.
2. `talk <npc>` while NPC is in same hex enters social scene. NPC has UNE-generated personality. Player can `convince`, `intimidate`, `deceive`, `bribe`, `connect`, `ask`. Each resolves as a skill check with disposition change and GM narration.
3. Disposition hitting -3 mid-conversation transitions to combat scene automatically.
4. Faction turns fire automatically when the world clock advances a week. Each faction takes an action (attack, expand, create asset, repair, harvest, refit). Events narrated to player if significant and nearby.
5. Faction disposition toward player modifies encounter tables in that faction's territory.
6. `oracle <question> (<likelihood>)` resolves via the Mythic fate chart. Chaos factor adjusts based on outcomes.
7. Scene checks fire on scene transitions. Interrupts generate random events.
8. `add thread <title>`, `resolve thread <id>`, `list threads` manage adventure threads.
9. `create plotline <title> <theme>` starts an Adventure Crafter plotline. Scenes advance through it.
10. `shop` in a settlement opens a store. `list`, `buy <item>`, `sell <item>`, `examine <item>` all work. Inventory and gold update correctly.
11. Expert class: after a failed skill check, player is prompted to use their reroll ability. Reroll available once per scene.
12. `admin list mappings` (CLI) shows all skill mappings. `admin set mapping convince Talk CHA 8` updates it. Subsequent social checks use the updated mapping.
13. `/admin` route in frontend shows tabbed admin panel. Skill mappings tab shows all verbs with inline editing. Save and reset work.
14. All config tables (skill_mappings, difficulty_targets, disposition_outcomes, encounter_weights, faction_asset_stats) are seeded from YAML at world creation.

---

## Task 4.0: M3 Cleanup

> **What:** Finish the two partial M3 tasks before building M4 features.
> **Estimated time:** 2–3 hours

**Files:**
- `src/harsh_realm/gm/scenes/exploration.py` — add `take` command
- `src/harsh_realm/gm/scenes/respawn.py` — wire new-character option
- `src/harsh_realm/engine/healing.py` — wire rest interruption
- `src/harsh_realm/gm/scenes/exploration.py` — wire healer NPC flow

**Deliverables:**

**3.8 gap — `take` command and new-character-on-death:**
- Add `take <item>` command to exploration scene. If player is at a hex with a death marker, items listed at that hex can be retrieved. Items transfer to player inventory; death marker removed when all items retrieved.
- Wire new-character option in respawn scene. When player dies, prompt: "Respawn as Kira Voss with penalties? (yes) / Start a new character? (new)". `new` → transition to CharacterCreation scene. New character starts at the world's starting settlement.

**3.11 gap — rest interruption and healer NPC:**
- Wire rest interruption: when player issues `rest` command, each rest tick rolls an encounter check (same logic as hex entry). If hostile encounter rolls, interrupt rest, narrate interruption, transition to appropriate scene.
- Wire healer NPC: in exploration scene, if player types `talk to <npc>` and the NPC has `occupation: healer` in their data, transition to a `HealerInteraction` flow: greet → offer healing for fee → player can accept or decline → if accept, deduct gold, restore HP to max.

**Tests:**
- `take` at non-death hex → "Nothing here to take."
- `take <item>` at death hex → item moves to inventory, death marker updated.
- All items retrieved → death marker removed from hex.
- Death → choose `new` → CharacterCreation scene begins.
- `rest` for 5 ticks with hostile encounter table → at least 1 interruption in 20 test runs (probabilistic).
- `talk to healer` → healing offered → accept → gold deducted, HP restored.
- `talk to healer` with insufficient gold → "You can't afford that."

**Acceptance:** All M3 acceptance criteria now fully satisfied. `pytest` still green.

---

## Task 4.1: Config Tables & YAML Seeds

> **What:** Create all editable config YAML files and the SQLite tables they seed. These tables are the data foundation everything in M4 reads from.
> **Estimated time:** 3 hours

**Files:**
- `data/skill_mappings.yaml` (NEW)
- `data/difficulty_targets.yaml` (NEW)
- `data/disposition_outcomes.yaml` (NEW)
- `data/encounter_weights.yaml` (NEW)
- `data/faction_assets.yaml` (NEW)
- `src/harsh_realm/db.py` — extend `WorldDatabase.create()` to seed config tables
- `src/harsh_realm/admin/service.py` (NEW) — `AdminService` class

**Deliverables:**

`data/skill_mappings.yaml` — 7 entries:
```yaml
- verb: convince
  skill: Talk
  attribute: CHA
  base_difficulty: 8
  opposed: true
  description: "Persuade an NPC to agree with you or do something."

- verb: intimidate
  skill: Talk
  attribute: STR
  base_difficulty: 10
  opposed: true
  description: "Coerce through threat of force."

- verb: deceive
  skill: Talk
  attribute: CHA
  base_difficulty: 10
  opposed: true
  description: "Lie to an NPC."

- verb: bribe
  skill: Trade
  attribute: CHA
  base_difficulty: 8
  opposed: false
  description: "Offer payment to get what you want."

- verb: connect
  skill: Connect
  attribute: CHA
  base_difficulty: 8
  opposed: false
  description: "Leverage social network or contacts."

- verb: ask
  skill: Talk
  attribute: CHA
  base_difficulty: 6
  opposed: false
  description: "Ask a direct question."

- verb: perform
  skill: Perform
  attribute: CHA
  base_difficulty: 8
  opposed: false
  description: "Entertain or impress through performance."
```

`data/difficulty_targets.yaml` — 6 entries (trivial 4, routine 8, challenging 10, hard 12, formidable 14, heroic 16).

`data/disposition_outcomes.yaml` — 7 entries mapping outcome keys to disposition deltas. Special entries: `intimidate_success` (delta -1 even on success), `deceive_caught` (delta -3 on fail by 3+).

`data/encounter_weights.yaml` — weight modifiers per faction disposition × encounter tag.

`data/faction_assets.yaml` — full WWN asset list. See `docs/rules_reference/faction_turns.md` for complete asset definitions.

**SQLite schema additions** (add to `WorldDatabase._init_schema()`):
```sql
CREATE TABLE skill_mappings (
    verb            TEXT PRIMARY KEY,
    skill           TEXT NOT NULL,
    attribute       TEXT NOT NULL,
    base_difficulty INTEGER NOT NULL DEFAULT 8,
    opposed         INTEGER DEFAULT 0,
    description     TEXT
);

CREATE TABLE difficulty_targets (
    name        TEXT PRIMARY KEY,
    target      INTEGER NOT NULL,
    description TEXT
);

CREATE TABLE disposition_outcomes (
    outcome_key TEXT PRIMARY KEY,
    delta       INTEGER NOT NULL,
    description TEXT
);

CREATE TABLE encounter_weights (
    faction_disposition TEXT NOT NULL,
    encounter_tag       TEXT NOT NULL,
    weight_modifier     INTEGER NOT NULL,
    PRIMARY KEY (faction_disposition, encounter_tag)
);

CREATE TABLE faction_asset_stats (
    asset_type    TEXT PRIMARY KEY,
    category      TEXT NOT NULL,
    min_attribute INTEGER NOT NULL,
    cost          INTEGER NOT NULL,
    upkeep        INTEGER DEFAULT 0,
    max_hp        INTEGER NOT NULL,
    attack_stat   TEXT,
    counter_stat  TEXT,
    attack_roll   TEXT,
    special       TEXT,
    description   TEXT
);

CREATE TABLE threads (
    id       TEXT PRIMARY KEY,
    type     TEXT NOT NULL,
    title    TEXT NOT NULL,
    status   TEXT DEFAULT 'active',
    progress INTEGER DEFAULT 0,
    data     TEXT DEFAULT '{}'
);

CREATE TABLE oracle_npcs (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    status    TEXT DEFAULT 'active',
    notes     TEXT,
    entity_id TEXT REFERENCES entities(id)
);

CREATE TABLE plotlines (
    id     TEXT PRIMARY KEY,
    title  TEXT NOT NULL,
    theme  TEXT NOT NULL,
    status TEXT DEFAULT 'active',
    scenes TEXT DEFAULT '[]',
    data   TEXT DEFAULT '{}'
);
```

`AdminService` in `src/harsh_realm/admin/service.py`:
- `seed_all_from_yaml(self) -> SeedResult` — reads all 5 YAML files, populates all 5 config tables. Called by `WorldDatabase.create()`.
- `list_skill_mappings(self) -> list[SkillMapping]`
- `get_skill_mapping(self, verb: str) -> SkillMapping | None`
- `set_skill_mapping(self, verb, skill, attribute, base_difficulty, opposed, description) -> SkillMapping`
- `reset_skill_mapping(self, verb: str) -> SkillMapping` — re-reads YAML, overwrites row
- `reset_all_skill_mappings(self) -> list[SkillMapping]`
- (same pattern for difficulty_targets, disposition_outcomes, encounter_weights, faction_asset_stats)
- `export_world_config(self) -> dict` — returns all 5 config tables as a single dict

**Tests:** `tests/test_admin_service.py`
- Fresh world → `skill_mappings` table has 7 rows matching YAML
- `set_skill_mapping("convince", "Talk", "CHA", 10, True, "")` → row updated in DB
- `get_skill_mapping("convince").base_difficulty == 10`
- `reset_skill_mapping("convince")` → difficulty returns to 8
- `reset_all_skill_mappings()` → all rows match YAML defaults
- `seed_all_from_yaml()` on empty tables → all 5 tables populated
- Setting faction asset below valid range raises `ValidationError`

**Acceptance:** `WorldDatabase.create()` produces a world with all 5 config tables seeded. `pytest tests/test_admin_service.py` passes.

---

## Task 4.2: Admin REST API & CLI

> **What:** REST endpoints and CLI script exposing `AdminService`. Both thin wrappers over the same service.
> **Estimated time:** 4 hours

**Files:**
- `src/harsh_realm/api/admin_routes.py` (NEW)
- `src/harsh_realm/admin/cli.py` (NEW)
- `src/harsh_realm/main.py` — mount admin router
- `src/harsh_realm/parser/commands.py` — add `admin` command group

**Deliverables:**

REST router at `/api/admin`. All endpoints accept `?world=<path>` query param; if omitted, uses currently loaded world.

Endpoints (implement for all 5 config categories — shown for skill_mappings, same pattern for others):
```
GET    /api/admin/skill-mappings
GET    /api/admin/skill-mappings/{verb}
PUT    /api/admin/skill-mappings/{verb}
POST   /api/admin/skill-mappings/{verb}/reset
POST   /api/admin/skill-mappings/reset-all
GET    /api/admin/worlds                         → list .db files in worlds/
POST   /api/admin/worlds/{name}/clone            → SQLite backup API
GET    /api/admin/worlds/{name}/export           → stream ZIP (db + config_snapshot.json + metadata.json)
DELETE /api/admin/worlds/{name}                  → delete file, confirm if loaded
GET    /api/admin/yaml-files                     → directory listing of data/
GET    /api/admin/yaml-files/{path}              → download raw file
POST   /api/admin/yaml-files/{path}              → upload replacement (validate YAML before write)
GET    /api/admin/world-meta                     → all world_meta key/value pairs
PUT    /api/admin/world-meta/{key}               → set one key
POST   /api/admin/export-config                  → full config dict as JSON download
```

CLI entry point (`python -m harsh_realm.admin <world_path> <command> [args]`):
```bash
python -m harsh_realm.admin worlds/ashfall.db skill-mappings list
python -m harsh_realm.admin worlds/ashfall.db skill-mappings get convince
python -m harsh_realm.admin worlds/ashfall.db skill-mappings set convince --skill Talk --attr CHA --difficulty 8 --opposed
python -m harsh_realm.admin worlds/ashfall.db skill-mappings reset convince
python -m harsh_realm.admin worlds/ashfall.db skill-mappings reset-all
python -m harsh_realm.admin worlds/ashfall.db difficulties list
python -m harsh_realm.admin worlds/ashfall.db difficulties set hard --target 11
python -m harsh_realm.admin worlds/ashfall.db disposition list
python -m harsh_realm.admin worlds/ashfall.db encounter-weights list
python -m harsh_realm.admin worlds/ashfall.db faction-assets list
python -m harsh_realm.admin worlds/ashfall.db export-config > config.json
python -m harsh_realm.admin worlds/ashfall.db seed-from-yaml   # confirmation prompt
```

In-game admin commands (gated by `config.admin_mode = true`):
```
admin list mappings
admin show mapping <verb>
admin set mapping <verb> <skill> <attr> <difficulty>
admin reset mapping <verb>
admin reset all mappings
admin list difficulties
admin set difficulty <name> <target>
admin list dispositions
admin set disposition <key> <delta>
admin export config
```

Admin commands output prefixed `[ADMIN]` in game log. Fire no `GameEvent` entries.

**Tests:** `tests/test_admin_routes.py`, `tests/test_admin_cli.py`
- `PUT /api/admin/skill-mappings/convince` with `base_difficulty: 12` → `GET` returns 12
- `POST /api/admin/skill-mappings/convince/reset` → `GET` returns 8
- CLI `set` followed by `get` returns updated value
- CLI `reset` followed by `get` returns YAML default
- Missing world file → clear error message
- Upload malformed YAML to `/api/admin/yaml-files/data/skill_mappings.yaml` → 422, file unchanged
- Upload valid YAML → file on disk updated
- `admin set mapping convince Talk CHA 12` in-game with `admin_mode=false` → "Admin mode is not enabled."

**Acceptance:** `python -m harsh_realm.admin worlds/ashfall.db skill-mappings list` shows 7 verbs. REST endpoints tested via `TestClient`. In-game admin commands blocked without `admin_mode`.

---

## Task 4.3: Vue `/admin` Panel

> **What:** Frontend admin panel at `/admin` route. Tabbed layout covering all editable config categories. Extends from M4 design spec at `docs/design/milestone_4_admin_spec.md`.
> **Estimated time:** 4 hours

**Files:**
- `frontend/src/views/AdminView.vue` (NEW)
- `frontend/src/components/admin/AdminWorldSelector.vue` (NEW)
- `frontend/src/components/admin/SkillMappingsTab.vue` (NEW)
- `frontend/src/components/admin/DifficultiesTab.vue` (NEW)
- `frontend/src/components/admin/DispositionOutcomesTab.vue` (NEW)
- `frontend/src/components/admin/EncounterWeightsTab.vue` (NEW)
- `frontend/src/components/admin/FactionAssetsTab.vue` (NEW)
- `frontend/src/components/admin/RandomTablesTab.vue` (NEW)
- `frontend/src/components/admin/AdminTable.vue` (NEW — shared inline-edit table)
- `frontend/src/components/admin/ConfirmResetDialog.vue` (NEW)
- `frontend/src/stores/admin.ts` (NEW)
- `frontend/src/router/index.ts` — add `/admin` route

**Deliverables:**

`admin.ts` Pinia store:
```typescript
interface AdminStore {
  activeWorldPath: string       // defaults to gameStore.currentWorldPath on mount
  availableWorlds: WorldMeta[]  // from GET /api/worlds
  isDirty: boolean
  activeTab: string
  pendingHexSelection: {q: number, r: number} | null  // for cross-tab linking (M4.5)
}
```

`AdminView.vue` — header with world selector + tabs:
```
┌──────────────────────────────────────────────────────────────┐
│  ⚙ Admin   World: [ashfall.db ▼] ● loaded   [Export Config] │
├──────────────────────────────────────────────────────────────┤
│  Skill Mappings | Difficulties | Disposition |               │
│  Encounter Weights | Faction Assets | Tables                 │
└──────────────────────────────────────────────────────────────┘
```

World selector: dropdown lists all `.db` files from `GET /api/worlds`. Changing selection reloads all tab data. If `isDirty`, show "Unsaved changes — leave anyway?" before switching. "● loaded" indicator when selected world matches active game world.

`SkillMappingsTab.vue` — inline-editable table:
- Columns: Verb, Skill (dropdown from skills list), Attribute (dropdown: STR/DEX/CON/INT/WIS/CHA), Difficulty (number 4–20), Opposed (checkbox), [Save] [Reset]
- "Reset All" button with double-confirm dialog
- Skill and attribute dropdowns populated from `data/skills.yaml` keys

`DifficultiesTab.vue` — inline table: Name, Target (number), Description, [Save] [Reset]

`DispositionOutcomesTab.vue` — inline table: Outcome Key, Delta (number -5 to +5), Description, [Save] [Reset]

`EncounterWeightsTab.vue` — matrix display: rows = dispositions, columns = encounter tags. Cells inline-editable. Empty cell = no modifier (row not stored). Save on blur.

`FactionAssetsTab.vue` — inline table matching `faction_asset_stats` schema. Sortable by category.

`RandomTablesTab.vue` — two-panel: left = table list (filterable by category), right = selected table entries (weight + result JSON). Add/remove entry buttons. "Reset Table" with confirmation.

**Tests:** (Playwright E2E or manual verification)
- Navigate to `/admin` → all 6 tabs visible
- Change world selector while dirty → confirmation appears
- Edit convince difficulty → Save → refresh → value persists
- Reset convince → value returns to 8
- "● loaded" indicator appears on active world, disappears on other worlds

**Acceptance:** `/admin` loads, all tabs render, skill mapping edit + save + reset round-trip correctly.

---

## Task 4.4: UNE Personality System

> **What:** Verbatim UNE tables as YAML, generator class, NPC personality storage.
> **Estimated time:** 4 hours
> **Read first:** `docs/rules_reference/social.md` (UNE section)

**Files:**
- `data/tables/npc/une_power_level.yaml` (NEW)
- `data/tables/npc/une_descriptors.yaml` (NEW)
- `data/tables/npc/une_motivation_verbs.yaml` (NEW)
- `data/tables/npc/une_motivation_nouns.yaml` (NEW)
- `data/tables/npc/une_bearings.yaml` (NEW)
- `data/tables/npc/une_moods.yaml` (NEW)
- `src/harsh_realm/engine/npc_personality.py` (NEW)
- `src/harsh_realm/models/npc.py` — extend entity data JSON schema

**Deliverables:**

All 6 UNE YAML files encoded verbatim from rulebook. Entry counts: power_level=7, descriptors=100, motivation_verbs=100, motivation_nouns=100, bearings=8×5 sub-entries, moods=7.

`NPCPersonality` frozen dataclass:
```python
@dataclass(frozen=True)
class NPCPersonality:
    power_level: str
    descriptor: str
    motivation_verb: str
    motivation_noun: str
    bearing: str
    bearing_focus: str
    base_disposition: int = 0
```

`UNEGenerator` class:
```python
class UNEGenerator:
    def generate_personality(self, power_level: str | None = None) -> NPCPersonality: ...
    def generate_motivation(self) -> tuple[str, str]: ...
    def generate_bearing(self, chaos_factor: int, relationship: str) -> tuple[str, str]: ...
```

Entity data JSON extended with `une_personality` block:
```json
{
  "une_personality": {
    "power_level": "average",
    "descriptor": "scheming",
    "motivation_verb": "advance",
    "motivation_noun": "wealth",
    "bearing": "scheming",
    "bearing_focus": "future action",
    "base_disposition": 0
  }
}
```

NPCs with no `une_personality` in data get one generated and persisted on first `talk` command.

`DispositionSystem` in same file:
```python
DISPOSITION_LABELS = {-3: "Hostile", -2: "Unsteady", -1: "Guarded",
                       0: "Indifferent", 1: "Sociable", 2: "Friendly", 3: "Helpful"}

def score_to_label(score: int) -> str: ...
def clamp(score: int) -> int: ...  # clamps to [-3, 3]
```

`DispositionChangeEvent(entity_id, old_score, new_score, reason)` added to event types.

**Tests:** `tests/test_npc_personality.py`
- `generate_personality()` called 100 times → all results have valid fields within table bounds
- Power level entry count = 7
- Descriptors entry count = 100
- Motivation verbs and nouns entry count = 100 each
- NPC entity round-trips through SQLite with `une_personality` intact
- NPC without personality → `talk` → personality generated and persisted
- `score_to_label(-3) == "Hostile"`, `score_to_label(3) == "Helpful"`
- Disposition clamped: `clamp(5) == 3`, `clamp(-5) == -3`

**Acceptance:** UNE tables load, `generate_personality()` works, NPCs get personalities on first contact.

---

## Task 4.5: Social Scene

> **What:** Full social scene state handler — entry/exit, valid commands, skill check resolution, narration.
> **Estimated time:** 8 hours
> **Read first:** `docs/rules_reference/social.md` (full doc)

**Files:**
- `src/harsh_realm/gm/scenes/social.py` (NEW)
- `src/harsh_realm/gm/controller.py` — add Social scene transitions
- `src/harsh_realm/parser/commands.py` — add social verbs
- `src/harsh_realm/engine/skill_checks.py` — extend with social check resolver

**Deliverables:**

`SocialSceneHandler` implementing `SceneHandler` protocol.

**Entry triggers:**
- `talk <npc>` while NPC is in same hex/location → enter Social scene
- GM auto-triggers when player encounters a non-hostile NPC (disposition ≥ -1) during encounter resolution

Entry blocked if NPC disposition is -3 (Hostile). GM narrates: "[NPC] isn't interested in talking."

**Exit triggers:**
- Player types `leave` or `goodbye` → return to Exploration
- Disposition drops to -3 mid-conversation → GM narrates hostility escalation → transition to Combat
- Scene check fires interrupt (wired in Task 4.8) → exit per interrupt result

**Valid commands in Social scene:**
`ask`, `convince`, `intimidate`, `deceive`, `bribe`, `connect`, `perform`, `leave`, `goodbye`, `oracle`, `status`, `inventory`, `help`

`attack` is NOT valid in Social — player must `leave` first. If player types `attack` in social scene: "You'll need to end the conversation first. Type `leave` if you want to fight."

`get_prompt()` output includes:
- NPC name and power level flavor text
- Current bearing sub-entry (what the NPC is doing/saying)
- NPC's disposition label

**Social skill check resolution** (`SocialCheckResolver`):

1. Look up verb in `skill_mappings` table (reads SQLite, not hardcoded)
2. Get character's skill level and attribute modifier
3. If `opposed=true`, generate NPC resistance modifier from WIS modifier
4. Roll `2d6 + skill_level + attr_mod - npc_resistance_mod` vs `base_difficulty`
5. Calculate margin, look up outcome in `disposition_outcomes` table
6. Apply special cases: `intimidate_success` always costs -1 disposition even on success; `deceive_caught` (fail by 3+) applies `deceive_caught` delta instead of standard failure delta
7. Update NPC disposition in DB, fire `DispositionChangeEvent`
8. Fire `action.skill_check` event with full roll details
9. Fire `social.dialogue` event with outcome

Outcome margin → disposition lookup:
```
margin ≤ -4: exceptional_failure
margin -3 to -1: failure
margin 0–1: bare_success
margin 2–3: solid_success
margin ≥ 4: exceptional_success
```

Narrator produces contextual text per NPC bearing + outcome. At minimum 2 variants per bearing type × outcome combination (use `data/templates/social_narration.yaml`).

**Tests:** `tests/test_social_scene.py`, `tests/test_social_checks.py`
- `talk bandit` with bandit disposition -3 → entry blocked, narration produced
- `talk merchant` with merchant disposition 0 → social scene entered
- NPC gains UNE personality if missing → personality persisted after entry
- `convince` → skill check → disposition changes per outcome band
- Exceptional failure (margin ≤ -4) → disposition -2
- `intimidate` success → disposition -1 (special case)
- `deceive` fail by 3+ → disposition -3 (caught lying)
- Three consecutive intimidate failures → disposition hits -3 → scene transitions to Combat
- `leave` → return to Exploration scene
- `attack` in social → error message, scene not changed
- Skill mapping changes via AdminService take effect immediately on next check

**Content stub needed:**
- `data/templates/social_narration.yaml` — at minimum 2 narration variants per bearing × outcome. Mark as `# DEVELOPER: expand with more variants`.

**Acceptance:** Full social flow works. Disposition changes, exits to combat, skill mappings are data-driven.

---

## Task 4.6: Faction System — Data & Turn Engine

> **What:** WWN faction turns — full asset system, faction AI, weekly tick.
> **Estimated time:** 14 hours
> **Read first:** `docs/rules_reference/faction_turns.md` (full doc)

**Files:**
- `data/factions/` (NEW directory — starting faction YAML files)
- `src/harsh_realm/faction/faction_turn.py` (NEW)
- `src/harsh_realm/faction/faction_ai.py` (NEW)
- `src/harsh_realm/faction/assets.py` (NEW)
- `src/harsh_realm/db.py` — extend `FactionRepository`
- `src/harsh_realm/engine/world_clock.py` (NEW or extend existing)

**Deliverables:**

`data/factions/` — minimum 3 starting factions for the world, each in a separate YAML file:
```yaml
# data/factions/iron_pact.yaml
id: iron_pact
name: The Iron Pact
hp: 7
max_hp: 7
force: 3
cunning: 2
wealth: 4
xp: 0
home_hex: {q: 0, r: 0}  # near starting area
goals:
  - "Expand territory into the northern ruins"
  - "Eliminate the Scavenger Brotherhood"
tags: [military, feudal, hostile_to_players]
starting_assets:
  - type: Warriors
    hp: 6
    location: {q: 0, r: 0}
  - type: Informers
    hp: 3
    location: {q: 2, r: -1}
relationships:
  - faction: merchant_guild
    disposition: unfriendly
  - faction: scavenger_brotherhood
    disposition: hostile
```

`FactionRepository` in `db.py`:
```python
class FactionRepository:
    async def get_faction(self, faction_id: str) -> Faction | None: ...
    async def list_factions(self) -> list[Faction]: ...
    async def update_faction_hp(self, faction_id: str, delta: int) -> Faction: ...
    async def get_faction_assets(self, faction_id: str) -> list[FactionAsset]: ...
    async def update_asset_hp(self, asset_id: str, delta: int) -> FactionAsset: ...
    async def remove_asset(self, asset_id: str) -> None: ...
    async def add_asset(self, faction_id: str, asset_type: str, location: tuple) -> FactionAsset: ...
    async def get_relationship(self, faction_a: str, faction_b: str) -> str: ...
    async def set_relationship(self, faction_a: str, faction_b: str, disposition: str) -> None: ...
    async def get_reputation(self, entity_id: str, faction_id: str) -> int: ...
    async def update_reputation(self, entity_id: str, faction_id: str, delta: int) -> int: ...
```

`FactionTurnEngine` in `faction/faction_turn.py`:
```python
class FactionTurnEngine:
    async def run_all_turns(self, world: WorldDatabase) -> list[GameEvent]: ...
    async def run_faction_turn(self, faction: Faction, world: WorldDatabase) -> FactionTurnResult: ...
```

All 7 WWN faction actions implemented per `docs/rules_reference/faction_turns.md`:
- **Attack:** Roll `attack_roll` vs defender's counter. On hit, defender loses HP. If asset reaches 0 HP, remove it; faction takes 1 HP.
- **Expand:** Move or place asset into adjacent/uncontrolled hex (cannot expand into hostile-controlled hex without attacking first).
- **Create Asset:** Spend faction XP equal to asset cost. Faction must meet `min_attribute` for the asset type.
- **Repair:** Restore asset HP equal to 1d6. Cost = ½ asset cost in faction XP.
- **Seize Territory:** Claim hex currently uncontrolled or from a losing faction.
- **Sell Asset:** Remove asset, recover ½ cost in faction XP.
- **Refit:** Faction recovers 1d6 HP (no other action this turn).
- **Harvest:** Faction gains 1d6 XP from Wealth assets.

`FactionAI` in `faction/faction_ai.py` — priority-based action selection:
```
1. Attack if: enemy asset in range AND faction HP > 50%
2. Repair if: any asset at < 50% HP AND XP ≥ repair cost
3. Create asset if: XP > threshold AND asset count < cap
4. Expand into adjacent neutral hexes
5. Harvest if: no better option
6. Refit if: faction HP < 50%
Allied factions never attack each other's assets.
Target lowest-HP enemy asset when attacking.
```

World clock (`engine/world_clock.py`):
- Track `current_week` in `gm_state` table
- `advance_time(ticks: int)` — advances clock, checks if week boundary crossed
- If week boundary crossed: call `FactionTurnEngine.run_all_turns()`
- Faction turns fire `world.faction_action` events for each action taken
- Significant events (asset destroyed, territory taken, conflict within 3 hexes of player) → narrated to player via GM

Reputation system:
- Killing faction member → `update_reputation(player_id, faction_id, -10)` + fire `reputation.change` event
- Completing faction task → `update_reputation(player_id, faction_id, +15)`
- Bribing faction member → `update_reputation(player_id, faction_id, +5)`

Reputation score → disposition label thresholds:
```
score ≤ -30: hostile
-29 to -10: unfriendly
-9 to +9:   neutral
+10 to +29: friendly
score ≥ +30: allied
```

Encounter weight modifier: in `EncounterGenerator`, look up player's reputation with controlling faction of current hex, apply modifiers from `encounter_weights` table.

**Tests:** `tests/test_faction_turns.py`, `tests/test_faction_ai.py`
- Run 4 faction turns: faction HP, asset HP, territory change coherently
- Asset at 0 HP removed; faction takes 1 HP damage
- Cannot create asset without meeting `min_attribute` requirement
- AI: damaged asset → prioritizes repair over expansion
- AI: 0 XP → cannot create, falls through to expand/harvest
- Allied factions never attack each other
- Advancing time 6 days → no faction turn
- Advancing time 7 days → faction turn fires
- Reputation -30 → disposition "hostile"; +30 → disposition "allied"
- Player in hostile faction's territory → encounter table has higher patrol_hostile weight

**Content stubs needed:**
- `data/factions/merchant_guild.yaml`
- `data/factions/scavenger_brotherhood.yaml`
- All marked `# DEVELOPER: expand stats from WWN faction creation tables`

**Acceptance:** Weekly tick fires faction turns. Faction actions logged. Encounter tables modified by faction reputation.

---

## Task 4.7: Mythic Oracle — Core GME

> **What:** Full Mythic GME replacing the M2 placeholder oracle. Fate chart, chaos factor, scene checks, random event tables, thread/NPC tracking.
> **Estimated time:** 10 hours
> **Read first:** `docs/rules_reference/oracle.md` (Mythic GME section)

**Files:**
- `src/harsh_realm/engine/oracle.py` — full replacement of M2 placeholder
- `data/tables/oracle/fate_chart.yaml` (NEW)
- `data/tables/oracle/event_focus.yaml` (NEW)
- `data/tables/oracle/event_action.yaml` (NEW)
- `data/tables/oracle/event_subject.yaml` (NEW)
- `src/harsh_realm/parser/commands.py` — add oracle/thread/npc commands

**Deliverables:**

`data/tables/oracle/fate_chart.yaml` — 9×9 matrix. 9 likelihood values × 9 chaos factors. Each cell: `{yes_threshold: int, exceptional_yes: int, exceptional_no: int}`. Encode verbatim from Mythic GME rulebook.

3 random event tables: `event_focus.yaml` (100 entries), `event_action.yaml` (100 entries), `event_subject.yaml` (100 entries). Verbatim from Mythic GME.

`OracleSystem` class (full replacement):
```python
class OracleSystem:
    async def fate_check(self, likelihood: str, world: WorldDatabase) -> FateResult: ...
    async def scene_check(self, world: WorldDatabase) -> SceneModification: ...
    async def random_event(self, world: WorldDatabase) -> RandomEvent: ...
    async def adjust_chaos(self, direction: int, world: WorldDatabase) -> int: ...
    async def get_chaos_factor(self, world: WorldDatabase) -> int: ...
```

`FateResult` dataclass: `{likelihood, chaos_factor, roll, result, exceptional}`. Results: "Exceptional Yes", "Yes", "No", "Exceptional No".

Scene check procedure:
- Roll d10 vs chaos factor
- `roll > chaos`: Scene proceeds as expected
- `roll ≤ chaos AND roll is odd`: Interrupt — generate random event
- `roll ≤ chaos AND roll is even`: Altered Scene — scene proceeds differently

`RandomEventGenerator.generate()` → rolls on focus, action, subject tables → returns `RandomEvent(focus, action, subject, description)`.

Chaos stored in `gm_state` table, key `oracle_chaos_factor`, default 5.

New commands:
```
oracle <question> (<likelihood>)    → fate check, print result
scene check                         → explicit scene check
add thread <title>                  → create thread record (type=story)
resolve thread <id>                 → mark thread resolved
list threads                        → show active threads
add npc <name>                      → add to oracle NPC list
remove npc <id>                     → remove from oracle NPC list
list npcs                           → show oracle NPC list
```

Scene check wired to GM Controller: fires automatically on every scene transition (hex entry, social entry, rest, combat end, etc.). Result logged to event log. Interrupts generate random event and narrate to player.

Chaos adjustment: player wins/controlled outcome → chaos -1; player loses/things go wrong → chaos +1. Chaos shown in `status` output.

**Tests:** `tests/test_oracle.py`
- Fate chart: LIKELY + chaos 5 → correct threshold (verify against rulebook)
- Roll 1–4 at highest probability → Exceptional Yes
- Roll 96–100 at lowest probability → Exceptional No
- Scene check at chaos 9: in 100 rolls, > 80 produce interrupt or altered scene
- Scene check at chaos 1: in 100 rolls, < 20 produce interrupt or altered scene
- Random event: 100 generations all produce valid focus/action/subject combinations
- Thread add → list shows it → resolve → no longer in active list
- Chaos adjusts correctly, clamped to [1, 9]
- `oracle "is there a guard?" (likely)` → produces a fate result with roll shown

**Acceptance:** `oracle` command works. Scene checks fire on transitions. Chaos factor tracks and adjusts. Threads and NPC lists persist.

---

## Task 4.8: Adventure Crafter

> **What:** Full Mythic Adventure Crafter — plotlines, themes, thread progression.
> **Estimated time:** 10 hours
> **Read first:** `docs/rules_reference/oracle.md` (Adventure Crafter section)

**Files:**
- `src/harsh_realm/engine/adventure_crafter.py` (NEW)
- `data/tables/oracle/ac_themes.yaml` (NEW)
- `data/tables/oracle/ac_characters.yaml` (NEW)
- `data/tables/oracle/ac_plots.yaml` (NEW)
- `src/harsh_realm/parser/commands.py` — add plotline commands

**Deliverables:**

3 Adventure Crafter YAML tables verbatim from rulebook:
- `ac_themes.yaml` — 5 themes (Action, Tension, Mystery, Social, Personal) with weighted sub-tables
- `ac_characters.yaml` — character element table
- `ac_plots.yaml` — plot element table

`AdventureCrafter` class:
```python
class AdventureCrafter:
    async def create_plotline(self, title: str, theme: str,
                              world: WorldDatabase) -> Plotline: ...
    async def generate_scene(self, plotline_id: str,
                             world: WorldDatabase) -> ACScene: ...
    async def advance_plotline(self, plotline_id: str,
                               world: WorldDatabase) -> PlotlineAdvancement: ...
    async def check_thread_progression(self, world: WorldDatabase) -> list[ThreadEvent]: ...
```

Plotlines stored in `plotlines` table (created in Task 4.1 schema).

Theme-weighted scene generation: Active plotline's theme biases scene element rolls toward that theme's sub-table.

Thread progression: completing scenes increments `threads.progress`. Threshold check per AC rules:
- Character thread: every 3 scene completions → resolution check
- Story thread: every 5 scene completions → resolution check
- Resolution check: roll d10 vs current progress — if roll ≤ progress, thread resolves

New commands:
```
create plotline <title> <theme>    → Action|Tension|Mystery|Social|Personal
list plotlines                     → show active plotlines
advance plotline <id>              → generate next scene for plotline
resolve plotline <id>              → mark plotline complete
```

`advance plotline` output format:
```
Advancing plotline "Find the Starship" (Action theme)...

Scene 3: A conflict erupts between those who seek the same thing.
Characters: The Iron Pact commander, A mysterious stranger
Plot element: Hidden agenda revealed

Thread check: "Find the Starship" (progress 3/5) — not yet resolved.
Thread check: "Trust the stranger" (progress 3/3) — Resolution roll: 8 vs 3 — Not yet.
```

**Tests:** `tests/test_adventure_crafter.py`
- Theme distribution: 100 rolls from Action theme produce > 60% action-flavored elements
- Create plotline → `list plotlines` shows it as active
- `advance plotline` 3 times → plotline has 3 scenes in DB
- Character thread progress 3 → resolution check triggered
- Story thread progress 5 → resolution check triggered
- Thread resolve roll success → thread marked resolved in DB
- Scene generation from Tension theme vs Action theme produce statistically different elements (run 20 of each)

**Acceptance:** Plotlines create, advance, and close. Threads progress and resolve per AC rules.

---

## Task 4.9: Shopping Scene

> **What:** Simple store in settlements. Browse, buy, sell.
> **Estimated time:** 5 hours

**Files:**
- `src/harsh_realm/gm/scenes/shopping.py` (NEW)
- `src/harsh_realm/gm/controller.py` — add Shopping scene transition
- `src/harsh_realm/parser/commands.py` — add shop/buy/sell/examine commands
- `data/tables/shops/general_store.yaml` (NEW)
- `data/tables/shops/blacksmith.yaml` (NEW)
- `data/tables/shops/apothecary.yaml` (NEW)

**Deliverables:**

`ShoppingSceneHandler` implementing `SceneHandler` protocol.

**Entry trigger:** Player in settlement hex AND types `shop`, `buy`, or `visit merchant`. If not in settlement: "There's nothing to buy out here."

**Exit trigger:** `leave` or `done` or `exit`.

**Valid commands:** `list`, `buy <item>`, `sell <item>`, `examine <item>`, `status`, `inventory`, `leave`

`list` output:
```
General Store — Ashfall

WEAPONS
  Dagger        1d4 dmg    1 slot    5 GP
  Spear         1d6+1 dmg  2 slots   10 GP

ARMOR
  Leather Armor  AC 13     2 slots   30 GP

GEAR
  Rope (50ft)    —          1 slot    1 GP
  Torch (×6)     —          1 slot    1 GP

CONSUMABLES
  Healing Herb   1d6+1 HP   1 slot    15 GP

Gold: 42 GP
```

`buy <item>` — deduct gold, add item to inventory. If insufficient gold: "You can't afford that." (42 GP shown in list, insufficient → clear message). XWN encumbrance slots update.

`sell <item>` — remove item from inventory, add gold at 50% base value. If equipped armor: un-equip first, then remove.

`examine <item>` — full stats without buying.

Settlement shop inventory loaded from YAML tables. Different establishment types (`general_store`, `blacksmith`, `apothecary`) have different stock tables.

All transactions fire `shopping.purchase` or `shopping.sale` events with item name and gold delta.

**Tests:** `tests/test_shopping.py`
- `shop` outside settlement → error message
- `shop` in settlement → shopping scene entered
- `list` → shows items with correct prices
- `buy dagger` with sufficient gold → inventory updated, gold deducted, encumbrance updated
- `buy plate_armor` with 5 GP (costs 100 GP) → "You can't afford that."
- `sell dagger` → gold increases by 50% of 5 GP (2 GP, round down)
- `sell equipped_armor` → armor un-equipped before removal
- `examine torch` → shows stats, no purchase made
- `leave` → returns to Exploration

**Content stubs needed:**
- `data/tables/shops/general_store.yaml` — 10–15 items minimum
- `data/tables/shops/blacksmith.yaml` — weapons and armor focus
- `data/tables/shops/apothecary.yaml` — consumables focus

**Acceptance:** Full shopping flow. Buy, sell, examine all work. Encumbrance updates correctly.

---

## Task 4.10: Expert Class Ability

> **What:** Expert reroll mechanic — once per scene after a failed skill check.
> **Estimated time:** 2 hours

**Files:**
- `src/harsh_realm/models/character.py` — add `expert_reroll_available` flag
- `src/harsh_realm/engine/skill_checks.py` — add reroll prompt logic
- `src/harsh_realm/gm/controller.py` — reset flag on scene change

**Deliverables:**

Add `expert_reroll_available: bool = True` to character data JSON (resets to `True` on every `gm.scene_change` event).

After any failed skill check, if character class is Expert and `expert_reroll_available == True`:
- GM prompts: "Your [Skill] check failed. Use your Expert ability to reroll? (yes/no)"
- `yes`: roll again, use the better result, set flag to False
- `no`: accept the failure, set flag to False
- Fire `character.expert_reroll` event recording original roll, reroll, and which was used

Non-Expert characters: never prompted.
Expert after flag is False: never prompted again until scene change.

**Tests:** `tests/test_expert_ability.py`
- Expert fails skill check → prompted for reroll
- Non-Expert fails skill check → not prompted
- Expert uses reroll → flag set to False → second failure in same scene → not prompted
- `gm.scene_change` event → flag resets to True
- Expert declines reroll → failure stands, flag set to False
- Expert reroll result worse than original → original (better) result used

**Acceptance:** Expert reroll works once per scene. Resets correctly on scene change.

---

## Task 4.11: Integration & Full Flow

> **What:** Wire all M4 systems together. Full integration test. Update CLAUDE.md.
> **Estimated time:** 4 hours

**Files:**
- `tests/test_integration_m4.py` (NEW)
- `CLAUDE.md` — update to "Milestone 4 complete"

**Deliverables:**

Integration test covering full M4 acceptance criteria (all 14 success criteria at top of this file).

Verify frontend updates:
- Social scene: NPC personality, skill check results, disposition changes appear in chat panel
- Status sidebar: Chaos factor displayed
- Admin panel: `/admin` route accessible from main nav

Update `CLAUDE.md`:
- Mark Milestone 4 complete with date
- Record test count
- Document any deviations from spec
- Update "Next" section to Milestone 4.5

**Acceptance:** All 14 success criteria from the top of this file pass in a live playtest. `pytest` green with new M4 tests counted. CLAUDE.md updated.

---

## Dependency Graph

```
Task 4.0 (M3 cleanup) ← do first, unblocks nothing but clean foundation
  │
Task 4.1 (Config tables + YAML seeds + AdminService)
  ├──→ Task 4.2 (Admin REST API + CLI) ← needs AdminService
  ├──→ Task 4.3 (Vue /admin panel) ← needs Admin REST API
  ├──→ Task 4.5 (Social scene) ← reads skill_mappings table
  └──→ Task 4.6 (Faction system) ← reads faction_asset_stats + encounter_weights
  │
Task 4.4 (UNE personality) ← needs no config tables, can parallelize with 4.1
  └──→ Task 4.5 (Social scene) ← needs UNE personality
  │
Task 4.5 (Social scene)
  │
Task 4.7 (Mythic Oracle core) ← independent, can start after 4.1 schema
  └──→ Task 4.8 (Adventure Crafter) ← needs oracle core
  │
Task 4.6 (Faction system) ← independent after 4.1
  │
Task 4.9 (Shopping) ← independent, only needs M3 inventory system
  │
Task 4.10 (Expert ability) ← independent, only needs skill_checks.py
  │
Task 4.11 (Integration) ← needs everything
```

Recommended build order:
1. 4.0 (cleanup — 2 hours, clear the slate)
2. 4.1 (data foundation — everything reads from these tables)
3. 4.4 (UNE — needed by social)
4. 4.5 (social — highest player value)
5. 4.7 (oracle core — scene checks feed into social)
6. 4.2 + 4.3 (admin API + Vue panel — can be done in parallel with 4.5/4.7)
7. 4.6 (faction turns — self-contained after data layer)
8. 4.8 (adventure crafter — builds on oracle)
9. 4.9 (shopping — fast, independent)
10. 4.10 (expert — fastest task)
11. 4.11 (integration)

---

## Content Stubs Needed

| File | Content | Stub Size | Notes |
|---|---|---|---|
| `data/skill_mappings.yaml` | 7 social verbs → skill + attr + difficulty | 7 entries | Defined in Task 4.1 |
| `data/difficulty_targets.yaml` | Named difficulty levels | 6 entries | Defined in Task 4.1 |
| `data/disposition_outcomes.yaml` | Outcome key → delta | 7 entries | Defined in Task 4.1 |
| `data/encounter_weights.yaml` | Faction disposition × encounter tag modifiers | ~10 entries | Defined in Task 4.1 |
| `data/faction_assets.yaml` | Full WWN asset list | All assets | Developer fills from WWN faction rules |
| `data/factions/*.yaml` | 3 starting factions | 3 files | Developer fills — use WWN faction creation tables |
| `data/tables/npc/une_*.yaml` | 6 UNE tables | Verbatim counts | Developer encodes from UNE rulebook |
| `data/tables/oracle/fate_chart.yaml` | 9×9 Mythic fate chart | 81 cells | Developer encodes from Mythic GME |
| `data/tables/oracle/event_*.yaml` | 3 random event tables | 100 entries each | Developer encodes from Mythic GME |
| `data/tables/oracle/ac_*.yaml` | 3 Adventure Crafter tables | Per rulebook | Developer encodes from Mythic GME |
| `data/templates/social_narration.yaml` | Bearing × outcome narration variants | 2 per combo min | Mark as expand |
| `data/tables/shops/*.yaml` | 3 shop type inventories | 10–15 items each | Developer populates from WWN equipment lists |

---

## Notes for the Coding Agent

- Read `docs/rules_reference/social.md`, `docs/rules_reference/faction_turns.md`, and `docs/rules_reference/oracle.md` before starting any task. These are the authoritative rules sources for M4.
- The M4 design specs at `docs/design/milestone_4_spec.md` and `docs/design/milestone_4_admin_spec.md` contain additional detail, interface signatures, and design rationale. Read them for context on complex tasks.
- Skill mappings are data, not code. Never hardcode a verb-to-skill mapping in Python. Always read from the `skill_mappings` SQLite table via `AdminService` or direct DB query.
- The M2 oracle (`engine/oracle.py`) is a placeholder. Task 4.7 replaces it entirely — do not try to extend it.
- The `admin` command group in-game must check `config.admin_mode` before executing. Default is `false`. Never bypass this check.
- UNE tables must be encoded verbatim from the rulebook. Do not paraphrase or approximate table entries.
- Adventure Crafter thread progression uses specific thresholds from the AC rulebook — read `docs/rules_reference/oracle.md` carefully before implementing.
- The Vue admin panel reads from the same REST API as the CLI. If the API is correct, the panel is straightforward. Build and test the API first.
- After completing all tasks, update `CLAUDE.md` with "Milestone 4 complete", the final test count, and any deviations from this spec.
