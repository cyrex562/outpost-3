# Feature: Galaxy Map & Star Exploration System

**Feature ID**: 13  
**Date**: October 12, 2025  
**Status**: In Progress  
**Architecture**: Event-sourced, pure functional core, presenter pattern

---

## 📋 Requirements Summary

### Core Gameplay Flow

1. **Game Start**: Generate 100 stars in a 100 LY radius around Sol (home system)
2. **Initial State**: All stars except Sol are "Detected" (greyed out, basic info only)
3. **Probe Launch**: Player sends probe to unexplored star
4. **Probe Arrival**: System characteristics re-rolled, bodies partially revealed
5. **Body Exploration**: Additional probes sent to individual bodies for full details
6. **Map Visualization**: 2D top-down scatter plot with zoom/pan controls

### Visual Requirements

- **Star Rendering**:
  - Size represents stellar magnitude (luminosity)
  - Color represents spectral classification (O=blue, B=blue-white, A=white, F=yellow-white, G=yellow, K=orange, M=red)
  - Greyed out if not visited by probe
  - Full color if probe arrived
  - Selected star has green/blue circle highlight
  
- **Star Labels**:
  - System name displayed to lower-right of each star
  - Label visibility adapts to zoom level
  
- **Map Controls**:
  - 2D top-down view (Z-axis not visualized)
  - Pan with mouse drag
  - Zoom with mouse wheel
  - Click star to select

### Discovery Levels

```csharp
public enum DiscoveryLevel
{
    Unknown,      // Not on map (future: hidden systems)
    Detected,     // Visible on map, name + position only
    Scanned,      // Probe visited: star details + body count/types
    Explored      // Bodies individually probed: full details
}
```

### Home System (Sol)

- Name: "Sol"
- Position: (0, 0, 0) - center of map
- Spectral Class: "G2V" (yellow dwarf)
- Initial state: Fully explored
- Bodies: Earth, Mars, Venus, Mercury, Jupiter, Saturn, Uranus, Neptune, etc.

### Procedural Generation

- **Galaxy Shape**: 100 stars within 100 LY radius sphere
- **Distribution**: Perlin noise sampling for realistic clustering
- **Naming**: Procedural catalog names (e.g., "HD-12345", "Gliese-581", "Kepler-442")
- **Star Properties**: Spectral class, luminosity, age, metallicity
- **Deterministic**: Seeded random for reproducibility

### Discovery Mechanics

#### On Probe Arrival (Scanned):
1. Re-roll star characteristics (small chance of change from initial detection)
2. Generate list of bodies with:
   - Body type: Planet, Moon, Asteroid Belt, Cometary Belt
   - Composition: Rocky, Gas Giant, Ice Giant, Asteroid, Comet, Unknown
   - Partial info: Some bodies may reveal atmosphere/surface type (random chance)
3. Emit `SystemScanned` event

#### On Body Probe Arrival (Explored):
1. Reveal full body details:
   - Surface composition
   - Atmospheric composition
   - Temperature
   - Gravity
   - Resources
   - Hazards
2. Emit `BodyExplored` event

---

## 🏗️ Architecture Design

### Domain Model Changes

#### StarSystem (Updated)

```csharp
public record StarSystem
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public Vector3 Position { get; init; }  // NEW: (x, y, z) in light-years
    public float DistanceFromSol { get; init; }  // NEW: calculated distance
    public string SpectralClass { get; init; } = "";
    public float Luminosity { get; init; }  // NEW: for visual size
    public DiscoveryLevel DiscoveryLevel { get; init; }  // NEW: exploration state
    public List<CelestialBody> Bodies { get; init; } = new();
}
```

#### CelestialBody (Updated)

```csharp
public record CelestialBody
{
    public Ulid Id { get; init; }
    public string Name { get; init; } = "";
    public string BodyType { get; init; } = ""; // Planet, Moon, Asteroid Belt, etc.
    public string Composition { get; init; } = ""; // Rocky, Gas Giant, Ice Giant, etc.
    public bool Explored { get; init; } = false;
    
    // Optional partial discovery data (revealed on system scan)
    public string? AtmosphereType { get; init; }  // NEW: null if not yet discovered
    public string? SurfaceType { get; init; }     // NEW: null if not yet discovered
    
    // Full exploration data (revealed on body probe)
    public float? Temperature { get; init; }
    public float? Gravity { get; init; }
    public List<string>? Resources { get; init; }
    public List<string>? Hazards { get; init; }
}
```

#### New Enums

```csharp
public enum DiscoveryLevel
{
    Unknown,
    Detected,
    Scanned,
    Explored
}

public enum SpectralClass
{
    O,  // Blue, hottest
    B,  // Blue-white
    A,  // White
    F,  // Yellow-white
    G,  // Yellow (Sun)
    K,  // Orange
    M   // Red, coolest
}
```

### New Systems

#### GalaxyGenerationSystem

```csharp
public static class GalaxyGenerationSystem
{
    public static List<StarSystem> GenerateGalaxy(int seed, int starCount = 100, float radiusLY = 100f)
    {
        // Generate Sol at center
        // Generate 99 stars using Perlin noise sampling
        // Assign procedural names
        // Calculate distances from Sol
        // All stars except Sol start as Detected
    }
    
    private static Vector3 SamplePositionWithPerlin(Random rng, float radiusLY)
    {
        // Use Perlin noise for realistic star clustering
    }
    
    private static StarSystem GenerateRandomStar(Ulid id, Vector3 position, Random rng)
    {
        // Randomly select spectral class, luminosity, age
    }
}
```

#### Updated TimeSystem

```csharp
private static (GameState, List<IGameEvent>) HandleProbeArrival(GameState state, ProbeInFlight probe)
{
    var events = new List<IGameEvent>();
    
    // Find existing system (Detected level)
    var existingSystem = state.Systems.Find(s => s.Id == probe.TargetSystemId);
    
    if (existingSystem == null)
    {
        // Error: probe sent to unknown system
        return (state, events);
    }
    
    // Re-roll characteristics (small chance of change)
    var scannedSystem = RescanStarSystem(existingSystem, state.GameTime);
    
    // Generate bodies with partial info
    var bodiesWithPartialInfo = GenerateBodiesWithPartialDiscovery(scannedSystem.Id, state.GameTime);
    
    // Update system to Scanned
    var updatedSystem = scannedSystem with 
    { 
        DiscoveryLevel = DiscoveryLevel.Scanned,
        Bodies = bodiesWithPartialInfo
    };
    
    // Replace in state
    var newState = state.WithSystemUpdated(updatedSystem);
    
    events.Add(new SystemScanned(updatedSystem.Id, updatedSystem.Name) 
    { 
        GameTime = (float)state.GameTime 
    });
    
    return (newState, events);
}
```

### New Commands

```csharp
public record InitializeGalaxy(int Seed) : ICommand;
public record SendBodyProbe(Ulid SystemId, Ulid BodyId) : ICommand;
```

### New Events

```csharp
public record GalaxyInitialized(int StarCount, int Seed) : GameEvent;
public record SystemScanned(Ulid SystemId, string SystemName) : GameEvent;
public record BodyExplored(Ulid SystemId, Ulid BodyId, string BodyName) : GameEvent;
```

### UI Components

#### StarMapScene.tscn

```
StarMapScene (Control)
├── Camera2D (for zoom/pan)
├── StarMapContainer (Control)
│   └── [Star nodes dynamically created]
├── UIOverlay (CanvasLayer)
│   ├── SelectedSystemPanel
│   │   ├── SystemNameLabel
│   │   ├── DistanceLabel
│   │   ├── DiscoveryLevelLabel
│   │   ├── SendProbeButton (if Detected)
│   │   └── ViewSystemButton (if Scanned/Explored)
│   └── MapControls
│       ├── ZoomInButton
│       ├── ZoomOutButton
│       └── ResetViewButton
```

#### StarMapPresenter.cs

```csharp
public partial class StarMapPresenter : Control
{
    private Camera2D _camera;
    private Control _starMapContainer;
    private Dictionary<Ulid, StarNode> _starNodes = new();
    private Ulid? _selectedSystemId;
    
    public override void _Ready()
    {
        // Subscribe to StateStore
        // Render all stars from state
        // Set up input handling
    }
    
    private void RenderStars()
    {
        // Create/update star nodes based on state
        // Apply colors, sizes, grey out if not scanned
    }
    
    private void OnStarClicked(Ulid systemId)
    {
        // Emit SelectSystemCommand
    }
    
    private void HighlightSelectedStar()
    {
        // Draw circle around selected star
    }
}
```

#### StarNode (Custom Control)

```csharp
public partial class StarNode : Control
{
    public Ulid SystemId { get; set; }
    public Color StarColor { get; set; }
    public float StarSize { get; set; }
    public bool IsScanned { get; set; }
    public bool IsSelected { get; set; }
    
    public override void _Draw()
    {
        // Draw star circle with appropriate color/size
        // Draw label with system name
        // Draw selection highlight if selected
    }
}
```

---

## 🎯 Implementation Tasks

### Task 1: Update Domain Models (30 min)

**Files to modify:**
- `godot-project/scripts/Core/Domain/StarSystem.cs`
- `godot-project/scripts/Core/Domain/GameState.cs`

**Changes:**
- Add `Vector3 Position`, `float DistanceFromSol`, `float Luminosity`, `DiscoveryLevel` to `StarSystem`
- Add `string Composition`, `string? AtmosphereType`, `string? SurfaceType`, exploration fields to `CelestialBody`
- Create `DiscoveryLevel` enum
- Add `GameState.WithSystemUpdated(StarSystem)` method

### Task 2: Create Galaxy Generation System (45 min)

**New file:**
- `godot-project/scripts/Core/Systems/GalaxyGenerationSystem.cs`

**Features:**
- Perlin noise-based star distribution
- Sol generation at (0,0,0)
- 99 additional stars within 100 LY radius
- Procedural naming system
- Spectral class distribution (more M-class, fewer O/B)
- Luminosity calculation

### Task 3: Update Probe Arrival Logic (30 min)

**Files to modify:**
- `godot-project/scripts/Core/Systems/TimeSystem.cs`

**Changes:**
- Split `GenerateSystem` into `RescanStarSystem` and `GenerateBodiesWithPartialDiscovery`
- Add characteristic re-roll logic (5% chance of variance)
- Add partial body info revelation (30% chance for atmosphere/surface)
- Update discovery level from Detected → Scanned

### Task 4: Create Star Map UI (60 min)

**New files:**
- `godot-project/Scenes/UI/StarMapScene.tscn`
- `godot-project/scripts/UI/StarMapPresenter.cs`
- `godot-project/scripts/UI/StarNode.cs`

**Features:**
- 2D scatter plot rendering
- Camera2D for zoom/pan
- Color-coded stars by spectral class
- Size-coded stars by luminosity
- Grey out unscanned stars
- Click detection and selection highlighting
- System name labels

### Task 5: Add Commands & Events (20 min)

**New files:**
- `godot-project/scripts/Core/Commands/InitializeGalaxy.cs`
- `godot-project/scripts/Core/Commands/SendBodyProbe.cs`
- `godot-project/scripts/Core/Events/GalaxyInitialized.cs`
- `godot-project/scripts/Core/Events/SystemScanned.cs`
- `godot-project/scripts/Core/Events/BodyExplored.cs`

**Updates:**
- `GameEventJsonConverter.cs` - register new events

### Task 6: Write GdUnit4 Tests (45 min)

**New test files:**
- `Tests/GdUnit/GalaxyGenerationGdTests.cs`
- `Tests/GdUnit/StarDiscoveryGdTests.cs`

**Test coverage:**
- Galaxy generation with seed produces deterministic results
- 100 stars generated within radius
- Sol always at center
- Distance calculations correct
- Perlin noise produces clustering
- Probe arrival updates discovery level
- Characteristic re-rolls work
- Partial body info revelation

### Task 7: Manual Testing Guide (15 min)

**New file:**
- `docs/14_manual_testing_star_map.md`

**Content:**
- How to initialize galaxy in Godot editor
- How to test star map rendering
- How to test selection and highlighting
- How to send probes and verify discovery updates
- Visual verification checklist

---

## 📊 Spectral Class → Color Mapping

```csharp
public static class SpectralClassColors
{
    public static Color GetColor(string spectralClass)
    {
        return spectralClass[0] switch
        {
            'O' => new Color(0.6f, 0.7f, 1.0f),    // Blue
            'B' => new Color(0.7f, 0.8f, 1.0f),    // Blue-white
            'A' => new Color(1.0f, 1.0f, 1.0f),    // White
            'F' => new Color(1.0f, 1.0f, 0.9f),    // Yellow-white
            'G' => new Color(1.0f, 1.0f, 0.7f),    // Yellow (Sun)
            'K' => new Color(1.0f, 0.85f, 0.6f),   // Orange
            'M' => new Color(1.0f, 0.6f, 0.5f),    // Red
            _ => new Color(1.0f, 1.0f, 1.0f)       // Default white
        };
    }
    
    public static Color GetGreyedColor(string spectralClass)
    {
        var baseColor = GetColor(spectralClass);
        return new Color(0.4f, 0.4f, 0.4f, 1.0f);  // Grey for unscanned
    }
}
```

---

## 🧪 Testing Strategy

### Unit Tests (GdUnit4)
- Galaxy generation determinism
- Distance calculations
- Discovery state transitions
- Partial data revelation probability
- Characteristic re-roll mechanics

### Integration Tests
- Probe launch → arrival → system update flow
- Multiple probe arrivals to same system
- Body probe exploration

### Manual Tests (Godot Editor)
- Visual star rendering verification
- Map zoom/pan controls
- Star selection and highlighting
- Label readability at different zoom levels
- Performance with 100 stars rendered

---

## 🚀 Deployment Notes

### Performance Considerations
- 100 stars should render efficiently in Godot 2D
- Consider spatial partitioning if map becomes slow
- Label culling at far zoom levels

### Future Enhancements (Post-MVP)
- 3D star map view (toggle between 2D/3D)
- Hidden systems requiring special events to discover
- Dynamic galaxy events (supernovas, black holes)
- Trade routes between systems
- Faction territories on map

---

## ✅ Acceptance Criteria

- [ ] Game start initializes 100 stars around Sol
- [ ] Sol is centered at (0,0,0) and fully explored
- [ ] All other stars start as "Detected" (greyed out)
- [ ] Star map displays 2D scatter plot with correct colors/sizes
- [ ] Clicking star selects it (green/blue highlight)
- [ ] System name labels visible to lower-right of stars
- [ ] Zoom/pan controls work smoothly
- [ ] Sending probe to star updates discovery level
- [ ] Probe arrival re-rolls characteristics
- [ ] Bodies list shows with partial info (some atmosphere/surface data)
- [ ] Unscanned bodies show "Unknown" or "?" for missing data
- [ ] GdUnit4 tests pass for generation and discovery
- [ ] Manual testing guide complete

---

## 📝 Implementation Order

1. ✅ Create this plan document
2. ⏳ Update domain models (StarSystem, CelestialBody, enums)
3. ⏳ Create GalaxyGenerationSystem
4. ⏳ Update probe arrival logic in TimeSystem
5. ⏳ Create star map UI (scene + presenter)
6. ⏳ Write GdUnit4 tests
7. ⏳ Create manual testing guide
8. ⏳ Test in Godot editor
9. ⏳ Document completion and next steps
