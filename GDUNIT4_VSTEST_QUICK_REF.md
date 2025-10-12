# GdUnit4 VSTest Adapter - Quick Reference

## Test Discovery & Execution in VSCode

### Prerequisites
✅ C# Dev Kit extension installed
✅ GdUnit4.VSTest.Adapter package installed
✅ `Tests/gdunit4.runsettings` configured
✅ `.vscode/settings.json` points to runsettings file

### Test Explorer
1. Open **Testing** view (beaker icon in sidebar or `Ctrl+Shift+T`)
2. Tests auto-discover when you open test files
3. Click **Run All Tests** or run individual tests
4. View results in Test Explorer panel

### Running Tests
- **All tests**: Click ▶️ at workspace level in Test Explorer
- **Single test**: Click ▶️ next to test name
- **Debug test**: Right-click test → Debug Test

## Command-Line Usage

### Using PowerShell Script (Recommended)
```powershell
# Run all GdUnit4 tests
.\run-gdunit4-vstest.ps1

# Run with verbose output
.\run-gdunit4-vstest.ps1 -Verbose

# Run specific test filter
dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings --filter "FullyQualifiedName~MyTestClass"
```

### Direct dotnet test Commands
```powershell
# Run all tests with GdUnit4 configuration
dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings

# Run with detailed output
dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings --verbosity detailed

# Run specific test class
dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings --filter "FullyQualifiedName~FileEventStoreTests"

# List all discovered tests
dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings --list-tests
```

## Configuration Files

### Tests/gdunit4.runsettings
Main configuration for GdUnit4 VSTest adapter:
- Points to Godot executable
- Disables default logger for cleaner output
- Configures test execution parameters

### .vscode/settings.json
VSCode integration:
```json
{
  "dotnet.defaultSolution": "godot-project/Outpost3.sln",
  "dotnet.unitTests.runSettingsPath": "${workspaceFolder}/Tests/gdunit4.runsettings"
}
```

## Troubleshooting

### Tests Not Discovered
1. Check Test Output panel for errors
2. Verify Godot path in `gdunit4.runsettings`
3. Rebuild: `dotnet build Tests/Outpost3.Tests.csproj`
4. Reload VSCode window

### Tests Fail to Run
1. Ensure Godot is closed (tests launch headless instance)
2. Check GdUnit4.VSTest.Adapter version matches GdUnit4 version
3. Verify `.gdunit4/settings.conf` exists in godot-project/

### Performance Issues
- Use test filters to run subset of tests
- Close unnecessary Godot editor instances
- Check system resources (Godot headless can be memory-intensive)

## Test Filters

### Filter by Class
```powershell
--filter "FullyQualifiedName~FileEventStoreTests"
```

### Filter by Method
```powershell
--filter "FullyQualifiedName~FileEventStoreTests.AppendEvent_SingleEvent_StoresCorrectly"
```

### Filter by Namespace
```powershell
--filter "FullyQualifiedName~Outpost3.Tests.Persistence"
```

### Multiple Filters (OR)
```powershell
--filter "FullyQualifiedName~FileEventStore|FullyQualifiedName~GameState"
```

## Integration with CI/CD

### GitHub Actions Example
```yaml
- name: Run GdUnit4 Tests
  run: dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings --logger:trx --results-directory ./TestResults
```

### Azure Pipelines Example
```yaml
- task: DotNetCoreCLI@2
  inputs:
    command: 'test'
    projects: 'Tests/Outpost3.Tests.csproj'
    arguments: '--settings:Tests/gdunit4.runsettings --logger:trx'
```

## Related Documentation
- [GdUnit4 VSTest Adapter Setup](https://mikeschulze.github.io/gdUnit4/csharp_project_setup/vstest-adapter/)
- [Full Setup Guide](./docs/GDUNIT4_VSTEST_SETUP.md)
- [Testing Guide](./docs/TESTING_GUIDE.md)
