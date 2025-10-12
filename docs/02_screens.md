# Game Screens 

| **Category / Phase**             | **Screen / Modal**                        | **Purpose / Description**                                        | **Key Navigation / Links**                            |
| -------------------------------- | ----------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------- |
| **Global / Core**                | Main Menu                                 | Entry point to start, load, or configure game                    | → New Game Config, Load Game, Settings, Mods, Credits |
|                                  | Game Settings                             | Adjust global audio, graphics, controls, gameplay, accessibility | ← Main Menu / Pause Menu                              |
|                                  | Load Game                                 | Load or delete save files                                        | ← Main Menu / In-Game Menu                            |
|                                  | Save Game                                 | Save or overwrite current session                                | ← Pause Menu                                          |
|                                  | Mod Management                            | Enable/disable and order mods                                    | ← Main Menu                                           |
|                                  | Game Credits                              | Show team, contributors, licenses                                | ← Main Menu                                           |
|                                  | ✅ Pause / In-Game Menu                    | In-session menu for save, load, settings, exit                   | ↔ All gameplay screens                                |
|                                  | ✅ Tutorial / Onboarding                   | Introduce controls, objectives, context                          | → Main Menu (optional replay)                         |
|                                  | ✅ Help / Codex                            | Reference encyclopedia for concepts & resources                  | Accessible from any gameplay screen                   |
| **New Game Setup**               | New Game Configuration                    | Define player, difficulty, starting conditions                   | → Star System Selection                               |
| **Mission Planning**             | Star System Selection Map                 | Select target system for colonization                            | ↔ Star System Details Modal                           |
|                                  | Star System Details Modal                 | Show info about selected system (resources, habitability)        | → Ship Configuration                                  |
| **Ship Prep & Journey**          | Ship Configuration                        | Prepare colony ship: crew, cargo, type                           | → Ship Journey Log                                    |
|                                  | Cargo Details Modal                       | Show breakdown of cargo, dependencies, warnings                  | Within Ship Config                                    |
|                                  | Ship Journey Log                          | Event-driven voyage sequence                                     | → Star System Map (on arrival)                        |
|                                  | Outcome Summary Modal                     | Show cumulative journey effects                                  | Within Ship Journey Log                               |
| **System Arrival & Exploration** | Star System Map                           | Primary system view, shows orbits and planets                    | ↔ Celestial Body Details Modal                        |
|                                  | Celestial Body Details Modal              | Planet info, probe options, travel                               | → Celestial Body Orbit View                           |
|                                  | Celestial Body Orbital View               | Deploy satellites, landers, scan surfaces                        | → Surface Map                                         |
|                                  | Satellite Deployment Modal                | Configure orbit assets                                           | From Orbital View                                     |
|                                  | Surface Scan Results Modal                | Display scan data                                                | From Orbital View                                     |
| **Planetary Survey & Landing**   | Celestial Body Surface Map (All Sectors)  | Map of sectors with exploration data                             | ↔ Sector Details Modal                                |
|                                  | Sector Details Modal                      | Sector terrain/resources/hazards                                 | → Launch Mission / Select for Colony                  |
|                                  | Celestial Body Sector Map (Single Sector) | Detailed colony management map                                   | ↔ Tile Details Modal                                  |
|                                  | Map Tile Details Modal                    | Tile terrain/resources/actions                                   | Contextual actions like build/excavate                |
| **Encounters & Missions**        | Random Event / Encounter Modal            | Narrative or random events                                       | Global trigger                                        |
|                                  | Probe / Mission Details Modal             | Manage exploration or expedition missions                        | From many screens                                     |
| **Colony Management**            | Colony Overview                           | Central hub for colony status & metrics                          | → Sub-Overviews                                       |
|                                  | Settlement / Installation Overview        | List of bases and installations                                  | → Settlement Details                                  |
|                                  | Settlement / Installation Details         | Detailed per-base configuration                                  | ↔ Overview / Colony                                   |
|                                  | Celestial Body Overview                   | List of system bodies & stats                                    | → Orbital / Surface screens                           |
|                                  | Trade / Logistics Overview                | Visualize and edit trade routes                                  | ↔ Colonies and vehicles                               |
|                                  | Production Chain Configuration            | Manage extraction → manufacturing workflows                      | ↔ Vehicles / Buildings                                |
|                                  | Automation Configuration Modal            | Script and control automated behavior                            | ↔ Vehicles / Buildings                                |
|                                  | Vehicle Overview                          | List and control of all vehicles                                 | → Vehicle Details Modal                               |
|                                  | Vehicle Details Modal                     | Configure, upgrade, automate vehicle                             | ↔ Vehicle Overview                                    |
|                                  | Building Details Modal                    | Configure, repair, upgrade structures                            | ↔ Colony / Sector Map                                 |
|                                  | Policy & Governance Screen                | Manage laws, taxes, workforce, etc.                              | ↔ Colony Overview                                     |
|                                  | Research Screen                           | Manage research tree and labs                                    | ↔ Colony Overview                                     |
|                                  | ✅ Analytics / Statistics Screen           | Charts of performance over time                                  | ↔ Colony Overview                                     |
|                                  | ✅ Achievements / Milestones Screen        | Track long-term goals and successes                              | ↔ Colony Overview                                     |
| **UI / Meta Layers**             | Messages Panel                            | Persistent alerts & notifications                                | Present on all gameplay screens                       |
|                                  | Event Log Screen                          | Persistent log of events & outcomes                              | Global navigation                                     |
| **Expansion (Future)**           | System Overview Screen                    | Multi-colony/system-level dashboard                              | ↔ Colony & Celestial Body Overviews                   |

## Main Menu Screen

- Purpose: Entry point; access to game start, load, settings, credits, exit
- Elements:
    - "New" Game btn -> "New Game Configuration" Screen
    - "Load" Game btn -> "Load Game" Screen
    - "Settings" btn -> "Game Settings" Screen
    - "Mods" btn -> "Mod Management" Screen
    - "Credits" btn -> Credis Screen
    - "Exit" btn -> Exit to Desktop
    - "Continue" btn (If compatible save exists) -> Load Gamen and Show Last Screen
    - Background: Dynamic animated view
    - Version & Build Info

## Game Settings Screen

- Purpose: Adjust global settings
- Elements
    - Tabs
        - Audio Tab
        - Graphics Tab
        - Controls Tab
            - Keybindings
            - Controller configuration
        - Gameplay Tab
        - Accessibility Tab
    - "Back" btn -> Return to "Main Menu" Screen

## Load Game Screen

- Purpose: List and manage save files
- Elements:
    - Save file list (date, colony name, system name)
    - Preview info: screenshot, summary stats
    - "Load" btn -> Load selected save file: show confirmation modal
    - "Delete" btn -> Delete selected save file; show confirmation modal
    - "Back" btn -> Return to Main Menu Screen or to In-Game Settings Screen
    - Quick save/Quick load

### Save Mode

- Same screen, but called from in-game
- Adds ability to create a save game or save to an existing file
- Quick save / quick load features

## Mod Management Screen

- Purpose: show mods detected in folder, display information about them including compatibility, and allow them to be enabled/disabled. Indicate if a mod will result in disabling achievements
- Elements
    - version compatiblity
    - load order controls
    - enable/disable all btn
    - dependency conflict indicator

## Pause/In-Game Menu Modal

- Purpose: in-session access to settings, save/load, exit, and help functions
- Accessed by ESC key or menu button
- Elements
    - Resume
    - Save/Load -> Save/Load Screen
    - Settings -> Settings Screen
    - Exit to Main Menu -> Main Menu Screen (with confirmation modal)
    - Exit to Desktop -> Exit to desktop (with confirmation modal)

## Help/Codex Screen

- Purpose: provide a wiki-like document about all aspects of the game
- Elements
    - Search bar
    - Tabs, other wiki elements

## Game Credits Screen

- Contains information about the developer and publisher, along with a scrollable (scrolling?) list of attributions

## New Game Configuration Screen

- Purpose: Define player, difficulty, and starting conditions
- Elements:
    - Input for player name
    - Dropdowns, sliders, input boxes, check boxes for difficulty, starting conditions
    - Mod selector modal
    - Advanced settings button -> Advanced settings modal/screen
    - "Next" btn -> Star System Selection

## Star System Selection Map Screen

- Purpose: Select target system for colonization
- Elements:
    - Map of nearby stars with with probe coverage indicators
    - Hover tooltips showing distance, type, probe data completeness
    - Travel time indicator
    - Ship power indicator?
    - Star System Details Modal
        - Purpose: Show information about selected star system
        - Elements
            - System name
            - Spectral type
            - Estimated planet count
            - Known resources
            - Anomalies
            - Habitability
            - (If probe sent): Sensor uncertainty
            - Select system btn -> Select system as colony target
            - Send probe btn -> Send probe
    - mini-map of galaxy for context
    - Filters for distance, habitability, richness, risk

## Ship Configuration Screen

- Purpose: Prepare the colony ship
- Elements
    - Tabs
        - Crew
        - Cargo
        - Robots/Vehicles
        - Modules
        - Supplies
    - Resource bars
        - Mass
        - Volume
        - Power Budget
        - Cost?
    - Toggle for "Generation Ship" or "Sleeper Ship"
    - Summary/manifest preview
    - "Launch Mission" btn
    - "Cargo Details" Modal: Show a breakdown of item types, dependencies, storage, location, and warnings
    - Template management
    - (Future) ship schematic graphic
    - Warning indicator

## Ship Journey Log Screen

- Purpose: Event-driven narrative during travel
- Elements
    - Timeline or scrollable log wiith events
    - Illustrations, comm logs, crew messages
    - Player choice modals for events
    - System integrity and morale meters
    - Outcome summary modal: Show the impact of events during the journey to the colony mission
        - Enable player to review choices?

## Star System Map Screen

- Purpose: Primary map view of destination system
- Elements
    - Map
        - Star at center
        - Orbit lines
        - Planet Icons
        - Asteroid and comet zones
        - Ship orbit and position
        - Indicators for explored/unexplored bodies
    - Send probe btn
    - Visit btn
    - Celestial body details modal
        - image
        - type
        - atmosphere/climate
        - gravity
        - resources summary
        - anomalies
        - hazards
        - missions
        - "View Orbit" btn -> "Celstial Body Orbital View" Screen
        - "Send Probe" btn -> send probe; events; update info
        - "Travel to Orbit" btn -> move ship to orbit around body
        - "Mission" btn -> select for establishing a colony, sending a mission, etc.

## Celestial Body Orbital View Screen

- Purpose: Transition point between system and surface maps
- Elements
    - Planet rotating view (2.5 D)
    - Buttons to deploy satellites, landers, or scan surface
    - Readouts: fuel
    - Satellite deployment modal
    - Surface scan results modal
    - Missions summary list
    - "Show Surface Map" btn -> "Celestial Body Surface Map"
    - Button to dock at a station or de-orbit/land
    - Btn to view orbital assets in an overview list
    - Orbital asset list panel/modal
        - list of orbital assets
        - action buttons
        - double-click for details modal

## Celestial Body Surface Map (All Sectors) Screen

- Purpose: Planet/moon map segmented into sectors
- Elements
    - Rectangular map divided into sectors; divisions drawn with borders
    - Click to select sector -> display details in sector details modal
    - Toggle layers
        - Topography
        - Structures
        - Roads
        - Subsurface
        - Energy grid
        - Temperatures/weather/atmosphere
        - Resource deposits
        - hazards/anomalies
        - missions list
        - Colony site candidates (heat map for desirability)
            - Filters for desirability
    - Sector Details Modal
        - Terrain info
        - Hazards/anomalies
        - Resources
        - Atmosphere/weather
        - Missions list
        - Exploration status
    - Launch Mission btn -> send expedition, send probe, select for colonization

## Celestial Body Sector Map (Single Sector) Screen

- Purpose: Main colony management and construction view
- Elements
    - Tile-based layout showing terrain, structures, vehicles, resources
    - Layer: topography, structures, roads, infrastructure
    - Elevation toggle for different levels
    - Unit/building summary panel
    - Selected Tile summary panel
    - Action buttons
        - Game speed/pause btn
        - Settings btn
    - Map Tile Details Modal
        - Terrain
        - Resources
        - Structures
        - Weather conditions
        - Contextual Actions like Build, Excavate, Survey, Repair, etc.
    - Mini-map for navigation
    - Zoom btn for zooming in/out

## Random Event/Encounter Modal

- Purpose: Deliver narrative decisions or random challenges
- Elements
    - Text description
    - character/scene images
    - Data readouts
    - Choice buttons

## Vehicle Details/Configuration Modal
- Tabs for status, orders, upgrades, crew, maintenance, and history
- Action buttons
    - automate/manual control
    - go to location
    - start/pause/stop/abort orders
    - return to base
- alerts

## Building Details/Configuration Modal
- Tabs for power, production, storage, workforce, upgrades
- Action buttons
    - Repair
    - Deconstruct
    - Pause/resume building (mothball)
- Alerts

## Colony Overview Screen

- Purpose: Central hub for the entire colony
- Elements
    - Aggregate stats: population, morale, resources
    - Graphs for production and consumption
    - Btn to display summary/list screens
        - Settlement/Installation Overview
        - Celestial Body Overview
        - Vehicle Overview
        - Policy & Governance
        - Research Overview
        - Trade Overview

## Trade / Logistics Network Overview

- Visual map of resource flows between settlements and installations
- Manual and automated route configuration
- Graphs for volume
- Stats/lists for supply/demand
- Special mode for system map to create/edit routes

## Settlement/Installation Overview Screen

- Purpose: A list of all settlements and installations, their status, and way to navigate to them
- Elements
    - List of settlements/installations
    - Double click on list element to go to overview screen for that settlement/installation
    - Search/Filter

## Celestial Body Overview Screen

- Purpose: a list of all major celestial bodies in the system and their properties
- Elements
    - List of bodies and info about each
    - Double click to go to overview screen for Celestial body
    - Search/Filter

## Policy & Governance Screen

- Purpose: Manage laws, research priorities, taxes, rationing, worker conditions, etc.
- Elements
    - Charts and graphs of status
    - Modals to manage policies and other configurations

## Research  Screen

- Purpose: Tech tree, lab projects, engineering projects
- Elements
    - Tech tree view
        - Researchable tech
        - Engineering projects
        - Status of each task (researchable, in progress, not available)
    - Lab overview
        - Lab assignment
        - Progress/status

## Vehicle Overview Screen

- Purpose: list all vehicles in the game
- Elements
    - Select by X
    - Search/Filter
    - Double click for details
    - List of vehicles and basic info for them in each row
    - Action buttons to upgrade, decommission, return to base/storage, stop/pause work, edit orders/automation

## Settlement/Installation Details/Configuration Screen

- Purpose:  Filtered-down version of Colony Overview just for the current settlement/installation in the sector
- Elements
    - Resource/production graphs
    - Colonist info
    - Labor allocation
    - Other resource allocation
    - Action buttons: upgrade, decommission, reassign staff, set policies
## Production Chain Details/Configuration Screen

- Purpose: Visualize and manage product chains of vehicles and structures. Production chain can entail one single item from extraction to production or a more complex system to produce a finished product like a vehicle (should other production chains be selectable?)
- Elements
    - Flow diagram
        - Nodes
            - Location for a resource
            - Extracting vehicle/installation
            - Processing vehicle/installation
            - Manufacturing vehicle/installation
            - Storage/disposition vehicle/installation
        - Edges
            - Transport links like roads, rails, maritime, pipelines, conveyors, interplanetary routes
        - Create new node modal
        - Edit node modal
        - Drag points between nodes or edit nodes to select destinations, sources
    - Toggle automation/manual
    - Template use and management
    - Also script mode that displays highlighted text (YAML format?) for hand editing
    - Export/load YAML file
    - Validate chain btn
    - Highlight on map btn -> show highlights on different maps for production chain elements

## Automation Details/Configuration Modal

- Purpose: Script-like interface for management and automation
- Elements
    - Flow diagram
        - Nodes: triggers, actions
        - Edges: if/then/repeat
    - Template management
    - Text editor
        - Syntax highlighted procedure in YAML
    - Load/Export YAML

## Probe/Mission Details/Configuration Modal

- Purpose: create and manage missions
- Elements
    - Mission objectives
    - Duration indicator
    - Risk indicator
    - Current status
    - Log list
    - Cost
    - Launch, pause, abort
    - Template management

## Messages Panel

- Present on every gameplay screen
- Display event notifications on every screen.
- List of events
- Flashing/other indicators for new messages
- Dismiss without reading or open message to make choice, go to location/screen for message, etc

## Event Log Screen

- Persistent list of all major events, choices, and outcomes
- Search/filter
- Generate a report for debugging or narrative
- Color coding
- Pinning (good for journal entries too -- could help with tutorial)

## Analytics Screen

- Show graphs of different statistics over time
- Export data
- Time range and filtering

## Achievements/Milestones Screen

- Show completed achievements and player milestones in-session/all-time

```
flowchart TD

%% === MAIN MENU & CORE ===
A[Main Menu] --> B[New Game Configuration]
A --> C[Load Game]
A --> D[Game Settings]
A --> E[Mod Management]
A --> F[Credits]
A --> G[Tutorial / Onboarding]

%% === GAME SETTINGS FLOW ===
D --> A
C --> A
E --> A
F --> A

%% === NEW GAME SETUP ===
B --> H[Star System Selection Map]
H --> I[Star System Details Modal]
I --> J[Ship Configuration]
J --> K[Ship Journey Log]

%% === VOYAGE PHASE ===
K --> L[Star System Map]

%% === SYSTEM EXPLORATION ===
L --> M[Celestial Body Details Modal]
M --> N[Celestial Body Orbital View]
N --> O[Surface Scan Results Modal]
N --> P[Satellite Deployment Modal]
N --> Q[Celestial Body Surface Map (All Sectors)]

%% === PLANETARY & COLONY PHASE ===
Q --> R[Sector Details Modal]
R --> S[Celestial Body Sector Map (Single Sector)]
S --> T[Map Tile Details Modal]

%% === RANDOM EVENTS & MISSIONS ===
S --> U[Random Event / Encounter Modal]
S --> V[Probe / Mission Details Modal]

%% === COLONY MANAGEMENT ===
S --> W[Colony Overview]
W --> X[Settlement / Installation Overview]
X --> Y[Settlement / Installation Details]
W --> Z[Celestial Body Overview]
W --> AA[Trade / Logistics Overview]
W --> AB[Policy & Governance Screen]
W --> AC[Research Screen]
W --> AD[Production Chain Config Screen]
W --> AE[Analytics / Statistics Screen]
W --> AF[Achievements / Milestones Screen]
W --> AG[Vehicle Overview]
AG --> AH[Vehicle Details Modal]
S --> AI[Building Details Modal]
S --> AJ[Automation Config Modal]

%% === UI / META ===
W --> AK[Event Log Screen]
W --> AL[Messages Panel]
W --> AM[Pause / In-Game Menu]
AM --> D
AM --> C
AM --> AN[Save Game]

%% === SYSTEM OVERVIEW ===
W --> AO[System Overview Screen]

%% === NAVIGATION BACKTRACKS ===
Q --> L
S --> Q
Y --> X
X --> W
L --> A
```
