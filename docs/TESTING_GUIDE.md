# Testing Guide for Outpost3

## Overview

Outpost3 uses a dual testing approach:
1. **xUnit** - Standard .NET testing for CI/CD and command-line execution
2. **GdUnit4** - Godot-integrated testing framework with VSTest adapter support

## Directory Structure

```
Tests/
├── README.md                          # This file
├── Outpost3.Tests.csproj             # Test project file (includes both frameworks)
├── FileEventStoreTests.cs            # xUnit tests for FileEventStore
├── EventStoreWorkflowTests.cs        # xUnit workflow tests
└── GdUnit/
    ├── FileEventStoreGdTests.cs      # GdUnit4 tests for FileEventStore
    └── EventStoreWorkflowGdTests.cs  # GdUnit4 workflow tests
```

## Running Tests

### Command Line (All Tests)

```powershell
# From repository root
dotnet test

# With verbose output
dotnet test -v detailed

# Run only xUnit tests
dotnet test --filter "FullyQualifiedName!~GdUnit"

# Run only GdUnit4 tests
dotnet test --filter "FullyQualifiedName~GdUnit"
```

### Visual Studio Code

1. Install the C# extension
2. Use the Test Explorer
3. Click the play button next to any test

### From Godot Editor (GdUnit4 only)

1. Open the Godot project
2. Install GdUnit4 addon from Asset Library (if not already installed)
3. Open the GdUnit4 panel (bottom of editor)
4. Run tests from the GdUnit4 inspector

## Writing New Tests

### For Core Domain Logic (Reducers, Systems)

Use **xUnit** for pure function tests:

```csharp
[Fact]
public void Reducer_AppliesCommand_ProducesCorrectEvents()
{
    // Arrange
    var initialState = new GameState();
    var command = new LaunchProbe(...);
    
    // Act
    var (newState, events) = ProbeSystem.Apply(initialState, command);
    
    // Assert
    Assert.Equal(expected, newState);
    Assert.Contains(events, e => e is ProbeLaunched);
}
```

### For Godot-Specific Features

Use **GdUnit4** when testing needs:
- Scene interaction
- Node manipulation
- Godot-specific APIs

```csharp
[TestSuite]
public class MyGodotFeatureTests
{
    [TestCase]
    public void NodeInteraction_BehavesCorrectly()
    {
        // Arrange
        var scene = ...;
        
        // Act & Assert
        Assertions.AssertThat(scene.GetNode("MyNode")).IsNotNull();
    }
}
```

## Test Naming Conventions

Follow the pattern: `MethodName_Scenario_ExpectedBehavior`

Examples:
- `Append_SingleEvent_WritesToFile`
- `ReadFrom_ValidOffset_ReturnsCorrectEvents`
- `Constructor_ExistingFile_RestoresOffset`

## Testing Principles

### 1. Pure Functions
```csharp
// Good: Pure reducer test
var (state, events) = Reducer.Apply(initialState, command);
Assert.Equal(expectedState, state);

// Bad: Mutating shared state
GlobalState.Update(command); // ❌ No side effects!
```

### 2. Deterministic Time
```csharp
// Good: Explicit time
var command = new AdvanceTime(1000f);

// Bad: Hidden time source
var command = new AdvanceTime(DateTime.UtcNow); // ❌ Not in Core!
```

### 3. Event Sourcing Verification
```csharp
// Verify events can reconstruct state
var events = eventStore.ReadFrom(0);
var rebuiltState = events.Aggregate(GameState.Empty, Reducer.Apply);
Assert.Equal(currentState, rebuiltState);
```

### 4. Isolation
```csharp
// Each test is isolated with setup/teardown
[Before] / [After]  (GdUnit4)
IDisposable pattern (xUnit)
```

## Assert Patterns

### xUnit Assertions
```csharp
Assert.Equal(expected, actual);
Assert.True(condition);
Assert.NotNull(value);
Assert.Contains(item, collection);
Assert.Throws<Exception>(() => action());
```

### GdUnit4 Assertions
```csharp
Assertions.AssertThat(value).IsEqual(expected);
Assertions.AssertThat(condition).IsTrue();
Assertions.AssertThat(value).IsNotNull();
Assertions.AssertThat(collection).Contains(item);
Assertions.AssertThrows<Exception>(() => action());
```

## Testing Event Store

Event store tests verify:
1. **Persistence**: Events are written to disk correctly
2. **Replay**: Events can be read back in order
3. **Offset management**: Sequential offsets are maintained
4. **Error handling**: Corrupted data throws appropriate exceptions

Example workflow test:
```csharp
[TestCase]
public void CompleteJourney_AllEventsPersistedCorrectly()
{
    var events = new[] {
        new ShipDepartedEvent(...),
        new MechanicalFailureEvent(...),
        new ShipArrivedEvent(...)
    };
    
    eventStore.Append(events);
    
    var stored = eventStore.ReadFrom(0).ToList();
    Assert.Equal(events.Length, stored.Count);
    // Verify each event preserved correctly
}
```

## Continuous Integration

Tests run automatically on:
- Pull requests
- Commits to main branch
- Scheduled nightly builds

CI uses **xUnit** tests via `dotnet test` command.

## Property-Based Testing (Future)

Per architecture docs, we should add FsCheck for property testing:

```csharp
// Future: Property-based test example
[Property]
public Property EventReplay_AlwaysProducesSameState(List<GameEvent> events)
{
    var state1 = Replay(events);
    var state2 = Replay(events);
    return (state1 == state2).ToProperty();
}
```

## Test Coverage Goals

- **Core reducers**: 100% coverage
- **Domain logic**: 90%+ coverage
- **Event persistence**: All happy & error paths
- **UI Presenters**: Behavior verification (command emission, projection subscription)

## Common Issues

### Issue: Tests can't find Godot types
**Solution**: Ensure `Outpost3.csproj` is referenced in test project.

### Issue: GdUnit4 tests don't appear in Test Explorer
**Solution**: Restore NuGet packages: `dotnet restore Tests/Outpost3.Tests.csproj`

### Issue: File already in use errors
**Solution**: Ensure proper cleanup in `[After]` / `Dispose()` methods.

## Resources

- [xUnit Documentation](https://xunit.net/)
- [GdUnit4 Documentation](https://mikeschulze.github.io/gdUnit4/)
- [GdUnit4 C# API](https://github.com/MikeSchulze/gdUnit4Net)
- Project Architecture: `docs/05_architecture.md`
- Implementation Plan: `docs/06_implementation_plan_1.md`
