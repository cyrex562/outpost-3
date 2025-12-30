# GdUnit4 Test Setup & Migration Guide

## Current Status

### ✅ Configured
- **gdUnit4.api** v5.0.0 (NuGet package)
- **gdUnit4.test.adapter** v3.0.0 (VSTest integration)
- **.gdunit4** configuration file created
- **VS Code settings** updated with Godot executable path
- **PowerShell test runner** scripts created

### Test Distribution
- **15 GdUnit4 tests** (EventStoreWorkflow, FileEventStore)
- **43 xUnit tests** (SystemSelection, Serialization)  
- **9 SaveLoad tests** (mixed xUnit/GdUnit4)

## Test Execution Methods

### Method 1: Standard dotnet test (Current Working Approach)
**Command:**
```powershell
.\run-tests.ps1 -Filter "*GdUnit*"
```

**How it works:**
- Uses `dotnet test` with gdUnit4.test.adapter
- VSTest discovers and runs GdUnit4 tests
- **Limitation:** Cannot run tests requiring Godot.Node outside Godot runtime

**Best for:**
- Pure C# logic tests (no Godot dependencies)
- CI/CD pipelines
- Quick test runs

### Method 2: GdUnit4 CLI Tool (Recommended but currently broken)
**Command:**
```powershell
gdUnit4Test --godot <path> --testadapter <dll>
```

**Status:** ❌ Installation fails with path error (known issue)

**When fixed, this provides:**
- Full Godot runtime context
- Can test Godot.Node classes
- Can test Signals, scenes, UI components
- Designed for CI/CD integration

### Method 3: VS Code Test Explorer
**Configuration:** Already set up in `.vscode/settings.json`

**How to use:**
1. Open VS Code
2. Go to Testing view (beaker icon)
3. Tests auto-discovered via VSTest adapter
4. Click play button to run individual tests or suites

**Limitation:** Same as Method 1 - no Godot runtime for Node-dependent tests

## Architecture Decision: Test Framework Strategy

### Current Hybrid Approach (Recommended)
Keep both xUnit and GdUnit4 for different purposes:

#### ✅ Use xUnit for:
- **Pure domain logic** (Commands, Events, Reducers)
  - SystemSelectionTests
  - TimeSystemTests  
  - Event serialization tests
- **Persistence layer** (FileEventStore, JsonSnapshotStore)
- **JSON converters** (UlidJsonConverter, GameEventJsonConverter)
- **Any code that doesn't use Godot APIs**

**Benefits:**
- Fast execution (no Godot runtime needed)
- Standard .NET testing ecosystem
- Excellent IDE integration
- Works great in CI/CD

#### ✅ Use GdUnit4 for:
- **Godot.Node subclasses** (StateStore, UI Presenters)
- **Signal emissions** and event handling
- **Scene instantiation** and node tree tests
- **UI component integration** tests
- **Any code using GD.*, Node, PackedScene, etc.**

**Benefits:**
- Full Godot runtime available
- Can test actual game engine integration
- Designed specifically for Godot C# testing

### Alternative: Full GdUnit4 Migration

**Pros:**
- Single test framework
- Consistent API
- All tests can access Godot runtime

**Cons:**
- ❌ GdUnit4 CLI tool currently broken
- ❌ Slower test execution (requires Godot runtime even for pure logic)
- ❌ More complex CI/CD setup
- ❌ Smaller community than xUnit
- ❌ Less mature tooling

**Verdict:** NOT RECOMMENDED at this time due to CLI tool issues

## Recommended Action Plan

### Phase 1: Optimize Current Hybrid Setup ✅ DONE
1. ✅ Created `.gdunit4` configuration
2. ✅ Updated VS Code settings
3. ✅ Created test runner scripts
4. ✅ Documented test strategy

### Phase 2: Categorize and Organize Tests
1. Move all **pure domain tests** to `Tests/` (xUnit)
   - Keep SystemSelectionTests.cs
   - Keep SystemSelectedSerializationTests.cs
   - Keep FileEventStoreTests.cs (if no Godot deps)

2. Move all **Godot-dependent tests** to `Tests/GdUnit/` (GdUnit4)
   - Keep EventStoreWorkflowGdTests.cs
   - Keep FileEventStoreGdTests.cs
   - Add StateStore integration tests
   - Add UI presenter tests

### Phase 3: Fix Godot Dependencies
For tests requiring Godot runtime but currently using xUnit:

**Option A:** Convert to GdUnit4 and move to `Tests/GdUnit/`
```csharp
// Example: SaveLoadServiceTests requiring StateStore
[TestSuite]
public class SaveLoadServiceGdTests
{
    [TestCase]
    public void SaveGame_CreatesSnapshot()
    {
        var stateStore = new StateStore(eventStore);  // OK in GdUnit4!
        // ... test code ...
    }
}
```

**Option B:** Refactor to remove Godot dependencies
```csharp
// Extract interface
public interface IStateStore
{
    GameState State { get; }
    void ApplyCommand(ICommand command);
}

// Pure C# implementation for testing
public class InMemoryStateStore : IStateStore
{
    // No Godot dependencies
}

// Godot adapter
public partial class StateStore : Node, IStateStore
{
    // Godot-specific implementation
}
```

### Phase 4: Document Test Guidelines
Create `Tests/README.md` with:
- When to use xUnit vs GdUnit4
- How to run different test categories
- CI/CD integration guide
- VS Code testing workflow

## CI/CD Integration

### GitHub Actions Example
```yaml
name: Tests

on: [push, pull_request]

jobs:
  test-domain-logic:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-dotnet@v3
        with:
          dotnet-version: '9.0.x'
      
      # Run xUnit tests (pure domain logic, fast)
      - name: Run xUnit Tests
        run: dotnet test --filter "FullyQualifiedName!~GdUnit" --logger "trx"
      
      - name: Upload Test Results
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: '**/TestResults/*.trx'

  test-godot-integration:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      # Download Godot
      - name: Download Godot
        run: |
          wget https://github.com/godotengine/godot/releases/download/4.5-stable/Godot_v4.5-stable_mono_linux_x86_64.zip
          unzip Godot_v4.5-stable_mono_linux_x86_64.zip
      
      # Run GdUnit4 tests (requires Godot runtime)
      - name: Run GdUnit4 Tests
        run: |
          # When CLI tool is fixed:
          # gdUnit4Test --godot ./Godot_mono --testadapter ./Tests/bin/Debug/net9.0/Outpost3.Tests.dll
          # For now, skip or use alternative approach
          echo "GdUnit4 CLI tool needs fixing"
```

## Key Files Created

### Configuration Files
- **`.gdunit4`** - Godot executable configuration
- **`.vscode/settings.json`** - VS Code test integration
- **`Tests/coverlet.runsettings`** - Coverage settings (existing)

### Test Runner Scripts
- **`run-tests.ps1`** - Quick test runner using dotnet test
- **`run-gdunit4-tests.ps1`** - GdUnit4 CLI runner (when tool is fixed)
- **`run-coverage.ps1`** - Coverage runner (existing)

### Documentation
- **`docs/GDUNIT4_MIGRATION_PLAN.md`** - Full migration analysis
- **`Tests/TEST_FIX_SUMMARY.md`** - Godot dependency fixes
- **This file** - Setup and usage guide

## Troubleshooting

### GdUnit4 tests not discovered in VS Code
1. Ensure `gdUnit4.test.adapter` package is installed
2. Check `.vscode/settings.json` has correct Godot path
3. Rebuild test project: `dotnet build Tests/Outpost3.Tests.csproj`
4. Reload VS Code window

### Tests fail with "Godot.Node requires runtime"
- Test is using Godot APIs but running through xUnit
- **Fix:** Move test to `Tests/GdUnit/` folder and convert to GdUnit4 syntax
- **Alternative:** Refactor code to remove Godot dependencies

### GdUnit4 CLI tool installation fails
- **Known issue:** Tool has path problems on some systems
- **Workaround:** Use `dotnet test` with VSTest adapter instead
- **Monitor:** https://github.com/MikeSchulze/gdUnit4Net/issues

## Best Practices

### DO ✅
- Use xUnit for pure domain logic (fast, reliable)
- Use GdUnit4 for Godot-specific features (Node, signals, scenes)
- Keep test files organized by framework (`Tests/` vs `Tests/GdUnit/`)
- Document which tests require Godot runtime
- Use `[TestSuite]` and `[TestCase]` for GdUnit4 tests
- Use `[Fact]` for xUnit tests

### DON'T ❌
- Don't use Godot APIs in xUnit tests (will crash)
- Don't port pure logic tests to GdUnit4 (slower for no benefit)
- Don't mix assertion styles in same file
- Don't rely on GdUnit4 CLI until installation issues are fixed

## Next Steps

1. **Run existing tests:**
   ```powershell
   .\run-tests.ps1
   ```

2. **Check test results** in VS Code Test Explorer

3. **Decide on migration strategy:**
   - Keep hybrid (recommended)
   - OR wait for GdUnit4 CLI fix and migrate gradually

4. **Document test patterns** in team wiki/README

5. **Set up CI/CD** using hybrid approach shown above

## Resources

- GdUnit4 Documentation: https://mikeschulze.github.io/gdUnit4/
- VSTest Adapter: https://github.com/MikeSchulze/gdUnit4Net/tree/master/TestAdapter
- CI/CD Guide: https://mikeschulze.github.io/gdUnit4/faq/ci/
- GitHub Issues: https://github.com/MikeSchulze/gdUnit4Net/issues
