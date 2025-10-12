# Code Coverage Quick Reference

## 🚀 Quick Commands

### Run Coverage (Full Process)
```powershell
cd Tests
dotnet test --collect:"XPlat Code Coverage" --results-directory:../coverage --settings:coverlet.runsettings
cd ..
reportgenerator -reports:"coverage\**\coverage.cobertura.xml" -targetdir:"coverage\report" -reporttypes:"Html;Badges;TextSummary"
start coverage\report\index.html
```

### View Last Report
```powershell
start coverage\report\index.html
```

## 📊 Current Status (2025-10-11)

- **Line Coverage**: 5.6% 🔴
- **Branch Coverage**: 3.6% 🔴  
- **Method Coverage**: 8.6% 🔴

## ✅ Well-Tested (80%+)
- Event models (100%)
- Persistence layer (61-65%)

## ⚠️ Needs Tests (0%)
- UI Presenters
- Core Systems (StateStore, TimeSystem)
- Services (EventExporter)
- Domain Models (GameState, ProbeInFlight)

## 📝 Testing Godot Scenes

Godot scenes (.tscn) **cannot** be measured by traditional coverage tools.

### Alternative: Scene Test Specifications

Create test specs in `Tests/GdUnit/SceneSpecs/`:

```markdown
# SceneName.scene-spec.md

## Components Under Test
- [ ] Feature 1 works
- [ ] Feature 2 works
- [ ] All node paths resolve

## Manual Tests
- [ ] UI interaction 1
- [ ] UI interaction 2
```

### Automated Scene Tests (GDUnit4)

```csharp
[TestSuite]
public class SceneNameTests
{
    [TestCase]
    public void Scene_LoadsWithoutErrors()
    {
        var scene = ResourceLoader.Load<PackedScene>("res://scenes/SceneName.tscn");
        var instance = scene.Instantiate();
        Assertions.AssertThat(instance).IsNotNull();
        instance.QueueFree();
    }
}
```

## 🎯 Coverage Goals

| Component | Target | Current |
|-----------|--------|---------|
| Events | 95%+ | 100% ✅ |
| Persistence | 80%+ | 63% 🟡 |
| Core Logic | 85%+ | 0% 🔴 |
| Systems | 70%+ | 0% 🔴 |
| UI | 50%+ | 0% 🔴 |

## 📚 Full Documentation

See [`docs/CODE_COVERAGE_GUIDE.md`](./CODE_COVERAGE_GUIDE.md) for complete guide.
