# Session 2.2 Implementation Plan: Star System Map Scene

## Overview
Create an orbital view scene showing the star system with orbits, animated celestial bodies, interactive controls, and a toggleable overview panel. This scene is the primary view for exploring and managing activities within a star system.

---

## 1. Core Domain (Game.Core)

### 1.1 New Value Types

**File:** `src/Game.Core/Domain/ValueTypes.cs`

```csharp
// Orbital parameters for celestial bodies
public record OrbitalParameters(
    double SemiMajorAxisAU,      // Distance from star in AU
    double OrbitalPeriodDays,    // Time to complete one orbit
    double StartingAngleDegrees, // Initial position on orbit (0-360)
    double Eccentricity          // 0 = circular, >0 = elliptical
);

// Camera state for viewport persistence
public record CameraState(
    Vector2 PanPosition,  // Camera offset from center
    float ZoomLevel       // Zoom multiplier (1.0 = default)
);

// Screen navigation identifier
public record ScreenId(string Value)
{
    public static ScreenId GalaxyMap => new("GalaxyMap");
    public static ScreenId StarSystemMap => new("StarSystemMap");
    public static ScreenId ShipJourneyLog => new("ShipJourneyLog");
    public static ScreenId SystemDetails => new("SystemDetails");
}

// Seed for deterministic generation
public record SystemSeed(string Value) // Alphanumeric, uppercase
{
    public static SystemSeed FromSystemId(SystemId id) => 
        new(GenerateSeedFromUlid(id.Value));
    
    private static string GenerateSeedFromUlid(string ulid) =>
        // Hash ULID to create 8-character alphanumeric seed
        Convert.ToBase64String(
            System.Security.Cryptography.SHA256.HashData(
                System.Text.Encoding.UTF8.GetBytes(ulid)
            )
        ).Replace("+", "").Replace("/", "").Substring(0, 8).ToUpperInvariant();
}
```

### 1.2 Extended CelestialBody

**File:** `src/Game.Core/Domain/CelestialBody.cs`

```csharp
public record CelestialBody(
    BodyId Id,
    string Name,
    string BodyType,
    bool Explored,
    OrbitalParameters? OrbitalParams,  // NEW: null for star, set for orbiting bodies
    double MassEarthMasses,            // NEW
    double RadiusKm                    // NEW
);
```

### 1.3 Extended StarSystem

**File:** `src/Game.Core/Domain/StarSystem.cs`

```csharp
public record StarSystem(
    SystemId Id,
    string Name,
    Vector2 GalacticPosition,
    string SpectralClass,
    bool HasProbeData,
    SystemSeed Seed,                      // NEW: for deterministic generation
    ImmutableList<CelestialBody> Bodies,
    ImmutableList<AsteroidBelt> Belts,   // NEW
    OortCloud OortCloud                   // NEW
);

public record AsteroidBelt(
    BeltId Id,
    string Name,
    double InnerRadiusAU,
    double OuterRadiusAU
);

public record OortCloud(
    double RadiusAU  // Outermost extent of system
);
```

### 1.4 GameState Extensions

**File:** `src/Game.Core/Domain/GameState.cs`

```csharp
public record GameState(
    // ... existing fields ...
    SystemId? SelectedSystemId,                           // NEW
    BodyId? SelectedBodyId,                              // NEW
    ImmutableStack<ScreenId> NavigationStack,            // NEW: FILO screen history
    ImmutableDictionary<SystemId, CameraState> CameraStates, // NEW: per-system camera
    bool SystemOverviewPanelOpen,                        // NEW: panel toggle state
    GameSpeed CurrentSpeed,                              // NEW
    bool IsPaused                                        // NEW
);

public enum GameSpeed
{
    Paused = 0,
    Normal = 1,
    Fast = 2,
    Faster = 5,
    Fastest = 10
}
```

---

## 2. Commands

**File:** `src/Game.Core/Commands/StarSystemCommands.cs`

```csharp
// Navigation commands
public record PushScreen(ScreenId Screen) : ICommand;
public record PopScreen() : ICommand;
public record NavigateToScreen(ScreenId Screen) : ICommand;

// System generation command
public record GenerateSystemDetails(SystemId SystemId, Tick CurrentTick) : ICommand;

// Selection commands
public record SelectCelestialBody(BodyId? BodyId) : ICommand;  // null = deselect

// Camera commands
public record UpdateCamera(SystemId SystemId, CameraState State) : ICommand;
public record ResetCamera(SystemId SystemId) : ICommand;

// UI state commands
public record ToggleSystemOverviewPanel() : ICommand;

// Time control commands
public record SetGameSpeed(GameSpeed Speed) : ICommand;
public record TogglePause() : ICommand;
```

---

## 3. Events

**File:** `src/Game.Core/Events/StarSystemEvents.cs`

```csharp
// Navigation events
public record ScreenPushed(ScreenId Screen, Tick Timestamp) : IEvent;
public record ScreenPopped(ScreenId Screen, Tick Timestamp) : IEvent;
public record NavigatedToScreen(ScreenId Screen, Tick Timestamp) : IEvent;

// System generation event
public record SystemDetailsGenerated(
    SystemId SystemId,
    SystemSeed Seed,
    ImmutableList<CelestialBody> GeneratedBodies,
    ImmutableList<AsteroidBelt> GeneratedBelts,
    OortCloud OortCloud,
    Tick Timestamp
) : IEvent;

// Selection events
public record CelestialBodySelected(BodyId? BodyId, Tick Timestamp) : IEvent;

// Camera events
public record CameraUpdated(SystemId SystemId, CameraState State, Tick Timestamp) : IEvent;
public record CameraReset(SystemId SystemId, Tick Timestamp) : IEvent;

// UI state events
public record SystemOverviewPanelToggled(bool IsOpen, Tick Timestamp) : IEvent;

// Time control events
public record GameSpeedChanged(GameSpeed NewSpeed, Tick Timestamp) : IEvent;
public record GamePauseToggled(bool IsPaused, Tick Timestamp) : IEvent;
```

---

## 4. Reducers

**File:** `src/Game.Core/Reducers/StarSystemReducer.cs`

```csharp
public static class StarSystemReducer
{
    public static (GameState, ImmutableList<IEvent>) Handle(
        GameState state, 
        GenerateSystemDetails cmd)
    {
        var system = state.DiscoveredSystems
            .FirstOrDefault(s => s.Id == cmd.SystemId);
        
        if (system == null || system.Bodies.Any(b => b.OrbitalParams != null))
            return (state, ImmutableList<IEvent>.Empty);  // Already generated
        
        var seed = SystemSeed.FromSystemId(cmd.SystemId);
        var (bodies, belts, oortCloud) = ProceduralGenerator.GenerateSystemDetails(
            system, 
            seed
        );
        
        var evt = new SystemDetailsGenerated(
            cmd.SystemId,
            seed,
            bodies,
            belts,
            oortCloud,
            cmd.CurrentTick
        );
        
        return (state, ImmutableList.Create<IEvent>(evt));
    }
    
    public static GameState Apply(GameState state, SystemDetailsGenerated evt)
    {
        var updatedSystems = state.DiscoveredSystems.Select(sys =>
            sys.Id == evt.SystemId
                ? sys with 
                  { 
                      Seed = evt.Seed,
                      Bodies = evt.GeneratedBodies,
                      Belts = evt.GeneratedBelts,
                      OortCloud = evt.OortCloud
                  }
                : sys
        ).ToImmutableList();
        
        return state with { DiscoveredSystems = updatedSystems };
    }
    
    // Additional Apply methods for selection, camera, navigation, etc.
    public static GameState Apply(GameState state, CelestialBodySelected evt) =>
        state with { SelectedBodyId = evt.BodyId };
    
    public static GameState Apply(GameState state, ScreenPushed evt) =>
        state with { NavigationStack = state.NavigationStack.Push(evt.Screen) };
    
    public static GameState Apply(GameState state, ScreenPopped evt)
    {
        if (state.NavigationStack.IsEmpty) return state;
        return state with { NavigationStack = state.NavigationStack.Pop() };
    }
    
    public static GameState Apply(GameState state, CameraUpdated evt) =>
        state with 
        { 
            CameraStates = state.CameraStates.SetItem(evt.SystemId, evt.State) 
        };
    
    public static GameState Apply(GameState state, SystemOverviewPanelToggled evt) =>
        state with { SystemOverviewPanelOpen = evt.IsOpen };
    
    public static GameState Apply(GameState state, GameSpeedChanged evt) =>
        state with { CurrentSpeed = evt.NewSpeed };
    
    public static GameState Apply(GameState state, GamePauseToggled evt) =>
        state with { IsPaused = evt.IsPaused };
}
```

---

## 5. Procedural Generation

**File:** `src/Game.Core/Systems/ProceduralGenerator.cs`

```csharp
public static class ProceduralGenerator
{
    public static (
        ImmutableList<CelestialBody> bodies,
        ImmutableList<AsteroidBelt> belts,
        OortCloud oortCloud
    ) GenerateSystemDetails(StarSystem system, SystemSeed seed)
    {
        var random = new Random(seed.Value.GetHashCode());
        
        // Generate orbital parameters for existing bodies
        var bodiesWithOrbits = system.Bodies.Select((body, index) =>
        {
            var distance = GenerateOrbitalDistance(index, random);
            var period = CalculateOrbitalPeriod(distance);
            var startAngle = random.NextDouble() * 360.0;
            var eccentricity = random.NextDouble() * 0.1; // Low eccentricity
            
            var orbitalParams = new OrbitalParameters(
                distance,
                period,
                startAngle,
                eccentricity
            );
            
            var mass = GenerateMass(body.BodyType, random);
            var radius = GenerateRadius(body.BodyType, mass, random);
            
            return body with 
            { 
                OrbitalParams = orbitalParams,
                MassEarthMasses = mass,
                RadiusKm = radius
            };
        }).ToImmutableList();
        
        // Generate asteroid belts (0-2)
        var belts = GenerateAsteroidBelts(bodiesWithOrbits, random);
        
        // Generate Oort cloud
        var maxOrbit = bodiesWithOrbits.Max(b => b.OrbitalParams!.SemiMajorAxisAU);
        var oortCloud = new OortCloud(maxOrbit * 100); // ~100x furthest planet
        
        return (bodiesWithOrbits, belts, oortCloud);
    }
    
    private static double GenerateOrbitalDistance(int index, Random random)
    {
        // Titius-Bode-like law with randomization
        var baseDistance = 0.4 + (0.3 * Math.Pow(2, index));
        return baseDistance * (0.8 + random.NextDouble() * 0.4);
    }
    
    private static double CalculateOrbitalPeriod(double semiMajorAxisAU)
    {
        // Kepler's third law (simplified, assuming solar mass)
        return 365.25 * Math.Pow(semiMajorAxisAU, 1.5);
    }
    
    private static double GenerateMass(string bodyType, Random random) =>
        bodyType switch
        {
            "Gas Giant" => 50 + random.NextDouble() * 300,   // 50-350 Earth masses
            "Ice Giant" => 10 + random.NextDouble() * 20,     // 10-30 Earth masses
            "Terrestrial" => 0.1 + random.NextDouble() * 5,   // 0.1-5 Earth masses
            "Dwarf" => 0.001 + random.NextDouble() * 0.1,     // Very small
            _ => 1.0
        };
    
    private static double GenerateRadius(string bodyType, double mass, Random random) =>
        bodyType switch
        {
            "Gas Giant" => 40000 + random.NextDouble() * 40000,
            "Ice Giant" => 20000 + random.NextDouble() * 30000,
            "Terrestrial" => 3000 + random.NextDouble() * 10000,
            "Dwarf" => 1000 + random.NextDouble() * 3000,
            _ => 6371
        };
    
    private static ImmutableList<AsteroidBelt> GenerateAsteroidBelts(
        ImmutableList<CelestialBody> bodies,
        Random random)
    {
        var belts = ImmutableList.CreateBuilder<AsteroidBelt>();
        var beltCount = random.Next(0, 3); // 0-2 belts
        
        for (int i = 0; i < beltCount; i++)
        {
            // Place belt between planets
            var innerRadius = 1.5 + i * 2.0 + random.NextDouble();
            var outerRadius = innerRadius + 0.5 + random.NextDouble() * 0.5;
            
            belts.Add(new AsteroidBelt(
                new BeltId(Ulid.NewUlid().ToString()),
                $"Asteroid Belt {i + 1}",
                innerRadius,
                outerRadius
            ));
        }
        
        return belts.ToImmutable();
    }
}
```

---

## 6. Projections

**File:** `src/Game.App/Projections/StarSystemMapProjection.cs`

```csharp
public record StarSystemMapViewModel(
    SystemId SystemId,
    string SystemName,
    string SpectralClass,
    SystemSeed Seed,
    Color StarColor,
    ImmutableList<BodyViewModel> Bodies,
    ImmutableList<BeltViewModel> Belts,
    OortCloud OortCloud,
    BodyId? SelectedBodyId,
    CameraState Camera,
    bool OverviewPanelOpen,
    GameSpeed CurrentSpeed,
    bool IsPaused,
    Tick CurrentTick
);

public record BodyViewModel(
    BodyId Id,
    string Name,
    string BodyType,
    OrbitalParameters OrbitalParams,
    double CurrentAngleDegrees,  // Updated each tick
    Color DisplayColor,
    float DisplayRadius
);

public record BeltViewModel(
    BeltId Id,
    string Name,
    double InnerRadiusAU,
    double OuterRadiusAU,
    Color DisplayColor
);

public class StarSystemMapProjection
{
    private StarSystemMapViewModel? _current;
    
    public void Apply(GameState state)
    {
        if (state.SelectedSystemId == null)
        {
            _current = null;
            return;
        }
        
        var system = state.DiscoveredSystems
            .FirstOrDefault(s => s.Id == state.SelectedSystemId);
        
        if (system == null) return;
        
        var starColor = SpectralClassColors.GetColor(system.SpectralClass);
        
        var bodies = system.Bodies
            .Where(b => b.OrbitalParams != null)
            .Select(b => new BodyViewModel(
                b.Id,
                b.Name,
                b.BodyType,
                b.OrbitalParams!,
                CalculateCurrentAngle(b.OrbitalParams!, state.CurrentTick),
                BodyTypeColors.GetColor(b.BodyType),
                CalculateDisplayRadius(b.RadiusKm)
            ))
            .ToImmutableList();
        
        var belts = system.Belts
            .Select(b => new BeltViewModel(
                b.Id,
                b.Name,
                b.InnerRadiusAU,
                b.OuterRadiusAU,
                new Color(0.6f, 0.6f, 0.5f, 0.3f)
            ))
            .ToImmutableList();
        
        var camera = state.CameraStates.TryGetValue(system.Id, out var cam)
            ? cam
            : new CameraState(Vector2.Zero, 1.0f);
        
        _current = new StarSystemMapViewModel(
            system.Id,
            system.Name,
            system.SpectralClass,
            system.Seed,
            starColor,
            bodies,
            belts,
            system.OortCloud,
            state.SelectedBodyId,
            camera,
            state.SystemOverviewPanelOpen,
            state.CurrentSpeed,
            state.IsPaused,
            state.CurrentTick
        );
    }
    
    public StarSystemMapViewModel? Current => _current;
    
    private static double CalculateCurrentAngle(OrbitalParameters orbit, Tick currentTick)
    {
        // Simple linear motion (can be improved with proper orbital mechanics)
        var daysPassed = currentTick.Value / 86400.0; // Assuming 1 tick = 1 second
        var orbitalProgress = (daysPassed % orbit.OrbitalPeriodDays) / orbit.OrbitalPeriodDays;
        return (orbit.StartingAngleDegrees + orbitalProgress * 360.0) % 360.0;
    }
    
    private static float CalculateDisplayRadius(double radiusKm)
    {
        // Scale radius for display (logarithmic scale for visibility)
        return (float)(Math.Log10(radiusKm) * 5.0f);
    }
}

public static class SpectralClassColors
{
    public static Color GetColor(string spectralClass) =>
        spectralClass[0] switch
        {
            'O' => new Color(0.6f, 0.7f, 1.0f),   // Blue
            'B' => new Color(0.7f, 0.8f, 1.0f),   // Blue-white
            'A' => new Color(0.9f, 0.9f, 1.0f),   // White
            'F' => new Color(1.0f, 1.0f, 0.9f),   // Yellow-white
            'G' => new Color(1.0f, 1.0f, 0.7f),   // Yellow
            'K' => new Color(1.0f, 0.8f, 0.6f),   // Orange
            'M' => new Color(1.0f, 0.6f, 0.5f),   // Red
            _ => new Color(1.0f, 1.0f, 1.0f)
        };
}

public static class BodyTypeColors
{
    public static Color GetColor(string bodyType) =>
        bodyType switch
        {
            "Gas Giant" => new Color(0.8f, 0.7f, 0.5f),
            "Ice Giant" => new Color(0.5f, 0.7f, 0.9f),
            "Terrestrial" => new Color(0.6f, 0.5f, 0.4f),
            "Dwarf" => new Color(0.5f, 0.5f, 0.5f),
            _ => new Color(0.7f, 0.7f, 0.7f)
        };
}
```

---

## 7. Godot Scene Structure

### 7.1 Scene Hierarchy

**File:** `godot-project/Scenes/StarSystem/StarSystemMap.tscn`

```
StarSystemMap (Control - full screen)
├── ViewportContainer (fills screen)
│   └── SubViewport
│       └── StarSystemView (Node2D - the zoomable/pannable content)
│           ├── Star (Sprite2D or ColorRect - at origin)
│           ├── OrbitsContainer (Node2D - orbital rings)
│           ├── BeltsContainer (Node2D - asteroid belts)
│           ├── BodiesContainer (Node2D - planet icons)
│           └── SelectionRing (Sprite2D - follows selected body)
├── UI (CanvasLayer - always on top)
│   ├── HeaderPanel (PanelContainer - top)
│   │   ├── HBox
│   │   │   ├── SystemNameLabel
│   │   │   ├── VSeparator
│   │   │   ├── GameTimeLabel
│   │   │   ├── VSeparator
│   │   │   └── GameSpeedLabel
│   │   └── TimeControls (HBox - right side)
│   │       ├── PauseButton
│   │       ├── SlowerButton (-)
│   │       ├── FasterButton (+)
│   │       └── SpeedLabel
│   ├── BottomBar (HBox - bottom)
│   │   ├── BackButton
│   │   ├── ResetCameraButton (Home icon)
│   │   └── ToggleOverviewButton (right side)
│   └── SystemOverviewPanel (PanelContainer - right side, toggleable)
│       └── TabContainer
│           ├── Bodies Tab (VBox with ScrollContainer)
│           └── Messages Tab (VBox with ScrollContainer)
└── TooltipPanel (PanelContainer - follows mouse, initially hidden)
```

### 7.2 Body Icon Component

**File:** `godot-project/Scenes/StarSystem/BodyIcon.tscn`

```
BodyIcon (Node2D)
├── Sprite (Sprite2D or ColorRect)
└── Area2D (for mouse detection)
    └── CollisionShape2D
```

---

## 8. Presenter

**File:** `godot-project/scripts/UI/StarSystemMapPresenter.cs`

```csharp
public partial class StarSystemMapPresenter : Control
{
    // Scene nodes
    private Node2D _starSystemView;
    private Node2D _star;
    private Node2D _orbitsContainer;
    private Node2D _beltsContainer;
    private Node2D _bodiesContainer;
    private Sprite2D _selectionRing;
    
    private Label _systemNameLabel;
    private Label _gameTimeLabel;
    private Label _gameSpeedLabel;
    private Button _pauseButton;
    private Button _slowerButton;
    private Button _fasterButton;
    private Button _backButton;
    private Button _resetCameraButton;
    private Button _toggleOverviewButton;
    private PanelContainer _systemOverviewPanel;
    private VBoxContainer _bodiesListContainer;
    private VBoxContainer _messagesListContainer;
    private PanelContainer _tooltipPanel;
    
    // State
    private StarSystemMapProjection _projection;
    private Dictionary<BodyId, Node2D> _bodyNodes = new();
    private Camera2D _camera;
    private bool _isPanning;
    private Vector2 _lastMousePosition;
    
    // Constants
    private const float MIN_ZOOM = 0.1f;
    private const float MAX_ZOOM = 5.0f;
    private const float ZOOM_STEP = 0.1f;
    private const float PAN_SPEED = 5.0f;
    private const float AU_TO_PIXELS = 100.0f; // 1 AU = 100 pixels at zoom 1.0
    
    public override void _Ready()
    {
        // Get node references (omitted for brevity)
        // ...
        
        // Setup camera
        _camera = new Camera2D();
        _starSystemView.AddChild(_camera);
        
        // Wire up signals
        _backButton.Pressed += OnBackPressed;
        _resetCameraButton.Pressed += OnResetCamera;
        _toggleOverviewButton.Pressed += OnToggleOverview;
        _pauseButton.Pressed += OnPausePressed;
        _slowerButton.Pressed += OnSlowerPressed;
        _fasterButton.Pressed += OnFasterPressed;
        
        // Subscribe to projection
        _projection = ServiceLocator.Get<StarSystemMapProjection>();
        _projection.OnChanged += OnProjectionChanged;
        
        // Initial generation if needed
        var state = StateStore.Current;
        if (state.SelectedSystemId != null)
        {
            CommandBus.Send(new GenerateSystemDetails(
                state.SelectedSystemId,
                state.CurrentTick
            ));
            CommandBus.Send(new PushScreen(ScreenId.StarSystemMap));
        }
    }
    
    public override void _Process(double delta)
    {
        HandleKeyboardInput();
        UpdateOrbitalPositions();
        UpdateSelectionRing();
    }
    
    public override void _UnhandledInput(InputEvent @event)
    {
        if (@event is InputEventMouseButton mouseButton)
        {
            HandleMouseButton(mouseButton);
        }
        else if (@event is InputEventMouseMotion mouseMotion)
        {
            HandleMouseMotion(mouseMotion);
        }
    }
    
    private void OnProjectionChanged()
    {
        var vm = _projection.Current;
        if (vm == null) return;
        
        // Update header
        _systemNameLabel.Text = $"{vm.SystemName} ({vm.SpectralClass})";
        _gameTimeLabel.Text = FormatGameTime(vm.CurrentTick);
        _gameSpeedLabel.Text = vm.IsPaused ? "PAUSED" : $"{vm.CurrentSpeed}x";
        
        // Update camera
        _camera.Position = vm.Camera.PanPosition;
        _camera.Zoom = Vector2.One * vm.Camera.ZoomLevel;
        
        // Update overview panel
        _systemOverviewPanel.Visible = vm.OverviewPanelOpen;
        
        // Rebuild star
        UpdateStar(vm.StarColor);
        
        // Rebuild orbits
        RebuildOrbits(vm.Bodies, vm.Belts, vm.OortCloud);
        
        // Rebuild bodies
        RebuildBodies(vm.Bodies);
        
        // Update bodies list
        UpdateBodiesList(vm.Bodies);
    }
    
    private void HandleMouseButton(InputEventMouseButton mouseButton)
    {
        if (mouseButton.ButtonIndex == MouseButton.Right)
        {
            _isPanning = mouseButton.Pressed;
            _lastMousePosition = mouseButton.Position;
        }
        else if (mouseButton.ButtonIndex == MouseButton.Left && !mouseButton.Pressed)
        {
            // Check if clicked on a body
            var clickedBody = GetBodyAtPosition(mouseButton.Position);
            if (clickedBody != null)
            {
                if (mouseButton.DoubleClick)
                {
                    OpenBodyDetails(clickedBody);
                }
                else
                {
                    SelectBody(clickedBody);
                }
            }
            else
            {
                DeselectBody();
            }
        }
        else if (mouseButton.ButtonIndex == MouseButton.WheelUp)
        {
            ZoomAt(mouseButton.Position, ZOOM_STEP);
        }
        else if (mouseButton.ButtonIndex == MouseButton.WheelDown)
        {
            ZoomAt(mouseButton.Position, -ZOOM_STEP);
        }
    }
    
    private void HandleMouseMotion(InputEventMouseMotion mouseMotion)
    {
        if (_isPanning)
        {
            var delta = mouseMotion.Position - _lastMousePosition;
            _camera.Position -= delta / _camera.Zoom.X;
            _lastMousePosition = mouseMotion.Position;
            SaveCameraState();
        }
        
        // Update tooltip
        UpdateTooltip(mouseMotion.Position);
    }
    
    private void HandleKeyboardInput()
    {
        var delta = Vector2.Zero;
        
        if (Input.IsActionPressed("ui_up") || Input.IsKeyPressed(Key.W))
            delta.Y -= PAN_SPEED;
        if (Input.IsActionPressed("ui_down") || Input.IsKeyPressed(Key.S))
            delta.Y += PAN_SPEED;
        if (Input.IsActionPressed("ui_left") || Input.IsKeyPressed(Key.A))
            delta.X -= PAN_SPEED;
        if (Input.IsActionPressed("ui_right") || Input.IsKeyPressed(Key.D))
            delta.X += PAN_SPEED;
        
        if (delta != Vector2.Zero)
        {
            _camera.Position += delta / _camera.Zoom.X;
            SaveCameraState();
        }
        
        if (Input.IsActionJustPressed("ui_cancel")) // ESC
        {
            if (_systemOverviewPanel.Visible)
                OnToggleOverview();
            else
                OnBackPressed();
        }
        
        if (Input.IsKeyPressed(Key.Home))
        {
            OnResetCamera();
        }
        
        if (Input.IsKeyPressed(Key.Z))
        {
            ZoomAt(GetViewportRect().Size / 2, ZOOM_STEP);
        }
        
        if (Input.IsKeyPressed(Key.X))
        {
            ZoomAt(GetViewportRect().Size / 2, -ZOOM_STEP);
        }
        
        if (Input.IsActionJustPressed("ui_select")) // Space
        {
            OnPausePressed();
        }
    }
    
    private void UpdateOrbitalPositions()
    {
        var vm = _projection.Current;
        if (vm == null || vm.IsPaused) return;
        
        foreach (var body in vm.Bodies)
        {
            if (_bodyNodes.TryGetValue(body.Id, out var node))
            {
                var angleRad = body.CurrentAngleDegrees * Math.PI / 180.0;
                var distance = body.OrbitalParams.SemiMajorAxisAU * AU_TO_PIXELS;
                node.Position = new Vector2(
                    (float)(Math.Cos(angleRad) * distance),
                    (float)(Math.Sin(angleRad) * distance)
                );
            }
        }
    }
    
    private void ZoomAt(Vector2 screenPosition, float zoomDelta)
    {
        var oldZoom = _camera.Zoom.X;
        var newZoom = Mathf.Clamp(oldZoom + zoomDelta, MIN_ZOOM, MAX_ZOOM);
        
        // Zoom toward mouse position
        var worldPos = _camera.GetGlobalMousePosition();
        _camera.Zoom = Vector2.One * newZoom;
        var newWorldPos = _camera.GetGlobalMousePosition();
        _camera.Position += worldPos - newWorldPos;
        
        SaveCameraState();
    }
    
    private void SaveCameraState()
    {
        var vm = _projection.Current;
        if (vm == null) return;
        
        CommandBus.Send(new UpdateCamera(
            vm.SystemId,
            new CameraState(_camera.Position, _camera.Zoom.X)
        ));
    }
    
    private void OnResetCamera()
    {
        var vm = _projection.Current;
        if (vm == null) return;
        
        CommandBus.Send(new ResetCamera(vm.SystemId));
    }
    
    private void OnBackPressed()
    {
        CommandBus.Send(new PopScreen());
    }
    
    private void OnToggleOverview()
    {
        CommandBus.Send(new ToggleSystemOverviewPanel());
    }
    
    private void OnPausePressed()
    {
        CommandBus.Send(new TogglePause());
    }
    
    private void OnSlowerPressed()
    {
        var vm = _projection.Current;
        if (vm == null) return;
        
        var newSpeed = vm.CurrentSpeed switch
        {
            GameSpeed.Fastest => GameSpeed.Faster,
            GameSpeed.Faster => GameSpeed.Fast,
            GameSpeed.Fast => GameSpeed.Normal,
            _ => GameSpeed.Normal
        };
        
        CommandBus.Send(new SetGameSpeed(newSpeed));
    }
    
    private void OnFasterPressed()
    {
        var vm = _projection.Current;
        if (vm == null) return;
        
        var newSpeed = vm.CurrentSpeed switch
        {
            GameSpeed.Normal => GameSpeed.Fast,
            GameSpeed.Fast => GameSpeed.Faster,
            GameSpeed.Faster => GameSpeed.Fastest,
            _ => GameSpeed.Fastest
        };
        
        CommandBus.Send(new SetGameSpeed(newSpeed));
    }
    
    // Additional helper methods for rendering orbits, bodies, etc.
    // ... (omitted for brevity)
}
```

---

## 9. Testing (GdUnit4)

**File:** `godot-project/tests/StarSystemMapTests.gd`

```gdscript
extends GdUnitTestSuite

func test_system_generation():
    var system_id = SystemId.new("test-system")
    var seed = SystemSeed.from_system_id(system_id)
    assert_that(seed.Value).is_not_empty()
    
func test_orbital_calculation():
    var orbit = OrbitalParameters.new(1.0, 365.25, 0.0, 0.0)
    var angle = StarSystemMapProjection.calculate_current_angle(orbit, Tick.new(0))
    assert_that(angle).is_equal(0.0)

func test_camera_persistence():
    var state = GameState.initial()
    var system_id = SystemId.new("test")
    var camera = CameraState.new(Vector2(100, 100), 2.0)
    
    var cmd = UpdateCamera.new(system_id, camera)
    var (new_state, events) = StarSystemReducer.handle(state, cmd)
    
    assert_that(new_state.CameraStates.has(system_id)).is_true()
    assert_that(new_state.CameraStates[system_id].ZoomLevel).is_equal(2.0)

func test_navigation_stack():
    var state = GameState.initial()
    state = StarSystemReducer.apply(state, ScreenPushed.new(ScreenId.GalaxyMap, Tick.new(0)))
    state = StarSystemReducer.apply(state, ScreenPushed.new(ScreenId.StarSystemMap, Tick.new(1)))
    
    assert_that(state.NavigationStack.count()).is_equal(2)
    
    state = StarSystemReducer.apply(state, ScreenPopped.new(ScreenId.StarSystemMap, Tick.new(2)))
    assert_that(state.NavigationStack.peek()).is_equal(ScreenId.GalaxyMap)
```

---

## 10. Implementation Checklist

### Phase 1: Core Domain (30 min)
- [ ] Add new value types (OrbitalParameters, CameraState, ScreenId, SystemSeed)
- [ ] Extend CelestialBody with orbital params, mass, radius
- [ ] Extend StarSystem with belts, Oort cloud, seed
- [ ] Extend GameState with selection, navigation, camera, speed, pause

### Phase 2: Commands & Events (15 min)
- [ ] Create all navigation commands/events
- [ ] Create system generation commands/events
- [ ] Create selection commands/events
- [ ] Create camera commands/events
- [ ] Create UI state commands/events
- [ ] Create time control commands/events

### Phase 3: Reducers (20 min)
- [ ] Implement GenerateSystemDetails handler
- [ ] Implement all Apply methods in StarSystemReducer
- [ ] Wire up reducers in StateStore

### Phase 4: Procedural Generation (30 min)
- [ ] Implement SystemSeed generation from ULID
- [ ] Implement orbital distance generation (Titius-Bode)
- [ ] Implement mass/radius generation by body type
- [ ] Implement asteroid belt generation
- [ ] Implement Oort cloud generation

### Phase 5: Projection (20 min)
- [ ] Create StarSystemMapViewModel
- [ ] Implement StarSystemMapProjection
- [ ] Add color helpers (spectral class, body type)
- [ ] Implement current angle calculation
- [ ] Register projection in app startup

### Phase 6: Godot Scene (30 min)
- [ ] Create StarSystemMap.tscn hierarchy
- [ ] Create BodyIcon.tscn component
- [ ] Set up ViewportContainer and SubViewport
- [ ] Create UI panels (header, bottom bar, overview)
- [ ] Style with theme

### Phase 7: Presenter Logic (45 min)
- [ ] Create StarSystemMapPresenter.cs
- [ ] Implement node references and setup
- [ ] Implement projection subscription
- [ ] Implement star rendering
- [ ] Implement orbit rendering
- [ ] Implement body rendering with icons
- [ ] Implement belt/Oort cloud rendering

### Phase 8: Camera & Input (25 min)
- [ ] Implement mouse pan (right-click drag)
- [ ] Implement mouse zoom (wheel)
- [ ] Implement keyboard pan (WASD/arrows)
- [ ] Implement keyboard zoom (Z/X)
- [ ] Implement zoom-to-mouse-position
- [ ] Implement camera reset (Home key)
- [ ] Implement camera state persistence

### Phase 9: Interaction (25 min)
- [ ] Implement body hover detection
- [ ] Implement tooltip display
- [ ] Implement single-click selection
- [ ] Implement double-click details modal
- [ ] Implement selection ring rendering
- [ ] Implement click-empty-space-to-deselect

### Phase 10: UI Controls (20 min)
- [ ] Wire up back button (navigation stack pop)
- [ ] Wire up pause/resume button
- [ ] Wire up speed controls (+/-)
- [ ] Wire up toggle overview panel
- [ ] Update header labels from projection
- [ ] Implement bodies list in overview panel

### Phase 11: Orbital Motion (15 min)
- [ ] Implement per-tick orbital position updates
- [ ] Respect pause state
- [ ] Respect game speed multiplier
- [ ] Test orbital motion visually

### Phase 12: Testing (20 min)
- [ ] Write GdUnit4 tests for procedural generation
- [ ] Write GdUnit4 tests for orbital calculations
- [ ] Write GdUnit4 tests for camera persistence
- [ ] Write GdUnit4 tests for navigation stack
- [ ] Run all tests and fix issues

### Phase 13: Polish & Bug Fixes (15 min)
- [ ] Test all hotkeys
- [ ] Test all mouse interactions
- [ ] Test navigation flow (from galaxy map and back)
- [ ] Test with multiple systems
- [ ] Fix any visual glitches

---

## 11. Estimated Time: **75 minutes** (as per roadmap)

**Breakdown:**
- Domain/Commands/Events/Reducers: 65 min
- Procedural Generation: 30 min
- Projection: 20 min
- Scene Setup: 30 min
- Presenter Implementation: 70 min
- Camera & Input: 25 min
- Interaction: 25 min
- UI Controls: 20 min
- Orbital Motion: 15 min
- Testing: 20 min
- Polish: 15 min

**Total:** ~335 minutes (~5.5 hours)

**Recommendation:** Split into sub-sessions if needed, or allocate multiple work sessions.

---

## 12. Success Criteria

- [ ] Can navigate to Star System Map from System Details Modal
- [ ] System details are procedurally generated on first view
- [ ] Star is rendered at center with correct spectral color
- [ ] Orbital rings are drawn for all bodies
- [ ] Bodies move along orbits based on game speed
- [ ] Asteroid belts and Oort cloud are visible
- [ ] Can select bodies with single click (shows selection ring)
- [ ] Can open body details with double-click
- [ ] Hover shows tooltip with body name/type/distance
- [ ] Camera can pan with mouse (right-click drag) and keyboard (WASD/arrows)
- [ ] Camera can zoom with mouse wheel and keyboard (Z/X)
- [ ] Zoom centers on mouse position
- [ ] Reset camera button returns to default view
- [ ] Home key resets camera
- [ ] Back button pops navigation stack
- [ ] ESC key goes back or closes overview panel
- [ ] System overview panel can be toggled
- [ ] Bodies list in overview panel works and syncs with map selection
- [ ] Pause button freezes orbital motion
- [ ] Speed controls change game speed
- [ ] Header displays system name, time, and speed correctly
- [ ] Camera state persists when navigating away and back
- [ ] Navigation stack works correctly (back button behavior)
- [ ] All GdUnit4 tests pass

---

## 13. Future Enhancements (Post-Session 2.2)

- Moons as sub-orbits around parent bodies
- Elliptical orbits (use eccentricity parameter)
- More detailed star rendering (gradient, glow shader)
- Body textures/sprites instead of colored circles
- Orbital trail lines showing recent path
- Minimap for quick navigation
- Click-and-drag to measure distances
- Bookmark/favorite specific bodies
- Filter bodies by type
- Search bar for body names
- Context menu on right-click
- Detailed orbital information overlay

---

## 14. Key Design Decisions

### Navigation Flow
- Navigation stack is FILO (First-In, Last-Out / stack)
- Once colony ship is launched, player cannot return to Galaxy Map or Journey Log
- Back button pops from navigation stack; does nothing if stack is empty
- ESC key closes modals first, then goes back

### Camera Behavior
- Per-system camera state (each system remembers its own zoom/pan)
- Only persists camera state during active game session
- Zoom always centers on mouse position for intuitive control
- Reset camera returns to default view (origin, 1.0 zoom)

### Procedural Generation
- Deterministic seeding based on system ULID hash
- Seed displayed as 8-character uppercase alphanumeric
- Generation happens on first view command
- Titius-Bode-like orbital spacing with randomization
- Kepler's third law for orbital periods
- 0-2 asteroid belts + always 1 Oort cloud

### Time & Motion
- Orbital positions update each game tick
- Motion freezes when paused
- Speed multiplier affects tick advancement
- Simple linear approximation (can be enhanced later)

### UI/UX
- Single-click selects, double-click opens details
- Click empty space to deselect
- Hover shows tooltip with basic info
- Overview panel toggleable, state persisted
- Time controls integrated into header

---

## 15. Notes

- **Architecture Compliance:** All domain logic is pure and event-sourced
- **No UI State Mutation:** All state changes go through Commands/Events
- **Deterministic Generation:** Same system ID always produces same layout
- **Testability:** GdUnit4 tests cover all domain logic
- **Godot Integration:** Presenter pattern keeps UI thin
- **Scalability:** Can add more orbital mechanics, bodies, features later

---

## 16. Dependencies

**Requires completion of:**
- Session 1.1: Domain types, GameState, StateStore
- Session 1.2: Probe system (provides discovered systems)
- Session 2.1: System selection & details modal (entry point)

**Enables:**
- Session 2.3: Body details modal (double-click target)
- Session 2.4: Body exploration missions
- Future sessions: Colony landing, orbital operations

---

**End of Implementation Plan**
