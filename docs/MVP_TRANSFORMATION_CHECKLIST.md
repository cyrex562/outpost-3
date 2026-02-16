# Outpost 3 — MVP Transformation Checklist

**Source:** `docs/Outpost3_Design_V5.md` (v5.0, 2026-02-15)
**Purpose:** Track every task required to transform the current codebase into a playable MVP as defined in the V5 design document.
**Current State:** Prototype v0.1.0 — event-sourced Actix-web server with colony/building/train/cargo systems implemented but not aligned to V5 architecture or MVP scope.

---

## Current Codebase vs. V5 MVP — Gap Summary

| Area | Current State | V5 MVP Target | Gap |
|---|---|---|---|
| **Architecture** | Monolithic `src/` with `outpost3` crate | `outpost-core` (pure logic) + `outpost-server` (web) split | Major refactor needed |
| **Data Loading** | Hardcoded enums for buildings, resources | YAML/JSON data-driven definitions | New data loading layer |
| **Time System** | Turn-based (advance manually) | Real-time ticks with pause/play/speed | Rewrite simulation loop |
| **UI Shell** | Basic colony + starmap pages | Full sidebar + top bar + event log ticker layout | Significant template work |
| **Domain Models** | Train/cargo/rail-heavy (pre-V5 scope) | Site/building/resource/population-focused | Simplify and refocus |
| **Event System** | EventStore for sourcing only | Gameplay events with triggers, choices, YAML defs | New game event engine |
| **Save/Load** | Event replay from SQLite | JSON file serialization with auto-save | New persistence layer |
| **Population** | Basic aggregate model | Aggregate + representative characters + needs | Expand significantly |
| **Power/Life Support** | Basic power grid tracking | Full power grid + life support (O2, water, temp) | Expand |
| **Frontend** | HTMX + Tera + Vite/Pixi.js/Alpine.js | HTMX + Alpine.js + Tera (no canvas) | Remove Pixi.js, simplify |

---

## Phase 0: Project Scaffolding & Architecture Refactor

### 0.1 Workspace Restructuring
- [ ] Create Cargo workspace with two crates: `outpost-core` and `outpost-server`
- [ ] Move all pure domain logic (`domain/`, `events/`, `commands/`, `simulation/`) into `outpost-core`
- [ ] Move web layer (`http/`, `db/`, `config.rs`, `main.rs`, templates, static) into `outpost-server`
- [ ] `outpost-core` must have **zero I/O dependencies** — no `actix-web`, no `rusqlite`, no `tokio`
- [ ] `outpost-server` depends on `outpost-core` for all game logic
- [ ] Update `Cargo.toml` workspace root to declare both members
- [ ] Ensure `cargo check --workspace` passes
- [ ] Ensure `cargo test --workspace` passes

### 0.2 Remove Pre-V5 Scope Code
- [ ] Archive or remove train system (`domain/train.rs`, `train_movement.rs`, `train_advancement.rs`)
- [ ] Archive or remove rail system (`domain/rail.rs`)
- [ ] Archive or remove cargo system (`domain/cargo_transfer.rs`, `cargo_events.rs`, `cargo_request.rs`)
- [ ] Archive or remove route/route_assignment system (`domain/route.rs`, `route_assignment.rs`)
- [ ] Archive or remove production chain routes (`domain/production_chain_routes.rs`)
- [ ] Archive or remove station module (`domain/station.rs`)
- [ ] Remove Pixi.js dependency from `package.json` and Vite config
- [ ] Remove Chart.js dependency (not needed for MVP text-and-tables approach)
- [ ] Clean up `old/` directory references if any are imported
- [ ] Remove Bevy-related CI steps from `.github/workflows/build.yml`
- [ ] Update all `mod.rs` files to remove archived module declarations
- [ ] Ensure `cargo check --workspace` and `cargo test --workspace` still pass

### 0.3 Data-Driven Content System
- [ ] Create `content/` directory at project root for YAML data files
- [ ] Add `serde_yaml` dependency to `outpost-core`
- [ ] Implement a `ContentLoader` in `outpost-core` that reads and validates YAML definitions
- [ ] Define YAML schema for building types (see MVP.2)
- [ ] Define YAML schema for resource types (see MVP.3)
- [ ] Define YAML schema for event definitions (see MVP.6)
- [ ] Define YAML schema for tech tree nodes (placeholder for Alpha)
- [ ] Write unit tests: loading valid YAML produces correct domain types
- [ ] Write property tests: round-trip serialization of content definitions

### 0.4 Configuration & Defaults
- [ ] Update `AppConfig` to include V5 game settings (tick rate, idle safety, auto-save interval)
- [ ] Support loading config from file (`config.toml` or `config.yaml`)
- [ ] Add environment variable overrides for all config values
- [ ] Write tests for config loading and defaults

---

## Phase 1: MVP.1 — Foundation & Time System

### 1.1 Game State Model
- [ ] Define core entity hierarchy: `Galaxy` → `StarSystem` → `CelestialBody` → `Site` → `Building`
- [ ] Implement `GameState` root struct holding all simulation state in `outpost-core`
- [ ] Use UUID-based IDs for all entities (`uuid::Uuid` or newtype wrappers)
- [ ] Implement `StarSystem` struct (name, bodies, procedural generation seed)
- [ ] Implement `CelestialBody` struct (name, type, atmosphere, temperature, size, hazard level, resource deposits)
- [ ] Implement `Site` struct (settlement or installation, building list, construction queue, resource stockpile, population, power grid)
- [ ] Define `SiteType` enum: `Settlement` (permanent pop) vs `Installation` (rotating crew)
- [ ] Write unit tests for entity creation and relationships
- [ ] Write property tests for entity ID uniqueness

### 1.2 Time System
- [ ] Implement `GameClock` struct in `outpost-core`: current tick, tick rate, paused state, speed multiplier
- [ ] Define tick duration (configurable, default ~1 game-hour per tick)
- [ ] Implement `TimeCommand` enum: `Pause`, `Resume`, `SetSpeed(u8)` (1x, 2x, 5x, 10x)
- [ ] Implement real-time tick loop in `outpost-server` using `tokio::time::interval`
- [ ] Tick loop calls `outpost-core` simulation step function with current tick number
- [ ] Implement idle safety mode: auto-pause if critical resources drop below threshold
- [ ] Implement auto-pause on critical events
- [ ] Time state exposed via API endpoint for frontend polling
- [ ] Write unit tests: clock advances correctly, pause/resume works, speed changes work
- [ ] Write integration tests: server tick loop advances game state

### 1.3 UI Shell
- [ ] Redesign `base.html` template with V5 layout: sidebar + top bar + content area + event log ticker
- [ ] Implement sidebar navigation component with all MVP categories:
  - Dashboard
  - Colonies (site list)
  - Event Log
  - Settings
- [ ] Implement top bar component:
  - Time controls (pause/play/speed buttons via HTMX POST)
  - Current game date/time display
  - Global resource summary (key resources with trend arrows)
  - Alert indicators (badge counts by severity)
- [ ] Implement collapsible event log ticker at bottom of content area
- [ ] Implement breadcrumb navigation component
- [ ] Apply V5 theme: dark mode, monospace numbers, high contrast, minimal decoration
- [ ] Create CSS variables for theming (colors, fonts, spacing)
- [ ] Wire up HTMX: sidebar links load content into main area without full page reload
- [ ] Write `static/css/` files: `variables.css`, `layout.css`, `sidebar.css`, `topbar.css`, `ticker.css`, `tables.css`, `modals.css`, `forms.css`
- [ ] Remove any canvas/WebGL/Pixi.js references from templates and JS

### 1.4 Dashboard View
- [ ] Create `templates/dashboard.html` with:
  - Colony summary (site count, total population, overall morale)
  - Resource overview (top resources with rates)
  - Recent events (last N events)
  - Active construction (currently building items)
  - Alerts panel (unresolved critical/warning events)
- [ ] Create HTMX endpoint `GET /dashboard` returning rendered dashboard
- [ ] Dashboard auto-refreshes key stats via HTMX polling (every tick)
- [ ] Write integration test: dashboard endpoint returns valid HTML

---

## Phase 2: MVP.2 — Site & Building System

### 2.1 Building Type Definitions (Data-Driven)
- [ ] Create `content/buildings.yaml` with 8-10 MVP building types:
  - `habitat` — housing for colonists
  - `mine` — extracts raw ore from deposits
  - `smelter` — refines ore into metal
  - `fabricator` — manufactures components from metal
  - `solar_array` — generates power (no fuel)
  - `nuclear_reactor` — generates power (consumes fuel)
  - `greenhouse` — produces food
  - `storage_depot` — increases storage capacity
  - `water_purifier` — produces clean water
  - `life_support` — produces oxygen, regulates temperature
- [ ] Each building definition includes: name, category, construction cost (resources), construction time (ticks), labor slots, power consumption/generation, inputs, outputs, description
- [ ] Load building definitions via `ContentLoader` at startup
- [ ] Write unit tests: all MVP building definitions load and validate

### 2.2 Site & Construction Mechanics
- [ ] Implement construction queue on `Site`: ordered list of `ConstructionJob` (building type, progress, resources committed)
- [ ] `ConstructBuilding` command: validates resource availability, labor, adds to queue
- [ ] Construction progresses each tick: `progress += construction_labor_available * efficiency`
- [ ] On completion: building moves from queue to active building list, `BuildingConstructed` event emitted
- [ ] `CancelConstruction` command: returns partial resources (minus waste %), removes from queue
- [ ] `PauseConstruction` / `ResumeConstruction` commands
- [ ] Implement `BuildingState` enum: `UnderConstruction`, `Operational`, `Paused`, `Damaged`, `Destroyed`
- [ ] Write unit tests: construction lifecycle (queue → progress → complete)
- [ ] Write property tests: resource conservation during construction (input = output + waste)
- [ ] Write integration tests: full construction workflow via HTTP

### 2.3 Site Detail View
- [ ] Create `templates/site_detail.html` with tabbed layout:
  - **Overview tab:** Site name, type, population, morale, power status, key stats
  - **Buildings tab:** Table of all buildings (name, state, workers, efficiency, output)
  - **Construction tab:** Queue with progress bars, cancel/pause/priority controls
  - **Resources tab:** (see MVP.3)
  - **Labor tab:** (see MVP.4)
- [ ] Create HTMX endpoints:
  - `GET /site/{id}` — full site detail page
  - `GET /site/{id}/buildings` — buildings tab partial
  - `GET /site/{id}/construction` — construction tab partial
  - `POST /site/{id}/build` — enqueue building construction
  - `POST /site/{id}/construction/{job_id}/cancel` — cancel construction
  - `POST /site/{id}/construction/{job_id}/pause` — toggle pause
- [ ] Build menu: categorized list of available buildings with costs and requirements
- [ ] Building detail modal: click a building row → modal with full stats, toggle on/off, repair
- [ ] Write integration tests: site detail endpoints return valid HTML
- [ ] Write integration tests: construction POST endpoints modify state correctly

### 2.4 Colonies Overview
- [ ] Create `templates/colonies.html` — master list of all sites
- [ ] Table columns: name, type, body, population, morale, power status, building count
- [ ] Click row → navigate to site detail
- [ ] `GET /colonies` endpoint
- [ ] Write integration test for colonies list

---

## Phase 3: MVP.3 — Resource Extraction & Production

### 3.1 Resource Type Definitions (Data-Driven)
- [ ] Create `content/resources.yaml` with 15-20 MVP resources:
  - **Raw:** Iron ore, copper ore, silicon, ice, regolith, uranium ore, carbon compounds, rare earth ore
  - **Refined:** Iron, copper, silicon wafer, water, uranium fuel rod, carbon fiber, rare earth metals
  - **Manufactured:** Structural components, electronics, machine parts, construction materials
  - **Consumable:** Food, oxygen, medical supplies
- [ ] Each resource definition includes: name, category, tier, unit, storage type (bulk, liquid, gas, manufactured), description
- [ ] Load resource definitions via `ContentLoader`
- [ ] Write unit tests: resource definitions load and validate

### 3.2 Resource Deposits
- [ ] Implement `ResourceDeposit` struct on `CelestialBody`: resource type, total quantity, extraction difficulty, depletion rate
- [ ] Procedural generation: assign deposits to bodies based on body type and seed
- [ ] Deposits deplete as resources are extracted
- [ ] Write unit tests: deposit generation produces valid distributions
- [ ] Write property tests: extraction never exceeds deposit quantity

### 3.3 Production Chains
- [ ] Create `content/recipes.yaml` defining production recipes:
  - Mine: labor + power → ore (from deposit)
  - Smelter: ore + power → metal
  - Fabricator: metal + power → components
  - Greenhouse: water + power + labor → food
  - Water purifier: ice + power → water
  - Life support: power → oxygen
- [ ] Each recipe: input resources + quantities, output resources + quantities, processing time (ticks), labor required, power required
- [ ] Buildings reference recipes; some buildings support recipe selection
- [ ] Per-tick simulation: operational buildings consume inputs, produce outputs, deduct from/add to site stockpile
- [ ] Storage capacity limits: production halts if output storage is full
- [ ] Write unit tests: recipe execution produces correct outputs
- [ ] Write property tests: resource conservation (inputs consumed = outputs produced within recipe ratios)
- [ ] Write integration tests: multi-tick production chain (mine → smelt → fabricate)

### 3.4 Resources UI
- [ ] Site Detail — Resources tab:
  - Stockpile table: resource name, quantity, storage capacity, production rate, consumption rate, net rate, trend
  - Color coding: green (surplus), yellow (low), red (deficit/depleted)
  - Storage utilization bar per resource category
- [ ] Global resource summary in top bar: key resources with trend arrows (HTMX polling)
- [ ] Tooltips on resource rows: show which buildings produce/consume, current rates, projections
- [ ] Write integration tests: resource tab renders correct data after production ticks

---

## Phase 4: MVP.4 — Population & Labor

### 4.1 Population Model
- [ ] Implement aggregate population on `Site`: total count, demographic breakdown (age buckets), skill distribution
- [ ] Skill categories: `Laborer`, `Engineer`, `Scientist`, `Farmer`, `Medic`, `Operator`
- [ ] Implement `RepresentativeCharacter` struct: name, age, skills, traits, health, morale, assigned role
- [ ] Generate 5-10 starting representative characters with procedural names and skill assignments
- [ ] Write unit tests: population creation and skill distribution

### 4.2 Needs System
- [ ] Implement `ColonistNeeds` tracker per site: food, water, oxygen, housing satisfaction (0.0–1.0 each)
- [ ] Each tick: calculate demand (population × per-capita consumption rates)
- [ ] Each tick: compare demand to available supply (stockpile + production)
- [ ] Satisfaction = min(supply / demand, 1.0) per need
- [ ] Unmet needs effects:
  - Food < threshold → health decline, eventual deaths
  - Water < threshold → health decline, eventual deaths
  - Oxygen < threshold → rapid death
  - Housing < threshold → morale penalty
- [ ] Write unit tests: needs calculation for various supply/demand scenarios
- [ ] Write property tests: satisfaction is always in [0.0, 1.0]

### 4.3 Labor Assignment
- [ ] Implement labor pool per site: available workers by skill
- [ ] Buildings declare labor requirements (slots by skill type)
- [ ] `AssignLabor` command: assign worker(s) to building
- [ ] `DeallocateLabor` command: remove worker(s) from building
- [ ] Buildings with insufficient labor operate at reduced efficiency
- [ ] Efficiency formula: `min(assigned_workers / required_workers, 1.0) * morale_modifier`
- [ ] Write unit tests: labor assignment and efficiency calculation
- [ ] Write integration tests: labor assignment via HTTP endpoint

### 4.4 Morale System
- [ ] Implement `Morale` as a composite score on `Site` (0–100 scale)
- [ ] Morale factors:
  - Needs satisfaction (food, water, housing, oxygen) — weighted heavily
  - Entertainment / recreation availability
  - Working conditions
  - Recent events (positive/negative modifiers with decay)
  - Governance policies
- [ ] Morale effects:
  - High morale (>70): productivity bonus (+10-20%)
  - Neutral morale (40-70): no modifier
  - Low morale (<40): productivity penalty (-10-30%)
  - Very low morale (<20): risk of event triggers (strikes, unrest)
- [ ] Morale updates each tick based on current conditions
- [ ] Write unit tests: morale calculation from factors
- [ ] Write property tests: morale is always in [0, 100]

### 4.5 Labor & Population UI
- [ ] Site Detail — Labor tab:
  - Worker pool summary: total workers, employed, unemployed, by skill
  - Building labor table: building name, slots filled/required, efficiency
  - Assign/deallocate controls per building
- [ ] Population panel on Site Overview: total pop, morale gauge, growth rate, key needs status
- [ ] Character roster (collapsible): list of representative characters with key stats
- [ ] Write integration tests: labor tab renders and assignment endpoints work

---

## Phase 5: MVP.5 — Power & Life Support

### 5.1 Power Grid
- [ ] Implement `PowerGrid` per site: total generation, total consumption, net surplus/deficit
- [ ] Power-generating buildings contribute to generation (when operational and fueled)
- [ ] Power-consuming buildings draw from the grid
- [ ] Brownout mechanic: if deficit, buildings lose efficiency proportional to shortfall
- [ ] Priority system: essential buildings (life support, habitat) prioritized during brownout
- [ ] `ToggleBuildingPower` command: manually enable/disable power to a building
- [ ] Write unit tests: power grid calculation, brownout priority
- [ ] Write property tests: total consumption never exceeds total generation + deficit tolerance

### 5.2 Life Support
- [ ] Implement `LifeSupport` tracker per site: oxygen level, water level, temperature
- [ ] Life support buildings produce oxygen and regulate temperature
- [ ] Per-tick consumption based on population
- [ ] Failure cascade: if life support fails, oxygen depletes → colonist death within ticks
- [ ] Idle safety mode: auto-pause simulation if life support critical
- [ ] Alerts and event log entries for life support warnings/failures
- [ ] Write unit tests: life support depletion and failure scenarios
- [ ] Write integration tests: idle safety triggers auto-pause

### 5.3 Power & Life Support UI
- [ ] Site Overview: power status widget (generation vs consumption bar, surplus/deficit number)
- [ ] Site Overview: life support status (oxygen, water, temp indicators with green/yellow/red)
- [ ] Power detail section: table of all power-generating and power-consuming buildings with values
- [ ] Brownout alerts in event log
- [ ] Write integration tests: power and life support UI reflects state correctly

---

## Phase 6: MVP.6 — Event System & Log

### 6.1 Game Event Engine
- [ ] Implement `GameEventEngine` in `outpost-core` (distinct from event sourcing `EventStore`)
- [ ] Event engine evaluates trigger conditions each tick against game state
- [ ] Trigger conditions: expressions on game state (e.g., `site.building_count >= 5`)
- [ ] Probability-based firing: when conditions met, roll against probability
- [ ] Event effects: modify game state (resources, morale, building health, population)
- [ ] Event choices: player-facing decisions with different outcomes
- [ ] Skill checks: representative character skills affect outcome probabilities
- [ ] Write unit tests: trigger evaluation, probability rolling, effect application
- [ ] Write property tests: event effects stay within defined bounds

### 6.2 Event Data Definitions
- [ ] Create `content/events.yaml` with 15-20 starter events:
  - **Disaster:** Equipment failure, habitat fire, pressure leak, power surge, storm damage
  - **Discovery:** Mineral vein found, unusual formation, underground cavity
  - **Social:** Morale celebration, interpersonal conflict, skill breakthrough, birth
  - **Technical:** Process optimization, equipment upgrade opportunity, malfunction
  - **Economic:** Supply windfall, waste reduction discovery
- [ ] Each event: id, name, category, severity, auto_pause flag, trigger conditions, probability, description template, choices with effects
- [ ] Write unit tests: all event definitions load and validate
- [ ] Write tests: event descriptions render correctly with template variables

### 6.3 Event Log
- [ ] Implement `EventLog` in `outpost-core`: ordered list of fired events with timestamps
- [ ] Event severity levels: `Info`, `Warning`, `Critical`
- [ ] Event categories for filtering: `Disaster`, `Discovery`, `Social`, `Technical`, `Economic`
- [ ] Color coding by category and severity
- [ ] Auto-pause on critical events (configurable per event type)
- [ ] Write unit tests: event log ordering, filtering, severity classification

### 6.4 Event UI
- [ ] Event log ticker (always visible, bottom of content area):
  - Stream of latest events, color-coded
  - Click event → navigate to relevant entity or expand details
  - Collapsible (toggle visibility)
- [ ] Full event log page (`GET /events`):
  - Table of all events with timestamp, category, severity, summary
  - Filter by category, severity, date range
  - Search by keyword
  - Pagination
- [ ] Event choice modal:
  - Triggered when a choice event fires and auto-pauses
  - Shows event description, relevant data, choice buttons
  - Choice result displayed after selection
- [ ] Alert badges in top bar: count of unread events by severity
- [ ] Write integration tests: event log endpoint, event choice submission

---

## Phase 7: MVP.7 — Save/Load & Settings

### 7.1 Save System
- [ ] Implement `GameState` serialization to JSON in `outpost-core`
- [ ] `SaveCommand`: serialize full game state to a `.json` file in a saves directory
- [ ] `LoadCommand`: deserialize game state from JSON, replace current state
- [ ] Auto-save: configurable interval (default every N ticks), saves to rotating slots
- [ ] Quick save/load: keyboard shortcut triggers save/load of a designated slot
- [ ] Save metadata: save name, game date, real date, play time, colony summary
- [ ] Write unit tests: round-trip serialization (save → load produces identical state)
- [ ] Write property tests: arbitrary game states serialize/deserialize correctly

### 7.2 Save/Load UI
- [ ] Save/load page or modal (`GET /saves`):
  - List of save files with metadata (name, date, summary)
  - Save button (new save or overwrite)
  - Load button (confirm before replacing current state)
  - Delete save button
- [ ] Auto-save indicator in top bar
- [ ] Write integration tests: save/load endpoints work correctly

### 7.3 Settings
- [ ] Settings page (`GET /settings`):
  - **Gameplay:** Tick speed default, idle safety toggle, auto-save interval, event auto-pause categories
  - **Display:** Theme toggle (dark/light — dark default), font size, number format
  - **Keybindings:** Display current keybindings (future: customize)
- [ ] Settings persisted to config file or localStorage (via Alpine.js)
- [ ] Write integration test: settings page renders

---

## Phase 8: MVP.8 — Polish & Integration

### 8.1 Navigation & UX
- [ ] Breadcrumb navigation on all detail pages (e.g., Colonies > Planet Alpha > Settlement Prime)
- [ ] Consistent back links from detail views to parent list views
- [ ] Tooltips on all stat values showing calculation breakdown
- [ ] Loading indicators for HTMX requests
- [ ] Error handling: user-friendly error messages for failed actions
- [ ] Keyboard shortcuts: space (pause/resume), +/- (speed), Ctrl+S (quick save)
- [ ] Write integration tests: navigation flows between views

### 8.2 Styling & Theming
- [ ] Consistent color palette across all views (CSS variables)
- [ ] Resource type colors (consistent everywhere: tables, charts, tooltips)
- [ ] Event severity colors (red=critical, yellow=warning, blue=info)
- [ ] Monospace numbers in all data tables for alignment
- [ ] Responsive layout: works at 1024px+ width (desktop-first)
- [ ] Accessible contrast ratios (WCAG AA minimum)

### 8.3 Onboarding
- [ ] New game setup page: player name, difficulty settings (resource abundance, event frequency)
- [ ] Procedural world generation on new game: star system with bodies, deposits, initial site
- [ ] Starting conditions: initial resources, colonists, buildings based on difficulty
- [ ] Basic tutorial hints: first-time overlays or event log messages guiding the player
- [ ] Codex entries for MVP content: building types, resource types, game concepts

### 8.4 Balance & Testing
- [ ] Balance pass: resource extraction rates, construction times, consumption rates, event probabilities
- [ ] Playtest: complete loop from new game → build colony → survive 100+ ticks
- [ ] All unit tests pass
- [ ] All property tests pass
- [ ] All integration tests pass
- [ ] No `unwrap()` or `expect()` in production code paths
- [ ] No I/O in `outpost-core` crate
- [ ] `cargo clippy --workspace` clean
- [ ] `cargo fmt --check` passes

---

## Cross-Cutting Concerns (Apply Throughout)

### Testing Requirements
- [ ] Every domain model has unit tests
- [ ] Every command has validation tests (valid and invalid inputs)
- [ ] Every production chain has resource-conservation property tests
- [ ] Every HTTP endpoint has integration tests (status code, content type, key content)
- [ ] Event replay determinism: replaying the same events produces identical state (property test)
- [ ] Save/load round-trip: save → load → save produces identical output (property test)

### Code Quality
- [ ] `outpost-core` has zero I/O dependencies
- [ ] All domain errors use `thiserror`
- [ ] All application errors use `anyhow` with context
- [ ] No `.unwrap()` or `.expect()` in non-test code
- [ ] All public APIs have doc comments
- [ ] Structured logging with `tracing` in server code (not in core)
- [ ] All entity IDs are newtype-wrapped UUIDs

### CI/CD
- [ ] Update `.github/workflows/build.yml`:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo fmt --check`
  - Remove Bevy-specific steps
  - Remove Windows build (web-only for MVP)
- [ ] Add CI step: validate YAML content files parse correctly

---

## Dependency Changes for MVP

### Add
```toml
# outpost-core
serde_yaml = "0.9"       # YAML content loading
uuid = { version = "1", features = ["v4", "serde"] }

# outpost-server (in addition to existing)
tokio = { version = "1", features = ["full"] }  # already present
```

### Remove / Move to Dev
```toml
# Remove from production
# pixi.js, chart.js from package.json (not needed for text-and-tables MVP)
```

### Keep
```toml
actix-web = "4"
actix-files = "0.6"
tera = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.24"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
rand = "0.8"
proptest = "1"   # dev-dependency
```

---

## Recommended Implementation Order

1. **Phase 0** — Scaffolding & architecture refactor (foundation for everything else)
2. **Phase 1** — Foundation, time system, UI shell (enables visible progress)
3. **Phase 2** — Site & building system (core gameplay interaction)
4. **Phase 3** — Resources & production (makes buildings meaningful)
5. **Phase 4** — Population & labor (makes the colony alive)
6. **Phase 5** — Power & life support (adds survival tension)
7. **Phase 6** — Events (adds narrative and dynamism)
8. **Phase 7** — Save/load & settings (makes it a real game)
9. **Phase 8** — Polish (makes it playable)

Each phase should end with all tests passing and the application in a runnable state.
