# AGENTS.md - AI Agent Guide for Outpost 3

## Project Overview

**Outpost 3** is a spiritual successor to the original Outpost game, built with Godot 4.5 and C#, featuring 2D graphics. This is an early prototype focused on fleshing out all features and screens to develop basic gameplay mechanics.

### Project Identity
- **Name**: Outpost 3
- **Description**: Re-imagining of the Outpost game
- **Version**: 0.1.0
- **Engine**: Godot 4.5 with C# scripting
- **Status**: Early prototype - active feature development
- **Repository**: outpost-3

## Quick Start for AI Agents

### Project Structure
```
outpost-3/
├── godot-project/          # Main Godot project
│   ├── scripts/Core/      # Event-sourced game logic
│   ├── scripts/Services/  # Application services
│   ├── scripts/UI/        # User interface
│   └── scenes/            # Godot scenes
├── Tests/                 # GDUnit4 tests
├── docs/                  # Documentation
└── data/                  # Game data
```

### Key Files to Understand
1. `godot-project/project.godot` - Godot project configuration
2. `godot-project/Autoload/GameServices.cs` - Central service container
3. `godot-project/scripts/Core/Domain/GameState.cs` - Core game state
4. `godot-project/scripts/Core/Events/GameEvent.cs` - Event base class
5. `godot-project/scripts/Core/Commands/ICommand.cs` - Command interface

## Architecture Overview

### Event Sourcing Architecture 🎯
This is the **most important** pattern to understand. All state changes flow through:

```
User Action → Command → Validation → Event → Event Store → State Update
```

**Critical Rules:**
- ✅ **DO**: Use commands for all state mutations
- ✅ **DO**: Make events immutable
- ✅ **DO**: Store events for replay capability
- ❌ **DON'T**: Directly modify game state
- ❌ **DON'T**: Bypass the command/event pattern

### Directory Structure

#### `scripts/Core/` - Pure Game Logic
- **Commands/**: Actions that can be taken (e.g., `LaunchProbe`, `AdvanceTime`)
- **Events/**: Things that happened (e.g., `GalaxyInitialized`, `ProbeDespatched`)
- **Domain/**: Game entities and value types (e.g., `StarSystem`, `GameState`)
- **Services/**: Core business logic services

#### `scripts/Services/` - Application Layer
Orchestration between UI and core logic

#### `scripts/UI/` - Presentation Layer
Godot nodes and UI controllers

## Common Agent Tasks

### Task 1: Adding a New Feature
**Template Workflow:**
```
1. Identify domain concepts needed
2. Create domain models in scripts/Core/Domain/
3. Define events in scripts/Core/Events/
4. Implement commands in scripts/Core/Commands/
5. Write tests in Tests/
6. Create/update UI in scripts/UI/
7. Update documentation in docs/
```

**Example: Adding Resource Collection**
1. Domain: `ResourceDeposit`, `ResourceType` (in Domain/)
2. Event: `ResourceCollected` (in Events/)
3. Command: `CollectResource` (in Commands/)
4. Test: `CollectResourceTests` (in Tests/)
5. UI: Update resource display panel

### Task 2: Debugging Issues
**Investigation Checklist:**
```
□ Check event store for unexpected events
□ Verify command validation logic
□ Review test coverage
□ Check GameServices initialization
□ Look for Godot lifecycle issues (_Ready, _Process)
□ Review DebugSettings.cs configuration
```

### Task 3: Writing Tests
**GDUnit4 Test Pattern:**
```csharp
[TestSuite]
public class MyFeatureTests
{
    [TestCase]
    public void ShouldDoSomething()
    {
        // Arrange
        var command = new MyCommand();

        // Act
        var result = command.Execute();

        // Assert
        Assert.That(result).IsNotNull();
    }
}
```

**Run tests:**
```powershell
.\run-gdunit4-tests.ps1
```

### Task 4: Modifying Existing Code
**Before Making Changes:**
1. Read related tests to understand behavior
2. Check for existing similar patterns
3. Verify event sourcing flow won't be broken
4. Consider backward compatibility for saves

**After Making Changes:**
1. Run existing tests
2. Add new tests for new behavior
3. Update relevant documentation
4. Check for compilation errors

## Agent-Specific Guidelines

### For Code Generation Agents
1. **Always generate complete, compilable code**
2. **Include necessary using statements**
3. **Follow C# and Godot naming conventions**
4. **Add XML documentation comments**
5. **Handle edge cases and null checks**
6. **Generate corresponding tests**

### For Code Review Agents
1. **Verify event sourcing pattern compliance**
2. **Check test coverage**
3. **Validate C# best practices**
4. **Ensure Godot integration is correct**
5. **Look for potential performance issues**
6. **Verify documentation updates**

### For Testing Agents
1. **Use GDUnit4 framework**
2. **Test commands and event generation**
3. **Verify event application to state**
4. **Test edge cases and validation**
5. **Ensure test isolation**
6. **Mock external dependencies**

### For Documentation Agents
1. **Update docs/ for new mechanics**
2. **Document architectural decisions**
3. **Keep README files current**
4. **Add code comments for complex logic**
5. **Update this file when patterns change**

### For Refactoring Agents
1. **Preserve event sourcing architecture**
2. **Maintain backward compatibility**
3. **Update all affected tests**
4. **Run full test suite**
5. **Document breaking changes**

## Technology Stack

### Godot 4.5 + C#
- **Engine**: Godot 4.5
- **Language**: C# (.NET)
- **Scripting**: C# scripts attached to nodes
- **Scenes**: .tscn files (text format)
- **Resources**: res:// path prefix

### Key Godot Concepts
- **Nodes**: Building blocks of scenes
- **Signals**: Event system for UI updates
- **Autoload**: Globally accessible singletons
- **Scenes**: Reusable node hierarchies
- **Resources**: Data containers

### C# Patterns Used
- Event Sourcing
- Command Pattern
- Repository Pattern (Event Store)
- Service Locator (GameServices)
- Value Objects (Domain types)

## Testing Framework

### GDUnit4
- **Type**: xUnit-style testing for Godot C#
- **Location**: Tests/ directory
- **Runner**: Multiple options (see scripts)
- **Coverage**: Supported via run-coverage.ps1

### Test Organization
```
Tests/
├── Core/
│   ├── Commands/     # Command tests
│   ├── Events/       # Event tests
│   └── Domain/       # Domain model tests
└── Services/         # Service tests
```

## Domain Knowledge

### Game Concepts
- **Star Systems**: Procedurally generated solar systems
- **Probes**: Unmanned exploration craft
- **Discovery**: Progressive revelation of system details
- **Anomalies**: Special events and discoveries
- **Resources**: Materials for construction and survival
- **Colonies**: Player settlements
- **Time**: Turn-based simulation

### Physics Simulation
- Orbital mechanics
- Probe travel time
- Resource extraction rates
- Scientific constants in `PhysicsConstants.cs`

## Code Patterns

### Creating a Command
```csharp
public class MyCommand : ICommand
{
    public bool CanExecute(GameState state)
    {
        // Validation logic
        return true;
    }

    public IEnumerable<GameEvent> Execute(GameState state)
    {
        // Generate events
        yield return new MyEvent(/*params*/);
    }
}
```

### Creating an Event
```csharp
public class MyEvent : GameEvent
{
    public readonly string SomeData;

    public MyEvent(string someData)
    {
        SomeData = someData;
    }

    public override void Apply(GameState state)
    {
        // Update state
    }
}
```

### Creating a UI Component
```csharp
public partial class MyPanel : Control
{
    public override void _Ready()
    {
        // Initialize
        var gameServices = GetNode<GameServices>("/root/GameServices");
        // Subscribe to events, setup UI
    }
}
```

## File Naming Conventions
- **Commands**: `VerbNoun.cs` (e.g., `LaunchProbe.cs`)
- **Events**: `NounVerbed.cs` (e.g., `ProbeDespatched.cs`)
- **Domain**: `Noun.cs` (e.g., `StarSystem.cs`)
- **UI**: `NounPanel.cs` or `NounScreen.cs`
- **Tests**: `ClassNameTests.cs`

## Build & Run Commands

### PowerShell Scripts
```powershell
# Run tests
.\run-gdunit4-tests.ps1

# Run tests with VSTest
.\run-gdunit4-vstest.ps1

# Run with coverage
.\run-coverage.ps1

# Run all tests
.\run-tests.ps1
```

### Godot Editor
1. Open `godot-project/project.godot`
2. Build → Build Solution
3. Run → Play (F5)
4. Quick Save → F5 (in-game)
5. Quick Load → F9 (in-game)

## Save/Load System
- **Format**: Event store replay
- **Location**: Player data directory
- **Quick Save**: F5 key
- **Quick Load**: F9 key
- **Metadata**: `SaveMetadata.cs`

## Documentation Resources
- `docs/` - Game design and mechanics
- `COVERAGE_QUICK_REF.md` - Coverage tools guide
- `GDUNIT4_VSTEST_QUICK_REF.md` - Testing guide
- `TESTING_SETUP_COMPLETE.md` - Test setup info

## Agent Best Practices

### DO ✅
- Follow the event sourcing pattern religiously
- Write tests for all new code
- Keep domain logic pure and testable
- Use meaningful, domain-driven names
- Document complex logic
- Consider save/load compatibility
- Update documentation when adding features
- Run tests before committing

### DON'T ❌
- Bypass command/event architecture
- Modify state directly without events
- Mix UI code with domain logic
- Ignore test failures
- Create circular dependencies
- Hard-code game data in scripts
- Skip validation in commands
- Forget to handle edge cases

## Common Pitfalls

1. **Forgetting to apply events to state** - Events must update GameState
2. **Mutable events** - Events should be immutable (readonly fields)
3. **State in commands** - Commands should be stateless
4. **Direct state mutation** - Always go through events
5. **Missing tests** - All commands need tests
6. **Godot lifecycle issues** - Remember _Ready() initialization order
7. **Path issues** - Use res:// for Godot resources

## Performance Considerations
- Event store can grow large - consider snapshots
- Avoid expensive operations in _Process()
- Cache frequently accessed data
- Use object pooling for frequently created objects
- Profile before optimizing

## Security & Safety
- No user-generated code execution
- Validate all command inputs
- Sanitize save file data
- Handle file I/O errors gracefully

## Version Information
- **Godot**: 4.5
- **.NET**: 6+
- **C#**: Latest features available
- **GDUnit4**: Included in addons/

## Questions or Issues?
1. Check `docs/` for design documentation
2. Review `Tests/` for usage examples
3. Look at existing similar features
4. Check Godot documentation for engine-specific issues

## Project Status
**Current Phase**: Early prototype
**Focus**: Feature completeness and basic gameplay
**Stability**: Expect architectural changes
**Priority**: Rapid iteration over perfection

---

**Remember**: This is event-sourced architecture. Every state change is an event. Every event tells a story. Keep the story consistent and testable.
