# Outpost 3 — Master Task List

**Design doc:** `docs/Outpost3_Design_V5.md`
**Engine:** Godot 4 + C#
**Test command:** `dotnet test tests/OutpostCore.Tests/OutpostCore.Tests.csproj`
**Current test count:** 163 passing

---

## Status Legend

- `[x]` Complete
- `[-]` In progress
- `[ ]` Not started
- `[~]` Deferred / backlog

---

## Phase 0 — Headless Core Domain ✅

- [x] Project scaffold (godot/, tests/OutpostCore.Tests/)
- [x] Resource definitions (26 resources, Raw/Refined/Advanced/Virtual tiers)
- [x] Building definitions (13 buildings with skills, recipes, power)
- [x] ColonyGrid — multi-size placement, occupancy map, overlap/bounds validation
- [x] ResourceStore — add/consume/cap/snapshot
- [x] LaborPool — allocation, idle/allocated tracking
- [x] PowerGrid — producers/consumers, brownout, essential priority
- [x] PopulationGroup — needs, health/morale deltas, deaths
- [x] TurnManager — sol advancement, strategic turn bridge, wait directive
- [x] ColonyTurnProcessor — construction, production, consumption, needs, deaths
- [x] ColonyState — full state container
- [x] ColonySession — high-level API (QueueConstruction, EndTurn, AutoAssign, Save/Load)
- [x] Save/load — JSON round-trip via ColonySaveData

## Phase 1 — Isometric Colony Grid ✅

- [x] TerrainGenerator — biome-aware two-pass generation (6 biomes)
- [x] ColonyGridView — _Draw() based isometric renderer, biome color palettes
- [x] Building placement ghost (valid/invalid color feedback)
- [x] Basic camera controls

## Phase 2 — Colony Survival Loop (UI Layer)

### 2.1 — Domain ↔ Godot connection
- [x] ColonyViewController — owns ColonySession, drives turn advancement, updates grid view *(ColonyScene + ColonyHud)*
- [x] Building state changes reflected in tile tints (constructing, damaged, unpowered) *(ColonyGridView.StyleForState)*
- [x] Signal wiring: TurnManager.TurnAdvanced → processor → grid refresh *(ColonySession.StateChanged)*

### 2.2 — Build menu
- [x] Categorized building list panel (group by type: Power, Production, Habitat, Storage)
- [x] Cost display per building, greyed-out when resources insufficient
- [x] Queue construction on click, show construction progress bar per building *(ColonyGridView.DrawProgressBar)*
- [x] Cancel construction command

### 2.3 — Resource HUD
- [x] Top bar: resource amounts with delta-per-turn indicators (+/- arrows)
- [x] Storage capacity bar per resource
- [x] Power grid status (capacity vs consumption, brownout indicator)

### 2.4 — Population panel
- [x] Population count, health bar, morale bar
- [x] Needs satisfaction breakdown (food/water/oxygen/housing — % met each)
- [x] Labor: total / allocated / idle worker counts
- [x] Skill distribution display

### 2.5 — Event log
- [x] Scrolling event log panel with severity color coding
- [x] Alert badges for critical events (starvation, death, power failure)
- [x] Event choice modal for player-decision events (when events have options)

### 2.6 — Turn advance controls
- [x] Advance 1 turn button
- [x] Advance N turns (input field)
- [x] Turn counter display (Sol NNNN)
- [x] Speed mode (skip rendering between ticks for rapid advance)

### 2.7 — New game setup
- [x] Colony name input
- [x] Difficulty preset selector (Sandbox / Easy / Normal / Hard / Brutal)
- [x] Biome selector (Barren / Rocky / Polar / Desert / Volcanic / MarginalHabitable)
- [x] Pass SiteDefinition to colony scene on confirm

### 2.8 — Save/load UI
- [x] Save slot list (up to 5 slots) *(plus dedicated autosave + quicksave slots)*
- [x] Save with name, load with confirmation, delete
- [x] Autosave every 10 turns
- [x] Quick save (Ctrl+S)

## Phase 3 — Content Data Loading

Goal: migrate hardcoded BuildingRegistry and ResourceRegistry to data files.

### 3.1 — JSON content pipeline
- [x] Design JSON schema for buildings (id, name, category, size, costs, recipe, power, labor, skill)
- [x] Design JSON schema for resources (id, name, tier, category, weight, description)
- [ ] Design JSON schema for events (id, trigger, probability, effects, player choices)
- [x] ContentLoader class in Core — parses JSON strings, validates, populates registries
- [x] Replace hardcoded BuildingRegistry static initializer with ContentLoader
- [x] Replace hardcoded ResourceRegistry static initializer with ContentLoader
- [x] NUnit tests validate JSON parses correctly (duplicates, missing fields, bad enums rejected)
- [x] NUnit tests confirm all existing building/resource IDs still resolve after migration
- [x] Runtime file loading from `res://content/*.json` via `ContentBootstrap` (with user:// override)

### 3.2 — Content expansion
- [x] Add building categories to building definitions *(done in Phase 2.2)*
- [x] Add building upgrade tiers (mk1→mk2 via `UpgradesTo` + `ColonySession.UpgradeBuilding`; same-footprint required)
- [x] Tech tree JSON schema (id, prerequisites, unlocks, research cost) + `TechRegistry`
- [x] JSON-driven event registry — schema + `EventRegistry` + bootstrap loading
- [x] `RandomEventProcessor` consumes `EventRegistry` via `EventOutcomeExecutor` (arrival event still hardcoded — morale gating not yet in schema)

## Phase 4 — Planet Hex Map

- [ ] HexPlanetMap scene — Godot hex TileMapLayer
- [ ] Placeholder hex tiles, biome color-coded
- [ ] PlanetGenerator — seed → hex grid (TerrainType, BiomeType, ResourceDeposits)
- [ ] Landing site selection — hex → preview panel → SiteDefinition → colony scene
- [ ] Scene transition: planet → colony (pass SiteDefinition), colony → planet (summary overlay)

## Phase 5 — Strategic Layer Skeleton

- [ ] StrategicTurnManager — fires every 30 colony sols
- [ ] Strategic project system (ProjectDefinition, cost, duration, completion bonus)
- [ ] First projects: Orbital Survey, Weather Station, Planetary Highway Segment
- [ ] Resource flow: colony surplus → StrategicResourcePool
- [ ] Strategic HUD: month/year, pending projects, pool levels

## Phase 6 — Multiple Colonies + Auto-Sim

- [ ] Found Colony expedition project
- [ ] Inter-colony trade routes (source, destination, resource, max flow)
- [ ] AutoSimAI — construction priority, labor allocation, power management, emergency response
- [ ] Auto-sim colony survives 500 turns unattended with reasonable starting conditions

## Phase 7 — Full Strategic Layer

- [ ] System-wide projects (Asteroid Mining, Gas Giant Extraction, Orbital Station, Terraforming)
- [ ] System map skeletal view (planet node, asteroid belt, gas giant)
- [ ] All difficulty multipliers verified across colony and strategic layers
- [ ] Codex entries for buildings, resources, projects, biomes
- [ ] Calculation breakdown tooltips on stat values

## Phase 8 — Interstellar + Victory

- [ ] Victory conditions: Economic, Population, Tech, Timeline
- [ ] Interstellar expedition project (multi-stage: prep → launch → transit → arrival)
- [ ] Second system skeleton — new planet generation on arrival
- [ ] CI hardening (NUnit green in CI, content validation)

---

## Backlog — Ideas from Archive (not yet scheduled)

These came from earlier design iterations and are candidates for any phase above.

### Systems depth
- [ ] Building efficiency modifiers — power deficit, morale, and pollution all scale output continuously (not binary)
- [ ] Resource depletion — deposits on biome tiles have a depletion field; older colonies see declining yields
- [ ] Pollution system — factories generate pollution per turn; accumulates; reduces morale and increases mortality; recycling buildings reduce it
- [ ] Waste/recycling — production creates waste resource; recycler converts waste → usable materials at 80–90%
- [ ] Research system — lab buildings, tech tree with prerequisites, permanent efficiency bonuses and building unlocks
- [ ] Policy system — enact/repeal colony-wide policies (Increased Automation, Labor Protections, Resource Rationing) with tradeoffs
- [ ] Automation scripts — player-defined rules (if power < 30%, shut down non-essential factories)
- [ ] Soft brownouts — instead of hard shutdowns, buildings operate at reduced efficiency proportional to power deficit

### Colony management
- [ ] Morale factor breakdown — decompose morale into named factors (housing quality, food, pollution, unemployment, recreation) so players can target fixes
- [ ] Job specialization — colonists have role effectiveness ratings; assign Engineers to reactors for efficiency bonus
- [ ] Immigration/emigration — colonies gain immigrants when morale > 70%, lose colonists (emigration) when morale < 30%
- [ ] Building upgrade tiers — mk1/mk2/mk3 upgrades that increase output, power draw, and labor requirements
- [ ] Achievement system — milestone tracking (First 100 Pop, No Casualties, 1000 Steel Produced)

### UI/UX
- [ ] Building outliner sidebar — list of all placed buildings with status badge icons
- [ ] Map data layers — toggle overlays: Power Layer, Resource Layer, Production Layer, Pollution Layer
- [ ] Charts panel — line charts for resource stock history, bar chart for power generation vs consumption
- [ ] Event decision modal — structured UI for player-choice events with outcome previews
- [ ] Building detail modal — stats, current workers, efficiency breakdown, actions (pause, upgrade, demolish)

### Content
- [ ] More building types: Recycler, Research Lab, Medical Bay, Recreation Center, Water Well, Atmospheric Processor
- [ ] More resources: Waste, Rare Earth, Hydrogen, Machinery, Medical Supplies
- [ ] More events: equipment failure with repair option, colonist morale crisis with choice, resource cache discovery
- [ ] Faction system (late game) — NPC traders with relationship scores, trade agreements, embargo mechanics

---

## Repository Cleanup Remaining

- [ ] Decide on `node_modules/` — remove if the old web stack is fully abandoned
- [ ] Decide on `godot/tests/` — audit which GDUnit4 tests are still valid vs obsolete
- [ ] Update `.github/workflows/build.yml` and `visual_tests.yml` to reflect Godot+C# stack
- [ ] Update `.github/AGENTS.md` and `copilot-instructions.md` to match new direction
- [ ] Remove or repurpose `old/` once Rust code is confirmed no longer needed as reference
