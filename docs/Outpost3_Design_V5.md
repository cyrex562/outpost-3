# Outpost 3 — Design Document V5

**Status:** Active Design
**Version:** 5.0
**Last Updated:** 2026-02-15

---

## Table of Contents

1. [Vision & Concept](#1-vision--concept)
2. [Core Design Principles](#2-core-design-principles)
3. [Game Progression & Phases](#3-game-progression--phases)
4. [Time System](#4-time-system)
5. [UI Layout & Navigation](#5-ui-layout--navigation)
6. [Views & Pages](#6-views--pages)
7. [Game Systems & Mechanics](#7-game-systems--mechanics)
8. [Resource System](#8-resource-system)
9. [User Workflows](#9-user-workflows)
10. [System Dependency Matrix](#10-system-dependency-matrix)
11. [Architecture](#11-architecture)
12. [Modding Support](#12-modding-support)
13. [Development Roadmap](#13-development-roadmap)
14. [Open Questions & Future Work](#14-open-questions--future-work)

---

## 1. Vision & Concept

### Premise

The player leads humanity's effort to establish a self-sustaining colony beyond the Solar System. The game blends strategic planning, resource and systems management, and narrative encounters across multiple layers of scale — from a single landing site to planetary settlement, orbital infrastructure, and eventual interstellar expansion through wormhole gates.

### Elevator Pitch

A deeply simulated colony-building game presented through a data-dense, text-and-tables interface in the style of Melvor Idle and Simple MMO. The player is a strategic director — deciding what to build, research, and prioritize — while the simulation handles tactical execution. Complexity scales from managing a single outpost to governing a multi-world civilization connected by wormhole gates.

### Inspirations

- **UI/UX:** Melvor Idle, A Dark Room, OGame, Simple MMO, text-based MMORPGs
- **Simulation Depth:** Factorio, Satisfactory, Captain of Industry, Dwarf Fortress
- **Narrative & Setting:** Peter F. Hamilton's Commonwealth Saga (wormhole mechanics), The Expanse, Kim Stanley Robinson's Mars trilogy
- **Scope & Progression:** Stellaris, Distant Worlds, Victoria series

### Victory Conditions

Victory is achieved by reaching milestones in one or more categories. The player may continue playing after reaching any or all victory conditions.

| Victory Type | Condition | Description |
|---|---|---|
| **Economic** | Cumulative trade volume exceeds threshold (e.g., $1T traded) | Mastery of production chains, logistics, and inter-colony commerce |
| **Scientific** | Complete all research projects | Full exploration of the tech tree and engineering projects |
| **Population** | Total population exceeds threshold (e.g., 1B colonists) | Sustained growth, quality of life, and multi-world habitation |

Victory thresholds are configurable at game start based on difficulty settings.

### Core Gameplay Loop

```
Develop Colony → Extract Resources → Build Infrastructure → Research Technology
      ↑                                                            ↓
 Expand Network ← Create Trade Routes ← Establish Gates ← Explore New Worlds
```

The inner loop (develop → extract → build → research) operates at the site/colony level. The outer loop (explore → gates → trade → expand) operates at the system and interstellar level, unlocked as the player progresses.

---

## 2. Core Design Principles

### 2.1 Text-and-Tables First

The simulation tracks full positional, physical, and spatial data for all entities (bodies, buildings, vehicles, orbits, routes). However, the player interacts with the simulation entirely through tables, lists, forms, and control panels. There are no maps, hex grids, sprite movements, or spatial visualizations in the base implementation.

Visual elements (graphs, diagrams, maps) may be added incrementally in the future once the simulation mechanics are proven, but they are never required for gameplay.

### 2.2 Strategic Director, Not Tactical Manager

The player makes high-level decisions: what to build, what to research, how to allocate labor, what policies to set. The simulation handles tactical execution: building placement, pathfinding, scheduling, resource routing. This is essential for scaling from 1 site to hundreds of sites without drowning the player in micromanagement.

### 2.3 Automation as a Core Pillar

Automation is not a late-game unlock — it is fundamental to the game's design from the first turn. Every screen and system includes automation policy controls. The player's role shifts over time from hands-on queue management of a single site to policy-setting across a civilization.

Automation policies cascade hierarchically:
- **Global defaults** apply to all sites unless overridden
- **Body-level policies** override globals for all sites on that body
- **Site-level policies** override body-level for that specific site
- **Building/vehicle-level overrides** for fine-grained exceptions

### 2.4 The Event Log Is the Nerve Center

An always-visible event log is the player's primary mechanism for maintaining awareness and control across a growing empire. The event log:
- Is visible on every gameplay screen
- Color-codes events by type and severity
- Supports filtering, searching, and pinning
- Links events to relevant screens/entities (click to navigate)
- Certain event categories auto-pause the simulation, requiring player acknowledgment before resuming (configurable per event type)

### 2.5 Deep Simulation, Moddable Data

Game content (resources, recipes, buildings, tech, events) is defined in data files, not hardcoded. This enables:
- A deeply detailed resource chain rivaling or exceeding Factorio/Satisfactory
- Modder-extensible content (new resources, buildings, production chains, events)
- Balance tuning without code changes

### 2.6 Idle-Safe Operation

The simulation can run in the background with an "idle safety" mode that prevents death spirals while the player is away. When idle safety is active:
- Events that require player choice are queued (simulation continues around them)
- Critical resource depletion triggers automatic conservation/rationing rather than cascading failure
- No colonist death from starvation, suffocation, or exposure while idle safety is active
- The player receives a summary of queued events and automatic actions taken upon return

---

## 3. Game Progression & Phases

The game unfolds in phases that introduce new systems and expand the player's scope. Phases are not rigid gates — systems overlap and the player may engage with multiple phases simultaneously.

### Phase 1: New Game Setup

The player configures a new game:
- Player name and call sign
- Difficulty settings (resource abundance, event frequency, failure tolerance, victory thresholds)
- Starting conditions (technology level, starting cargo, crew size and composition)
- Mod selection (enable/disable installed mods)

The game procedurally generates:
- A target star system with planets, moons, asteroids, and comets
- A landing site on a suitable body with terrain, resources, atmosphere, and hazards
- Initial cargo and colonist manifest based on starting conditions

The player begins at the landing site with basic exploration already conducted and equipment just landed.

### Phase 2: Initial Colony (Single Site)

The player establishes a first settlement:
- Unpack and deploy starting structures (habitats, power, life support)
- Begin resource extraction (mining, harvesting)
- Assign labor to buildings and tasks
- Establish basic production chains (ore → metal → components)
- Manage colonist needs (food, water, air, housing, morale)
- React to early events (equipment failures, weather, discoveries)

### Phase 3: Colony Growth & Development

The settlement matures:
- Expand building inventory (factories, labs, services, entertainment)
- Deepen production chains (multi-step manufacturing)
- Begin research to unlock new technologies
- Excavation: surface terraforming and subsurface expansion
- Population growth and social dynamics
- Establish governance policies

### Phase 4: Planetary Expansion

The player establishes additional sites on the same body:
- Found new settlements (permanent population, full building set)
- Found installations (specialized function, crew-operated: mines, refineries, military, research)
- Build inter-site infrastructure (roads, rails, pipelines, conveyors)
- Establish intra-planetary trade and logistics
- Undertake megaprojects (bridges, dams, canals, large plants)

### Phase 5: Orbital & System Expansion

The player moves beyond the planet surface:
- Launch satellites (communications, mapping, weather, resource survey)
- Build orbital stations (habitation, manufacturing, power, research, fuel depots)
- Explore other bodies in the system (probes, manned expeditions)
- Establish colonies and installations on other planets, moons, asteroids
- Build inter-body trade routes (cargo ships, fuel logistics)
- Mine system resources (asteroids, comets, gas giant atmospheres, stellar material)
- Space program: build and launch ships, missions, probes

### Phase 6: Wormhole Breakthrough (Mid-Game Milestone)

The centerpiece technology milestone:
- Major research project to develop wormhole technology
- Construct the first wormhole gate — a massive engineering project requiring:
  - Significant resources and energy
  - Possibly orbital construction (due to scale)
  - Or a massive surface laboratory/facility
- Wormhole gates are entangled pairs: one endpoint stays, the other must be physically transported to the destination via STL (sub-light) ship
- First gate deployment: build a ship, load the far-end gate, launch to a target system
- Transit time is real (years/decades at sub-light speeds) — the player continues managing their existing civilization while the gate ship travels
- Upon arrival and activation, the gate pair connects the two systems

### Phase 7: Interstellar Network

With wormhole capability proven:
- Send probes and gate ships to additional star systems
- Subsequent research reduces gate size and power requirements
- Each new system: explore, colonize, connect via gate
- Interstellar trade routes through the gate network
- Manage a growing network of connected worlds
- Link colonies and installations to wormhole gates via surface/orbital infrastructure
- Wormhole transport characteristics (instant? bandwidth-limited? power cost per transit?) — see Open Questions

### Phase 8: Late Game & Victory

The player optimizes and expands toward victory conditions:
- Mature economy across multiple systems
- Deep research tree completion
- Population growth across worlds
- Optional: continued expansion beyond victory

---

## 4. Time System

### Real-Time with Player Control

Time flows continuously in the simulation. The player controls the flow:

| Control | Function |
|---|---|
| **Pause** | Simulation halts; player can review, configure, issue orders |
| **Play (1x)** | Normal simulation speed |
| **Fast (2x, 5x, 10x)** | Accelerated simulation; useful for waiting on construction, travel, research |
| **Decelerate** | Step down one speed tier |

### Idle Safety Mode

A toggle (on by default) that prevents catastrophic failure while the player is not actively engaged:

**When idle safety is ON:**
- Events requiring player choice are queued; simulation continues without them
- Critical resource shortages trigger automatic rationing/conservation
- No colonist death from deprivation
- Production stops gracefully when inputs are exhausted (no damage, just idle)
- Auto-pause triggers are suppressed; events accumulate in the queue

**When the player returns:**
- Summary notification of time elapsed, events queued, and automatic actions taken
- Queued choice-events presented in order
- Player can review and adjust before resuming normal play

### Auto-Pause Events

Certain event types automatically pause the simulation (when idle safety is OFF), requiring player acknowledgment. The player configures which event types trigger auto-pause. Default auto-pause events include:
- Major disasters (structural failure, epidemic, radiation event)
- Hostile encounters (if applicable)
- Research breakthroughs with choices
- Wormhole gate arrivals
- Critical resource exhaustion warnings
- First contact or anomaly discoveries

### Time Scale

The base time unit and turn granularity are implementation details. The simulation should support:
- Construction measured in hours/days
- Intra-planetary travel measured in hours
- Orbital operations measured in hours/days
- In-system travel measured in days/weeks/months
- Research measured in days/weeks
- Interstellar STL travel measured in years/decades
- Population growth measured in years/generations

---

## 5. UI Layout & Navigation

### Layout Structure

The UI follows a sidebar-plus-content-area layout inspired by modern web dashboards:

```
+------------------+---------------------------------------------------+
|  SIDEBAR NAV     |  TOP BAR                                          |
|                  |  [Time Controls] [Speed] [Global Resources] [Alerts] |
|  - Dashboard     +---------------------------------------------------+
|  - Starmap       |                                                   |
|  - Colonies      |  MAIN CONTENT AREA                                |
|  - Logistics     |                                                   |
|  - Research      |  (Server-rendered page content via HTMX)          |
|  - Fleet         |                                                   |
|  - Governance    |                                                   |
|  - Analytics     |                                                   |
|  - Event Log     |                                                   |
|  - Settings      +---------------------------------------------------+
|                  |  EVENT LOG TICKER (always visible, collapsible)    |
|                  |  [Filter] [Search] Latest events stream...         |
+------------------+---------------------------------------------------+
```

### Theme & Style

- Dark mode, high contrast
- Monospace numbers for data alignment
- Minimal decoration; information density prioritized
- Tooltips on hover for detailed stats, formulas, modifiers
- Click for actions (instant or modal)
- Color coding: consistent palette for resource types, event severity, status indicators

### Navigation Model

- **Sidebar:** Top-level navigation categories (always visible)
- **Breadcrumbs:** Show current location in hierarchy (e.g., Colonies > Planet Alpha > Settlement Prime > Mine #3)
- **Drill-down:** Tables link to detail views (click a settlement row → settlement detail page)
- **Modals:** Used for quick actions and confirmations without leaving the current page
- **Back navigation:** Every detail page links back to its parent list/overview

### Global UI Elements (Present on All Gameplay Pages)

| Element | Description |
|---|---|
| **Time controls** | Pause / Play / Speed buttons; current game date & time display |
| **Global resource summary** | Key resource levels (power, food, water, air, population) with trend arrows |
| **Alert indicators** | Badge counts for unread events by severity (critical, warning, info) |
| **Event log ticker** | Collapsible bottom panel showing latest events in real-time; click to expand full log |
| **Breadcrumbs** | Current navigation path |

---

## 6. Views & Pages

### 6.1 Meta / Out-of-Game Views

#### Main Menu
- **Purpose:** Entry point to the game
- **Controls:**
  - New Game → New Game Configuration
  - Continue (if save exists) → Load most recent save
  - Load Game → Load Game screen
  - Settings → Game Settings
  - Mods → Mod Management
  - Credits → Credits screen
  - Exit → Close application
- **Display:** Version & build info, background art or animation (future)

#### New Game Configuration
- **Purpose:** Configure a new game session
- **Controls:**
  - Player name and call sign (text inputs)
  - Difficulty preset (dropdown) with individual sliders for: resource abundance, event frequency, failure severity, victory thresholds
  - Starting conditions: technology era, cargo quantity, crew size/composition
  - Mod selector (checkbox list of installed mods, with compatibility indicators)
  - Advanced settings (expandable/modal): detailed tuning
  - Start Game button → generates world and begins at landing site

#### Game Settings
- **Purpose:** Configure application settings
- **Tabs:**
  - Audio: volume sliders (master, music, SFX, notifications)
  - Display: resolution, UI scale, color theme, font size
  - Controls: keybindings, controller configuration
  - Gameplay: auto-pause event types, notification preferences, idle safety toggle, auto-save interval
  - Accessibility: colorblind mode, high contrast, screen reader support, reduced motion

#### Load / Save Game
- **Purpose:** Manage save files
- **Controls:**
  - Save file list (sortable table: date, colony name, system name, play time, game version, mod list)
  - Save file preview: summary stats
  - Load / Save / Delete buttons with confirmation modals
  - Quick save / Quick load hotkeys
- **Notes:** When accessed from in-game, adds "Save Current" functionality

#### Mod Management
- **Purpose:** Enable, disable, and order mods
- **Controls:**
  - Mod list with: name, version, compatibility status, description
  - Enable/disable toggles per mod
  - Load order drag-and-drop
  - Enable/disable all buttons
  - Dependency conflict warnings
  - Achievement compatibility indicator

#### Help / Codex
- **Purpose:** Searchable encyclopedia of all game concepts, resources, buildings, mechanics
- **Controls:**
  - Search bar with auto-complete
  - Category tabs or tree navigation
  - Cross-linked entries (click resource name → resource detail)
  - Context-sensitive: can be opened from any game screen with the current entity pre-selected

#### Credits
- **Purpose:** Developer, publisher, and attribution information
- **Display:** Scrollable text

#### Pause / In-Game Menu (Modal)
- **Purpose:** In-session menu overlay
- **Trigger:** ESC key or menu button
- **Controls:**
  - Resume
  - Save Game → Save screen
  - Load Game → Load screen
  - Settings → Settings screen
  - Help / Codex → Codex
  - Exit to Main Menu (with confirmation)
  - Exit to Desktop (with confirmation)

### 6.2 Core Gameplay Views

#### Dashboard (Home)
- **Purpose:** High-level overview of the entire empire
- **Display:**
  - Empire summary: total population, total sites, total trade volume, net resource flow
  - Active alerts: critical issues across all sites (clickable → navigate to source)
  - Active projects: top research projects with progress bars, top construction jobs
  - Recent events: last N events from the log
  - Quick stats: graphs of key metrics over recent time (future enhancement)
- **Controls:**
  - Time controls (always present)
  - Click any alert/project/event to navigate to relevant detail view

#### Starmap (Stellar Cartography Database)
- **Purpose:** List and manage known star systems
- **Display:** Sortable/filterable table:

| Column | Description |
|---|---|
| System Name | Procedurally generated or player-renamed |
| Spectral Class | Star type (G2V, M3V, etc.) |
| Distance | Light-years from home system |
| Bodies | Number of known planets/moons |
| Status | Unknown / Probed / Explored / Colonized / Gate Connected |
| Gate Status | None / In Transit (ETA) / Active |
| Key Resources | Summary of notable resources |

- **Controls:**
  - Filters: by distance, class, status, resource type
  - Sort: by any column
  - Actions per row: View System Detail, Send Probe, Send Gate Ship
  - Search bar

#### Star System Detail
- **Purpose:** Overview of a single star system
- **Display:**
  - System properties: star type, luminosity, habitable zone range, age
  - Celestial bodies table:

| Body | Type | Orbit | Mass | Atmosphere | Gravity | Status | Resources | Hazards |
|---|---|---|---|---|---|---|---|---|
| Alpha I | Rocky | 0.7 AU | 0.8 E | Thin CO2 | 0.6g | Colonized | Iron, Water | Radiation |

  - Gate status (if applicable): gate location, connection, throughput
  - Active missions in system
  - Sites summary: count of settlements, installations, stations per body
- **Controls:**
  - Click body row → Body Detail
  - Send Probe to body
  - Launch Mission / Expedition
  - Rename system

#### Celestial Body Detail
- **Purpose:** Overview of a single planet, moon, asteroid, or other body
- **Display:**
  - Physical properties: mass, radius, gravity, day length, year length, axial tilt
  - Atmosphere: composition, pressure, temperature range, weather patterns
  - Biome summary (if applicable)
  - Resource deposits: table of deposit type, estimated quantity, accessibility
  - Hazards: radiation, seismic, volcanic, storms, etc.
  - Orbital assets: satellites, stations (table with status)
  - Surface sites: settlements and installations (table with population, status, key output)
  - Exploration status: percentage surveyed, notable anomalies
- **Controls:**
  - Click site row → Site Detail
  - Click orbital asset → Asset Detail
  - Found New Settlement / Installation
  - Deploy Satellite
  - Launch Surface Expedition
  - Manage Orbital Assets

#### Site Detail (Settlement / Installation)
- **Purpose:** Primary management view for a single site
- **Tabs:**

**Overview Tab:**
  - Site type (settlement / installation), location, founding date
  - Population: total, employed, unemployed, growth rate
  - Morale: composite score with breakdown (housing, food, safety, entertainment, etc.)
  - Power: generation, consumption, surplus/deficit
  - Life support: oxygen, water, temperature status
  - Storage: capacity used / total, per resource category
  - Key alerts for this site

**Buildings Tab:**
  - Building list table:

| Building | Type | Level | Status | Workers | Output | Efficiency | Actions |
|---|---|---|---|---|---|---|---|
| Mine #1 | Mining | 2 | Operating | 12/15 | 50 ore/hr | 87% | [Config] [Upgrade] [Mothball] |

  - Build new building (opens build menu with categories: Industrial, Population, Infrastructure, Military, Research)
  - Bulk actions: pause all, resume all, set automation policy
  - Filter by type, status

**Labor Tab:**
  - Job allocation table: role, assigned, needed, efficiency modifier
  - Unassigned colonist pool
  - Assignment controls: sliders or +/- to allocate workers between roles
  - Skill breakdown: workforce skill distribution

**Resources Tab:**
  - Stockpile table: resource, quantity, storage capacity, production/hr, consumption/hr, net/hr, trend
  - Resource flow visualization (future: Sankey diagram)
  - Import/export settings (if trade routes exist)

**Production Tab:**
  - Active production chains at this site
  - Queue: ordered list of production orders
  - Add production order (select recipe, quantity, priority)
  - Chain status: input availability, output destination, bottleneck alerts

**Construction Tab:**
  - Active construction projects: building name, progress %, time remaining, resources consumed/needed
  - Excavation projects: type (flatten, dig shaft, clear subsurface), progress, time remaining
  - Construction queue: ordered list, drag to reorder priority
  - Add to queue: build building, excavation task, infrastructure

**Infrastructure Tab:**
  - Connected infrastructure: roads, rails, pipelines, conveyors to/from this site
  - Infrastructure projects: build road to [site], build rail line, etc.
  - Transportation assets at this site: vehicles, rolling stock

**Automation Tab:**
  - Automation policy for this site (inherits from body/global unless overridden)
  - Toggle: use inherited policy / custom policy
  - Policy settings: auto-build, auto-assign labor, auto-trade, auto-ration, auto-repair
  - Threshold settings: minimum stockpile levels, maximum storage, alert triggers

- **Controls (all tabs):**
  - Breadcrumbs back to Body Detail and Colonies list
  - Navigation to: Governance, Research, Trade for this site's context

#### Building Detail (Modal or Page)
- **Purpose:** Configure a single building
- **Tabs:**
  - Status: operational state, health/condition, power draw, current output
  - Production: current recipe, input/output rates, efficiency, change recipe
  - Workers: assigned crew, skill levels, shifts
  - Upgrades: available upgrades with cost and effect
  - Maintenance: repair status, maintenance schedule, parts needed
  - History: log of events, production history, state changes
- **Controls:**
  - Start / Pause / Mothball / Demolish
  - Repair
  - Set automation (on/off, recipe auto-switch)
  - Upgrade

#### Vehicle / Ship Detail (Modal or Page)
- **Purpose:** Configure a single vehicle or spacecraft
- **Tabs:**
  - Status: location, condition, fuel, cargo
  - Orders: current task, order queue
  - Modules: installed modules (for spacecraft: mining, refinery, lab, etc.)
  - Crew: assigned operators, skills
  - Maintenance: wear, repair needs, parts
  - History: mission log, route history
- **Controls:**
  - Move to / Navigate to
  - Start / Pause / Abort orders
  - Automate (assign route template, mining loop, patrol)
  - Return to base / dock
  - Upgrade / Refit
  - Decommission

### 6.3 Fleet & Assets Views

#### Fleet Overview
- **Purpose:** List all ships, vehicles, and mobile assets
- **Display:** Sortable/filterable table:

| Name | Type | Location | Status | Cargo | Fuel | Condition | Orders |
|---|---|---|---|---|---|---|---|
| Rover Alpha | Hauler | Site Prime | In Transit (3h) | 80% | 60% | Good | Haul Route #2 |

- **Controls:**
  - Filter by type (surface vehicle, spacecraft, drone), location, status
  - Search
  - Click row → Vehicle/Ship Detail
  - Bulk actions: recall all to base, pause all, set automation

#### Satellite / Orbital Asset Overview
- **Purpose:** List all satellites, stations, and orbital infrastructure
- **Display:** Table with: name, type, body/orbit, status, function, condition
- **Controls:**
  - Click row → Asset Detail
  - Deploy new satellite
  - Decommission

### 6.4 Economy & Logistics Views

#### Colonies Overview
- **Purpose:** Master list of all sites across all bodies and systems
- **Display:** Hierarchical or flat table:

| System | Body | Site | Type | Population | Status | Key Output | Key Need |
|---|---|---|---|---|---|---|---|
| Home | Alpha I | Prime | Settlement | 1,200 | Growing | Metals | Food |

- **Controls:**
  - Group by: system, body, type
  - Filter / Search
  - Click row → Site Detail
  - Found New Settlement / Installation (links to body selection)

#### Trade & Logistics Overview
- **Purpose:** Manage inter-site and inter-system trade
- **Display:**
  - Routes table:

| Route ID | Origin | Destination | Cargo | Transport | Status | Volume/hr | Efficiency |
|---|---|---|---|---|---|---|---|
| R-001 | Prime | Mine Beta | Empty → Ore | Hauler #3 | Active | 20t/hr | 95% |

  - Supply/demand summary per site: surplus resources, deficit resources
  - Bottleneck alerts
- **Controls:**
  - Create route: select origin, destination, cargo type, assign transport
  - Edit / Delete route
  - Auto-route suggestion (system proposes routes to balance supply/demand)
  - Filter by system, body, resource type, status

#### Production Chain Viewer
- **Purpose:** Visualize and manage production chains
- **Display:**
  - Chain list: each chain from raw input to final product
  - Per chain: table of steps showing: input resource → building/process → output resource, rate, location
  - Bottleneck indicators (where throughput is constrained)
  - Template library for saved chain configurations
- **Controls:**
  - Create new chain
  - Edit chain steps
  - Assign buildings to chain steps
  - Save / Load template
  - Validate chain (check for missing links, insufficient capacity)
  - Export / Import chain definition (data file for modding/sharing)

### 6.5 Research & Governance Views

#### Research
- **Purpose:** Manage technology research and engineering projects
- **Display:**
  - Tech tree: expandable/collapsible list organized by category
    - Categories: Physics, Engineering, Biology, Computing, Materials, Social Sciences
    - Per node: name, prerequisites, cost (time, resources), effect description, status (locked / available / in progress / complete)
  - Active research: progress bars, assigned labs, estimated completion
  - Research queue: ordered list of next projects
  - Engineering projects: large one-time builds (orbital station, wormhole gate, megastructure) with milestone tracking
- **Controls:**
  - Start research / Add to queue
  - Assign / Reassign labs
  - Set research priority
  - Pause / Cancel research
  - View completed tech and their effects

#### Governance & Policy
- **Purpose:** Set empire-wide and site-level policies
- **Display:**
  - Policy categories with current settings:
    - Economy: taxation, trade regulations, currency policy
    - Labor: work hours, assignment priority, training policy
    - Rationing: resource allocation priorities during shortage
    - Security: alert levels, defense posture
    - Ethics: research ethics, environmental policy, AI autonomy limits
    - Immigration: population movement between sites
  - Policy effects summary: projected impact on morale, efficiency, growth
  - Cooldown timers for recently changed policies
- **Controls:**
  - Adjust policy sliders / toggles
  - Set scope: global, per-body, per-site
  - View policy history
  - Governance type selection (future: democracy, corporate, AI-led, etc.)

### 6.6 Analytics & Meta Views

#### Analytics / Statistics
- **Purpose:** Historical data visualization and performance tracking
- **Display:**
  - Metric selection: population, production by resource, trade volume, morale, power, etc.
  - Time range selector
  - Data table (and future: line charts, bar charts)
  - Aggregate statistics: averages, peaks, trends
- **Controls:**
  - Filter by site, body, system
  - Export data (CSV)
  - Compare metrics (overlay two data series)

#### Event Log (Full View)
- **Purpose:** Complete history of all events, decisions, and outcomes
- **Display:**
  - Chronological event list with: timestamp, type, severity, summary, location
  - Color coding by type (discovery, disaster, construction, research, trade, social, etc.)
  - Expandable detail per event (full description, choices made, outcomes)
- **Controls:**
  - Filter by type, severity, location, date range
  - Search by keyword
  - Pin events (bookmarks for important moments)
  - Generate report (export filtered log)

#### Achievements / Milestones
- **Purpose:** Track long-term goals and session progress
- **Display:**
  - Achievement list: name, description, progress, status (locked/in progress/complete)
  - Categories: economic, scientific, population, exploration, construction
  - Session stats: play time, turns elapsed, sites founded, etc.
- **Controls:**
  - Filter by category, status
  - View achievement detail (requirements, rewards if any)

### 6.7 Utility Views

#### Debug Console
- **Purpose:** Developer/power-user command interface
- **Trigger:** Tilde (~) key or equivalent binding
- **Display:**
  - Command prompt for entering debug/admin commands
  - Output log
  - Search and filter bar
- **Controls:**
  - Enable/disable from game settings
  - Command history (up/down arrows)
  - Auto-complete for known commands
- **Note:** May disable achievements when used

#### Event / Encounter Modal
- **Purpose:** Present narrative events and player choices
- **Display:**
  - Event title and description text
  - Relevant data readouts (affected site stats, risk probabilities)
  - Choice buttons with outcome hints (if investigation/intel allows)
  - Image/illustration placeholder (future)
- **Controls:**
  - Select choice → event resolves, outcome displayed
  - Dismiss (for informational events)
  - "Investigate" option (spend time/resources for more info before choosing)

#### Mission / Expedition Detail (Modal or Page)
- **Purpose:** Configure and monitor missions (probes, expeditions, gate deployments)
- **Display:**
  - Mission type, objective, destination
  - Duration estimate, current progress
  - Risk assessment
  - Cost (resources, personnel)
  - Log of mission events
- **Controls:**
  - Launch / Pause / Abort
  - Adjust parameters (crew, equipment)
  - Template management (save/load mission configs)

---

## 7. Game Systems & Mechanics

### 7.1 Colonization System

#### Site Foundation
- **Settlements:** Permanent population centers that grow like cities. Residents live there. Full building set available. Growth driven by immigration, birth rate, and quality of life.
- **Installations:** Purpose-built facilities with rotating crews (not permanent residents). Types: mining, refinery, military, research, relay. Focused on specific output.
- **Orbital Stations:** Space-based installations. Types: habitation, manufacturing, power generation, research, fuel depot, shipyard.

The player selects a body and founds a site. The simulation determines valid placement based on terrain, resources, and existing infrastructure. A body's size determines how many sites it can support (potentially tens to hundreds for large planets).

#### Construction
Buildings are added to a site's construction queue. The simulation handles physical placement. Construction requires:
- Available resources (consumed during build)
- Labor (construction workers)
- Time (based on building complexity and available workforce)
- Power (some construction requires active power supply)

Construction can be prioritized, paused, or cancelled. Partially completed buildings retain invested resources (minus waste).

#### Excavation
Surface and subsurface modifications to prepare for construction or access resources:
- **Surface flattening:** Prepare terrain for large structures
- **Subsurface shaft:** Dig access to underground resources or habitable volume
- **Subsurface clearing:** Excavate large underground spaces for protected construction
- **Tunneling:** Connect subsurface areas

Excavation is queued like construction and requires labor, equipment, and time.

#### Building Types

**Industrial Buildings:**
| Category | Examples | Notes |
|---|---|---|
| Power Generation | Solar array, nuclear reactor, RTG, fusion plant, geothermal | Some consume fuel, some produce waste |
| Mining / Extraction | Surface mine, deep mine, atmospheric processor, well | Extract raw resources from deposits |
| Refining / Processing | Smelter, refinery, chemical plant, water purifier | Convert raw → processed materials |
| Manufacturing | Fabricator, assembly plant, electronics factory, vehicle factory | Produce components and finished goods |
| Farming / Agriculture | Greenhouse, hydroponics bay, aquaculture tank, rangeland | Produce food and biological materials |
| Transportation | Cargo terminal, rail station, pipeline hub, spaceport | Interface between transport networks |
| Storage | Warehouse, tank farm, cold storage, fuel depot | Store resources with capacity limits |

**Population Buildings:**
| Category | Examples | Notes |
|---|---|---|
| Housing | Basic habitat, apartments, family housing, luxury housing | Quality affects morale |
| Retail / Services | Market, shops, bank, post office | Economic activity, morale |
| Food Service | Canteen, restaurant, food processing | Food distribution |
| Entertainment | Recreation center, park, theater, sports facility | Major morale factor |
| Education | School, university, training center, library | Skill development, research boost |
| Health | Clinic, hospital, pharmacy, mental health center | Health maintenance, injury treatment |
| Public Services | Administration, fire station, waste processing, security | Governance and safety |

**Infrastructure:**
| Category | Examples | Notes |
|---|---|---|
| Transportation | Road, rail line, pipeline, conveyor, tunnel, bridge | Connects sites and buildings |
| Communications | Comm tower, relay station, network hub | Required for inter-site coordination |
| Utilities | Power grid, water main, air distribution, waste pipeline | Distributes resources within a site |

Buildings can be:
- **Fixed:** Built on foundations, permanent position
- **Mobile (future):** Rail-mounted, trailer-based, ship-based, or floating — relocatable

All buildings support:
- Upgrades (improve capacity, efficiency, or unlock capabilities)
- Module additions (extend functionality)
- Recipe/operation configuration (select what to produce)
- Connection to transportation and utility networks
- Automation policy settings

#### Pollution & Environmental Impact
Industrial buildings can produce pollution. Pollution:
- Degrades local environment (affects agriculture, health, morale)
- Can be remediated with dedicated buildings (scrubbers, waste processors, reclamation)
- Policies can regulate pollution limits and penalties

### 7.2 Resource & Economy System

*See Section 8 for the full resource taxonomy.*

#### Resource Flow
Resources move through a pipeline:
1. **Extraction:** Raw materials harvested from deposits (mining, drilling, atmospheric collection, farming, fishing, forestry)
2. **Processing:** Raw materials refined into usable materials (smelting, chemical processing, purification)
3. **Manufacturing:** Materials combined into components and sub-assemblies (multiple cycles of increasing complexity)
4. **Assembly:** Components assembled into finished goods (vehicles, equipment, consumer goods, building materials)
5. **Distribution:** Finished goods transported to where they're needed (trade routes, logistics network)
6. **Consumption:** Resources consumed by colonists (food, goods, services), buildings (fuel, maintenance parts), and projects (construction materials)

#### Storage
Every site has storage with finite capacity. Storage is segmented by type (bulk solids, liquids, gases, manufactured goods, hazardous materials). When storage is full, production halts.

#### Trade & Logistics
- **Intra-site:** Automatic resource routing between buildings within a site (via utility networks)
- **Inter-site (same body):** Surface trade routes using vehicles (haulers, trains, pipeline flow)
- **Inter-body (same system):** Space trade routes using cargo ships
- **Inter-system:** Trade through wormhole gates (when connected)

Trade routes are configured by the player or auto-suggested by the simulation. Each route specifies: origin, destination, cargo type, transport method, priority.

#### Financial System
- Abstract currency representing economic value
- Revenue from trade, taxation, services
- Expenses for wages, maintenance, imports
- Budget tracking with surplus/deficit
- Future: post-scarcity economics, alternative currencies, market systems

### 7.3 Population & Morale System

#### Population Model
Colonists are modeled at two levels:
- **Aggregate:** Total population with demographic stats (age distribution, skill distribution, employment, growth rate)
- **Representative Characters:** A subset of named individuals with detailed character sheets for narrative events and key roles. These characters:
  - Have names, backgrounds, skills, traits, health, morale
  - Fill key positions (chief engineer, lead scientist, governor, etc.)
  - Feature in narrative events
  - Scale: dozens to hundreds of characters, even as aggregate population reaches millions/billions

This hybrid approach provides narrative richness without simulating billions of individual agents.

#### Colonist Needs
| Need | Source | Effect if Unmet |
|---|---|---|
| Food | Farms, imports | Starvation → health decline → death |
| Water | Purifiers, imports | Dehydration → health decline → death |
| Air / Oxygen | Life support systems | Suffocation → rapid death |
| Housing | Habitat buildings | Overcrowding → morale drop |
| Goods | Manufacturing, imports | Dissatisfaction → morale drop |
| Healthcare | Medical buildings | Disease spread, injury deaths |
| Entertainment | Recreation buildings | Boredom → morale drop, unrest |
| Education | Education buildings | Skill stagnation, reduced research |
| Data / Communications | Comms infrastructure | Isolation → morale drop |

#### Morale
Composite score calculated from satisfaction of needs, working conditions, governance, events, and environment. Morale affects:
- Productivity (high morale = bonus, low morale = penalty)
- Population growth (morale affects birth rate and immigration)
- Event triggers: celebrations (high), protests/strikes (low), mutiny (very low)
- Research speed and construction efficiency

#### Labor
Workers are assigned to roles/buildings. Assignment considers:
- Skill matching (engineer, scientist, laborer, farmer, medic, etc.)
- Fatigue and shift management
- Efficiency modifiers (skill level × morale × equipment quality)
- Automation can auto-assign workers based on priority settings

#### Population Growth
- Birth rate: influenced by morale, housing quality, healthcare, policy
- Death rate: influenced by healthcare, hazards, age, resource availability
- Immigration: colonists move between settlements based on quality of life differential
- Training: colonists gain experience in their roles, improve skill levels over time

### 7.4 Research & Progression System

#### Research Projects
- Conducted in research labs by assigned scientists
- Cost: time (measured in research points) and resources (lab supplies, materials)
- Research point generation: function of scientist count, skill, lab quality, morale, policy
- Breakthrough chance: rare bonus completion or bonus discovery

#### Tech Tree
Organized by category with dependencies:
- **Physics:** Sensors, propulsion, energy systems, wormhole theory
- **Engineering:** Construction techniques, vehicles, orbital structures, megaprojects
- **Materials Science:** Alloys, composites, superconductors, nanomaterials
- **Biology:** Agriculture, medicine, bioengineering, terraforming
- **Computing:** Automation, AI, communications, simulation
- **Social Sciences:** Governance, economics, psychology, culture

Each tech node:
- Has prerequisites (other tech nodes)
- Costs research points (escalating per tier) and possibly resources
- Unlocks: new buildings, recipes, efficiency upgrades, automation features, policies, capabilities
- Some nodes represent engineering projects: one-time large builds (wormhole gate, orbital elevator, megastructure)

#### Engineering Projects
Large-scale construction projects unlocked by research:
- Have milestone stages (design → prototype → construction → testing → operational)
- Require sustained resource investment over many game-time periods
- Examples: first wormhole gate, orbital shipyard, terraforming engine, system-wide defense network

### 7.5 Exploration System

#### Probes
Unmanned reconnaissance missions:
- Target: other bodies in-system or other star systems
- Cost: resources to build, fuel to launch
- Data quality: depends on probe type, mission duration, equipment
- Results: reveal body properties, resource deposits, hazards, anomalies
- Probe types (unlocked through research): fast flyby, orbital survey, lander, deep space

#### Expeditions
Manned or robotic surface/space missions:
- Crew: selected from colonist pool (skills matter)
- Equipment: vehicles, supplies, instruments
- Duration: hours to months depending on destination and objective
- Risks: equipment failure, environmental hazards, encounters
- Results: detailed survey data, resource samples, anomaly investigation, event triggers

#### Anomalies
Unusual discoveries on bodies or in space:
- Types: alien artifacts, geological oddities, energy signatures, ancient structures (lore-dependent)
- Investigation: requires expedition or probe, may trigger event chains
- Outcomes: technology bonuses, unique resources, narrative content, dangers

### 7.6 Wormhole / Gate System

#### Core Mechanics
- Wormhole gates are entangled pairs manufactured together
- One end remains at the origin; the other is transported via STL ship to the destination
- STL transit takes real game time (years/decades depending on distance)
- Once both ends are in position and powered, the gate activates
- Transport through an active gate (characteristics TBD — see Open Questions)

#### Progression
1. **Research:** Wormhole theory → gate engineering → gate miniaturization
2. **First Gate:** Massive project — potentially orbital construction, enormous resource and energy cost
3. **Subsequent Gates:** Reduced cost and size through research
4. **Gate Deployment:** Build gate pair → load far-end on ship → launch ship → wait → activate
5. **Network Growth:** Each new gate connection opens a system for full colonization and trade

#### Gate Infrastructure
- Gates require continuous power to remain open
- Surface gates: connected to planetary infrastructure (roads, rails to gate terminal)
- Orbital gates: connected to station infrastructure
- Trade routes can be configured through gates like any other transport link

### 7.7 Space Program

#### Satellites
Deployable from surface (via launch facility) or orbit:
- Types: communication, mapping, weather, resource survey, defense (future), relay
- Constellations: groups of satellites providing system-wide coverage
- Maintenance: degradation over time, replacement needed

#### Spacecraft
Built at shipyards (surface or orbital):
- Module-based design: select hull, add modules (cargo, mining, refinery, lab, military, habitation)
- Crew capacity (optional — some ships are autonomous)
- Fuel and life support requirements
- Fleet formation support

#### Missions
Structured space operations:
- Types: cargo run, exploration, gate deployment, military (future), rescue
- Planning: select ship, crew, equipment, destination, objectives
- Execution: managed by the simulation; events may occur en route
- Templates: save and reuse mission configurations

### 7.8 Governance & Policy System

#### Policies
Player-configurable rules that affect the entire empire or specific scopes:
- **Economy:** Tax rates, trade regulations, subsidies, price controls
- **Labor:** Work hours, mandatory rest, specialization requirements, child labor laws
- **Rationing:** Priority allocation during shortages (essential services first, military first, equal distribution, etc.)
- **Security:** Alert levels, curfews, surveillance, defense spending
- **Ethics:** Research restrictions, AI rights, environmental protection, genetic engineering
- **Immigration:** Open borders, skill-based selection, population caps per site

Policy effects:
- Immediate: morale impact, efficiency modifiers
- Long-term: population growth, research speed, economic output
- Cooldown: policies cannot be changed again for a configurable period after enactment

#### Governance Evolution (Future)
As the colony grows, governance models unlock:
- Autocracy (default starting government)
- Democracy, technocracy, corporate, AI-led, theocracy
- Each model has different policy options, morale effects, and efficiency profiles

### 7.9 Automation System

#### Policy-Based Automation
Every building, vehicle, site, and system supports automation policy:
- **Auto-build:** Automatically construct buildings when resources and labor are available, based on priority templates
- **Auto-assign:** Automatically assign workers to roles based on demand and skill
- **Auto-trade:** Automatically create/adjust trade routes to balance supply and demand
- **Auto-ration:** Automatically implement rationing when resources drop below thresholds
- **Auto-repair:** Automatically schedule maintenance and repairs
- **Auto-research:** Automatically select next research project based on priority queue

#### Automation Hierarchy
Policies cascade: Global → Body → Site → Building/Vehicle. Lower levels inherit from higher unless explicitly overridden.

#### Automation Scripting (Future)
Advanced automation via scripted rules:
- Trigger-condition-action format
- Visual flow builder (nodes and edges: if/then/repeat)
- Text editor with syntax highlighting (YAML format)
- Template library for sharing and reuse
- Export/import for modding and sharing

### 7.10 Terraforming System

Long-term planetary modification projects:
- **Atmosphere:** Add/remove gases, adjust pressure, temperature regulation
- **Hydrosphere:** Melt ice caps, redirect water, create bodies of water
- **Biosphere:** Introduce organisms, establish ecosystems, soil creation
- Requires massive sustained resource investment
- Effects emerge over long time scales
- Unlocked through research
- Changes affect site conditions (may reduce need for life support, enable open-air farming, etc.)

### 7.11 Event System

#### Event Structure
Each event has:
- **Trigger conditions:** Game state requirements (phase, time, location, resource levels, population, technology)
- **Probability:** Chance of occurrence when conditions are met (some events are guaranteed)
- **Description:** Narrative text explaining the situation
- **Data readouts:** Relevant stats and information
- **Choices:** Player options with:
  - Visible outcomes (known effects)
  - Hidden outcomes (probability-weighted effects revealed after choice)
  - Skill checks (character skills affect outcome probability)
- **Effects:** Modify game state (resources, morale, building status, population, research progress, etc.)
- **Prerequisites:** Ensure events only fire in appropriate contexts (e.g., generation ship events don't fire on sleeper ships)

#### Event Categories
- **Discovery:** New resource deposit, anomaly, archaeological find, scientific observation
- **Disaster:** Equipment failure, natural disaster (quake, storm, eruption), epidemic, fire
- **Social:** Cultural milestone, political movement, crime, celebration, conflict
- **Technical:** Breakthrough, malfunction, innovation, sabotage
- **External:** Anomalous signal, unknown object, first contact (future)
- **Economic:** Market shift, supply disruption, trade opportunity
- **Personal:** Character events (birth, death, achievement, conflict) for representative characters

#### Event Data Format
Events are defined in data files (YAML or similar) to support modding:
```yaml
event:
  id: colony_fire_01
  name: "Habitat Fire"
  category: disaster
  severity: critical
  auto_pause: true
  triggers:
    - condition: site.building_count >= 5
    - condition: site.fire_suppression_level < 2
  probability: 0.02  # per time unit when conditions met
  description: "A fire has broken out in {building.name} at {site.name}..."
  choices:
    - id: evacuate
      label: "Evacuate immediately"
      effects:
        - morale: -5
        - building.health: -50
        - population.casualties: 0
    - id: fight_fire
      label: "Organize firefighting crews"
      effects:
        - morale: -2
        - building.health: {roll: -20 to -80}
        - population.casualties: {roll: 0 to 5}
      skill_check:
        skill: engineering
        difficulty: 3
        success_bonus:
          building.health: +30
```

#### Event Display
- Events appear in the event log with color coding by category/severity
- Critical events trigger auto-pause (configurable)
- Choice events present a modal with description, data, and choice buttons
- Outcome is displayed after choice, with effects summarized
- Events link to relevant entities (click to navigate to affected building/site)

---

## 8. Resource System

### Design Philosophy
The resource system aims for deep simulation with moddable extensibility. Raw materials are diverse and geologically/biologically realistic. Processing chains are multi-step, with intermediate products and branching paths. The system should rival or exceed Factorio and Satisfactory in breadth, particularly on the raw materials side.

All resource definitions, recipes, and production chains are loaded from data files to support modding.

### 8.1 Resource Categories

#### Tier 0: Raw Materials (Extracted)

**Geological — Metallic Ores:**
- Iron ore, copper ore, aluminum ore (bauxite), titanium ore (ilmenite/rutile)
- Nickel ore, cobalt ore, chromium ore, manganese ore
- Tin ore (cassiterite), zinc ore, lead ore
- Tungsten ore (wolframite), molybdenum ore, vanadium ore
- Gold ore, silver ore, platinum group ores
- Lithium ore (spodumene), beryllium ore
- Rare earth ores (light: cerium, lanthanum, neodymium; heavy: yttrium, dysprosium)
- Uranium ore, thorium ore

**Geological — Non-Metallic Minerals:**
- Silica (quartz sand), feldspar, ite ite
- Calcium carbonate (limestone), calcium sulfate (gypite)
- Phosphate rock, potash (potassium salts)
- Sulfur, salt (halite)
- Clay (kaolin, bentonite), talc
- Graphite, diamond (industrial)
- Gemstones (future/decorative)

**Geological — Hydrocarbons & Carbon:**
- Coal, peat
- Crude oil, natural gas, methane hydrates
- Tar sands, oil shale
- Kerogen

**Atmospheric Gases:**
- Nitrogen, oxygen, carbon dioxide, argon
- Hydrogen, helium
- Water vapor
- Trace gases (neon, xenon, krypton — for specialized applications)
- Toxic gases (chlorine, sulfur dioxide, ammonia — hazards and industrial feedstock)

**Hydrospheric:**
- Fresh water, salt water, brine
- Ice (water ice, CO2 ice, methane ice, ammonia ice)
- Dissolved minerals in solution

**Biological — Agricultural:**
- Grain crops (wheat, rice, corn equivalents)
- Vegetable crops, fruit crops
- Legumes (nitrogen-fixing)
- Oilseed crops
- Fiber crops (cotton, hemp equivalents)
- Stimulant/luxury crops (coffee, tea, spice equivalents)

**Biological — Forestry:**
- Softwood timber, hardwood timber
- Tree resin, latex/rubber
- Bark (tannins, cork)
- Biome-specific wood variants

**Biological — Aquaculture/Fishery:**
- Fish (finfish), shellfish, crustaceans
- Algae, seaweed, kelp
- Plankton biomass

**Biological — Other:**
- Animal products (meat, dairy, eggs, wool, leather, bone)
- Microbial cultures (yeast, bacteria for industrial processes)
- Fungi (mushrooms, mycelium)
- Insect biomass, silk

**Energy Sources (Raw):**
- Solar irradiance (collected, not stored as material)
- Geothermal heat
- Wind energy
- Tidal/wave energy (if hydrosphere exists)
- Radioactive isotopes (for RTGs, reactors)

#### Tier 1: Processed Materials (Refined)

**Metals:**
- Iron/steel (various grades: mild, stainless, high-carbon, tool steel)
- Copper, bronze, brass
- Aluminum, titanium, nickel alloys
- Precious metals (gold, silver, platinum — refined)
- Rare earth metals (purified)
- Specialty alloys (superalloys, memory alloys, superconductors)
- Uranium fuel rods, thorium fuel

**Non-Metallic Materials:**
- Glass (various: silica glass, borosilicate, fiber optic grade)
- Cement, concrete
- Ceramics (structural, refractory, electronic)
- Graphite (processed), carbon fiber, carbon nanotubes (advanced)
- Silicon (metallurgical grade, semiconductor grade)
- Phosphoric acid, sulfuric acid, hydrochloric acid

**Chemicals:**
- Industrial chemicals (solvents, catalysts, reagents)
- Fertilizers (nitrogen, phosphorus, potassium-based)
- Plastics / polymers (polyethylene, polypropylene, nylon, polycarbonate)
- Rubber (natural/synthetic)
- Pharmaceuticals (base compounds)
- Explosives / propellants
- Paints, coatings, adhesives

**Biological Products:**
- Processed food (flour, sugar, oils, preserved food, prepared meals)
- Animal feed
- Biofuels (ethanol, biodiesel, biogas)
- Biomaterials (bioplastics, biocomposites)
- Textiles (woven fiber, synthetic fabric)
- Paper, cardboard
- Pharmaceuticals (derived)
- Fertilizer (organic)

**Energy (Converted):**
- Electrical energy (stored in batteries, capacitors)
- Hydrogen (electrolyzed — fuel and industrial)
- Heat (thermal energy for processes)
- Compressed gases

**Life Support Products:**
- Purified water, potable water
- Breathable air (mixed gases)
- Filtered/recycled waste water
- Climate-controlled atmosphere

#### Tier 2: Components (Manufactured)

- Structural beams, plates, fasteners
- Pipes, valves, fittings
- Wire, cable, conduit
- Electronic components (resistors, capacitors, chips, boards)
- Optical components (lenses, fiber, sensors)
- Mechanical components (gears, bearings, motors, actuators)
- Hydraulic/pneumatic components
- Thermal components (radiators, heat exchangers, insulation)
- Pressure vessels, tanks
- Seals, gaskets, filters
- Computer hardware (processors, memory, storage devices)
- Communication hardware (transceivers, antennas)
- Power cells, battery packs
- Solar panels (manufactured)
- Reactor components
- Construction materials (prefab panels, hab modules, airlocks)

#### Tier 3: Finished Goods (Assembled)

**Equipment:**
- Mining equipment, drilling rigs
- Construction equipment (cranes, excavators, bulldozers)
- Laboratory instruments
- Medical equipment
- Communication systems
- Power systems (generators, solar arrays, reactor assemblies)
- Life support units
- Environmental sensors

**Vehicles:**
- Surface rovers, haulers, excavators
- Trains, rail cars
- Drones (aerial, ground)
- Spacecraft (built from hull + modules)
- Submarines/boats (if hydrosphere)

**Consumer Goods:**
- Clothing, furniture, personal electronics
- Recreational equipment
- Luxury goods
- Cultural items (art, media — morale impact)

**Military (Future):**
- Weapons, armor, fortifications
- Military vehicles
- Defense platforms

**Megastructure Components:**
- Wormhole gate segments
- Orbital station modules
- Space elevator components
- Terraforming equipment

#### Special Resources

- **Waste:** Byproduct of production and consumption. Must be processed, stored, or disposed. Types: solid waste, liquid waste, gaseous waste, radioactive waste, biological waste, electronic waste
- **Data / Information:** Produced by research, exploration, communication. Not a physical resource but tracked as a flow
- **Cultural Output:** Produced by entertainment, education, governance. Affects morale and diplomatic standing (future)

### 8.2 Production Chain Structure

Each production step is a **recipe**:
```yaml
recipe:
  id: smelt_iron
  name: "Iron Smelting"
  building_type: smelter
  inputs:
    - resource: iron_ore
      quantity: 10
      per: hour
    - resource: coal
      quantity: 2
      per: hour
    - resource: electrical_energy
      quantity: 50
      per: hour
  outputs:
    - resource: pig_iron
      quantity: 8
      per: hour
    - resource: slag
      quantity: 2
      per: hour
      category: waste
  requirements:
    - tech: basic_metallurgy
  modifiers:
    - skill: metallurgy
      effect: output_quantity * 1.1
    - building_level: 2
      effect: input_quantity * 0.9
```

Recipes are chained: the output of one recipe is the input to the next. Complex products require multiple chains converging (e.g., a vehicle requires structural components + electronic components + mechanical components + power cells + software).

### 8.3 Resource Deposits

Bodies have procedurally generated resource deposits:
- **Type:** Which raw resource
- **Quantity:** Total extractable amount (depletable or renewable)
- **Richness:** Extraction rate modifier (rich deposits yield more per unit effort)
- **Accessibility:** Extraction difficulty modifier (surface vs. deep, terrain factors)
- **Discovery:** Some deposits hidden until surveyed/explored

Deposit generation is influenced by body type:
- Rocky planets: diverse geological resources, possible atmosphere
- Gas giants: atmospheric gases, rare materials in deep layers
- Ice bodies: water ice, frozen gases, subsurface oceans
- Asteroids/comets: concentrated ores, ice, rare metals
- Moons: varies by parent body

### 8.4 Modding the Resource System

All of the following are data-driven and moddable:
- Resource definitions (add new resources, modify properties)
- Recipe definitions (add new recipes, modify inputs/outputs)
- Building definitions (new building types that perform new recipes)
- Deposit generation rules (new deposit types, distribution rules)
- Tech tree modifications (new tech that unlocks new recipes/buildings)

Modding format: YAML or similar structured data files loaded at game start. Mod loading order determines override priority.

---

## 9. User Workflows

### 9.1 New Game → First Actions

1. Main Menu → New Game Configuration
2. Set player name, difficulty, starting conditions, mods
3. Click Start → Game generates system, body, landing site
4. Dashboard displays initial state: one site with starting buildings and colonists
5. Event log shows: "Colony established at {site name} on {body name}"
6. Player navigates to Site Detail to review starting resources and buildings
7. Player queues first construction projects and assigns labor
8. Player starts the time simulation (unpause)

### 9.2 Build a Building

1. Navigate to Site Detail → Construction Tab
2. Click "Build New"
3. Browse building categories (Industrial, Population, Infrastructure, etc.)
4. Select building type → see cost, prerequisites, time estimate, effects
5. Confirm → building added to construction queue
6. Monitor progress on Construction Tab or via event log notifications
7. On completion: event log entry, building appears in Buildings Tab

### 9.3 Establish a Production Chain

1. Navigate to Site Detail → Production Tab (or Production Chain Viewer)
2. Review available recipes (filtered by researched tech and available buildings)
3. Select a recipe to produce (e.g., "Smelt Iron Ore → Pig Iron")
4. Assign a building to run the recipe (e.g., Smelter #1)
5. Verify input availability (iron ore in storage, coal in storage, power available)
6. If inputs are unavailable: build extraction buildings, create trade routes, or adjust priorities
7. Start production → monitor output rate and efficiency
8. Chain recipes: set Fabricator to consume pig iron → produce steel plates

### 9.4 Found a New Settlement

1. Navigate to Celestial Body Detail (from Starmap → System Detail → Body)
2. Review body properties, resource deposits, existing sites
3. Click "Found New Settlement"
4. Configure: name, initial focus (mining, agriculture, general)
5. Select colonists to transfer from existing settlements
6. Select initial cargo/equipment to transfer
7. Confirm → settlement appears on body's site list
8. Build infrastructure connecting to existing sites (roads, rails)
9. Manage new settlement via Site Detail

### 9.5 Send a Probe to Another System

1. Navigate to Starmap
2. Find target system in the list (filter by distance, status = Unknown)
3. Click "Send Probe" action
4. Select probe type (affects data quality and travel time)
5. Review cost (resources consumed) and estimated arrival time
6. Confirm launch → probe appears in mission list
7. Event log notifies on launch
8. Time passes... event log notifies on arrival
9. System status updates to "Probed" with revealed data
10. Navigate to System Detail to review discovered information

### 9.6 Deploy a Wormhole Gate

1. Prerequisites: wormhole tech researched, gate pair constructed, gate ship built
2. Navigate to Starmap → identify target system (probed, desirable)
3. Select "Send Gate Ship" action
4. Configure: select gate pair, assign ship, select crew (if crewed)
5. Review: travel time, fuel cost, mission risk
6. Confirm launch → gate ship in transit
7. Event log tracks milestones (departure, mid-journey events, deceleration, arrival)
8. On arrival: event notification, option to activate gate
9. Activate → both gate endpoints become operational
10. Trade routes can now be configured through the gate
11. System status updates to "Gate Connected"

### 9.7 Respond to an Event

1. Event fires → event log entry appears (color-coded, timestamped)
2. If auto-pause event: simulation pauses, modal appears
3. Read event description and data readouts
4. Review choices (each with described or hinted outcomes)
5. Optionally: click "Investigate" to spend time/resources for more information
6. Select a choice → outcome resolves
7. Outcome displayed: effects on resources, morale, buildings, population
8. Event log records the event, choice, and outcome
9. Simulation resumes (if was auto-paused)

### 9.8 Configure Automation

1. Navigate to any level: Global (Governance), Body Detail, Site Detail, or Building Detail
2. Go to Automation Tab/Section
3. Review inherited policy (from parent level)
4. Choose: use inherited policy or set custom override
5. Configure settings: auto-build priorities, auto-assign rules, stockpile thresholds, etc.
6. Save → policy takes effect immediately
7. Monitor via event log (automation actions logged at info level)
8. Adjust as needed — lower-level overrides don't affect other sites

### 9.9 Manage Trade Between Sites

1. Navigate to Trade & Logistics Overview
2. Review supply/demand table: which sites have surplus, which have deficit
3. Click "Create Route"
4. Select origin site, destination site
5. Select cargo type(s) and quantity
6. Assign transport (vehicle/ship from fleet) or request auto-assignment
7. Review: travel time, fuel cost, throughput estimate
8. Confirm → route is active
9. Monitor route status, efficiency, and volume in the routes table
10. Auto-trade automation can suggest and create routes automatically if enabled

---

## 10. System Dependency Matrix

Systems produce state/data/resources consumed by other systems. This matrix defines the dataflow.

**Legend:**
- **D** = data/state, **R** = resources/throughput, **E** = events/triggers
- **P** = policy/control, **H** = human factors (morale, labor)
- **T** = tech unlocks, **L** = logistics/routes, **S** = save/telemetry

| Produces ↓ \ Consumes → | Colonization | Resource & Economy | Automation | Population & Morale | Research | Governance | Expansion | Exploration | Events | Analytics |
|---|---|---|---|---|---|---|---|---|---|---|
| **Colonization** | — | R,D | D | H,D | D | D | L,D | D | E | S |
| **Resource & Economy** | R | — | R,D | H | D | D | R,L | — | E | S |
| **Automation** | D | R,D | — | H | D | D | L | — | E | S |
| **Population & Morale** | H | H | H | — | H | H | H | — | E | S |
| **Research** | T | T | T | T | — | T | T | T | E | S |
| **Governance** | P | P | P | P | P | — | P | P | E | S |
| **Expansion** | L,D | R,L | L | H | D | P | — | D | E | S |
| **Exploration** | D | D | D | E | D | D | D | — | E | S |
| **Events** | E | E | E | E | E | E | E | E | — | S |
| **Analytics** | S | S | S | S | S | S | S | S | S | — |

---

## 11. Architecture

### System Components

| Component | Role | Technology |
|---|---|---|
| **Outpost-Core** | Business logic library; all simulation mechanics | Rust |
| **Outpost-Server** | Web server, REST API, template rendering, database | Rust (Actix-Web), Tera templates, SQLite |
| **Outpost-Client** | Programmatic API client | Rust static library + Python wrapper |
| **Outpost-Desktop** | Native desktop client (future) | TBD |

### Server Stack

- **Backend:** Actix-Web (Rust) serving REST API and rendered HTML
- **Database:** SQLite for game state persistence
- **Cache (possible):** Redis K/V store for session data, real-time state
- **Frontend:** HTMX + Alpine.js + TypeScript + additional JS/TS libraries
- **Templates:** Tera (Rust template engine) for server-side HTML rendering
- **Future:** WASM for client-side computation, plugin system

### Design Patterns

- **Event Sourcing / CQRS:** Game state changes are captured as events; current state is derived from event history. Commands (player actions) generate events; queries read projected state.
- **Data-Driven Content:** Resources, recipes, buildings, tech, events loaded from data files (YAML/JSON)
- **Entity Identification:** UUID-based entity IDs for all game objects
- **Real-Time Updates:** HTMX polling or server-sent events for UI updates during simulation tick

### Frontend Approach (Text-and-Tables)

- **No Canvas/WebGL:** Pure DOM elements — tables, lists, forms, text
- **CSS Grid/Flexbox:** For layout
- **CSS Variables:** For theming (dark mode, color-blind modes)
- **HTMX:** Partial page updates (click "Build" → replaces just the building list), polling for resource tickers
- **Alpine.js:** Lightweight client-side interactivity (dropdowns, toggles, modals)
- **Modals:** For quick actions, confirmations, event choices
- **Future visual additions:** Charts (Chart.js or similar), diagrams, maps added as needed — never required

---

## 12. Modding Support

### Moddable Content

| Content Type | Format | Location |
|---|---|---|
| Resources | YAML data files | `/data/resources/` |
| Recipes / Production Chains | YAML data files | `/data/recipes/` |
| Buildings | YAML data files | `/data/buildings/` |
| Tech Tree Nodes | YAML data files | `/data/tech/` |
| Events | YAML data files | `/data/events/` |
| Policies | YAML data files | `/data/policies/` |
| Vehicle / Ship Modules | YAML data files | `/data/vehicles/` |
| Localization | YAML/JSON files | `/data/locale/` |

### Mod Structure

```
/mods/
  my_mod/
    mod.yaml          # Mod metadata: name, version, author, dependencies, compatibility
    /data/
      resources/      # New or overridden resource definitions
      recipes/        # New or overridden recipes
      buildings/      # New or overridden buildings
      tech/           # New or overridden tech nodes
      events/         # New or overridden events
```

### Mod Loading

- Mods are detected in the mods folder at game launch
- Load order configurable by player (determines override priority)
- Dependency and compatibility checking at load time
- Mods can: add new content, override base content, or modify existing content
- Achievement compatibility flag: some mods disable achievements

---

## 13. Development Roadmap

### Milestone Overview

| Milestone | Scope | Description |
|---|---|---|
| **MVP** | Single site, basic loop | Playable colony: build, extract, refine, manufacture, survive |
| **Alpha** | Multi-site, full colony systems | Expansion, trade, research, governance, events |
| **Beta** | Multi-body, orbital, full system | Space program, system exploration, advanced economy |
| **Release** | Wormhole, multi-system, victory | Interstellar expansion, endgame, polish |
| **Post-Launch** | Expansions | Prequel (voyage), combat, diplomacy, advanced terraforming |

---

### MVP: Single Site Colony Management

**Goal:** One site on one body. Player can build, extract resources, refine, manufacture, assign labor, and sustain a colony. The core gameplay loop is playable and testable.

#### MVP.1: Foundation & Time System
- Project structure, build system, basic server with HTMX
- Game state model (system, body, site, building, resource, colonist)
- Time system: pause/play/speed controls
- Turn processing loop (real-time ticks)
- Basic UI shell: sidebar, top bar, content area, event log ticker
- Dashboard view (placeholder stats)

#### MVP.2: Site & Building System
- Site entity with building list, construction queue
- Building type definitions loaded from YAML (5-10 basic types: habitat, mine, smelter, fabricator, power plant, storage, greenhouse)
- Construction mechanic: queue building, consume resources, wait, complete
- Building states: under construction, operational, paused, damaged
- Site Detail view: Overview, Buildings, and Construction tabs
- Build menu with categories

#### MVP.3: Resource Extraction & Production
- Resource type definitions loaded from YAML (10-20 base resources for MVP)
- Resource deposits on bodies (procedurally generated)
- Extraction: mine building consumes labor/power, produces ore from deposit
- Refining: smelter consumes ore, produces metal
- Manufacturing: fabricator consumes metal, produces components
- Storage with capacity limits
- Site Detail: Resources tab with stockpile table, production/consumption rates

#### MVP.4: Population & Labor
- Colonist aggregate model: total population, employment, skill distribution
- A few representative characters (named, with skills and traits)
- Needs system: food, water, air, housing — basic satisfaction tracking
- Labor assignment: workers to buildings/roles
- Morale: basic composite score from needs satisfaction
- Productivity modifier from morale
- Site Detail: Labor tab

#### MVP.5: Power & Life Support
- Power generation buildings produce energy
- Power consumption by buildings
- Power grid: surplus/deficit calculation per site
- Brownout mechanic: buildings lose efficiency or shut down without power
- Life support: oxygen, water, temperature — basic tracking
- Failure cascade prevention with idle safety
- Power and life support status on Site Overview

#### MVP.6: Event System & Log
- Event engine: trigger conditions, probability, effects
- 10-20 starter events (equipment failure, discovery, weather, social)
- Event data loaded from YAML
- Event log: always-visible ticker, full-page view with filtering
- Auto-pause on critical events
- Event choice modal
- Color coding and severity levels

#### MVP.7: Save/Load & Settings
- Serialize game state to file (JSON)
- Save/load UI
- Quick save/load hotkeys
- Auto-save on interval
- Game settings: basic audio, display, gameplay, keybindings
- Idle safety toggle

#### MVP.8: Polish & Integration
- Breadcrumb navigation throughout
- Tooltips on key stats and values
- Consistent styling and color coding
- Tutorial hints / onboarding flow (basic)
- Codex entries for MVP content
- Balance pass on resource rates, construction times, event frequency
- Bug fixing and testing

---

### Alpha: Multi-Site & Full Colony Systems

#### Alpha.1: Planetary Expansion
- Found additional settlements and installations on the same body
- Per-site building lists, labor pools, resource stockpiles
- Body Detail view: list of all sites on a body
- Colonies Overview: master list of all sites

#### Alpha.2: Inter-Site Infrastructure & Trade
- Infrastructure types: roads, rails, pipelines (data-defined)
- Trade routes between sites on the same body
- Vehicle types: haulers, trains (basic)
- Trade & Logistics Overview view
- Supply/demand balancing

#### Alpha.3: Research & Tech Tree
- Research lab building
- Tech tree loaded from YAML (20-50 nodes for alpha)
- Research point generation, project queue
- Tech unlocks: new buildings, recipes, efficiency upgrades
- Research view with tree and progress tracking

#### Alpha.4: Governance & Policy
- Policy system with categories
- Policy effects on morale, efficiency, growth
- Policy cooldowns
- Governance view
- Scope cascading: global → body → site

#### Alpha.5: Automation Framework
- Automation policy settings at every level
- Auto-assign labor, auto-trade, auto-ration, auto-repair
- Policy inheritance and override system
- Automation tab on all relevant views

#### Alpha.6: Deep Resource Chain
- Expand resource definitions to full taxonomy (100+ resources)
- Multi-step production chains
- Production Chain Viewer
- Bottleneck detection and alerts
- Recipe configuration per building

#### Alpha.7: Excavation & Terrain
- Excavation tasks: flatten, dig shaft, clear subsurface, tunnel
- Construction queue integration
- Terrain modifiers affecting building efficiency and construction cost

#### Alpha.8: Advanced Events
- Expand event library (50+ events)
- Character-driven events using representative characters
- Event chains (multi-stage events with consequences)
- Skill checks affecting outcomes
- Morale-triggered events (celebrations, strikes, mutiny)

---

### Beta: System Expansion & Space

#### Beta.1: Orbital Operations
- Satellite types: comm, mapping, weather, survey
- Satellite deployment and constellation management
- Orbital station construction
- Orbital asset views

#### Beta.2: System Exploration
- Probe system: build, launch, travel time, data return
- Expeditions: manned/robotic surface missions
- Anomaly generation and investigation
- Starmap and System Detail views

#### Beta.3: Multi-Body Colonization
- Found colonies on other bodies in-system
- Inter-body trade (cargo ships)
- Space vehicle construction and fleet management
- Fleet Overview view

#### Beta.4: Space Program
- Shipyard building (surface and orbital)
- Ship design: hull + modules
- Mission planning and execution
- Mission Detail view

#### Beta.5: Megaprojects
- Engineering project system (milestone-based)
- Mega-infrastructure: bridges, dams, canals, orbital elevators
- Terraforming: initial atmosphere and biosphere modification

#### Beta.6: Advanced Economy
- Financial system: currency, taxation, budgets
- Market dynamics (supply/demand pricing)
- Economic policies and trade regulations
- Economic analytics

#### Beta.7: Analytics & Achievements
- Statistics tracking over time
- Analytics view with data tables (future: charts)
- Achievement definitions and tracking
- Export data to CSV

---

### Release: Wormhole & Multi-System

#### Release.1: Wormhole Technology
- Wormhole research chain (theory → engineering → miniaturization)
- Gate construction (massive engineering project)
- Gate pair manufacturing

#### Release.2: Gate Deployment
- Gate ship construction
- STL transit mechanics (long-duration mission)
- Gate activation upon arrival
- Starmap updates for gate status

#### Release.3: Interstellar Network
- Multi-system management
- Trade through gates
- Cross-system logistics
- System-level policy and automation

#### Release.4: Victory Conditions
- Economic victory tracking
- Scientific victory tracking
- Population victory tracking
- Victory screen with stats
- Continue-playing mode after victory

#### Release.5: Polish & Balance
- Full UI polish pass
- Comprehensive tutorial/onboarding
- Complete codex entries
- Balance tuning across all systems
- Performance optimization
- Mod validation and documentation

---

### Post-Launch Expansions

#### Expansion A: Prequel — Voyage to the Stars
- Star system selection phase
- Ship configuration (generation ship / sleeper ship)
- Voyage simulation with narrative events
- Starting conditions derived from voyage outcomes

#### Expansion B: Combat & Defense
- Military buildings and units
- Defensive structures
- Space combat (text-based battle reports)
- Threat events (pirates, rival factions, natural threats)

#### Expansion C: Diplomacy & Factions
- AI-controlled rival colonies
- Trade negotiations
- Territorial disputes
- Alliance and conflict systems

#### Expansion D: Advanced Terraforming
- Full planetary transformation
- Ecosystem simulation
- Alien biosphere interactions
- Climate modeling

#### Expansion E: Automation Scripting
- Visual flow builder for automation rules
- YAML script editor with syntax highlighting
- Template marketplace for sharing
- Conditional logic and complex workflows

---

## 14. Open Questions & Future Work

### Open Design Questions

1. **Wormhole Transport Characteristics:** Is transport through an active gate instant? Bandwidth-limited (tonnage per hour)? Does it cost power per transit? Can people walk through or only cargo?

2. **Victory Threshold Configurability:** Are victory thresholds set at game start (tied to difficulty)? Can they be modified mid-game? Are there multiple difficulty presets?

3. **Representative Character Scaling:** As population grows from hundreds to billions, how many representative characters should exist? Is there a cap? Do they age and die, requiring replacement?

4. **Idle Safety Granularity:** How granular are idle safety protections? Does it prevent all negative outcomes or just catastrophic ones? Can the player configure what's protected?

5. **Wormhole Size Progression:** How large is the first gate? Ship-sized? Building-sized? Does miniaturization through research reach personal/vehicle scale (like Hamilton's novels)?

6. **Inter-System Communication:** Before a gate is active, can systems communicate? Laser comms (light-speed delay)? Or total isolation until gate activation?

7. **Procedural Generation Parameters:** How much control does the player have over world generation? Seed input? Bias toward habitable vs. challenging environments?

8. **Mobile Buildings:** How do mobile buildings (trailer-based, ship-based, rail-mounted) differ from fixed buildings mechanically? Are they a priority or future feature?

9. **Pollution Model Depth:** Is pollution a simple modifier or a full environmental simulation with diffusion, accumulation, and ecosystem impact?

10. **Currency and Post-Scarcity:** When does the economy transition from scarcity-based to post-scarcity? Is this a tech unlock, a policy choice, or emergent?

### Technical Open Questions

1. **Simulation Tick Rate:** What is the minimum tick interval? Does it vary with game speed?
2. **Save Format:** JSON vs. binary vs. SQLite snapshot? Versioning for save compatibility across updates?
3. **Maximum Scale:** What is the upper bound on simultaneous sites, buildings, trade routes? Performance budgets?
4. **Mod Sandboxing:** How are mods validated and sandboxed to prevent game-breaking or malicious content?
5. **Multiplayer (Future):** Is multiplayer a consideration? Shared galaxy? Separate civilizations in same galaxy?

### Content Backlog

- Full tech tree design (200+ nodes)
- Full event library (200+ events)
- Full building catalog (100+ types)
- Full recipe database (500+ recipes)
- Achievement list
- Codex content
- Tutorial script
- Balance spreadsheets for resource rates, costs, timings
