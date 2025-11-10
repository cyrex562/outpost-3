# GdUnit4 Migration Plan

## Goal
Port all tests to GdUnit4 and set up proper CLI + VSTest integration for CI/CD

## Current State Analysis
- ✅ Test project has `gdUnit4.api` v5.0.0
- ✅ Test project has `gdUnit4.test.adapter` v3.0.0
- ✅ 15 existing GdUnit4 tests (EventStoreWorkflow, FileEventStore)
- ✅ 43 xUnit tests (SystemSelection, Serialization)
- ✅ 9 SaveLoad tests (currently mixed xUnit/GdUnit4)
- ⚠️ StateStore extends Godot.Node (requires runtime)
- ⚠️ No `.gdunit4` configuration file

## GdUnit4 Test Execution Options

### Option 1: GdUnit4 CLI Tool (Recommended for CI/CD)
**Source:** https://mikeschulze.github.io/gdUnit4/faq/ci/

```bash
# Install CLI globally
dotnet tool install --global gdUnit4.test.adapter

# Run tests from Godot project
gdUnit4Test --testadapter "C:\path\to\test\assembly.dll"
```

**Pros:**
- Runs inside actual Godot runtime
- Supports Godot.Node dependencies
- Generate XML/HTML reports
- Perfect for CI/CD pipelines

**Cons:**
- Requires Godot executable in PATH
- More complex setup

### Option 2: VSTest Adapter (VS Code Integration)
**Source:** https://mikeschulze.github.io/gdUnit4/csharp_project_setup/vstest-adapter/

Tests discovered and run through VSTest protocol, but **still requires Godot runtime**.

**Key Requirement:** `.gdunit4` configuration file pointing to Godot executable

```yaml
# .gdunit4 file
Godot4Executable: C:\path\to\Godot_v4.5-stable_mono_win64.exe
```

### Option 3: Pure GdUnit4 (No Godot Runtime)
Only works for tests that **don't** use Godot APIs (Node, signals, GD.Print, etc.)

## Migration Strategy

### Phase 1: Configure GdUnit4 for CI/CD ✅ PRIORITY
1. Create `.gdunit4` config file with Godot executable path
2. Set up GdUnit4 CLI tool
3. Create PowerShell script for running tests
4. Verify VSTest integration works in VS Code

### Phase 2: Port Pure Domain Tests
Port tests that **don't** require Godot runtime:
- ✅ SystemSelectionTests (33 tests) - Pure domain logic
- ✅ SystemSelectedSerializationTests (10 tests) - JSON serialization
- ✅ FileEventStoreTests (if not using Godot APIs)
- ✅ UlidJsonConverter tests

### Phase 3: Keep/Update Godot-Dependent Tests  
Tests that **require** Godot runtime:
- ✅ EventStoreWorkflowGdTests (already GdUnit4)
- ✅ FileEventStoreGdTests (already GdUnit4)
- ✅ SaveLoadServiceGdTests (new, requires StateStore)
- ✅ JsonSnapshotStore tests (currently uses Console.WriteLine)

### Phase 4: Remove xUnit Dependencies (Optional)
Once all tests migrated, can remove:
- `xunit` package
- `xunit.runner.visualstudio` package
- Keep `Microsoft.NET.Test.Sdk` (required by GdUnit4)

## Implementation Steps

### Step 1: Create .gdunit4 Configuration
```yaml
# File: C:\Users\cyrex\projects\outpost-3\.gdunit4
Godot4Executable: C:\Users\cyrex\projects\outpost-3\bin\Godot_v4.5-stable_mono_win64_console.exe
```

### Step 2: Install GdUnit4 CLI Tool
```powershell
dotnet tool install --global gdUnit4.test.adapter
# Or update if already installed
dotnet tool update --global gdUnit4.test.adapter
```

### Step 3: Create Test Runner Script
```powershell
# File: run-gdunit4-tests.ps1
param(
    [string]$Filter = "*",
    [string]$OutputPath = ".\TestResults"
)

$godotExe = "C:\Users\cyrex\projects\outpost-3\bin\Godot_v4.5-stable_mono_win64_console.exe"
$testDll = "C:\Users\cyrex\projects\outpost-3\Tests\bin\Debug\net9.0\Outpost3.Tests.dll"

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path $OutputPath | Out-Null

# Run tests with Godot runtime
gdUnit4Test `
    --godot $godotExe `
    --testadapter $testDll `
    --filter $Filter `
    --reportdir $OutputPath

# Display results
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ All tests passed!" -ForegroundColor Green
} else {
    Write-Host "❌ Tests failed with exit code: $LASTEXITCODE" -ForegroundColor Red
}

exit $LASTEXITCODE
```

### Step 4: Port SystemSelectionTests to GdUnit4

**Current:** `Tests/SystemSelectionTests.cs` (xUnit)
**Target:** `Tests/GdUnit/SystemSelectionGdTests.cs` (GdUnit4)

Key changes:
```csharp
// FROM (xUnit):
[Fact]
public void SelectSystemCommand_Creation_SetsProperties()
{
    Assert.Equal(systemId, command.SystemId);
}

// TO (GdUnit4):
[TestCase]
public void SelectSystemCommand_Creation_SetsProperties()
{
    Assertions.AssertThat(command.SystemId).IsEqual(systemId);
}
```

### Step 5: Update VS Code Settings
```json
// .vscode/settings.json
{
    "dotnet.testWindow.useTestExplorer": true,
    "gdUnit4.godotExecutable": "C:\\Users\\cyrex\\projects\\outpost-3\\bin\\Godot_v4.5-stable_mono_win64_console.exe"
}
```

## Benefits of Full GdUnit4 Migration

✅ **Single Test Framework**
- No confusion between xUnit and GdUnit4
- Consistent assertion syntax
- Unified test discovery

✅ **Godot Runtime Support**
- Can test StateStore and other Godot.Node classes
- Full integration testing possible
- Signals, scenes, nodes all testable

✅ **CI/CD Ready**
- GdUnit4 CLI tool designed for automation
- XML/HTML report generation
- Exit codes for pipeline integration

✅ **VS Code Integration**
- Test Explorer support
- Run/Debug individual tests
- Real-time test discovery

## Open Questions

1. **Do we remove Console.WriteLine from JsonSnapshotStore?**
   - If yes, revert to GD.Print() for Godot logging
   - If no, keep Console.WriteLine for compatibility

2. **Do we keep dual framework temporarily?**
   - Allows gradual migration
   - Or do full migration at once?

3. **Coverage reporting?**
   - Does GdUnit4 support coverlet?
   - Alternative coverage tools?

## Recommended Next Steps

1. ✅ Create `.gdunit4` config file
2. ✅ Install GdUnit4 CLI tool globally
3. ✅ Create `run-gdunit4-tests.ps1` script
4. ✅ Test CLI execution with existing GdUnit4 tests
5. ✅ Port SystemSelectionTests to GdUnit4
6. ✅ Port SerializationTests to GdUnit4
7. ✅ Update JsonSnapshotStore to use GD.Print() again
8. ✅ Remove xUnit packages
9. ✅ Update documentation

## Timeline Estimate

- **Phase 1 (Config & CLI):** 30 minutes
- **Phase 2 (Port Domain Tests):** 2 hours
- **Phase 3 (Godot Tests):** 1 hour
- **Phase 4 (Cleanup):** 30 minutes

**Total:** ~4 hours for complete migration
