# HR-786 — Searchable Loot Sources (ground, containers, corpses)

> Status: design approved 2026-07-06 (full scope + loot panel chosen). Issue: #99.
> Extends the existing `search` + death-marker/`take` systems into a unified
> "search a source → reveal its contents → take" flow with a skill/luck element
> and empty-handed results, across ground, containers, and corpses.

## Goal

Let the player search distinct loot sources on a cell — the ground, placed
**containers** (crates/chests), and **corpses** of defeated enemies/NPCs — to
reveal their contents (gated by a skill/luck roll), then take items. One
consistent flow, one data model, one loot panel.

## Decisions (from planning)

- **Scope:** all three sources in one feature — including dropping lootable
  corpses on enemy/NPC death.
- **UI:** a dedicated **loot panel** showing each revealed source's contents with
  click-to-take, alongside the chat narration.

## What exists / what's new

Reused as-is: the `DiscoverySystem` skill check (2d6 + terrain `survive`/`notice`
skill + attr mod vs difficulty; margin bands `exceptional_failure`…
`exceptional_success`), the luck save (d20 vs 15), the skill-check resolver,
`inventory.item_given`, and the `take` handler. New: the container/corpse
sources, the reveal-then-take flow, and the loot panel.

**Interaction with in-flight work:** the existing `cell.data.death_markers`
(player death drops) and the HR-787 battle-grid `ground_loot` (#124, unmerged)
are **left untouched** to avoid churn/merge conflicts. HR-786 adds a parallel,
additive `cell.data.loot_sources`; the `take` path and loot panel read both
death-markers and revealed loot-sources.

## Data model

A loot source lives on the cell as JSON (mirrors `death_markers`; no schema
migration):

```rust
// crates/harsh-core/src/loot_source.rs (new)
pub struct LootSource {
    pub id: String,           // unique within the cell
    pub kind: String,         // "ground" | "container" | "corpse"
    pub name: String,         // "crate", "ancient chest", "goblin's corpse"
    #[serde(default)] pub contents: Vec<JsonObject>, // item payloads
    #[serde(default)] pub gold: i32,
    #[serde(default)] pub difficulty: i32, // skill-check DC to reveal (0 = trivial)
    #[serde(default)] pub searched: bool,  // revealed yet?
    #[serde(default)] pub empty: bool,      // fully looted / came up empty
}
```

Stored under `cell.data.loot_sources: [LootSource…]`. A small
`LootSourceRepository` (over `&WorldDatabase`, mirroring how `handle_take`
reads/writes `cell.data`) provides `list(coord)`, `upsert(coord, source)`,
`remove_item(coord, source_id, item_name)`, and `mark_searched`.

## Search → reveal flow

`handle_search` (`exploration.rs`) gains source handling **in addition to** the
existing ground DiscoverySystem roll:

1. Gather the cell's `loot_sources` with `searched == false`.
2. For each, roll the terrain-appropriate skill check (`DiscoverySystem`'s
   existing `survive`/`notice` selection) vs the source's `difficulty`.
   - **success** → `searched = true`; margin band decides accessibility:
     `bare_success` reveals the contents but a **luck save** (d20 vs 15) gates a
     "complication" (a fraction stays stuck / a minor mishap narration);
     `solid`/`exceptional` reveal fully, `exceptional_success` adds a small bonus
     roll. Emit a `loot.source_revealed` event with the source payload.
   - **failure** → narrate "you find nothing you can get at"; the source stays
     `searched = false` (retry after the existing per-hex cooldown).
   - **exceptional_failure** → empty-handed: narrate a mishap; optionally mark a
     low-value source `empty`.
3. **Ground:** the existing DiscoverySystem find now creates a **revealed
   ground `LootSource`** (kind `"ground"`, `searched = true`) holding the found
   items instead of granting them straight to inventory — so ground, containers,
   and corpses all flow through take. (Keeps the existing skill check + cooldown;
   the behaviour change is grant→reveal, matching the ticket.)

Empty-handed results and partial reveals come from the margin bands + luck save,
reusing `classify_margin` and `resolve_save` — no new dice code.

## Take flow

`handle_take` is generalised to take from a unified pool:
- current `death_markers` items (unchanged), **plus**
- items in **revealed** (`searched == true`) `loot_sources`.

`take <item>` matches by name across both; `take` (bare) lists everything
available. Taking removes the item from its source (and drops a fully-emptied
source), persists `cell.data`, and emits `inventory.item_given` as today. A
`take all` convenience takes every revealed item.

## Sources: where they come from

- **Ground:** created by a successful ground `search` (above).
- **Containers:** placed at generation.
  - World gen (`generators/world_gen/…`): a low-probability container on
    ruins/lair/wilderness cells (e.g. a weathered crate), value scaled by
    remoteness; `difficulty` from the terrain.
  - Dungeon gen (`generators/dungeon_gen.rs`): the current hardcoded
    `hidden_loot` on rooms becomes a `container` LootSource on the room's cell,
    so dungeon loot uses the same search→reveal→take flow.
- **Corpses:** on enemy defeat in an exploration encounter and on NPC death,
  drop a `corpse` LootSource on the cell holding that creature's/NPC's loot
  (rolled via the existing loot generator). This is the world-map analogue of
  the HR-787 battle-grid drop; combat-victory inventory grants remain, but a
  portion (or the full drop, per tuning) is left on the corpse to search.

## Events

- **`loot.source_revealed`** (new, client-facing): `{ q, r, source: LootSourceView }`
  where `LootSourceView { id, kind, name, contents: [{name,type,value}], gold }`.
  Registered in `CLIENT_FACING_EVENT_TYPES`, ts-rs codegen, and a worldModel
  reducer (guarded by the coverage gate).
- **`loot.source_updated`** / removal: when an item is taken or a source empties,
  emit an update so the panel reflects it (or reuse `inventory.item_given` +
  refetch of the cell's sources). Simplest: `take` re-emits the affected
  `loot.source_revealed` (or a `loot.sources` list for the cell).
- Take still emits `inventory.item_given` (inventory panel + refetch).

## Frontend — loot panel

- **`stores/loot.ts`** (new): `sources: LootSourceView[]` for the current cell;
  `setSources`, `removeItem`, `clear`. Populated by the reveal reducer; cleared
  on move / scene change.
- **`components/LootPanel.vue`** (new): a panel (via the window manager, like
  `InventoryPanel`) listing each revealed source (icon by kind + rarity-tinted
  item rows reusing HR-787's `lootTier`) with a **Take** button per item and a
  **Take all**. Buttons send the `take <item>` command through the existing WS
  send path. `data-testid` `loot-panel`, `loot-source-{id}`, `loot-take-{id}-{item}`.
- **worldModel:** `types/api.ts` + `events.gen.ts` types; a `loot.source_revealed`
  reducer → `model.loot` → projection → `lootStore`; add to `suppressed`/coverage
  bookkeeping as needed.
- Auto-show the panel when a source is revealed (mirror the encounter-window
  auto-open in `GameView.vue`).

## Testing

**Rust (cargo core):**
- LootSource (de)serialize round-trip; repository list/upsert/remove/persist.
- Search reveal: success reveals + emits `loot.source_revealed`; failure leaves
  `searched=false`; exceptional_failure → empty-handed; luck-save complication on
  bare_success. Ground search now yields a revealed ground source (not a direct
  grant).
- Take: pulls from death-markers AND revealed sources; removing the last item
  drops the source; emits `inventory.item_given`; `take all`.
- Generation: containers placed within expected probability (seeded RNG);
  dungeon `hidden_loot` becomes a container source.
- Corpse drop: defeating an enemy in exploration leaves a `corpse` source with
  its loot.
- Coverage gate: `loot.source_revealed` has a reducer.

**vitest:** loot store (setSources/removeItem/clear); reveal reducer →
model.loot; projection → lootStore; `lootTier` reuse.

**Playwright:** search a seeded container cell → assert the loot panel shows the
revealed contents → click Take → item enters inventory and leaves the panel;
an empty/failed search shows the empty-handed narration and no panel items.

Every change ships a regression test that fails without it.

## Delivery — reviewable slices (one branch, one PR closing #99)

1. **Model + repository + events + reveal/take flow** (backend core) — the
   unified `LootSource`, search→reveal, generalised take, `loot.source_revealed`.
2. **Sources** — container placement (world + dungeon gen) and corpse drops
   (exploration combat / NPC death).
3. **Frontend loot panel** — store, reducer/projection, `LootPanel.vue`, types,
   auto-show, e2e.

Each slice is a commit with its tests; the full gate (cargo core+web, clippy,
vue-tsc, vitest, Playwright) must pass before the PR.

## Non-goals / future

- No locked/trapped containers or lockpicking mini-game (a `difficulty`/`locked`
  hook is left for later).
- `death_markers` and HR-787 battle-grid `ground_loot` are not refactored here
  (additive to avoid churn); a later pass could fold them into `LootSource`.
- No weight/encumbrance changes to taking loot beyond what `take` already does.
