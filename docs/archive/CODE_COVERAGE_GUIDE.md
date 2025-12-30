# Code Coverage Guide for Outpost3

## Overview

This guide explains how to measure and improve code coverage for the Outpost3 project, covering both C# code and Godot scenes.

## Current Coverage Status (as of 2025-10-11)

### Overall Metrics
- **Line Coverage**: 5.6% (147/2580 lines)
- **Branch Coverage**: 3.6% (39/1082 branches)
- **Method Coverage**: 8.6% (19/220 methods)

### Well-Covered Components ✅
1. **Event Records** (100% coverage):
   - `GameEvent` base class
   - `ShipDepartedEvent`
   - `ShipArrivedEvent`
   - `MechanicalFailureEvent`
   - `SocialConflictEvent`

2. **Persistence Layer** (60%+ coverage):
   - `FileEventStore` (61.6%)
   - `GameEventJsonConverter` (65.1%)

### Components Needing Coverage ⚠️
- **UI Components** (0% coverage)
  - EventLogScreenPresenter
  - ShipJourneyLogPresenter
  - DebugEventPanel
  - GameHUD
  - JourneyEventEntry

- **Core Systems** (0% coverage)
  - StateStore
  - TimeSystem
  
- **Services** (0% coverage)
  - EventExporter

- **Domain Models** (0% coverage)
  - GameState
  - ProbeInFlight

## Running Code Coverage

### Quick Start

```powershell
# Run tests with coverage and generate HTML report
cd Tests
dotnet test --collect:"XPlat Code Coverage" --results-directory:../coverage --settings:coverlet.runsettings
cd ..
reportgenerator -reports:"coverage\**\coverage.cobertura.xml" -targetdir:"coverage\report" -reporttypes:"Html;HtmlSummary;Badges"

# View the report
start coverage\report\index.html
```

### Using the Coverage Script (Recommended)

A PowerShell script is provided for convenience:

```powershell
# Run coverage and generate reports (after enabling script execution)
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
.\run-coverage.ps1

# Or bypass execution policy for single run
powershell -ExecutionPolicy Bypass -File .\run-coverage.ps1

# Open HTML report automatically
powershell -ExecutionPolicy Bypass -File .\run-coverage.ps1 -OpenReport
```

## Understanding Coverage Reports

### Coverage Metrics Explained

1. **Line Coverage**: Percentage of code lines executed during tests
   - Target: 80%+ for core logic
   - Current: 5.6%

2. **Branch Coverage**: Percentage of decision branches (if/else, switch) tested
   - Target: 70%+ for core logic
   - Current: 3.6%

3. **Method Coverage**: Percentage of methods called during tests
   - Target: 75%+ for core logic
   - Current: 8.6%

### Coverage Tiers

- **🟢 Excellent**: 80%+ (Event records, persistence layer)
- **🟡 Good**: 60-79% (Converters, utilities)
- **🟠 Fair**: 40-59% (Domain models)
- **🔴 Needs Improvement**: < 40% (UI, services, systems)

## C# Code Coverage Strategy

### Priority 1: Core Domain Logic (Target: 85%+)
These components implement business rules and must be thoroughly tested:

```csharp
// Core/Domain/
- GameState.cs
- ProbeInFlight.cs

// Core/Events/
- All event classes (already at 100%!)

// Core/Persistence/
- FileEventStore.cs (currently 61.6%, target 85%+)
- GameEventJsonConverter.cs (currently 65.1%, target 85%+)
```

**How to Test**:
- Use xUnit for pure domain logic
- Focus on edge cases and validation
- Test all command/event combinations

### Priority 2: Systems & Services (Target: 70%+)

```csharp
// Core/Systems/
- TimeSystem.cs (currently 0%)

// Services/
- EventExporter.cs (currently 0%)
```

**How to Test**:
- Use GDUnit4 for Godot-integrated services
- Mock external dependencies
- Test state transitions

### Priority 3: UI Presenters (Target: 50-60%)

```csharp
// UI/
- EventLogScreenPresenter.cs
- ShipJourneyLogPresenter.cs
- DebugEventPanel.cs
```

**How to Test**:
- Use GDUnit4 for Godot node integration
- Focus on presenter logic, not UI rendering
- Test data binding and event handling
- Mock view components

## Godot Scene Coverage

### Current State
❌ **Godot scenes (.tscn files) cannot be directly measured by traditional code coverage tools.**

### Recommended Approach: Scene Testing Specifications

For each scene, create a test specification document that manually tracks:

#### 1. **Scene Test Matrix**

Create `Tests/GdUnit/SceneSpecs/` directory with files like:

```markdown
# EventLogScreen.scene-spec.md

## Scene: EventLogScreen.tscn

### Components Under Test
- [ ] Search box filters events correctly
- [ ] Type filter dropdown works
- [ ] Severity filter dropdown works
- [ ] Clear filters button resets state
- [ ] Event selection shows details
- [ ] Export JSON button works
- [ ] Export YAML button works
- [ ] Back button navigates correctly

### Node Path Tests
- [ ] All @onready vars resolve correctly
- [ ] No null references on _Ready()
- [ ] Scene loads without errors

### Integration Tests
- [ ] Receives events from EventStore
- [ ] Updates UI when new events arrive
- [ ] Handles empty event list gracefully
```

#### 2. **Automated Scene Tests**

Use GDUnit4 to create automated scene tests:

```csharp
// Tests/GdUnit/Scenes/EventLogScreenSceneTests.cs
[TestSuite]
public class EventLogScreenSceneTests
{
    [TestCase]
    public void Scene_LoadsWithoutErrors()
    {
        var scene = ResourceLoader.Load<PackedScene>("res://scenes/UI/EventLogScreen.tscn");
        Assertions.AssertThat(scene).IsNotNull();
        
        var instance = scene.Instantiate<EventLogScreenPresenter>();
        Assertions.AssertThat(instance).IsNotNull();
        
        instance.QueueFree();
    }
    
    [TestCase]
    public void AllNodePathsResolve()
    {
        var scene = ResourceLoader.Load<PackedScene>("res://scenes/UI/EventLogScreen.tscn");
        var instance = scene.Instantiate<EventLogScreenPresenter>();
        
        // Add to scene tree so _Ready() is called
        AutoFree(instance);
        
        // Verify critical nodes exist (these would be null if paths are wrong)
        // The presenter's _Ready() method will crash if @onready paths fail
        Assertions.AssertThat(instance).IsNotNull();
    }
}
```

#### 3. **Manual Test Checklist**

For complex UI interactions that are hard to automate:

```
# UI/EventLogScreen.manual-tests.md

## Manual Test Checklist - Event Log Screen

Date Tested: ______ | Tester: ______ | Build: ______

### Search Functionality
- [ ] Search by event description
- [ ] Search by system name
- [ ] Search is case-insensitive
- [ ] Search updates results in real-time

### Filter Combinations
- [ ] Type + Severity filters work together
- [ ] Type + Time range filters work together
- [ ] All three filters work simultaneously
- [ ] Filters persist across scene changes

### Export Functionality
- [ ] JSON export contains all filtered events
- [ ] YAML export is valid YAML
- [ ] File save dialog appears
- [ ] Exported file can be re-imported
```

### Scene Coverage Tracking

Create a scene coverage dashboard:

```markdown
# SCENE_COVERAGE.md

| Scene | Test Spec | Automated Tests | Manual Tests | Status |
|-------|-----------|-----------------|--------------|--------|
| EventLogScreen.tscn | ✅ | ✅ 8/10 | ✅ | 🟢 Good |
| ShipJourneyLog.tscn | ✅ | ✅ 6/8 | ✅ | 🟡 Fair |
| DebugEventPanel.tscn | ❌ | ❌ 0/6 | ❌ | 🔴 Poor |
| GameHUD.tscn | ❌ | ❌ 0/12 | ❌ | 🔴 Poor |
```

## Improving Coverage

### Adding Tests for Untested Code

Example: Adding tests for `TimeSystem`:

```csharp
// Tests/Core/TimeSystemTests.cs
public class TimeSystemTests
{
    [Fact]
    public void TimeAdvances_EmitsTimeAdvancedEvent()
    {
        // Arrange
        var store = new InMemoryEventStore(); // Create test implementation
        var timeSystem = new TimeSystem(store);
        
        // Act
        timeSystem.Tick(1000f);
        
        // Assert
        var events = store.ReadFrom(0).ToList();
        Assert.Single(events);
        Assert.IsType<TimeAdvanced>(events[0]);
    }
}
```

### Excluding UI Code from Coverage Requirements

Update `Tests/coverlet.runsettings`:

```xml
<Exclude>[*]*.UI.*</Exclude>
```

This excludes UI code from coverage metrics since UI is better tested manually or with scene-specific tests.

## Coverage Goals by Milestone

### Milestone 1.3 (Current)
- ✅ Event models: 100%
- ✅ Persistence layer: 60%+
- 🎯 Core domain: 85%+ (currently 0%)
- 🎯 Systems: 70%+ (currently 0%)

### Milestone 1.4
- 🎯 Services: 70%+
- 🎯 Scene test specs: 100% coverage
- 🎯 Automated scene tests: 50%+ of critical paths

### Milestone 2.0
- 🎯 Overall line coverage: 75%+
- 🎯 Branch coverage: 65%+
- 🎯 All scenes have automated smoke tests

## Continuous Integration

### GitHub Actions Example

```yaml
# .github/workflows/coverage.yml
name: Code Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Setup .NET
        uses: actions/setup-dotnet@v1
        with:
          dotnet-version: '8.0.x'
      
      - name: Run Tests with Coverage
        run: |
          cd Tests
          dotnet test --collect:"XPlat Code Coverage" --results-directory:../coverage
      
      - name: Generate Coverage Report
        run: |
          dotnet tool install --global dotnet-reportgenerator-globaltool
          reportgenerator -reports:"coverage/**/coverage.cobertura.xml" -targetdir:"coverage/report" -reporttypes:"Html;Cobertura"
      
      - name: Upload Coverage to Codecov
        uses: codecov/codecov-action@v2
        with:
          files: ./coverage/report/Cobertura.xml
```

## Tools & Resources

### Installed Tools
- **coverlet**: Collects coverage data during test execution
- **ReportGenerator**: Generates HTML reports from coverage data
- **GDUnit4**: Testing framework for Godot-specific code

### Useful Commands

```powershell
# Quick coverage check (console only)
dotnet test /p:CollectCoverage=true /p:CoverletOutputFormat=cobertura

# Generate badges
reportgenerator -reports:"coverage.cobertura.xml" -targetdir:"coverage" -reporttypes:"Badges"

# Coverage with specific threshold
dotnet test /p:CollectCoverage=true /p:Threshold=80 /p:ThresholdType=line
```

## Best Practices

1. **Test Core Logic First**: Focus on domain models and business logic
2. **Use Appropriate Frameworks**: xUnit for pure C#, GDUnit4 for Godot integration
3. **Don't Chase 100%**: Aim for meaningful coverage, not metrics
4. **Test Behavior, Not Implementation**: Focus on what code does, not how
5. **Keep Tests Fast**: Slow tests won't be run regularly
6. **Document Scene Tests**: Use test specs for manual verification
7. **Review Coverage Reports**: Look for untested critical paths
8. **Automate in CI**: Run coverage checks on every PR

## FAQ

**Q: Why is UI coverage 0%?**
A: UI presenters require Godot's scene tree to be initialized. Use GDUnit4 tests for these components.

**Q: Should I aim for 100% coverage?**
A: No. Aim for 80%+ on critical business logic. Some code (like simple getters, UI glue code) doesn't benefit from testing.

**Q: How do I test Godot-specific code?**
A: Use GDUnit4 which runs in the Godot runtime. See existing tests in `Tests/GdUnit/`.

**Q: Can I exclude generated code from coverage?**
A: Yes, the runsettings file already excludes `*.g.cs` files and Godot.SourceGenerators.

**Q: How often should I check coverage?**
A: Run coverage locally before PRs, and have CI check it automatically.

## Next Steps

1. ✅ Set up coverage reporting (Complete!)
2. 🔲 Add tests for `StateStore` and `TimeSystem`
3. 🔲 Create scene test specifications
4. 🔲 Add GDUnit4 tests for presenters
5. 🔲 Set up CI pipeline with coverage checks
6. 🔲 Add coverage badge to README.md
