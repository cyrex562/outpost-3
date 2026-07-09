# Outpost 3 — Codebase Review
**Date:** 2026-07-08  
**Engine:** Godot 4 + C# (.NET 8)  
**Test suite:** 163 tests, all passing  
**Phases complete:** 0, 1, 2, 3 (mostly)  
**Phases not started:** 4, 5, 6, 7, 8

---

## 1. Project Summary

Outpost 3 is a turn-based isometric colony survival game. Players manage a Martian-style outpost sol-by-sol, balancing power, resources, population needs, labor allocation, construction, and random events. The long-term design (documented in `Outpost3_Design_V5.md`) scales toward planetary expansion, orbital infrastructure, wormhole gates, and multi-system grand strategy.

The current implementation is a functional, well-tested MVP covering the single-colony survival loop. The codebase is clean, the architecture is sound, and the test coverage is thorough. The largest gaps are the tech tree simulation, planet hex map, and all post-colony-tier features from the design doc.

---

## 2. Architecture Assessment

### 2.1 Strengths

**Clean Core/Rendering split.** All simulation logic lives in `godot/src/Core/` — pure C# with zero Godot dependencies. This is strictly enforced and makes every system independently testable without spinning up a Godot runtime. The boundary is crossed only at `ColonyScene.cs` → `ColonySession` and in `ColonyHud.cs`/`ColonyGridView.cs`.

**Comprehensive test coverage.** 163 NUnit tests across 14 files cover every major system: grid placement, resource store, power grid, turn processing, construction fleet, skill-based labor, random events, difficulty presets, and save/load round-trips. Tests run headless in under a second.

**JSON-driven content pipeline.** Buildings, resources, events, and tech nodes are all loaded from JSON via `ContentLoader.cs`. The content loader validates IDs, enums, cross-references, and required fields with descriptive exceptions. `EmbeddedContent.cs` provides a compiled-in fallback that keeps tests working without any file I/O.

**Robust construction pipeline.** The FIFO construction queue with atomic fleet-slot + operator allocation is well-designed. Buildings stall cleanly when resources aren't available and resume correctly. Cancel gives a 50% refund; repair costs 30% of construction cost.

**Well-parameterized difficulty and biome systems.** Five difficulty presets with multiplicative resource and consumption scalars. Six biomes with distinct terrain generation ratios and starting resource modifiers. All wired into `ColonySession.SeedDefaults()` and `ColonyTurnProcessor`.

**Skill efficiency system is elegant.** Three-tier skill matching (1.0× exact, 0.80× Laborer generalist, 0.65× wrong specialist) produces meaningful specialization without complexity. `AutoAssignLabor()` respects skill preference order.

**Save/load is complete.** Full JSON round-trip via `ColonySaveData` preserves all state including skill pools, construction queue, tech registry, event log, difficulty, and grid layout.

### 2.2 Weaknesses and Bugs

#### Bug: `uranium_fuel` vs `uranium_fuel_rod` ID mismatch
**Location:** `EmbeddedContent.cs` (nuclear reactor recipe input) and `ColonySession.SeedDefaults()` vs `ResourceRegistry` (registered ID `uranium_fuel_rod`).  
`ResourceStore` silently accepts unregistered string keys, so no exception is thrown — but the reactor maintenance cost spends a resource that is never produced, and the HUD cannot display it by name. Fix: rename all uses to `uranium_fuel_rod` or add `uranium_fuel` as an alias resource entry.

#### Bug: `RecomputePowerGrid` overwrites all storage caps every turn
**Location:** `ColonyTurnProcessor.RecomputePowerGrid()`.  
Each turn, the method calls `SetCap(res.Id, 100f + storageCapacity)` for all non-virtual resources, overwriting the per-resource caps set in `SeedDefaults()`. After the first turn, every resource shares the same cap formula regardless of design intent. Fix: compute storage caps separately from power grid recomputation, or compute caps only when buildings change rather than every turn.

#### Design gap: `BuildingState.Powered`/`Unpowered` not set at runtime
The enum has `Powered` and `Unpowered` variants, and `ColonyHud` has a `StateTint()` branch for `Unpowered`, but the turn processor never transitions buildings into these states — power is always determined via `PowerGrid.IsPowered()` at query time. The HUD's visual for unpowered buildings therefore never fires. This should either be removed from the enum or the turn processor should set the state.

#### Design gap: `spawn_decision` event outcome is a no-op
**Location:** `EventOutcomeExecutor.cs`.  
The outcome kind `spawn_decision` passes content validation and appears in the embedded events JSON but produces no game effect. The comment notes it is "reserved for nested decisions." Either wire it up or remove it from the event schema to avoid silent no-ops.

#### Design gap: colonist arrival is hardcoded outside the event system
**Location:** `RandomEventProcessor.TryTriggerArrival()`.  
Colonist arrivals use a hardcoded morale threshold (≥60), probability (10%), and colonist count (3–8). The comment explains the morale trigger isn't expressible in the current `EventTrigger` schema. This breaks the content-driven principle — arrivals should be an `EventDefinition` entry. Fix: extend `EventTrigger` to support morale-gate conditions.

#### Data inconsistency: orphaned YAML files in `content/`
`content/basic_buildings.yaml` and sibling YAML files are not loaded anywhere. `EmbeddedContent.cs` is the authoritative source. `CLAUDE.md` instructs contributors to edit `godot/content/<file>.json`, but those JSON files do not exist as separate files. This will confuse future contributors.

#### Data inconsistency: `prefab_components` tier
`prefab_components` is classified as `Refined` tier but is manufactured from `steel + components`. It should be `Advanced` tier to match the resource hierarchy documented in the design doc.

#### Design doc architecture section is obsolete
Section 11 of `Outpost3_Design_V5.md` describes a Rust + Actix-Web + SQLite + HTMX web stack. The actual implementation is Godot 4 + C#. The document should be updated to prevent confusion.

---

## 3. System-by-System Review

### 3.1 Colony State (`Core/Colony/`)
**Status: Complete and solid.**

`ColonyState` correctly owns all sub-systems as value-type-adjacent C# objects. The ownership model is clear. `ColonyGrid`, `ResourceStore`, `PowerGrid`, `LaborPool`, `ConstructionFleet`, and `OperatorPool` are all self-contained with clean APIs.

`PopulationGroup.MoraleModifier` uses a step function (five bands) — this creates noticeable cliff effects at thresholds. Consider a smooth lerp for a more gradual difficulty curve.

`PopulationGroup.ComputeDeaths` triggers on `NeedsDeficitTurns > 30` — but there is no per-need deficit counter, only a global one. A colonist can suffer severe food shortage masked by adequate water/oxygen, or vice versa. Consider tracking per-need deficit severity separately.

### 3.2 Content Pipeline (`Core/Content/`)
**Status: Complete for buildings/resources/events/tech. Event JSON schema not finalized.**

`ContentLoader.cs` is one of the most robust files in the project. Validation is thorough and the error messages are descriptive. The cross-reference validation for tech prerequisites is well-implemented.

The `EventRegistry` and `TechRegistry` exist and load correctly. The gap is that neither is wired into any live simulation logic beyond `RandomEventProcessor` (for events) and the display-only HUD panel (for tech).

### 3.3 Simulation (`Core/Simulation/`)
**Status: Core loop complete. Tech tree and advanced events are stubs.**

`ColonyTurnProcessor` implements a clean 12-step pipeline per turn. The ordering is correct: construction → dust storm → production → fleet growth → consumption → needs → deaths → growth → power → status → deltas.

`ColonySession` is the right abstraction for the Godot layer. Its 723-line size is appropriate given it aggregates the full colony API.

`RandomEventProcessor` works correctly for 4 events. The strategic event interval (every N sols per difficulty) is a sensible design.

`DifficultySettings` presets are well-balanced on paper. The Sandbox preset (0× consumption multiplier) effectively disables the survival loop — intentional for testing, but worth noting.

### 3.4 Rendering / UI (`Game/`, `Rendering/`, `UI/`)
**Status: Functional but `ColonyHud.cs` at 2,222 lines is a maintenance risk.**

`ColonyHud.cs` builds the entire UI programmatically in C# with no `.tscn` files. This works but makes the file very large and difficult to navigate. The UI includes: resource tiles, population panel, labor panel, buildings panel, fleet panel, tech panel, build bar, decision modal, save/load modal, and Esc menu — all in one file.

The HUD is functional and covers all MVP requirements. The tech panel is display-only (no interaction). The decision modal correctly blocks input until the player answers.

The isometric renderer (`ColonyGridView.cs`) uses `_Draw()` — correct approach for a custom 2D renderer. Building state tints (constructing, damaged, unpowered) are implemented visually.

---

## 4. Content Coverage vs Design Doc

The design doc describes a resource system with 100+ raw materials and 500+ recipes. The current implementation covers 26 resources and 19 buildings — appropriate for the MVP phase.

| Design Doc Feature | Current Status |
|---|---|
| Colony sim (resources, population, power, buildings) | Complete |
| Construction queue with fleet gating | Complete |
| Random + choice events | Complete (4 events) |
| Skill system + difficulty presets | Complete |
| Biome terrain generation | Complete |
| Save/load | Complete |
| Isometric Godot rendering | Complete |
| Building upgrades (mk1→mk2) | Complete |
| Tech tree data | 7 nodes defined |
| Tech tree simulation (research queue, unlocks) | Not implemented |
| Planet hex map / landing site selection | Not started |
| Multiple colonies | Not started |
| Inter-colony trade routes | Not started |
| Strategic layer (orbital survey, highway, etc.) | Not started |
| Wormhole gates | Not started |
| Victory conditions | Not started |
| Fleet / spacecraft | Not started |
| Governance / policies | Not started |
| Pollution system | Not started |
| Research system | Not started |
| Analytics / achievements | Not started |

---

## 5. TODO List

Tasks are ordered by phase, then by priority within each phase. Items marked **[BUG]** should be fixed before new features.

---

### Immediate Bug Fixes

- [ ] **[BUG]** Fix `uranium_fuel` vs `uranium_fuel_rod` ID mismatch in `EmbeddedContent.cs` and `ColonySession.SeedDefaults()` — reconcile to `uranium_fuel_rod` throughout
- [ ] **[BUG]** Fix `RecomputePowerGrid()` overwriting storage caps every turn — separate storage cap computation from power grid recomputation; only recalculate caps when buildings change
- [ ] **[BUG]** Remove or wire up `BuildingState.Powered`/`Unpowered` — either set these states in the turn processor when power status changes, or remove the variants and the dead HUD branch
- [ ] **[BUG]** Fix `prefab_components` resource tier — change from `Refined` to `Advanced` in `EmbeddedContent.cs`

---

### Phase 3 — Remaining Content Work

- [ ] Design and implement `EventTrigger` morale-gate condition type so colonist arrival can be expressed as an `EventDefinition` (remove hardcoded `TryTriggerArrival`)
- [ ] Wire up `spawn_decision` outcome in `EventOutcomeExecutor` or remove it from the event schema
- [ ] Add at least 6 more events to `EmbeddedContent.cs`: equipment failure with repair choice, food shortage warning, dust storm all-clear, colonist unrest, cache discovery, medical emergency
- [ ] Create `godot/content/buildings.json` and `godot/content/resources.json` as actual files matching `EmbeddedContent.cs` — resolve the CLAUDE.md / orphaned YAML confusion
- [ ] Delete or archive `content/basic_buildings.yaml` and sibling YAML files (they are not loaded anywhere)
- [ ] Update `CLAUDE.md` to accurately describe the content pipeline (embedded JSON in `EmbeddedContent.cs` is authoritative; runtime files are optional overrides)

---

### Phase 3 — Tech Tree Simulation

- [ ] Add `ResearchPoints` to `ColonyState` (accumulated per turn by scientist-staffed research labs)
- [ ] Add `ResearchLab` building to `EmbeddedContent.cs` with Scientist skill, generates `research_data` per turn
- [ ] Add `TechState` to `ColonyState`: researched set, queue, current project progress
- [ ] Implement `TechUnlockProcessor`: apply `TechUnlocks.Buildings` to whitelist new construction options; apply `TechUnlocks.Bonuses` as named multipliers
- [ ] Implement `ColonySession.QueueResearch(techId)` / `CancelResearch()` / `GetResearchProgress()`
- [ ] Wire tech unlock into `ColonySession.BuildableBuildings` — locked buildings should not appear or should be greyed out
- [ ] Connect HUD tech panel to live research state: show current project progress bar, queue, and completed nodes
- [ ] Add NUnit tests for research queue, point accumulation, unlock effects, and prerequisite enforcement
- [ ] Add `research_data` resource entries if not already present as a proper resource

---

### Phase 4 — Planet Hex Map

- [ ] Design `HexCell` record: position, terrain type, biome, resource deposit (type + richness + quantity)
- [ ] Design `PlanetMapState`: seed, body type, hex grid, sites list
- [ ] Implement `PlanetGenerator`: seed → hex grid using biome thresholds; place resource deposits based on body type
- [ ] Create `HexPlanetMap` Godot scene with `TileMapLayer` for hex rendering
- [ ] Implement landing site selection: click hex → preview panel (terrain, deposits, hazard level) → confirm → `SiteDefinition`
- [ ] Scene transition: planet hex map → colony scene (pass `SiteDefinition`); colony → planet (summary overlay)
- [ ] Wire `SiteDefinition.StartingDeposits` into `ColonySession.SeedDefaults()` (field exists but is unused)
- [ ] Add NUnit tests for `PlanetGenerator` — verify deposit distribution by body type, biome coverage ratios

---

### Phase 5 — Strategic Layer Skeleton

- [ ] Implement `StrategicTurnManager`: fires every 30 colony sols, distinct from colony `TurnManager`
- [ ] Design `ProjectDefinition`: id, category, cost (resources), duration (strategic turns), completion effect
- [ ] Implement `StrategicProjectProcessor`: ticks active projects, applies completion bonuses
- [ ] Add first 3 strategic projects to content: Orbital Survey, Weather Station, Planetary Highway Segment
- [ ] Implement `StrategicResourcePool`: surplus from colony production flows here between strategic turns
- [ ] Add Strategic HUD overlay: year/month display, active projects with progress, pool resource levels

---

### Phase 6 — Multiple Colonies and Auto-Sim

- [ ] Implement "Found Colony" as a strategic project that creates a new `ColonyState` from a planet hex selection
- [ ] Design `TradeRoute`: source colony, destination colony, resource, max flow per strategic turn
- [ ] Implement `InterColonyTradeProcessor`: evaluates routes, transfers surplus resources
- [ ] Implement `AutoSimAI`: headless turn processor for unattended colonies using heuristic build/labor/power priorities
- [ ] Survival test: `AutoSimAI` must keep a colony alive for 500 turns from standard starting conditions (Normal difficulty, Rocky biome)
- [ ] Add NUnit tests for trade route resolution, surplus calculation, and AI survival benchmark

---

### Phase 7 — Full Strategic Layer

- [ ] Add system-wide strategic projects: Asteroid Mining Platform, Gas Giant Extraction, Orbital Station, Terraforming Engine (early stage)
- [ ] Implement skeletal system map view: planet nodes + asteroid belt + gas giant as clickable entities
- [ ] Codex system: in-game encyclopedia with entries for all buildings, resources, biomes, and projects
- [ ] Calculation breakdown tooltips: click any stat value to see its component factors (e.g., morale breakdown, power balance)
- [ ] Verify all difficulty multipliers apply correctly across both colony and strategic layers
- [ ] Add achievement/milestone tracking: First 100 Population, No Casualties 100 Turns, 1000 Steel Produced, First Tech Unlock, etc.

---

### Phase 8 — Interstellar and Victory

- [ ] Implement three victory conditions: Economic (cumulative trade volume), Population (threshold), Scientific (all tech researched)
- [ ] Add victory tracking to `ColonyState` / `StrategicState` with per-condition progress
- [ ] Implement interstellar expedition as a multi-stage strategic project: Preparation → Launch → Transit → Arrival
- [ ] On arrival: procedurally generate a second star system with planet hex map
- [ ] CI hardening: add NUnit run to `.github/workflows/build.yml`; add content validation step that loads all JSON and confirms zero validation exceptions

---

### Backlog — Systems Depth

- [ ] Morale factor decomposition: break composite morale into named sub-scores (food satisfaction, housing quality, pollution, unemployment, recreation) so players can target specific fixes
- [ ] Per-need deficit tracking: replace global `NeedsDeficitTurns` with per-need counters for food, water, oxygen, housing — deaths should require specific need starvation, not a blended average
- [ ] Smooth morale-to-efficiency curve: replace the 5-band step function in `PopulationGroup.MoraleModifier` with a piecewise linear or quadratic curve to eliminate cliff effects at band boundaries
- [ ] Building efficiency modifiers: power deficit, morale, and dust storm all currently snap to discrete states — extend to continuous output scaling (a building at 60% power runs at 60% output)
- [ ] Resource depletion: add `DepletionRate` to deposits; older colonies see declining extraction yields from the same mine
- [ ] Pollution system: industrial buildings generate pollution per turn; accumulates on the grid; reduces morale and mortality; recycler building reduces it
- [ ] Waste/recycling: manufacturing generates a `waste` resource; recycler converts waste → usable materials at configurable efficiency
- [ ] Policy system: enact/repeal colony-wide policies (Increased Automation, Labor Protections, Resource Rationing) with morale/efficiency tradeoffs and cooldown timers
- [ ] Immigration/emigration: colonies gain immigrants when morale ≥ 70; lose colonists (emigration events) when morale ≤ 30 for sustained periods
- [ ] Soft brownouts: rather than hard binary powered/unpowered, buildings operate at output proportional to available power when in deficit

---

### Backlog — UI/UX

- [ ] Building outliner sidebar: scrollable list of all placed buildings with status badge icons (constructing, operational, damaged, unpowered)
- [ ] Map data layers: toggleable overlays on the isometric grid — Power Layer (green/red per tile), Resource Layer, Production Layer, Pollution Layer
- [ ] Charts panel: line chart for resource stock history (last 100 turns), bar chart for power generation vs consumption
- [ ] Building detail improvements: show full recipe chain (what this building produces, what it feeds into), upgrade path, construction history
- [ ] Morale breakdown tooltip: hover over morale bar to see per-factor contributions
- [ ] Refactor `ColonyHud.cs` (2,222 lines): split into separate panel classes or `.tscn` scenes — one file per logical panel (ResourceHud, PopulationPanel, LaborPanel, BuildMenu, etc.)

---

### Backlog — Content

- [ ] More building types: Recycler, Medical Bay, Recreation Center, Atmospheric Processor, Research Lab (upgrade tier), Water Treatment Plant
- [ ] More resources: `waste`, `hydrogen`, `rare_earth_metals` (already in registry but no producer building), `medical_supplies` producer building
- [ ] Expand event library to 20+ events with varied triggers, choices, and outcomes
- [ ] Faction system: NPC trader factions with relationship scores; trade agreements and embargo events
- [ ] Balance spreadsheet: document intended resource flow rates, production chain throughput, and population scaling — validate against simulation output

---

### Repository Cleanup

- [ ] Update `Outpost3_Design_V5.md` Section 11 (Architecture) to reflect Godot 4 + C# stack — the Rust/Actix-Web description is obsolete
- [ ] Remove `node_modules/` if the old web stack is fully abandoned
- [ ] Audit `godot/tests/` (GDUnit4 tests) — identify which are still valid vs obsolete vs duplicates of NUnit tests
- [ ] Update `.github/workflows/build.yml` and `visual_tests.yml` to reflect the Godot + C# stack
- [ ] Update `.github/AGENTS.md` and `copilot-instructions.md` to match current engine and architecture
- [ ] Remove or archive `old/` directory once confirmed no longer needed as reference

---

## 6. Risk Areas

**`ColonyHud.cs` at 2,222 lines** is the highest maintenance-risk file. Adding more UI panels will make it harder to navigate. Should be refactored into panel components before Phase 4 UI work begins.

**No CI integration yet.** Tests pass locally but there is no automated gate. A single wrong merge could introduce a silent regression in the 163-test suite.

**Design doc vs implementation divergence.** The design doc describes a web-based stack and a vastly larger feature set than implemented. New contributors reading the doc will have incorrect expectations. The doc needs an "Implementation Status" section aligned with `TODO.md`.

**Resource ID fragility.** `ResourceStore` silently accepts unregistered string keys. The `uranium_fuel` bug demonstrates this risk — the system fails silently rather than loudly. Consider adding a `Debug.Assert` or compile-time registry validation to catch unregistered IDs early.

---

## 7. What to Build Next

Based on the current state and the design goals, the recommended sequence is:

1. **Fix the two bugs** (`uranium_fuel` ID, storage cap overwrite) — they create silent incorrect state that will mask balance testing.
2. **Complete the event content** (6+ more events, wire `spawn_decision`, move arrival to data) — events are the primary player engagement mechanism in the current loop.
3. **Implement tech tree simulation** — the data is there, the display panel is there; the missing piece is research points and unlock application. This gates late-game content.
4. **Planet hex map** (Phase 4) — the first major new feature; gates everything that follows.
5. **Refactor `ColonyHud.cs`** before starting Phase 5 UI work.
6. **Add CI** (`dotnet test` in GitHub Actions) before Phase 6 multi-colony work introduces more complexity.
