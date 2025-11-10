# Task 2.1: System Selection & Details - Implementation Plan

## Overview

**Goal**: Implement system selection UI that allows players to click discovered star systems to view detailed information, including a modal/panel showing system properties, bodies list, and navigation to detailed system map view.

**Duration**: 60 minutes  
**Architecture Alignment**: Event-sourced, pure functional core, presenter pattern for UI

---

## Current State Analysis

### ✅ What We Have:
1. **Star System Domain**:
   - `StarSystem` record with properties (StarType, Age, Luminosity, etc.)
   - `CelestialBody` records for planets/moons
   - Procedural generation working via probe arrivals
   - Systems stored in `GameState.DiscoveredSystems`

2. **Probe System**:
   - Probes discover systems successfully
   - ProbeArrived events trigger system generation
   - System discovery logged in event stream

3. **UI Infrastructure**:
   - Presenter pattern established in Session 1.2
   - StateStore subscription pattern working
   - Basic UI components (panels, labels, buttons)

### 🔨 What We Need:
1. **System List UI**: Display all discovered systems in scrollable list
2. **System Selection**: Click handler to select a system
3. **System Details Modal**: Show detailed system properties
4. **Bodies List Display**: Show all celestial bodies in the system
5. **Navigation Button**: "View System Map" action (placeholder for 2.2)
6. **Commands/Events**: SystemSelected command and event
7. **State Updates**: Track currently selected system in GameState

---

## Architecture Design

### Selection State Pattern

We'll add selection state to `GameState` to track the currently selected system:

```csharp
public record GameState
{
    // ...existing properties...
    public Ulid? SelectedSystemId { get; init; }
}
```

### Command/Event Flow

```
User clicks system in list
  → UI emits SelectSystemCommand
  → Reducer updates GameState.SelectedSystemId
  → SystemSelected event emitted
  → UI reacts to state change, shows details modal
```

### UI Component Hierarchy

```
Main.tscn
└── SystemsPanel (left sidebar)
    ├── ScrollContainer
    │   └── VBoxContainer (SystemListContainer)
    │       └── [SystemListItem × N]
    └── SystemDetailsModal (popup)
        ├── SystemInfoPanel (star properties)
        ├── BodiesList (scrollable)
        │   └── [BodyListItem × N]
        └── ActionButtons
            └── ViewSystemMapButton
```

---

## Implementation Breakdown

### **Task 2.1.1: Domain Commands & Events (10 min)**

#### 1A: Create SelectSystemCommand

**File**: `src/Game.Core/Commands/SelectSystemCommand.cs`

```csharp
namespace Outpost3.Core.Commands;

/// <summary>
/// Command to select a star system for viewing details.
/// </summary>
public record SelectSystemCommand : ICommand
{
    public Ulid SystemId { get; init; }
    
    public SelectSystemCommand(Ulid systemId)
    {
        SystemId = systemId;
    }
}
```

#### 1B: Create SystemSelected Event

**File**: `src/Game.Core/Events/SystemSelected.cs`

```csharp
namespace Outpost3.Core.Events;

/// <summary>
/// Event fired when a star system is selected by the player.
/// </summary>
public record SystemSelected : IGameEvent
{
    public Ulid SystemId { get; init; }
    
    public SystemSelected(Ulid systemId)
    {
        SystemId = systemId;
    }
}
```

#### 1C: Add Event Type to GameEventJsonConverter

**File**: `src/Game.Persistence/GameEventJsonConverter.cs`

```csharp
// ...existing code...

private static readonly Dictionary<string, Type> EventTypes = new()
{
    // ...existing entries...
    ["SystemSelected"] = typeof(SystemSelected)
};
```

---

### **Task 2.1.2: State Updates & Reducer (10 min)**

#### 2A: Add SelectedSystemId to GameState

**File**: `src/Game.Core/Domain/GameState.cs`

```csharp
public record GameState
{
    // ...existing properties...
    
    /// <summary>
    /// The currently selected star system (for UI display).
    /// Null if no system is selected.
    /// </summary>
    public Ulid? SelectedSystemId { get; init; }
    
    // ...existing methods...
    
    /// <summary>
    /// Creates a new state with the selected system updated.
    /// </summary>
    public GameState WithSelectedSystem(Ulid? systemId)
    {
        return this with { SelectedSystemId = systemId };
    }
}
```

#### 2B: Create SystemSelectionSystem Reducer

**File**: `src/Game.Core/Systems/SystemSelectionSystem.cs`

```csharp
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;

namespace Outpost3.Core.Systems;

/// <summary>
/// Pure reducer for system selection logic.
/// </summary>
public static class SystemSelectionSystem
{
    /// <summary>
    /// Handles system selection commands.
    /// </summary>
    public static (GameState newState, IGameEvent[] events) HandleSelectSystem(
        GameState state,
        SelectSystemCommand command)
    {
        // Validate system exists
        if (!state.DiscoveredSystems.ContainsKey(command.SystemId))
        {
            // Invalid system ID - ignore command
            return (state, Array.Empty<IGameEvent>());
        }
        
        // Update state
        var newState = state.WithSelectedSystem(command.SystemId);
        
        // Emit event
        var evt = new SystemSelected(command.SystemId);
        
        return (newState, new[] { evt });
    }
    
    /// <summary>
    /// Handles deselection (e.g., closing details modal).
    /// </summary>
    public static (GameState newState, IGameEvent[] events) HandleDeselectSystem(GameState state)
    {
        var newState = state.WithSelectedSystem(null);
        return (newState, Array.Empty<IGameEvent>());
    }
}
```

#### 2C: Wire Reducer into StateStore

**File**: `src/Game.App/StateStore.cs`

```csharp
// ...existing code...

public void Dispatch(ICommand command)
{
    // ...existing code...
    
    (GameState newState, IGameEvent[] events) = command switch
    {
        // ...existing cases...
        SelectSystemCommand cmd => SystemSelectionSystem.HandleSelectSystem(_state, cmd),
        // ...rest of cases...
    };
    
    // ...existing code...
}
```

---

### **Task 2.1.3: System List UI Component (15 min)**

#### 3A: Create SystemListItem Scene

**File**: `godot-project/Scenes/UI/SystemListItem.tscn`

```
[gd_scene load_steps=2 format=3 uid="uid://c8h9k2m4p5q6r"]

[ext_resource type="Script" path="res://scripts/UI/SystemListItemComponent.cs" id="1_abc123"]

[node name="SystemListItem" type="PanelContainer"]
custom_minimum_size = Vector2(0, 60)
script = ExtResource("1_abc123")

[node name="MarginContainer" type="MarginContainer" parent="."]
layout_mode = 2
theme_override_constants/margin_left = 8
theme_override_constants/margin_top = 8
theme_override_constants/margin_right = 8
theme_override_constants/margin_bottom = 8

[node name="VBoxContainer" type="VBoxContainer" parent="MarginContainer"]
layout_mode = 2
theme_override_constants/separation = 4

[node name="HeaderHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_constants/separation = 8

[node name="StarColorIndicator" type="ColorRect" parent="MarginContainer/VBoxContainer/HeaderHBox"]
custom_minimum_size = Vector2(16, 16)
layout_mode = 2

[node name="SystemNameLabel" type="Label" parent="MarginContainer/VBoxContainer/HeaderHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 14
text = "System Name"

[node name="DistanceHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_constants/separation = 4

[node name="DistanceLabel" type="Label" parent="MarginContainer/VBoxContainer/DistanceHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 10
text = "Distance:"

[node name="DistanceValue" type="Label" parent="MarginContainer/VBoxContainer/DistanceHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 10
text = "0.0 LY"

[node name="BodiesHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_constants/separation = 4

[node name="BodiesLabel" type="Label" parent="MarginContainer/VBoxContainer/BodiesHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 10
text = "Bodies:"

[node name="BodiesValue" type="Label" parent="MarginContainer/VBoxContainer/BodiesHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 10
text = "0"
```

#### 3B: Implement SystemListItem Component Script

**File**: `godot-project/scripts/UI/SystemListItemComponent.cs`

```csharp
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// UI component for a single star system in the systems list.
/// </summary>
public partial class SystemListItemComponent : PanelContainer
{
    private ColorRect _starColorIndicator;
    private Label _systemNameLabel;
    private Label _distanceValue;
    private Label _bodiesValue;
    
    [Signal]
    public delegate void SystemClickedEventHandler(Ulid systemId);
    
    private Ulid _systemId;
    private bool _isSelected;
    
    public Ulid SystemId => _systemId;
    
    public override void _Ready()
    {
        // Get node references
        _starColorIndicator = GetNode<ColorRect>("MarginContainer/VBoxContainer/HeaderHBox/StarColorIndicator");
        _systemNameLabel = GetNode<Label>("MarginContainer/VBoxContainer/HeaderHBox/SystemNameLabel");
        _distanceValue = GetNode<Label>("MarginContainer/VBoxContainer/DistanceHBox/DistanceValue");
        _bodiesValue = GetNode<Label>("MarginContainer/VBoxContainer/BodiesHBox/BodiesValue");
        
        GuiInput += OnGuiInput;
    }
    
    public void SetSystemData(StarSystem system, double distanceFromEarth)
    {
        _systemId = system.Id;
        
        _systemNameLabel.Text = system.Name;
        _distanceValue.Text = $"{distanceFromEarth:F1} LY";
        _bodiesValue.Text = system.Bodies.Count.ToString();
        
        // Set star color based on type
        _starColorIndicator.Color = GetStarColor(system.StarType);
    }
    
    public void SetSelected(bool selected)
    {
        _isSelected = selected;
        
        // Visual feedback
        var styleBox = new StyleBoxFlat();
        styleBox.BgColor = selected 
            ? new Color(0.3f, 0.5f, 0.7f, 0.4f) 
            : new Color(0.15f, 0.15f, 0.15f, 0.3f);
        styleBox.BorderColor = selected 
            ? new Color(0.5f, 0.7f, 1.0f) 
            : new Color(0.3f, 0.3f, 0.3f);
        styleBox.BorderWidthAll = selected ? 2 : 1;
        AddThemeStyleboxOverride("panel", styleBox);
    }
    
    private void OnGuiInput(InputEvent @event)
    {
        if (@event is InputEventMouseButton mouseEvent 
            && mouseEvent.Pressed 
            && mouseEvent.ButtonIndex == MouseButton.Left)
        {
            EmitSignal(SignalName.SystemClicked, _systemId);
        }
    }
    
    private Color GetStarColor(StarType starType) => starType switch
    {
        StarType.RedDwarf => new Color(1.0f, 0.3f, 0.2f),
        StarType.OrangeDwarf => new Color(1.0f, 0.6f, 0.2f),
        StarType.YellowDwarf => new Color(1.0f, 0.95f, 0.4f),
        StarType.WhiteDwarf => new Color(0.9f, 0.9f, 1.0f),
        StarType.BlueGiant => new Color(0.4f, 0.6f, 1.0f),
        StarType.RedGiant => new Color(1.0f, 0.4f, 0.3f),
        _ => new Color(0.8f, 0.8f, 0.8f)
    };
}
```

---

### **Task 2.1.4: System Details Modal (15 min)**

#### 4A: Create BodyListItem Scene

**File**: `godot-project/Scenes/UI/BodyListItem.tscn`

```
[gd_scene load_steps=2 format=3 uid="uid://d9j0l3n5q6r7s"]

[ext_resource type="Script" path="res://scripts/UI/BodyListItemComponent.cs" id="1_def456"]

[node name="BodyListItem" type="PanelContainer"]
custom_minimum_size = Vector2(0, 70)
script = ExtResource("1_def456")

[node name="MarginContainer" type="MarginContainer" parent="."]
layout_mode = 2
theme_override_constants/margin_left = 6
theme_override_constants/margin_top = 6
theme_override_constants/margin_right = 6
theme_override_constants/margin_bottom = 6

[node name="VBoxContainer" type="VBoxContainer" parent="MarginContainer"]
layout_mode = 2
theme_override_constants/separation = 3

[node name="BodyNameLabel" type="Label" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_font_sizes/font_size = 12
text = "Body Name"

[node name="TypeHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_constants/separation = 4

[node name="TypeLabel" type="Label" parent="MarginContainer/VBoxContainer/TypeHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 9
text = "Type:"

[node name="TypeValue" type="Label" parent="MarginContainer/VBoxContainer/TypeHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 9
text = "RockyPlanet"

[node name="OrbitHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_constants/separation = 4

[node name="OrbitLabel" type="Label" parent="MarginContainer/VBoxContainer/OrbitHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 9
text = "Orbit:"

[node name="OrbitValue" type="Label" parent="MarginContainer/VBoxContainer/OrbitHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 9
text = "0.00 AU"

[node name="MassHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_constants/separation = 4

[node name="MassLabel" type="Label" parent="MarginContainer/VBoxContainer/MassHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 9
text = "Mass:"

[node name="MassValue" type="Label" parent="MarginContainer/VBoxContainer/MassHBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 9
text = "0.00 M⊕"
```

#### 4B: Implement BodyListItem Component Script

**File**: `godot-project/scripts/UI/BodyListItemComponent.cs`

```csharp
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// UI component for a single celestial body in the bodies list.
/// </summary>
public partial class BodyListItemComponent : PanelContainer
{
    private Label _bodyNameLabel;
    private Label _typeValue;
    private Label _orbitValue;
    private Label _massValue;
    
    public override void _Ready()
    {
        _bodyNameLabel = GetNode<Label>("MarginContainer/VBoxContainer/BodyNameLabel");
        _typeValue = GetNode<Label>("MarginContainer/VBoxContainer/TypeHBox/TypeValue");
        _orbitValue = GetNode<Label>("MarginContainer/VBoxContainer/OrbitHBox/OrbitValue");
        _massValue = GetNode<Label>("MarginContainer/VBoxContainer/MassHBox/MassValue");
    }
    
    public void SetBodyData(CelestialBody body)
    {
        _bodyNameLabel.Text = body.Name;
        _typeValue.Text = body.Type.ToString();
        _orbitValue.Text = $"{body.OrbitRadius:F2} AU";
        _massValue.Text = $"{body.Mass:F2} M⊕";
        
        // Visual styling based on body type
        var styleBox = new StyleBoxFlat();
        styleBox.BgColor = GetBodyColor(body.Type);
        styleBox.BorderColor = new Color(0.4f, 0.4f, 0.4f);
        styleBox.BorderWidthAll = 1;
        AddThemeStyleboxOverride("panel", styleBox);
    }
    
    private Color GetBodyColor(BodyType bodyType) => bodyType switch
    {
        BodyType.RockyPlanet => new Color(0.3f, 0.25f, 0.2f, 0.3f),
        BodyType.GasGiant => new Color(0.4f, 0.35f, 0.25f, 0.3f),
        BodyType.IceGiant => new Color(0.2f, 0.3f, 0.4f, 0.3f),
        BodyType.Moon => new Color(0.25f, 0.25f, 0.25f, 0.3f),
        BodyType.DwarfPlanet => new Color(0.3f, 0.3f, 0.3f, 0.3f),
        BodyType.AsteroidBelt => new Color(0.35f, 0.3f, 0.25f, 0.3f),
        _ => new Color(0.2f, 0.2f, 0.2f, 0.3f)
    };
}
```

#### 4C: Create SystemDetailsModal Scene

**File**: `godot-project/Scenes/UI/SystemDetailsModal.tscn`

```
[gd_scene load_steps=3 format=3 uid="uid://e0k1m4n6r7s8t"]

[ext_resource type="Script" path="res://scripts/UI/SystemDetailsModalPresenter.cs" id="1_ghi789"]
[ext_resource type="PackedScene" uid="uid://d9j0l3n5q6r7s" path="res://Scenes/UI/BodyListItem.tscn" id="2_jkl012"]

[node name="SystemDetailsModal" type="PopupPanel"]
size = Vector2i(600, 500)
script = ExtResource("1_ghi789")
body_list_item_scene = ExtResource("2_jkl012")

[node name="MarginContainer" type="MarginContainer" parent="."]
offset_right = 600.0
offset_bottom = 500.0
theme_override_constants/margin_left = 15
theme_override_constants/margin_top = 15
theme_override_constants/margin_right = 15
theme_override_constants/margin_bottom = 15

[node name="VBoxContainer" type="VBoxContainer" parent="MarginContainer"]
layout_mode = 2
theme_override_constants/separation = 10

[node name="HeaderHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2

[node name="TitleLabel" type="Label" parent="MarginContainer/VBoxContainer/HeaderHBox"]
layout_mode = 2
size_flags_horizontal = 3
theme_override_font_sizes/font_size = 20
text = "System Name"

[node name="CloseButton" type="Button" parent="MarginContainer/VBoxContainer/HeaderHBox"]
custom_minimum_size = Vector2(32, 32)
layout_mode = 2
text = "✕"

[node name="HSeparator" type="HSeparator" parent="MarginContainer/VBoxContainer"]
layout_mode = 2

[node name="ContentHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
size_flags_vertical = 3
theme_override_constants/separation = 15

[node name="LeftVBox" type="VBoxContainer" parent="MarginContainer/VBoxContainer/ContentHBox"]
layout_mode = 2
size_flags_horizontal = 3

[node name="PropertiesTitle" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 14
text = "Star Properties"

[node name="HSeparator" type="HSeparator" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox"]
layout_mode = 2

[node name="PropertiesGrid" type="GridContainer" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox"]
layout_mode = 2
theme_override_constants/h_separation = 8
theme_override_constants/v_separation = 8
columns = 2

[node name="TypeLabel" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "Type:"

[node name="TypeValue" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "YellowDwarf"

[node name="LuminosityLabel" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "Luminosity:"

[node name="LuminosityValue" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "1.00 L☉"

[node name="AgeLabel" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "Age:"

[node name="AgeValue" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "4.5 billion years"

[node name="MassLabel" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "Mass:"

[node name="MassValue" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid"]
layout_mode = 2
text = "1.00 M☉"

[node name="RightVBox" type="VBoxContainer" parent="MarginContainer/VBoxContainer/ContentHBox"]
layout_mode = 2
size_flags_horizontal = 3
size_flags_stretch_ratio = 2.0

[node name="BodiesTitle" type="Label" parent="MarginContainer/VBoxContainer/ContentHBox/RightVBox"]
layout_mode = 2
theme_override_font_sizes/font_size = 14
text = "Celestial Bodies"

[node name="HSeparator" type="HSeparator" parent="MarginContainer/VBoxContainer/ContentHBox/RightVBox"]
layout_mode = 2

[node name="ScrollContainer" type="ScrollContainer" parent="MarginContainer/VBoxContainer/ContentHBox/RightVBox"]
custom_minimum_size = Vector2(0, 250)
layout_mode = 2
size_flags_vertical = 3

[node name="BodiesListContainer" type="VBoxContainer" parent="MarginContainer/VBoxContainer/ContentHBox/RightVBox/ScrollContainer"]
layout_mode = 2
size_flags_horizontal = 3
theme_override_constants/separation = 4

[node name="HSeparator2" type="HSeparator" parent="MarginContainer/VBoxContainer"]
layout_mode = 2

[node name="ActionsHBox" type="HBoxContainer" parent="MarginContainer/VBoxContainer"]
layout_mode = 2
alignment = 1
theme_override_constants/separation = 10

[node name="ViewSystemMapButton" type="Button" parent="MarginContainer/VBoxContainer/ActionsHBox"]
layout_mode = 2
disabled = true
text = "View System Map"
```

#### 4D: Implement SystemDetailsModal Presenter Script

**File**: `godot-project/scripts/UI/SystemDetailsModalPresenter.cs`

```csharp
using System.Linq;
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the system details modal popup.
/// </summary>
public partial class SystemDetailsModalPresenter : PopupPanel
{
    [Export] private PackedScene _bodyListItemScene;
    
    private Label _titleLabel;
    private Button _closeButton;
    private Label _starTypeValue;
    private Label _luminosityValue;
    private Label _ageValue;
    private Label _massValue;
    private VBoxContainer _bodiesListContainer;
    private Button _viewSystemMapButton;
    
    private StarSystem _currentSystem;
    
    public override void _Ready()
    {
        // Get node references
        _titleLabel = GetNode<Label>("MarginContainer/VBoxContainer/HeaderHBox/TitleLabel");
        _closeButton = GetNode<Button>("MarginContainer/VBoxContainer/HeaderHBox/CloseButton");
        _starTypeValue = GetNode<Label>("MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid/TypeValue");
        _luminosityValue = GetNode<Label>("MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid/LuminosityValue");
        _ageValue = GetNode<Label>("MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid/AgeValue");
        _massValue = GetNode<Label>("MarginContainer/VBoxContainer/ContentHBox/LeftVBox/PropertiesGrid/MassValue");
        _bodiesListContainer = GetNode<VBoxContainer>("MarginContainer/VBoxContainer/ContentHBox/RightVBox/ScrollContainer/BodiesListContainer");
        _viewSystemMapButton = GetNode<Button>("MarginContainer/VBoxContainer/ActionsHBox/ViewSystemMapButton");
        
        _closeButton.Pressed += Hide;
        _viewSystemMapButton.Pressed += OnViewSystemMapPressed;
        
        // Close on background click
        PopupHide += OnPopupHide;
    }
    
    public void ShowSystem(StarSystem system)
    {
        _currentSystem = system;
        
        // Update title
        _titleLabel.Text = system.Name;
        
        // Update star properties
        _starTypeValue.Text = system.StarType.ToString();
        _luminosityValue.Text = $"{system.Luminosity:F2} L☉";
        _ageValue.Text = $"{system.Age:F1} billion years";
        _massValue.Text = $"{system.Mass:F2} M☉";
        
        // Clear and populate bodies list
        foreach (var child in _bodiesListContainer.GetChildren())
        {
            child.QueueFree();
        }
        
        var sortedBodies = system.Bodies.OrderBy(b => b.OrbitRadius).ToList();
        
        foreach (var body in sortedBodies)
        {
            var bodyItem = _bodyListItemScene.Instantiate<BodyListItemComponent>();
            _bodiesListContainer.AddChild(bodyItem);
            bodyItem.SetBodyData(body);
        }
        
        // Show modal centered
        PopupCentered();
    }
    
    private void OnViewSystemMapPressed()
    {
        // TODO: Navigate to system map scene (Session 2.2)
        GD.Print($"View System Map: {_currentSystem.Name} (not yet implemented)");
    }
    
    private void OnPopupHide()
    {
        // Deselect system when modal closes
        // We could dispatch a DeselectSystemCommand here if needed
    }
}
```

---

### **Task 2.1.5: Systems Panel Integration (10 min)**

#### 5A: Update Main Scene with Systems Panel

**File**: `godot-project/Scenes/Main.tscn`

Add the SystemsPanel to the existing Main scene. The complete updated Main.tscn should include:

```
[gd_scene load_steps=6 format=3 uid="uid://bqnlbwvjxgn46"]

[ext_resource type="Script" path="res://scripts/MainPresenter.cs" id="1_4q0xt"]
[ext_resource type="Script" path="res://scripts/UI/ProbesPanelPresenter.cs" id="3_r6tsk"]
[ext_resource type="PackedScene" uid="uid://bl0sso65w0xjn" path="res://Scenes/UI/ProbeEntry.tscn" id="4_62wrb"]
[ext_resource type="Script" path="res://scripts/UI/SystemsPanelPresenter.cs" id="5_abc123"]
[ext_resource type="PackedScene" uid="uid://c8h9k2m4p5q6r" path="res://Scenes/UI/SystemListItem.tscn" id="6_def456"]

[node name="Main" type="Control"]
layout_mode = 3
anchors_preset = 15
anchor_right = 1.0
anchor_bottom = 1.0
grow_horizontal = 2
grow_vertical = 2
script = ExtResource("1_4q0xt")

[node name="ColorRect" type="ColorRect" parent="."]
layout_mode = 1
anchors_preset = 15
anchor_right = 1.0
anchor_bottom = 1.0
grow_horizontal = 2
grow_vertical = 2
color = Color(0.0823529, 0.0823529, 0.0980392, 1)

[node name="TimeDisplay" type="VBoxContainer" parent="."]
layout_mode = 1
anchors_preset = 5
anchor_left = 0.5
anchor_right = 0.5
offset_left = -100.0
offset_top = 20.0
offset_right = 100.0
offset_bottom = 120.0
grow_horizontal = 2
theme_override_constants/separation = 10

[node name="GameTimeLabel" type="Label" parent="TimeDisplay"]
layout_mode = 2
theme_override_font_sizes/font_size = 24
text = "Game Time: 0.0h"
horizontal_alignment = 1

[node name="RealTimeLabel" type="Label" parent="TimeDisplay"]
layout_mode = 2
theme_override_font_sizes/font_size = 16
text = "Real Time: 0.0s"
horizontal_alignment = 1

[node name="ProbesPanel" type="MarginContainer" parent="."]
layout_mode = 1
anchors_preset = 6
anchor_left = 1.0
anchor_top = 0.5
anchor_right = 1.0
anchor_bottom = 0.5
offset_left = -310.0
offset_top = -200.0
offset_right = -10.0
offset_bottom = 200.0
grow_horizontal = 0
grow_vertical = 2
script = ExtResource("3_r6tsk")
probe_entry_scene = ExtResource("4_62wrb")

[node name="PanelContainer" type="PanelContainer" parent="ProbesPanel"]
custom_minimum_size = Vector2(300, 0)
layout_mode = 2

[node name="MarginContainer" type="MarginContainer" parent="ProbesPanel/PanelContainer"]
layout_mode = 2
theme_override_constants/margin_left = 10
theme_override_constants/margin_top = 10
theme_override_constants/margin_right = 10
theme_override_constants/margin_bottom = 10

[node name="VBoxContainer" type="VBoxContainer" parent="ProbesPanel/PanelContainer/MarginContainer"]
layout_mode = 2
theme_override_constants/separation = 8

[node name="TitleLabel" type="Label" parent="ProbesPanel/PanelContainer/MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_font_sizes/font_size = 14
text = "Active Probes"

[node name="HSeparator" type="HSeparator" parent="ProbesPanel/PanelContainer/MarginContainer/VBoxContainer"]
layout_mode = 2

[node name="ScrollContainer" type="ScrollContainer" parent="ProbesPanel/PanelContainer/MarginContainer/VBoxContainer"]
custom_minimum_size = Vector2(0, 300)
layout_mode = 2
size_flags_vertical = 3

[node name="ProbeListContainer" type="VBoxContainer" parent="ProbesPanel/PanelContainer/MarginContainer/VBoxContainer/ScrollContainer"]
layout_mode = 2
size_flags_horizontal = 3
theme_override_constants/separation = 4

[node name="SystemsPanel" type="MarginContainer" parent="."]
layout_mode = 1
anchors_preset = 4
anchor_top = 0.5
anchor_bottom = 0.5
offset_left = 10.0
offset_top = -200.0
offset_right = 260.0
offset_bottom = 200.0
grow_vertical = 2
script = ExtResource("5_abc123")
system_list_item_scene = ExtResource("6_def456")

[node name="PanelContainer" type="PanelContainer" parent="SystemsPanel"]
custom_minimum_size = Vector2(250, 0)
layout_mode = 2

[node name="MarginContainer" type="MarginContainer" parent="SystemsPanel/PanelContainer"]
layout_mode = 2
theme_override_constants/margin_left = 10
theme_override_constants/margin_top = 10
theme_override_constants/margin_right = 10
theme_override_constants/margin_bottom = 10

[node name="VBoxContainer" type="VBoxContainer" parent="SystemsPanel/PanelContainer/MarginContainer"]
layout_mode = 2
theme_override_constants/separation = 8

[node name="TitleLabel" type="Label" parent="SystemsPanel/PanelContainer/MarginContainer/VBoxContainer"]
layout_mode = 2
theme_override_font_sizes/font_size = 14
text = "Discovered Systems"

[node name="HSeparator" type="HSeparator" parent="SystemsPanel/PanelContainer/MarginContainer/VBoxContainer"]
layout_mode = 2

[node name="ScrollContainer" type="ScrollContainer" parent="SystemsPanel/PanelContainer/MarginContainer/VBoxContainer"]
custom_minimum_size = Vector2(0, 300)
layout_mode = 2
size_flags_vertical = 3

[node name="SystemListContainer" type="VBoxContainer" parent="SystemsPanel/PanelContainer/MarginContainer/VBoxContainer/ScrollContainer"]
layout_mode = 2
size_flags_horizontal = 3
theme_override_constants/separation = 4

[node name="LaunchProbeButton" type="Button" parent="."]
layout_mode = 1
anchors_preset = 7
anchor_left = 0.5
anchor_top = 1.0
anchor_right = 0.5
anchor_bottom = 1.0
offset_left = -75.0
offset_top = -60.0
offset_right = 75.0
offset_bottom = -20.0
grow_horizontal = 2
grow_vertical = 0
text = "Launch Probe"
```

#### 5B: Implement SystemsPanel Presenter Script

**File**: `godot-project/scripts/UI/SystemsPanelPresenter.cs`

```csharp
using System.Linq;
using Godot;
using Outpost3.Core.Commands;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the systems list panel.
/// Subscribes to StateStore and displays discovered systems.
/// </summary>
public partial class SystemsPanelPresenter : MarginContainer
{
    [Export] private PackedScene _systemListItemScene;
    
    private VBoxContainer _systemListContainer;
    private StateStore _stateStore;
    private CommandBus _commandBus;
    private Ulid? _currentSelectedSystemId;
    private SystemDetailsModalPresenter _systemDetailsModal;
    
    public override void _Ready()
    {
        // Get node references
        _systemListContainer = GetNode<VBoxContainer>("PanelContainer/MarginContainer/VBoxContainer/ScrollContainer/SystemListContainer");
        
        // Get services from App
        var app = GetNode<App>("/root/App");
        _stateStore = app.GetStateStore();
        _commandBus = app.GetCommandBus();
        
        // Create and add system details modal to the scene tree
        _systemDetailsModal = GD.Load<PackedScene>("res://Scenes/UI/SystemDetailsModal.tscn").Instantiate<SystemDetailsModalPresenter>();
        GetTree().Root.AddChild(_systemDetailsModal);
        
        _stateStore.StateChanged += OnStateChanged;
        
        RefreshSystemsList();
    }
    
    public override void _ExitTree()
    {
        if (_stateStore != null)
        {
            _stateStore.StateChanged -= OnStateChanged;
        }
        
        if (_systemDetailsModal != null && !_systemDetailsModal.IsQueuedForDeletion())
        {
            _systemDetailsModal.QueueFree();
        }
    }
    
    private void OnStateChanged()
    {
        var state = _stateStore.State;
        
        // Check if selection changed
        if (state.SelectedSystemId != _currentSelectedSystemId)
        {
            _currentSelectedSystemId = state.SelectedSystemId;
            
            // Update visual selection
            UpdateSelectionVisuals();
            
            // Show details modal if system selected
            if (_currentSelectedSystemId.HasValue && 
                state.DiscoveredSystems.TryGetValue(_currentSelectedSystemId.Value, out var system))
            {
                _systemDetailsModal.ShowSystem(system);
            }
        }
        
        // Refresh list if systems added/changed
        RefreshSystemsList();
    }
    
    private void RefreshSystemsList()
    {
        var state = _stateStore.State;
        var existingItems = _systemListContainer.GetChildren()
            .OfType<SystemListItemComponent>()
            .ToList();
        
        // Calculate distances (placeholder - using orbit radius as proxy)
        var systemsWithDistance = state.DiscoveredSystems.Values
            .Select(s => (system: s, distance: s.Position.Length()))
            .OrderBy(x => x.distance)
            .ToList();
        
        // Remove excess items
        while (existingItems.Count > systemsWithDistance.Count)
        {
            var lastItem = existingItems[^1];
            existingItems.RemoveAt(existingItems.Count - 1);
            lastItem.QueueFree();
        }
        
        // Update or create items
        for (int i = 0; i < systemsWithDistance.Count; i++)
        {
            var (system, distance) = systemsWithDistance[i];
            
            SystemListItemComponent item;
            if (i < existingItems.Count)
            {
                item = existingItems[i];
            }
            else
            {
                item = _systemListItemScene.Instantiate<SystemListItemComponent>();
                item.SystemClicked += OnSystemClicked;
                _systemListContainer.AddChild(item);
            }
            
            item.SetSystemData(system, distance);
            item.SetSelected(system.Id == _currentSelectedSystemId);
        }
    }
    
    private void UpdateSelectionVisuals()
    {
        foreach (var item in _systemListContainer.GetChildren().OfType<SystemListItemComponent>())
        {
            item.SetSelected(item.SystemId == _currentSelectedSystemId);
        }
    }
    
    private void OnSystemClicked(Ulid systemId)
    {
        var command = new SelectSystemCommand(systemId);
        _commandBus.Dispatch(command);
    }
}
```

---

## ✅ Acceptance Criteria

- [ ] Discovered systems appear in left sidebar list
- [ ] Clicking a system in the list selects it (visual feedback)
- [ ] System details modal opens when system is clicked
- [ ] Modal shows star properties (type, luminosity, age, mass)
- [ ] Modal shows scrollable list of all celestial bodies
- [ ] Each body shows name, type, orbit, and mass
- [ ] Close button dismisses the modal
- [ ] "View System Map" button exists (disabled/placeholder for 2.2)
- [ ] SelectSystemCommand/SystemSelected event flow works
- [ ] GameState.SelectedSystemId tracks current selection
- [ ] UI updates reactively when selection changes
- [ ] No business logic in UI code (only presenters)

---

## 📦 Implementation Summary

### Files to Create:
1. ✅ `src/Game.Core/Commands/SelectSystemCommand.cs`
2. ✅ `src/Game.Core/Events/SystemSelected.cs`
3. ✅ `src/Game.Core/Systems/SystemSelectionSystem.cs`
4. ✅ `godot-project/Scenes/UI/SystemListItem.tscn`
5. ✅ `godot-project/Scenes/UI/BodyListItem.tscn`
6. ✅ `godot-project/Scenes/UI/SystemDetailsModal.tscn`
7. ✅ `godot-project/scripts/UI/SystemListItemComponent.cs`
8. ✅ `godot-project/scripts/UI/BodyListItemComponent.cs`
9. ✅ `godot-project/scripts/UI/SystemDetailsModalPresenter.cs`
10. ✅ `godot-project/scripts/UI/SystemsPanelPresenter.cs`

### Files to Modify:
1. ✅ `src/Game.Core/Domain/GameState.cs` (add SelectedSystemId property)
2. ✅ `src/Game.App/StateStore.cs` (wire SelectSystemCommand)
3. ✅ `src/Game.Persistence/GameEventJsonConverter.cs` (add SystemSelected)
4. ✅ `godot-project/Scenes/Main.tscn` (add SystemsPanel)

### Estimated Lines of Code:
- Core Domain: ~100 lines
- UI Components: ~400 lines
- Presenters: ~200 lines
- Scene Files: ~300 lines
- **Total**: ~1000 lines

---

## 🎯 Testing Workflow

### Manual Test Sequence:

1. **Test System List Display**
   - Launch game
   - Launch 2-3 probes to different coordinates
   - Wait for probe arrivals
   - Verify systems appear in left sidebar list
   - Verify star color indicators match star types
   - Verify distance and body count displays correctly

2. **Test System Selection**
   - Click on a system in the list
   - Verify visual selection (border/background change)
   - Verify system details modal appears
   - Verify star properties display correctly
   - Verify bodies list shows all bodies

3. **Test Modal Interaction**
   - Scroll through bodies list
   - Click "✕" close button
   - Verify modal closes
   - Click system again
   - Verify modal reopens
   - Click outside modal (background)
   - Verify modal closes

4. **Test State Persistence**
   - Select a system
   - Verify details modal shows
   - Close modal
   - Select different system
   - Verify new system details appear
   - Verify selection tracking updates

---

## 🚀 Implementation Order

**Follow this sequence step-by-step:**

1. **Task 2.1.1**: Domain Commands & Events (10 min)
   - Create SelectSystemCommand
   - Create SystemSelected event
   - Register event in JSON converter

2. **Task 2.1.2**: State Updates & Reducer (10 min)
   - Add SelectedSystemId to GameState
   - Create SystemSelectionSystem
   - Wire into StateStore dispatcher

3. **Task 2.1.3**: System List UI Component (15 min)
   - Create SystemListItem.tscn
   - Implement SystemListItemComponent.cs
   - Test standalone rendering

4. **Task 2.1.4**: System Details Modal (15 min)
   - Create BodyListItem.tscn
   - Implement BodyListItemComponent.cs
   - Create SystemDetailsModal.tscn
   - Implement SystemDetailsModalPresenter.cs

5. **Task 2.1.5**: Systems Panel Integration (10 min)
   - Update Main.tscn with SystemsPanel
   - Implement SystemsPanelPresenter.cs
   - Wire up event handlers

6. **Manual Testing**
   - Test all workflows above
   - Verify no console errors
   - Polish visual feedback

---

**Total Estimated Time**: 60 minutes

This implementation maintains our event-sourced architecture while delivering a rich, interactive UI for system exploration! 🌟
