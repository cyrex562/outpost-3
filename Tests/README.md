# Outpost 3 - Event System Tests

This directory contains unit and integration tests for the Event Log System (Task 1.3).

## Test Structure

### Unit Tests

**FileEventStoreTests.cs** - Tests for the FileEventStore implementation:
- `Append_SingleEvent_WritesToFile` - Verifies single event persistence
- `Append_MultipleEvents_AssignsSequentialOffsets` - Tests offset assignment
- `ReadFrom_ValidOffset_ReturnsCorrectEvents` - Tests reading from specific offset
- `ReadFrom_ZeroOffset_ReturnsAllEvents` - Tests reading all events
- `ReadFrom_OffsetBeyondEnd_ReturnsEmpty` - Tests edge case handling
- `Constructor_ExistingFile_RestoresOffset` - Tests state restoration
- `Count_ReturnsCorrectCount` - Tests event counting
- `Append_CorruptedLine_ThrowsException` - Tests error handling
- `ReadFrom_PreservesEventTypes` - Tests polymorphic serialization
- `Append_UpdatesCurrentOffset` - Tests offset management

### Integration Tests

**JourneyLogIntegrationTests.cs** - End-to-end tests for Journey Log workflow:
- `JourneyLog_FullWorkflow_DisplaysEventsCorrectly` - Full journey simulation
- `JourneyLog_FilterByType_ReturnsOnlyMatchingEvents` - Event filtering
- `JourneyLog_FilterBySeverity_ReturnsCorrectEvents` - Severity filtering
- `JourneyLog_ExportJson_CreatesValidFile` - JSON export validation
- `JourneyLog_EventsPreserveGameTime` - Time preservation test

## Running Tests

### Prerequisites

Ensure you have the .NET 8.0 SDK installed:
```bash
dotnet --version  # Should show 8.0.x or higher
```

### Run All Tests

From the `Tests` directory:
```powershell
dotnet test
```

### Run Specific Test Class

```powershell
dotnet test --filter "FullyQualifiedName~FileEventStoreTests"
dotnet test --filter "FullyQualifiedName~JourneyLogIntegrationTests"
```

### Run Single Test

```powershell
dotnet test --filter "FullyQualifiedName~Append_SingleEvent_WritesToFile"
```

### Run with Detailed Output

```powershell
dotnet test --logger "console;verbosity=detailed"
```

### Generate Code Coverage

```powershell
dotnet test /p:CollectCoverage=true
```

## Test Coverage

The tests cover:

✅ **Event Persistence**:
- Single and multiple event appending
- Sequential offset assignment
- File format validation

✅ **Event Reading**:
- Reading from specific offsets
- Reading all events
- Lazy enumeration
- Event type preservation

✅ **State Management**:
- CurrentOffset tracking
- Event count accuracy
- State restoration from file

✅ **Error Handling**:
- Corrupted file detection
- Invalid data handling
- Exception throwing

✅ **Integration Scenarios**:
- Complete journey workflow
- Event filtering by type and severity
- JSON export functionality
- Game time preservation

## Notes

- Tests use temporary files that are automatically cleaned up
- Each test is isolated with its own file paths
- Tests can run in parallel (xUnit default)
- Integration tests simulate real gameplay scenarios

## Future Enhancements

- Add YAML export tests (requires YamlDotNet package)
- Add UI component tests (requires Godot test framework)
- Add performance tests for large event stores (1000+ events)
- Add concurrency tests for multi-reader scenarios
