# Outpost 3: UI Overhaul - Text-Based & Simulation Focused

**Status**: Proposal
**Inspiration**: Melvor Idle, A Dark Room, OGame, Text-Based MMORPGs
**Philosophy**: "Spreadsheet in Space" - Focus on the numbers, rates, and detailed simulation mechanics rather than spatial visualization.

---

## 1. Core Vision

We are pivoting from a map-based graphical interface (hex grids, 2D scatter plots) to a **data-dense, menu-driven interface**.
The simulation remains complex (positions, physics, orbits still exist in the backend), but the player interacts with them through **abstractions** (tables, lists, control panels).

### Key Changes
- **No Galaxy Map Visual**: Replaced by a sortable/filterable "Star Systems" list.
- **No Colony Grid**: Buildings are abstract slots or a list of constructed facilities. Position on planet is irrelevant for UI.
- **No Unit Movement**: Ships/Trains have status bars (e.g., "In Transit: 4 turns remaining") instead of moving sprites.

---

## 2. Layout & UX

The UI will mimic a modern web dashboard or complex idle game.

**Layout Structure:**
```
+----------------+---------------------------------------------------+
|  SIDEBAR       |  TOP BAR (Global Resources / Alerts / Time)       |
|                +---------------------------------------------------+
|  - Dashboard   |                                                   |
|  - Starmap     |  MAIN CONTENT AREA                                |
|  - Colony      |                                                   |
|  - Logistics   |  (Dynamic HTMX content)                           |
|  - Research    |                                                   |
|  - Fleet       |                                                   |
|  - Settings    |                                                   |
|                |                                                   |
+----------------+---------------------------------------------------+
```

- **Theme**: Dark mode, high contrast constraints, monospace numbers.
- **Interactivity**:
    - **Hover**: Tooltips for detailed stats (equations, modifiers).
    - **Click**: instant or modal actions.
    - **Real-time**: HTMX polling for resource ticking (or simulated turns).

---

## 3. Screen Designs

### 3.1 Dashboard (Home)
**Purpose**: High-level overview of the empire.
- **Empire Summary**: Total population, total credit income, net power.
- **Alerts**: "Colony Alpha: Low Food", "Ship 'Voyager': Arrived at Sirius".
- **Active Projects**: Research progress bar, top 3 construction jobs.

### 3.2 Starmap (Exploration)
**Previously**: 2D Scatter Plot.
**New Design**: **"Stellar Cartography Database"** (Table View)

| System Name | Class | Distance | Status | Operations |
|-------------|-------|----------|--------|------------|
| Sol         | G2V   | 0 LY     | Home   | [View] |
| Alpha Cent. | G2V   | 4.3 LY   | Scanned| [Send Probe] [Warp] |
| Sirius      | A1V   | 8.6 LY   | Unknown| [Send Probe] |

- **Filters**: By Distance, Class, Status.
- **Actions**:
    - **Probe**: Opens modal to select probe type/fuel.
    - **Warp**: Send fleet (if available).
    - **View**: Go to System detail.

**System Detail View**:
- **Celestial Bodies**: List of planets/moons.
- **Resources**: Known abundance (Iron: High, Water: None).
- **Anomalies**: List of signal sources.

### 3.3 Colony Management
**Previously**: Hex grid building placement.
**New Design**: **"Colony Operations"** (Tabbed Interface)

**Tab 1: Infrastructure**
- **Building List**: Grouped by type (Mining, Industrial, Habitation).
- **Construction**: "Build Mine (Cost: 100 Iron, 50 Credits)". No grid placement.
- **Status**:
    - Mine (Lvl 1) x5: [Operational] [Shutdown] [Upgrade]
    - Efficiency: 95% (Lack of Power)

**Tab 2: Population & Labor**
- **Job Board**: Table of jobs vs available workers.
- **Assignment**: Sliders or +/- buttons to assign workers to "miners", "farmers", "scientists".
- **Morale**: Detailed breakdown (Housing +10, Overworked -5).

**Tab 3: Resources (Warehouse)**
- **Stockpile**: Table of all resources.
- **Flow**: Production/Turn, Consumption/Turn, Net Change.
- **Storage**: Capacity/Usage bars.

### 3.4 Logistics (Trains/Trade)
**Previously**: Visual lines on map.
**New Design**: **"Trade Network Console"**

- **Routes Table**:
    | Origin | Dest | Cargo | Status | Efficiency |
    |--------|------|-------|--------|------------|
    | Sol    | Alpha| Iron  | Active | 100%       |
- **Create Route**: Form to select Origin, Dest, Cargo Types.
- **Fleet**: List of available Transports.

### 3.5 Research
- **Tech Tree**: Indented list or collapsible folders.
- **Progress**: Progress bars for active research.
- **Queue**: List of queued tech.

---

## 4. Mechanics Adaptation

| Mechanic | Old Visualization | New Mechanics-First Approach |
|----------|-------------------|------------------------------|
| **Distance** | Visual gap on map | "Travel Time: 5 Turns". Fuel cost calculation visible. |
| **Orbit** | Moving dots | "Current Season: Summer (Perihelion)". Modifies solar output. |
| **Terrain** | Hex tiles | "Planet Modifier: Rocky (+10% Mining cost)". |
| **Combat** | (Future) Units on map | "Battle Report": Text log of damage/rounds (like OGame). |

---

## 5. Technical Implementation

**Stack**: Rust (Actix) + Tera Templates + HTMX.

### 5.1 CSS Framework
- **Grid/Flexbox**: For layouts.
- **CSS Variables**: Theming.
- **No Canvas**: Pure DOM elements.

### 5.2 HTMX Usage
- **Polling**: `hx-trigger="every 5s"` for resource tickers.
- **Partials**: Clicking "Build" replaces just the building row/list.
- **Modals**: Used for complex inputs (e.g. detailed route config).

---

## Next Steps

1.  **Approval**: Confirm this text-based direction.
2.  **Prototype**: Build the "Stellar Cartography" list view to replace the Galaxy Map.
3.  **Migration**: Port existing Colony logic to the new List view.
