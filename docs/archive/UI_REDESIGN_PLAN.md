# Outpost 3: UI Redesign Plan

**Created**: 2026-01-01
**Status**: Planning
**Phase**: 3.5 (between Phase 3 and Phase 4)

---

## Executive Summary

This document outlines the plan to redesign the Outpost 3 UI from its current functional-but-basic state into a game-quality interface inspired by Outpost (1994), Europa Universalis IV/V, and SimCity. The redesign will introduce:

1. **Data visualization** - Charts for population, power, resources, and finances
2. **Hex-based maps** - 2D sprite/tile graphics with top-down or isometric perspective
3. **Modal dialogs** - Configuration panels, action dialogs, and building detail views
4. **Game-like UX** - Sidebar navigation, tooltips, keyboard shortcuts, contextual menus

---

## Technology Decision: Web App with Canvas/JavaScript

### Options Evaluated

| Approach | Description | Pros | Cons |
|----------|-------------|------|------|
| **A: Enhanced Web App** | Keep Rust backend + add JavaScript/Canvas | Incremental migration, familiar tech, works everywhere | Two codebases (Rust + JS) |
| **B: WASM Desktop App** | Full rewrite with Bevy/macroquad | Single codebase, better perf, native feel | Major rewrite, steeper learning curve |
| **C: Hybrid** | Rust backend + TypeScript SPA | Best tooling, separation of concerns | API boundary, two languages |

### Recommendation: Option A (Enhanced Web App)

**Rationale:**
1. **Incremental Migration** - Build on existing HTMX infrastructure, add JS progressively
2. **Lower Risk** - No major architectural changes required
3. **Future Path** - ROADMAP Phase 8 already plans WASM migration; this is a stepping stone
4. **Time to Value** - Faster to implement, can release improvements iteratively

**Technology Stack for UI Redesign:**

| Component | Library | Purpose |
|-----------|---------|---------|
| **2D Rendering** | PixiJS 8.x | Hex grid, sprites, tiles, map rendering |
| **Charts** | Chart.js 4.x | Line/bar/pie charts for data visualization |
| **State Management** | Alpine.js 3.x | Lightweight reactivity without full SPA |
| **Modals/UI** | Keep HTMX + add custom modal system | Dialogs, configuration panels |
| **Build Tooling** | Vite | Bundle JS/CSS, HMR during development |

---

## Design Principles

### 1. Reference Games UI Analysis

**Outpost (1994)**
- Sidebar-based navigation
- Top-down colony view with building placement grid
- Status bars for life support, power, morale
- Functional, information-dense panels

**Europa Universalis IV/V**
- Map-centric design (90% of screen)
- Collapsible side panels
- Outliner (quick-access list of units/provinces)
- Tooltips with rich information
- Date/speed controls always visible
- Mini-map for navigation

**SimCity (2013/Classic)**
- Isometric grid view
- Data layers (power, water, crime, etc.)
- Advisor notifications
- Zoning/building radial menus
- Budget graphs and charts

### 2. Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Grid Type** | Hexagonal | Better for network connections (wormholes, rail lines) |
| **Perspective** | Top-down (with optional isometric) | Simpler to implement initially |
| **Resolution** | Responsive (1024px minimum) | Support desktop and tablets |
| **Theme** | Dark mode (current) | Fits space theme, reduces eye strain |
| **Layout** | Map-centric with collapsible panels | Maximizes strategic view |

---

## UI Architecture

### Screen Layout

```
┌────────────────────────────────────────────────────────────────────┐
│  HEADER: Logo | Colony Name | Turn | Date | Resources Bar         │
├──────┬─────────────────────────────────────────────────────────────┤
│      │                                                             │
│  S   │                                                             │
│  I   │                    MAIN CANVAS AREA                        │
│  D   │                  (Hex Grid / Charts)                        │
│  E   │                                                             │
│  B   │                                                             │
│  A   │                                                             │
│  R   │                                                             │
│      │                                                             │
│      ├─────────────────────────────────────────────────────────────┤
│      │  BOTTOM BAR: Notifications | Mini-map | Speed Controls     │
└──────┴─────────────────────────────────────────────────────────────┘
```

### Component Hierarchy

```
App
├── Header
│   ├── Logo
│   ├── ColonySelector
│   ├── TurnCounter
│   ├── ResourceBar (compact view)
│   └── MenuButton
├── Sidebar (collapsible)
│   ├── Navigation
│   │   ├── ColonyView
│   │   ├── MapView
│   │   ├── TrainsView
│   │   ├── EconomyView
│   │   └── ResearchView
│   ├── Outliner
│   │   ├── BuildingsList
│   │   ├── TrainsList
│   │   └── AlertsList
│   └── QuickActions
├── MainCanvas
│   ├── HexMapRenderer (PixiJS)
│   │   ├── TerrainLayer
│   │   ├── BuildingsLayer
│   │   ├── ResourcesLayer
│   │   ├── NetworkLayer (rails, wormholes)
│   │   └── SelectionOverlay
│   └── ChartRenderer (Chart.js)
│       ├── PopulationChart
│       ├── PowerChart
│       ├── ResourcesChart
│       └── FinancialChart
├── BottomBar
│   ├── NotificationLog
│   ├── MiniMap
│   └── SpeedControls
└── ModalSystem
    ├── BuildingDetailModal
    ├── ConstructionModal
    ├── ConfigurationModal
    └── ConfirmationDialog
```

---

## Remaining Phase 3 Items

Before starting the UI redesign, complete these Phase 3 items:

### 3.1 Building Upgrade System UI
- [ ] Add upgrade button to building cards
- [ ] Show upgrade costs and benefits
- [ ] Display upgrade progress indicator

### 3.2 Building Detail Modal
- [ ] Create modal component structure
- [ ] Show building stats (workers, power, production)
- [ ] Display input/output resources
- [ ] Add upgrade/repair/shutdown actions
- [ ] Show efficiency and status

### 3.3 Power Status per Building
- [ ] Add power consumption indicator to building cards
- [ ] Show power deficit warnings
- [ ] Color-code buildings by power status

### 3.4 Production Output Indicators
- [ ] Add production rate display to buildings
- [ ] Show resource flow arrows
- [ ] Indicate bottlenecks and shortages

---

## Phase 3.5: UI Redesign Implementation

### Stage 1: Build System & Infrastructure (Week 1)

**Goals:** Set up JavaScript tooling, create component structure

#### 1.1 Development Environment
- [ ] Install Node.js dependencies (package.json)
- [ ] Configure Vite for JS/CSS bundling
- [ ] Set up hot module replacement (HMR)
- [ ] Create TypeScript configuration (optional, can use JS)
- [ ] Add source maps for debugging

#### 1.2 File Structure
```
static/
├── js/
│   ├── main.js           # Entry point
│   ├── components/
│   │   ├── sidebar.js
│   │   ├── header.js
│   │   └── modal.js
│   ├── canvas/
│   │   ├── hex-grid.js
│   │   ├── map-renderer.js
│   │   └── sprites.js
│   ├── charts/
│   │   ├── population.js
│   │   ├── power.js
│   │   └── resources.js
│   └── utils/
│       ├── api.js
│       └── state.js
├── css/
│   ├── main.css          # Existing
│   ├── layout.css        # New layout system
│   ├── sidebar.css
│   ├── modal.css
│   └── charts.css
└── assets/
    ├── sprites/          # Hex tiles, buildings
    ├── icons/            # UI icons
    └── fonts/
```

#### 1.3 Base Layout Refactor
- [ ] Update base.html with new layout structure
- [ ] Add sidebar toggle mechanism
- [ ] Create responsive breakpoints
- [ ] Implement CSS Grid for main layout

---

### Stage 2: Sidebar & Navigation (Week 2)

**Goals:** Implement collapsible sidebar with game-like navigation

#### 2.1 Sidebar Structure
- [ ] Create sidebar HTML/CSS framework
- [ ] Implement collapse/expand animation
- [ ] Add navigation icons (Colony, Map, Trains, Economy)
- [ ] Create active state indicators
- [ ] Save sidebar state in localStorage

#### 2.2 Outliner Panel
- [ ] Buildings quick-list with status icons
- [ ] Train fleet summary (when implemented)
- [ ] Alert/notification badges
- [ ] Click-to-focus behavior

#### 2.3 Quick Actions
- [ ] Advance Turn button (prominent)
- [ ] Common build shortcuts
- [ ] Context-sensitive actions

---

### Stage 3: Modal & Dialog System (Week 3)

**Goals:** Create reusable modal framework for all dialogs

#### 3.1 Modal Framework
- [ ] Create modal container component
- [ ] Implement open/close animations
- [ ] Add overlay with click-to-close
- [ ] Keyboard navigation (Escape to close)
- [ ] Stack multiple modals if needed

#### 3.2 Building Detail Modal
- [ ] Full building stats display
- [ ] Production chain visualization
- [ ] Worker allocation controls
- [ ] Action buttons (upgrade, repair, shutdown)
- [ ] Historical production graph

#### 3.3 Construction Modal
- [ ] Building type grid with icons
- [ ] Cost breakdown display
- [ ] Requirements check (resources, power, workers)
- [ ] Build confirmation with placement preview

#### 3.4 Configuration Panels
- [ ] Labor allocation dialog
- [ ] Market pricing settings (Phase 6)
- [ ] Route configuration (Phase 5)

---

### Stage 4: Data Visualization (Week 4)

**Goals:** Add Chart.js graphs for key metrics

#### 4.1 Chart Infrastructure
- [ ] Install and configure Chart.js
- [ ] Create chart wrapper component
- [ ] Set up dark theme styling
- [ ] Add responsive sizing

#### 4.2 Population Chart
- [ ] Line chart: Population over turns
- [ ] Stacked areas: Employed vs Unemployed
- [ ] Morale trend line
- [ ] Housing capacity comparison

#### 4.3 Power Grid Chart
- [ ] Bar chart: Generation by source
- [ ] Line overlay: Consumption trend
- [ ] Net power indicator
- [ ] Brownout event markers

#### 4.4 Resource Charts
- [ ] Multi-line: Stock levels over time
- [ ] Stacked bar: Production vs Consumption
- [ ] Trade balance (when economy implemented)
- [ ] Resource flow Sankey diagram (stretch goal)

#### 4.5 Financial Dashboard (Phase 6 prep)
- [ ] Income/Expense breakdown pie chart
- [ ] Profit trend line chart
- [ ] Budget projections
- [ ] Trade revenue by route

---

### Stage 5: Hex Map Foundation (Week 5-6)

**Goals:** Create the core hex grid rendering with PixiJS

#### 5.1 PixiJS Setup
- [ ] Install and configure PixiJS
- [ ] Create canvas container in layout
- [ ] Set up render loop
- [ ] Configure resolution and scaling

#### 5.2 Hex Grid System
- [ ] Define hex coordinate system (axial coordinates)
- [ ] Create hex tile rendering function
- [ ] Implement camera pan and zoom
- [ ] Add grid overlay toggle
- [ ] Create coordinate display on hover

#### 5.3 Terrain Rendering
- [ ] Define terrain types (plains, mountains, water, resources)
- [ ] Create terrain tile sprites (placeholder art initially)
- [ ] Implement terrain layer rendering
- [ ] Add terrain-based building placement rules

#### 5.4 Building Rendering
- [ ] Create building sprite sheet
- [ ] Render buildings on hex tiles
- [ ] Add status indicators (operational, damaged, construction)
- [ ] Implement building selection highlight
- [ ] Show worker count badges

---

### Stage 6: Map Interactions (Week 7)

**Goals:** Make the map interactive and informative

#### 6.1 Selection & Hover
- [ ] Click to select hex/building
- [ ] Hover tooltip with tile info
- [ ] Multi-select for groups (Shift+click)
- [ ] Keyboard navigation (arrow keys)

#### 6.2 Building Placement
- [ ] Enter build mode from construction modal
- [ ] Show valid placement hexes (green overlay)
- [ ] Preview building on hover
- [ ] Confirm placement with click
- [ ] Cancel with Escape or right-click

#### 6.3 Data Layers (Toggle Views)
- [ ] Power layer: Color hexes by power status
- [ ] Resource layer: Show extractable resources
- [ ] Production layer: Color by output volume
- [ ] Pollution layer: Show contamination spread

#### 6.4 Network Visualization
- [ ] Rail line rendering between stations
- [ ] Wormhole gate connections (dashed lines initially)
- [ ] Train position indicators (Phase 5 prep)
- [ ] Animate moving trains on routes

---

### Stage 7: Asset Creation (Parallel Track)

**Goals:** Create 2D sprite assets for the game

#### 7.1 Hex Tiles (32x32 or 64x64 base)
- [ ] Terrain types: Empty, Plains, Hills, Mountains, Water
- [ ] Resource deposits: Iron, Copper, Rare Metals, Coal, Uranium, etc.
- [ ] Overlays: Selection, Valid placement, Invalid, Fog of war

#### 7.2 Building Sprites
- [ ] Mine (with ore cart)
- [ ] Power Plant (coal, with smoke)
- [ ] Solar Power Plant (panels)
- [ ] Nuclear Power Plant (cooling towers)
- [ ] Housing (residential blocks)
- [ ] Farm (crop fields)
- [ ] Factory (industrial building)
- [ ] Refinery (pipes and tanks)
- [ ] Warehouse (storage containers)
- [ ] Research Facility (satellite dish)
- [ ] Medical Facility (cross symbol)
- [ ] Commercial Zone (shops)
- [ ] Train Station (platform)
- [ ] Wormhole Gate (ring structure)

#### 7.3 UI Assets
- [ ] Resource icons (28 types)
- [ ] Navigation icons (sidebar)
- [ ] Status icons (operational, damaged, constructing, shutdown)
- [ ] Button styles and states
- [ ] Modal decorations

#### 7.4 Art Style Guide
- **Style**: Clean pixel art or stylized low-poly 2D
- **Palette**: Dark blues, purples (space theme), with accent colors for resources
- **Size**: 64x64 base tiles, scalable
- **Animation**: Simple 2-4 frame loops for operational buildings

---

### Stage 8: Polish & Integration (Week 8)

**Goals:** Refine UX, add transitions, finalize for release

#### 8.1 Transitions & Animations
- [ ] Page transition effects
- [ ] Modal slide-in animations
- [ ] Chart data update animations
- [ ] Map zoom/pan smoothing
- [ ] Loading indicators for async operations

#### 8.2 Keyboard Shortcuts
- [ ] Space: Advance turn
- [ ] B: Open build menu
- [ ] Escape: Close modal / Cancel action
- [ ] 1-5: Switch view modes
- [ ] +/-: Zoom map
- [ ] Arrow keys: Pan map

#### 8.3 Notifications & Alerts
- [ ] Toast notification system
- [ ] Event log panel (bottom bar)
- [ ] Alert badges on sidebar items
- [ ] Sound effects (optional, muted by default)

#### 8.4 Performance Optimization
- [ ] Lazy load chart libraries
- [ ] Sprite sheet optimization
- [ ] Viewport culling for large maps
- [ ] Debounce map interactions
- [ ] Cache rendered elements

#### 8.5 Accessibility
- [ ] Keyboard navigation for all UI
- [ ] Screen reader labels
- [ ] High contrast mode option
- [ ] Reduced motion preference respect

---

## Backend API Requirements

The UI redesign will require these backend changes:

### New Endpoints
```
GET  /api/colony/{id}/history          # Historical data for charts
GET  /api/colony/{id}/map              # Hex map data
GET  /api/colony/{id}/building/{id}    # Building detail
POST /api/colony/{id}/building/place   # Place building at hex coords
GET  /api/colony/{id}/layers/{layer}   # Data layer info
```

### Data Format Changes
- Add turn history tracking for population, resources, power
- Include hex coordinates for buildings
- Add map generation for colonies (terrain, resources)

---

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| PixiJS learning curve | Medium | Medium | Start with simple tiles, iterate |
| Performance with large maps | High | Low | Implement viewport culling early |
| Asset creation time | Medium | High | Use placeholder art, iterate later |
| Scope creep | High | Medium | Strict phase boundaries, MVP focus |
| Browser compatibility | Medium | Low | Target evergreen browsers only |

---

## Success Criteria

### Minimum Viable Product (MVP)
- [ ] Functional sidebar with navigation
- [ ] Building detail modal working
- [ ] At least 2 chart types (population, power)
- [ ] Basic hex map displaying buildings
- [ ] Click-to-select on map

### Full Release
- [ ] All 4 chart categories implemented
- [ ] Complete hex map with all layers
- [ ] Building placement workflow complete
- [ ] All building sprites created
- [ ] Keyboard shortcuts working
- [ ] Notification system active

---

## Appendix: Hex Grid Coordinate System

Using **axial coordinates** (q, r) for hex grid:

```
     ___     ___     ___
    /0,0\   /1,0\   /2,0\
    \___/   \___/   \___/
   /0,1\   /1,1\   /2,1\
   \___/   \___/   \___/
  /0,2\   /1,2\   /2,2\
  \___/   \___/   \___/
```

**Key formulas:**
- Cube to axial: `q = x`, `r = z`
- Distance: `max(abs(q1-q2), abs(r1-r2), abs(-q1-r1+q2+r2))`
- Neighbors: 6 directions from each hex

**Reference:** [Red Blob Games - Hexagonal Grids](https://www.redblobgames.com/grids/hexagons/)

---

## Appendix: Sprite Sheet Layout

```
sprite-sheet.png (1024x1024)
┌─────────────────────────────────────────────┐
│ Terrain (row 0-1)                           │
│ [empty][plains][hills][mtn][water][res1]... │
├─────────────────────────────────────────────┤
│ Buildings (row 2-5)                         │
│ [mine][power][solar][nuke][house][farm]...  │
├─────────────────────────────────────────────┤
│ Overlays (row 6)                            │
│ [select][valid][invalid][fog]               │
├─────────────────────────────────────────────┤
│ Icons (row 7-8)                             │
│ [resource icons][status icons][UI icons]    │
└─────────────────────────────────────────────┘
```

---

## Timeline Summary

| Stage | Description | Duration |
|-------|-------------|----------|
| Prep | Complete Phase 3 remaining items | - |
| 1 | Build system & infrastructure | - |
| 2 | Sidebar & navigation | - |
| 3 | Modal & dialog system | - |
| 4 | Data visualization (charts) | - |
| 5-6 | Hex map foundation | - |
| 7 | Asset creation (parallel) | - |
| 8 | Polish & integration | - |

**Note:** Stages can overlap. Asset creation runs parallel to development.

---

## Next Steps

1. [ ] **Approve this plan** - Review with stakeholders
2. [ ] **Complete Phase 3 items** - Building detail modal, power indicators
3. [ ] **Set up build tooling** - Vite, npm dependencies
4. [ ] **Start Stage 1** - Layout refactor and component structure
5. [ ] **Begin asset creation** - Placeholder sprites for buildings

---

*Document maintained by the development team. Update as implementation progresses.*
