# Milestone 2: Things to Find — Task Specification

> **Goal:** The world has content to discover. Moving around triggers finds and encounters. NPCs exist in settlements. The frontend gains a window manager with draggable/resizable panels for chat, hex map, and status sidebar.
> **Estimated time:** 2-3 weeks (AI-assisted development)
> **Prerequisite:** Milestone 1 complete. Read CLAUDE.md, AGENTS.md, and all docs in `docs/rules_reference/`.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green.
2. Entering an unexplored hex has a ~50% chance of triggering a discovery via `search` or automatic find.
3. `search` command produces context-sensitive results from terrain-appropriate tables, gated by skill checks for rarer finds.
4. Searching the same hex again within a time window returns nothing; after the refresh period, it becomes searchable again.
5. Settlements contain generated NPCs with names, occupations, and personality traits following WWN/SWN generation tables.
6. `examine <npc>` shows NPC description. `talk to <npc>` produces a brief personality-driven template response.
7. `explore town` in a settlement hex lists available businesses and notable NPCs.
8. The frontend uses a window manager with draggable, resizable, overlapping panels.
9. Three default panels exist: Chat/Log, SVG Hex Map, and Status Sidebar.
10. Panel layout (position, size, z-order) persists in the database between sessions.
11. The SVG hex map shows terrain colors, fog of war, player position marker, and feature icons. It updates in real time as the player moves.
12. The status sidebar shows character name, class, level, HP, AC, location, XP, and active conditions.
13. World creation and loading works entirely through the chat interface (Menu scene).

---

## Task 2.0: Prerequisite Fixes — Menu Scene & M1 Frontend Gaps

**What:** Implement the Menu scene handler so world creation/loading works through the chat. Fix any Milestone 1 frontend issues (narration display, message type styling).

**Files:**
- `src/harsh_realm/gm/scenes/menu.py` — Menu scene handler (NEW)
- `src/harsh_realm/gm/controller.py` — Modify to start in Menu state when no world is loaded
- `src/harsh_realm/api/websocket.py` — Modify to handle no-world-loaded state
- `frontend/src/components/ChatLog.vue` — Fix message type rendering if needed

**Deliverables:**

`MenuScene` handler implementing `SceneHandler` protocol:
- Valid commands: `new`, `load`, `list`, `help`, `quit`
- On connect with no world loaded, GM displays:
  ```
  Welcome to Harsh Realm.
  
  Type 'new <name>' to create a new world.
  Type 'load <name>' to load an existing world.
  Type 'list' to see available worlds.
  ```
- `new <name>`:
  1. Create world database
  2. Generate hex map (calls WorldGenerator)
  3. Place features
  4. Set GM state to character creation
  5. Transition to CharacterCreation scene
  6. GM: "World '<name>' created. Let's build your character."
- `load <name>`:
  1. Find matching `.db` file in worlds directory
  2. Open database
  3. Check if character exists → transition to Exploration
  4. If no character → transition to CharacterCreation
  5. If world not found → GM: "No world named '<name>' found. Type 'list' to see available worlds."
- `list`:
  1. List all `.db` files in worlds directory with names and last modified dates
  2. GM displays the list
- `help`: Show available commands

Frontend fixes (verify and correct):
- Player input displayed as `> <text>` in a distinct lighter color
- GM narration displayed as main text, no prefix
- System messages (connection status, save confirmations) in dimmer color
- Multi-paragraph narration renders with proper paragraph spacing
- Chat auto-scrolls to bottom on new messages

**Tests:** `test_menu_scene.py`
- Menu scene returns correct valid commands
- `new testworld` creates a database file, generates hex map, transitions to CharacterCreation
- `load testworld` opens existing database, transitions to correct scene based on character existence
- `load nonexistent` returns error message, stays in Menu
- `list` returns available worlds
- Duplicate world name handling (error or unique suffix)

**Tests:** `test_frontend_messages.py` (Playwright or manual verification)
- Player input renders with `>` prefix and distinct styling
- GM narration renders without prefix in main text style
- Multi-paragraph narration has proper spacing
- System messages render in dimmed style

---

## Task 2.1: Random Table Engine

**What:** The core engine for loading YAML tables into SQLite and rolling on them with weighted entries, subtable resolution, and tag-based filtering.

**Files:**
- `src/harsh_realm/engine/tables.py` — TableEngine class (NEW)
- `data/schemas/table_schema.yaml` — JSON/YAML schema for table validation (NEW)

**Deliverables:**

`TableEngine` class:
- `load_tables(data_dir: str, db: WorldDatabase) -> int`
  - Recursively scan `data/tables/` for YAML files
  - Validate each against the table schema
  - Insert into `random_tables` SQLite table
  - Return count of tables loaded
  - Skip files that are already loaded (based on `id` match) unless file is newer

- `roll_on(table_id: str, context: dict | None = None) -> TableResult`
  - Fetch table from SQLite by ID
  - Apply context-based entry filtering if context provided (e.g., only entries matching certain tags)
  - Perform weighted random selection among entries
  - If selected entry references a subtable (`{"table": "other_table_id"}`), recursively resolve
  - Return `TableResult` with the final result and the chain of rolls for logging

- `roll_with_tags(category: str, tags: list[str]) -> TableResult`
  - Find tables matching the given category AND having at least one of the specified tags
  - If multiple tables match, pick one randomly (or merge entries)
  - Roll on the selected table

- `generate(generator_id: str, params: dict | None = None) -> dict`
  - Load a generator definition (YAML file describing a multi-step generation process)
  - Execute each step: roll on tables, assign results to named fields, run compute steps
  - Return a dict of generated values

`TableResult` frozen dataclass:
```python
@dataclass(frozen=True)
class TableResult:
    table_id: str
    roll_chain: list[str]      # IDs of tables rolled on (for subtable chains)
    result_type: str           # from the result entry: "item", "creature", "npc", etc.
    result: dict               # the final result data
    raw_text: str | None       # human-readable description if present
```

Table YAML schema:
```yaml
# Required fields
id: string                    # Unique table identifier
category: string              # "encounter", "discovery", "npc", "name", etc.
name: string                  # Human-readable name

# Optional fields
tags: list[string]            # For filtering: ["wilderness", "forest", "temperate"]
source: string                # Source book reference

# Entries (required)
entries:
  - weight: number            # Relative probability weight
    result: object | string   # Result data or subtable reference
    # Result can be:
    #   A string: "A rusty sword"
    #   A subtable ref: { table: "other_table_id" }
    #   A typed object: { type: "item", name: "Rusty Sword", value: 5 }
    #   A complex object: { type: "creature", table: "forest_creatures", context: "hostile" }
    min_difficulty: number     # Optional: minimum skill check difficulty to find this entry
    tags: list[string]         # Optional: entry-level tags for filtering
```

Generator YAML schema:
```yaml
id: string
name: string
steps:
  - roll: string              # Table ID to roll on
    assign: string            # Field name to assign result to
    count: number             # Optional: roll this many times (default 1)
  - compute: string           # Python function name to call
    assign: string
    params: object            # Parameters passed to the function, can reference prior assignments with {field_name}
```

**Tests:** `test_tables.py`
- Load tables from YAML directory → correct count in SQLite
- Roll on a table → returns a valid TableResult
- Weighted rolls: over 1000 rolls, distribution roughly matches weights (within tolerance)
- Subtable resolution: entry referencing another table → recursively resolved
- Tag filtering: `roll_with_tags("encounter", ["forest"])` only rolls on forest-tagged tables
- Context filtering: entries with non-matching tags excluded from roll
- Generator: multi-step generation produces a dict with all assigned fields
- Missing table ID raises appropriate error
- Empty table (all entries filtered out) raises appropriate error
- Duplicate table IDs: later load overwrites earlier (or raises error — decide and document)

---

## Task 2.2: Discovery System — Search Command

**What:** Implement the `search` command that produces context-sensitive discoveries from terrain-appropriate tables, gated by skill difficulty tiers, with time-based refresh.

**Files:**
- `src/harsh_realm/engine/discovery.py` — Discovery system (NEW)
- `src/harsh_realm/gm/scenes/exploration.py` — Add `search` to valid commands and handling
- `data/tables/discoveries/` — Discovery table YAML files (NEW directory)

**Deliverables:**

`DiscoverySystem` class:
- `search_hex(hex_data: dict, character: Character, db: WorldDatabase) -> DiscoveryResult`
  1. Check if hex was searched recently (time-based refresh). If within cooldown period, return `DiscoveryResult(found=False, message="You've already thoroughly searched this area.")`
  2. Roll discovery probability: 50% base chance on unexplored hexes, 30% on previously explored hexes. (These values should be configurable.)
  3. If probability check fails, return `DiscoveryResult(found=False, message=<flavor text about finding nothing>)`
  4. If probability check succeeds:
     a. Select discovery table based on hex terrain (e.g., `discoveries_forest`, `discoveries_ruins`)
     b. Roll on table to get a candidate result
     c. If result has a `min_difficulty`, perform a skill check (Notice or Survive depending on terrain context) against that difficulty
     d. If skill check fails, downgrade to a common result or return a "you sense something but can't find it" message
     e. If skill check succeeds (or no difficulty gate), create the discovery in the world state
  5. Record search timestamp on the hex
  6. Return `DiscoveryResult` with found item/feature/info

- `_select_skill(terrain: str) -> tuple[str, str]`
  - Returns (skill_name, attribute) based on terrain context
  - Wilderness terrains (forest, hills, plains, swamp, desert, wasteland): Survive + WIS
  - Ruins, settlements: Notice + INT or WIS
  - Default: Notice + WIS

`DiscoveryResult` frozen dataclass:
```python
@dataclass(frozen=True)
class DiscoveryResult:
    found: bool
    category: str | None       # "item", "relic", "ingredient", "environmental", "clue", None
    result: dict | None        # The discovered thing's data
    message: str               # Narration text for the chat log
    skill_check: SkillCheckResult | None  # If a check was made, include details
```

World state changes on discovery:
- **Items/relics/ingredients:** Create an entity of type "item" at the hex location OR add to a "hex_items" list in the hex data. Player can `take` it (Milestone 5 inventory system) or it's noted as present.
- **Environmental discovery:** Add a feature to the hex's `features` array and update `data`. Example: `{"features": ["hidden_spring"], "data": {"hidden_spring": {"description": "A clear spring bubbles from between moss-covered rocks."}}}`
- **Clue/lore:** Add to hex data and emit a `gm.narrate` event with the clue text. Example: `{"clues": ["Faction markings of the Iron Brotherhood are scratched into the wall."]}`
- **Nothing notable:** No world state change, just narration.

Time-based refresh:
- Store `last_searched_tick` in the hex's `data` JSON
- Refresh period: configurable, default 100 ticks (representing roughly a day of game time — exact tick-to-time mapping defined later)
- After refresh period, hex is searchable again with a fresh roll

Exploration scene changes:
- Add `search` to valid commands
- On `search` command: call `DiscoverySystem.search_hex()`, emit appropriate events
- Display skill check results in the log: "You search the area... [Notice check: rolled 8 + 1 = 9 vs. difficulty 8 — success]"
- Display discovery narration: "Among the underbrush, you find a corroded metal cylinder. It might be a pretech component."

**Discovery table YAML files to create (stubs with 5-8 entries each):**

| File | Terrain | Notes |
|---|---|---|
| `data/tables/discoveries/forest.yaml` | Forest | Herbs, animal tracks, hidden clearings, old campsites |
| `data/tables/discoveries/plains.yaml` | Plains | Trails, abandoned gear, plant materials, old foundations |
| `data/tables/discoveries/hills.yaml` | Hills | Cave entrances, mineral deposits, vantage points, cairns |
| `data/tables/discoveries/ruins.yaml` | Ruins | Pretech fragments, data chips, structural features, old signage |
| `data/tables/discoveries/wasteland.yaml` | Wasteland | Scrap metal, chemical residue, bone fields, buried containers |
| `data/tables/discoveries/desert.yaml` | Desert | Exposed foundations, glass formations, dried specimens, cached supplies |
| `data/tables/discoveries/swamp.yaml` | Swamp | Medicinal plants, sunken objects, preserved remains, gas vents |
| `data/tables/discoveries/common.yaml` | Any | Generic finds usable in any terrain (rope, cloth, tools) |

Each table should have entries across the difficulty spectrum:
- No difficulty gate (weight 4-5): Common mundane items, basic environmental details
- Difficulty 8 (weight 3): Useful items, interesting environmental features, minor clues
- Difficulty 10 (weight 2): Valuable items, significant features, important clues
- Difficulty 12+ (weight 1): Pretech relics, major discoveries, critical information

**Tests:** `test_discovery.py`
- `search_hex` on an unsearched hex with successful probability → returns a discovery
- `search_hex` on a recently searched hex → returns "already searched" message
- `search_hex` after refresh period → hex is searchable again
- Skill check gating: with mocked dice, a result requiring difficulty 10 is found when skill check succeeds and downgraded when it fails
- Discovery creates world state change: item appears in hex data, or feature added to hex
- Context-sensitive skill selection: forest uses Survive, ruins uses Notice
- Probability is ~50% on unexplored hex (test over 1000 rolls, verify within tolerance)
- Probability is ~30% on explored hex

---

## Task 2.3: Encounter System — Hex Entry Checks

**What:** When entering a new hex, the system checks for encounters. Encounters can be hostile creatures, neutral NPCs, environmental events, or other situations. For Milestone 2, encounters are narrated but not mechanically resolved (combat comes in Milestone 3).

**Files:**
- `src/harsh_realm/engine/encounters.py` — Encounter check system (NEW)
- `src/harsh_realm/gm/scenes/exploration.py` — Add encounter check on hex entry
- `data/tables/encounters/` — Encounter table YAML files (NEW directory)

**Deliverables:**

`EncounterSystem` class:
- `check_encounter(hex_data: dict, character: Character, db: WorldDatabase) -> EncounterResult | None`
  1. Roll encounter probability: ~50% on unexplored hexes, ~25% on explored hexes. Modified by terrain (ruins +10%, swamp +10%, plains near settlement -10%). Values configurable.
  2. If no encounter triggered, return None
  3. If triggered, select encounter table by terrain tags
  4. Roll on encounter table → `EncounterResult`
  5. For hostile encounters: in Milestone 2, narrate the encounter but don't start combat. "A pack of wild dogs blocks the trail ahead, snarling." For now, player can type `flee` (auto-succeed, retreat to previous hex) or any other command is met with "You cautiously avoid the threat." Combat resolution comes in Milestone 3.
  6. For non-hostile encounters: narrate and apply effects. NPC encounters spawn the NPC entity. Environmental events update hex data.

`EncounterResult` frozen dataclass:
```python
@dataclass(frozen=True)
class EncounterResult:
    encounter_type: str        # "hostile", "neutral_npc", "environmental", "discovery"
    name: str                  # Brief label: "Wild Dogs", "Traveling Merchant", "Sudden Storm"
    description: str           # Narration text
    entity_id: str | None      # If an NPC/creature was spawned, its entity ID
    data: dict                 # Additional encounter data
```

Integration with Exploration scene:
- After processing a `go` command and entering a new hex, call `EncounterSystem.check_encounter()`
- If an encounter occurs, narrate it after the hex description
- If hostile encounter: add `flee` as a temporarily valid command. Other movement commands work (you move away). Note in narration that combat will be available in a future update.
- If NPC encounter: the NPC is spawned at this hex and can be examined/talked to

**Encounter table YAML files to create (stubs):**

| File | Terrain | Notes |
|---|---|---|
| `data/tables/encounters/forest.yaml` | Forest | Wildlife, bandits, foragers, hermits, animal dens |
| `data/tables/encounters/plains.yaml` | Plains | Patrols, caravans, herds, travelers, exposed ruins |
| `data/tables/encounters/hills.yaml` | Hills | Prospectors, predators, rockslides, hidden camps |
| `data/tables/encounters/ruins.yaml` | Ruins | Scavengers, automated defenses (narrated), structural hazards, other explorers |
| `data/tables/encounters/wasteland.yaml` | Wasteland | Mutant creatures, dust storms, scrap traders, derelict vehicles |
| `data/tables/encounters/desert.yaml` | Desert | Sandstorms, nomads, heat hazards, buried structures |
| `data/tables/encounters/swamp.yaml` | Swamp | Leeches, bog gas, smugglers, drowned ruins |
| `data/tables/encounters/common.yaml` | Any | Weather events, distant sounds, old battlefield remains |

Each encounter table should include a mix of:
- Hostile encounters (weight 2-3): Creatures, bandits, hostile NPCs. Tagged `hostile`.
- Neutral NPC encounters (weight 2-3): Travelers, merchants, refugees, scouts. Tagged `npc`.
- Environmental encounters (weight 2-3): Weather, terrain hazards, atmospheric events. Tagged `environmental`.
- Discovery encounters (weight 1-2): Stumble onto something notable. Tagged `discovery`.

**Tests:** `test_encounters.py`
- `check_encounter` on unexplored hex triggers ~50% of the time (test over 1000 rolls)
- `check_encounter` on explored hex triggers ~25% of the time
- Terrain modifiers adjust probability correctly
- Hostile encounter returns type "hostile" with description
- NPC encounter spawns an entity in the database
- Environmental encounter updates hex data
- Encounter tables load and resolve correctly via TableEngine
- No encounter returns None

---

## Task 2.4: NPC Generation

**What:** Generate NPCs following WWN/SWN patterns. NPCs have names, occupations, personality traits, and brief descriptions. Minimal interaction: `examine` and `talk to`.

**Files:**
- `src/harsh_realm/generators/npc_gen.py` — NPC generator (NEW)
- `src/harsh_realm/models/npc.py` — NPC data model (update if needed)
- `src/harsh_realm/gm/scenes/exploration.py` — Add `examine` and `talk to` command handling for NPCs
- `data/generators/npc_basic.yaml` — NPC generator definition (NEW)
- `data/tables/npcs/` — NPC generation tables (NEW directory)

**Deliverables:**

`NPCGenerator` class:
- `generate_npc(context: dict | None = None) -> dict`
  - Uses the TableEngine's generator system with `npc_basic` generator
  - Produces a complete NPC data dict suitable for storing in the `entities` table
  - Context can include terrain, settlement size, faction affiliation to influence generation

NPC data model (stored in entity `data` JSON):
```python
@dataclass
class NPCData:
    occupation: str            # "blacksmith", "merchant", "farmer", "soldier", etc.
    personality_traits: list[str]  # 1-2 traits: "suspicious", "generous", "fearful"
    motivation: str            # What drives them: "wealth", "safety", "revenge", "curiosity"
    appearance: str            # Brief physical description
    greeting: str              # Template-based first interaction line
    faction_id: str | None     # Faction affiliation if any
    disposition: str           # "hostile", "wary", "neutral", "friendly" (toward strangers)
```

NPC interaction commands added to Exploration scene:
- `examine <npc_name>`: Display NPC appearance, occupation, and visible demeanor.
  - Example: "Gareth is a stocky blacksmith with burn-scarred hands. He watches you with guarded curiosity."
- `talk to <npc_name>` or `talk <npc_name>`: Display a template-based response reflecting personality and disposition.
  - Example: "Gareth grunts a greeting. 'Not many travelers come through here. What do you want?'"
  - Hostile NPCs: "The soldier glares at you and rests a hand on his weapon. 'Move along.'"
  - Friendly NPCs: "The old woman smiles warmly. 'Welcome, traveler. You look like you could use a hot meal.'"
- NPC matching: when player types a name, match against NPCs at the current hex location. Partial name matching (case-insensitive, prefix match).

Settlement NPC population:
- When a settlement is generated (during world creation), generate 3-8 NPCs depending on settlement size
- NPCs are stored as entities with `entity_type = "npc"` and location matching the settlement hex
- Each NPC has an occupation relevant to the settlement (blacksmith, innkeeper, merchant, elder, guard, farmer, healer, etc.)

`explore town` command (settlement hexes only):
- Lists the settlement name and size
- Lists available "businesses" derived from NPC occupations:
  - Blacksmith → "Forge"
  - Merchant/Trader → "Trading Post"
  - Innkeeper → "Inn"
  - Healer → "Healer's Hut"
  - (Map occupations to business types)
- Lists notable NPCs by name and occupation
- Example output:
  ```
  Millhaven — Village

  Establishments:
    The Rusty Anvil (forge) — run by Gareth
    Millhaven Trading Post — run by Sera
    The Weary Traveler (inn) — run by Old Bram

  Notable residents:
    Captain Voss — village militia
    Maren — healer
  ```

**NPC table YAML files to create (stubs, to be expanded from WWN source material):**

| File | Content | Minimum entries |
|---|---|---|
| `data/tables/npcs/names_male.yaml` | Male first names | 30 names |
| `data/tables/npcs/names_female.yaml` | Female first names | 30 names |
| `data/tables/npcs/names_surnames.yaml` | Surnames / epithets | 30 names |
| `data/tables/npcs/occupations_settlement.yaml` | Settlement occupations | 15 occupations |
| `data/tables/npcs/occupations_wandering.yaml` | Traveler/wilderness occupations | 10 occupations |
| `data/tables/npcs/personality_traits.yaml` | Personality descriptors | 20 traits |
| `data/tables/npcs/motivations.yaml` | NPC driving motivations | 10 motivations |
| `data/tables/npcs/appearances.yaml` | Physical appearance snippets | 20 descriptions |
| `data/tables/npcs/greetings.yaml` | Template greeting lines by disposition | 4 dispositions × 3 variants |
| `data/generators/npc_basic.yaml` | Generator combining the above tables | Complete generator definition |

**Tests:** `test_npc_gen.py`
- `generate_npc()` produces a dict with all required fields (name, occupation, personality, etc.)
- Generated NPC stored in entities table with correct entity_type and location
- Settlement generation produces 3-8 NPCs per settlement
- No duplicate names within a single settlement
- `examine <npc>` returns description text
- `talk to <npc>` returns personality-appropriate response
- `explore town` lists establishments and NPCs
- `explore town` on a non-settlement hex returns "There's no settlement here."
- Partial name matching works: `examine gar` matches "Gareth"
- NPC not at current location: "You don't see anyone by that name here."

---

## Task 2.5: Settlement Generation Enhancement

**What:** Enhance settlement generation from Milestone 1's basic placement to produce named settlements with NPCs, establishments, and descriptions.

**Files:**
- `src/harsh_realm/generators/settlement_gen.py` — Settlement generator (NEW or significantly expand)
- `data/tables/settlements/` — Settlement generation tables (NEW directory)

**Deliverables:**

`SettlementGenerator` class:
- `generate_settlement(hex_coord: HexCoord, size: str, terrain: str, db: WorldDatabase) -> dict`
  1. Generate settlement name from name tables
  2. Determine establishment count based on size:
     - Hamlet: 1-2 establishments, 3-4 NPCs
     - Village: 2-4 establishments, 5-8 NPCs
     - Town: 4-6 establishments, 8-12 NPCs
  3. Generate establishments from occupation-to-business mapping
  4. Generate NPCs for each establishment plus a few unattached residents
  5. Generate a brief settlement description based on terrain and size
  6. Store settlement data in hex `data` JSON
  7. Store NPCs as entities in the database
  8. Return settlement data dict

Settlement sizes for map generation:
- Starting settlement (from Milestone 1): always Village
- Additional settlements: randomly assigned Hamlet or Village (Towns are rare, maybe 0-1 per map)

Settlement data stored in hex:
```json
{
  "features": ["settlement"],
  "data": {
    "settlement": {
      "name": "Millhaven",
      "size": "village",
      "description": "A weathered cluster of timber buildings huddled around a muddy crossroad.",
      "establishments": [
        {"name": "The Rusty Anvil", "type": "forge", "npc_id": "uuid-gareth"},
        {"name": "Millhaven Trading Post", "type": "trading_post", "npc_id": "uuid-sera"},
        {"name": "The Weary Traveler", "type": "inn", "npc_id": "uuid-bram"}
      ]
    }
  }
}
```

**Settlement table YAML files to create (stubs):**

| File | Content | Minimum entries |
|---|---|---|
| `data/tables/settlements/names_prefix.yaml` | Settlement name prefixes | 20 entries (Mill-, Iron-, Ash-, etc.) |
| `data/tables/settlements/names_suffix.yaml` | Settlement name suffixes | 20 entries (-haven, -ford, -gate, etc.) |
| `data/tables/settlements/descriptions.yaml` | Settlement description templates by size and terrain | 3 per size × 3 terrain combos |
| `data/tables/settlements/establishment_names.yaml` | Business name templates by type | 5 per establishment type |

**Tests:** `test_settlement_gen.py`
- Generate hamlet → 1-2 establishments, 3-4 NPCs
- Generate village → 2-4 establishments, 5-8 NPCs
- Settlement has a name, size, and description
- Each establishment has a name, type, and associated NPC
- NPCs are stored as entities in the database with correct location
- Settlement data stored correctly in hex data JSON
- No duplicate establishment types in a single settlement (no two forges)

---

## Task 2.6: Frontend — Window Manager System

**What:** Implement a draggable, resizable, overlapping panel system for the Vue frontend. All UI components become panels managed by this system.

**Files:**
- `frontend/src/components/WindowManager.vue` — Window manager container (NEW)
- `frontend/src/components/PanelWindow.vue` — Individual draggable/resizable panel (NEW)
- `frontend/src/stores/layout.ts` — Pinia store for panel layout state (NEW)
- `frontend/src/composables/usePanelLayout.ts` — Layout persistence composable (NEW)
- `frontend/src/App.vue` — Refactor to use WindowManager

**Deliverables:**

`PanelWindow` component:
- Props: `panelId: string`, `title: string`, `defaultPosition: {x, y}`, `defaultSize: {width, height}`, `minSize: {width, height}`, `zIndex: number`
- Draggable by title bar (click and drag header area)
- Resizable by dragging edges or corners
- Click-to-focus: clicking a panel brings it to the front (highest z-index)
- Close button in title bar (optional — some panels may not be closable)
- Minimize button (collapse to title bar only)
- Slot-based content: `<PanelWindow title="Chat"><ChatLog /></PanelWindow>`
- Emits position/size/z-index changes to the layout store

`WindowManager` component:
- Container that holds all `PanelWindow` instances
- Manages z-index ordering (global z-index counter)
- Provides a method to reset all panels to default positions
- Renders a subtle background (dark theme appropriate for the setting)

`layout` Pinia store:
- State: `panels: Record<string, PanelState>` where `PanelState` includes:
  ```typescript
  interface PanelState {
    x: number
    y: number
    width: number
    height: number
    zIndex: number
    minimized: boolean
    visible: boolean
  }
  ```
- Actions: `updatePanel(id, partial)`, `bringToFront(id)`, `resetLayout()`, `saveLayout()`, `loadLayout()`
- On any panel state change, debounce and persist to server

Layout persistence:
- `POST /api/ui/layout` — save panel layout JSON to the active world database (in `world_meta` with key `"ui_layout"`)
- `GET /api/ui/layout` — load panel layout from active world
- On frontend load: fetch layout from server. If none exists, use defaults.
- On panel move/resize: debounce 500ms, then save to server.

Backend endpoints:
- `src/harsh_realm/api/routes.py` — Add `/api/ui/layout` GET and POST endpoints
- Store in `world_meta` table: key `"ui_layout"`, value is JSON string of panel states

Default panel layout:
```
┌─────────────────────────────────────────────────────────┐
│  ┌──────────────────────────┐ ┌───────────────────────┐ │
│  │       Hex Map            │ │    Status Sidebar     │ │
│  │     (500x400)            │ │     (250x400)         │ │
│  │     top-right            │ │     right of map      │ │
│  └──────────────────────────┘ └───────────────────────┘ │
│  ┌────────────────────────────────────────────────────┐ │
│  │                  Chat / Log                         │ │
│  │                (full width, bottom half)             │ │
│  │                  (800x350)                          │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

Implementation notes:
- Use native mouse events for drag/resize rather than a heavy library, OR use a lightweight Vue library like `vue-draggable-resizable` if it simplifies implementation. Agent should evaluate and choose the simpler option.
- Panels must work on a single-screen layout. No need for multi-monitor support.
- Minimum panel sizes to prevent collapsing to unusable dimensions.
- Title bar should show the panel name and be styled to match the dark theme.

**Tests:** `test_window_manager.ts` (or Playwright E2E)
- Three default panels render on load (Chat, Map, Status)
- Panel can be dragged to a new position (verify position state updates)
- Panel can be resized (verify size state updates)
- Clicking a panel brings it to the front
- Minimize collapses panel to title bar
- Layout persists: move panels → reload page → panels are in the same positions
- Reset layout restores defaults
- Panels can overlap each other

---

## Task 2.7: Frontend — SVG Hex Map Panel

**What:** An SVG hex map showing explored terrain, fog of war, player position, and feature icons. View-only, updates in real time via WebSocket.

**Files:**
- `frontend/src/components/HexMap.vue` — SVG hex map component (NEW)
- `frontend/src/stores/map.ts` — Pinia store for map state (NEW)

**Deliverables:**

`HexMap` Vue component:
- Renders a pointy-top hex grid in SVG
- Each hex is a `<polygon>` element colored by terrain type
- Terrain color scheme (dark theme appropriate):
  ```
  plains:    #4a5d3a (muted green)
  forest:    #2d4a2d (dark green)
  hills:     #6b6b4f (olive brown)
  mountains: #5a5a5a (dark gray)
  water:     #2a3d5a (dark blue)
  swamp:     #3d4a3a (murky green-brown)
  desert:    #7a6a4a (sand brown)
  wasteland: #5a4a3a (dark brown)
  ruins:     #4a4a5a (dark purple-gray)
  ```
- **Fog of war:** Unexplored hexes are dark/hidden (very dark overlay or solid dark color). Explored hexes show terrain color.
- **Player position:** A distinct marker (bright dot, arrow, or character icon) on the player's current hex.
- **Feature icons:** Small icons or symbols on hexes with features:
  - Settlement: small house icon or "S"
  - Ruins: small broken column icon or "R"
  - Lair: small skull icon or "L"
  - Landmark: small star icon or "★"
  - (Simple text characters are fine for now — proper icons can come later)
- **Viewport:** The map should center on the player's position. If the full 20x20 grid doesn't fit, show the nearby region and allow scrolling/panning.
- **Pan and zoom:** Mouse wheel to zoom, click-and-drag on empty space to pan. Or scroll bars. Basic navigation so the player can see the whole map or focus on their area.

Map state store:
- Receives hex data from the server via WebSocket or REST API
- Tracks which hexes are explored, terrain types, features, player position
- Updates reactively when `exploration.enter_hex` events arrive via WebSocket

Data source:
- On world load: `GET /api/worlds/current/map` returns all hex data (terrain, explored, features for explored hexes)
- On player movement: `exploration.enter_hex` WebSocket event includes the new hex data and updated player position
- The frontend updates the map store and re-renders affected hexes

Backend additions:
- Ensure the map API endpoint returns data in a format the frontend can efficiently render
- `exploration.enter_hex` event should include: `{hex: {q, r, terrain, features, explored}, player_position: {q, r}, adjacent_explored: [{q, r, terrain, features}, ...]}`

**Tests:** (Playwright E2E or manual verification)
- Map renders with correct number of visible hexes
- Unexplored hexes are dark/hidden
- Player position marker is visible
- Moving via chat command → map updates with new position
- Newly explored hex transitions from fog to terrain color
- Feature icons appear on hexes with features
- Pan and zoom work
- Terrain colors match the scheme

---

## Task 2.8: Frontend — Status Sidebar Panel

**What:** A compact panel showing character vitals that updates in real time.

**Files:**
- `frontend/src/components/StatusSidebar.vue` — Status sidebar component (NEW)
- `frontend/src/stores/game.ts` — Extend game store with character state

**Deliverables:**

`StatusSidebar` Vue component:
- Displays in a `PanelWindow` titled "Status"
- Content sections (compact, no excessive spacing):

  ```
  ▸ Kira Voss
    Warrior 1

  ▸ Vitals
    HP  9/9  ████████████
    AC  15
    XP  0 / 1500

  ▸ Location
    Thornwood Edge (3, -1)
    Terrain: Forest
    Features: Settlement

  ▸ Conditions
    (none)
  ```

- HP bar: visual bar (colored green > yellow > red based on percentage) alongside the numbers
- Sections are collapsible (click header to expand/collapse)
- Updates reactively when game state changes

Data source:
- On world load: `GET /api/character` returns current character data
- On state changes: WebSocket events update relevant fields
  - `action.move` → update location
  - `character.xp_gained` → update XP
  - HP changes (future milestones) → update HP bar
  - `exploration.enter_hex` → update terrain and features display

Backend addition:
- `GET /api/character` endpoint returning current player character data (or include in world load response)

**Tests:** (Playwright E2E or manual verification)
- Sidebar renders with character name, class, level
- HP bar displays correctly and changes color at different percentages
- Location updates when player moves
- XP display is correct
- Panel is draggable and resizable within the window manager

---

## Task 2.9: Frontend — Chat Panel Refactor

**What:** Refactor the existing chat log and command input into a panel managed by the window manager.

**Files:**
- `frontend/src/components/ChatPanel.vue` — Chat panel wrapping ChatLog + CommandInput (NEW or refactor)
- `frontend/src/components/ChatLog.vue` — Modify if needed
- `frontend/src/components/CommandInput.vue` — Modify if needed

**Deliverables:**

- Wrap existing `ChatLog` and `CommandInput` components inside a `PanelWindow` titled "Chat"
- Chat panel should maintain all existing functionality:
  - Scrollable message log
  - Text input with enter-to-send
  - Message type styling (player input, GM narration, system messages)
  - Auto-scroll on new messages
- Command input always stays at the bottom of the chat panel regardless of panel resize
- Chat panel should have a minimum width/height to remain usable
- Add a scroll-to-bottom button that appears when the user has scrolled up (so they can quickly return to latest messages)
- Command history: up/down arrow keys cycle through previously entered commands (store last 50 commands in memory)

**Tests:** (Playwright E2E or manual verification)
- Chat panel renders inside window manager
- Can be dragged and resized
- Messages display correctly with type-based styling
- Command input works and submits on enter
- Auto-scroll works
- Command history (up arrow recalls previous command)
- Scroll-to-bottom button appears when scrolled up

---

## Task 2.10: Integration & Data Flow

**What:** Wire everything together. Verify the full flow from world creation through exploration with discoveries, encounters, NPCs, and real-time frontend updates.

**Files:**
- Various — integration test files and any fixes needed

**Deliverables:**

Integration test or verification script covering:
1. Start server
2. Open frontend → see Menu scene prompt in chat panel
3. Type `new Ashfall` → world created, GM transitions to character creation
4. Complete character creation → GM transitions to Exploration
5. Map panel shows starting hex explored, player position marked
6. Status sidebar shows character info and location
7. Move to several hexes → map updates, descriptions appear in chat
8. Enter a hex that triggers an encounter → encounter narrated in chat
9. Enter a settlement hex → `explore town` lists establishments and NPCs
10. `examine <npc>` → NPC description appears
11. `talk to <npc>` → NPC greeting appears
12. `search` in a wilderness hex → discovery result narrated, item/feature created
13. `search` same hex again → "already searched" message
14. Move panels around, resize them → reload page → layout preserved
15. `save mysave` → snapshot created
16. Close browser, reopen → world loads, character and position persist, panels in saved positions

Update `CLAUDE.md` with "Milestone 2 complete" and note any deviations.

---

## Dependency Graph

```
Task 2.0 (Menu scene + M1 fixes)
  ↓
Task 2.1 (Table engine) ──────────────────────────────┐
  ↓                                                     │
Task 2.2 (Discovery/search) ← uses table engine        │
  ↓                                                     │
Task 2.3 (Encounter system) ← uses table engine        │
  ↓                                                     │
Task 2.4 (NPC generation) ← uses table engine + generators
  ↓                                                     │
Task 2.5 (Settlement enhancement) ← uses NPC gen       │
                                                        │
Task 2.6 (Window manager) ← independent frontend work  │
  ↓                                                     │
Task 2.7 (Hex map panel) ← needs window manager        │
  ↓                                                     │
Task 2.8 (Status sidebar panel) ← needs window manager │
  ↓                                                     │
Task 2.9 (Chat panel refactor) ← needs window manager  │
  ↓                                                     │
Task 2.10 (Integration) ← needs everything             │
```

Backend tasks (2.0-2.5) and frontend tasks (2.6-2.9) can be developed in parallel by different agents or in any convenient order, as long as the integration task (2.10) comes last.

---

## Content Stubs Needed

The coding agent should create stubs for all YAML files below with the minimum specified entries. Files marked "Developer populates" should include a comment noting they need expansion from source material.

| File | Content | Stub Size | Notes |
|---|---|---|---|
| `data/tables/discoveries/forest.yaml` | Forest discoveries | 8 entries | Developer populates from source material |
| `data/tables/discoveries/plains.yaml` | Plains discoveries | 8 entries | Developer populates |
| `data/tables/discoveries/hills.yaml` | Hills discoveries | 8 entries | Developer populates |
| `data/tables/discoveries/ruins.yaml` | Ruins discoveries | 8 entries | Critical for setting flavor — pretech focus |
| `data/tables/discoveries/wasteland.yaml` | Wasteland discoveries | 8 entries | Developer populates |
| `data/tables/discoveries/desert.yaml` | Desert discoveries | 8 entries | Developer populates |
| `data/tables/discoveries/swamp.yaml` | Swamp discoveries | 8 entries | Developer populates |
| `data/tables/discoveries/common.yaml` | Universal discoveries | 6 entries | Generic items for any terrain |
| `data/tables/encounters/forest.yaml` | Forest encounters | 8 entries | Mix of hostile, NPC, environmental |
| `data/tables/encounters/plains.yaml` | Plains encounters | 8 entries | Mix of types |
| `data/tables/encounters/hills.yaml` | Hills encounters | 8 entries | Mix of types |
| `data/tables/encounters/ruins.yaml` | Ruins encounters | 8 entries | Heavier on hostile + discovery |
| `data/tables/encounters/wasteland.yaml` | Wasteland encounters | 8 entries | Mix of types |
| `data/tables/encounters/desert.yaml` | Desert encounters | 8 entries | Mix of types |
| `data/tables/encounters/swamp.yaml` | Swamp encounters | 8 entries | Mix of types |
| `data/tables/encounters/common.yaml` | Universal encounters | 6 entries | Weather, distant events |
| `data/tables/npcs/names_male.yaml` | Male first names | 30 names | Developer expands |
| `data/tables/npcs/names_female.yaml` | Female first names | 30 names | Developer expands |
| `data/tables/npcs/names_surnames.yaml` | Surnames / epithets | 30 names | Developer expands |
| `data/tables/npcs/occupations_settlement.yaml` | Settlement jobs | 15 entries | Developer expands |
| `data/tables/npcs/occupations_wandering.yaml` | Traveler jobs | 10 entries | Developer expands |
| `data/tables/npcs/personality_traits.yaml` | Personality descriptors | 20 traits | Developer expands from WWN |
| `data/tables/npcs/motivations.yaml` | NPC motivations | 10 entries | Developer expands from WWN |
| `data/tables/npcs/appearances.yaml` | Physical descriptions | 20 entries | Developer expands |
| `data/tables/npcs/greetings.yaml` | Greeting templates | 12 entries | 4 dispositions × 3 variants |
| `data/tables/settlements/names_prefix.yaml` | Settlement name parts | 20 entries | |
| `data/tables/settlements/names_suffix.yaml` | Settlement name parts | 20 entries | |
| `data/tables/settlements/descriptions.yaml` | Settlement descriptions | 9 entries | 3 sizes × 3 terrain combos |
| `data/tables/settlements/establishment_names.yaml` | Business names | 30 entries | 5 per establishment type |
| `data/generators/npc_basic.yaml` | NPC generator definition | Complete | Multi-step generator |

---

## Notes for the Coding Agent

- Read AGENTS.md before writing any code. Follow all conventions.
- The random table engine (Task 2.1) is foundational — everything else in this milestone depends on it. Get the schema right and test it thoroughly.
- The window manager (Task 2.6) is the most complex frontend task. Evaluate available Vue libraries before implementing from scratch. `vue-draggable-resizable` or similar may save significant time. Prioritize reliability over features — panels must not jump, glitch, or lose position.
- Discovery tables should feel atmospheric and setting-appropriate. This is a dark, hostile feudal planet with pretech ruins. Common finds are scrap and survival supplies, not treasure chests of gold. Pretech finds should feel rare and significant.
- NPC generation should produce varied, interesting characters even from a small table set. The greeting templates should reflect personality traits visibly — a "suspicious" NPC greets differently from a "generous" one.
- For encounter narration (Task 2.3), hostile encounters in Milestone 2 are narrated but NOT mechanically resolved. Include a clear note in the chat that combat will be available in a future update. The player can flee (auto-success) or the encounter resolves narratively.
- All YAML stub files should include a comment: `# TODO: Expand from source material` at the top.
- After completing all tasks, update `CLAUDE.md` with "Milestone 2 complete" and note any deviations or issues discovered.
