# Milestone 1: The Empty World — Task Specification

> **Goal:** Generate a hex map, create a character through a guided flow, move around the map, and see atmospheric terrain descriptions. All interaction is through the chat log — no graphical map yet.
> **Estimated time:** 1-2 weeks (AI-assisted development)
> **Prerequisite:** Milestone 0 complete. Read CLAUDE.md, AGENTS.md, and all docs in `docs/rules_reference/`.

## Success Criteria

When this milestone is complete:

1. `pytest` passes with all tests green.
2. Create a new world → a 20x20 hex map is generated with varied terrain, 1-3 bounded edges, features placed on hexes.
3. The GM walks you through character creation step by step (name → class → roll attributes → assign → skills → kit → confirm).
4. Move in all 6 hex directions with `go <direction>` or shorthand (`n`, `ne`, `se`, `s`, `sw`, `nw`).
5. Each hex entry produces an atmospheric terrain description with variation (not the same text every time for the same terrain type).
6. Adjacent hex features are mentioned in descriptions when not blocked by terrain.
7. Moving into impassable terrain (mountains, water) is blocked with a narrative explanation.
8. `look` redescribes the current hex. `status` prints the character sheet. `help` lists commands.
9. Fog of war: hexes track whether they've been explored. Revisiting a hex notes familiarity.
10. Character data persists in SQLite — close and reopen the world, character is intact.

---

## Task 1.1: Hex Map Data Model & Terrain Definitions

**What:** Define terrain types, hex grid math, and the data structures for the map.

**Files:**
- `src/harsh_realm/models/hex_map.py` — Hex coordinate math, terrain definitions, hex data model
- `data/terrain.yaml` — Terrain type definitions

**Deliverables:**

Hex coordinate system using axial coordinates `(q, r)` for a pointy-top hex grid:
- `HexCoord` frozen dataclass with `q: int, r: int`
- Neighbor calculation for 6 directions: N, NE, SE, S, SW, NW
- Distance calculation between two hexes
- Direction enum: `HexDirection` with the 6 values

Terrain type definitions in YAML, loaded at startup:

```yaml
# data/terrain.yaml
terrains:
  - id: plains
    name: Plains
    passable: true
    blocks_vision: false    # Can see through to hexes beyond
    description_pool: descriptions_plains  # reference to description templates

  - id: forest
    name: Forest
    passable: true
    blocks_vision: true     # Cannot see features in hexes beyond forest

  - id: hills
    name: Hills
    passable: true
    blocks_vision: false

  - id: mountains
    name: Mountains
    passable: false
    blocks_vision: true

  - id: water
    name: Water
    passable: false
    blocks_vision: false

  - id: swamp
    name: Swamp
    passable: true
    blocks_vision: true     # Dense vegetation blocks view

  - id: desert
    name: Desert
    passable: true
    blocks_vision: false

  - id: wasteland
    name: Wasteland
    passable: true
    blocks_vision: false

  - id: ruins
    name: Ruins
    passable: true
    blocks_vision: false    # Visible from a distance
```

`TerrainType` frozen dataclass loaded from YAML:
- `id`, `name`, `passable`, `blocks_vision`, `description_pool`

**Tests:** `test_hex_map.py`
- HexCoord neighbor calculation returns correct coordinates for all 6 directions
- Distance between (0,0) and (3,0) is 3
- Distance between (0,0) and (0,0) is 0
- Terrain definitions load from YAML correctly
- All 9 terrain types present after loading

---

## Task 1.2: Hex Map Generator

**What:** Generate a 20x20 hex map using table-based terrain assignment with neighbor-weighted adjacency.

**Files:**
- `src/harsh_realm/generators/world_gen.py` — Map generation logic
- `data/tables/terrain/terrain_weights.yaml` — Base terrain probability weights and adjacency modifiers

**Deliverables:**

`WorldGenerator` class:
- `generate_region(width: int, height: int, seed: int | None = None) -> None`
  - Writes generated hexes to the `hexes` table in the active world database
  - Deterministic if a seed is provided (for testing)

Generation algorithm:
1. **Determine bounded edges:** Roll for number of bounded sides (1-3). Randomly select which sides (N, E, S, W). For each bounded side, randomly select boundary type (ocean/mountains/cliffs). Fill boundary hexes with the corresponding impassable terrain (water for ocean, mountains for mountains/cliffs).
2. **Seed terrain:** Place 5-10 random seed hexes with terrain chosen from base weight table.
3. **Fill outward:** For each unfilled hex, look at already-filled neighbors. Weight the terrain probability table by adjacency modifiers (forest next to forest is more likely, mountains next to mountains form ranges, etc.). Roll on the weighted table to assign terrain.
4. **Guarantee variety:** After generation, verify at least 3 different passable terrain types exist. Regenerate if not.
5. **Place starting settlement:** Find a passable hex on or near the open edge. Place a settlement feature on it.

The adjacency modifier YAML:

```yaml
# data/tables/terrain/terrain_weights.yaml
base_weights:
  plains: 20
  forest: 20
  hills: 15
  mountains: 5
  water: 5
  swamp: 5
  desert: 10
  wasteland: 15
  ruins: 5

# When a neighbor has this terrain, multiply the weight of the candidate terrain by this factor.
# Values > 1 mean "more likely near this terrain", < 1 means "less likely".
adjacency_modifiers:
  forest:
    forest: 2.0      # Forest tends to cluster
    hills: 1.3
    swamp: 1.5
    plains: 0.8
    desert: 0.3       # Forest rarely borders desert
  mountains:
    mountains: 2.5    # Mountain ranges
    hills: 2.0        # Foothills
    plains: 0.5
    forest: 0.7
  hills:
    mountains: 1.5
    hills: 1.5
    forest: 1.2
    plains: 1.2
  water:
    water: 3.0        # Bodies of water cluster
    swamp: 2.0
    plains: 1.2
  swamp:
    water: 2.0
    forest: 1.5
    swamp: 1.5
    desert: 0.1
  desert:
    desert: 2.5
    wasteland: 2.0
    forest: 0.2
    swamp: 0.1
  wasteland:
    wasteland: 2.0
    desert: 1.5
    ruins: 1.5
  ruins:
    wasteland: 1.5
    ruins: 0.5        # Ruins don't cluster much — they're scattered
  plains:
    plains: 1.5
    forest: 1.2
    hills: 1.2
```

**Tests:** `test_world_gen.py`
- Generate a map with a fixed seed → verify it produces the expected dimensions (20x20 = 400 hexes)
- All hexes have a valid terrain type
- At least 3 different passable terrain types present
- Bounded edges contain impassable terrain
- At least one open edge exists
- A settlement feature exists on or near the open edge
- Deterministic: same seed → same map

---

## Task 1.3: Feature Placement

**What:** Place features (settlements, ruins POIs, landmarks) on the generated map.

**Files:**
- `src/harsh_realm/generators/world_gen.py` (extend)
- `data/tables/terrain/features.yaml` — Feature type definitions and placement probabilities

**Deliverables:**

Feature placement runs after terrain generation:
1. **Starting settlement** already placed (from Task 1.2) on the open edge.
2. **Additional settlements:** Place 2-4 more small settlements on passable hexes, spaced at least 3 hexes apart. Prefer plains and hills terrain.
3. **Ruin sites:** Place 4-8 ruin features on passable hexes (excluding settlements). Mark as unexplored. These are the pretech sites the player will eventually delve into.
4. **Landmarks:** Place 3-6 landmarks (distinctive terrain features, ancient structures, natural wonders) for navigation reference and flavor.
5. **Lairs/camps:** Place 2-4 dangerous sites (monster lairs, bandit camps). These will tie into encounter tables in Milestone 2.

Features are stored in the hex's `features` JSON array and `data` JSON column:
```json
{
  "features": ["settlement"],
  "data": {
    "settlement": {
      "name": "Millhaven",
      "size": "village",
      "description": "A cluster of timber buildings..."
    }
  }
}
```

Feature names are generated from placeholder name tables (simple lists the developer can expand).

**Tests:** `test_world_gen.py` (extend)
- Generated map has 1 starting settlement on the open edge
- Total settlements between 3-5
- Ruin sites between 4-8
- Settlements are at least 3 hexes apart
- Features stored correctly in hex data

---

## Task 1.4: Hex Description Templates

**What:** Atmospheric text descriptions for each terrain type with variation and context awareness.

**Files:**
- `data/templates/terrain_descriptions.yaml` — Description template pools
- `src/harsh_realm/gm/narrator.py` — Template selection and rendering

**Deliverables:**

Description template YAML with 5-8 variants per terrain type:

```yaml
# data/templates/terrain_descriptions.yaml
terrain_descriptions:
  forest:
    base:
      - "Dense pines crowd the trail. The air is damp and cold under the canopy."
      - "Pale, thin trees stretch in every direction. The ground is carpeted with dead needles."
      - "A dark wood of ancient oaks, their branches intertwined overhead like a vault."
      - "Birch and ash grow thick here. Shafts of light pierce the leaf cover at odd angles."
      - "The forest floor is soft with moss. Something rustles in the undergrowth and goes still."
    # TODO: Expand from source material. These are starter templates.

  plains:
    base:
      - "Open grassland rolls to the horizon. The wind bends the dry stalks in waves."
      - "Flat, featureless scrubland under a wide sky. The ground is hard-packed earth."
      - "Tall grass obscures the ground. A faint trail cuts through, barely visible."
      - "A broad plain dotted with clusters of low brush. Distant shapes move against the skyline."
      - "Cracked, dry earth stretches out. Tufts of wiry grass cling to shallow depressions."

  # ... similar pools for: hills, mountains, water, swamp, desert, wasteland, ruins
```

`Narrator` class (or module):
- `describe_hex(hex_data, terrain_type, visited_before, time_of_day) -> str`
  - Selects a random base description from the pool (avoid repeating the last description used for this terrain type)
  - Appends visited modifier if hex has been explored before: "You recognize this area." / "You've passed through here before."
  - Appends feature descriptions if features exist on this hex
- `describe_adjacent_features(current_hex, adjacent_hexes, terrain_types) -> str`
  - For each adjacent hex that has features, check if the current terrain blocks vision
  - If not blocked, generate a directional hint: "To the north, you see smoke rising from what might be a settlement." / "Crumbling structures are visible to the east."
  - Impassable hexes also get mentioned: "A sheer mountain wall blocks passage to the northwest." / "Dark water stretches to the south."
- `describe_movement(from_terrain, to_terrain, direction) -> str`
  - Brief travel sentence: "You head north through the thinning trees." / "You climb a low rise and descend into open grassland."
  - 3-5 variants per terrain transition (can start with 1-2 and expand)

**Tests:** `test_narrator.py`
- `describe_hex` returns a non-empty string for every terrain type
- Calling `describe_hex` twice for the same terrain returns different descriptions (not always, but over 10 calls, at least 2 unique)
- `describe_adjacent_features` includes directional hints for featured adjacent hexes
- `describe_adjacent_features` does NOT include features behind vision-blocking terrain
- Movement into impassable terrain returns a blocking description

---

## Task 1.5: Character Creation Flow

**What:** GM-guided step-by-step character creation as a scene state.

**Files:**
- `src/harsh_realm/gm/scenes/character_creation.py` — CharacterCreation scene handler
- `src/harsh_realm/models/character.py` — Character data model
- `src/harsh_realm/engine/dice.py` — Dice roller (if not already complete from M0)
- `data/classes.yaml` — Class definitions (with placeholders)
- `data/skills.yaml` — Skill definitions
- `data/equipment_kits.yaml` — Starting equipment kits

**Deliverables:**

`Character` dataclass:
```python
@dataclass
class Character:
    id: str
    name: str
    character_class: str         # "warrior", "expert", "adventurer"
    level: int = 1
    xp: int = 0
    xp_next: int = 0             # XP needed for next level
    attributes: dict[str, int]   # {"str": 14, "dex": 12, ...}
    attr_mods: dict[str, int]    # {"str": 1, "dex": 0, ...} derived from attributes
    skills: dict[str, int]       # {"stab": 1, "survive": 0, ...} -1 = untrained
    hp: int = 0
    max_hp: int = 0
    ac: int = 10
    attack_bonus: int = 0
    equipment: list[dict]        # list of item dicts
    class_abilities: dict        # tracks ability usage (veteran_luck_used, etc.)
```

`CharacterCreation` scene handler implementing the `SceneHandler` protocol:

**Step 1 — Name:**
- GM: "Welcome to Harsh Realm. Let's create your character. What is your name?"
- Player types a name. Any non-empty string accepted.
- GM: "Well met, {name}."

**Step 2 — Class:**
- GM: Presents the three classes with brief descriptions:
  - Warrior: "The fighter. Best in combat, can shrug off a hit, deals extra damage."
  - Expert: "The skilled specialist. More versatile, can retry failed skill checks."
  - Adventurer: "A mix of both. Pick two partial class abilities."
- Player types class name (or abbreviation: w/e/a).
- If Adventurer, GM notes: "You gain partial abilities from both Warrior and Expert."

**Step 3 — Roll attributes:**
- GM rolls 4d6-drop-lowest six times. Displays all six results.
- GM: "Your rolls are: 15, 12, 10, 14, 8, 11. Assign each to an attribute."
- GM: "Which score do you want for Strength?" → player types a number from the available pool
- Repeat for DEX, CON, INT, WIS, CHA (in order)
- After each assignment, show remaining scores and remaining attributes
- Calculate and display modifiers after all are assigned

**Step 4 — Derived stats:**
- GM calculates HP (roll class HD + CON mod, minimum 1), AC (10 + DEX mod before armor), attack bonus, saves
- GM displays the results

**Step 5 — Skills:**
- GM shows available skills and starting skill points for the chosen class
- GM: "You have N skill points to allocate. Available skills: [list]. Which skill would you like to invest in?"
- Player types skill name. GM asks for level (0 or 1, depending on point cost rules — see placeholders)
- Repeat until all points allocated or player types "done"
- All unallocated skills default to -1

**Step 6 — Equipment kit:**
- GM presents 2-3 kits appropriate for the class
- Player types kit name or number
- Kit items added to equipment, AC recalculated with armor

**Step 7 — Confirmation:**
- GM displays complete character summary
- GM: "Is this your character? (yes/no)"
- If yes: save character to `entities` table, transition to Exploration scene
- If no: offer to restart from a specific step

The character creation scene tracks its current step internally and handles only the commands appropriate to that step.

**Tests:** `test_character_creation.py`
- Step-by-step flow produces valid commands at each stage
- Attribute generation produces 6 values each in range 3-18
- Assigning all 6 scores to attributes succeeds
- Cannot assign the same score twice
- Character saved to DB after confirmation
- HP is at least 1 regardless of CON modifier
- Skills default to -1 for unallocated skills
- Equipment kit items appear in character equipment
- AC reflects armor from kit

---

## Task 1.6: Command Parser

**What:** Parse player text input into structured commands, context-filtered by scene state.

**Files:**
- `src/harsh_realm/parser/parser.py` — Parsing logic
- `src/harsh_realm/parser/commands.py` — Command definitions and aliases

**Deliverables:**

`ParsedCommand` dataclass:
```python
@dataclass(frozen=True)
class ParsedCommand:
    verb: str              # normalized verb: "go", "look", "status", etc.
    target: str | None     # "north", "sword", "goblin", etc.
    modifiers: dict[str, str]  # additional qualifiers
    raw: str               # original input text
```

`CommandParser` class:
- `parse(input_text: str, valid_verbs: list[str] | None = None) -> ParsedCommand`
  - Normalize input: strip whitespace, lowercase
  - Match against verb alias table
  - Extract target (first word after verb, or the verb itself for directional shortcuts)
  - If `valid_verbs` is provided and the parsed verb isn't in the list, return a command with verb `"unknown"`
  - Handle directional shortcuts: bare `n`, `ne`, `se`, `s`, `sw`, `nw` → `ParsedCommand(verb="go", target="north")`, etc.

Verb alias table:
```python
VERB_ALIASES = {
    "go": ["go", "move", "walk", "head", "travel"],
    "look": ["look", "examine", "inspect", "l", "x"],
    "status": ["status", "stats", "character", "sheet", "char"],
    "help": ["help", "?", "commands"],
    "save": ["save"],
    "quit": ["quit", "exit"],
}

DIRECTION_ALIASES = {
    "north": ["north", "n"],
    "northeast": ["northeast", "ne"],
    "southeast": ["southeast", "se"],
    "south": ["south", "s"],
    "southwest": ["southwest", "sw"],
    "northwest": ["northwest", "nw"],
}
```

**Tests:** `test_parser.py`
- "go north" → verb="go", target="north"
- "n" → verb="go", target="north"
- "ne" → verb="go", target="northeast"
- "look" → verb="look", target=None
- "look around" → verb="look", target="around" (or None, either is fine)
- "status" → verb="status", target=None
- "char" → verb="status", target=None (alias)
- "help" → verb="help", target=None
- "?" → verb="help", target=None
- "xyzzy" → verb="unknown", raw="xyzzy"
- "  GO   NORTH  " → verb="go", target="north" (whitespace/case normalized)
- With valid_verbs=["go", "look"], "attack goblin" → verb="unknown"

---

## Task 1.7: GM Controller & Exploration Scene

**What:** The GM state machine and the Exploration scene handler that ties everything together.

**Files:**
- `src/harsh_realm/gm/controller.py` — GM state machine
- `src/harsh_realm/gm/scenes/base.py` — SceneHandler protocol
- `src/harsh_realm/gm/scenes/exploration.py` — Exploration scene handler

**Deliverables:**

`GMController` class:
- Holds current `SceneState` (enum value)
- Holds reference to `EventBus`, `WorldDatabase`, `CommandParser`, `Narrator`
- `handle_input(text: str) -> list[GameEvent]`:
  1. Parse command using `CommandParser` with valid verbs from current scene
  2. If unknown command: emit `gm.narrate` event with help text
  3. Otherwise: delegate to current scene handler's `handle_command`
  4. Process returned events through event bus
  5. Check scene handler's `check_transitions` for scene state changes
  6. Return all generated events (for WebSocket forwarding)
- `get_prompt() -> str`: Ask current scene handler for its prompt text
- `transition_to(new_state: SceneState)`: Change scene, emit `gm.scene_change` event

`SceneHandler` protocol (in `base.py`):
```python
class SceneHandler(Protocol):
    def get_valid_commands(self) -> list[str]: ...
    def get_prompt(self, db: WorldDatabase) -> str: ...
    def handle_command(self, cmd: ParsedCommand, db: WorldDatabase) -> list[GameEvent]: ...
    def check_transitions(self, events: list[GameEvent]) -> SceneState | None: ...
```

`ExplorationScene` handler:
- Valid commands: `go`, `look`, `status`, `help`, `save`, `quit`
- `handle_command` for `go`:
  1. Resolve direction from command target
  2. Get target hex coordinates from current position + direction
  3. Check if target hex exists and is passable
  4. If impassable: emit `gm.narrate` event with blocking description, stay in place
  5. If passable: update character location in DB, mark hex as explored, emit `action.move` event
  6. Generate movement description (narrator)
  7. Generate new hex description with adjacent feature visibility (narrator)
  8. Emit `gm.narrate` event(s) with the descriptions
  9. Emit `exploration.enter_hex` event with hex data
- `handle_command` for `look`:
  1. Generate description of current hex (narrator)
  2. Include adjacent feature visibility
  3. Emit `gm.narrate` event
- `handle_command` for `status`:
  1. Format character sheet as text
  2. Emit `gm.narrate` event with the formatted sheet
- `handle_command` for `help`:
  1. Emit `gm.narrate` event listing available commands with brief descriptions
- `handle_command` for `save`:
  1. Create named snapshot via WorldDatabase
  2. Emit `gm.narrate` confirming save
- `get_prompt`: Return empty string or subtle prompt (the chat input is always visible)
- `check_transitions`: For Milestone 1, no transitions (combat/social/etc. come later). Always returns None.

**Tests:** `test_gm_controller.py`
- GM starts in CharacterCreation state (or Menu → CharacterCreation flow)
- After character creation completes, GM transitions to Exploration
- In Exploration: "go north" produces movement + description events
- In Exploration: "go" into impassable terrain produces blocking narration, position unchanged
- In Exploration: "look" produces description events
- In Exploration: "status" produces character sheet events
- In Exploration: "help" produces command list
- In Exploration: unknown command produces helpful error message
- Position persists in DB after movement

---

## Task 1.8: WebSocket Integration & Narration Display

**What:** Wire the GM Controller into the WebSocket handler so player commands flow through the GM and narration flows back to the frontend.

**Files:**
- `src/harsh_realm/api/websocket.py` (modify from Milestone 0)
- `frontend/src/components/ChatLog.vue` (modify)

**Deliverables:**

Backend changes:
- Replace the echo behavior from Milestone 0 with GM Controller integration
- On receiving a `{"type": "command", "text": "..."}` message:
  1. Pass text to `GMController.handle_input()`
  2. Collect returned events
  3. For each `gm.narrate` event, send a `{"type": "narration", "text": "...", "source": "gm"}` message to the client
  4. For other events, send as `{"type": "game_event", "event": {...}}` (as before)
- On world load, initialize GMController with appropriate starting scene state:
  - If no character exists in the world → CharacterCreation
  - If character exists → Exploration (at last saved position)
- On WebSocket connect, send initial prompt/scene description

Frontend changes:
- Distinguish message types visually:
  - Player input: prefixed with `> ` in a distinct color (e.g., lighter text)
  - GM narration: no prefix, main text color
  - System messages: dimmer color (connection status, save confirmations)
- Narration text may contain multiple paragraphs — render them with proper spacing
- Chat log auto-scrolls on new messages

**Tests:** `test_integration_m1.py`
- Create world → connect via WebSocket → receive character creation prompt
- Complete character creation via WebSocket commands → receive exploration scene description
- Send "go north" → receive movement narration + new hex description
- Send "look" → receive current hex description
- Send "status" → receive formatted character sheet
- Close and reopen connection → character and position persist, exploration resumes

---

## Task 1.9: World Creation Flow

**What:** When creating a new world, automatically generate the hex map and prepare for character creation.

**Files:**
- `src/harsh_realm/api/routes.py` (modify world creation endpoint)

**Deliverables:**

Modify `POST /api/worlds` to accept optional generation parameters:
```json
{
  "name": "Ashfall",
  "width": 20,
  "height": 20,
  "seed": null
}
```

On world creation:
1. Create the SQLite database (as before)
2. Load terrain definitions from `data/terrain.yaml`
3. Run `WorldGenerator.generate_region()` with the specified dimensions
4. Run feature placement
5. Set `gm_state` scene to `"char_create"`
6. Return world info including hex count and feature summary

Also add: `GET /api/worlds/current/map` — returns the hex map data as JSON (for debugging and future frontend use):
```json
{
  "width": 20,
  "height": 20,
  "hexes": [
    {"q": 0, "r": 0, "terrain": "forest", "features": [], "explored": false},
    ...
  ]
}
```

**Tests:**
- Create world with seed → map generated with expected hex count
- Create world → GM state is "char_create"
- Map endpoint returns all hexes with valid terrain types

---

## Dependency Graph

```
Task 1.1 (hex model + terrain)
  ↓
Task 1.2 (map generator) → Task 1.3 (feature placement)
  ↓
Task 1.4 (description templates)
  ↓
Task 1.5 (character creation) ← needs dice.py, class/skill YAML
  ↓
Task 1.6 (command parser)
  ↓
Task 1.7 (GM controller + exploration scene) ← ties everything together
  ↓
Task 1.8 (WebSocket integration + frontend)
  ↓
Task 1.9 (world creation flow)
```

Tasks 1.1-1.3 (map generation) and Task 1.5 (character creation) can be developed in parallel since they're independent until Task 1.7 connects them.

---

## Content Stubs Needed

The following YAML files need to exist with at least placeholder content. The coding agent should create stubs with 3-5 entries each. The developer will expand them from source material.

| File | Content | Minimum stub entries |
|---|---|---|
| `data/terrain.yaml` | Terrain type definitions | All 9 types (fully defined in Task 1.1) |
| `data/templates/terrain_descriptions.yaml` | Description text pools per terrain | 3 descriptions per terrain type |
| `data/templates/movement_descriptions.yaml` | Travel transition text | 2 per common terrain transition |
| `data/templates/adjacent_hints.yaml` | Directional feature visibility text | 2 per feature type |
| `data/templates/blocked_passage.yaml` | Impassable terrain descriptions | 2 per impassable type |
| `data/tables/terrain/terrain_weights.yaml` | Generation weights + adjacency | Full table (defined in Task 1.2) |
| `data/tables/terrain/features.yaml` | Feature types and placement rules | 5 feature types |
| `data/tables/names/settlement_names.yaml` | Settlement name list | 20 names |
| `data/tables/names/landmark_names.yaml` | Landmark name list | 15 names |
| `data/tables/names/ruin_names.yaml` | Ruin site name list | 15 names |
| `data/classes.yaml` | Class definitions (with placeholders) | 3 classes |
| `data/skills.yaml` | Skill definitions | All 19 skills |
| `data/equipment_kits.yaml` | Starting equipment kits | 2 kits per class (6 total) |

---

## Notes for the Coding Agent

- Read `docs/rules_reference/attributes.md`, `classes.md`, and `skills.md` before implementing character creation. Many values are marked `[PLACEHOLDER]` — use reasonable defaults and mark them in code.
- The Narrator module is important for the feel of the game. Invest in making descriptions varied and atmospheric. The template system should make it easy to add more variants without code changes.
- The GM Controller is the most architecturally significant piece. Get the SceneHandler protocol right — every future milestone adds new scene handlers, and they all need to work the same way.
- Adjacent hex visibility is a small feature but matters a lot for the exploration experience. Don't skip it.
- Character creation is a multi-turn conversation. The scene handler needs to track which step it's on and only accept appropriate input for that step. Invalid input should get a helpful response, not an error.
- After completing all tasks, update `CLAUDE.md` with "Milestone 1 complete" and note any deviations or issues.
