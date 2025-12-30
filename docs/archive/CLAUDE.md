# CLAUDE.md - AI Assistant Guide for Outpost 3

## Project Overview

**Outpost 3** is a spiritual successor to the original Outpost game, built with Godot 4.5 and C#, featuring 2D graphics. This is an early prototype focused on fleshing out all features and screens to develop basic gameplay mechanics.

### Project Identity
- **Name**: Outpost 3
- **Description**: Re-imagining of the Outpost game
- **Version**: 0.1.0
- **Engine**: Godot 4.5 with C# scripting
- **Status**: Early prototype - active feature development

## Project Structure

```
outpost-3/
├── godot-project/          # Main Godot project directory
│   ├── scripts/
│   │   ├── Core/          # Core game logic and domain
│   │   │   ├── Commands/  # Command pattern implementations
│   │   │   ├── Domain/    # Domain models and value types
│   │   │   ├── Events/    # Event sourcing events
│   │   │   └── Services/  # Core services
│   │   ├── Services/      # Application services
│   │   └── UI/            # User interface components
│   ├── scenes/            # Godot scene files
│   ├── Autoload/          # Auto-loaded singletons (GameServices)
│   └── project.godot      # Godot project configuration
├── Tests/                 # GDUnit4 test suite
├── docs/                  # Project documentation
├── data/                  # Game data files
└── bin/                   # Build outputs
```

## Architecture & Key Patterns

### 1. Event Sourcing Architecture
This project uses **event sourcing** as a core architectural pattern. All state changes are captured as immutable events.

**Key Components:**
- **Events** ([scripts/Core/Events/](godot-project/scripts/Core/Events/)): Immutable records of state changes
  - `GameEvent`: Base class for all events
  - `GalaxyInitialized`, `AnomalyDetectedEvent`, etc.
  - Events are stored in an event store for replay and state reconstruction

- **Commands** ([scripts/Core/Commands/](godot-project/scripts/Core/Commands/)): Encapsulated actions that generate events
  - `ICommand`: Command interface
  - `InitializeGalaxy`, `LaunchProbe`, `AdvanceTime`, etc.
  - Commands validate input and produce events

- **Domain Models** ([scripts/Core/Domain/](godot-project/scripts/Core/Domain/)): Core game entities
  - `GameState`: Current game state reconstructed from events
  - `StarSystem`, `ProbeInFlight`: Domain entities
  - `PhysicsConstants`, `ValueTypes`: Domain primitives

**When working with state changes:**
1. Create a command to encapsulate the action
2. Validate the command
3. Generate appropriate events
4. Store events in the event store
5. Apply events to update game state

### 2. Command Pattern
All user actions and game logic mutations go through commands implementing `ICommand`. This provides:
- Undo/redo capability
- Clear separation of concerns
- Testable game logic
- Event generation consistency

### 3. Service Layer
Services handle cross-cutting concerns and coordinate between domain logic and UI:
- Located in [scripts/Services/](godot-project/scripts/Services/)
- Keep services stateless where possible
- Services should orchestrate commands and queries

### 4. Auto-loaded Singletons
- **GameServices** ([Autoload/GameServices.cs](godot-project/Autoload/GameServices.cs)): Central service container auto-loaded at startup
- Access via `GameServices` singleton throughout the application

## Testing

### GDUnit4 Test Framework
This project uses **GDUnit4** for testing, which provides a C#-friendly testing experience in Godot.

**Test Location**: [Tests/](Tests/) directory

**Running Tests:**
```powershell
# Run all GDUnit4 tests
.\run-gdunit4-tests.ps1

# Run tests through VSTest adapter
.\run-gdunit4-vstest.ps1

# Run with coverage
.\run-coverage.ps1
```

**Testing Guidelines:**
- Write tests for all commands and domain logic
- Test event generation and application
- Use GDUnit4 assertions and test attributes
- Keep tests isolated and deterministic
- Mock external dependencies

**Quick References:**
- [GDUNIT4_VSTEST_QUICK_REF.md](GDUNIT4_VSTEST_QUICK_REF.md)
- [COVERAGE_QUICK_REF.md](COVERAGE_QUICK_REF.md)
- [TESTING_SETUP_COMPLETE.md](TESTING_SETUP_COMPLETE.md)

## Domain Concepts

### Game Mechanics
The game simulates space exploration and colony management:

**Key Concepts:**
- **Star Systems**: Procedurally generated systems with celestial bodies
- **Probes**: Unmanned craft for exploration
- **Discovery Levels**: Progression of knowledge about systems
- **Anomalies**: Special discoveries and events
- **Time Advancement**: Turn-based simulation

### Physics & Simulation
- `PhysicsConstants`: Contains scientific constants for realistic simulation
- Time-based mechanics for probe travel, resource gathering, etc.

## Development Guidelines

### Code Organization
1. **Domain logic** goes in `scripts/Core/Domain/`
2. **Commands** for state mutations go in `scripts/Core/Commands/`
3. **Events** for state changes go in `scripts/Core/Events/`
4. **UI components** go in `scripts/UI/`
5. **Services** for orchestration go in `scripts/Services/`

### Naming Conventions
- Events: Past tense (e.g., `GalaxyInitialized`, `ProbeDespatched`)
- Commands: Imperative verbs (e.g., `LaunchProbe`, `AdvanceTime`)
- Value types: Clear, domain-specific names

### When Adding Features
1. **Define domain models** first (what entities and concepts exist?)
2. **Create events** that represent state changes
3. **Implement commands** to validate and generate events
4. **Write tests** for the command and event logic
5. **Update UI** to trigger commands and display state
6. **Document** any new mechanics in [docs/](docs/)

### File Paths & Godot Resources
- Use `res://` for Godot resource paths
- Scene references: `res://Scenes/SceneName.tscn`
- Script references: Relative to project root

## Common Tasks

### Adding a New Command
1. Create class implementing `ICommand` in `scripts/Core/Commands/`
2. Define validation logic
3. Generate appropriate event(s)
4. Write unit tests in `Tests/`
5. Wire up to UI or service layer

### Adding a New Event
1. Create event class inheriting from `GameEvent` in `scripts/Core/Events/`
2. Make it immutable (readonly fields/properties)
3. Implement event application logic in appropriate domain model
4. Update event store serialization if needed
5. Test event generation and application

### Adding UI Screens
1. Create scene in [scenes/](godot-project/scenes/)
2. Create C# script in [scripts/UI/](godot-project/scripts/UI/)
3. Connect to GameServices for state access
4. Use commands to trigger game logic
5. Subscribe to relevant events for updates

## Save System
The project includes quick save/load functionality:
- **Quick Save**: F5 key
- **Quick Load**: F9 key
- Save metadata in `scripts/Core/Domain/SaveMetadata.cs`

## Documentation
Additional documentation lives in [docs/](docs/):
- Game mechanics and design
- Development plans
- Technical architecture
- Roadmap and todos

## Dependencies & Setup
- **Godot 4.5** with C# support
- **.NET SDK** for C# compilation
- **GDUnit4** addon (included in `addons/`)

## Building & Running
1. Open `godot-project/project.godot` in Godot Editor
2. Ensure .NET SDK is configured
3. Build project (Build > Build Solution)
4. Run main scene: `res://Scenes/MainMenuScreen.tscn`

## AI Assistant Best Practices

### When Modifying Code:
1. **Preserve event sourcing patterns** - don't bypass the command/event architecture
2. **Write tests** for new commands and domain logic
3. **Keep domain logic pure** - separate from Godot-specific code where possible
4. **Update documentation** if adding significant features
5. **Follow the existing patterns** in commands, events, and domain models

### When Adding Features:
1. Review existing similar features for consistency
2. Consider how it fits into the event sourcing model
3. Think about save/load implications
4. Plan for testing from the start
5. Keep UI and logic separate

### When Debugging:
1. Check event store for unexpected events
2. Verify command validation logic
3. Look at test coverage for related features
4. Use Godot's debugger and print statements
5. Check [DebugSettings.cs](godot-project/scripts/Core/DebugSettings.cs) for debug options

## Notes
- This is an **early prototype** - expect frequent architectural changes
- The original "Harsh Realm" documentation may reference older iterations
- Focus is on rapid prototyping and gameplay iteration
- Code quality matters, but perfect is the enemy of done

## Questions or Issues?
Refer to existing documentation in [docs/](docs/) or review test examples in [Tests/](Tests/) for patterns and conventions.
