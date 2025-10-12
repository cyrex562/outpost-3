# Implementation Plan (Godot 4 + C# 9, FP-leaning, event-sourced)

This expands the earlier “First Implementation Steps” into a concrete, Copilot-friendly, step-by-step plan you can execute in VS Code. Each phase includes:

* **Tasks** (checklist you can track),
* **Commands** (copy/paste shell steps),
* **Copilot Prompts** (paste into an editor comment to steer Copilot),
* **Acceptance Criteria** (definition of done).

---

## 0) Principles & Guardrails (once)

**Goals:** keep the Core pure, a single writer for state, message-passing only, deterministic time.

**Tasks**

* [ ] Core library has **zero Godot dependencies**.
* [ ] Use C# **records**, `with`, and avoid `null` in domain types.
* [ ] One simulation thread; UI <-> Sim via queues.
* [ ] Deterministic time: pass `Tick`/`Duration` as params; no `DateTime.UtcNow` in Core.

**Copilot Prompt**

```
Add a PRIMER.md explaining our architectural rules:
- engine-agnostic Core (.NET classlib)
- reducers: (State, Command|Tick) -> (State', Events[])
- append-only EventStore + snapshots
- Projections are rebuildable caches; UI only reads projections
- single-writer StateStore; message-passing between UI and Sim
```

---

## 1) Repo & Tooling Scaffold

**Tasks**

* [ ] Create root repo with solution and baseline folders.
* [ ] Add `.editorconfig`, `Directory.Build.props`, Roslyn analyzers, `dotnet-tools.json`.
* [ ] Decide test runner: **xUnit** + **FsCheck.Xunit** + **FluentAssertions**.

**Commands**

```bash
mkdir game && cd game
dotnet new sln -n Game
mkdir -p src/Game.Core src/Game.App src/Game.Persistence src/Game.Tests tools content
dotnet new classlib -n Game.Core -o src/Game.Core
dotnet new classlib -n Game.App -o src/Game.App
dotnet new classlib -n Game.Persistence -o src/Game.Persistence
dotnet new xunit    -n Game.Tests -o src/Game.Tests
dotnet sln Game.sln add src/*/*.csproj
dotnet add src/Game.App/Game.App.csproj reference src/Game.Core/Game.Core.csproj
dotnet add src/Game.Persistence/Game.Persistence.csproj reference src/Game.App/Game.App.csproj src/Game.Core/Game.Core.csproj
dotnet add src/Game.Tests/Game.Tests.csproj reference src/Game.Core/Game.Core.csproj src/Game.App/Game.App.csproj
dotnet add src/Game.Tests/Game.Tests.csproj package FsCheck.Xunit FluentAssertions
dotnet add src/Game.Persistence/Game.Persistence.csproj package Microsoft.Data.Sqlite LiteDB
dotnet add src/Game.App/Game.App.csproj package System.Threading.Channels
```

**Directory.Build.props (root)**

```xml
<Project>
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <LangVersion>9.0</LangVersion>
    <Nullable>enable</Nullable>
    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>
    <Deterministic>true</Deterministic>
    <WarningsAsErrors>nullable</WarningsAsErrors>
    <EnablePreviewFeatures>false</EnablePreviewFeatures>
  </PropertyGroup>
</Project>
```

**Acceptance Criteria**

* `dotnet build` succeeds.
* Analyzer warnings fail the build if you introduce null/unused issues.

---

## 2) Core Domain Skeleton (pure, engine-agnostic)

**Tasks**

* [ ] Define **State** root and key aggregates as **records**.
* [ ] Define **Commands** (intent) and **Events** (facts).
* [ ] Create **Systems** (reducers) stubs and **Rules** modules.

**File layout**

```
src/Game.Core/
  Domain/
    GameState.cs
    StarSystem.cs
    Colony.cs
    Building.cs
    Vehicle.cs
  Commands/
    AdvanceTime.cs
    LaunchProbe.cs
    BuildStructure.cs
  Events/
    TimeAdvanced.cs
    ProbeLaunched.cs
    BuildingConstructed.cs
  Systems/
    ExplorationSystem.cs
    EconomySystem.cs
    PopulationSystem.cs
  Rules/
    TravelRules.cs
    ProductionRules.cs
    MoraleRules.cs
  Simulation/
    Reducer.cs
    SimulationClock.cs
```

**Copilot Prompt**

```
/// Create immutable domain records for GameState, StarSystem, Colony, Sector, Vehicle.
/// Use small value objects where helpful (Id, Name, Coordinates, ResourceAmount).
/// Avoid nulls; expose static constructors that validate invariants.
```

**Acceptance Criteria**

* `GameState` composes other records; no engine refs; only value semantics.
* Commands/Events are plain types (no behavior, no I/O).

---

## 3) Reducer Base & First Feature Slice

**Tasks**

* [ ] Implement a **Reducer** helper pattern.
* [ ] Implement **ExplorationSystem** with one command (`LaunchProbe`) and time tick (`AdvanceTime`).
* [ ] Emit **ProbeLaunched** and **TimeAdvanced** events.

**Copilot Prompt**

```
/// Implement ExplorationSystem.Reduce for:
/// - (state, LaunchProbe cmd) -> (state', [ProbeLaunched])
/// - (state, AdvanceTime tick) -> (state', [TimeAdvanced, maybe other Events])
/// The reducer must be pure. Do not read global time; tick duration is in the command.
```

**Acceptance Criteria**

* Pure functions compile and pass unit tests that assert determinism (same inputs → same outputs).

---

## 4) Event Store (in-memory) + StateStore (single writer)

**Tasks**

* [ ] Define `IEventStore`, `ISnapshotStore` interfaces in **Game.App**.
* [ ] Implement `InMemoryEventStore`.
* [ ] Implement `StateStore` that:

  * Validates and runs reducers,
  * Appends Events,
  * Replaces current immutable `GameState`,
  * Publishes new events to subscribers.

**Interfaces (minimal)**

```csharp
public interface IEventStore {
  long Append(ReadOnlySpan<object> events);
  IAsyncEnumerable<object> ReadFrom(long offset, CancellationToken ct = default);
  long CurrentOffset { get; }
}

public interface ISnapshotStore {
  ValueTask SaveAsync(long offset, GameState state, CancellationToken ct = default);
  ValueTask<(long offset, GameState state)?> LoadLatestAsync(CancellationToken ct = default);
}
```

**Acceptance Criteria**

* `StateStore.Apply(Command)` returns `(Events[], NewState)`.
* `StateStore.Advance(TimeSpan dt)` generates `TimeAdvanced` events.
* Unit tests verify append offsets, ordering, and replay to same state.

---

## 5) Projection Engine + First Read Model

**Tasks**

* [ ] Define `IProjection<TView>`: `TView Fold(TView view, object @event)`.
* [ ] Implement `ProjectionEngine` that subscribes to EventStore and updates materialized views.
* [ ] Create **ColonyOverviewProjection** and **StarMapProjection**.

**Copilot Prompt**

```
/// Implement ProjectionEngine that:
/// - on startup, replays events from 0 into registered projections
/// - subscribes to new events from the EventStore (Channel or IAsyncEnumerable)
/// - exposes thread-safe getters for current view snapshots (copy semantics)
```

**Acceptance Criteria**

* Rebuild from zero produces the same views as incremental updates.
* Tests: feed a canned event stream → assert expected view DTOs.

---

## 6) Scheduler & CommandBus

**Tasks**

* [ ] Implement `CommandBus` (bounded Channel) to decouple UI producers from Sim consumer.
* [ ] Implement `Scheduler`:

  * Fixed timestep (e.g., 100ms) tick producer,
  * Drains command queue each tick,
  * Invokes `StateStore` then notifies `ProjectionEngine`.

**Acceptance Criteria**

* Black-box test: enqueue commands, run N ticks → projections reflect expected results.
* Backpressure: channel drops or blocks by policy (configurable).

---

## 7) Persistence Adapters (JSONL + Snapshots)

**Tasks**

* [ ] Implement `FileEventStore` (JSON Lines) with simple schema `{offset, ts, type, payload}`.
* [ ] Implement `FileSnapshotStore` (periodic snapshots every N events).
* [ ] CLI tool: `replay-cli` to open a save, replay, and print summary.

**Commands**

```bash
dotnet new console -n replay-cli -o tools/replay-cli
dotnet sln Game.sln add tools/replay-cli/replay-cli.csproj
dotnet add tools/replay-cli/replay-cli.csproj reference src/Game.App/Game.App.csproj src/Game.Core/Game.Core.csproj
```

**Copilot Prompt**

```
/// Implement FileEventStore using System.Text.Json source gen:
/// - Append: append serialized envelope per line
/// - ReadFrom: stream and deserialize lazily
/// Implement FileSnapshotStore: .snapshot files with offset + state blob.
```

**Acceptance Criteria**

* Save/Load works: restore (snapshot + tail events) equals pre-save state and views.

---

## 8) SQLite EventStore (optional but recommended)

**Tasks**

* [ ] `SQLiteEventStore` with tables `Events(offset PK, ts, type, payload BLOB)` and `Snapshots(offset, state BLOB)`.
* [ ] Parameterized commands; WAL mode; indices on `offset` and `type`.

**Acceptance Criteria**

* Same replay semantics as File store.
* Ad-hoc queries possible for debugging (e.g., count of events per type).

---

## 9) Godot Project Setup & Wiring (Adapter)

**Tasks**

* [ ] Create **GodotProject/** (Godot 4.x, .NET).
* [ ] Add a **Godot C#** project that references built DLLs of `Game.Core` and `Game.App`.
* [ ] Create a minimal **Main** scene that:

  * bootstraps `Scheduler`, `StateStore`, `ProjectionEngine`,
  * subscribes to projections,
  * binds a simple UI to a view.

**Notes**

* Build your .NET libs to a known `./bin` path; reference them from Godot’s C# project.
* In Godot, your scripts layer must **not** mutate state; it only sends Commands and renders Views.

**Acceptance Criteria**

* Press a button in Godot → sends `LaunchProbe` → projection updates and UI reflects change.

---

## 10) First Presenter & ViewModels

**Tasks**

* [ ] Define lightweight **ViewModel** DTOs inside Godot adapter (or a shared DTO lib).
* [ ] Implement a **Presenter** for Star System Map or Colony Overview.
* [ ] Use Godot Signals to capture UI intents and turn them into **Commands**.

**Copilot Prompt**

```
/// Create a StarSystemPresenter.cs that:
/// - Subscribes to StarMapProjection snapshots (read-only)
/// - Renders a list of discovered sectors
/// - On "Launch Probe" button click, enqueues LaunchProbe command with target sector
```

**Acceptance Criteria**

* All UI updates are driven by projection snapshots (pull or push).
* No UI class holds references to mutable game state.

---

## 11) Save/Load Coordinator

**Tasks**

* [ ] Implement `SaveLoadService` in **Game.App**:

  * `SaveAsync(label)` → snapshot + rotate event log,
  * `LoadAsync(path)` → restore & replay.
* [ ] Godot menu buttons: Save, Load, Continue.

**Acceptance Criteria**

* E2E: start → play → save → quit → load → state & views identical.

---

## 12) Mod/Content Loader (data-first rules)

**Tasks**

* [ ] Define JSON schemas for tech tree, buildings, events.
* [ ] Content loader converts JSON to domain DTOs (pure).
* [ ] Rules read their constants from content (pass as parameters).

**Acceptance Criteria**

* Changing a JSON value (e.g., building cost) changes outcomes without code changes.
* Tests pin JSON → outcome with golden files.

---

## 13) Testing Strategy (property-based + golden)

**Tasks**

* [ ] **Unit tests** for reducers and rules (determinism, invariants).
* [ ] **Property tests** (FsCheck): e.g., production never negative; total mass conserved.
* [ ] **Golden tests**: event streams → final projections; serialized golden JSON compared with baseline.

**Copilot Prompt**

```
/// Add FsCheck property tests:
/// - For any sequence of valid commands, total resource counts never drop below zero
/// - Replaying concatenated event segments equals replaying the whole stream
```

**Acceptance Criteria**

* `dotnet test` runs quickly with good coverage on Core.

---

## 14) Dev Ergonomics: Replay CLI & Content Packer

**Tasks**

* [ ] `replay-cli`: open save, list stats, export projections to JSON.
* [ ] `content-packer`: validate and bundle content JSON with checksums.

**Acceptance Criteria**

* Single command produces a JSON artifact of projections for debugging/analytics.

---

## 15) Performance & Profiling

**Tasks**

* [ ] Microbenchmarks for hot rules (BenchmarkDotNet).
* [ ] Load test: N ticks with M colonies; capture allocations and GC pressure.
* [ ] Profile projection rebuild cost; target “cold boot < 1s” for typical saves.

**Acceptance Criteria**

* Fixed target: 10k events replay < 250ms on dev machine.
* GC pauses do not exceed frame budget during UI interaction.

---

## 16) Packaging & CI

**Tasks**

* [ ] Add GitHub/GitLab CI to:

  * build libraries and tests,
  * export Godot project (headless),
  * attach artifacts (saves, content bundles).
* [ ] Add `justfile` or `Makefile` with common tasks.

**Acceptance Criteria**

* One command (or CI job) builds everything consistently on a clean runner.

---

## 17) Security & Stability Hooks (early)

**Tasks**

* [ ] Validate command inputs at boundary; reject out-of-domain values.
* [ ] Event versioning (add `v` field) and upcasters for future schema changes.
* [ ] Crash safety: projection rebuild on start; write-ahead append before state replacement.

**Acceptance Criteria**

* Corrupting the last event line does not prevent loading from previous snapshot.

---

## 18) Roadmap: Next Feature Slices (choose any)

* **Economy loop:** production/consumption, stockpiles, trade routes.
* **Population:** assignments, morale, incidents.
* **Research:** tech tree unlocks, prerequisites, costs.
* **Automation:** queued jobs, priorities, work shifts.

Each slice follows the same pattern:

1. Add Commands/Events → 2) Extend reducers/rules → 3) Extend projections → 4) Tests → 5) Minimal UI.

---

# Copilot-Ready “Story Cards” (paste into editor as comments)

Use these verbatim to nudge Copilot:

### Story 1 — InMemoryEventStore

```
TASK: Implement InMemoryEventStore.
GOAL: append-only, ordered, thread-safe, offset-based reads.
API:
- long Append(ReadOnlySpan<object> events)
- IAsyncEnumerable<object> ReadFrom(long offset, CancellationToken ct = default)
- long CurrentOffset { get; }
CONSTRAINTS:
- no blocking on reads; use Channel for fan-out
- offsets are contiguous starting at 0
TESTS:
- append 3 events -> offsets 0..2
- ReadFrom(1) yields last two events in order
```

### Story 2 — StateStore (single writer)

```
TASK: Implement StateStore.
GOAL: own current GameState; apply Commands and Ticks to produce new State + Events.
API:
- (IReadOnlyList<object> events, GameState newState) Apply(object command)
- (IReadOnlyList<object> events, GameState newState) Advance(TimeSpan dt)
DETAILS:
- use reducers in Game.Core
- append to EventStore; replace immutable state; publish events to ProjectionEngine
TESTS:
- deterministic: same inputs -> same outputs
- replay: events applied to initial state reconstruct current state
```

### Story 3 — ProjectionEngine

```
TASK: Implement ProjectionEngine with IProjection<TView>.
GOAL: rebuildable views; subscribe to new events.
API:
- Register(IProjection<TView> projection)
- Get<TView>() returns immutable copy
- RebuildFrom(offset)
TESTS:
- cold-start rebuild equals incremental updates
- concurrency: readers see consistent snapshots
```

### Story 4 — ExplorationSystem

```
TASK: Implement ExplorationSystem reducers.
COMMANDS: LaunchProbe(sectorId), AdvanceTime(dt)
EVENTS: ProbeLaunched, TimeAdvanced
RULES: TravelRules.ComputeTravelTime, risk is pure function
TESTS: launch + N ticks -> probe arrival event fired when eta <= 0
```

### Story 5 — FileEventStore + Snapshots

```
TASK: Implement FileEventStore and FileSnapshotStore.
FORMAT: JSONL for events; .snapshot with offset+state blob.
PERF: buffered writes; fsync on close.
TESTS: save->load roundtrip produces equal state and views
```

### Story 6 — Godot Presenter Stub

```
TASK: StarSystemPresenter (C#).
GOAL: render StarMapProjection; emit LaunchProbe on button click.
CONSTRAINTS: presenter never mutates GameState; it only enqueues commands.
```

---

# Final Project Structure (target)

```
game/
├─ Game.sln
├─ Directory.Build.props
├─ .editorconfig
├─ GodotProject/
│  ├─ project.godot
│  ├─ scenes/...
│  ├─ scripts/
│  │  ├─ App.cs               # bootstrap adapters (Scheduler, Stores)
│  │  ├─ Presenters/StarSystemPresenter.cs
│  │  └─ ViewModels/StarSystemVm.cs
│  └─ assets/...
├─ src/
│  ├─ Game.Core/              # pure domain
│  ├─ Game.App/               # StateStore, EventStore iface, Projections, Scheduler
│  ├─ Game.Persistence/       # FileEventStore, SQLiteEventStore, Snapshots
│  └─ Game.Tests/             # xUnit + FsCheck
├─ tools/
│  ├─ replay-cli/
│  └─ content-packer/
├─ content/
│  ├─ tech_tree.json
│  ├─ buildings.json
│  └─ events/...
└─ PRIMER.md
```

---

## What to implement first (short path to “it runs”)

1. **Core**: `GameState`, `LaunchProbe`, `AdvanceTime`, Exploration reducer + rules.
2. **Game.App**: `InMemoryEventStore`, `StateStore`, `ProjectionEngine`, `StarMapProjection`.
3. **Godot**: main scene + `StarSystemPresenter` + 1 button → `LaunchProbe`.
4. **Persistence**: `FileEventStore` + `FileSnapshotStore` + Save/Load buttons.
5. **Tests**: reducer determinism, event replay = state, projection rebuild = incremental.
