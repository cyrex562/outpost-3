# Architecture
## Major Architectural Components
### Core
- Engine-agnostic C#
- Pure simulation and data model, no Godot references
- **Domain Model (records)**: Immutable records for Entities/Value Objects (GameSession, StarSystem, CelestialBody, Sector, Colony, Building, Vehicle, InventoryItem, ResearchProject...)
- **Commands (API)**: intent messages from UI/AI (e.g. LaunchProbe, AssignCrew, BuildStructure, AdvaceTime(dt))
    - All commands are plain types; no side effects
- **Reducers/Systems**: Pure functions `(State, Command) -> (State', Events[])` and `(State, Tick) -> (State', Events[])` for time-step updates
    - **Examples**: Exploration, Voyage, Colonization, Economy, Automation, Population, Research, Governance
- **Event Types**: append-only facts emitted by reducers (e.g. ProbeLaunched, SectoreExplored, BuildingConstructed, MoraleChanged)
- **Rules/Policies**: deterministic math and constraints; encapsulated calculators used by reducers (landing success, production throughput, morale deltas, logistics travel time, etc.)
- **Simulation Clock**: Pure tick generator that advances time and feeds systems in a fixed order.
- **Projections**: Read-models built from replaying Events into denormalized views for UI (e.g. ColonyOverview, TradeNetworkGraph, Analytics time series)
> The only mutable thing in the whole Core is a local variable inside a reducer method while building the next immutable state
### State & Persistance
- Purpose: own the "one true" simulation state and the event log; expose read-side projections.
- **StateStore**: The only owner of the current state (immutable snapshot) and the EventLog. Provides transactional methods: `Apply(Command)` and `Advance(dt)`
- **EventStore (append-only)**: in-memory by default; optionally persisted:
    - **Option A (Simple)**: JSONL append-only file + periodic snapshots (fast, trivial).
    - **Option B (SQLite)**: `Events(id, ts, type, payload)` + `Snapshots(id, ts, state_blob)` using `Microsoft.Date.Sqlite` (works in-memory via `DataSource=:memory`) or a file path.
    - **Option C (LiteDB)**: pure-managed embedded document store (no native deps)
- **Projection Engine**: subscribes to the EventStore and materializes read models on demand or continuously
- Functional pattern: Commands mutate nothing directly; they produce Events, and State is derived from replaying Events (plus occaisonal snapshots for load speed)
### Orchestrator
- Purpose: glue between core and adapters
- Scheduler/Jobs: runs the fixed-timestep loop. queues Commands, batches projection updates.
- Save/Load Coordinator: triggers snapshotting and restores `(StateSnapshot + tailEvents)`
- **Mod Loader**: loads extra rule modules/JSON data packs (tech tree, items, events)
### Adapters (I/O)
- Purpose: everything side-effectful lives here
- **Godot UI Adapter**: translates input -> Commands, subscribes to Projections, renders. Uses Godot Signals to request Commands; never touches state directly.
- **Persistence Adapter**: SQLite/LiteDB/filesystem implementations of the EventStore/SnapshotStore
- **Telemetry/Debug Adapter**: exports logs/metrics for analytics, test fixtures, and repros.
- **Mod/Content Adapter**: reads JSON/YAML assets into domain DTOs.
### Godot Front-End
- Purpose: View + Controller; no business logic
- **ViewModels**: immutable DTOs fed by Projections; minimal mapping overhead
- **SceneGraph & UI**: scenes for system map, sector map, colony overview, modals, etc
- **Input Mapping**: maps to high-level commands
- **FX/Audio/Camera**: purely reactive to Projections and on-frame UI state.
## Data & Concurrency Model (No Shared State)
- **Single Writer**: only the `StateStore` mutates the "current snapshot" (by replacing it with anew immutable instance). All others read projections or `State` copies.
- **Message Passing**: UI raises `Command` via a thread-safe queue; the Orchestrator processes them on the simulation thread; generated `Event` flow to projections and `UI`
- **Projections as Caches**: read-optimized, cna be rebuilt form events at any time (crash-safe)
- **Threading**: keep one simulation thread; UI thread stays in Godot. Communicate via queues.
## In-Memory "DB" Choice
- You dont need a general purpose in-memory DB to be fast
- **Primary**: in-memory **EventStore** + **immutable State** in RAM -> O(1) access
- **Projections**: specialized in-memory indices (dictionaries/tries/graphs) per UI screen
- **Optional SQLite/LiteDB**: for persistence on disk and query tooling
    - **SQLite** **memory**: for tests/benchmarks; switch to file for saves
    - **LiteDB** is friction-free if you prefer document-style blobls and zero native deps.
- Gives you the speed of a K/V with the ergonomics of strongly typed projections

## Project Structure

```
game/
├─ GodotProject/                  # Godot 4.x project (editor opens here)
│  ├─ project.godot
│  ├─ scenes/
│  │  ├─ MainMenu.tscn
│  │  ├─ StarSystemMap.tscn
│  │  ├─ OrbitalView.tscn
│  │  ├─ SurfaceMap.tscn
│  │  ├─ ColonyOverview.tscn
│  │  ├─ Modals/
│  │  │  ├─ EventModal.tscn
│  │  │  ├─ SectorDetails.tscn
│  │  │  ├─ VehicleDetails.tscn
│  │  │  └─ BuildingDetails.tscn
│  ├─ ui/
│  │  ├─ Controls/                # custom controls, panels
│  │  ├─ Themes/
│  │  └─ Icons/
│  ├─ scripts/                    # thin adapter code only
│  │  ├─ App.gd                   # boot, DI wiring for adapters
│  │  ├─ ViewModels/              # DTOs fed by projections
│  │  │  ├─ ColonyVm.cs
│  │  │  ├─ StarSystemVm.cs
│  │  │  └─ AnalyticsVm.cs
│  │  ├─ Presenters/              # scene binders
│  │  │  ├─ StarSystemPresenter.cs
│  │  │  ├─ SurfaceMapPresenter.cs
│  │  │  └─ ColonyOverviewPresenter.cs
│  │  ├─ Input/
│  │  │  └─ InputMapper.cs        # maps input → Commands
│  │  └─ Signals/
│  │     └─ UiSignals.cs          # Godot signals emitted to adapters
│  ├─ assets/                     # textures, audio, fonts
│  └─ addons/                     # (optional) editor add-ons
│
├─ src/                           # .NET solution root for engine-agnostic code
│  ├─ Game.Core/                  # Pure functional domain + systems
│  │  ├─ Game.Core.csproj
│  │  ├─ Domain/                  # immutable records (entities/values)
│  │  │  ├─ GameSession.cs
│  │  │  ├─ StarSystem.cs
│  │  │  ├─ CelestialBody.cs
│  │  │  ├─ Sector.cs
│  │  │  ├─ Colony.cs
│  │  │  ├─ Building.cs
│  │  │  ├─ Vehicle.cs
│  │  │  └─ Economy.cs
│  │  ├─ Commands/
│  │  │  ├─ LaunchProbe.cs
│  │  │  ├─ EstablishColony.cs
│  │  │  ├─ BuildStructure.cs
│  │  │  └─ AdvanceTime.cs
│  │  ├─ Events/
│  │  │  ├─ ProbeLaunched.cs
│  │  │  ├─ SectorExplored.cs
│  │  │  ├─ BuildingConstructed.cs
│  │  │  └─ TimeAdvanced.cs
│  │  ├─ Systems/                 # pure reducers
│  │  │  ├─ ExplorationSystem.cs
│  │  │  ├─ VoyageSystem.cs
│  │  │  ├─ ColonizationSystem.cs
│  │  │  ├─ EconomySystem.cs
│  │  │  ├─ AutomationSystem.cs
│  │  │  ├─ PopulationSystem.cs
│  │  │  └─ ResearchSystem.cs
│  │  ├─ Projections/
│  │  │  ├─ ColonyOverviewProjection.cs
│  │  │  ├─ TradeNetworkProjection.cs
│  │  │  └─ AnalyticsProjection.cs
│  │  ├─ Rules/                   # math & formulas (pure)
│  │  │  ├─ LandingRules.cs
│  │  │  ├─ ProductionRules.cs
│  │  │  └─ MoraleRules.cs
│  │  └─ Simulation/
│  │     ├─ Reducer.cs            # base reducer helpers
│  │     └─ SimulationClock.cs
│  │
│  ├─ Game.App/                   # Orchestrator (headless friendly)
│  │  ├─ Game.App.csproj
│  │  ├─ StateStore.cs            # owns current State + applies Commands
│  │  ├─ EventStore.cs            # interface + in-memory impl
│  │  ├─ SnapshotStore.cs         # interface
│  │  ├─ ProjectionEngine.cs
│  │  ├─ CommandBus.cs
│  │  └─ Scheduler.cs
│  │
│  ├─ Game.Persistence/           # storage adapters
│  │  ├─ Game.Persistence.csproj
│  │  ├─ SQLiteEventStore.cs
│  │  ├─ FileEventStore.cs        # JSONL
│  │  └─ LiteDbEventStore.cs
│  │
│  └─ Game.Tests/                 # unit/property tests (Core only)
│     ├─ Game.Tests.csproj
│     ├─ ExplorationSystemTests.cs
│     ├─ EconomySystemTests.cs
│     └─ SerializationTests.cs
│
├─ tools/
│  ├─ content-packer/             # packs JSON/YAML data into bundles
│  └─ replay-cli/                 # headless event replay & profiling
│
├─ content/                       # human-editable data packs (mods later)
│  ├─ tech_tree.json
│  ├─ buildings.json
│  ├─ vehicles.json
│  └─ events/
│     ├─ voyage_events.json
│     └─ colony_events.json
│
└─ README.md

```

## Key Files

- `StateStore.cs` - the single writer. Applies commands by calling Core reducers, appends events, updates snapshot, notifies projection
- `EventStore` (interface) + `FileEventStore.cs` / `SQLiteEventStore.cs` - append-only event log with `Append`, `ReadFrom(offset`
- `ProjectionEngine.cs` - subscribes to new events and updates read models; supports full rebuild
- `*System.cs` - pure reducers that convert `(State, Command|Tick)` -> `(State', Events[])`
- `*Rules.cs` - math helpers (e.g. `ComputeLandingRisk(...)`); pure and testable.
- `*Projection.cs` - builds DTOs for UI (e.g. `ColonyOverviewVm`)
- `Presenters/*.cs` - Godot scene binders that render a ViewModel and emit commands via `CommandBus`.

## Godot <-> Core Flow

```
flowchart LR
  UI[Godot UI] -- Commands --> BUS[CommandBus]
  BUS --> SCHED[Scheduler]
  SCHED --> STORE[StateStore]
  STORE -->|append| EVT[EventStore]
  STORE -->|replace| STATE[(Immutable State)]
  EVT --> PROJ[Projection Engine]
  PROJ --> VMS[ViewModels / Projections]
  VMS --> UI

```

## Coding Practices

- **C# Records** + `with` for structural updates
- **No nulls** in domain; use option types (`OneOf`, custom `Option<T>`, or nullable with guards)
- **Pure methods** for all reducers/rules; inject constants/config as parameters
- **Deterministic time**: pass a clock/tick size into systems; no `DateTime.UtcNow` inside Core.
- **Property-base tests** for rules and reducers (FsCheck or similar for C#)
- **Serialization boundaries**: `Core` exposes DTOs; `Persistence` owns serialization format (JSON/Messageack)

## Storage Recommendations

- Start with FileEventStore (JSONL) + Snapshots every N events for immediate productivity
- Add SQLiteEventStore when you want
    - quick ad-hoc queries
    - robust file locking
    - easy save-file portability
- Keep in-memory mode for tests `SQLite: memory` or a pure in-mem `EventStore` impl.

