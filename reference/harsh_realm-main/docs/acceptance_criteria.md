# docs/acceptance_criteria.md

## How to use this document
- Each feature block has a Status and a set of testable criteria
- Status: COMPLETE | PARTIAL | MISSING | PENDING
- Criteria are written as observable player or system outcomes
- This document is the source of truth for bot goal definitions
  and Playwright test coverage
- Update status and criteria when a milestone closes

---

## Summary

| Milestone | Features | Complete | Partial | Missing |
|-----------|----------|----------|---------|---------|
| M0        | 3        | 3        | 0       | 0       |
| M1        | 6        | 6        | 0       | 0       |
| M2        | 7        | 7        | 0       | 0       |
| M3        | 9        | 9        | 0       | 0       |
| M4        | 15       | 15       | 0       | 0       |
| M4.5      | 7        | 4        | 0       | 3       |
| M4.6      | 8        | 8        | 0       | 0       |
| M4.7      | 5        | 5        | 0       | 0       |
| M4.8      | 5        | 5        | 0       | 0       |
| M4.9      | 6        | 4        | 0       | 2       |
| Rules P1  | 5        | 5        | 0       | 0       |

---

## M0 — Foundation

### WebSocket Connection
**Status:** COMPLETE
**Milestone:** M0
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Player connects via WebSocket and receives a welcome message when no world is loaded
- [x] Player command text echoes back as a `player_input` message type
- [x] GameEvents published to EventBus are broadcast to all connected WebSocket clients
- [x] Disconnected clients are removed silently without affecting other connections
- [x] Dead connections are pruned during broadcast

### World File Management
**Status:** COMPLETE
**Milestone:** M0
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] POST `/api/worlds` creates a new world database file and returns 201
- [x] GET `/api/worlds` lists all available world files
- [x] POST `/api/worlds/load` loads a world by filename; returns 404 for missing files
- [x] GET `/api/worlds/current` returns the currently loaded world name
- [x] POST `/api/worlds/save` creates a named snapshot file on disk
- [x] EventLogger writes events to the `event_log` SQLite table on world load

### Vue Chat Interface
**Status:** COMPLETE
**Milestone:** M0
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] Chat log auto-scrolls to the bottom when new messages arrive
- [x] Player input is rendered in green with a `>` prefix
- [x] GM narration is rendered in amber/warm white
- [x] System events are rendered in dim grey italic
- [x] Command input supports arrow-key history navigation (last 50 commands)
- [x] Tab autocompletes against current suggestions list

---

## M1 — The Empty World

### Map Generation
**Status:** COMPLETE
**Milestone:** M1
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] A 20x20 hex grid produces exactly 400 hexes with valid terrain types
- [x] Generated region contains at least 3 distinct passable terrain types
- [x] At least one map border edge is entirely impassable (mountains or water)
- [x] At least one border edge has passable hexes for entry/exit
- [x] Same seed produces identical terrain layout across runs
- [x] 3-5 settlements placed, each at least 3 hex distance apart
- [x] 4-8 ruin sites placed

### Character Creation
**Status:** COMPLETE
**Milestone:** M1
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] Player is prompted for name, class, attributes, skills, equipment kit, and confirmation
- [x] Invalid class name is rejected with an error and the prompt repeats
- [x] Blank Enter at attribute step rolls 3d6 scores for all six attributes
- [x] Skill points can be raised and lowered; points refund on lower
- [x] Typing "done" with unspent skill points skips to kit selection
- [x] Typing "no" at confirmation restarts character creation from the beginning
- [x] After confirmation, character appears in the entities table and exploration begins

### Movement
**Status:** COMPLETE
**Milestone:** M1
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] Player can move in all 6 hex directions using full names or abbreviations (e, ne, se, sw, w, nw)
- [x] Moving into impassable terrain (mountains, water) is blocked with a descriptive message
- [x] Moving off the map edge produces an "edge of the known world" message
- [x] Character position updates in the entities table after each move
- [x] "north" and "south" are not valid directions on the hex grid

### Terrain Descriptions
**Status:** COMPLETE
**Milestone:** M1
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] Each hex movement produces a narration describing the new terrain
- [x] First visit to a hex produces a distinct "first visit" description
- [x] `look` command re-describes the current hex
- [x] Adjacent hex features are mentioned in the description ("To the northeast you can see...")
- [x] Vision-blocking terrain suppresses adjacent feature descriptions

### Fog of War
**Status:** COMPLETE
**Milestone:** M1
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] Hexes start as unexplored (explored=0 in database)
- [x] Moving into a hex marks it as explored permanently
- [x] GET `/api/worlds/current/map` returns explored status for each hex
- [x] Frontend hex map renders explored and unexplored hexes differently

### Persistence
**Status:** COMPLETE
**Milestone:** M1
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] GM scene state and tick persist in the `gm_state` table across restarts
- [x] Reconnecting with an existing character resumes in exploration (not character creation)
- [x] World creation API returns extended response with hex count and terrain distribution
- [x] `status` command shows character name, HP, AC, and XP from persisted data

---

## M2 — Discovery

### Hex Exploration & Discovery System
**Status:** COMPLETE
**Milestone:** M2
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `search` command at an unexplored hex triggers discoveries approximately 50% of the time
- [x] `search` at an explored hex triggers discoveries approximately 30% of the time
- [x] Searching the same hex within 100 ticks returns a cooldown message
- [x] High-difficulty discoveries are gated behind a skill check
- [x] Environmental discoveries add a feature tag to the hex in the database
- [x] Unknown terrain falls back to the `discoveries_common` table

### Encounter System (generation only)
**Status:** COMPLETE
**Milestone:** M2
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Moving to an unexplored hex triggers an encounter check at approximately 50% rate
- [x] Moving to an explored hex triggers at approximately 25% rate
- [x] Terrain modifiers affect encounter rates (ruins +10%, plains -5%)
- [x] NPC encounters spawn an entity row in the database at the current hex
- [x] Unknown terrain falls back to `encounters_common` table

### NPC Generation (basic)
**Status:** COMPLETE
**Milestone:** M2
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Generated NPCs have name, occupation, personality traits, motivation, appearance, greeting, and disposition
- [x] NPC name combines a first name and surname from name tables
- [x] `examine <npc>` finds NPC by partial name match and shows occupation and appearance
- [x] `talk <npc>` retrieves the NPC's greeting text
- [x] NPCs are only found when the player is at the same hex

### Settlement Generation
**Status:** COMPLETE
**Milestone:** M2
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Settlement size determines NPC count: hamlet 3-4, village 5-8, town 8-12
- [x] Establishment count scales with size: hamlet 1-2, village 2-4
- [x] Each establishment has a linked operator NPC stored in the entities table
- [x] No duplicate establishment types within the same settlement
- [x] `explore town` lists settlement name, size, establishments, and resident NPCs

### Table Engine
**Status:** COMPLETE
**Milestone:** M2
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] YAML table files load recursively from the data directory
- [x] Table entries are upserted into SQLite `random_tables` and `table_entries` tables
- [x] `roll_on(table_id)` returns a weighted random result with roll chain metadata
- [x] Subtable references resolve to nested rolls

### SVG Grid Map Panel
**Status:** COMPLETE
**Milestone:** M2
**Test coverage:** unit ✗ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] Hex map renders as SVG with pointy-top odd-r layout
- [x] Each terrain type has a distinct color
- [x] Settlement (S), ruins (R), and landmark icons render on appropriate hexes
- [x] Player position marker is visible and centers the viewport
- [x] Fog of war visually distinguishes explored from unexplored hexes

### Window Manager & Panel Layout
**Status:** COMPLETE
**Milestone:** M2
**Test coverage:** unit ✗ | property ✗ | mutation ✗ | E2E ✓

**Criteria:**
- [x] Chat panel, hex map, and status sidebar render as separate panels
- [x] Panel layout persists in the `world_meta` table per world
- [x] Layout is restored when reconnecting to a world

---

## M3 — Combat

### Combat Scene
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Hostile encounter triggers an awareness check before combat begins
- [x] Combat initializes combatants with correct HP, initiative, and display names
- [x] Duplicate enemies are auto-numbered ("Wolf (1)", "Wolf (2)")
- [x] Single enemies are not numbered
- [x] Valid commands in combat include attack, flee, use, status, and help

### Initiative & Turn Order
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Initiative rolls are d20 + DEX modifier for the player, d20 for creatures
- [x] Turn order is sorted by initiative (highest first)
- [x] Player surprise skips the first enemy turn
- [x] Enemy surprise skips the first player turn

### Attack Resolution
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Attack roll is d20 + attack bonus vs. target AC
- [x] Damage is rolled from the weapon/creature damage expression
- [x] Creatures with multiple attacks resolve each sequentially
- [x] Combat ends when all enemies reach 0 HP

### Damage
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Damage expressions (e.g. "1d6+2") parse and roll correctly
- [x] Damage is capped at the target's remaining HP (no negative HP)
- [x] Damage expression round-trip (parse then format) is consistent

### Flee Mechanic
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Flee always succeeds (guaranteed escape)
- [x] Skill check (best of exert+STR or sneak+DEX) determines clean vs. messy escape
- [x] Clean escape has no consequences; messy escape may cause damage or item loss
- [x] Flee destination is an adjacent hex
- [x] Flee difficulty equals the highest enemy flee_difficulty (default 6)

### Death & Respawn
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] HP dropping to 0 triggers a last stand with -2 attack penalty
- [x] Failing the last stand presents two respawn options
- [x] Option 1 (respawn): teleports to nearest settlement at 50% HP, loses one item and 15% XP
- [x] Option 2 (new character): transitions to character creation scene
- [x] Death hex is marked with a death marker containing lost items
- [x] `take` command at a death hex retrieves dropped items

### Healing (rest + healer NPC)
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Short rest (10 ticks) restores 1 HP
- [x] Full rest (50+ ticks) restores level + CON modifier HP
- [x] First aid skill check (heal+WIS vs. DC 8) restores 1-3 HP on success
- [x] Healing items restore HP based on their healing expression
- [x] Town healer restores full HP at 5 gold per HP missing
- [x] No healing method can exceed max HP
- [x] Rest can be interrupted by encounters (10% chance per tick)

### XP & Leveling
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] XP awarded after combat equals the sum of defeated enemy XP values
- [x] Level-up triggers when XP reaches the threshold (doubling: 1500, 3000, 6000...)
- [x] Level-up increases max HP by class hit die + CON modifier (minimum 1)
- [x] Saves improve by 1 on every even level
- [x] Maximum level is 10

### Class Abilities (Warrior, Expert, Adventurer)
**Status:** COMPLETE
**Milestone:** M3
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Warriors gain +1 attack bonus per level (always highest AB at any level)
- [x] Experts gain +1 attack bonus per 2 levels (ceiling)
- [x] Warrior hit die is d8; expert and adventurer use d6
- [x] Expert reroll ability is tracked (wired in M4)

---

## M4 — People

### Social Scene
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `talk <npc>` at a hex with NPCs enters the social scene with that NPC's data
- [x] Valid social commands include convince, intimidate, deceive, bribe, connect, ask, and leave
- [x] Scene prompt shows NPC name and current disposition label
- [x] `leave` command transitions back to exploration
- [x] Hostile NPC (disposition -3) refuses to talk

### UNE Personality Generation
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Generated personality includes power_level, descriptor, motivation_verb, motivation_noun, bearing, bearing_focus, and base_disposition
- [x] All personality string values are non-empty
- [x] Explicit power level overrides the random roll
- [x] Base disposition always starts at 0
- [x] High chaos (7+) shifts disposition by -1; low chaos (3-) shifts by +1

### Disposition System
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Disposition maps to 7 labels: Hostile (-3), Unsteady (-2), Guarded (-1), Indifferent (0), Sociable (1), Friendly (2), Helpful (3)
- [x] Disposition is always clamped to [-3, +3]
- [x] Disposition changes persist to the NPC's entity data in the database
- [x] Disposition reaching -3 during conversation auto-transitions to combat
- [x] Legacy string dispositions convert to integers (backward compatible)

### Skill Check Resolution
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Social verbs map to skills and attributes via the data-driven skill_mappings table
- [x] Skill check rolls 2d6 + skill level + attribute modifier vs. difficulty
- [x] Margin classifies as exceptional_failure (<=4), failure, bare_success, solid_success, exceptional_success (>=4)
- [x] Intimidate uses STR modifier (not CHA)
- [x] Deceive failure by 4+ causes disposition drop of -3 (NPC catches the lie)
- [x] Unknown verbs fall back to Talk/CHA defaults

### Expert Reroll
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Expert class is prompted to reroll after a failed skill check
- [x] Non-expert classes (warrior, adventurer) are never prompted
- [x] Confirming the reroll fires a `character.expert_reroll` event with original and reroll totals
- [x] The reroll flag is consumed after use (once per scene)
- [x] Declining the reroll clears the pending state without consuming the flag

### Mythic Oracle — Fate Chart
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `oracle <question> (<likelihood>)` resolves against the 9x9 Mythic fate chart
- [x] All 9 likelihood levels produce a valid result (YES, NO, EXCEPTIONAL_YES, EXCEPTIONAL_NO)
- [x] Higher chaos factor increases the probability of YES results
- [x] Narration includes likelihood, chaos factor, roll value, and bolded result
- [x] Same seed + likelihood + chaos always produces the same result

### Mythic Oracle — Scene Checks & Chaos
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Scene checks fire automatically on every scene transition (except CHARACTER_CREATION and RESPAWN)
- [x] Roll d10 vs. chaos factor: odd roll <= chaos = INTERRUPT, even roll <= chaos = ALTERED, roll > chaos = NORMAL
- [x] Chaos factor is always clamped to [1, 9] after any adjustment
- [x] Low chaos (1) produces mostly NORMAL results; high chaos (9) produces mostly modified results

### Mythic Oracle — Random Events
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] INTERRUPT scene modifications generate a random event with focus, action, and subject
- [x] Random events use d100 rolls against weighted event tables
- [x] Multiple generations produce variety (at least 2 distinct focuses and actions in 20 events)
- [x] Seeded RNG produces repeatable random events

### Adventure Crafter — Plotlines & Threads
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `create plotline <title> <theme>` creates a plotline with status=active and empty scenes list
- [x] Theme is normalized to title case ("action" -> "Action")
- [x] `advance plotline <id>` generates a scene with theme-specific elements
- [x] Plotlines are retrievable by full ID or ID prefix
- [x] `add thread <title>` / `resolve thread <id>` / `list threads` manage adventure threads
- [x] Oracle NPCs are tracked independently from world entities
- [x] Threads persist in the `threads` table and plotlines in the `plotlines` table

### Faction System — Turn Engine
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Faction turns fire when the world clock advances past a 168-tick (weekly) boundary
- [x] Harvest action with appropriate asset increases faction wealth
- [x] Create action deducts wealth and spawns a new asset (requires minimum attribute)
- [x] Attack action damages a target faction's asset HP
- [x] Sell action removes an asset and recovers 50% of its cost
- [x] Repair action restores asset HP (capped at max) at half creation cost
- [x] `run_all_turns()` processes every faction and produces per-faction results

### Faction System — Reputation & Encounter Modification
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Player reputation maps to 5 disposition tiers: hostile (<-30), unfriendly, neutral, friendly, allied (>=30)
- [x] Higher reputation always produces an equal or more favorable disposition
- [x] Default reputation with any faction is 0 (neutral)
- [x] Hostile reputation adds patrol_hostile weight modifier (+4) to encounter tables
- [x] Allied reputation suppresses hostile encounters (patrol_hostile modifier -999)
- [x] Neutral reputation produces no encounter modifiers

### Shopping Scene
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `shop` command at a settlement hex transitions to the shopping scene
- [x] `list` shows available items with prices
- [x] `buy <item>` deducts gold and adds the item to character equipment
- [x] `buy` with insufficient gold produces a "can't afford" message
- [x] `sell <item>` recovers 50% of item value and removes it from equipment
- [x] `examine <item>` shows item details (name, price, description)
- [x] `leave` transitions back to exploration
- [x] Gold changes persist to the character's data in the database

### Admin System — Config Tables
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] All 5 config tables (skill_mappings, difficulty_targets, disposition_outcomes, encounter_weights, faction_asset_stats) are seeded from YAML at world creation
- [x] Seed counts match YAML file entries (7, 6, 7, 9, 9+)
- [x] Each table supports get, set, and reset-to-default operations
- [x] Export produces a JSON document containing all 5 tables
- [x] Intimidate mapping confirms STR attribute (design constraint verified)

### Admin System — REST API & CLI
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] GET/PUT endpoints exist for each config table (list, get, update)
- [x] POST reset endpoint reverts individual entries to YAML defaults
- [x] POST export-config returns all tables as JSON
- [x] API returns 404 when no world is loaded
- [x] CLI `list`, `get`, `set`, and `reset` subcommands work for skill-mappings
- [x] CLI returns non-zero exit code for missing world file

### Admin System — Vue Panel
**Status:** COMPLETE
**Milestone:** M4
**Test coverage:** unit ✗ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `/admin` route loads the admin panel with tabbed navigation
- [x] All 5 config tabs render: Skill Mappings, Difficulties, Disposition, Encounters, Faction Assets
- [x] Inline editing updates values via PUT API calls
- [x] Save and Reset buttons function for each config table
- [x] Export Config button downloads all tables as JSON

**Notes:** No Playwright E2E tests for the admin panel. Frontend tests deferred.

---

## M4.5 — Editor

### Character Editor (recalc)
**Status:** COMPLETE
**Milestone:** M4.5
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Attribute modifiers calculate correctly per XWN rules (e.g. STR 14 -> +1, CON 18 -> +2)
- [x] Attack bonus follows class rules: warrior=level, expert=ceil(level/2), adventurer=ceil(level*2/3)
- [x] AC calculates from armor bonus + DEX modifier (heavy armor ignores DEX)
- [x] Saves equal 15 - (level // 2) for all three categories
- [x] Preview-recalc endpoint returns calculated values without persisting
- [x] Saving a character auto-triggers recalculation

### Hex Editor
**Status:** COMPLETE
**Milestone:** M4.5
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] GET /api/admin/hexes lists all hexes with terrain, features, explored, and faction_id
- [x] GET /api/admin/hexes/{q}/{r} returns full hex data
- [x] PUT /api/admin/hexes/{q}/{r} persists terrain and feature changes
- [x] POST /api/admin/hexes/bulk-update modifies multiple hexes in one request
- [x] `editor.hex_updated` WebSocket event fires on hex edits

**Notes:** Visual SVG click-to-edit hex editor deferred per CLAUDE.md. Current editor is table-based with inline editing.

### Faction/Asset CRUD
**Status:** COMPLETE
**Milestone:** M4.5
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Faction editor tab lists all factions with Force/Cunning/Wealth stats
- [x] Full CRUD for factions, assets, and inter-faction relations via REST API
- [x] `editor.entity_updated` WebSocket event fires on faction edits

### Dungeon CRUD (JSON form)
**Status:** COMPLETE
**Milestone:** M4.5
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] POST /api/admin/dungeons creates a dungeon with an auto-generated entrance room
- [x] Rooms and connections are edited as JSON
- [x] Dungeon editor tab provides functional CRUD

**Notes:** Visual node-graph dungeon editor deferred per CLAUDE.md. Current editor uses JSON textarea.

### World Operations (clone, export, delete)
**Status:** COMPLETE
**Milestone:** M4.5
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] GET /api/admin/worlds lists all world files
- [x] POST /api/admin/worlds/{name}/clone creates a copy with a new name
- [x] Export produces a downloadable zip of the world database

### YAML File Management
**Status:** COMPLETE
**Milestone:** M4.5
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] GET /api/admin/yaml-files lists data files from the data directory
- [x] Download endpoint returns raw YAML content
- [x] Upload endpoint validates YAML syntax before accepting

### World Meta Editor
**Status:** COMPLETE
**Milestone:** M4.5
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Key-value CRUD for the `world_meta` table via REST API
- [x] Known keys (grid_type, chaos_factor, etc.) are labeled in the UI
- [x] `editor.world_meta_updated` WebSocket event fires on edits

**Notes (M4.5 deferred items):**
- MISSING: Visual hex editor (SVG click-to-edit) — deferred per CLAUDE.md
- MISSING: Visual node-graph dungeon editor — deferred per CLAUDE.md
- MISSING: Playwright E2E tests for all editor tabs — deferred per CLAUDE.md

---

## M4.6 — Combat Completion

### Item Registry & ID System
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `ItemRegistry` loads all items from `data/items/*.yaml` at startup
- [x] All item IDs match pattern `^[a-z]+\.[a-z0-9_]+$`
- [x] Duplicate item IDs raise `ValueError` on load
- [x] `get()` returns `None` for unknown items; `get_or_raise()` raises `KeyError`
- [x] `items_by_tag()` and `items_by_category()` filter correctly
- [x] Six YAML files exist: weapons, armor, ammo, gear, consumables, pretech

### Weapons Data
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `data/items/weapons.yaml` contains melee and ranged weapons with damage, shock, range_band
- [x] Ranged weapons have `ammo_type` referencing ammo item IDs
- [x] All weapon IDs resolve via `ItemRegistry`

### Ammo Tracking
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Ranged attacks consume ammo via `_consume_ammo()` in combat scene
- [x] When ammo depleted, auto-switches to melee weapon or unarmed

### Range Bands
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `Combatant` dataclass has `range_band` field (default "melee")
- [x] Melee weapons require melee range; ranged weapons work at near/far

### Saving Throw Types
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Four save types: physical (CON mod), evasion (DEX mod), mental (WIS mod), luck (no mod)
- [x] `resolve_save()` rolls d20 + stat modifier vs base target
- [x] `SaveResult` frozen Pydantic model with roll, modifier, target, passed
- [x] `classes.yaml` has physical/evasion/mental keys (luck falls back to base 15)

### Shock Damage
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `resolve_shock()` returns damage when attack misses and target AC ≤ weapon shock threshold
- [x] No shock when weapon has `shock_damage=0` or target AC exceeds threshold
- [x] Shock damage clamped to minimum 0
- [x] `combat.attack` event includes `shock` field

### Equipped Weapon Resolution
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] PC attack uses equipped weapon stats from `ItemRegistry`
- [x] Unarmed fallback uses 1d2 + STR mod

### Combat Log Formatting
**Status:** COMPLETE
**Milestone:** M4.6
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `combat.attack` event carries roll, modifier, total, target_ac, hit, damage, shock, critical
- [x] `combat.save` event carries save_type, roll, modifier, total, target, passed
- [x] Frontend `useWebSocket.ts` handles `combat.attack` and `combat.save` events

---

## M4.7 — Town Depth

### Settlement-Size Shop Tiers
**Status:** COMPLETE
**Milestone:** M4.7
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Shop inventory driven by building type + settlement tier (small/medium/large)
- [x] `data/shops/` contains per-building-type YAML files (blacksmith, general_store, healer, tavern)
- [x] `shop_inventory.py` loads inventory via `ItemRegistry` lookup
- [x] Settlement size maps to tier: hamlet→small, village→medium, town→large

**Notes:** Spec called for per-tier files (hamlet.yaml, village.yaml, town.yaml). Implementation uses per-building-type files with tier variants instead. Same functional outcome, different file organisation. No `shop_tiers` SQLite table — YAML-based approach used.

### PC Inventory Panel
**Status:** COMPLETE
**Milestone:** M4.7
**Test coverage:** unit ✗ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `InventoryPanel.vue` component exists with toggle open/close
- [x] Shows equipped and stowed items with enc values
- [x] Shows enc used / enc capacity total
- [x] Updates after shop transactions (loadCharacter on purchase/sale events)

### look Lists NPCs
**Status:** COMPLETE
**Milestone:** M4.7
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `look` at settlement hex lists NPCs by name and occupation ("Present: Name (occupation)")
- [x] `look` at non-settlement hex does not include NPC list

### Shop Rejection Outside Settlement
**Status:** COMPLETE
**Milestone:** M4.7
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `shop` command at non-settlement hex returns error message

### NPC Persistence Verification
**Status:** COMPLETE
**Milestone:** M4.7
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] NPCs persist across settlement re-entry (same entity IDs)
- [x] Disposition changes survive re-entry
- [x] Dead NPCs not listed after re-entry

---

## M4.8 — Bot Framework

### Bot Package & BotRunner
**Status:** COMPLETE
**Milestone:** M4.8
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `src/harsh_realm/bot/` package with runner.py, models.py, logger.py, assertions.py, pathfinder.py
- [x] `BotRunner` connects via WebSocket, sends commands, waits for responses
- [x] Structured JSON log written per bot run

### World Map API Endpoint
**Status:** COMPLETE
**Milestone:** M4.8
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `GET /api/world/map` returns full cell grid with coordinates, terrain, passability, features
- [x] Returns 404 when no world loaded
- [x] `MapCell` and `MapGrid` Pydantic models in `models/map.py`

### Pathfinding
**Status:** COMPLETE
**Milestone:** M4.8
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] A* pathfinder over square grid using Chebyshev distance
- [x] Only traverses passable cells
- [x] `direction_to()` returns correct compass direction for all 8 directions
- [x] `all_reachable_cells()` via BFS

### Goal & Assertion System
**Status:** COMPLETE
**Milestone:** M4.8
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `Goal`, `BotAction`, `BotState`, `AssertionResult` models defined
- [x] `assert_contains`, `assert_exact`, `assert_threshold` helpers work
- [x] Bot tests marked `@pytest.mark.bot` and skipped without `--run-bot`

### First Goal Suite
**Status:** COMPLETE
**Milestone:** M4.8
**Test coverage:** bot ✓ (requires --run-bot)

**Criteria:**
- [x] 6 goal tests: create character, explore map, complete combat, flee combat, buy item, talk NPC
- [x] All skip by default; run with `pytest --run-bot`

---

## M4.9 — Cleanup & Polish

### mutmut Coverage — M4 Modules
**Status:** MISSING
**Milestone:** M4.9

**Criteria:**
- [ ] mutmut run on all M4 engine modules with ≥85% kill rate
- [ ] Surviving mutants documented as equivalents with inline comments

### StatusSidebar — Gold, Scene, Chaos
**Status:** COMPLETE
**Milestone:** M4.9
**Test coverage:** unit ✗ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Gold balance displayed with coin icon in StatusSidebar
- [x] Scene type badge with colour coding (Exploring/Social/Shopping/Combat)
- [x] Chaos factor displayed with colour scale (green 1–3, yellow 4–6, red 7–9)

### ChatLog — Social Event Formatting
**Status:** COMPLETE
**Milestone:** M4.9
**Test coverage:** unit ✗ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `social.disposition_change` renders with NPC name, old/new score, delta
- [x] `action.skill_check` renders with verb, skill, roll breakdown, pass/fail
- [x] `character.expert_reroll` renders with original and reroll values

### ChatLog — Shopping Event Formatting
**Status:** COMPLETE
**Milestone:** M4.9
**Test coverage:** unit ✗ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `shopping.purchase` shows item name, price, balance
- [x] `shopping.sale` shows item name, price, total

### Backend Event Emission — Factions & Oracle
**Status:** COMPLETE
**Milestone:** M4.9
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `faction.turn_completed` events published to event bus via `_run_faction_turns()` in GM controller
- [x] `faction.reputation_changed` events auto-published by `ReputationSystem` when event bus provided
- [x] `oracle.chaos_changed` events emitted by ChaosTracker via callback wired in GM controller
- [x] Frontend handlers ready for all three event types in `useWebSocket.ts`
- [x] Chaos factor persisted to gm_state after scene checks
- [x] Tick advances on each player action

### Playwright E2E — Admin Panel
**Status:** MISSING
**Milestone:** M4.9

**Criteria:**
- [ ] Playwright tests for all 12 admin panel tabs
- [ ] Each tab: renders, edit+save works, reset restores defaults

---

## Modular Rules Architecture — Phase 0 Foundation

### Pack Foundation
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 0
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Pack manifests parse and validate `pack.yaml`
- [x] Version constraints support comparison and compatible-release checks
- [x] Directory packs load content records with provenance fields
- [x] Pack registry validates dependencies, conflicts, cycles, and load order
- [x] Registry can detect duplicate qualified content IDs
- [x] Registry read APIs return records and qualified IDs across loaded packs

### Built-In XWN Pack
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 0
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `packs/xwn-core/pack.yaml` exists and loads
- [x] Game content moved from root `data/` into `packs/xwn-core/content/`
- [x] Root `data/` contains editor/schema metadata only
- [x] Default content path resolves to `packs/xwn-core/content/`
- [x] Code-bearing pack hook loads `packs/xwn-core/code/__init__.py`

### World Pack Persistence
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 0
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] World schema includes `world_packs` and `pack_overrides`
- [x] `WorldPackRepository` owns pack binding and override SQL
- [x] World creation accepts pack IDs and persists resolved bindings
- [x] World load reconstructs `PackRegistry`
- [x] Installed/recorded pack version mismatches are detected

### Override-Aware Content
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 0
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `ContentService` resolves records through pack data plus per-world overrides
- [x] Override CRUD endpoints can get, put, delete, and list world overrides
- [x] Override records carry provenance and `_overridden = True`
- [x] Admin UI has an override indicator and revert action surface

### Pack UI And Tooling
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 0
**Test coverage:** typecheck ✓ | E2E ✗

**Criteria:**
- [x] `/api/packs` lists available pack manifests
- [x] World creation UI includes a pack picker
- [x] World creation sends selected pack IDs
- [x] Pack migration scaffolding discovers and executes schema/data migrations
- [x] CLAUDE.md and AGENTS.md describe pack-aware architecture

---

## Modular Rules Architecture — Phase 1 Procedures And Status Effects

### Procedure Framework
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 1
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Procedure schemas validate roll, compute, procedure, and format steps
- [x] `ProcedureRunner` executes override-aware pack procedure content
- [x] Code-bearing packs register compute functions through `ComputeRegistry`
- [x] World load attaches `compute_registry` and `procedure_runner` to app state
- [x] Procedure CRUD and admin-run endpoints exist under `/api/world/procedures`

### Procedure Content Migration
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 1
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] UNE personality, motivation, and bearing generators exist as xwn-core procedure records
- [x] UNE procedure output validates against the legacy `UNEPersonality` shape
- [x] `wickham-tables` pack exists and depends on `xwn-core`
- [x] A world can be created with `[xwn-core, wickham-tables]`
- [x] `wickham-tables:procedures.fantasy_prompt` runs through the procedure API

### Status Effect Persistence
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 1
**Test coverage:** unit ✓ | property ✓ | mutation ✗ | E2E ✗

**Criteria:**
- [x] Status effect content records validate with duration and stacking rules
- [x] World schema includes durable `entity_status_effects`
- [x] `StatusEffectRepository` owns all active-effect SQL
- [x] `StatusEffectService` supports apply, remove, remove-by-id, list, and expire
- [x] Replace, extend, and stack re-application policies are covered by tests

### Status Effect Events
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 1
**Test coverage:** unit ✓ | property ✗ | mutation ✗ | E2E ✗

**Criteria:**
- [x] `status.apply_requested` emits `status.applied` after persistence
- [x] `status.remove_requested` emits `status.removed`
- [x] `world.tick_advanced` expires due effects and emits `status.expired`
- [x] Handler failures are logged and do not propagate to the dispatcher
- [x] xwn-core ships `xwn-core:status_effects.poisoned`

### Status Effect UI
**Status:** COMPLETE
**Milestone:** Modular Rules Phase 1
**Test coverage:** typecheck ✓ | E2E ✗

**Criteria:**
- [x] `/api/character/{entity_id}/status_effects` returns active effects with content labels
- [x] Frontend character state includes active status effects
- [x] WebSocket `status.applied`, `status.removed`, and `status.expired` update the store
- [x] `StatusSidebar` displays effect name, optional icon, description tooltip, and expiry label
- [x] `vue-tsc --noEmit` passes
