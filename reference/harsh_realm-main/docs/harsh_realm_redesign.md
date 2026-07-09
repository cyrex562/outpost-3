# Harsh Realm — Architecture & Milestone Plan

> **Date:** 2026-03-12
> **Revised:** 2026-03-12 (v3 — XWN base system, agent-driven development workflow)
> **Purpose:** A focused design for a single-player MUD with procedural world and expert-system GM, built to be playable fast and deepened incrementally.

---

## Table of Contents

1. [What Harsh Realm Is](#1-what-harsh-realm-is)
2. [Mechanical Foundation](#2-mechanical-foundation)
3. [Technology Stack](#3-technology-stack)
4. [Architecture Overview](#4-architecture-overview)
5. [Data Model (SQLite)](#5-data-model-sqlite)
6. [Subsystem Design](#6-subsystem-design)
7. [Frontend](#7-frontend)
8. [Milestone Plan](#8-milestone-plan)
9. [Repository Layout](#9-repository-layout)
10. [Development Workflow & Agent Strategy](#10-development-workflow--agent-strategy)
11. [Design Decisions Summary](#11-design-decisions-summary)
12. [Resolved Questions](#12-resolved-questions)
13. [Open Questions](#13-open-questions)

---

## 1. What Harsh Realm Is

A **single-player MUD with a procedural world and an expert-system GM.** The player interacts primarily through a text command interface (with structured UI elements alongside it). The software handles all mechanical resolution, world generation, NPC behavior, and narrative presentation. The world persists between sessions and simulates autonomously at a coarse level (faction actions, NPC activities, world events) so that time passes meaningfully.

### Core Experience Loop

```
┌──────────────────────────────────────────────────────────┐
│  1. GM presents the current situation                    │
│     (location, what you see, anything demanding          │
│      attention, ambient world events)                    │
│                                                          │
│  2. Player types a command or question                   │
│     "look around" / "go north" / "attack the bandit" /  │
│     "is there a hidden door?" / "check my inventory"     │
│                                                          │
│  3. System resolves the action                           │
│     - Parse command → structured action                  │
│     - Mechanics check (skill roll, combat, etc.)         │
│     - World state update                                 │
│     - Consequence generation (events cascade)            │
│                                                          │
│  4. GM narrates the result                               │
│     - Text description in the log                        │
│     - UI updates (HP bar, map reveal, inventory)         │
│     - If new scene/mode triggered, transition            │
│                                                          │
│  5. World tick (if time passed)                           │
│     - Faction actions, NPC movement, random events       │
│     - Clock advances                                     │
│                                                          │
│  └──→ Back to step 1                                     │
└──────────────────────────────────────────────────────────┘
```

### Design Principles

1. **Playable at every milestone.** Each development phase produces something you can sit down and interact with.
2. **Tables and generators before rules.** A world to explore matters more than perfect combat mechanics.
3. **Chat-first, components later.** The text log is the primary interface. Maps, status panels, and forms are progressive enhancements.
4. **Events everywhere.** All state changes flow through an event bus. This enables logging, undo, extensibility, and real-time UI updates.
5. **One world = one file.** Each world is a SQLite database you can copy, back up, swap, or delete.
6. **XWN as the mechanical foundation.** Stars/Worlds Without Number provides the core rules. House rules extend where needed. Content from other RPGs (GURPS, Rifts, Traveller, Starfinder, Five Parsecs, Cepheus/Hostile) is adapted into XWN-compatible form before encoding.
7. **Extension points, not abstractions.** The codebase has clear, named places where house rules and new mechanics slot in. The system doesn't try to be a generic RPG engine — it's an XWN engine with well-defined customization hooks.

### Starting Campaign Concept

**Tone:** Blade Runner meets Alien meets Dune. Dark, gritty, hostile. Technology exists but most of it is broken, hoarded, or forgotten. The people in charge are awful. The world itself is trying to kill you. Monsters are real — both natural predators and things humans made before the collapse. Secrets exist because humans don't truly forget, but they must be rediscovered in ruins, data vaults, and the wreckage of a fallen civilization.

**Setting:** A **feudal planet cut off from interstellar civilization.** A standard SWN "lost colony" world, generations after the Silence (or equivalent collapse event). The baseline tech level is TL3 (medieval/early industrial) with scattered TL4+ relics. Key setting elements:

| Element | Detail |
|---|---|
| Tech baseline | TL3: hand-forged weapons, animal labor, basic chemistry, crude firearms |
| Pretech relics | Scattered in ruins and hoarded by the powerful. Energy weapons, medical tech, data cores, powered armor fragments. Feudal lords use hoarded tech to maintain dominance over their subjects. |
| Ruins | Abundant and largely unexplored. Factories, research stations, military installations, starports. Dangerous (automated defenses, structural collapse, creatures nesting) but full of recoverable technology. |
| The goal | Somewhere on this planet, a starship (or the parts to build/repair one) exists. Finding it is the long-term campaign arc — but it requires exploring ruins, dealing with or overthrowing the factions that control access, and surviving the hostile world long enough to get there. |
| Firearms & chemistry | Work normally. No abstraction into "workings" or magical effects. A gun is a gun. Chemistry produces explosives, medicine, and poisons through understood (if sometimes rediscovered) processes. |
| Monsters | Real and varied. Natural megafauna adapted to a harsh world, plus engineered creatures from before the collapse (bioweapons, guard beasts, escaped experiments). Not fantasy monsters — things with ecological or engineered reasons to exist. |
| The powerful | Feudal lords, merchant combines, religious orders, military brotherhoods. All corrupt to varying degrees. They hoard pretech, control populations through force and ignorance, and fight each other for dominance. Some are merely selfish; others are genuinely monstrous. |
| The people | Struggling under oppression, adapted to hardship, wary of outsiders. Knowledge is fragmented — a blacksmith might understand metallurgy but have no concept of electronics. Communities are insular and suspicious. |

**Factions as active threats:** When the player angers a faction, that faction becomes a source of hostile encounters. Faction disposition directly modifies encounter tables — an unfriendly faction sends patrols, spies, and bounty hunters into the player's vicinity. An enemy faction actively hunts the player. This means faction turns (weekly) have immediate gameplay consequences, not just background flavor.

This gives a natural development progression:

- **Phase 1 (Milestones 0-5):** Planetary sandbox. Medieval/industrial world with ruins to explore, factions to navigate, and survival as the baseline challenge.
- **Phase 2 (post-Milestone 6):** Introduce SWN interstellar elements as the character gains access to technology. The starship arc becomes possible.
- **Phase 3 (future):** Full SWN space gameplay — sector generation, ship combat, planet hopping. New planets with different conditions and factions.

---

## 2. Mechanical Foundation

### Base System: XWN (Shared Core of WWN/SWN/CWN/AWN)

The *Without Number family shares a common mechanical core:

| Mechanic | Implementation |
|---|---|
| Skill checks | 2d6 + skill modifier + attribute modifier vs. target (default 8+, adjustable) |
| Attack rolls | d20 + attack bonus + modifiers vs. target's AC |
| Saving throws | d20 + level-based save vs. target (15 - half level, round down) |
| Hit Points | Class-based HD per level (d6 Experts, d8 Warriors) + CON modifier |
| Attributes | Classic six: STR, DEX, CON, INT, WIS, CHA. 3d6 or array. Modifiers: -2 to +2 |
| Armor Class | Ascending. Unarmored = 10, modified by DEX and armor worn |
| Damage | Weapon-based dice. Melee adds STR modifier, ranged adds DEX modifier (optional) |
| Initiative | d8 + DEX modifier per side or per individual |
| Encumbrance | Slot-based: readied items (quick access) + stowed items (backpack) |

### Starting Classes

| Class | Role | Key mechanic |
|---|---|---|
| Warrior | Combat specialist | +1 hit bonus/level, once/fight negate a hit or force a miss, bonus damage |
| Expert | Skilled specialist | Reroll one failed skill check/scene, bonus skill points, wide skill access |
| Adventurer | Partial class (pick 2) | Mix of partial Warrior + partial Expert abilities |

Mage/Psychic classes deferred until magic/psionics are implemented.

### XWN Skill List (Starting Set)

Administer, Connect, Exert, Fix, Heal, Know, Lead, Notice, Perform, Pilot, Program, Punch, Shoot, Sneak, Stab, Survive, Talk, Trade, Work

Skills range from -1 (untrained) to +4 (master). Skill check = 2d6 + skill + attribute modifier vs. difficulty (typically 8 for routine, 10 for challenging, 12+ for hard).

### Combat Summary

1. Roll initiative (d8 + DEX mod)
2. On your turn: move + one action (attack, skill check, item use, etc.)
3. Attack: d20 + attack bonus + skill (Stab/Shoot/Punch) + attribute mod vs. AC
4. Damage: weapon die + attribute mod. Warriors add half level (round up) to damage.
5. At 0 HP: dying. Allies can stabilize with a Heal check. Without aid, death in 6 rounds.
6. Healing: natural rest restores 1 HP/day (plus level after full rest). First aid restores 1d6+skill HP once after combat.

### Faction Turn System (WWN/SWN Native)

This runs as its own subsystem alongside individual-scale play:

| Concept | Description |
|---|---|
| Factions | Organizations with HP, assets, and goals |
| Assets | Units a faction controls (military, economic, special). Each has stats, cost, upkeep |
| Faction turn | Each faction takes one action per turn: attack, expand, create asset, repair, etc. |
| Turn frequency | One faction turn per in-game **week** (accelerated from standard monthly for solo pacing). Adjustable to every few days if weekly still feels sparse. |
| Player interaction | Player actions can damage/help factions. Faction actions create world events the player encounters |

Faction HP, assets, and actions use their own mechanics (not XWN individual-scale rules). This is by design — the faction system is a strategy layer that generates emergent situations for the player.

**Faction disposition → encounter table modification:** A faction's attitude toward the player directly affects what encounters appear in their territory:

| Disposition | Encounter Effect |
|---|---|
| Allied | Faction patrols ignore or help the player. Safe passage, trade opportunities. |
| Friendly | Reduced hostile encounters in faction territory. NPCs are helpful. |
| Neutral | Standard encounter tables. |
| Unfriendly | Faction sends patrols and spies. Increased hostile encounter frequency. NPCs are suspicious. |
| Hostile | Faction actively hunts the player. Bounty hunters, ambushes, wanted posters. Encounter tables heavily weighted toward faction military assets. |

This means pissing off a powerful faction has immediate, tangible consequences in the moment-to-moment gameplay — not just in the background strategy layer.

### House Rules & Extension Points

The codebase defines explicit **extension points** where house rules modify or extend base XWN mechanics. Each extension point has a default implementation (base XWN) and a hook for custom behavior.

```python
# Extension point pattern:
class SkillCheckResolver:
    """Default: XWN 2d6 + skill + attr_mod vs. target."""
    
    def resolve(self, skill: str, attr_mod: int, skill_level: int, 
                difficulty: int = 8, modifiers: list[Modifier] = []) -> SkillCheckResult:
        """Override this method to add house rules."""
        roll = dice.roll_2d6()
        total = roll + skill_level + attr_mod + sum(m.value for m in modifiers)
        margin = total - difficulty
        return SkillCheckResult(
            roll=roll, total=total, target=difficulty,
            margin=margin, success=total >= difficulty,
            natural_2=(roll == 2), natural_12=(roll == 12),
        )
```

Planned extension points:

| Extension Point | Default (XWN) | Example House Rules |
|---|---|---|
| `SkillCheckResolver` | 2d6 + mod vs 8+ | Margin-of-success effects, critical ranges, Traveller-style task chains |
| `AttackResolver` | d20 + AB vs AC | Called shots, hit location tables (from GURPS), combat maneuvers |
| `DamageResolver` | Weapon die + mods | Wound severity (cutting/impaling multipliers from GURPS), limb damage |
| `InitiativeResolver` | d8 + DEX mod | Individual vs. side initiative, speed-factor initiative |
| `EncounterGenerator` | XWN encounter tables | Rifts-style random encounters, ecological encounter chains |
| `NPCGenerator` | XWN NPC tables | Traveller career backgrounds, Starfinder species, GURPS personality |
| `PlanetGenerator` | SWN world tags | Traveller UWP classification, extended atmosphere/gravity rules |
| `WeatherSystem` | Simple seasonal table | Detailed weather simulation with mechanical effects on travel/combat |
| `AdvancementResolver` | XP → level up | Usage-based skill improvement overlay, milestone bonuses |
| `FactionTurnResolver` | WWN/SWN faction rules | Extended faction assets, custom faction actions |

House rules are documented in markdown files in `docs/house_rules/` and implemented as Python modules in `src/harsh_realm/house_rules/`. The coding agent reads the markdown spec and implements the module.

### Advancement System

XWN uses standard XP → level advancement. XP is earned from:

- Surviving dangerous encounters
- Achieving goals / completing objectives
- Discovering significant locations or secrets
- Faction-relevant accomplishments

**House rule overlay (planned):** usage-based skill improvement.

```
Character uses Stab in combat → accumulate "practice" ticks for Stab
Hit a threshold → skill improves (or earns a bonus skill point to allocate)
This supplements, not replaces, the standard level-up skill point allocation.

Thresholds scale with current skill level:
  -1 → 0: 10 practice ticks
   0 → 1: 25 practice ticks
   1 → 2: 50 practice ticks
   2 → 3: 100 practice ticks
   3 → 4: 200 practice ticks

Practice tick rates:
  Routine success: 0.5
  Contested/pressured success: 1.0
  Narrow success (margin 0-1): 1.5
  Natural 12: 3.0
  Meaningful failure: 0.5
```

### Content Adaptation Pipeline

Content from non-XWN systems is adapted to XWN terms before encoding:

```
Source material (GURPS monster, Rifts equipment, Traveller planet)
  → Designer (Josh) reads source, makes conversion decisions
    → Documents in markdown or fills in YAML template
      → Coding agent implements if new mechanics needed
        → Content appears in game as XWN-compatible entity
```

Examples of adaptation:
- **GURPS monster → XWN statblock:** Map ST/DX/IQ/HT to XWN attributes (STR/DEX/CON/INT/WIS/CHA), convert damage to XWN dice, assign HD based on HP, assign AC based on DR, map skills to XWN skills.
- **Traveller UWP → extended planet tags:** Keep SWN world tags but add Traveller-style starport class, atmosphere type, hydrosphere percentage, government type, law level, tech level as additional metadata.
- **Rifts equipment → XWN gear:** Convert MDC to XWN damage scale, assign cost in credits, add SWN-style encumbrance slots.

The designer (Josh) does the conversion judgment calls. The system stores the result in XWN-native format.

---

## 3. Technology Stack

| Layer | Technology | Rationale |
|---|---|---|
| Language | Python 3.12+ | Rapid iteration, rich ecosystem, AI-assistable |
| Web framework | FastAPI + uvicorn | Async, WebSocket support, minimal |
| Database | SQLite (via aiosqlite) | Embedded, single-file-per-world, transactional |
| Reference data | YAML files loaded at startup | Easy to author, human-readable content encoding |
| Real-time | WebSocket (FastAPI native) | Push events to frontend without polling |
| Frontend | Vue 3 + TypeScript + Vite | Familiar ecosystem, AI-assistable, good component model |
| Styling | Tailwind CSS | Utility-first, fast to prototype |
| Testing | pytest + playwright | Unit/integration for backend, E2E for frontend |
| Optional future | Rust (PyO3) | Performance-critical simulation if/when needed |

---

## 4. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Vue 3 Frontend                           │
│  ┌─────────┐  ┌──────────┐  ┌──────────┐  ┌────────────────┐   │
│  │ Chat/Log│  │ Map Panel│  │ Status   │  │ Context Panels │   │
│  │ + Input │  │ (hex/room)│ │ (HP, etc)│  │ (inv, skills)  │   │
│  └────┬────┘  └────┬─────┘  └────┬─────┘  └──────┬─────────┘   │
│       └─────────┬──┴────────────┬┘               │              │
│            WebSocket + REST API                                  │
└─────────────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────────────┐
│                     FastAPI Backend                              │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    GM Controller                          │   │
│  │  (State machine: Exploration | Combat | Social |          │   │
│  │   Travel | Rest | Dungeon | CharacterCreation | Menu)     │   │
│  │                                                           │   │
│  │  Knows what mode we're in, what commands are valid,       │   │
│  │  what to present next. Delegates all work via events.     │   │
│  └──────────┬───────────────────────────────────┬───────────┘   │
│             │ emits events                      │ reads state    │
│  ┌──────────▼──────────┐          ┌─────────────▼────────────┐  │
│  │    Event Bus         │          │    World State (SQLite)   │  │
│  │                      │          │                           │  │
│  │  publish(event)      │          │  Entities, components,    │  │
│  │  subscribe(type, fn) │          │  map hexes, factions,     │  │
│  │  history log         │          │  NPCs, items, clock       │  │
│  └──┬───┬───┬───┬───┬──┘          └───────────────────────────┘  │
│     │   │   │   │   │                                            │
│  ┌──▼┐ ┌▼──┐ ┌─▼─┐ ┌▼──────┐ ┌──▼──────┐                      │
│  │Cmbt│ │Nav│ │NPC│ │Tables │ │Narrator │  ← Subsystems         │
│  │Sys │ │Sys│ │Sys│ │& Gens │ │/ Output │    (event handlers)   │
│  └────┘ └───┘ └───┘ └───────┘ └─────────┘                      │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Command Parser                               │   │
│  │  Input text → structured Action (or OracleQuery)          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Rules Engine (XWN + House Rules)             │   │
│  │  Dice, skill checks, attack resolution, damage, saves,   │   │
│  │  with named extension points for house rules              │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Oracle System                                │   │
│  │  Mythic-style fate chart, chaos factor, scene interrupts, │   │
│  │  context-modified yes/no/and/but resolution               │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Faction System (WWN/SWN native)              │   │
│  │  Faction turns, assets, goals, actions, world events      │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### The GM Controller — Hybrid State Machine + Event Bus

The GM Controller is the orchestration brain. It maintains a **scene state** that determines the current mode of play. Each mode defines:

- **Valid commands** (what the player can do right now)
- **Prompt behavior** (what the GM tells you when it's your turn to act)
- **Tick behavior** (what happens automatically each turn/round)
- **Transition triggers** (what causes a mode switch)

The GM does NOT implement game logic directly. When the player types "attack the bandit," the GM:

1. Confirms we're in a mode where `attack` is valid
2. Emits an `ActionRequested(Attack { target: "bandit" })` event
3. The **Combat System** (an event handler) picks it up, resolves the attack via the Rules Engine, updates world state, and emits result events
4. The **Narrator** (another event handler) picks up the result events and produces text output
5. The GM sees the result events and decides if the scene state changes

This means:
- **Adding a new subsystem** = write event handlers and subscribe them
- **Tuning game feel** = adjust the GM controller's transition logic and prompting
- **Debugging flow** = look at the event log; every decision is traceable

#### Scene States

```python
class SceneState(Enum):
    MENU = "menu"                       # World selection, settings
    CHARACTER_CREATION = "char_create"  # Building a new character
    EXPLORATION = "exploration"         # Grid-crawl overworld (hex grid)
    DUNGEON = "dungeon"                 # Room-by-room interior (square grid)
    COMBAT = "combat"                   # Turn-based tactical
    SOCIAL = "social"                   # NPC interaction
    TRAVEL = "travel"                   # Long-distance movement (montage)
    REST = "rest"                       # Camp/inn, recovery, downtime
    SHOPPING = "shopping"               # Merchant interaction
```

Each state implements:

```python
class SceneHandler(Protocol):
    def get_valid_commands(self) -> list[CommandSpec]: ...
    def get_prompt(self, world: WorldState) -> str: ...
    def handle_command(self, cmd: ParsedCommand, world: WorldState) -> list[Event]: ...
    def tick(self, world: WorldState) -> list[Event]: ...
    def check_transitions(self, events: list[Event]) -> SceneState | None: ...
```

### Event Bus

All state changes flow through events. Events are logged, can trigger cascades, and are forwarded to the frontend via WebSocket.

```python
@dataclass
class GameEvent:
    id: str                          # auto-generated UUID
    tick: int                        # world clock
    event_type: str                  # "combat.attack", "movement.enter_hex", etc.
    data: dict                       # payload
    source: str = "system"           # "player", "system", "gm", subsystem name

class EventBus:
    def subscribe(self, event_type: str, handler: Callable) -> None: ...
    def publish(self, event: GameEvent) -> list[GameEvent]: ...
    def subscribe_all(self, handler: Callable) -> None: ...  # wildcard
```

Event type namespace:

```
player.command          # raw player input received
action.move             # movement action resolved
action.attack           # attack action resolved
action.skill_check      # skill check performed
combat.start            # combat initiated
combat.turn_start       # new combat turn
combat.end              # combat resolved
social.dialogue         # NPC interaction
exploration.enter_hex   # player enters a new hex
exploration.discover    # something found
world.tick              # world simulation step
world.faction_action    # a faction did something
oracle.fate_check       # oracle consulted
gm.scene_change         # GM changed scene mode
gm.narrate              # narrative text generated
character.xp_gained     # XP earned
character.level_up      # character leveled up
character.practice      # skill practice tick earned
```

---

## 5. Data Model (SQLite)

Each world is a single `.db` file. Structured columns for frequently queried data; JSON columns for flexible/extensible component data.

### Core Tables

```sql
-- World metadata
CREATE TABLE world_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Grid map (hex or square — table named "hexes" for backwards compat)
CREATE TABLE hexes (
    q           INTEGER NOT NULL,
    r           INTEGER NOT NULL,
    terrain     TEXT NOT NULL,
    features    TEXT DEFAULT '[]',       -- JSON array of feature tags
    explored    INTEGER DEFAULT 0,
    description TEXT,
    faction_id  TEXT,
    data        TEXT DEFAULT '{}',       -- JSON extensible
    PRIMARY KEY (q, r)
);

-- Entities (characters, NPCs, monsters, items, buildings, etc.)
CREATE TABLE entities (
    id          TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    name        TEXT NOT NULL,
    location_q  INTEGER,
    location_r  INTEGER,
    alive       INTEGER DEFAULT 1,
    data        TEXT DEFAULT '{}',       -- JSON: class, level, attrs, skills, HP, AC, inventory
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Factions (WWN/SWN faction turn system)
CREATE TABLE factions (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    hp          INTEGER NOT NULL DEFAULT 7,
    max_hp      INTEGER NOT NULL DEFAULT 7,
    force       INTEGER NOT NULL DEFAULT 0,
    cunning     INTEGER NOT NULL DEFAULT 0,
    wealth      INTEGER NOT NULL DEFAULT 0,
    xp          INTEGER NOT NULL DEFAULT 0,
    home_hex_q  INTEGER,
    home_hex_r  INTEGER,
    goals       TEXT DEFAULT '[]',
    tags        TEXT DEFAULT '[]',
    data        TEXT DEFAULT '{}'
);

-- Faction assets
CREATE TABLE faction_assets (
    id          TEXT PRIMARY KEY,
    faction_id  TEXT NOT NULL REFERENCES factions(id),
    asset_type  TEXT NOT NULL,
    category    TEXT NOT NULL,           -- "force", "cunning", "wealth"
    hp          INTEGER NOT NULL,
    max_hp      INTEGER NOT NULL,
    location_q  INTEGER,
    location_r  INTEGER,
    data        TEXT DEFAULT '{}'
);

-- Faction relationships
CREATE TABLE faction_relations (
    faction_a   TEXT NOT NULL REFERENCES factions(id),
    faction_b   TEXT NOT NULL REFERENCES factions(id),
    disposition TEXT NOT NULL DEFAULT 'neutral',
    history     TEXT DEFAULT '[]',
    PRIMARY KEY (faction_a, faction_b)
);

-- Entity-faction reputation
CREATE TABLE reputation (
    entity_id   TEXT NOT NULL REFERENCES entities(id),
    faction_id  TEXT NOT NULL REFERENCES factions(id),
    score       INTEGER DEFAULT 0,
    PRIMARY KEY (entity_id, faction_id)
);

-- Random tables (loaded from YAML at startup)
CREATE TABLE random_tables (
    id          TEXT PRIMARY KEY,
    category    TEXT NOT NULL,
    name        TEXT NOT NULL,
    entries     TEXT NOT NULL,
    tags        TEXT DEFAULT '[]',
    source      TEXT
);

-- Event log (append-only)
CREATE TABLE event_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    tick        INTEGER NOT NULL,
    event_type  TEXT NOT NULL,
    data        TEXT NOT NULL,
    created_at  TEXT NOT NULL
);

-- GM state
CREATE TABLE gm_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Dungeon instances
CREATE TABLE dungeons (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    hex_q       INTEGER NOT NULL,
    hex_r       INTEGER NOT NULL,
    rooms       TEXT NOT NULL,
    connections TEXT NOT NULL,
    data        TEXT DEFAULT '{}'
);

-- Practice tracking (house rule)
CREATE TABLE practice_log (
    entity_id   TEXT NOT NULL REFERENCES entities(id),
    skill       TEXT NOT NULL,
    ticks       REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (entity_id, skill)
);
```

### World File Management

```python
db = WorldDatabase.create("worlds/ashfall.db", name="Ashfall", settings={...})
db = WorldDatabase.open("worlds/ashfall.db")
worlds = WorldDatabase.list_worlds("worlds/")

# Manual save = named snapshot
db.save_snapshot("before_the_raid")

# Periodic checkpoint (automatic, every N ticks)
# Uses SQLite backup API
```

---

## 6. Subsystem Design

### 6.1 Command Parser

```python
@dataclass
class ParsedCommand:
    verb: str
    target: str | None
    modifiers: dict[str, str]
    raw: str

VERB_ALIASES = {
    "go": ["go", "move", "walk", "head", "travel", "n", "s", "e", "w", "ne", "nw", "se", "sw"],
    "look": ["look", "examine", "inspect", "l", "x"],
    "attack": ["attack", "fight", "hit", "strike"],
    "talk": ["talk", "speak", "ask", "greet"],
    "take": ["take", "pick up", "grab", "get"],
    "use": ["use", "activate", "apply"],
    "inventory": ["inventory", "inv", "i", "items"],
    "status": ["status", "stats", "character", "sheet", "char"],
    "help": ["help", "?", "commands"],
    "oracle": ["oracle", "fate", "is there", "does", "do they"],
    "wait": ["wait", "rest", "camp"],
    "save": ["save"],
    "quit": ["quit", "exit"],
}
```

### 6.2 Rules Engine (XWN + House Rules)

```python
class DiceRoller:
    def roll(self, expression: str) -> RollResult: ...
    def roll_2d6(self) -> int: ...
    def roll_d20(self) -> int: ...
    def roll_with_seed(self, expr: str, seed: int) -> RollResult: ...

class SkillCheckResolver:
    """XWN 2d6 + skill + attr_mod vs. target. Extension point."""
    def resolve(self, skill: str, attr_mod: int, skill_level: int,
                difficulty: int = 8, modifiers: list[Modifier] = []) -> SkillCheckResult: ...

class AttackResolver:
    """XWN d20 + AB + skill + attr_mod vs. AC. Extension point."""
    def resolve(self, attacker: Entity, target: Entity,
                weapon: Weapon, modifiers: list[Modifier] = []) -> AttackResult: ...

class DamageResolver:
    """Weapon die + mods. Extension point."""
    def resolve(self, weapon: Weapon, attacker: Entity,
                hit_margin: int = 0) -> DamageResult: ...

class SaveResolver:
    """XWN saving throw: d20 vs 15 - half_level."""
    def resolve(self, save_type: str, level: int,
                modifiers: list[Modifier] = []) -> SaveResult: ...

@dataclass
class SkillCheckResult:
    roll: int
    total: int
    target: int
    margin: int
    success: bool
    natural_2: bool
    natural_12: bool
```

### 6.3 Oracle System

```python
class OracleSystem:
    chaos_factor: int = 5

    def fate_check(self, likelihood: Likelihood) -> FateResult: ...
    def scene_check(self) -> SceneModification: ...
    def random_event(self) -> RandomEvent: ...
    def adjust_chaos(self, direction: int) -> None: ...

class Likelihood(Enum):
    IMPOSSIBLE = "impossible"
    NO_WAY = "no_way"
    VERY_UNLIKELY = "very_unlikely"
    UNLIKELY = "unlikely"
    EVEN = "50/50"
    LIKELY = "likely"
    VERY_LIKELY = "very_likely"
    SURE_THING = "sure_thing"
    HAS_TO_BE = "has_to_be"
```

### 6.4 Random Table Engine

```yaml
# data/tables/encounters_forest.yaml
id: encounters_forest
category: encounter
name: Forest Encounters
tags: [wilderness, forest, temperate]
entries:
  - weight: 3
    result: { type: "creature", table: "forest_creatures" }
  - weight: 2
    result: { type: "npc", table: "traveling_npcs", context: "forest" }
  - weight: 2
    result: { type: "discovery", table: "forest_discoveries" }
  - weight: 1
    result: { type: "event", table: "weather_events" }
  - weight: 1
    result: { type: "hazard", table: "natural_hazards_forest" }
  - weight: 1
    result: { type: "nothing", description: "The forest is quiet." }
```

```python
class TableEngine:
    def roll_on(self, table_id: str, context: dict | None = None) -> TableResult: ...
    def roll_with_tags(self, category: str, tags: list[str]) -> TableResult: ...
    def generate(self, generator_id: str, params: dict) -> GeneratedContent: ...
```

### 6.5 World Generator

```python
class WorldGenerator:
    def generate_region(self, width: int, height: int, params: RegionParams) -> None:
        # 1. Assign terrain (noise or table)
        # 2. Place geographic features
        # 3. Place settlements
        # 4. Place POIs (ruins, dungeons, pretech sites)
        # 5. Generate factions (WWN/SWN tables)
        # 6. Assign faction territories
        # 7. Generate faction relationships
        # 8. Place starting NPCs
```

### 6.6 Faction System (WWN/SWN Native)

```python
class FactionSystem:
    def run_faction_turn(self, world: WorldDatabase) -> list[GameEvent]: ...
    def faction_action_attack(self, attacker_asset, defender_asset) -> FactionCombatResult: ...
    def faction_action_expand(self, faction, target_hex) -> ExpansionResult: ...
    def faction_action_create_asset(self, faction, asset_type) -> AssetResult: ...
```

---

## 7. Frontend

### Phase 1: Pure Chat (Milestones 0-1)

```
┌──────────────────────────────────────────────────────┐
│  Harsh Realm                              [Save] [⚙] │
├──────────────────────────────────────────────────────┤
│                                                       │
│  GM: You stand at the edge of the Thornwood...        │
│                                                       │
│  > status                                             │
│                                                       │
│  Kira Voss — Warrior 1                               │
│  STR 14(+1) DEX 12(+0) CON 11(+0)                   │
│  INT 10(+0) WIS 13(+1) CHA  9(+0)                   │
│  HP 9/9  AC 15 (chain mail + shield)                 │
│  Stab-1, Exert-0, Survive-0, Notice-0               │
│  Location: Thornwood Edge (3, -1) — Forest           │
│  XP: 0 / 1500 (Level 2)                             │
│                                                       │
├──────────────────────────────────────────────────────┤
│ > _                                                   │
└──────────────────────────────────────────────────────┘
```

### Phase 2: Chat + Sidebar + Grid Map (Milestone 2)
### Phase 3: Combat tracker, inventory, NPC panels, dice details, dungeon map

---

## 8. Milestone Plan

### Milestone 0: Skeleton (1 week)

**Goal:** End-to-end data flow. No game logic.

**Deliverables:** FastAPI + WebSocket, SQLite world DB, event bus, Vue chat log + input, echo test, world file management, config, pytest setup.

**Acceptance test:** Type "hello" in browser → see echo. Create two worlds, switch between them.

---

### Milestone 1: The Empty World (1-2 weeks)

**Goal:** Grid map, character, movement, descriptions. Text-only.

**Deliverables:** Grid gen (hex world map), terrain types, description templates, character creation flow, XWN character model, command parser (go/look/status/help), GM Controller (Exploration + CharacterCreation scenes), movement, fog of war.

**Acceptance test:** Create world → create character via GM prompts → move in all grid directions → see terrain descriptions → `status` shows correct XWN stats.

---

### Milestone 2: Things to Find (2 weeks)

**Goal:** Content in the world. Discoveries, NPCs, settlements, oracle.

**Deliverables:** Table engine, 10+ YAML tables, POI gen, settlement gen, NPC gen, encounter checks, oracle (Mythic), `examine`, frontend sidebar + SVG grid map.

**Acceptance test:** Explore → find settlement with NPCs → trigger encounter → oracle answers question → grid map shows explored area.

---

### Milestone 3: Danger (2 weeks)

**Goal:** Combat works. You can fight and survive or die.

**Deliverables:** XWN combat (initiative, attack rolls, damage, HP tracking, death/dying), monster generation from tables (HD, AC, damage, behavior), combat scene state with turn prompts, valid commands: `attack <target>`, `flee`, `use <item>`, simple monster AI (fight to the death — approach and attack each round), encounter → combat transitions, loot generation on victory, XP awards, Warrior class ability (negate one hit per fight), death handling (respawn or new character).

**Starting combat is deliberately simple:** attack or flee. No defend/dodge/cover action yet — those come as house rule extensions later. Enemies fight to the death in v1; fleeing enemies and morale checks are a later addition. This keeps the first implementation small and testable.

**Acceptance test:** Explore → encounter triggers combat → fight 3 rounds with visible dice rolls and damage → win → gain XP and loot → HP reduced → rest to heal. Also: lose a fight → death screen → choose respawn with penalty or new character.

---

### Milestone 4: People (2 weeks)

**Goal:** Social encounters, factions, shopping.

**Deliverables:** NPC personality (UNE), social commands + skill checks, disposition, faction reputation, Mythic scene system, random events, chaos factor, shopping, Expert ability.

**Acceptance test:** Talk to NPC → skill check → buy equipment → scene check fires → faction reputation changes.

---

### Milestone 5: Depth (2-3 weeks)

**Goal:** Dungeons.

**Deliverables:** Dungeon gen (square grid), dungeon scene state, traps, treasure, inventory (XWN encumbrance), dungeon encounters, expanded skills, dungeon entry from world map.

**Acceptance test:** Enter dungeon → explore rooms → trap → fight → treasure → exit.

---

### Milestone 5.5: Python Plugins (1-2 weeks)

**Goal:** Extensibility via plugins.

**Deliverables:** Plugin manager, plugin API (events, commands, generators), sandbox, hot-reload, example plugins.

---

### Milestone 6: The Living World (3-4 weeks)

**Goal:** Autonomous world simulation.

**Deliverables:** World tick, WWN/SWN faction turns (full), faction AI, world events, NPC schedules, tension tracker, rumors, travel/rest scenes, between-session summary.

---

## 9. Repository Layout

```
harsh_realm/
├── pyproject.toml
├── config.yaml
├── CLAUDE.md                       # Project context for coding agents
├── AGENTS.md                       # Coding standards and conventions
├── worlds/                         # SQLite world databases (gitignored)
├── src/
│   └── harsh_realm/
│       ├── __init__.py
│       ├── main.py                 # FastAPI entry point
│       ├── config.py
│       ├── db.py                   # SQLite world database interface
│       ├── events.py               # Event bus
│       ├── models/
│       │   ├── character.py        # XWN character model
│       │   ├── entity.py
│       │   ├── grid.py
│       │   ├── hex_map.py
│       │   ├── faction.py
│       │   ├── npc.py
│       │   └── item.py
│       ├── engine/
│       │   ├── dice.py
│       │   ├── skill_checks.py     # Extension point
│       │   ├── combat.py
│       │   ├── saves.py
│       │   ├── advancement.py
│       │   ├── oracle.py
│       │   └── tables.py
│       ├── generators/
│       │   ├── world_gen.py
│       │   ├── settlement_gen.py
│       │   ├── dungeon_gen.py
│       │   ├── npc_gen.py
│       │   ├── encounter_gen.py
│       │   └── loot_gen.py
│       ├── gm/
│       │   ├── controller.py
│       │   ├── narrator.py
│       │   ├── scenes/
│       │   │   ├── base.py
│       │   │   ├── exploration.py
│       │   │   ├── combat.py
│       │   │   ├── social.py
│       │   │   ├── dungeon.py
│       │   │   ├── travel.py
│       │   │   ├── rest.py
│       │   │   ├── shopping.py
│       │   │   └── character_creation.py
│       │   └── tension.py
│       ├── faction/
│       │   ├── faction_turn.py
│       │   ├── faction_ai.py
│       │   └── assets.py
│       ├── house_rules/
│       │   ├── __init__.py         # Registry of active house rules
│       │   └── practice_skills.py
│       ├── simulation/
│       │   ├── world_tick.py
│       │   └── npc_behavior.py
│       ├── plugins/
│       │   ├── manager.py
│       │   ├── api.py
│       │   └── sandbox.py
│       ├── parser/
│       │   ├── parser.py
│       │   └── commands.py
│       ├── api/
│       │   ├── routes.py
│       │   └── websocket.py
│       └── web/
├── data/
│   ├── tables/
│   │   ├── encounters/
│   │   ├── npcs/
│   │   ├── loot/
│   │   ├── terrain/
│   │   ├── settlements/
│   │   └── names/
│   ├── generators/
│   ├── templates/
│   ├── skills.yaml
│   ├── classes.yaml
│   ├── weapons.yaml
│   ├── armor.yaml
│   └── schemas/
├── plugins/
│   ├── examples/
│   └── README.md
├── frontend/
│   ├── src/
│   │   ├── App.vue
│   │   ├── components/
│   │   │   ├── ChatLog.vue
│   │   │   ├── CommandInput.vue
│   │   │   ├── StatusSidebar.vue
│   │   │   └── HexMap.vue
│   │   ├── stores/
│   │   │   ├── game.ts
│   │   │   └── connection.ts
│   │   └── composables/
│   │       └── useWebSocket.ts
│   └── package.json
├── tests/
│   ├── test_dice.py
│   ├── test_skill_checks.py
│   ├── test_combat.py
│   ├── test_tables.py
│   ├── test_oracle.py
│   ├── test_world_gen.py
│   ├── test_gm_controller.py
│   └── test_faction_turn.py
├── docs/
│   ├── ARCHITECTURE.md             # This document
│   ├── house_rules/
│   │   ├── practice_skills.md
│   │   └── README.md
│   ├── rules_reference/            # XWN rules for agent context
│   │   ├── skill_checks.md
│   │   ├── combat.md
│   │   ├── classes.md
│   │   └── faction_turns.md
│   └── archive/
```

---

## 10. Development Workflow & Agent Strategy

### How Development Works

```
Josh ←→ Claude (conversation)
  │
  │  Produce: design docs, rule specs, feature specs + acceptance criteria
  │
  ▼
CLAUDE.md + AGENTS.md + task specs
  │
  ▼
Claude Code / Copilot (coding agent)
  │
  │  Reads: CLAUDE.md, AGENTS.md, docs/rules_reference/, docs/house_rules/,
  │         task spec with acceptance criteria
  │
  │  Builds: implementation + tests
  │
  ▼
Josh playtests → feedback → iterate → next feature
```

### What the Coding Agent Needs

1. **CLAUDE.md** — Project context: what is Harsh Realm, architecture, conventions, current state.
2. **AGENTS.md** — Coding standards: style, testing, patterns.
3. **docs/rules_reference/** — XWN rules summaries. Agent reads to understand what it's implementing.
4. **docs/house_rules/** — Custom rule specs documented by Josh. Agent implements corresponding modules.
5. **Feature specs with acceptance criteria** — What to build, interfaces, tests, player-visible behavior.

### Coding Standards (for AGENTS.md)

```
- Python 3.12+, black formatter (88-char lines)
- Double quotes for strings
- Type hints on all function signatures
- Pydantic BaseModel for all data models (frozen=True via ConfigDict for immutable value objects)
- Protocol classes for interfaces (not ABC)
- pytest for all tests; parametrize for table-driven tests
- No mutation of shared state outside WorldDatabase transactions
- All game state changes flow through EventBus
- Extension points use resolver pattern (default + overridable)
- YAML for authored content; JSON for SQLite complex data
- Docstrings on all public functions and classes
- Explicit error handling, no silent failures
- Logging via Python logging (structured, with context)
```

---

## 11. Design Decisions Summary

| Decision | Choice | Rationale |
|---|---|---|
| Language | Python 3.12+ | Rapid iteration, AI-assistable |
| Database | SQLite, one file per world | Simple, portable, transactional |
| Frontend | Vue 3, chat-first | Play via text immediately |
| Rules system | XWN + house rules | Native sandbox tools, extensible, light core |
| Starting classes | Warrior, Expert, Adventurer | Magic/Psionics deferred |
| Advancement | XWN XP/level + usage-based skill practice | Standard + depth overlay |
| Party control | Dual-mode: manual or AI per member | Flexible per-situation |
| GM architecture | State machine + event bus | Centralized flow, decoupled logic |
| Oracle | Mythic GME | Proven solo system |
| Faction system | WWN/SWN native faction turns | Best sandbox faction sim available |
| Content format | YAML → SQLite | Human-writable, fast to query |
| Content adaptation | Other RPGs → XWN by designer | GURPS, Rifts, Traveller etc. adapted before encoding |
| Commands | Keyword matching, context-filtered | Simple now, growable |
| LLM | Optional later | Not core to the loop |
| Persistence | SQLite = live state + snapshots | Always current |
| Extensibility | House rule modules + Python plugins + YAML tables | Three layers of customization |
| Map scales | Hex grid (overworld) + square grid (dungeons/towns) | Two grid types, injected via Grid protocol |
| Combat scope | Melee → ranged → tech → magic | Incremental |
| Death | Respawn with consequences or new character + difficulty settings | Player controls punishment |
| Quests | Emergent from factions + world events + oracle | Open-world |
| Setting | SWN feudal planet → expand to space | Blade Runner + Alien + Dune. Dark, gritty, hostile. |
| Faction turns | Weekly (adjustable to every few days) | Accelerated for solo pacing |
| Faction threat | Disposition modifies encounter tables | Hostile factions actively hunt the player |
| Combat v1 | Attack + flee, enemies fight to death | Simple first, extend later |
| Development | Agent-driven: spec → code → playtest → iterate | Fast with human-guided design |

---

## 12. Resolved Questions

- **Base rules system:** XWN, not GURPS. GURPS/Rifts/Traveller content adapted to XWN terms.
- **Party management:** Dual-mode, switchable per member per situation.
- **Advancement:** XWN XP/level + usage-based skill practice house rule.
- **Saves:** SQLite = live state. Manual save = snapshot. Periodic checkpoints.
- **Extensibility:** House rule modules + Python plugins + YAML tables.
- **Maps:** Hex grid (overworld) + square grid (dungeons, towns, interiors).
- **Content format:** YAML for everything authored.
- **Death:** Respawn with penalties or new character. Difficulty settings available.
- **Quests:** Emergent from factions, events, NPCs, oracle.
- **Magic:** Deferred entirely. Melee → ranged → tech → magic.
- **Setting:** SWN feudal planet (TL3 + scattered TL4+ relics). Blade Runner + Alien + Dune tone. Feudal lords hoarding pretech, abundant unexplored ruins, hostile world, real monsters. Firearms and chemistry work normally.
- **Faction system:** WWN/SWN native, runs alongside individual-scale XWN rules.
- **Faction turn frequency:** Weekly. Adjustable to every few days if weekly feels sparse.
- **Faction-as-threat:** Disposition directly modifies encounter tables. Hostile factions hunt the player with patrols, bounty hunters, ambushes.
- **Frontend phasing:** Chat-only → sidebar + grid map → richer panels.
- **Combat v1:** Attack + flee only. Enemies fight to the death. Defend/dodge added later as house rule extensions.
- **Starting classes:** Warrior, Expert, Adventurer. Godbound/Legate deferred but on long-term roadmap.
- **Tech treatment:** Technology works as real-world technology. No abstraction into workings or magical effects.

---

## 13. Open Questions

- **Godbound/Legate integration:** When introduced, how to handle workings/magnitude alongside XWN? Separate character type? Prestige class? Acquired through play?
- **Companion AI depth:** Simple ("attack nearest") vs. role-aware? Start simple, but what triggers more sophistication?
- **Narrative quality:** How many template variants needed before repetition becomes annoying? When to add LLM narration?
- **World persistence between sessions:** Tick only when player is active, or real-time with catch-up summary?
- **Traveller/Cepheus integration priority:** Useful for Phase 1 planet-side, or defer to Phase 2 space?
- **Sourcebook encoding workflow:** YAML template for converting tables from books? Helper tool? Batch conversion process?
- **Starting content volume:** How many tables needed for Milestone 2 to feel alive? 10? 20? 50?
- **Ruin/dungeon theming:** How many dungeon "themes" for variety? Pretech facility, cave system, monster lair, collapsed settlement?
- **Economy:** Credits? Coin? Mixed? Pretech artifacts as high-value loot implies black market / collector economy.
- **Weather/environment:** When to introduce mechanical weather effects? Phase 1 or later?
