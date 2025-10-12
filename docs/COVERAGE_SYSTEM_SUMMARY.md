# Code Coverage System - Summary

## ✅ What's Been Set Up

Your Outpost3 project now has a complete code coverage system with:

### 1. **C# Code Coverage** 
- ✅ Coverlet for coverage collection
- ✅ ReportGenerator for HTML reports
- ✅ Configuration file (`Tests/coverlet.runsettings`)
- ✅ Coverage reports in `coverage/report/index.html`

### 2. **Current Coverage Status**
As of 2025-10-11:
- **Line Coverage**: 5.6% (147/2580 lines)
- **Branch Coverage**: 3.6% (39/1082 branches)  
- **Method Coverage**: 8.6% (19/220 methods)

**Well-Tested Components** (100% coverage):
- All event record classes (ShipDepartedEvent, etc.)
- Core persistence layer (FileEventStore: 61.6%, GameEventJsonConverter: 65.1%)

**Needs Testing** (0% coverage):
- UI Presenters (EventLogScreenPresenter, etc.)
- Core Systems (StateStore, TimeSystem)
- Services (EventExporter)
- Domain Models (GameState, ProbeInFlight)

### 3. **Godot Scene Testing Strategy**
Since Godot scenes (.tscn files) **cannot be measured by traditional code coverage tools**, we've established:

- ✅ Scene Test Specification template
- ✅ Example spec for EventLogScreen
- ✅ Directory structure: `Tests/GdUnit/SceneSpecs/`
- ✅ Guidelines for automated GDUnit4 scene tests
- ✅ Manual test checklist template

## 🚀 How to Use

### Quick Coverage Check

```powershell
# Navigate to Tests directory
cd Tests

# Run tests with coverage
dotnet test --collect:"XPlat Code Coverage" --results-directory:../coverage --settings:coverlet.runsettings

# Generate HTML report
cd ..
reportgenerator -reports:"coverage\**\coverage.cobertura.xml" -targetdir:"coverage\report" -reporttypes:"Html;Badges;TextSummary"

# View report in browser
start coverage\report\index.html
```

### View Existing Report

```powershell
start coverage\report\index.html
```

### Check Current Status

```powershell
Get-Content coverage\report\Summary.txt
```

## 📚 Documentation

| Document | Purpose |
|----------|---------|
| **CODE_COVERAGE_GUIDE.md** | Complete guide to code coverage strategy, goals, and best practices |
| **COVERAGE_QUICK_REF.md** | Quick reference card with common commands and current status |
| **Tests/GdUnit/SceneSpecs/EventLogScreen.scene-spec.md** | Example scene test specification |

## 🎯 Next Steps

### Immediate (Milestone 1.3)
1. **Add tests for core domain logic** (Target: 85%+)
   - StateStore
   - TimeSystem
   - GameState
   - ProbeInFlight

2. **Improve persistence layer coverage** (Target: 85%+)
   - Increase FileEventStore from 61.6% to 85%+
   - Test edge cases and error handling

### Short-term (Milestone 1.4)
3. **Create scene test specs for all scenes**
   - ShipJourneyLog.tscn
   - DebugEventPanel.tscn
   - GameHUD.tscn
   
4. **Add GDUnit4 tests for presenters** (Target: 50%+)
   - Focus on presenter logic, not UI rendering
   - Test data binding and event handling

### Long-term (Milestone 2.0)
5. **Set up CI/CD with coverage checks**
   - GitHub Actions workflow
   - Automated coverage reporting
   - Coverage badges in README

6. **Achieve project coverage goals**
   - Overall line coverage: 75%+
   - Branch coverage: 65%+
   - All critical paths tested

## 🔧 Tools Installed

- **coverlet.collector** (v6.0.4) - Collects coverage during test runs
- **coverlet.msbuild** (v6.0.4) - MSBuild integration for coverage
- **dotnet-reportgenerator-globaltool** (v5.4.17) - Generates HTML reports

## 📊 Coverage Reports Include

- **Interactive HTML report** with file-by-file breakdown
- **Summary statistics** (line, branch, method coverage)
- **SVG badges** for README display
- **Risk hot spots** highlighting untested code
- **Historical trends** (when run repeatedly)

## ❓ Common Questions

**Q: Why can't I measure Godot scene coverage?**  
A: .tscn files are data files, not code. Use scene test specifications and GDUnit4 tests for the C# presenters instead.

**Q: Should I aim for 100% coverage?**  
A: No. Target 80%+ for core business logic. Some code (simple getters, UI glue) doesn't benefit from testing.

**Q: How do I test UI code?**  
A: Use GDUnit4 which runs in Godot's runtime. Test presenter logic, not rendering.

**Q: How often should I run coverage?**  
A: Locally before each PR. Set up CI to run automatically on push.

## 🎓 Best Practices

1. **Test behavior, not implementation** - Focus on what code does, not how
2. **Keep tests fast** - Slow tests won't be run regularly  
3. **Use appropriate frameworks** - xUnit for pure C#, GDUnit4 for Godot
4. **Don't chase metrics** - Focus on testing critical paths
5. **Review coverage reports** - Look for untested logic, not just numbers
6. **Document scene tests** - Use test specs for manual verification

## 🏆 Success Criteria

Your coverage system is successful when:

- ✅ All team members can run coverage locally
- ✅ CI runs coverage on every PR
- ✅ Core business logic has 80%+ coverage
- ✅ Critical bugs have corresponding regression tests
- ✅ Coverage trend is stable or improving
- ✅ Scene test specs exist for all major screens

---

**Ready to get started?** Run your first coverage report and explore the HTML output!

```powershell
start coverage\report\index.html
```
