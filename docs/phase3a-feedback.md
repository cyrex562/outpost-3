# Phase 3A — Gameplay Test Feedback

**Date started:** 2026-05-17
**Build under test:** Phases 0–3 complete (146 NUnit tests passing)
**Tester:** cyrex
**Output of this doc:** Triaged backlog for Phase 3B, 3C, 3D, … (likely an art
phase, a missing-UI phase, and a balance/bug-fix phase).

> Marking convention: `[Works]`, `[Partial]`, `[Broken]`, `[Confusing]` — leave
> a note on anything that isn't `[Works]` with no comment.

---

## First impressions (open notes)

- **Overall feel of the game on first launch:** Window is too small for the
  amount of HUD content — menus overlap each other and the playable colony
  area is cramped. Default resolution should be closer to 1920×1080.
- **Art / visual gripes (current is 100 % procedural — what jumps out):**
  - Construction-progress health/state slivers are too short to read at
    a glance — should be a little taller.
  - The slivers on the **Ice Drill** specifically are drawn at the wrong
    angle (the iso-projection math probably breaks for the 2×3 footprint).
- **"I expected X but got Y" moments:**
  - Construction rate of 1 sol per construction-turn feels very slow. A
    Basic Habitat takes 300 sols. Players expect either faster baselines
    or labour/skill to accelerate it.
  - No Escape-key in-game menu — players expect Esc to summon
    save / load / quicksave / settings / exit-to-menu / exit-to-desktop.
  - No mini-map.
  - No way to click a placed building and see its details.

---

## Per-feature checklist

### Main menu (`MainMenuScene.cs`)
- [ ] **New Colony flow** (name / difficulty / biome / seed) —
- [ ] **Continue** (auto-picks most-recent slot) —
- [ ] **Load Game…** (slot picker, all 7 slots visible) —

### Colony view (`ColonyGridView.cs`)
- [ ] Terrain colours read clearly per biome —
- [ ] Camera pan / zoom feel —
- [ ] Building footprint readability (operational / under-construction / damaged) —
- [ ] Construction-progress sliver visible while building —

### Build menu (HUD left panel)
- [ ] Categorized list (Power / Life Support / Habitat / Production / Storage) useful —
- [ ] Cost greying matches actual affordability —
- [ ] Hotkeys 1 / 2 / 3 discoverable (or do you want a tooltip?) —
- [ ] Cancel placement (ESC / right-click) works —
- [ ] "Cancel" button in **Under Construction** panel refunds 50 % —
- [ ] "Repair" button in **Damaged Buildings** panel works —

### Turn advance (HUD top)
- [ ] End Turn / Skip 10 / Skip 30 — cadence feels right —
- [ ] Custom Skip-N SpinBox sane —
- [ ] ⚡ Fast skip is actually faster (no frame stutter on long skips) —

### Resource HUD (bottom ticker)
- [ ] Per-resource tile readable (name / amount/cap / fill bar / ▲▼ delta) —
- [ ] Delta arrows useful for survival decisions —
- [ ] Storage cap rises after warehouse becomes operational —

### Population panel (HUD left)
- [ ] Pop count header informative —
- [ ] Health / morale bars colour shifts (green → yellow → orange → red) sensible —
- [ ] Needs breakdown (food / water / oxygen / housing %) clear —
- [ ] Labor "X/T working (I idle)" line useful —
- [ ] Skill distribution line readable (Lab/Eng/Sci/Far/Med/Op counts) —

### Power summary (HUD left)
- [ ] "X MW gen / Y MW used" readable —
- [ ] Brownout deficit warning shows when over-drawing —

### Events
- [ ] Random events pace correctly (Normal: every ~30 sols) —
- [ ] Critical-event badge (`⚠ N` red pulse in Event Log header) noticeable —
- [ ] Decision modal (**Mysterious Signal**) — title / description / two choices clear —
- [ ] Decision outcome reflects in resources + log immediately —

### Save / load
- [ ] **Ctrl+S** quicksave works (FlashStatus confirms) —
- [ ] **⚡ Quicksave** HUD button works —
- [ ] **Save…** modal: 5 manual slots + Quicksave + Autosave row visible —
- [ ] Autosave fires every 10 sols (check timestamps in Save modal) —
- [ ] Delete slot works —
- [ ] **Continue** from main menu picks the right (latest) save —
- [ ] **Load Game…** from main menu lists all slots —

---

## Known gaps (don't chase these during 3A)

- **Building upgrade** (Mk1 → Mk2) — `ColonySession.UpgradeBuilding()` works in
  Core but no HUD button. **Cannot be tested via gameplay.** → Phase 3B task.
- **Per-building labor sliders** — only global `Auto-Assign Labor` button. →
  Phase 3B task.
- **Tech tree UI** — `TechRegistry` loaded from `tech.json` but no UI. → Phase
  3C-or-later task.
- **Event browser / codex** — `EventRegistry` loaded but no UI to inspect. →
  later task.

---

## Missing / wanted features (drives Phase 3B+)

Free-form list. What did you expect to be able to do that you couldn't?

- **Bigger default window** — boot at ~1920×1080 so the HUD isn't fighting
  for space.
- **Multi-panel docking UI** — instead of fixed HUD panels, panels should
  be **movable / resizable / closeable / re-openable**, with a menu bar
  (top of screen?) to toggle each one. Targets: build menu, population,
  events, resources, future tech tree, future mini-map, building details.
- **Expandable/collapsible build menu** — submenus per category (Power /
  Production / Habitat / Storage / Life Support) that the player expands
  and collapses, rather than a single flat scroll list.
- **Compact resource HUD** — current per-resource tile layout takes too
  much real estate. Either shrink it or move it into an opt-in panel.
- **Icons + tooltip/popup details** — replace long text rows with icons
  that show a caption, and on click open a popup with the full details.
- **Click a building → details panel** — selecting a placed building
  (under-construction OR operational) opens a panel showing stats,
  workers, recipe, state, upgrade option.
- **Mini-map** — a small overview panel of the colony grid.
- **Resource details screen** — a dedicated, dockable panel that shows
  current stock / production / consumption per resource (drill-down from
  the compact HUD).
- **In-game menu on Escape** — save / load / quicksave / end turn /
  exit to main menu / exit to desktop / settings.

---

## Art upgrade targets (drives the art phase)

What needs to stop being procedural diamonds and become real art?

### Terrain
- 

### Building sprites
- 

### UI / fonts / icons
- 

### HUD widgets (progress bars, modals, etc.)
- 

### Audio (if any)
- 

---

## Bugs (anything actually broken)

- **Ice Drill construction sliver drawn at the wrong angle.** Likely a
  bug in the iso-projection math when a building has a 2×3 (non-square)
  footprint — the progress-bar diamond endpoints are computed wrong.
  See `ColonyGridView.DrawProgressBar` and `DrawBuildingSlot`. Probably
  same root cause affects any 2×3 / 3×2 footprint.
- **Health/state slivers are too short to read** — purely a visual /
  thickness issue, easy fix in the same render method.

---

## Balance / pacing notes

- **Construction times feel slow.** Currently a Basic Habitat = 300 sols,
  Nuclear Reactor = 500 sols, Solar Array Mk1 = 120 sols. The rate is
  hardcoded at **1 turn per sol per building, with no labour or skill
  acceleration** (`ColonyTurnProcessor.AdvanceConstruction` simply
  decrements `ConstructionTurnsRemaining`). Open question: should base
  numbers be reduced, or should labour-assigned workers + Engineer skill
  speed it up, or both?

---

## Triage (revised after first feedback round)

- **Phase 3B — UX quick wins** ✅ **DONE**
  - 1920×1080 default window in `project.godot`
  - Esc in-game menu (Resume / Save / Load / Quicksave / Settings stub /
    Exit to Menu / Exit to Desktop)
  - Construction progress bar: horizontal regardless of footprint
    (Ice Drill bug fixed), thicker (9 px), more prominent
  - Stopgap ~3× construction-time rebalance in `buildings.json` +
    `EmbeddedContent.cs` so the slow feel is mitigated until 3C lands

- **Phase 3C — Construction overhaul** (next, larger plan)
  - `ConstructionFleet` + `OperatorPool` in `ColonyState`
  - `RequiredFleetSlots` + `RequiredOperators` on `BuildingDefinition`
  - New `prefab_components` resource (single type for now; multi-type later)
  - **Starter loadout**: pre-placed Lander + multi-purpose habitat,
    generous prefab/resource stockpile
  - All buildings cost `prefab_components` + secondary materials
  - New `vehicle_factory` building (produces fleet slots over time)
  - New `prefab_factory` building (consumes raw mats → produces prefab)
  - Tech: `expanded_logistics` raises fleet cap
  - HUD: fleet/operator/prefab readout; per-building stall indicator

- **Phase 3D — Click-to-inspect** (after 3C)
  - Click building → details panel (stats / workers / recipe / upgrade button)
  - Expandable build menu (submenus per category)
  - Icons + tooltip popups instead of long text rows
  - Wire `UpgradeBuilding` into the details panel

- **Phase 3E — Dockable panel system** (large UI restructure)
  - Movable / resizable / closeable in-game panels
  - Top menu bar to toggle each panel
  - Mini-map panel
  - Resource details panel
  - Compact resource HUD as the default

- **Phase 3F — Art pass** (open-ended)
  - Terrain tiles
  - Building sprites
  - UI / fonts / icon set
  - HUD widget theming
