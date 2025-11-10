# Manual Testing Guide: Galaxy Map & Star Exploration

**Feature**: Galaxy Map with 100-star generation and discovery mechanics  
**Date**: October 12, 2025

---

## Prerequisites

Before testing, ensure:
- ✅ All code changes have been built successfully
- ✅ Godot Editor is closed (tests require exclusive access)
- ✅ You have the latest version of the project

---

## Test 1: Galaxy Initialization

### Goal
Verify that 100 stars are generated at game start with Sol at the center.

### Steps in Godot Editor

1. Open the Godot project
2. Open the Main scene (`Scenes/Main.tscn`)
3. Add an `InitializeGalaxy` command to the app initialization
4. Run the scene
5. Open the Debug Event Panel (if available)

### Expected Results
- ✅ Galaxy should initialize with 100 star systems
- ✅ Sol should be at position (0, 0, 0)
- ✅ Sol should be marked as "Explored"
- ✅ All other stars should be "Detected" (greyed out)
- ✅ Sol should have 8 bodies (Mercury through Neptune)

### Verification Commands (C# Console)
```csharp
var state = _stateStore.State;
GD.Print($"Total systems: {state.Systems.Count}");
var sol = state.Systems.Find(s => s.Name == "Sol");
GD.Print($"Sol position: {sol?.Position}");
GD.Print($"Sol discovery level: {sol?.DiscoveryLevel}");
GD.Print($"Sol bodies count: {sol?.Bodies.Count}");
```

---

## Test 2: Star Positions & Distribution

### Goal
Verify stars are distributed within 100 LY radius using Perlin noise clustering.

### Steps
1. Initialize galaxy with known seed (e.g., 12345)
2. Examine star positions in the systems list

### Expected Results
- ✅ All non-Sol stars within 100 LY radius
- ✅ Stars show clustering (not uniform distribution)
- ✅ Each star has unique position
- ✅ Distance from Sol calculated correctly

### Verification Script
```csharp
foreach (var system in state.Systems)
{
    if (system.Name != "Sol")
    {
        float distance = system.Position.Length();
        GD.Print($"{system.Name}: Position {system.Position}, Distance: {distance:F2} LY");
        
        // Verify within radius
        if (distance > 100f)
        {
            GD.PrintErr($"ERROR: {system.Name} exceeds 100 LY radius!");
        }
    }
}
```

---

## Test 3: Probe Launch to Detected Star

### Goal
Launch a probe to an unexplored star and verify it enters "in flight" state.

### Steps
1. Initialize galaxy
2. Select a non-Sol star from the list
3. Click "Launch Probe" button (or use command)
4. Verify probe appears in "Probes In Flight" list

### Expected Results
- ✅ Probe added to `ProbesInFlight` list
- ✅ `ProbeLaunched` event emitted
- ✅ Probe shows target system and ETA
- ✅ Target system remains "Detected" (not yet scanned)

### Commands
```csharp
var targetSystem = state.Systems.First(s => s.Name != "Sol");
var command = new LaunchProbe(targetSystem.Id);
_stateStore.ApplyCommand(command);

GD.Print($"Probes in flight: {state.ProbesInFlight.Count}");
```

---

## Test 4: Probe Arrival & System Scan

### Goal
Verify probe arrival updates system to "Scanned" and generates bodies.

### Steps
1. Launch a probe to a star
2. Advance time by 150+ game hours (probe travel time is 100h)
3. Observe system updates

### Expected Results
- ✅ Probe removed from "in flight" list
- ✅ `ProbeArrived` event emitted
- ✅ `SystemScanned` event emitted
- ✅ Target system's `DiscoveryLevel` changed from `Detected` to `Scanned`
- ✅ System now has 0-11 bodies generated
- ✅ Bodies have `BodyType` and `Composition` filled in
- ✅ Some bodies MAY have `AtmosphereType` or `SurfaceType` (30% chance each)
- ✅ Bodies do NOT have Temperature, Gravity, Resources, Hazards yet (not explored)

### Commands
```csharp
// After advancing time
var scannedSystem = state.Systems.First(s => s.Id == targetSystemId);
GD.Print($"Discovery level: {scannedSystem.DiscoveryLevel}");
GD.Print($"Bodies count: {scannedSystem.Bodies.Count}");

foreach (var body in scannedSystem.Bodies)
{
    GD.Print($"  {body.Name}: {body.BodyType}, {body.Composition}");
    if (body.AtmosphereType != null)
        GD.Print($"    Atmosphere: {body.AtmosphereType}");
    if (body.SurfaceType != null)
        GD.Print($"    Surface: {body.SurfaceType}");
}
```

---

## Test 5: Spectral Classification & Colors

### Goal
Verify stars have realistic spectral classifications and luminosities.

### Steps
1. Initialize galaxy
2. Examine spectral classes of generated stars
3. Count distribution

### Expected Results
- ✅ Spectral classes are: O, B, A, F, G, K, or M
- ✅ M-class stars are most common (~76%)
- ✅ O-class stars are very rare (<1%)
- ✅ Each star has positive luminosity value
- ✅ O/B stars have higher luminosity than M stars

### Verification Script
```csharp
var classCounts = new Dictionary<char, int>();
foreach (var system in state.Systems)
{
    if (system.Name == "Sol") continue;
    
    char spectralClass = system.SpectralClass[0];
    if (!classCounts.ContainsKey(spectralClass))
        classCounts[spectralClass] = 0;
    classCounts[spectralClass]++;
}

foreach (var kvp in classCounts.OrderBy(x => x.Key))
{
    GD.Print($"{kvp.Key}-class: {kvp.Value} stars ({kvp.Value * 100.0 / 99:F1}%)");
}
```

---

## Test 6: Star Name Uniqueness

### Goal
Verify all stars have unique procedural names.

### Steps
1. Initialize galaxy
2. Check for duplicate names

### Expected Results
- ✅ All 100 stars have unique names
- ✅ Names follow catalog formats: HD-, Gliese-, Kepler-, LHS-, 2MASS-
- ✅ Sol is the only exception to catalog naming

### Verification
```csharp
var names = state.Systems.Select(s => s.Name).ToList();
var uniqueNames = names.Distinct().ToList();

if (names.Count != uniqueNames.Count)
{
    GD.PrintErr($"ERROR: Found {names.Count - uniqueNames.Count} duplicate names!");
    var duplicates = names.GroupBy(n => n).Where(g => g.Count() > 1);
    foreach (var dup in duplicates)
    {
        GD.PrintErr($"  Duplicate: {dup.Key} ({dup.Count()} times)");
    }
}
else
{
    GD.Print("✅ All star names are unique");
}
```

---

## Test 7: Save/Load with Galaxy

### Goal
Verify galaxy state persists across save/load.

### Steps
1. Initialize galaxy
2. Launch a probe
3. Save the game
4. Load the saved game
5. Verify galaxy state matches

### Expected Results
- ✅ All 100 stars present after load
- ✅ Star positions unchanged
- ✅ Discovery levels preserved
- ✅ Probes in flight restored
- ✅ Bodies lists match

---

## Test 8: Multiple Probes to Same System

### Goal
Verify sending multiple probes to the same system works correctly.

### Steps
1. Launch 2 probes to the same target system
2. Advance time until both arrive

### Expected Results
- ✅ Both probes tracked in "in flight" list
- ✅ Both removed on arrival
- ✅ 2 `ProbeArrived` events emitted
- ✅ System only scanned once (idempotent)

---

## Performance Checks

### Memory Usage
- Monitor memory with 100 stars loaded
- Should be minimal (<10MB for star data)

### Initialization Time
- Galaxy generation should complete in <1 second
- Probe arrival processing should be instant

---

## Visual Verification (Future: Star Map UI)

When the star map UI is implemented, verify:
- ✅ Stars rendered as colored dots (by spectral class)
- ✅ Star size represents luminosity
- ✅ Unscanned stars appear greyed out
- ✅ Scanned stars appear in full color
- ✅ Click selection highlights star
- ✅ System name labels visible
- ✅ Zoom/pan controls work smoothly

---

## Common Issues & Troubleshooting

### Issue: Stars have duplicate names
**Cause**: Hash collision in name generation  
**Fix**: Verify Ulid generation is working properly

### Issue: Probe doesn't arrive
**Cause**: Insufficient time advancement  
**Fix**: Advance time by at least 100 game hours

### Issue: Bodies not generated on scan
**Cause**: Random generation may produce 0 bodies (valid)  
**Fix**: Try another star - some systems may be empty

### Issue: All stars clumped together
**Cause**: Perlin noise function not working  
**Fix**: Check SimplexNoise implementation

---

## Automated Test Results

Run automated tests to verify core functionality:

```powershell
# Galaxy generation tests
dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings --filter "FullyQualifiedName~GalaxyGenerationGdTests"

# Discovery mechanics tests
dotnet test Tests/Outpost3.Tests.csproj --settings:Tests/gdunit4.runsettings --filter "FullyQualifiedName~StarDiscoveryGdTests"
```

Expected: All tests pass (currently 8/10 passing - name determinism tests need fixes)

---

## Next Steps After Manual Testing

1. Implement Star Map UI scene and presenter
2. Add visual star rendering (colored dots, sizes)
3. Implement zoom/pan camera controls
4. Add selection highlighting
5. Wire up UI to existing commands
6. Test complete workflow: init → view map → select star → launch probe → scan → view bodies

---

**Testing Complete Checklist**

- [ ] Galaxy initializes with 100 stars
- [ ] Sol is at center and fully explored
- [ ] All stars within 100 LY radius
- [ ] Stars have unique names and IDs
- [ ] Spectral classes realistic distribution
- [ ] Probe launch works correctly
- [ ] Probe arrival scans system
- [ ] Bodies generated with partial info
- [ ] Discovery levels update properly
- [ ] Save/load preserves galaxy state
- [ ] Performance acceptable
- [ ] No crashes or errors
