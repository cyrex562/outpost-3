# Grid Abstraction — Changes & Deferred Work

## Completed (2026-03-29)

### Phase 1 — Foundation
- `src/harsh_realm/models/grid.py` — `GridCoord`, `GridType`, `Grid` protocol, `HexGrid`, `SquareGrid`, `create_grid()` factory
- `HexCoord` now extends `GridCoord` (backwards compatible)
- `models/__init__.py` exports all grid types
- 41 unit tests in `tests/test_grid.py`, 9 Hypothesis property tests in `tests/test_properties.py`

### Phase 2 — Grid Injection
All consumers accept `grid: Grid | None` parameter (default `HexGrid()`), preserving existing behavior:
- `gm/controller.py` — passes grid to scene constructors
- `gm/scenes/exploration.py` — uses `grid.neighbor()`, `grid.directions()` instead of `_DIRECTION_MAP` + `HexDirection`
- `gm/scenes/respawn.py` — uses `grid.distance()` instead of `_hex_distance()`
- `engine/combat.py` — `FleeResolver.resolve_flee()` uses `grid.random_neighbor()`
- `generators/world_gen.py` — `_get_neighbors_coords()` uses `grid.neighbors()` + `is_valid()`

### Phase 3 — Parser & API
- `parser/commands.py` — added `"north"`, `"south"`, `"s"` to `DIRECTION_ALIASES`. `"n"` stays as `"no"` (square grid players type full `"north"`)
- `api/routes.py` — `/api/worlds/current/map` includes `"grid_type"` field (from `world_meta`, defaults `"hex"`)

### Design Decisions
- DB table stays named `hexes` — it stores cells for both grid types
- `Grid` is injected, not global — each scene/engine gets it via constructor
- `HexGrid.distance()` uses correct cube-conversion formula (matches `respawn.py`); `HexCoord.distance()` kept as-is for backwards compat (simpler axial formula)
- 8-way diagonal corner-blocking for square grids is handled in scene handlers (not in `Grid` itself), since `Grid` doesn't know about terrain passability
- `SquareGrid.diagonal_cardinals(direction)` helper returns the two cardinal directions adjacent to a diagonal (for corner-blocking checks)

---

## Deferred — Phase 4 Tasks

These tasks build new gameplay features on top of the grid abstraction. They should be scoped into the milestone that introduces each feature.

### 4.1 — Scene States & Controller Wiring

**Add to:** whichever milestone introduces dungeon or town exploration.

| File | Change |
|---|---|
| `gm/scenes/base.py` | Add `DUNGEON` and `TOWN` to `SceneState` enum |
| `gm/controller.py` | Wire `DUNGEON` and `TOWN` transitions. Pass grid to new scene constructors. |

### 4.2 — Square Grid Terrain Data

**Add to:** whichever milestone introduces dungeon or town exploration.

| File | Description |
|---|---|
| `data/terrain_square.yaml` | **NEW** — Square grid terrain definitions. City: street, building, park, alley, plaza, wall. Dungeon: corridor, room, wall, door, stairs, trap. Town: road, house, shop, tavern, temple, open_ground. |

### 4.3 — Dungeon Scene

**Add to:** dungeon milestone.

| File | Description |
|---|---|
| `gm/scenes/dungeon.py` | **NEW** — `DungeonScene` using square grid (8-way movement). Room entry triggers descriptions, encounters, loot. Diagonal moves blocked when both adjacent cardinal cells are walls. Integrates with existing `dungeons` DB table (rooms JSON + connections JSON). |
| `tests/test_dungeon_scene.py` | **NEW** — Movement, room transitions, exit to exploration. |

**Scene flow:**
```
World map (hex) → "enter dungeon" at dungeon hex → Dungeon scene (square grid)
Dungeon scene → reach exit cell → back to world map hex
```

**Corner-blocking rule:** Can't move NE if both N and E cells are walls/impassable. Use `SquareGrid.diagonal_cardinals()` to find the two cardinal neighbors to check.

### 4.4 — Town Scene

**Add to:** town/city milestone (potentially CWN expansion).

| File | Description |
|---|---|
| `gm/scenes/town.py` | **NEW** — `TownScene` using square grid (8-way movement). Entry via `explore town` at settlement hex. NPCs positioned at specific cells. Shops at shop cells. Exit back to world hex map via edge cells or `leave`. |
| `tests/test_town_scene.py` | **NEW** — Movement, NPC interaction, shop access, exit to exploration. |

**Scene flow:**
```
World map (hex) → settlement hex → "explore town" → Town scene (square grid)
Town scene → "leave" or exit via edge cell → back to world map hex
```

### 4.5 — Square Grid Generator

**Add to:** whichever milestone first needs procedural square maps.

| File | Description |
|---|---|
| `generators/square_gen.py` | **NEW** — `SquareWorldGenerator` for city blocks, town layouts, dungeon floors. Takes `width`, `height`, `context` ("city"/"town"/"dungeon"). |
| `tests/test_square_gen.py` | **NEW** — Valid grids, terrain placement, connectivity. |

**Algorithm by context:**
- **Dungeon:** Random room placement on grid, corridors connecting rooms, doors at room boundaries. Persists via existing `dungeons` DB table.
- **Town:** Main road(s) through center, buildings along roads, open areas (plaza/park), NPCs placed in buildings. Uses settlement data from hex for context.
- **City (CWN):** Block-based layout with streets forming a grid, buildings filling blocks, districts with different character. Stubbed until CWN is scoped.

### 4.6 — Frontend Square Map Renderer

**Add to:** whichever milestone first creates a square map in-game.

| File | Description |
|---|---|
| `frontend/src/components/SquareMap.vue` | **NEW** — SVG rectangle grid renderer. Each cell is a colored rectangle. Player marker, fog of war, feature icons — same data format as HexMap but rendered as squares. |
| `frontend/src/components/HexMap.vue` or parent | Conditional: render `<HexMap>` or `<SquareMap>` based on `grid_type` from map API response. |

### 4.7 — Verification Checklist (for when Phase 4 tasks are implemented)

- [ ] `SceneState.DUNGEON` / `TOWN` added and wired in controller
- [ ] Dungeon scene: enter from exploration, move 8 directions, room descriptions, encounters, exit back to world map
- [ ] Town scene: enter from settlement hex, move through grid, talk to NPC at cell, shop at shop cell, leave to world map
- [ ] Corner-blocking: diagonal moves blocked when both adjacent cardinals are impassable
- [ ] `SquareMap.vue` renders town/dungeon grid correctly
- [ ] Square generator creates connected, valid grids for each context
- [ ] `mypy --strict` passes on all new/modified files
- [ ] `npx vue-tsc --noEmit` passes for frontend
- [ ] Unit + property + mutation tests for all new modules
- [ ] Playwright E2E tests for SquareMap component
