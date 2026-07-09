# HR-755 — Encounter Window (top-down battle grid)

**Status:** design iteration → implementation
**Date:** 2026-06-30

## Problem

HR-755 asks for an encounter window: a top-down grid of a battle showing the positions of the
player, monsters, and key terrain, updated live as the fight progresses.

Combat in Harsh Realm is **mechanically abstract**: `Combatant`
(`crates/harsh-core/src/combat_runtime.rs`) has no 2D coordinates, only a `range_band` string
(`melee` / `near` / `ranged`); the encounter's world-cell terrain is discarded after creature
selection. So a grid needs invented positions and a captured terrain backdrop.

## Decision

Build a **real battle-grid substrate** (not a purely client-side schematic, and not full
tactical combat). The backend gains genuine per-combatant `(q, r)` and a small terrain
battle-map, emitted via a new `combat.positions` event. Combat mechanics stay **band-driven**
for this iteration — positions *mirror* the range band — but the grid is real state that a
later iteration can promote to authoritative tactical movement.

## Battle grid & position algorithm

Reuses the existing `SquareGrid` / `GridCoord` topology and Chebyshev distance
(`grid.rs`, `resolution/targeting.rs`). All values below are named constants (no magic numbers).

- **Grid:** fixed `GRID_W = 9`, `GRID_H = 9`, `grid_type = "square"`. Cell coords `q ∈ 0..9`
  (column), `r ∈ 0..9` (row), `r = 0` at the top.
- **Enemy cluster center:** fixed at `ENEMY_CENTER = (4, 2)` (near the top).
- **Band → distance:** `band_distance(melee) = 1`, `band_distance(near) = 3`,
  `band_distance(ranged) = 5`.
- **PC token:** `(4, 2 + band_distance(player.range_band))`. Start band is whatever
  `create_combat` sets (currently `melee`, so PC starts at `(4, 3)`). `advance` → `melee` →
  PC at `(4, 3)` (closest); `withdraw` → `near` → PC at `(4, 5)`. The enemy cluster does not
  move — the PC token slides toward/away, which reads as advancing/withdrawing.
- **Enemy slots:** alive enemies are laid out around `ENEMY_CENTER` in roster order using a
  fixed deterministic offset list (spiral), skipping occupied/out-of-bounds cells:

  ```
  ENEMY_OFFSETS = [(0,0), (-1,0), (1,0), (0,-1), (-1,-1), (1,-1),
                   (-2,0), (2,0), (0,-2), (-2,-1), (2,-1), (-1,-2), (1,-2)]
  ```

  Enemy #i takes the i-th usable offset from `ENEMY_CENTER`. No RNG — layout is a pure
  function of the alive-enemy count, so tests are stable.
- **Terrain:** every one of the 81 cells is filled with the encounter's world-cell terrain id.
  The world cell's `features` (up to `MAX_FEATURE_STAMPS = 4`) are stamped as "key terrain"
  onto fixed corner cells `[(0,0), (8,0), (0,8), (8,8)]` (i-th feature → i-th corner, that
  cell's `features = [feature_id]`). All other cells have `features = []`.

Positions are recomputed by a pure helper `assign_positions(&mut CombatState)` that writes
`(q, r)` onto each `Combatant` from the rules above. `create_combat` calls it once; `advance` /
`withdraw` call it again before re-emitting.

## Event contract — `combat.positions`

Emitted as a normal event; `crates/harsh-web/src/wsmsg.rs` already forwards any `*.` event to
the client as a `game_event` frame. **This JSON is the contract both the backend and frontend
implement against.**

Frame the client receives:

```json
{
  "type": "game_event",
  "event": {
    "event_type": "combat.positions",
    "data": {
      "width": 9,
      "height": 9,
      "grid_type": "square",
      "cells": [
        { "q": 0, "r": 0, "terrain": "ruins", "features": ["rubble"] }
        /* … 81 cells total, row-major … */
      ],
      "entities": [
        { "entity_id": "player", "kind": "pc", "name": "Kesh",
          "q": 4, "r": 3, "hp": 8, "max_hp": 8, "alive": true },
        { "entity_id": "ash_crawler_1", "kind": "monster", "name": "Ash Crawler",
          "q": 4, "r": 2, "hp": 6, "max_hp": 6, "alive": true }
      ]
    }
  }
}
```

- `entities` includes the PC (`kind: "pc"`) and every enemy (`kind: "monster"`).
  Defeated enemies are kept with `alive: false` so the frontend shows them as dimmed
  corpses; this keeps the grid consistent whether an update arrives via
  `combat.enemy_defeated` or a `combat.positions` re-emit.
- `entity_id` matches the ids already used in `combat.start` / `combat.attack` so the frontend
  can correlate HP updates.

**Emission points** (in `gm/scenes/combat_scene.rs`):
1. Combat start — right after `make_start_event`.
2. `handle_advance` and `handle_withdraw` — after `assign_positions`.

Live HP/defeat updates reuse existing events (`combat.attack`, `combat.player_hit`,
`combat.enemy_defeated`); `combat.positions` is not re-emitted per attack.

### Rust payload (`crates/harsh-core/src/payloads/notices_combat.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionsCell { pub q: i32, pub r: i32, pub terrain: String, pub features: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionsEntity {
    pub entity_id: String, pub kind: String, pub name: String,
    pub q: i32, pub r: i32, pub hp: i32, pub max_hp: i32, pub alive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatPositionsNotice {
    pub width: i32, pub height: i32, pub grid_type: String,
    pub cells: Vec<PositionsCell>, pub entities: Vec<PositionsEntity>,
}
```

### TS types (`frontend/src/types/api.ts`)

```ts
export interface PositionsCell { q: number; r: number; terrain: string; features: string[]; }
export interface PositionsEntity {
  entity_id: string; kind: "pc" | "monster"; name: string;
  q: number; r: number; hp: number; max_hp: number; alive: boolean;
}
export interface CombatPositionsData {
  width: number; height: number; grid_type: string;
  cells: PositionsCell[]; entities: PositionsEntity[];
}
```

## Frontend

- `stores/encounter.ts` (model on `stores/town.ts`): `width/height/cells/entities`;
  `setGrid(data)`, `updateHp(id, hp, maxHp)`, `markDefeated(id)`, `clear()`.
- `_websocketHandlers.ts`: route `combat.positions` → `encounterStore.setGrid`; extend the
  existing `combat.attack` / `combat.player_hit` / `combat.enemy_defeated` branches to update the
  encounter entity; clear on `gm.scene_change` away from combat. Thread the store through
  `useWebSocket.ts` deps.
- `components/EncounterWindow.vue`: clone `TownMap.vue` (SVG + viewBox pan/zoom + `cellToPixel`);
  terrain `<rect>`s + feature markers; PC/monster tokens with HP coloring reused from
  `CombatPanel.vue`. `data-testid="encounter-window"`, `encounter-entity-{id}`.
- `stores/layout.ts`: add an `encounter` panel to `DEFAULT_PANELS`.
- `views/GameView.vue`: `<PanelWindow panel-id="encounter">` gated on `currentScene==='combat'`;
  extend the combat `watch` to auto-show + focus it.

## Reconnect resync

The WS host re-sends `get_initial_narration()` on every (re)connect. Its `Combat` arm
re-announces `gm.scene_change → combat` and replays the current `combat.start` roster +
`combat.positions` grid, so a client that drops mid-fight recovers the encounter view without
any client-side change.

## Non-goals (this iteration)

Enemy movement, distance-derived range, move/charge actions, LOS/cover, pathfinding. The grid is
a faithful visualization of band-based combat; promoting it to authoritative tactical movement is
a follow-up.

## Verification

`scripts/dev-test.sh` green; new Rust unit tests (deterministic layout, band→distance, defeated
omission); `e2e/encounter.spec.ts` (window renders, PC + monster tokens, `advance` moves the PC
token, defeated enemy token disappears); manual `--serve` check in a live fight.
