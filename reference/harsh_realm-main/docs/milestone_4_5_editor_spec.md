# Harsh Realm — Milestone 4.5: Comprehensive Editor
**Version:** 1.0  
**Date:** 2026-03-27  
**Estimated Duration:** 4–5 weeks (~69 hours)  
**Depends on:** M4 complete (admin panel, social, factions, oracle)

---

## Scope Summary

| Component | Hours | Notes |
|---|---|---|
| Visual hex editor | ~12 | SVG click + edit sidebar + live WebSocket push |
| Rule-aware character editor | ~10 | XWN recalc for PCs; same form for NPCs/monsters |
| Faction + assets CRUD | ~6 | Relational — faction owns assets |
| Items + equipment CRUD | ~5 | Flat records |
| Random + encounter tables | ~4 | Extends M4 admin tab |
| World operations | ~5 | Clone, export ZIP, delete |
| YAML file upload/download | ~4 | Replace on upload, serve raw on download |
| World metadata + settings | ~3 | key/value against `world_meta` |
| Visual node-graph dungeon editor | ~20 | Custom Vue graph component |
| **Total** | **~69** | |

---

## Architecture Notes

### What's Different from the M4 Admin Panel

The M4 admin panel edits **config data** — skill mappings, difficulty targets, faction asset stat templates. That data is world-independent (same shape in every world) and rarely changes during play.

M4.5 edits **world state** — the actual characters, hexes, factions, items, and dungeons in a specific running world. This means:

- Every change is a write to the world SQLite database
- Changes to live entities (characters, hexes) push a WebSocket event to the game UI immediately
- The editor must understand XWN rules well enough to recalculate derived values
- Operations like clone and delete touch the filesystem, not just the DB

### WebSocket Push for Live Edits

When the editor changes a live entity, the backend fires a synthetic game event over the WebSocket so the game UI reflects the change without a reload:

```python
# After any entity update in the editor:
await event_bus.publish(GameEvent(
    event_type="editor.entity_updated",
    data={"entity_id": entity.id, "entity_type": entity.entity_type},
    source="editor"
))
```

The game UI's existing event handlers pick this up and re-render the affected component (character sheet, hex map, status panel). No special editor-specific handling needed on the frontend game side.

**Hex edits** fire `editor.hex_updated` with `{q, r}` — the map component re-fetches and re-renders that hex.

**Deferred exception:** Dungeon edits do NOT push live — dungeons aren't loaded until the player enters them. Dungeon saves write to DB only.

---

## Section E.1 — Visual Hex Editor

**Estimated effort:** ~12 hours  
**Files:** `frontend/src/components/admin/HexEditorTab.vue`, `frontend/src/components/admin/HexEditPanel.vue`, `src/harsh_realm/api/admin_routes.py` (extend)

### E.1.1 Grid Display

The existing `HexMap.vue` game component renders the SVG hex grid for gameplay. The admin grid editor is a **separate component** that reuses the same SVG rendering logic but adds editor-specific behavior. Do not modify `HexMap.vue` — extract shared rendering into a composable. When square grid support is added, a `SquareMap.vue` component will be needed alongside `HexMap.vue` (see `docs/grid_changes.md`).

**Tasks:**
- [ ] Extract grid rendering logic from `HexMap.vue` into `composables/useGridRenderer.ts`
  - Shared: grid geometry (hex pointy-top or square), coordinate → pixel conversion, terrain color/pattern lookup
  - Game-only: fog of war, player position indicator, encounter flash
  - Editor-only: selection highlight, unsaved change indicator
  - **Test:** `HexMap.vue` renders identically after refactor
- [ ] Implement `HexEditorTab.vue` using `useGridRenderer`
  - Full-world SVG view with zoom controls (scroll wheel + +/− buttons)
  - Pan by click-drag on empty space
  - Minimap in corner for large worlds
  - **Test:** 12×12 grid renders without overflow or clipping
- [ ] Terrain color/pattern legend displayed alongside the map
  - **Test:** Each terrain type has a distinct color matching the map rendering

### E.1.2 Hex Selection + Edit Panel

**Tasks:**
- [ ] Click a hex → highlight it + open `HexEditPanel.vue` in a sidebar
  - Click same hex again → deselect + close panel
  - Click different hex → switch selection, panel updates
  - **Test:** Clicking hex (3, -1) opens panel showing that hex's current data
- [ ] `HexEditPanel.vue` fields:
  ```
  Coordinates:    q=3, r=-1  (read-only)
  Terrain:        [Forest ▼]  (select from terrain types)
  Explored:       [✓]         (checkbox)
  Faction Control:[Iron Pact ▼] (select from world factions + "None")
  Features:       [tag chips with × to remove] [+ Add Feature]
  Description:    [textarea — custom text overrides template]
  Raw Data (JSON):[collapsible textarea — escape hatch]
  
  [Save Changes]  [Reset to Generated]  [Cancel]
  ```
- [ ] "Reset to Generated" regenerates the hex description from the world generator using the current terrain type — does not change terrain/features, only the description text
  - **Test:** Reset on custom-described hex → description reverts to generated template text
- [ ] Save → `PUT /api/admin/hexes/{q}/{r}` → event bus fires `editor.hex_updated` → game map re-renders
  - **Test:** Change terrain from Forest to Mountains → hex color updates on game map within 500ms without page reload
- [ ] Add Feature: text input with autocomplete from known feature tags (ruins, settlement, dungeon_entrance, etc.)
  - **Test:** Type "ru" → autocomplete suggests "ruins", "ruined_tower"

### E.1.3 Bulk Hex Operations

**Tasks:**
- [ ] Shift+click to select multiple hexes
  - Selected hexes shown with distinct highlight color
  - **Test:** Shift+click 5 hexes → all 5 highlighted
- [ ] Bulk edit panel appears when multiple hexes selected:
  ```
  3 hexes selected
  Set terrain for all: [— no change — ▼]
  Set faction for all: [— no change — ▼]
  Mark all explored:   [ ]
  
  [Apply to All]  [Clear Selection]
  ```
- [ ] "Apply to All" updates all selected hexes, fires one `editor.hex_bulk_updated` event
  - **Test:** Select 4 hexes → set terrain to Desert → all 4 update, map re-renders all 4

---

## Section E.2 — Rule-Aware Character Editor

**Estimated effort:** ~10 hours  
**Files:** `frontend/src/components/admin/CharacterEditorTab.vue`, `frontend/src/components/admin/CharacterEditForm.vue`, `src/harsh_realm/engine/character_recalc.py`

### E.2.1 Character Recalculation Engine

XWN derived values must recalculate when base stats change. This logic lives in the **backend** — the frontend sends updated base stats and the backend returns the full recalculated character.

```python
# src/harsh_realm/engine/character_recalc.py

class CharacterRecalculator:
    def recalculate(self, base: CharacterBase) -> CharacterDerived:
        """
        Given editable base fields, returns all derived values.
        Called by the editor API before saving.
        """
        attr_mods = {
            attr: self._attr_to_mod(score)
            for attr, score in base.attributes.items()
        }
        max_hp = self._calc_max_hp(base.class_name, base.level,
                                    base.con_hp_rolls, attr_mods["CON"])
        ac = self._calc_ac(base.armor_id, base.shield, attr_mods["DEX"])
        attack_bonus = self._calc_attack_bonus(base.class_name, base.level)
        saves = self._calc_saves(base.level, attr_mods)
        encumbrance = self._calc_encumbrance(base.inventory)
        
        return CharacterDerived(
            attr_mods=attr_mods,
            max_hp=max_hp,
            ac=ac,
            attack_bonus=attack_bonus,
            melee_attack=attack_bonus + attr_mods["STR"],
            ranged_attack=attack_bonus + attr_mods["DEX"],
            saves=saves,
            encumbrance=encumbrance,
            readied_slots=encumbrance.readied_used,
            stowed_slots=encumbrance.stowed_used,
        )

    def _attr_to_mod(self, score: int) -> int:
        # XWN: 3→-2, 4-7→-1, 8-13→0, 14-17→+1, 18→+2
        if score <= 3: return -2
        if score <= 7: return -1
        if score <= 13: return 0
        if score <= 17: return 1
        return 2
```

**Tasks:**
- [ ] Implement `CharacterRecalculator` with all XWN derived value formulas
  - **Test (parametrized):** STR 14 → mod +1; CON 10 → mod 0; DEX 18 → mod +2
  - **Test:** Warrior level 3, CON 12 → correct max HP range
  - **Test:** Chain mail + DEX mod 0 → AC 16
  - **Test:** Level 5 → save = 15 - 2 = 13
- [ ] Expose via `POST /api/admin/characters/preview-recalc` — accepts base fields, returns derived (no DB write)
  - **Test:** POST with STR 16 → response contains `attr_mods.STR = 1`
- [ ] `PUT /api/admin/characters/{id}` runs recalc before saving
  - **Test:** Edit CON via API → max_hp in DB updates correctly

### E.2.2 Character List + Editor Form

**Tasks:**
- [ ] `CharacterEditorTab.vue` — split view:
  - Left: filterable character list (name, type, level, location)
  - Filter tabs: All | PCs | NPCs | Monsters | Dead
  - Right: `CharacterEditForm.vue` for selected character
  - **Test:** Filter to "PCs" → only player characters shown
- [ ] `CharacterEditForm.vue` sections:

  **Identity (always editable):**
  ```
  Name:       [Kira Voss          ]
  Type:       [PC ▼]
  Class:      [Warrior ▼]
  Level:      [1  ]
  Background: [Soldier            ]
  Description:[textarea           ]
  Location:   q=[3] r=[-1]  [Find on Map]
  Alive:      [✓]
  ```

  **Attributes (editable — derived shown inline):**
  ```
  STR  [14] → mod +1    DEX  [12] → mod +0
  CON  [11] → mod +0    INT  [10] → mod +0
  WIS  [13] → mod +1    CHA  [ 9] → mod +0
  ```
  Derived values update in real-time as user types (debounced 300ms → call preview-recalc):

  **Derived (read-only, recalculated):**
  ```
  Max HP:      9   ← recalculated
  Current HP: [9]  ← editable independently
  AC:         15   ← recalculated from equipped armor
  Attack Bonus: +1 ← recalculated
  Melee:       +2  ← attack bonus + STR mod
  Ranged:      +1  ← attack bonus + DEX mod
  
  Saves:
    Physical: 15   ← recalculated
    Evasion:  15
    Mental:   15
    Luck:     15
  ```

  **Skills (editable list):**
  ```
  Stab    [1 ▼]    Exert   [0 ▼]
  Survive [0 ▼]    Notice  [0 ▼]
  + Add skill
  ```

  **XP + Practice:**
  ```
  XP:      [0    ]   Next level: 1500
  Practice ticks: [view/edit per-skill]
  ```

  **UNE Personality (for NPCs — collapsible):**
  ```
  Power Level:  [Average ▼]
  Descriptor:   [Scheming  ]
  Motivation:   [Advance   ] [Wealth    ]
  Disposition:  [-1 (Guarded)]
  [Regenerate Personality]
  ```

- [ ] "Find on Map" button switches to hex editor tab with the character's hex highlighted
  - **Test:** Click "Find on Map" for character at (3,-1) → hex editor tab opens, hex (3,-1) selected
- [ ] Derived values recalculate on every base stat change without saving
  - **Test:** Change STR from 14 to 16 → melee attack display updates within 300ms
- [ ] Save button → `PUT /api/admin/characters/{id}` → fires `editor.entity_updated`
  - **Test:** Edit current HP → game UI character sheet reflects new HP within 500ms
- [ ] "New Character" button → blank form with XWN defaults → save creates new entity
- [ ] Delete button with confirmation → sets `alive=0` (soft delete) or hard delete toggle
  - **Test:** Soft delete → character disappears from "All" filter but appears in "Dead" filter

---

## Section E.3 — Faction + Assets CRUD

**Estimated effort:** ~6 hours  
**Files:** `frontend/src/components/admin/FactionEditorTab.vue`

### E.3.1 Faction List + Form

**Tasks:**
- [ ] Left panel: faction list (name, HP bar, Force/Cunning/Wealth badges, disposition)
- [ ] Right panel: faction edit form:
  ```
  Name:         [Iron Pact          ]
  HP:           [7  ] / Max HP: [7  ]
  Force:        [3  ]   Cunning: [2  ]   Wealth: [4  ]
  XP:           [12 ]
  Home Hex:     q=[2] r=[0]   [Find on Map]
  Goals:        [tag list with + Add]
  Tags:         [tag list with + Add]
  
  Relationships:
  ┌─────────────────────────────────┐
  │ Merchant Guild    [Friendly ▼]  │
  │ The Brotherhood   [Hostile  ▼]  │
  │ + Add relationship              │
  └─────────────────────────────────┘
  
  [Save]  [Delete Faction]
  ```
- [ ] Relationship dropdowns: Allied / Friendly / Neutral / Unfriendly / Hostile
  - Changing a relationship → updates `faction_relations` table both directions
  - **Test:** Set Iron Pact → Merchant Guild to Hostile → `faction_relations` has both (A→B) and (B→A) rows updated

### E.3.2 Asset Management

**Tasks:**
- [ ] Assets panel below faction form — collapsible table:
  ```
  Type              Cat.    HP      Location    Actions
  ──────────────────────────────────────────────────────
  Warriors          Force   4/6     (2,0)       [Edit] [Delete]
  Informers         Cunning 3/3     (5,-2)      [Edit] [Delete]
  + Add Asset
  ```
- [ ] "Add Asset" → dropdown of asset types from `faction_asset_stats` + location picker
  - **Test:** Cannot add asset if faction doesn't meet minimum attribute requirement — show warning
- [ ] Edit asset → inline form: HP, location hex, data JSON
  - **Test:** Edit Warriors HP to 2 → DB updates, faction tab shows 2/6

---

## Section E.4 — Items + Equipment CRUD

**Estimated effort:** ~5 hours  
**Files:** `frontend/src/components/admin/ItemEditorTab.vue`

### E.4.1 Item List

**Tasks:**
- [ ] Filterable item list: All | Weapons | Armor | Gear | Consumables | Unique
- [ ] Columns: Name, Type, Location (carried by / in hex / in dungeon), Weight (slots)
- [ ] Search by name
- [ ] "New Item" button

### E.4.2 Item Edit Form

Fields vary by item type. Common fields always shown; type-specific fields shown conditionally:

```
Name:        [Broadsword        ]
Type:        [Weapon ▼]
Description: [textarea          ]
Weight:      [1  ] slots   Readied: [✓]
Value:       [60 ] GP
Owner:       [Kira Voss ▼] or [Hex (3,-1) ▼] or [Dungeon room]

── Weapon fields (shown when Type = Weapon) ──
Damage Die:  [1d8 ▼]
Attribute:   [STR ▼]
Shock:       [2/AC15]
Range:       [— (melee)]
Tags:        [two-handed, heavy]

── Armor fields (shown when Type = Armor) ──
AC Bonus:    [4  ]
Defense:     [✓ worn]

── Consumable fields ──
Uses:        [3  ] remaining
Effect:      [textarea]
```

- [ ] Owner selector: character dropdown OR hex coordinate OR dungeon room
  - **Test:** Move item from character inventory to hex (4,2) → item location updates, character encumbrance recalculates
- [ ] Save fires `editor.entity_updated` for the owning character (if any) so inventory display refreshes
  - **Test:** Remove item from PC inventory → game UI inventory panel updates

---

## Section E.5 — Random Tables + Encounter Tables

**Estimated effort:** ~4 hours  
**Files:** `frontend/src/components/admin/RandomTablesTab.vue` (extend existing M4 tab)

The M4 admin panel already has a Random Tables tab. M4.5 adds:

### E.5.1 Encounter Table Integration

**Tasks:**
- [ ] Add "Encounter Tables" sub-filter in the tables tab (currently shows all tables)
- [ ] Encounter tables display terrain tag and faction modifier preview alongside entries
- [ ] Visual weight distribution — horizontal bar showing relative probability of each entry
  - **Test:** Entry with weight 3 shows bar 3× wider than entry with weight 1
- [ ] "Simulate 10 rolls" button — rolls on the table 10 times, shows results inline
  - **Test:** Simulate → modal shows 10 results, all within valid entry range

### E.5.2 Table Import/Export

**Tasks:**
- [ ] Export single table as YAML download
- [ ] Import table from YAML file — replaces existing if same `id`, creates new if not found
  - **Test:** Export table → modify YAML → re-import → table shows modified entries

---

## Section E.6 — World Operations

**Estimated effort:** ~5 hours  
**Files:** `src/harsh_realm/api/admin_routes.py` (extend), `frontend/src/components/admin/WorldsTab.vue`

### E.6.1 Worlds Tab

New tab in admin panel — replaces the world selector in the existing M4 panel header with a full management view.

```
┌──────────────────────────────────────────────────────────────┐
│  Worlds                                    [+ Create New]    │
├──────────────────────────────────────────────────────────────┤
│  Name           Size     Modified          Actions           │
│  ──────────────────────────────────────────────────────────  │
│  ● ashfall.db   2.3 MB   2026-03-27        [Load] [Clone]    │
│                                            [Export] [Delete] │
│  other.db       1.1 MB   2026-03-15        [Load] [Clone]    │
│                                            [Export] [Delete] │
└──────────────────────────────────────────────────────────────┘
```

- ● indicator on currently loaded world

### E.6.2 Clone World

**Tasks:**
- [ ] "Clone" button → prompt for new name → `POST /api/admin/worlds/{name}/clone`
  - Backend: SQLite backup API (`db.backup(dest)`) — atomic, handles in-flight reads safely
  - **Test:** Clone ashfall.db → new file exists with identical tables
  - **Test:** Clone while game is running → no corruption, game continues unaffected
- [ ] Cloned world appears in list immediately

### E.6.3 Export World as ZIP

**Tasks:**
- [ ] "Export" → `GET /api/admin/worlds/{name}/export` → streams ZIP download
  - ZIP contents:
    ```
    ashfall_export_2026-03-27/
      ashfall.db              ← full SQLite database
      config_snapshot.json    ← export of all admin config tables
      metadata.json           ← world name, created_at, version
    ```
  - **Test:** Exported ZIP contains all three files
  - **Test:** `ashfall.db` in ZIP is a valid SQLite file (not corrupted mid-write)
- [ ] Export uses SQLite backup API to snapshot the DB — never copies a live write

### E.6.4 Delete World

**Tasks:**
- [ ] "Delete" → two-step confirmation:
  - Step 1: "Are you sure? This cannot be undone." → [Cancel] [Yes, delete]
  - Step 2 (if currently loaded world): "This world is currently loaded. Deleting it will end your session." → [Cancel] [Delete and end session]
  - **Test:** Cancel at either step → no file deleted
  - **Test:** Delete non-loaded world → file removed, list updates, game continues
  - **Test:** Delete loaded world → game session ends, redirect to world selector

---

## Section E.7 — YAML File Upload/Download

**Estimated effort:** ~4 hours  
**Files:** `src/harsh_realm/api/admin_routes.py` (extend), `frontend/src/components/admin/YAMLFilesTab.vue`

### E.7.1 File Browser

**Tasks:**
- [ ] List all files in the `data/` directory tree, grouped by subdirectory
  ```
  data/
    skill_mappings.yaml          [Download] [Upload replacement]
    difficulty_targets.yaml      [Download] [Upload replacement]
    tables/
      encounters/
        encounters_forest.yaml   [Download] [Upload replacement]
        encounters_ruins.yaml    [Download] [Upload replacement]
      npcs/
        une_descriptors.yaml     [Download] [Upload replacement]
  ```
- [ ] Download: `GET /api/admin/yaml-files/{path}` → serves raw file with `Content-Type: text/yaml`
  - **Test:** Download `skill_mappings.yaml` → file contents match disk

### E.7.2 Upload (Replace)

**Tasks:**
- [ ] Upload button → file picker (`.yaml`, `.yml` only)
- [ ] On select: show confirmation dialog:
  ```
  ⚠ Replace skill_mappings.yaml?
  
  This will overwrite the existing file on disk.
  Changes take effect on next server restart or hot-reload.
  
  [Cancel]  [Replace File]
  ```
- [ ] `POST /api/admin/yaml-files/{path}` with multipart file upload → writes to disk
  - Validates YAML parses cleanly before writing — rejects malformed YAML
  - **Test:** Upload valid YAML → file on disk updated
  - **Test:** Upload malformed YAML → error response, original file unchanged
- [ ] After upload: if the file is one of the seeded config files (skill_mappings, etc.), offer "Re-seed world from this file?" prompt
  - **Test:** Upload new skill_mappings.yaml → prompt appears → confirm → `skill_mappings` table in current world updated

---

## Section E.8 — World Metadata + Settings

**Estimated effort:** ~3 hours  
**Files:** `frontend/src/components/admin/WorldMetaTab.vue`

**Tasks:**
- [ ] Read/write `world_meta` table key/value pairs
- [ ] Known keys rendered as typed form fields:
  ```
  World Name:      [Ashfall                    ]
  Created:         2026-01-15  (read-only)
  Last Played:     2026-03-27  (read-only)
  Starting Hex:    q=[0] r=[0]
  Calendar System: [Standard ▼]
  Current Week:    [14   ]      ← faction turn counter
  Chaos Factor:    [5    ]      ← oracle state
  Admin Mode:      [✓]          ← enables in-game admin commands
  Debug Logging:   [ ]
  ```
- [ ] Unknown keys (added by plugins or future features) rendered as raw string inputs
  - **Test:** Add arbitrary key "custom_setting" → appears as text input
- [ ] Save fires `editor.world_meta_updated` — game reads updated meta on next tick
  - **Test:** Change Current Week to 20 → faction turn counter in game reflects week 20

---

## Section E.9 — Visual Node-Graph Dungeon Editor

**Estimated effort:** ~20 hours  
**Files:** `frontend/src/components/admin/DungeonEditorTab.vue`, `frontend/src/components/admin/dungeon/RoomNode.vue`, `frontend/src/components/admin/dungeon/ConnectionEdge.vue`, `frontend/src/components/admin/dungeon/RoomEditPanel.vue`

### E.9.1 Graph Canvas

The dungeon is a graph: rooms are nodes, passages are edges. This requires a custom Vue canvas component — no off-the-shelf graph library is assumed (evaluate `vue-flow` or `d3-force` first before rolling custom).

**Tasks:**
- [ ] Evaluate `vue-flow` (Vue 3 node graph library) — use if it handles room nodes cleanly, otherwise implement custom SVG graph
  - **Decision gate:** If `vue-flow` works, ~8 hours saved. Spike 2 hours max before deciding.
- [ ] `DungeonEditorTab.vue` — split view:
  - Left: dungeon list (name, hex location, room count, status)
  - Right: graph canvas for selected dungeon
- [ ] Graph canvas features:
  - Rooms displayed as labeled rectangles (name + room type icon)
  - Connections displayed as lines between rooms
  - Pan by click-drag on empty canvas
  - Zoom by scroll wheel
  - Selected room highlighted
  - **Test:** 10-room dungeon renders without overlap at default zoom

### E.9.2 Room Management

**Tasks:**
- [ ] Double-click empty canvas space → create new room at that position
  - New room gets default name "Room N" and type "corridor"
  - **Test:** Double-click → room node appears → persists after save
- [ ] Click room → open `RoomEditPanel.vue` in sidebar:
  ```
  Name:         [Guard Post         ]
  Type:         [chamber ▼]          (corridor, chamber, vault, shrine, lair, entrance, exit)
  Description:  [textarea            ]
  
  ── Contents ──
  Monsters:     [tag chips]  [+ Add]
  Treasure:     [tag chips]  [+ Add]
  Traps:        [tag chips]  [+ Add]
  Features:     [textarea            ]
  
  ── Connections ──
  North → Room 4 (locked door)    [Edit] [Remove]
  South → Room 6 (open passage)   [Edit] [Remove]
  + Add connection
  
  ── Raw Data ──
  [collapsible JSON textarea]
  
  [Save Room]  [Delete Room]
  ```
- [ ] Drag room node to reposition (cosmetic — position stored in room `data.editor_pos`)
  - **Test:** Drag room → position updates → reopen dungeon → room in same position
- [ ] Delete room → confirmation → removes room and all connections to/from it
  - **Test:** Delete room with 3 connections → all 3 connections removed from DB

### E.9.3 Connection Management

**Tasks:**
- [ ] Draw connection: hover room → connection handle appears → drag to another room
  - **Test:** Drag from Room 3 handle to Room 7 → connection created
- [ ] Click connection line → `ConnectionEdge` inline edit:
  ```
  From: Guard Post → To: Main Hall
  Direction: [two-way ▼]   (one-way, two-way)
  Passage:   [open ▼]      (open, door, locked-door, secret, collapsed, portcullis)
  Notes:     [            ]
  [Save]  [Delete Connection]
  ```
- [ ] Secret passages rendered as dashed lines (visually distinct)
  - **Test:** Set passage to "secret" → line style changes to dashed
- [ ] One-way connections rendered with an arrow
  - **Test:** One-way North → South shows arrowhead pointing South

### E.9.4 Dungeon CRUD

**Tasks:**
- [ ] "New Dungeon" → name + hex location picker → creates empty dungeon with one "Entrance" room
  - **Test:** New dungeon in hex (5,-3) → dungeon record in DB with `hex_q=5, hex_r=-3`
- [ ] `PUT /api/admin/dungeons/{id}` — saves full dungeon (rooms + connections as JSON)
- [ ] Delete dungeon → confirmation → removes dungeon and all rooms/connections
- [ ] Dungeon list shows which hex each dungeon occupies
  - "Find on Map" button → switches to hex editor, highlights dungeon hex
  - **Test:** "Find on Map" for dungeon at (5,-3) → hex editor opens at (5,-3)

---

## New Admin Routes (extending M4 `/api/admin`)

```
# Hexes
GET    /api/admin/hexes                    → list all hexes (paginated)
GET    /api/admin/hexes/{q}/{r}            → get one hex
PUT    /api/admin/hexes/{q}/{r}            → update hex
POST   /api/admin/hexes/bulk-update        → update multiple hexes

# Characters / Entities
GET    /api/admin/characters               → list (filter: type, alive)
GET    /api/admin/characters/{id}          → get one
POST   /api/admin/characters/preview-recalc → recalc without saving
PUT    /api/admin/characters/{id}          → update + recalc + push WS event
POST   /api/admin/characters               → create new
DELETE /api/admin/characters/{id}          → soft or hard delete

# Factions
GET    /api/admin/factions                 → list
GET    /api/admin/factions/{id}            → get with assets + relations
PUT    /api/admin/factions/{id}            → update
POST   /api/admin/factions                 → create
DELETE /api/admin/factions/{id}            → delete + cascade assets
PUT    /api/admin/factions/{id}/assets/{asset_id}  → update asset
POST   /api/admin/factions/{id}/assets     → add asset
DELETE /api/admin/factions/{id}/assets/{asset_id}  → remove asset
PUT    /api/admin/faction-relations/{a}/{b} → update relationship

# Items
GET    /api/admin/items                    → list (filter: type, owner)
GET    /api/admin/items/{id}               → get one
PUT    /api/admin/items/{id}               → update + push WS if owner is PC
POST   /api/admin/items                    → create
DELETE /api/admin/items/{id}               → delete

# Dungeons
GET    /api/admin/dungeons                 → list
GET    /api/admin/dungeons/{id}            → get with rooms + connections
PUT    /api/admin/dungeons/{id}            → save full dungeon
POST   /api/admin/dungeons                 → create
DELETE /api/admin/dungeons/{id}            → delete

# World operations
GET    /api/admin/worlds                   → list .db files
POST   /api/admin/worlds/{name}/clone      → duplicate DB file
GET    /api/admin/worlds/{name}/export     → stream ZIP download
DELETE /api/admin/worlds/{name}            → delete file

# YAML files
GET    /api/admin/yaml-files               → directory listing
GET    /api/admin/yaml-files/{path}        → download file
POST   /api/admin/yaml-files/{path}        → upload replacement

# World meta
GET    /api/admin/world-meta               → all key/value pairs
PUT    /api/admin/world-meta/{key}         → set one key
```

---

## Vue Component Tree

```
frontend/src/views/AdminView.vue
  AdminWorldSelector.vue          ← M4 header (world dropdown)
  Tabs:
    SkillMappingsTab.vue          ← M4
    DifficultiesTab.vue           ← M4
    DispositionOutcomesTab.vue    ← M4
    EncounterWeightsTab.vue       ← M4
    FactionAssetsTab.vue          ← M4 (template stats)
    RandomTablesTab.vue           ← M4 + M4.5 extensions
    ─── M4.5 NEW TABS ───
    WorldsTab.vue
    HexEditorTab.vue
      useHexRenderer.ts           ← shared with HexMap.vue
      HexEditPanel.vue
    CharacterEditorTab.vue
      CharacterEditForm.vue
    FactionEditorTab.vue
    ItemEditorTab.vue
    DungeonEditorTab.vue
      dungeon/RoomNode.vue
      dungeon/ConnectionEdge.vue
      dungeon/RoomEditPanel.vue
    YAMLFilesTab.vue
    WorldMetaTab.vue
  ─── Shared ───
  AdminTable.vue                  ← M4
  ConfirmResetDialog.vue          ← M4
  ConfirmDeleteDialog.vue         ← M4.5 (harder confirm, two-step)
```

---

## Acceptance Tests (Full Milestone)

1. **Hex editor:** Open admin → Hex Editor tab → click hex (3,-1) → change terrain to Desert → Save → game map re-renders hex as desert color within 500ms
2. **Bulk hex edit:** Shift+click 4 hexes → set faction to Iron Pact → Apply → all 4 hexes show Iron Pact territory color
3. **Character recalc:** Edit Kira Voss STR from 14→16 → derived melee attack updates in form without saving → Save → game character sheet shows updated attack bonus
4. **Character live update:** Edit current HP from 9→6 → Save → game UI HP bar drops to 6 without page reload
5. **NPC personality:** Open NPC → "Regenerate Personality" → UNE personality fields update with new values → Save persists
6. **Faction relationship:** Set Iron Pact → Merchant Guild to Hostile → Save → faction_relations DB has both directions as hostile
7. **Asset constraint:** Attempt to add Arcane Agent (requires Cunning 4) to faction with Cunning 2 → warning shown, asset not added
8. **Dungeon creation:** New Dungeon "Old Fort" at hex (5,-3) → double-click canvas 5 times → 5 rooms appear → drag room 3 to new position → draw connection from room 1 to room 2 → set as locked door → Save → reopen dungeon → graph matches
9. **World clone:** Clone ashfall.db as "ashfall_backup" → new file appears in list → load backup → game state identical to original
10. **World export:** Export ashfall.db → download ZIP → unzip → contains ashfall.db + config_snapshot.json + metadata.json → ashfall.db is valid SQLite
11. **World delete (non-loaded):** Delete other.db → confirm → file gone from list → game session unaffected
12. **YAML upload:** Download skill_mappings.yaml → edit convince difficulty to 99 → re-upload → confirm → prompt to re-seed world → confirm → admin Skill Mappings tab shows difficulty 99 for convince
13. **YAML validation:** Upload malformed YAML → error message shown → original file unchanged on disk
14. **World meta:** Set Current Week to 20 → Save → faction turn counter reads week 20 → advance time → faction turn fires correctly from week 20

---

## Estimated Effort Summary

| Section | Hours |
|---|---|
| E.1 Visual Hex Editor | 12 |
| E.2 Rule-Aware Character Editor | 10 |
| E.3 Faction + Assets CRUD | 6 |
| E.4 Items + Equipment CRUD | 5 |
| E.5 Random + Encounter Tables | 4 |
| E.6 World Operations | 5 |
| E.7 YAML Upload/Download | 4 |
| E.8 World Metadata + Settings | 3 |
| E.9 Visual Node-Graph Dungeon Editor | 20 |
| **Total** | **~69 hours** |

**Critical path risk:** E.9 (dungeon editor). The `vue-flow` evaluation spike in E.9.1 should happen in the first day of work — if it saves 8 hours, the milestone compresses to ~61 hours.

**Dependency on M5:** The dungeon editor (E.9) creates dungeon records in the DB. M5 implements the dungeon scene state that loads and runs those records. Build order is correct — editor first, engine second.
