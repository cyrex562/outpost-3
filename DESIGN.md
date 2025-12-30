# Outpost 3: Wormhole Empire - Design Document

## Executive Summary

**Outpost 3: Wormhole Empire** is a web-based colony and trade simulation game that combines classic Outpost colony management with Peter F. Hamilton's Commonwealth universe wormhole and train mechanics. Players build sprawling settlements around wormhole gates, develop planetary economies, and create an interstellar trade network using literal trains traveling through wormhole connections.

**Core Pillars:**
1. **Colony Development** - Expand from small outpost to sprawling settlement
2. **Economic Simulation** - Resource extraction, production chains, and trade
3. **Train Logistics** - Manage cargo and passenger trains through wormhole network
4. **Network Building** - Explore and connect new worlds via wormhole gates
5. **Economic Empire** - Build a profitable multi-world trading network

## Game Overview

### Victory Condition
Build a thriving economic empire by:
- Developing profitable colonies across multiple worlds
- Creating efficient trade networks
- Maximizing economic output and trade volume
- Expanding to new worlds and establishing new gates

### Core Gameplay Loop
1. **Develop Colony** - Build facilities around wormhole gate on current planet
2. **Extract Resources** - Mine, farm, and produce goods
3. **Build Infrastructure** - Railways, power plants, housing, factories
4. **Explore New Worlds** - Use exploration facility to find new planets
5. **Establish Gates** - Build wormhole gates to new worlds
6. **Create Trade Routes** - Assign trains to move goods and passengers
7. **Optimize Economy** - Balance supply chains, markets, and profitability
8. **Expand Network** - Repeat on new worlds, building interconnected empire

## Technical Architecture

### Technology Stack

**Backend:**
- **Language**: Rust
- **Web Framework**: Actix-web 4.x
- **Database**: SQLite with r2d2 connection pooling
- **Architecture**: Event Sourcing with CQRS
- **Serialization**: Serde with JSON

**Frontend:**
- **Rendering**: Server-side with Tera templates
- **Interactivity**: HTMX for dynamic updates
- **Styling**: Custom CSS with responsive design
- **Enhanced UX**: TypeScript for advanced client-side features
- **Updates**: Polling-based (WebSockets/SSE for future enhancement)

### Architectural Patterns

#### Event Sourcing
All state changes are captured as immutable events stored in an event log. The current game state is derived by replaying events from the log.

**Benefits:**
- Complete audit trail
- Time travel debugging
- Easy save/load implementation
- Potential for replay and analytics
- Natural fit for turn-based simulation

**Event Flow:**
```
User Action → Command → Validation → Event(s) → Event Store → State Update → UI Refresh
```

#### CQRS (Command Query Responsibility Segregation)
Separate models for writes (commands) and reads (queries):
- **Command Side**: Validates actions and generates events
- **Query Side**: Optimized read models (projections) for UI display

#### Domain-Driven Design
Clear separation between:
- **Domain Layer**: Business logic and entities (no I/O)
- **Application Layer**: Orchestration and services
- **Infrastructure Layer**: Database, web, external systems
- **Presentation Layer**: HTTP handlers and templates

### Project Structure
```
outpost-3/
├── Cargo.toml              # Workspace configuration
├── CLAUDE_RUST.md          # Rust best practices for AI
├── DESIGN.md               # This document
├── ROADMAP.md              # Feature implementation checklist
├── README.md               # Setup and run instructions
├── src/
│   ├── main.rs            # Application entry point
│   ├── lib.rs             # Library exports
│   ├── config.rs          # Configuration management
│   ├── domain/            # Domain models and business logic
│   │   ├── colony.rs      # Colony entity and logic
│   │   ├── building.rs    # Building types and operations
│   │   ├── resource.rs    # Resource types and management
│   │   ├── wormhole.rs    # Wormhole gates and connections
│   │   ├── train.rs       # Train entities and routing
│   │   ├── planet.rs      # Planet generation and properties
│   │   ├── market.rs      # Market and pricing
│   │   └── population.rs  # Population and labor
│   ├── events/            # Event sourcing infrastructure
│   │   ├── event.rs       # Event definitions
│   │   └── store.rs       # Event store implementation
│   ├── commands/          # Command pattern
│   │   ├── colony_commands.rs
│   │   ├── train_commands.rs
│   │   └── handlers.rs
│   ├── queries/           # CQRS query side
│   │   ├── colony_queries.rs
│   │   └── projections.rs
│   ├── services/          # Application services
│   │   ├── colony_service.rs
│   │   ├── economy_service.rs
│   │   ├── train_service.rs
│   │   └── exploration_service.rs
│   ├── web/               # Web layer
│   │   ├── routes.rs
│   │   ├── handlers.rs
│   │   └── middleware.rs
│   ├── db/                # Database layer
│   │   ├── schema.rs
│   │   ├── migrations.rs
│   │   └── pool.rs
│   └── simulation/        # Turn-based simulation engine
│       ├── turn.rs
│       └── processor.rs
├── static/                # Static assets
│   ├── css/
│   │   ├── main.css
│   │   └── components/
│   ├── js/
│   │   └── htmx-extensions.js
│   └── images/
├── templates/             # Tera HTML templates
│   ├── base.html
│   ├── colony.html
│   ├── network.html
│   ├── exploration.html
│   └── components/
├── tests/                 # Tests
│   ├── domain_tests.rs
│   └── integration_tests.rs
└── migrations/            # SQL migrations
    └── 001_initial_schema.sql
```

## Game Systems Design

### 1. Colony System

#### Overview
Each colony starts as a small station around a wormhole gate and expands into a sprawling settlement covering the planet's surface.

#### Colony Entities

**Colony**
- Unique ID
- Planet reference
- Wormhole gate reference
- Name
- Founded timestamp
- Resources stockpile
- Population count
- Power grid status
- Pollution level

**Building Types**

*Resource Extraction:*
- **Mine** - Extracts ores, minerals from ground
  - Types: Iron, Copper, Rare Metals, etc.
  - Output rate based on planet richness
- **Well/Pump** - Extracts water, ice
- **Atmospheric Processor** - Captures atmospheric gases
- **Logging Camp** - Harvests timber (on planets with forests)
- **Farm** - Grows food crops

*Industrial:*
- **Factory** - Converts raw materials into products
  - Consumes inputs, produces outputs
  - Multiple factory types for different products
- **Refinery** - Processes raw materials
- **Assembly Plant** - Creates complex goods

*Infrastructure:*
- **Power Plant** - Generates electricity
  - Types: Solar, Nuclear, Fusion, Geothermal
  - Fuel consumption and output varies
- **Train Station** - Handles freight and passengers
  - Platform count determines capacity
- **Railway** - Connects areas, improves train efficiency
- **Warehouse** - Stores resources

*Civic:*
- **Housing** - Accommodates population
  - Capacity and comfort levels
- **Commercial Zone** - Retail and services
- **Research Facility** - Conducts planetary/product research
- **Medical Facility** - Healthcare for population
- **Recreation** - Improves morale

*Wormhole Infrastructure:*
- **Wormhole Gate** - Connection to other worlds (one per planet initially)
- **Exploration Facility** - Searches for new worlds to connect

#### Building Mechanics

**Construction:**
- Requires: Credits, materials, energy, labor
- Takes time to complete (turns)
- Can be cancelled with partial refund

**Operation:**
- Consumes power
- May consume fuel/materials
- Produces outputs (resources, products, services)
- Requires workers (population allocation)

**State:**
- Under Construction
- Operational
- Damaged (requires repair)
- Shutdown (manually disabled)

#### Colony Development Stages
1. **Outpost** - Initial gate station, minimal facilities
2. **Settlement** - Basic resource extraction and housing
3. **Town** - Diversified industry, commerce
4. **City** - Advanced production, R&D, major trade hub
5. **Metropolis** - Sprawling industrial complex, network anchor

### 2. Resource System

#### Resource Categories

**Raw Materials:**
- Ores: Iron, Copper, Aluminum, Rare Metals
- Energy: Coal, Uranium, Hydrogen, Solar
- Organics: Timber, Biomass, Food
- Liquids: Water, Oil, Chemical Compounds
- Gases: Oxygen, Nitrogen, Noble Gases

**Processed Goods:**
- Metals: Steel, Alloys, Electronics Components
- Fuels: Refined Petroleum, Reactor Fuel Rods
- Manufactured: Machinery, Vehicles, Electronics
- Consumer Goods: Food Products, Textiles, Luxuries

**Abstract Resources:**
- Credits (money)
- Energy (megawatts)
- Labor (population allocation)
- Research Points (knowledge accumulation)

#### Production Chains
Resources flow through production chains:

```
Iron Ore → Refinery → Steel → Factory → Machinery → Assembly → Trains
Food Crops → Processing → Food Products → Commercial → Population Consumption
```

#### Resource Storage
- Each colony has finite storage capacity
- Warehouses increase capacity
- Trains can store cargo in-transit
- Excess production is wasted or sold automatically

### 3. Wormhole & Gate System

#### Wormhole Gates

**Properties:**
- Source Planet ID
- Destination Planet ID
- Status (Under Construction, Active, Disabled)
- Throughput capacity (trains per turn)
- Construction cost (very high)
- Energy requirements

**Gate Construction:**
1. Prerequisites: Sufficient credits, materials, energy, personnel
2. Exploration must have discovered target planet
3. Takes multiple turns to build
4. Both endpoints must be constructed

**Gate Network:**
- Creates directed graph of connected worlds
- Each planet starts with one gate (can expand later)
- Network visualization shows connections
- Pathfinding for multi-hop routes

#### Exploration System

**Exploration Facility:**
- Scans galaxy for new worlds
- Requires energy and time
- Success rate based on facility level
- Discovers planet parameters

**Planet Discovery:**
- Generates new procedural planet
- Reveals basic properties
- Unlocks ability to build gate to that planet

### 4. Planet System

#### Procedural Generation

**Planet Parameters:**
- **Planet Type**: Terrestrial, Desert, Ice, Ocean, Gas Giant Moon, etc.
- **Atmosphere**: Breathable, Toxic, Thin, Dense, None
- **Temperature**: Arctic, Cold, Temperate, Hot, Scorching
- **Gravity**: Low, Normal, High
- **Size**: Small, Medium, Large
- **Resources**: Richness of various resource types
- **Hazards**: Radiation, Storms, Seismic Activity
- **Biosphere**: None, Microbial, Plant Life, Complex Ecosystems

**Difficulty Factors:**
- Easy planets: Temperate, breathable air, rich resources, low hazards
- Hard planets: Extreme temps, toxic air, poor resources, high hazards
- Difficulty affects: Construction costs, maintenance, worker efficiency

**Visual Identity:**
- Procedurally generated description
- Color scheme based on type
- Icon/image representation

### 5. Train System

#### Train Entities

**Train:**
- Unique ID
- Type: Freight, Passenger, Mixed
- Size: Small, Medium, Large, Massive
- Speed: Slow, Medium, Fast
- Capacity: Cargo tons or passenger count
- Current location (planet or in-transit)
- Assigned route

**Train Types:**

*Freight Trains:*
- Small Freight (50 tons)
- Standard Freight (200 tons)
- Heavy Freight (500 tons)
- Super Heavy (1000+ tons)

*Passenger Trains:*
- Commuter (100 passengers)
- Express (300 passengers)
- Luxury Liner (150 passengers, high comfort)

*Specialized:*
- Refrigerated (perishable goods)
- Tanker (liquids/gases)
- Container (standardized cargo)

#### Train Routes

**Route Definition:**
- Origin planet
- Destination planet (or multi-stop circuit)
- Path through wormhole network
- Frequency (trains per turn)
- Assigned trains
- Cargo/passenger type priority

**Route Types:**
- **Point-to-Point**: Direct connection between two planets
- **Circuit**: Multiple stops in sequence
- **Shuttle**: Continuous back-and-forth

**Route Economics:**
- Operating costs (fuel, maintenance, crew)
- Revenue from cargo delivery
- Revenue from passenger fares
- Profit/loss tracking

#### Train Operations

**Dispatching:**
- Assign train to route
- Load cargo/passengers at origin
- Depart when full or on schedule

**In-Transit:**
- Travel time based on distance (number of hops) and train speed
- Consumes fuel
- Cannot be redirected mid-journey

**Arrival:**
- Unload cargo/passengers
- Deliver to planet's stockpile/population
- Generate revenue
- Train can be reassigned or continue route

#### Infrastructure

**Train Stations:**
- Platform count limits simultaneous operations
- Throughput capacity
- Loading/unloading speed bonuses

**Railway Network:**
- On-planet rail improves efficiency
- Reduces local transport costs
- Connects production areas to station

### 6. Economic System

#### Market Mechanics

**Supply and Demand:**
- Each planet has local market
- Prices fluctuate based on supply/demand
- High supply → lower prices
- High demand → higher prices

**Trade:**
- Planets export surplus resources
- Planets import needed resources
- Price differences drive profitable trade routes
- Market equilibrium over time

**Pricing Model:**
```
Price = Base_Price × (Demand / Supply) × Planet_Modifier
```

**Planet Modifiers:**
- Resource scarcity/abundance on planet
- Industrial development level
- Population size
- Distance from other markets

#### Credits (Currency)

**Income Sources:**
- Selling resources on local market
- Trade profits from route deliveries
- Passenger fare revenue
- Commercial building revenue

**Expenses:**
- Building construction
- Building operation and maintenance
- Train purchase and upgrades
- Train operation (fuel, crew)
- Wormhole gate construction and operation
- Population services (healthcare, food)

**Economic Victory:**
- Accumulate massive wealth
- High trade volume across network
- Efficient profitable routes
- Diversified economy

### 7. Population & Labor

#### Population Mechanics

**Population Growth:**
- Natural growth based on:
  - Housing availability
  - Food supply
  - Healthcare
  - Morale/comfort
- Immigration from other planets (via passenger trains)

**Labor Allocation:**
- Total population = available workforce
- Workers assigned to buildings
- Unemployment if excess population
- Labor shortage if insufficient population
- Efficiency based on worker availability

**Needs:**
- Food consumption (constant)
- Housing (comfort affects morale)
- Consumer goods (morale)
- Healthcare (mortality rate)
- Recreation (morale)

**Morale:**
- Affects productivity
- Influences immigration/emigration
- Based on: Housing quality, goods availability, pollution, employment

### 8. Power & Energy

#### Power Grid

**Generation:**
- Power plants produce energy (MW)
- Different types: Solar, Coal, Nuclear, Fusion
- Fuel consumption varies by type
- Construction and operating costs vary

**Consumption:**
- Buildings consume power
- Wormhole gates consume massive power
- Insufficient power → brownouts → reduced building efficiency

**Grid Management:**
- Total generation vs. total consumption
- Surplus can be stored (if batteries built)
- Deficit requires building more plants or shutting down buildings

### 9. Pollution & Environment

#### Pollution System

**Sources:**
- Factories (industrial output)
- Power plants (especially fossil fuel)
- Large populations
- Mining operations

**Effects:**
- Reduced population morale
- Health impacts (higher mortality)
- Environmental degradation
- Can be visualized/tracked

**Remediation:**
- Build pollution control facilities
- Use cleaner tech (upgrade power plants)
- Balance industrial vs. environmental concerns

### 10. Research & Development

#### Research System

**Research Focus:**
- **Planetary Knowledge**: Learn about the planet's resources, ecosystems, hazards
- **Product Research**: Discover new production chains
- **Resource Analysis**: Improve extraction efficiency
- **Economic Studies**: Market and trade optimization

**Research Facilities:**
- Generate research points per turn
- Requires scientists (allocated population)
- Projects have completion thresholds

**Benefits:**
- Unlock building upgrades
- Improve efficiency
- Reduce costs
- Discover new opportunities

### 11. Turn-Based Simulation

#### Turn Structure

Each turn represents a fixed time period (e.g., 1 week, 1 month).

**Turn Processing Order:**
1. **Resource Extraction** - Mines, farms, etc. produce resources
2. **Production** - Factories consume inputs, produce outputs
3. **Power Generation** - Calculate grid supply/demand
4. **Building Operations** - Maintenance, consumption, outputs
5. **Population** - Growth, consumption, labor allocation
6. **Train Movement** - Progress in-transit trains
7. **Market Updates** - Adjust prices based on supply/demand
8. **Economic Calculations** - Income, expenses, profits
9. **Pollution & Environment** - Accumulate/reduce pollution
10. **Research Progress** - Advance active research projects
11. **Events** - Random events (optional)

**Turn Advancement:**
- Player manually advances turn (button click)
- All calculations processed server-side
- UI updates with new state
- Turn history stored in event log

### 12. Save/Load System

#### Event Sourcing Benefits

**Automatic Save:**
- All events are persisted to database
- Current game state = replay all events
- No explicit "save" needed
- Auto-save after each turn

**Load Game:**
- Reconstruct state by replaying events from database
- Fast snapshots for performance (optional)

**Save File:**
- SQLite database file
- Portable, single-file
- Can be backed up, shared

## User Interface Design

### Screen Layout

#### Base Template
- **Header**: Game title, current turn, credits display
- **Navigation**: Tabs for different screens
- **Main Content**: Screen-specific content
- **Footer**: Quick stats, advance turn button

#### Main Screens

**1. Colony Screen (Primary Focus for MVP)**
- Colony name and planet info
- Resource stockpile display
- Power grid status
- Population stats
- Building list with status
- Build menu (construct new buildings)
- Forms for building operations (shutdown, upgrade, etc.)

**2. Network Screen**
- Visual graph of wormhole network
- Planet nodes with colony info
- Connection edges showing train routes
- Click planet to view details
- Button to explore new worlds

**3. Train Management Screen**
- List of all trains and status
- Route definitions
- Assign trains to routes
- Purchase new trains
- Train upgrade options

**4. Exploration Screen**
- Discovered planets list
- Exploration facility status
- "Explore" button to find new worlds
- Planet details for discovered worlds
- "Build Gate" button (if feasible)

**5. Economy Dashboard**
- Income/expense breakdown
- Trade route profitability
- Market prices across planets
- Economic graphs and trends

**6. Research Screen**
- Active research projects
- Available projects
- Research points accumulation
- Completed research benefits

### HTMX Patterns

**Dynamic Updates:**
```html
<!-- Resource counter with auto-refresh -->
<div id="resource-credits" hx-get="/colony/1/resources/credits" hx-trigger="every 2s">
    <span class="value">{{ credits }}</span> Credits
</div>

<!-- Build form with inline response -->
<form hx-post="/colony/1/building/construct" hx-target="#building-list" hx-swap="beforeend">
    <select name="building_type">
        <option value="mine">Mine</option>
        <option value="factory">Factory</option>
    </select>
    <button type="submit">Build</button>
</form>

<!-- Turn advancement -->
<button hx-post="/game/advance-turn" hx-target="#main-content" hx-swap="outerHTML">
    Advance Turn
</button>
```

**Component Architecture:**
- Full page templates for initial loads
- Partial templates for HTMX swaps
- Reusable components in `templates/components/`

### Responsive Design
- Desktop-first (complex management UI)
- Tables for data-heavy displays
- Forms for interactions
- Flexbox/Grid layouts
- Mobile-friendly but not mobile-first

## Data Model

### Core Entities

#### Game
- `game_id`: Unique identifier
- `created_at`: Timestamp
- `current_turn`: Turn number
- `credits`: Global credits (or per-player)

#### Planet
- `planet_id`: Unique identifier
- `name`: Generated name
- `planet_type`: Enum (Terrestrial, Desert, Ice, etc.)
- `atmosphere`: Enum
- `temperature`: Enum
- `gravity`: Float
- `size`: Enum
- `resource_richness`: JSON (resource type → richness level)
- `hazard_level`: Int (0-10)
- `discovered_at`: Timestamp
- `description`: Generated text

#### Colony
- `colony_id`: Unique identifier
- `planet_id`: Foreign key
- `name`: String
- `founded_at`: Timestamp
- `population`: Int
- `morale`: Float (0-100)
- `pollution_level`: Float

#### Building
- `building_id`: Unique identifier
- `colony_id`: Foreign key
- `building_type`: Enum
- `state`: Enum (UnderConstruction, Operational, Damaged, Shutdown)
- `construction_progress`: Int (0-100)
- `position_x`, `position_y`: Int (for future spatial layout)
- `created_at`: Timestamp

#### WormholeGate
- `gate_id`: Unique identifier
- `source_planet_id`: Foreign key
- `destination_planet_id`: Foreign key
- `state`: Enum (UnderConstruction, Active, Disabled)
- `construction_progress`: Int (0-100)
- `throughput_capacity`: Int
- `created_at`: Timestamp

#### Train
- `train_id`: Unique identifier
- `name`: String
- `train_type`: Enum (Freight, Passenger, Mixed)
- `size`: Enum
- `speed`: Int
- `capacity`: Int
- `current_planet_id`: Foreign key (nullable if in-transit)
- `state`: Enum (Idle, InTransit, Loading, Unloading)
- `purchased_at`: Timestamp

#### Route
- `route_id`: Unique identifier
- `name`: String
- `origin_planet_id`: Foreign key
- `destination_planet_id`: Foreign key
- `route_type`: Enum (PointToPoint, Circuit)
- `frequency`: Int (trains per turn)
- `active`: Boolean

#### TrainAssignment
- `assignment_id`: Unique identifier
- `train_id`: Foreign key
- `route_id`: Foreign key
- `assigned_at`: Timestamp

#### Market
- `market_id`: Unique identifier
- `planet_id`: Foreign key
- `resource_type`: Enum
- `supply`: Int
- `demand`: Int
- `price`: Float

#### ResourceStockpile
- `stockpile_id`: Unique identifier
- `colony_id`: Foreign key
- `resource_type`: Enum
- `quantity`: Int

### Event Store Schema

#### events Table
- `event_id`: Primary key (auto-increment)
- `game_id`: Foreign key
- `timestamp`: Timestamp (when event occurred)
- `turn_number`: Int
- `event_type`: String (discriminator)
- `event_data`: JSON (serialized event payload)

### Database Indexes
- `events` table: Index on `game_id`, `turn_number`
- `buildings` table: Index on `colony_id`
- `trains` table: Index on `current_planet_id`, `state`
- `routes` table: Index on `origin_planet_id`, `destination_planet_id`

## Events and Commands

### Event Types

**Colony Events:**
- `ColonyFounded`
- `BuildingConstructionStarted`
- `BuildingConstructionCompleted`
- `BuildingStateChanged`
- `ResourcesExtracted`
- `ResourcesConsumed`
- `ResourcesProduced`
- `PopulationChanged`
- `PowerGridUpdated`
- `PollutionLevelChanged`

**Wormhole Events:**
- `PlanetDiscovered`
- `GateConstructionStarted`
- `GateConstructionCompleted`
- `GateActivated`
- `GateDeactivated`

**Train Events:**
- `TrainPurchased`
- `TrainAssignedToRoute`
- `TrainDispatched`
- `TrainArrived`
- `CargoLoaded`
- `CargoUnloaded`
- `PassengersBoarded`
- `PassengersDisembarked`

**Economic Events:**
- `CreditsEarned`
- `CreditsSpent`
- `MarketPriceChanged`
- `TradeCompleted`

**Simulation Events:**
- `TurnAdvanced`
- `ResearchCompleted`

### Command Types

**Colony Commands:**
- `FoundColony`
- `ConstructBuilding`
- `CancelConstruction`
- `ChangeuildingState` (shutdown, activate, repair)
- `AllocateLabor`

**Wormhole Commands:**
- `ExploreForPlanets`
- `ConstructGate`
- `ActivateGate`
- `DeactivateGate`

**Train Commands:**
- `PurchaseTrain`
- `CreateRoute`
- `AssignTrainToRoute`
- `RemoveTrainFromRoute`
- `DispatchTrain`

**Simulation Commands:**
- `AdvanceTurn`
- `StartResearch`

## MVP Feature Set

### Phase 1: Foundation (Skeleton)
- ✅ Project structure
- ✅ Actix-web server setup
- ✅ SQLite database and schema
- ✅ Event sourcing infrastructure
- ✅ Basic routing and templates
- ✅ HTMX integration
- ✅ CSS foundation

### Phase 2: Basic Colony Screen (First Prototype)
- Colony view page
- Display colony stats (population, credits, resources)
- Building list display
- Simple build form (construct 2-3 building types)
- Resource extraction simulation
- Turn advancement button
- Event log display (debug)

### Phase 3: Expand Colony Features
- More building types (10+ types)
- Building states (construction, operational, damaged)
- Power grid simulation
- Population and labor
- Production chains (basic)

### Phase 4: Wormhole & Exploration
- Planet procedural generation
- Exploration facility and discovery
- Gate construction
- Network view (simple list/table)

### Phase 5: Train System
- Train entities and types
- Route creation
- Train assignment
- Basic movement simulation
- Cargo loading/unloading

### Phase 6: Economy
- Market system
- Dynamic pricing
- Trade route profits
- Economic dashboard

### Phase 7: Polish
- Visual improvements
- Graphs and charts
- Better UX
- Tutorial/onboarding
- Save/load UI

## Future Enhancements

- **Multiplayer**: Compete or cooperate with other players
- **Random Events**: Natural disasters, discoveries, etc.
- **Advanced Research**: Tech tree for unlocks
- **Planetary Combat**: Defend against threats
- **Diplomacy**: AI factions or player alliances
- **Advanced Train Mechanics**: Congestion, scheduling, accidents
- **Visual Map**: Actual 2D map of settlements
- **WebSockets**: Real-time updates instead of polling
- **Desktop App**: Bevy engine with ECS architecture
- **WASM**: Client-side simulation with Rust

## Performance Considerations

### Optimization Strategies
- **Event Snapshots**: Periodically save full state to avoid replaying thousands of events
- **Read Model Projections**: Maintain denormalized views for fast queries
- **Lazy Loading**: Only load visible data
- **Connection Pooling**: Reuse database connections
- **Caching**: Cache frequently accessed data (planets, building templates)
- **Batch Processing**: Process multiple events in single transaction

### Scalability
- Single-player game: Not a concern initially
- Event log growth: Implement pruning/archival after X turns
- Database size: Monitor and optimize as needed

## Testing Strategy

### Unit Tests
- Domain logic (commands, events, entities)
- Business rules validation
- Production chain calculations
- Market pricing algorithms

### Integration Tests
- HTTP endpoints
- Database operations
- Event store functionality
- Full command → event → state flow

### Manual Testing
- UI interactions
- HTMX updates
- Multi-turn scenarios
- Edge cases (resource depletion, etc.)

## Development Workflow

### Iteration Process
1. Pick feature from ROADMAP.md
2. Design domain models
3. Define events
4. Implement commands
5. Write tests
6. Create web handlers
7. Build templates
8. Manual testing
9. Refine and polish

### Branching Strategy
- Feature branches for major additions
- Commit frequently
- Keep commits focused and atomic

### Documentation
- Update ROADMAP.md as features complete
- Document complex algorithms inline
- Keep DESIGN.md current with architecture changes

## Conclusion

This design provides a solid foundation for building a colony management and trade simulation game combining Outpost's settlement building with Commonwealth's wormhole train networks. The event-sourced architecture enables complex simulation with full auditability, while HTMX provides a smooth user experience without heavy frontend complexity.

The modular design allows for incremental development, starting with a basic colony screen and progressively adding systems (trains, markets, exploration) as the prototype matures.

**Next Steps:**
1. Review and approve this design document
2. Set up Rust project skeleton
3. Implement basic colony screen
4. Iterate on features per roadmap

---

**Document Version**: 1.0
**Last Updated**: 2025-12-30
**Author**: AI Assistant (Claude Code)
