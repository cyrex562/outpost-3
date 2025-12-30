# GdUnit4 VSTest Adapter Setup

This document describes the VSTest adapter setup for GdUnit4, enabling test discovery and execution in VSCode and from the command line.

## Overview

GdUnit4Net uses the industry-standard VSTest API to provide IDE integration. This allows VSCode (with C# Dev Kit) and other IDEs to:

- **Discover** tests automatically in your Godot C# projects
- **Execute** tests with real-time feedback
- **Debug** tests with breakpoints and variable inspection
- **Filter** and organize test runs
- **Generate** detailed reports in multiple formats (HTML, TRX, console)

## Prerequisites

### 1. Required Extensions (VSCode)

Install the **C# Dev Kit** extension in VSCode:
- Open VSCode Extensions (Ctrl+Shift+X)
- Search for "C# Dev Kit"
- Install version **1.5.12 or higher** (pre-release recommended)

### 2. Environment Setup

The `GODOT_BIN` environment variable is configured in the `.runsettings` file:
```
c:\Users\cyrex\bin\Godot_v4.5-stable_mono_win64\Godot_v4.5-stable_mono_win64.exe
```

If your Godot installation is elsewhere, update `Tests/gdunit4.runsettings`.

### 3. Project Configuration

Both test projects are already configured with required packages:

**Tests/Outpost3.Tests.csproj:**
- `gdUnit4.api` (v5.0.0) - Core API
- `gdUnit4.test.adapter` (v3.0.0) - VSTest adapter
- `Microsoft.NET.Test.Sdk` - Test platform
- `coverlet.collector` & `coverlet.msbuild` - Code coverage

## Configuration Files

### Tests/gdunit4.runsettings

This is the main configuration file for GdUnit4 test execution. It includes:

- **GODOT_BIN**: Path to Godot executable
- **TestSessionTimeout**: 180 seconds (3 minutes) for test discovery
- **CompileProcessTimeout**: 30 seconds for project compilation
- **Loggers**: Console (detailed), HTML, and TRX output
- **DisplayName**: FullyQualifiedName for clear test identification
- **Coverage**: Coverlet configuration for code coverage

### .vscode/settings.json

VSCode-specific settings:
```json
{
  "dotnet.unitTests.runSettingsPath": "${workspaceFolder}/Tests/gdunit4.runsettings"
}
```

This tells the C# Dev Kit where to find the runsettings file.

## Usage

### VSCode Test Explorer

1. **Open Test Explorer**:
   - Click the beaker icon in the Activity Bar (left side), OR
   - Press `Ctrl+Shift+P` and search for "Test: Focus on Test Explorer View"

2. **Discover Tests**:
   - Tests are automatically discovered when you open the Test Explorer
   - If tests don't appear, click the refresh icon
   - You should see all GdUnit4 tests organized by namespace/class

3. **Run Tests**:
   - Click the play button next to any test/class/namespace to run
   - Right-click for more options (debug, run with coverage, etc.)
   - Use the filter box to find specific tests

4. **Debug Tests**:
   - Click the debug icon next to a test
   - Set breakpoints in your test or source code
   - Debugger will attach to the Godot process

### Command Line

#### Run all tests:
```powershell
.\run-gdunit4-vstest.ps1
```

#### Run with filter:
```powershell
# Run only tests containing "ProbeSystem"
.\run-gdunit4-vstest.ps1 -Filter "ProbeSystem"

# Run only tests in a specific namespace
.\run-gdunit4-vstest.ps1 -Filter "FullyQualifiedName~Outpost3.Tests.Core"
```

#### Run without rebuilding:
```powershell
.\run-gdunit4-vstest.ps1 -NoBuild
```

#### Verbose output:
```powershell
.\run-gdunit4-vstest.ps1 -Verbose
```

#### Direct dotnet test command:
```powershell
dotnet test Tests/Outpost3.Tests.csproj --settings Tests/gdunit4.runsettings
```

### Filter Syntax

VSTest supports powerful filtering:

```powershell
# By test name
--filter "TestName=MyTest"

# By fully qualified name (namespace.class.method)
--filter "FullyQualifiedName~ProbeSystem"

# By class
--filter "FullyQualifiedName~MyTestClass"

# Multiple conditions
--filter "FullyQualifiedName~Core&TestName~Probe"

# Exclude tests
--filter "FullyQualifiedName!~Integration"
```

## Test Output & Reports

After running tests, you'll find results in `TestResults/`:

- **test-result.html** - Rich HTML report (open in browser)
- **test-result.trx** - TRX format (for CI/CD tools)
- **Console output** - Detailed verbosity in terminal

Code coverage reports (if enabled) go to `coverage/`:
- `coverage.cobertura.xml` - Cobertura format
- `coverage.opencover.xml` - OpenCover format
- `coverage.json` - JSON format

## Troubleshooting

### Tests not discovered

1. Check that C# Dev Kit is installed and enabled
2. Verify `GODOT_BIN` path in `gdunit4.runsettings`
3. Increase `TestSessionTimeout` in runsettings
4. Restart VSCode
5. Check Output panel → "Test" for discovery errors

### Compilation timeout

If tests fail with compilation timeout:
1. Increase `CompileProcessTimeout` in `gdunit4.runsettings`
2. Default is 30000ms (30 seconds)
3. Large projects may need 60000ms or more

### Godot not found

Error: "Could not find Godot executable"
- Verify `GODOT_BIN` environment variable in runsettings
- Use absolute path, not relative
- Ensure Godot executable exists at that path

### Tests run but don't appear in Test Explorer

This is a known VSCode/C# Dev Kit issue:
1. Try reloading the window (Ctrl+Shift+P → "Reload Window")
2. Delete `.vs/` folder and restart VSCode
3. Ensure you're using C# Dev Kit pre-release version

## Supported Features

| Feature | VSCode | Command Line |
|---------|--------|--------------|
| Test Discovery | ✅ | ✅ |
| Run Tests | ✅ | ✅ |
| Debug Tests | ✅ | ❌ |
| Filter Tests | ✅ | ✅ |
| HTML Reports | ✅ | ✅ |
| Code Coverage | ✅ | ✅ |

## References

- [GdUnit4 VSTest Adapter Docs](https://mikeschulze.github.io/gdUnit4/csharp_project_setup/vstest-adapter/)
- [VSTest Filter Syntax](https://learn.microsoft.com/en-us/dotnet/core/testing/selective-unit-tests)
- [C# Dev Kit Testing](https://code.visualstudio.com/docs/csharp/testing)
- [RunSettings Schema](https://learn.microsoft.com/en-us/visualstudio/test/configure-unit-tests-by-using-a-dot-runsettings-file)

## Integration with Existing Scripts

The existing test scripts remain functional:

- `run-tests.ps1` - Runs xUnit tests (Core logic tests)
- `run-gdunit4-tests.ps1` - Runs GdUnit4 tests (Godot integration tests)
- `run-coverage.ps1` - Runs tests with code coverage
- **NEW:** `run-gdunit4-vstest.ps1` - Runs GdUnit4 with VSTest adapter

You can continue using the old scripts. The VSTest adapter setup adds IDE integration and doesn't replace the existing workflow.

## Next Steps

1. **Restart VSCode** to activate the new settings
2. **Open Test Explorer** (beaker icon)
3. **Verify tests are discovered** - you should see your GdUnit4 tests
4. **Run a test** from the explorer to verify everything works
5. **Try debugging** a test with a breakpoint

If you encounter issues, check the Troubleshooting section above or the [official documentation](https://mikeschulze.github.io/gdUnit4/csharp_project_setup/vstest-adapter/).
