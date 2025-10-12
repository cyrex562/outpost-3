# Godot + C# Implementation Plan

## Project Setup Todo List

### ✅ Phase 0: Bootstrap (Do This First - Tonight)

#### Session 0.1: Create Project Structure (30 min)

- Create root directory outpost-game/
- Initialize git repository
- Create folder structure:

```
  outpost-game/
  ├── .gitignore
  ├── GodotProject/
  ├── docs/
  └── README.md
```

- Copy design docs into docs/
- Create basic .gitignore

#### Session 0.2: Godot Project Setup (45 min)

- Download Godot 4.3+ (.NET version)
- Create new Godot project in GodotProject/
- Configure project settings (name, resolution, etc.)
- Create basic folder structure in Godot:

```
  GodotProject/
  ├── Scenes/
  ├── Scripts/
  │   ├── Core/        # Pure C# game logic
  │   ├── UI/          # Godot-specific UI code
  │   └── App.cs       # Bootstrap
  └── Assets/
```

- Create Main.tscn scene with root node
- Test: Run empty project, see empty window


### 📋 Phase 1: Core Domain (Next 3-4 Sessions)

#### Session 1.1: Domain Types (60 min)

- Create Scripts/Core/Domain/GameState.cs
- Create Scripts/Core/Domain/StarSystem.cs
- Create Scripts/Core/Domain/CelestialBody.cs
- Add basic properties (IDs, names, time)
- Test: Instantiate objects in a test scene

#### Session 1.2: Commands & Events (60 min)

- Create Scripts/Core/Commands/Command.cs (base)
- Create Scripts/Core/Commands/AdvanceTime.cs
- Create Scripts/Core/Commands/LaunchProbe.cs
- Create Scripts/Core/Events/GameEvent.cs (base)
- Create Scripts/Core/Events/TimeAdvanced.cs
- Create Scripts/Core/Events/ProbeLaunched.cs
- Test: Serialize to JSON and back

#### Session 1.3: State Store (60 min)

- Create Scripts/App/StateStore.cs
- Implement ApplyCommand(Command cmd) method
- Implement simple reducer for AdvanceTime
- Add StateChanged signal
- Test: Apply command, check state updates

#### Session 1.4: First UI (60 min)

 Create Scenes/UI/GameHUD.tscn
 Add Label for game time
 Add Button "Advance 10 Hours"
 Create Scripts/UI/GameHUD.cs presenter
 Wire button to StateStore
 Test: Click button, see time advance


### 📋 Phase 2: Exploration System (Next 4-5 Sessions)

#### Session 2.1: Probe Domain (45 min)

- Add ProbeInFlight to domain
- Update GameState with probes list
- Add probe ID generation
- Test: Add probe to state

#### Session 2.2: Probe Launch Logic (60 min)

- Implement LaunchProbe reducer
- Calculate travel time (simple formula)
- Emit ProbeLaunched event
- Test: Launch probe, check it's in state

#### Session 2.3: Probe Arrival Logic (60 min)

- Update AdvanceTime reducer
- Check for probe arrivals
- Generate discovered system (simple)
- Emit SystemDiscovered event
- Remove arrived probes from state
- Test: Advance time past ETA, system appears

#### Session 2.4: Probe UI (60 min)

- Add "Launch Probe" button to HUD
- Add ItemList for probes in flight
- Show probe ID and ETA
- Update when StateChanged fires
- Test: Launch probe, see it in list

#### Session 2.5: System List UI (45 min)

- Add ItemList for discovered systems
- Show system name and ID
- Update when systems discovered
- Test: Full loop - launch → wait → discover

### 📋 Phase 3: Star System View (Next 3-4 Sessions)

#### Session 3.1: System Selection (60 min)

- Make system list clickable
- Store selected system ID in state
- Create "View System" button
- Test: Click system, button becomes active

#### Session 3.2: System Map Scene (60 min)

- Create Scenes/StarSystemMap.tscn
- Add back button
- Show selected system name
- List celestial bodies
- Test: Navigate to system view and back