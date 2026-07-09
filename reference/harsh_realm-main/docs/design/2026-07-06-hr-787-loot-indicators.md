# HR-787 — Visual Loot Indicators (map + encounter grid)

> Status: design approved 2026-07-06 (scope + rarity tiers chosen). Issue: #100.
> Two parts: (1) world-map indicators from death-markers (frontend-only),
> (2) a new encounter-grid loot-drop mechanic + rendering.

## Goal

Make worthwhile loot spottable at a glance on the world map and the encounter
battle grid, with the indicator scaled by a named **rarity tier** derived from
item value: **common → rare → epic → legendary**.

## Decisions (from planning)

- **Scope:** both the world map **and** the encounter grid. The encounter grid
  has no loot today (loot is generated post-victory into inventory), so Part 2
  adds a real loot-drop-on-grid mechanic.
- **Tiers:** named rarity tiers (common/rare/epic/legendary), derived from item
  `value` (there is no authored rarity field). Classic rarity colors.

## Rarity tiers (shared, frontend)

Items expose `value: i32` only. A single frontend util maps a representative
value → tier, used by both the world map and the encounter grid so the visual
language is identical:

```ts
type LootTier = "common" | "rare" | "epic" | "legendary";
function lootTier(value: number): LootTier // thresholds below (tunable)
```

| Tier | Value ≥ | Color | Glyph |
|---|---|---|---|
| common | 0 | `#9ca3af` (gray) | small coin dot |
| rare | 25 | `#3b82f6` (blue) | coin pile |
| epic | 75 | `#a855f7` (purple) | gem |
| legendary | 200 | `#f59e0b` (gold) | gem + glow/sparkle |

The **representative value of a loot pile = the max single-item value** in it
(rarity reflects the best drop); gold/currency counts as its face value. Glyph
size and a glow ring scale up per tier. Thresholds are named constants in
`mapGraphics.ts`, easy to tune.

---

## Part 1 — World map indicators (frontend-only)

**Data (already on the client):** `cell.data.death_markers` — the items (and
`gold`) a player drops on death, recoverable via `take`. The map REST response
(`worldsvc.rs map_json`) already sends full `cell.data`, and the client `Cell`
preserves it (index signature). No backend change.

**Render:** in `SquareMap.vue`, add a `lootRenders` computed mirroring the
existing `objectiveRenders`/`encounterRenders` pattern:
- iterate `mapStore.cells.values()`, skip unexplored cells;
- read `cell.data.death_markers`; for each cell with any marker items/gold,
  compute the pile's representative value → `lootTier`;
- emit `{ key, x, y, tier, value }`.
- Template: a `<g transform="translate(x,y)">` group, `data-testid="loot-indicator-{q}-{r}"`,
  `data-loot-tier`, drawing the tiered glyph (color + size + glow by tier).
- Add a `LOOT_COLORS`/thresholds block to `mapGraphics.ts` and a `lootTier` util.
- Add a legend entry in `MapLegend.vue`.

**Freshness:** death-markers change on player death (new pile) and on `take`
(items removed). The map is hydrated on load and cells refresh as the player
moves. Taking loot does not move the player, so a taken pile's indicator could
be stale until the next map refresh. Part 1 accepts this (indicators are a
navigation aid, not authoritative); a `refetch-map` trigger on `take`/respawn is
a possible follow-up (noted, not required).

**Tests:** vitest for `lootTier` (threshold boundaries → correct tier/color);
a component/vitest test that a cell with death-marker loot yields a
`lootRenders` entry with the right tier and none for an empty/loot-free cell;
Playwright: a death-drop shows a `loot-indicator` on the map (drive a death or
seed a marker via the API, then assert the indicator + tier attribute).

---

## Part 2 — Encounter-grid loot mechanic + rendering

Today loot is generated at victory (`handle_victory` → `generate_combat_loot`
over all defeated creatures → `inventory.item_given`) and never appears on the
battle grid. Part 2 makes each defeated enemy **drop its loot onto its grid
tile as it falls**, renders a tiered indicator there, and collects it at victory
— preserving the existing loot economy (same per-enemy rolls) and the HR-107
loot multipliers.

### Backend (`crates/harsh-core`)

1. **`combat_runtime.rs`** — add to `CombatState`:
   ```rust
   #[serde(default)]
   pub ground_loot: Vec<GroundLoot>,   // dropped piles, by defeated enemy
   // struct GroundLoot { entity_id: String, q: i32, r: i32,
   //                     items: Vec<JsonObject>, currency: i32 }
   ```
2. **`combat_scene.rs` — `reconcile_ground_loot(&mut self) -> bool`:** for every
   combatant that is a dead enemy with no `ground_loot` entry yet, find its
   `CreatureData` and roll **that one creature's** loot via the existing
   `LootGenerator::with_difficulty(.., loot_amount_mult, loot_probability_mult)`
   + `generate_combat_loot(slice_of_one, rng)` (so HR-107 mults + the per-enemy
   roll are unchanged), push a `GroundLoot` at the combatant's `(q,r)`, and
   accumulate `harvestable` into `state.pending_harvest`. Idempotent; returns
   whether anything new dropped.
3. **Call sites:** after the player attack resolves (the `if !target_alive`
   region), after `run_enemy_turns`, and after the last-stand attack — anywhere
   enemies can die. When it returns true, emit an updated `combat.positions`
   (the attack path does not currently re-emit positions, so add it there when
   loot dropped) so the grid shows the new pile immediately.
4. **`handle_victory`** — replace the victory-time `generate_combat_loot` call
   with **aggregation of `state.ground_loot`** into `items_gained` +
   `currency_gained` (reconcile once more first, to catch the killing blow).
   XP unchanged. This keeps rolls per-enemy (now at defeat) and avoids
   double-rolling. On flee, uncollected `ground_loot` is forfeited (documented).
5. **`payloads/notices_combat.rs`** — extend `CombatPositionsNotice` with
   `loot: Vec<PositionsLoot>` where `PositionsLoot { q, r, value, item_count }`
   (`value` = representative max item/currency value for tiering). `make_positions_event`
   fills it from `state.ground_loot`.

### Frontend (`frontend/src`)

6. **`types/api.ts`** — add `PositionsLoot` and extend `CombatPositionsData`
   with `loot: PositionsLoot[]`.
7. **`stores/encounter.ts`** — hold `loot` from the positions payload.
8. **`EncounterWindow.vue`** — after the terrain/feature layer and before entity
   tokens, render a tiered loot glyph per `loot` entry at `cellToPixel(q,r)`,
   reusing the same `lootTier`/`LOOT_COLORS` util as the world map.
   `data-testid="encounter-loot-{q}-{r}"`, `data-loot-tier`.

### Tests

- **Rust:** `reconcile_ground_loot` drops one pile per defeated enemy at its
  `(q,r)`, is idempotent (no double-drop on a second call), respects the HR-107
  loot mults, and leaves no drop for still-alive enemies; `handle_victory`
  aggregates `ground_loot` into `items_gained`/`currency_gained` and still emits
  `combat.victory_requested`; `make_positions_event` includes the loot with the
  representative value. Preserve the existing victory-loot tests (update
  expectations for the new timing, keep the economy identical).
- **vitest:** encounter store parses `loot`; `lootTier` shared util (Part 1).
- **Playwright:** enter combat, defeat an enemy, assert an `encounter-loot`
  indicator appears at a grid tile with the expected tier, and that loot is
  collected (inventory) at victory.

## Delivery

Two reviewable slices under this branch/issue:
- **Slice 1 (Part 1):** world-map indicators — frontend-only, low risk.
- **Slice 2 (Part 2):** encounter-grid loot mechanic — backend + frontend,
  higher risk (touches the combat loot flow modified in HR-107); gets its own
  careful review + full combat regression pass.

Each slice ships with the regression tests above and passes the full gate
(cargo core + web, vue-tsc, vitest, Playwright) before merge.

## Non-goals / future

- No authored per-item rarity field — tiers are value-derived (a future content
  pass could add explicit rarity and this util would read it instead).
- No free tactical pickup on the grid (combat movement is range-band abstract);
  ground loot is swept into inventory at victory. Fled/abandoned loot is
  forfeited.
- World-map indicator freshness after `take` (refetch-map trigger) is a possible
  follow-up.
