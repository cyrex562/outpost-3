# GdUnit4 Testing Integration

## Overview

This project now supports **dual testing frameworks**:

1. **xUnit** - Traditional .NET testing (CI/CD friendly)
2. **GdUnit4** - Godot-integrated testing framework

## Why GdUnit4?

GdUnit4 provides:

- ✅ Full C# support via `gdUnit4.api`
- ✅ VSTest adapter for VS Code, Visual Studio, and Rider
- ✅ Godot Editor integration
- ✅ Scene testing capabilities
- ✅ Better suited for testing Godot-specific features
- ✅ Can run alongside existing xUnit tests

## Setup

### 1. Install NuGet Packages

Run the setup script:

```powershell
.\setup-gdunit4.ps1
```

Or manually:

```powershell
cd Tests
dotnet restore
dotnet build
```

### 2. Install GdUnit4 Addon in Godot (Optional)

For Godot Editor integration:

1. Open the Godot Editor
2. Go to **AssetLib** tab
3. Search for "GdUnit4"
4. Click **Download** → **Install**
5. Enable plugin: **Project** → **Project Settings** → **Plugins** → Check **GdUnit4**

### 3. Verify Installation

```powershell
# Run all tests (both xUnit and GdUnit4)
dotnet test

# Run only GdUnit4 tests
dotnet test --filter "FullyQualifiedName~GdUnit"
```

## Project Structure Changes

```
Tests/
├── Outpost3.Tests.csproj          # Updated with gdUnit4 packages
├── FileEventStoreTests.cs         # Original xUnit tests (kept)
├── EventStoreWorkflowTests.cs     # Original xUnit tests (kept)
└── GdUnit/                        # NEW: GdUnit4 tests
    ├── FileEventStoreGdTests.cs
    └── EventStoreWorkflowGdTests.cs

godot-project/
├── Outpost3.csproj                # Updated with gdUnit4.api
└── addons/                        # GdUnit4 addon (if installed from AssetLib)
    └── gdUnit4/
```

## Test Framework Comparison

| Feature | xUnit | GdUnit4 |
|---------|-------|---------|
| **CI/CD Integration** | ✅ Excellent | ✅ Excellent (via VSTest) |
| **Godot Editor Integration** | ❌ No | ✅ Yes |
| **Scene Testing** | ❌ Limited | ✅ Native support |
| **C# Support** | ✅ Full | ✅ Full |
| **Assertion API** | `Assert.*` | `Assertions.AssertThat().Is*` |
| **Community** | Large | Growing |

## When to Use Each Framework

### Use xUnit for:

- ✅ Pure domain logic (reducers, systems)
- ✅ Event store persistence tests
- ✅ CI/CD pipelines
- ✅ Non-Godot specific code

### Use GdUnit4 for:

- ✅ Godot scene testing
- ✅ UI component behavior
- ✅ Node interactions
- ✅ Features requiring Godot APIs

### Both Work for:

- ✅ Event store tests
- ✅ Command/Event validation
- ✅ State reconstruction tests

## Running Tests

### Command Line

```powershell
# All tests
dotnet test

# Only xUnit
dotnet test --filter "FullyQualifiedName!~GdUnit"

# Only GdUnit4
dotnet test --filter "FullyQualifiedName~GdUnit"

# Specific test class
dotnet test --filter "FullyQualifiedName~FileEventStoreGdTests"

# With coverage
dotnet test /p:CollectCoverage=true
```

### Visual Studio Code

1. Install **C# Dev Kit** extension
2. Use **Test Explorer**
3. Click play button next to tests

### Godot Editor (GdUnit4 only)

1. Open GdUnit4 panel (bottom dock)
2. Browse test tree
3. Click **Run** button

## Migration Strategy

We're using a **coexistence strategy**:

1. ✅ **Keep existing xUnit tests** - They work great for core logic
2. ✅ **Add GdUnit4 tests** - For new Godot-specific features
3. ✅ **Duplicate critical tests** - Both frameworks test the same logic (example: FileEventStore)
4. ⏭️ **Future**: Migrate UI tests to GdUnit4 as they're developed

## Example Tests

### xUnit Test

```csharp
[Fact]
public void Append_SingleEvent_WritesToFile()
{
    var store = new FileEventStore(_testFilePath);
    var testEvent = new ShipDepartedEvent(...);
    
    store.Append(testEvent);
    
    Assert.True(File.Exists(_testFilePath));
    Assert.Equal(1, store.Count);
}
```

### GdUnit4 Test (Same Logic)

```csharp
[TestCase]
public void Append_SingleEvent_WritesToFile()
{
    var store = new FileEventStore(_testFilePath!);
    var testEvent = new ShipDepartedEvent(...);
    
    store.Append(testEvent);
    
    Assertions.AssertThat(File.Exists(_testFilePath!)).IsTrue();
    Assertions.AssertThat(store.Count).IsEqual(1);
}
```

## Documentation

- 📖 **Full Testing Guide**: `docs/TESTING_GUIDE.md`
- 📖 **Test Project README**: `Tests/README_NEW.md`
- 📖 **GdUnit4 Docs**: [https://mikeschulze.github.io/gdUnit4/](https://mikeschulze.github.io/gdUnit4/)
- 📖 **xUnit Docs**: [https://xunit.net/](https://xunit.net/)

## Benefits for This Project

Given Outpost3's architecture:

1. **Event-Sourced Core**: Both frameworks test pure reducers equally well
2. **Godot UI Layer**: GdUnit4 will shine here (presenters, scenes)
3. **CI/CD**: xUnit continues to work perfectly
4. **Developer Choice**: Use whichever framework fits the feature

## Next Steps

1. ✅ Install packages (run `setup-gdunit4.ps1`)
2. ✅ Verify tests pass: `dotnet test`
3. 🔄 Install GdUnit4 addon in Godot (optional, for editor integration)
4. 📝 Use GdUnit4 for future UI/scene tests
5. 📝 Keep xUnit for core domain logic tests
