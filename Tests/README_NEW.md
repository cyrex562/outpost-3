# Outpost3 Test Suite

This directory contains unit tests for the Outpost3 Core domain logic using both **xUnit** and **GdUnit4** frameworks.

## Testing Frameworks

### xUnit (Primary for CI/CD)

- Located in root of `Tests/` directory
- Used for automated CI/CD pipelines
- Standard .NET testing framework
- Run with `dotnet test`

### GdUnit4 (Godot Integration)

- Located in `Tests/GdUnit/` directory
- C# testing framework specifically designed for Godot
- Can be run from within Godot Editor
- Supports VSTest integration for VS Code/Visual Studio/Rider
- Run with `dotnet test` or from Godot Editor

## Test Philosophy

Following the event-sourced architecture:

- **Pure function tests**: Test reducers and systems as pure `(State, Command) -> (State', Events[])` functions
- **Deterministic**: All tests use explicit time values, no hidden time sources
- **Replay-based**: Verify that event replay produces correct state
- **No side effects**: Tests don't mutate global state or use I/O except persistence tests

## Running Tests

### All Tests

```powershell
# From repository root or Tests directory
dotnet test

# With detailed output
dotnet test --logger "console;verbosity=detailed"
```

### Filtered Tests

```powershell
# Run only xUnit tests
dotnet test --filter "FullyQualifiedName!~GdUnit"

# Run only GdUnit4 tests
dotnet test --filter "FullyQualifiedName~GdUnit"

# Run specific test class
dotnet test --filter "FullyQualifiedName~FileEventStoreTests"
```

### From Godot Editor (GdUnit4 only)

1. Open the Godot project
2. Install GdUnit4 addon from Asset Library (if not already installed)
3. Enable the GdUnit4 plugin in Project Settings
4. Open the GdUnit4 panel (bottom of editor)
5. Run tests from the GdUnit4 inspector

## Test Structure

### xUnit Tests (Original)

- `FileEventStoreTests.cs` - Unit tests for event persistence
- `EventStoreWorkflowTests.cs` - Integration tests for complete workflows

### GdUnit4 Tests (New)

- `GdUnit/FileEventStoreGdTests.cs` - Same tests as xUnit version, using GdUnit4 API
- `GdUnit/EventStoreWorkflowGdTests.cs` - Workflow tests using GdUnit4 API

Both test suites cover the same scenarios but use different assertion APIs.

## Test Coverage

The tests cover:

✅ **Event Persistence**:

- Single and multiple event appending
- Sequential offset assignment
- File format validation

✅ **Event Reading**:

- Reading from specific offsets
- Reading all events
- Event type preservation

✅ **State Management**:

- CurrentOffset tracking
- Event count accuracy
- State restoration from file

✅ **Error Handling**:

- Corrupted file detection
- Exception throwing

✅ **Integration Scenarios**:

- Complete journey workflow
- Event filtering by type and severity
- JSON serialization
- Chronological ordering

## Writing New Tests

See `docs/TESTING_GUIDE.md` for complete guidance on:

- Test naming conventions
- Assertion patterns (xUnit vs GdUnit4)
- Testing principles for event-sourced systems
- Property-based testing (planned)

## CI/CD Integration

Tests run automatically on:

- Pull requests
- Commits to main branch
- Scheduled builds

CI uses xUnit tests via `dotnet test` command.

## Notes

- Tests use temporary files that are automatically cleaned up
- Each test is isolated with proper setup/teardown
- Both frameworks can coexist and run together
- GdUnit4 provides better integration with Godot-specific features when needed

## Future Enhancements

- Add FsCheck property-based tests for event replay determinism
- Add tests for UI Presenters (command emission, projection subscription)
- Add scene testing using GdUnit4's scene runner
- Add performance tests for large event stores (1000+ events)
