# Outpost 3: Wormhole Empire - Feature Roadmap

This roadmap outlines the implementation plan for Outpost 3, organized by development phases. Check off items as they are completed.

**Last Updated**: 2025-12-30

---

## Phase 1: Foundation & Skeleton ✅

**Goal**: Set up basic project structure, dependencies, and infrastructure

### Project Setup
- [x] Create `CLAUDE_RUST.md` with Rust best practices
- [x] Create `DESIGN.md` with comprehensive game design
- [x] Create `ROADMAP.md` with feature checklist
- [x] Create `README.md` with setup instructions
- [x] Create `Cargo.toml` workspace configuration
- [x] Create project directory structure

### Core Infrastructure
- [x] Set up Actix-web server with basic configuration
- [x] Implement database connection pooling (r2d2 + SQLite)
- [x] Create initial database schema
- [x] Set up migration system
- [x] Configure Tera template engine
- [x] Set up static file serving (CSS, JS, images)
- [x] Implement basic logging with tracing
- [x] Create configuration management (config.rs)

### Event Sourcing Foundation
- [x] Define `GameEvent` base structure
- [x] Implement `EventStore` trait
- [x] Create SQLite-based event store implementation
- [x] Add event serialization/deserialization (serde)
- [x] Implement event replay mechanism
- [x] Create `Command` trait definition
- [x] Set up command validation framework

### Web Foundation
- [x] Create base HTML template (`base.html`)
- [x] Set up HTMX integration (CDN link)
- [x] Create main CSS file with reset and variables
- [x] Implement basic routing structure
- [x] Create index/home page route
- [x] Add 404 error handling
- [x] Set up HTMX response headers helper functions

### Testing Foundation
- [x] Set up test directory structure
- [x] Create sample unit test
- [x] Create sample integration test
- [x] Configure test database (in-memory SQLite)

---

## Phase 2: Basic Colony Screen (MVP) ✅

**Goal**: Implement a working colony view with basic building mechanics

### Domain Models
- [x] Define `ColonyId`, `BuildingId`, `PlanetId` newtypes
- [x] Create `Colony` entity struct
- [x] Create `Building` entity struct
- [x] Define `BuildingType` enum (Mine, PowerPlant, Housing)
- [x] Define `BuildingState` enum
- [x] Create `Resources` struct (Credits, Energy, Iron, Food)
- [x] Implement resource arithmetic operations

### Colony Events
- [x] `ColonyFounded` event
- [x] `BuildingConstructionStarted` event
- [x] `BuildingConstructionCompleted` event
- [x] `BuildingStateChanged` event
- [x] `ResourcesExtracted` event
- [x] `ResourcesConsumed` event
- [x] `TurnAdvanced` event

### Colony Commands
- [x] `FoundColony` command with validation
- [x] `ConstructBuilding` command with validation
- [x] `ChangeBuildingState` command
- [x] `AdvanceTurn` command

### Colony Service
- [x] Create `ColonyService` struct
- [x] Implement `get_colony()` query
- [x] Implement `execute_command()` orchestration
- [x] Implement basic turn processing logic
- [x] Add resource extraction simulation

### Database
- [x] Create `colonies` table schema
- [x] Create `buildings` table schema
- [x] Create `resource_stockpiles` table schema
- [x] Implement colony CRUD operations
- [x] Implement building CRUD operations
- [x] Create indexes for performance

### Colony Web Handlers
- [x] `GET /colony/{id}` - View colony page
- [x] `POST /colony/create` - Create new colony
- [x] `POST /colony/{id}/building` - Construct building
- [x] `GET /colony/{id}/resources` - Get resource stockpile (HTMX partial)
- [x] `POST /colony/{id}/turn` - Advance turn
- [x] `GET /colony/{id}/buildings` - Get building list (HTMX partial)

### Colony Templates
- [x] `colony.html` - Main colony screen
- [x] `components/colony_header.html` - Colony name and stats
- [x] `components/resource_display.html` - Resource stockpile
- [x] `components/building_list.html` - List of buildings
- [x] `components/build_menu.html` - Building construction form
- [x] `components/turn_controls.html` - Turn advancement UI

### Colony UI Features
- [x] Display colony name and founding date
- [x] Show current resource stockpile
- [x] List all buildings with status
- [x] Building construction form (select type)
- [x] Disable build button if insufficient resources
- [x] Show building construction progress
- [x] "Advance Turn" button
- [x] Display current turn number

### Colony CSS
- [x] Layout for colony screen (grid/flexbox)
- [x] Styling for resource display
- [x] Styling for building list
- [x] Styling for forms and buttons
- [x] Color scheme for different building types
- [x] Responsive layout (desktop-first)

### Testing
- [x] Unit tests for colony domain logic
- [x] Unit tests for colony commands
- [x] Unit tests for resource calculations
- [x] Integration test: Create colony
- [x] Integration test: Construct building
- [x] Integration test: Advance turn

---

## Phase 3: Expanded Colony Features

**Goal**: Add more building types, power grid, population, and production chains

### Domain Expansion
- [ ] Add 10+ more `BuildingType` variants
  - [ ] Factory (with input/output resources)
  - [ ] Farm
  - [ ] Refinery
  - [ ] Warehouse
  - [ ] Commercial Zone
  - [ ] Medical Facility
  - [ ] Research Facility
  - [ ] Train Station
  - [ ] Solar Power Plant
  - [ ] Nuclear Power Plant
- [ ] Create `ResourceType` comprehensive enum (20+ types)
- [ ] Add `Population` struct with population tracking
- [ ] Add `PowerGrid` struct with generation/consumption
- [ ] Create `ProductionChain` definitions

### Building Mechanics
- [ ] Implement building construction time (multi-turn)
- [ ] Add building operation costs (power, fuel, maintenance)
- [ ] Implement building outputs (resources produced)
- [ ] Add worker requirements (population allocation)
- [ ] Implement building upgrade system
- [ ] Add building damage and repair mechanics

### Power Grid System
- [ ] Calculate total power generation
- [ ] Calculate total power consumption
- [ ] Implement brownout effects (insufficient power)
- [ ] Add power grid UI display
- [ ] Show power status per building

### Population System
- [ ] Implement population count
- [ ] Add population growth simulation
- [ ] Create labor allocation system
- [ ] Implement population needs (food, housing)
- [ ] Add morale system basics
- [ ] Track unemployment vs. labor shortage

### Production Chains
- [ ] Define resource dependencies (e.g., Iron Ore → Steel)
- [ ] Implement factory input/output processing
- [ ] Add production efficiency calculations
- [ ] Create resource flow visualization (optional)

### Additional Events
- [ ] `PowerGridUpdated` event
- [ ] `PopulationGrew` event
- [ ] `ResourcesProduced` event
- [ ] `LaborAllocated` event
- [ ] `BuildingUpgraded` event
- [ ] `BuildingDamaged` event
- [ ] `BuildingRepaired` event

### Additional Commands
- [ ] `AllocateLabor` command
- [ ] `UpgradeBuilding` command
- [ ] `RepairBuilding` command
- [ ] `ShutdownBuilding` command

### UI Enhancements
- [ ] Power grid status indicator
- [ ] Population count and growth rate
- [ ] Labor allocation interface
- [ ] Building detail modal/panel
- [ ] Production output indicators
- [ ] Resource flow diagram (optional)

### Testing
- [ ] Unit tests for power grid calculations
- [ ] Unit tests for population growth
- [ ] Unit tests for production chains
- [ ] Integration test: Multi-turn production
- [ ] Integration test: Power shortage scenario

---

## Phase 4: Planet & Wormhole System

**Goal**: Add planet generation, exploration, and wormhole gate construction

### Planet Generation
- [ ] Create `Planet` struct with properties
- [ ] Define `PlanetType` enum (Terrestrial, Desert, Ice, etc.)
- [ ] Define `Atmosphere`, `Temperature`, `Gravity` enums
- [ ] Implement procedural planet name generator
- [ ] Implement planet parameter randomization
- [ ] Add resource richness generation
- [ ] Add hazard level calculation
- [ ] Create planet description generator
- [ ] Add difficulty rating calculation

### Wormhole Entities
- [ ] Create `WormholeGate` struct
- [ ] Create `WormholeId` newtype
- [ ] Define `GateState` enum
- [ ] Add gate construction properties (cost, time, energy)
- [ ] Create `WormholeNetwork` graph structure

### Exploration System
- [ ] Create `ExplorationFacility` building type
- [ ] Implement planet discovery algorithm
- [ ] Add discovery chance calculation
- [ ] Store discovered planets in database

### Wormhole Events
- [ ] `PlanetDiscovered` event
- [ ] `GateConstructionStarted` event
- [ ] `GateConstructionCompleted` event
- [ ] `GateActivated` event
- [ ] `GateDeactivated` event

### Wormhole Commands
- [ ] `ExploreForPlanets` command
- [ ] `ConstructGate` command
- [ ] `ActivateGate` command
- [ ] `DeactivateGate` command

### Wormhole Service
- [ ] Create `ExplorationService`
- [ ] Implement planet discovery logic
- [ ] Implement gate construction validation
- [ ] Create wormhole network pathfinding

### Database
- [ ] Create `planets` table schema
- [ ] Create `wormhole_gates` table schema
- [ ] Create `discoveries` table (planet exploration log)
- [ ] Add planet/gate CRUD operations

### Web Handlers
- [ ] `GET /planets` - List all discovered planets
- [ ] `GET /planet/{id}` - View planet details
- [ ] `POST /explore` - Trigger planet exploration
- [ ] `POST /gate/construct` - Start gate construction
- [ ] `GET /network` - View wormhole network

### Templates
- [ ] `exploration.html` - Exploration interface
- [ ] `planet_detail.html` - Planet information page
- [ ] `network.html` - Wormhole network visualization
- [ ] `components/planet_card.html` - Planet summary
- [ ] `components/gate_status.html` - Gate construction/status

### UI Features
- [ ] Planet list with filtering
- [ ] Planet detail view with all parameters
- [ ] Exploration button with cooldown
- [ ] Gate construction form
- [ ] Network graph visualization (simple table/list initially)
- [ ] Visual connection between planets

### Testing
- [ ] Unit tests for planet generation
- [ ] Unit tests for gate construction validation
- [ ] Integration test: Discover planet
- [ ] Integration test: Build gate
- [ ] Integration test: Network pathfinding

---

## Phase 5: Train System

**Goal**: Implement trains, routes, and cargo/passenger transport

### Train Entities
- [ ] Create `Train` struct
- [ ] Create `TrainId`, `RouteId` newtypes
- [ ] Define `TrainType` enum (Freight, Passenger, Mixed)
- [ ] Define `TrainSize` enum
- [ ] Define `TrainState` enum (Idle, InTransit, Loading, etc.)
- [ ] Create `Cargo` struct
- [ ] Create `Route` struct
- [ ] Define `RouteType` enum

### Train Mechanics
- [ ] Implement train purchase system
- [ ] Add train capacity calculations
- [ ] Implement route creation logic
- [ ] Add route pathfinding through network
- [ ] Implement train assignment to routes
- [ ] Create cargo loading logic
- [ ] Create passenger boarding logic
- [ ] Implement train dispatch
- [ ] Add travel time calculation
- [ ] Implement train arrival and unloading

### Train Events
- [ ] `TrainPurchased` event
- [ ] `RouteCreated` event
- [ ] `TrainAssignedToRoute` event
- [ ] `TrainDispatched` event
- [ ] `CargoLoaded` event
- [ ] `TrainDeparted` event
- [ ] `TrainArrived` event
- [ ] `CargoUnloaded` event
- [ ] `PassengersBoarded` event
- [ ] `PassengersDisembarked` event

### Train Commands
- [ ] `PurchaseTrain` command
- [ ] `CreateRoute` command
- [ ] `AssignTrainToRoute` command
- [ ] `RemoveTrainFromRoute` command
- [ ] `DispatchTrain` command
- [ ] `LoadCargo` command (automatic)
- [ ] `UnloadCargo` command (automatic)

### Train Service
- [ ] Create `TrainService`
- [ ] Implement train movement simulation
- [ ] Add route scheduling logic
- [ ] Calculate cargo revenue
- [ ] Calculate passenger revenue
- [ ] Track train operating costs

### Database
- [ ] Create `trains` table schema
- [ ] Create `routes` table schema
- [ ] Create `train_assignments` table schema
- [ ] Create `cargo_manifests` table (what's being shipped)
- [ ] Add train/route CRUD operations

### Web Handlers
- [ ] `GET /trains` - List all trains
- [ ] `POST /trains/purchase` - Buy new train
- [ ] `GET /train/{id}` - View train details
- [ ] `GET /routes` - List all routes
- [ ] `POST /routes/create` - Create new route
- [ ] `POST /route/{id}/assign` - Assign train to route
- [ ] `POST /train/{id}/dispatch` - Manually dispatch train

### Templates
- [ ] `trains.html` - Train management screen
- [ ] `routes.html` - Route management screen
- [ ] `components/train_card.html` - Train summary
- [ ] `components/route_card.html` - Route summary
- [ ] `components/train_purchase_form.html` - Buy train
- [ ] `components/route_form.html` - Create route

### UI Features
- [ ] Train list with status indicators
- [ ] Train purchase interface with type selection
- [ ] Route creation form (origin, destination, type)
- [ ] Drag-and-drop train assignment (or form-based)
- [ ] Train position tracking (which planet or in-transit)
- [ ] Cargo/passenger count display
- [ ] Route profitability display

### Testing
- [ ] Unit tests for train movement calculation
- [ ] Unit tests for cargo loading
- [ ] Unit tests for route pathfinding
- [ ] Integration test: Purchase and dispatch train
- [ ] Integration test: Complete cargo delivery
- [ ] Integration test: Multi-hop route

---

## Phase 6: Economic System

**Goal**: Implement markets, dynamic pricing, and trade profitability

### Market System
- [ ] Create `Market` struct
- [ ] Add supply tracking per resource per planet
- [ ] Add demand tracking per resource per planet
- [ ] Implement dynamic pricing algorithm
- [ ] Add market equilibrium simulation
- [ ] Create planet-specific price modifiers

### Economic Entities
- [ ] Create `Credits` newtype with operations
- [ ] Create `Transaction` log structure
- [ ] Add income/expense tracking
- [ ] Implement profit calculation for routes

### Economic Events
- [ ] `CreditsEarned` event (with source)
- [ ] `CreditsSpent` event (with reason)
- [ ] `MarketPriceChanged` event
- [ ] `TradeCompleted` event
- [ ] `ResourceSold` event
- [ ] `ResourcePurchased` event

### Economic Commands
- [ ] `SellResource` command (manual or automatic)
- [ ] `BuyResource` command (for importing)
- [ ] `SetMarketPolicy` command (auto-sell thresholds, etc.)

### Economic Service
- [ ] Create `EconomyService`
- [ ] Implement market price updates (per turn)
- [ ] Calculate supply/demand from production/consumption
- [ ] Implement automatic selling of surplus
- [ ] Implement automatic buying of shortages (optional)
- [ ] Track colony income statement

### Database
- [ ] Create `markets` table schema
- [ ] Create `transactions` table schema
- [ ] Create `price_history` table (for graphs)
- [ ] Add market CRUD operations

### Web Handlers
- [ ] `GET /economy` - Economy dashboard
- [ ] `GET /economy/colony/{id}` - Colony finances
- [ ] `GET /market/{planet_id}` - Planet market prices
- [ ] `POST /market/sell` - Manual resource sale
- [ ] `GET /economy/transactions` - Transaction history

### Templates
- [ ] `economy.html` - Economy dashboard
- [ ] `components/income_statement.html` - Income/expenses
- [ ] `components/market_prices.html` - Price table
- [ ] `components/trade_profitability.html` - Route profits
- [ ] `components/price_chart.html` - Price history graph (optional)

### UI Features
- [ ] Income/expense breakdown
- [ ] Profit/loss per colony
- [ ] Market prices table (all resources, all planets)
- [ ] Trade route profitability ranking
- [ ] Price trend indicators
- [ ] Transaction log viewer

### Testing
- [ ] Unit tests for pricing algorithm
- [ ] Unit tests for profit calculations
- [ ] Integration test: Market price changes over time
- [ ] Integration test: Profitable trade route
- [ ] Integration test: Market equilibrium

---

## Phase 7: Polish & Enhancement

**Goal**: Improve UX, add visualizations, and refine gameplay

### Visual Improvements
- [ ] Create custom color scheme and theming
- [ ] Design icons for building types
- [ ] Add icons for resource types
- [ ] Improve table styling
- [ ] Add hover effects and transitions
- [ ] Create loading indicators for HTMX requests
- [ ] Add success/error toast notifications

### Data Visualization
- [ ] Implement charts library (Chart.js or similar)
- [ ] Add resource stockpile graph (over time)
- [ ] Add population growth graph
- [ ] Add price history charts
- [ ] Add trade volume chart
- [ ] Create visual network graph (D3.js or similar)

### UX Enhancements
- [ ] Add keyboard shortcuts (e.g., Space to advance turn)
- [ ] Implement search/filter for building list
- [ ] Add sorting for tables (trains, routes, etc.)
- [ ] Create tooltips for complex UI elements
- [ ] Add confirmation dialogs for destructive actions
- [ ] Implement undo/redo (leverage event sourcing)
- [ ] Add autosave indicator

### Game Balance
- [ ] Tune building costs
- [ ] Adjust resource extraction rates
- [ ] Balance power generation/consumption
- [ ] Adjust train speeds and capacities
- [ ] Tune market price ranges
- [ ] Balance planet difficulty

### Tutorial & Help
- [ ] Create first-time user tutorial
- [ ] Add in-game help tooltips
- [ ] Create "How to Play" page
- [ ] Add building encyclopedia (what each building does)
- [ ] Create resource guide

### Save/Load UI
- [ ] Add "Save Game" button (manual snapshot)
- [ ] Create save file browser
- [ ] Implement "Load Game" functionality
- [ ] Add save file metadata display
- [ ] Allow save file export/import

### Performance Optimization
- [ ] Implement event store snapshots
- [ ] Add query result caching
- [ ] Optimize database indexes
- [ ] Lazy-load large lists
- [ ] Profile and optimize hot paths

### Testing & QA
- [ ] Comprehensive integration test suite
- [ ] Performance benchmarks
- [ ] Load testing (large game states)
- [ ] Manual QA across multiple browsers
- [ ] Fix identified bugs

---

## Phase 8: Advanced Features (Future)

**Goal**: Add depth and complexity for long-term gameplay

### Research & Technology
- [ ] Create research project entities
- [ ] Implement research point accumulation
- [ ] Add research completion system
- [ ] Create tech benefits (efficiency, unlocks)
- [ ] Add research UI screen

### Pollution & Environment
- [ ] Implement pollution generation
- [ ] Add pollution effects (morale, health)
- [ ] Create pollution remediation buildings
- [ ] Add environmental impact visualization

### Advanced Train Mechanics
- [ ] Implement train scheduling
- [ ] Add wormhole gate congestion/throughput limits
- [ ] Create train maintenance system
- [ ] Add train accidents/breakdowns
- [ ] Implement priority cargo systems

### Events & Narrative
- [ ] Create random event system
- [ ] Add natural disasters
- [ ] Implement discoveries and anomalies
- [ ] Create narrative event chains
- [ ] Add event notification UI

### Advanced Economy
- [ ] Implement loans and debt
- [ ] Add stock market for companies
- [ ] Create subsidies and taxes
- [ ] Implement contracts (deliver X by turn Y)

### AI & Automation
- [ ] Create AI governor for colony automation
- [ ] Implement automatic route creation
- [ ] Add automatic building construction
- [ ] Create resource balancing AI

### Multiplayer Preparation
- [ ] Refactor for multi-player state
- [ ] Add player entities
- [ ] Implement turn synchronization
- [ ] Create player interaction mechanics

### Migration to Desktop/WASM
- [ ] Research Bevy engine integration
- [ ] Design ECS architecture mapping
- [ ] Implement WASM build
- [ ] Create desktop build configuration

---

## Completed Features

### Phase 1: Foundation ✅
- [x] Created CLAUDE_RUST.md
- [x] Created DESIGN.md
- [x] Created ROADMAP.md

---

## Notes

- Check off items as they are completed
- Items can be reordered based on priority
- New features can be added to appropriate phases
- Some features may be split or combined during implementation
- Testing should accompany each feature implementation

**Development Approach**: Iterative and incremental. Build vertically (complete features) rather than horizontally (half-finished features).

---

**Last Updated**: 2025-12-30
