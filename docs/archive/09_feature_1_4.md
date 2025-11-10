# Task 1.4: Save/Load System - Implementation Plan

## Overview

**Goal**: Implement a complete save/load system that serializes `GameState` to JSON, saves/loads from disk, and provides quick save/load hotkeys with a save file management UI.

**Duration**: 90 minutes  
**Architecture Alignment**: Event-sourced, pure functional core, single-writer pattern

---

## Current State Analysis

### ✅ What We Have:
1. **Event Store Infrastructure**:
   - `IEventStore` interface
   - `FileEventStore` implementation (JSON Lines format)
   - Event persistence working with offsets
   - Events enriched with metadata (GameTime, RealTime, Offset)

2. **State Management**:
   - `StateStore` as single writer
   - Immutable `GameState` record
   - Pure reducers in `TimeSystem`
   - Event-based state transitions

3. **Serialization**:
   - `GameEventJsonConverter` for polymorphic events
   - JSON serialization working for events
   - File I/O patterns established

### 🔨 What We Need:
1. **GameState Serialization**: JSON serialization for the full `GameState` structure
2. **Snapshot Store**: Interface and implementation for state snapshots
3. **Save/Load Service**: Coordinates snapshots + event log tails
4. **Save File Management**: UI for listing, loading, deleting saves
5. **Quick Save/Load**: Hotkey system (F5/F9)
6. **Save Metadata**: Tracking save date, playtime, game progress

---

## Architecture Design

### Snapshot-Based Event Sourcing Pattern

Instead of replaying ALL events from the beginning (which gets slow), we'll use **snapshots + tail events**:

**Single-File Save Format (Recommended)**:
```json
{
  "version": "1.0",
  "saveTime": "2025-10-11T14:30:00Z",
  "saveName": "Autosave - Day 42",
  "snapshotOffset": 150,
  "gameState": { /* full GameState */ },
  "tailEvents": [ /* recent events after snapshot */ ]
}
```

### Key Components

1. **`ISnapshotStore`** - Interface for state persistence
2. **`JsonSnapshotStore`** - JSON-based snapshot implementation  
3. **`SaveLoadService`** - Orchestrates save/load operations
4. **`SaveMetadata`** - Record with save file information
5. **`SaveFileManager`** UI - Scene for managing saves
6. **Input handling** - Global hotkeys for quick save/load

---

## Implementation Breakdown

### **Task 1.4.1: Snapshot Store Infrastructure (20 min)**

#### 1A: Create `ISnapshotStore` Interface

**File**: `godot-project/scripts/Core/Persistence/ISnapshotStore.cs`

```csharp
namespace Outpost3.Core.Persistence;

/// <summary>
/// Interface for persisting and loading game state snapshots.
/// </summary>
public interface ISnapshotStore
{
    /// <summary>
    /// Saves a snapshot of the game state with metadata.
    /// </summary>
    void SaveSnapshot(GameState state, long eventOffset, SaveMetadata metadata);
    
    /// <summary>
    /// Loads the most recent snapshot for a save slot.
    /// Returns null if the save slot doesn't exist.
    /// </summary>
    (GameState state, long eventOffset, SaveMetadata metadata)? LoadSnapshot(string saveSlot);
    
    /// <summary>
    /// Lists all available save slots.
    /// </summary>
    IEnumerable<SaveMetadata> ListSaves();
    
    /// <summary>
    /// Deletes a save slot and all associated files.
    /// </summary>
    void DeleteSave(string saveSlot);
}
```

#### 1B: Create `SaveMetadata` Record

**File**: `godot-project/scripts/Core/Domain/SaveMetadata.cs`

```csharp
using System;

namespace Outpost3.Core.Domain;

/// <summary>
/// Metadata about a saved game.
/// </summary>
public record SaveMetadata
{
    public string SaveSlot { get; init; }      // "autosave", "quicksave", "manual_001"
    public string DisplayName { get; init; }   // "Autosave - Day 42"
    public DateTime SaveTime { get; init; }
    public double GameTime { get; init; }      // In-game time (hours)
    public int TotalEvents { get; init; }      // Event count at save time
    public string GameVersion { get; init; }   // "0.1.4"
    
    /// <summary>
    /// Creates metadata for a new save.
    /// </summary>
    public static SaveMetadata Create(string saveSlot, string displayName, GameState state, long eventOffset)
    {
        return new SaveMetadata
        {
            SaveSlot = saveSlot,
            DisplayName = displayName,
            SaveTime = DateTime.UtcNow,
            GameTime = state.GameTime,
            TotalEvents = (int)eventOffset + 1,
            GameVersion = "0.1.4"
        };
    }
}
```

#### 1C: Create `UlidJsonConverter`

**File**: `godot-project/scripts/Core/Persistence/UlidJsonConverter.cs`

```csharp
using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Outpost3.Core.Persistence;

/// <summary>
/// JSON converter for Ulid type.
/// </summary>
public class UlidJsonConverter : JsonConverter<Ulid>
{
    public override Ulid Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        var str = reader.GetString();
        return string.IsNullOrEmpty(str) ? Ulid.Empty : Ulid.Parse(str);
    }
        
    public override void Write(Utf8JsonWriter writer, Ulid value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(value.ToString());
    }
}
```

#### 1D: Implement `JsonSnapshotStore`

**File**: `godot-project/scripts/Core/Persistence/JsonSnapshotStore.cs`

```csharp
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.Core.Persistence;

/// <summary>
/// JSON-based implementation of snapshot storage.
/// Saves complete game state snapshots to individual files.
/// </summary>
public class JsonSnapshotStore : ISnapshotStore
{
    private readonly string _savesDirectory;
    private readonly JsonSerializerOptions _jsonOptions;
    
    public JsonSnapshotStore(string savesDirectory)
    {
        _savesDirectory = savesDirectory;
        _jsonOptions = new JsonSerializerOptions
        {
            WriteIndented = true,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
            Converters = 
            { 
                new UlidJsonConverter(),
                new JsonStringEnumConverter()
            }
        };
        
        EnsureDirectoryExists(_savesDirectory);
    }
    
    public void SaveSnapshot(GameState state, long eventOffset, SaveMetadata metadata)
    {
        var saveDir = Path.Combine(_savesDirectory, metadata.SaveSlot);
        EnsureDirectoryExists(saveDir);
        
        var saveData = new SaveFile
        {
            Version = metadata.GameVersion,
            Metadata = metadata,
            SnapshotOffset = eventOffset,
            GameState = state
        };
        
        try
        {
            var json = JsonSerializer.Serialize(saveData, _jsonOptions);
            var savePath = Path.Combine(saveDir, "save.json");
            File.WriteAllText(savePath, json);
            
            GD.Print($"✓ Saved game to '{metadata.SaveSlot}' at offset {eventOffset}");
        }
        catch (Exception ex)
        {
            GD.PrintErr($"Failed to save game: {ex.Message}");
            throw new EventStoreException($"Failed to save snapshot: {ex.Message}", ex);
        }
    }
    
    public (GameState state, long eventOffset, SaveMetadata metadata)? LoadSnapshot(string saveSlot)
    {
        var savePath = Path.Combine(_savesDirectory, saveSlot, "save.json");
        
        if (!File.Exists(savePath))
        {
            GD.PrintErr($"Save file not found: {savePath}");
            return null;
        }
        
        try
        {
            var json = File.ReadAllText(savePath);
            var saveData = JsonSerializer.Deserialize<SaveFile>(json, _jsonOptions);
            
            if (saveData == null)
            {
                GD.PrintErr($"Failed to deserialize save file: {savePath}");
                return null;
            }
            
            GD.Print($"✓ Loaded game from '{saveSlot}' (offset: {saveData.SnapshotOffset})");
            return (saveData.GameState, saveData.SnapshotOffset, saveData.Metadata);
        }
        catch (Exception ex)
        {
            GD.PrintErr($"Failed to load save: {ex.Message}");
            return null;
        }
    }
    
    public IEnumerable<SaveMetadata> ListSaves()
    {
        if (!Directory.Exists(_savesDirectory))
            return Enumerable.Empty<SaveMetadata>();
        
        var saves = new List<SaveMetadata>();
        
        foreach (var saveDir in Directory.GetDirectories(_savesDirectory))
        {
            var savePath = Path.Combine(saveDir, "save.json");
            if (!File.Exists(savePath))
                continue;
            
            try
            {
                var json = File.ReadAllText(savePath);
                var saveData = JsonSerializer.Deserialize<SaveFile>(json, _jsonOptions);
                
                if (saveData?.Metadata != null)
                {
                    saves.Add(saveData.Metadata);
                }
            }
            catch (Exception ex)
            {
                GD.PrintErr($"Failed to read save metadata from {saveDir}: {ex.Message}");
            }
        }
        
        return saves.OrderByDescending(s => s.SaveTime);
    }
    
    public void DeleteSave(string saveSlot)
    {
        var saveDir = Path.Combine(_savesDirectory, saveSlot);
        
        if (Directory.Exists(saveDir))
        {
            try
            {
                Directory.Delete(saveDir, recursive: true);
                GD.Print($"✓ Deleted save: {saveSlot}");
            }
            catch (Exception ex)
            {
                GD.PrintErr($"Failed to delete save: {ex.Message}");
                throw new EventStoreException($"Failed to delete save: {ex.Message}", ex);
            }
        }
    }
    
    private void EnsureDirectoryExists(string path)
    {
        if (!Directory.Exists(path))
        {
            Directory.CreateDirectory(path);
        }
    }
}

/// <summary>
/// Container for save file data.
/// </summary>
public record SaveFile
{
    public string Version { get; init; }
    public SaveMetadata Metadata { get; init; }
    public long SnapshotOffset { get; init; }
    public GameState GameState { get; init; }
}
```

---

### **Task 1.4.2: Save/Load Service (25 min)**

#### 2A: Create `SaveLoadService`

**File**: `godot-project/scripts/Core/Services/SaveLoadService.cs`

```csharp
using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using Outpost3.Core.Domain;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;

namespace Outpost3.Core.Services;

/// <summary>
/// Coordinates save and load operations for the game.
/// Manages snapshots, event logs, and state restoration.
/// </summary>
public class SaveLoadService
{
    private readonly StateStore _stateStore;
    private readonly IEventStore _eventStore;
    private readonly ISnapshotStore _snapshotStore;
    
    public SaveLoadService(StateStore stateStore, IEventStore eventStore, ISnapshotStore snapshotStore)
    {
        _stateStore = stateStore ?? throw new ArgumentNullException(nameof(stateStore));
        _eventStore = eventStore ?? throw new ArgumentNullException(nameof(eventStore));
        _snapshotStore = snapshotStore ?? throw new ArgumentNullException(nameof(snapshotStore));
    }
    
    /// <summary>
    /// Saves the current game state to a named slot.
    /// </summary>
    public void SaveGame(string saveSlot, string displayName)
    {
        var currentState = _stateStore.State;
        var currentOffset = _eventStore.CurrentOffset;
        
        var metadata = SaveMetadata.Create(saveSlot, displayName, currentState, currentOffset);
        
        _snapshotStore.SaveSnapshot(currentState, currentOffset, metadata);
        
        GD.Print($"💾 Game saved: {displayName} (offset: {currentOffset}, game time: {currentState.GameTime:F1}h)");
    }
    
    /// <summary>
    /// Loads a saved game, reconstructing state from snapshot.
    /// </summary>
    public bool LoadGame(string saveSlot)
    {
        var snapshot = _snapshotStore.LoadSnapshot(saveSlot);
        if (snapshot == null)
        {
            GD.PrintErr($"Save slot '{saveSlot}' not found");
            return false;
        }
        
        var (savedState, snapshotOffset, metadata) = snapshot.Value;
        
        // For now, we load the exact snapshot state
        // In a full event-sourced system, we would replay tail events here
        // TODO: Implement tail event replay if events exist after snapshot offset
        
        // Replace state in StateStore
        _stateStore.LoadState(savedState);
        
        GD.Print($"📂 Game loaded: {metadata.DisplayName} (game time: {metadata.GameTime:F1}h)");
        return true;
    }
    
    /// <summary>
    /// Lists all available save files.
    /// </summary>
    public IEnumerable<SaveMetadata> ListSaves() => _snapshotStore.ListSaves();
    
    /// <summary>
    /// Deletes a save slot.
    /// </summary>
    public void DeleteSave(string saveSlot)
    {
        _snapshotStore.DeleteSave(saveSlot);
    }
    
    /// <summary>
    /// Quick save to the quicksave slot.
    /// </summary>
    public void QuickSave()
    {
        var displayName = $"Quick Save - {DateTime.Now:yyyy-MM-dd HH:mm}";
        SaveGame("quicksave", displayName);
    }
    
    /// <summary>
    /// Auto save to the autosave slot.
    /// </summary>
    public void AutoSave()
    {
        var day = (int)(_stateStore.State.GameTime / 24.0);
        var displayName = $"Auto Save - Day {day}";
        SaveGame("autosave", displayName);
    }
    
    /// <summary>
    /// Quick load from the quicksave slot.
    /// </summary>
    public bool QuickLoad()
    {
        return LoadGame("quicksave");
    }
}
```

#### 2B: Add `LoadState` Method to StateStore

**File**: `godot-project/scripts/Core/StateStore.cs`

Add this method to the existing `StateStore` class:

```csharp
/// <summary>
/// Loads a saved state, replacing the current state.
/// Used during game load operations.
/// </summary>
public void LoadState(GameState loadedState)
{
    _state = loadedState;
    GD.Print($"State loaded: GameTime = {_state.GameTime:F2}h");
    EmitSignal(SignalName.StateChanged);
}
```

---

### **Task 1.4.3: Save File Management UI (25 min)**

#### 3A: Create Save/Load Menu Scene

**File**: `godot-project/scenes/UI/SaveLoadMenu.tscn`

Create this scene manually in Godot with the following structure:

```
Control (full screen, anchors: fill, name: SaveLoadMenu)
└── PanelContainer
    └── MarginContainer (margin: 20 on all sides)
        └── VBoxContainer (separation: 10)
            ├── Label (text: "Save Game Files", custom font size: 24, align: center)
            ├── HSeparator
            ├── HBoxContainer (Controls)
            │   ├── Button (name: NewSaveButton, text: "New Save")
            │   ├── Button (name: RefreshButton, text: "Refresh")
            │   └── Button (name: BackButton, text: "Back to Menu")
            ├── ScrollContainer (expand vertical, custom minimum size: 0x400)
            │   └── VBoxContainer (name: SaveListContainer)
            ├── HSeparator
            └── HBoxContainer (Selected Actions, separation: 10)
                ├── Button (name: LoadButton, text: "Load Selected", disabled: true)
                ├── Button (name: DeleteButton, text: "Delete Selected", disabled: true)
                └── Label (name: DetailsLabel, expand: true, text: "No saves found")
```

#### 3B: Create Save Entry Component

**File**: `godot-project/scenes/UI/SaveEntry.tscn`

```
PanelContainer (root, name: SaveEntry, custom minimum size: 0x80)
└── MarginContainer (margin: 10 on all sides)
    └── HBoxContainer (separation: 15)
        ├── VBoxContainer (expand, separation: 5)
        │   ├── Label (name: SaveNameLabel, custom font size: 16, bold)
        │   ├── HBoxContainer
        │   │   ├── Label (text: "Game Time:")
        │   │   └── Label (name: GameTimeLabel, text: "Day 0, 0.0h")
        │   ├── HBoxContainer
        │   │   ├── Label (text: "Saved:")
        │   │   └── Label (name: SaveTimeLabel, text: "2025-10-11 14:30")
        │   └── HBoxContainer
        │       ├── Label (text: "Version:")
        │       └── Label (name: VersionLabel, text: "0.1.4")
        └── VBoxContainer (size flags: shrink end, separation: 5)
            └── Label (name: EventCountLabel, text: "0 events")
```

#### 3C: Implement Save Entry Component Script

**File**: `godot-project/scripts/UI/SaveEntryComponent.cs`

```csharp
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

public partial class SaveEntryComponent : PanelContainer
{
    [Export] private Label _saveNameLabel;
    [Export] private Label _gameTimeLabel;
    [Export] private Label _saveTimeLabel;
    [Export] private Label _versionLabel;
    [Export] private Label _eventCountLabel;
    
    [Signal]
    public delegate void SelectedEventHandler();
    
    private SaveMetadata _saveData;
    private bool _isSelected;
    
    public string SaveSlot => _saveData?.SaveSlot;
    
    public override void _Ready()
    {
        GuiInput += OnGuiInput;
    }
    
    public void SetSaveData(SaveMetadata saveData)
    {
        _saveData = saveData;
        
        _saveNameLabel.Text = saveData.DisplayName;
        
        var day = (int)(saveData.GameTime / 24.0);
        var hour = saveData.GameTime % 24.0;
        _gameTimeLabel.Text = $"Day {day}, {hour:F1}h";
        
        _saveTimeLabel.Text = saveData.SaveTime.ToLocalTime().ToString("yyyy-MM-dd HH:mm");
        _versionLabel.Text = saveData.GameVersion;
        _eventCountLabel.Text = $"{saveData.TotalEvents} events";
    }
    
    public void SetSelected(bool selected)
    {
        _isSelected = selected;
        
        // Visual feedback
        var styleBox = new StyleBoxFlat();
        styleBox.BgColor = selected ? new Color(0.3f, 0.5f, 0.7f, 0.3f) : new Color(0.2f, 0.2f, 0.2f, 0.3f);
        styleBox.BorderColor = selected ? new Color(0.5f, 0.7f, 1.0f) : new Color(0.4f, 0.4f, 0.4f);
        styleBox.BorderWidthAll = selected ? 2 : 1;
        AddThemeStyleboxOverride("panel", styleBox);
    }
    
    private void OnGuiInput(InputEvent @event)
    {
        if (@event is InputEventMouseButton mouseEvent && mouseEvent.Pressed && mouseEvent.ButtonIndex == MouseButton.Left)
        {
            EmitSignal(SignalName.Selected);
        }
    }
}
```

#### 3D: Implement Save/Load Menu Presenter

**File**: `godot-project/scripts/UI/SaveLoadMenuPresenter.cs`

```csharp
using System.Linq;
using Godot;
using Outpost3.Core.Services;

namespace Outpost3.UI;

public partial class SaveLoadMenuPresenter : Control
{
    [Export] private VBoxContainer _saveListContainer;
    [Export] private Button _newSaveButton;
    [Export] private Button _refreshButton;
    [Export] private Button _loadButton;
    [Export] private Button _deleteButton;
    [Export] private Button _backButton;
    [Export] private Label _detailsLabel;
    [Export] private PackedScene _saveEntryScene;
    
    private SaveLoadService _saveLoadService;
    private string _selectedSaveSlot;
    
    public override void _Ready()
    {
        // Get service from App singleton
        _saveLoadService = GetNode<App>("/root/App").GetSaveLoadService();
        
        // Connect signals
        _newSaveButton.Pressed += OnNewSavePressed;
        _refreshButton.Pressed += RefreshSaveList;
        _loadButton.Pressed += OnLoadPressed;
        _deleteButton.Pressed += OnDeletePressed;
        _backButton.Pressed += OnBackPressed;
        
        // Load save list
        RefreshSaveList();
    }
    
    private void RefreshSaveList()
    {
        // Clear existing entries
        foreach (var child in _saveListContainer.GetChildren())
        {
            child.QueueFree();
        }
        
        var saves = _saveLoadService.ListSaves().ToList();
        
        if (saves.Count == 0)
        {
            _detailsLabel.Text = "No save files found";
            return;
        }
        
        foreach (var save in saves)
        {
            var entry = _saveEntryScene.Instantiate<SaveEntryComponent>();
            entry.SetSaveData(save);
            entry.Selected += () => OnSaveSelected(save.SaveSlot);
            _saveListContainer.AddChild(entry);
        }
        
        _detailsLabel.Text = $"{saves.Count} save file(s)";
    }
    
    private void OnSaveSelected(string saveSlot)
    {
        _selectedSaveSlot = saveSlot;
        _loadButton.Disabled = false;
        _deleteButton.Disabled = false;
        
        // Update visual selection
        foreach (var child in _saveListContainer.GetChildren().Cast<SaveEntryComponent>())
        {
            child.SetSelected(child.SaveSlot == saveSlot);
        }
        
        _detailsLabel.Text = $"Selected: {saveSlot}";
    }
    
    private void OnLoadPressed()
    {
        if (string.IsNullOrEmpty(_selectedSaveSlot))
            return;
        
        if (_saveLoadService.LoadGame(_selectedSaveSlot))
        {
            // Return to main game scene
            GetTree().ChangeSceneToFile("res://scenes/Main.tscn");
        }
        else
        {
            ShowError("Failed to load save file");
        }
    }
    
    private void OnNewSavePressed()
    {
        ShowSaveNameDialog();
    }
    
    private void OnDeletePressed()
    {
        if (string.IsNullOrEmpty(_selectedSaveSlot))
            return;
        
        ShowConfirmDialog($"Delete save '{_selectedSaveSlot}'?", () =>
        {
            _saveLoadService.DeleteSave(_selectedSaveSlot);
            _selectedSaveSlot = null;
            _loadButton.Disabled = true;
            _deleteButton.Disabled = true;
            RefreshSaveList();
        });
    }
    
    private void OnBackPressed()
    {
        GetTree().ChangeSceneToFile("res://scenes/game_start/StartMenu.tscn");
    }
    
    private void ShowSaveNameDialog()
    {
        var dialog = new AcceptDialog();
        dialog.Title = "New Save";
        dialog.DialogText = "Enter save name:";
        
        var lineEdit = new LineEdit();
        lineEdit.PlaceholderText = "My Save";
        lineEdit.Text = $"Save {System.DateTime.Now:yyyy-MM-dd HH:mm}";
        dialog.AddChild(lineEdit);
        
        dialog.Confirmed += () =>
        {
            var saveName = lineEdit.Text;
            if (!string.IsNullOrWhiteSpace(saveName))
            {
                var saveSlot = $"manual_{System.DateTime.Now:yyyyMMdd_HHmmss}";
                _saveLoadService.SaveGame(saveSlot, saveName);
                RefreshSaveList();
            }
        };
        
        AddChild(dialog);
        dialog.PopupCentered(new Vector2I(400, 150));
    }
    
    private void ShowConfirmDialog(string message, System.Action onConfirm)
    {
        var dialog = new ConfirmationDialog();
        dialog.DialogText = message;
        dialog.Confirmed += () => onConfirm();
        
        AddChild(dialog);
        dialog.PopupCentered();
    }
    
    private void ShowError(string message)
    {
        var dialog = new AcceptDialog();
        dialog.DialogText = message;
        AddChild(dialog);
        dialog.PopupCentered();
    }
}
```

---

### **Task 1.4.4: Quick Save/Load Hotkeys (10 min)**

#### 4A: Add Input Actions to Project Settings

Add these to `godot-project/project.godot` under `[input]` section, or configure in Godot Editor (Project > Project Settings > Input Map):

```
quick_save={
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":0,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":4194332,"key_label":0,"unicode":0,"echo":false,"script":null)
]
}
quick_load={
"deadzone": 0.5,
"events": [Object(InputEventKey,"resource_local_to_scene":false,"resource_name":"","device":0,"window_id":0,"alt_pressed":false,"shift_pressed":false,"ctrl_pressed":false,"meta_pressed":false,"pressed":false,"keycode":0,"physical_keycode":4194336,"key_label":0,"unicode":0,"echo":false,"script":null)
]
}
```

Note: F5 = keycode 4194332, F9 = keycode 4194336

#### 4B: Create Global Input Handler

**File**: `godot-project/scripts/Core/GlobalInputHandler.cs`

```csharp
using Godot;
using Outpost3.Core.Services;

namespace Outpost3.Core;

/// <summary>
/// Handles global input like quick save/load hotkeys.
/// </summary>
public partial class GlobalInputHandler : Node
{
    private SaveLoadService _saveLoadService;
    
    public override void _Ready()
    {
        _saveLoadService = GetNode<App>("/root/App").GetSaveLoadService();
        GD.Print("GlobalInputHandler ready - F5: Quick Save, F9: Quick Load");
    }
    
    public override void _Input(InputEvent @event)
    {
        if (@event.IsActionPressed("quick_save"))
        {
            _saveLoadService.QuickSave();
            ShowNotification("Quick saved!");
            GetViewport().SetInputAsHandled();
        }
        else if (@event.IsActionPressed("quick_load"))
        {
            if (_saveLoadService.QuickLoad())
            {
                ShowNotification("Quick loaded!");
            }
            else
            {
                ShowNotification("No quick save found!", isError: true);
            }
            GetViewport().SetInputAsHandled();
        }
    }
    
    private void ShowNotification(string message, bool isError = false)
    {
        // Simple console notification for now
        // TODO: Implement on-screen toast notification UI
        if (isError)
        {
            GD.PrintErr($"[Notification] {message}");
        }
        else
        {
            GD.Print($"[Notification] {message}");
        }
    }
}
```

---

### **Task 1.4.5: Wire Services into App (10 min)**

#### 5A: Update App.cs with Services

**File**: `godot-project/scripts/App.cs`

Add these fields and initialization to the existing `App` class:

```csharp
private SaveLoadService _saveLoadService;
private ISnapshotStore _snapshotStore;
private Timer _autoSaveTimer;

public override void _Ready()
{
    // ... existing initialization ...
    
    // Create snapshot store
    var savesPath = ProjectSettings.GlobalizePath("user://saves");
    _snapshotStore = new JsonSnapshotStore(savesPath);
    GD.Print($"Snapshot store initialized: {savesPath}");
    
    // Create save/load service
    _saveLoadService = new SaveLoadService(_stateStore, _eventStore, _snapshotStore);
    GD.Print("SaveLoadService initialized");
    
    // Add global input handler for quick save/load
    var inputHandler = new GlobalInputHandler();
    AddChild(inputHandler);
    
    // Setup auto-save timer (every 5 minutes)
    _autoSaveTimer = new Timer();
    _autoSaveTimer.WaitTime = 300.0; // 5 minutes
    _autoSaveTimer.Autostart = true;
    _autoSaveTimer.Timeout += OnAutoSave;
    AddChild(_autoSaveTimer);
    GD.Print("Auto-save timer initialized (5 minute interval)");
}

private void OnAutoSave()
{
    _saveLoadService.AutoSave();
    GD.Print("⏰ Auto-save triggered");
}

/// <summary>
/// Gets the save/load service for dependency injection.
/// </summary>
public SaveLoadService GetSaveLoadService() => _saveLoadService;
```

---

### **Task 1.4.6: Testing (Optional but Recommended)**

#### 6A: Create Unit Tests

**File**: `Tests/SaveLoadTests.cs`

```csharp
using System;
using System.IO;
using Xunit;
using Outpost3.Core.Domain;
using Outpost3.Core.Persistence;

namespace Outpost3.Tests;

public class SaveLoadTests : IDisposable
{
    private readonly string _tempDir;
    
    public SaveLoadTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"outpost3_test_{Guid.NewGuid()}");
        Directory.CreateDirectory(_tempDir);
    }
    
    public void Dispose()
    {
        if (Directory.Exists(_tempDir))
        {
            Directory.Delete(_tempDir, true);
        }
    }
    
    [Fact]
    public void SaveSnapshot_CreatesFile()
    {
        // Arrange
        var snapshotStore = new JsonSnapshotStore(_tempDir);
        var state = GameState.NewGame();
        var metadata = SaveMetadata.Create("test", "Test Save", state, 0);
        
        // Act
        snapshotStore.SaveSnapshot(state, 0, metadata);
        
        // Assert
        var saveFile = Path.Combine(_tempDir, "test", "save.json");
        Assert.True(File.Exists(saveFile));
    }
    
    [Fact]
    public void LoadSnapshot_RestoresState()
    {
        // Arrange
        var snapshotStore = new JsonSnapshotStore(_tempDir);
        var originalState = GameState.NewGame().WithAdvanceTime(100.0);
        var metadata = SaveMetadata.Create("test", "Test Save", originalState, 5);
        
        // Act - Save
        snapshotStore.SaveSnapshot(originalState, 5, metadata);
        
        // Act - Load
        var result = snapshotStore.LoadSnapshot("test");
        
        // Assert
        Assert.NotNull(result);
        var (loadedState, offset, loadedMetadata) = result.Value;
        Assert.Equal(100.0, loadedState.GameTime);
        Assert.Equal(5, offset);
        Assert.Equal("Test Save", loadedMetadata.DisplayName);
    }
    
    [Fact]
    public void ListSaves_ReturnsAllSaves()
    {
        // Arrange
        var snapshotStore = new JsonSnapshotStore(_tempDir);
        var state = GameState.NewGame();
        
        snapshotStore.SaveSnapshot(state, 0, SaveMetadata.Create("save1", "Save 1", state, 0));
        snapshotStore.SaveSnapshot(state, 0, SaveMetadata.Create("save2", "Save 2", state, 0));
        snapshotStore.SaveSnapshot(state, 0, SaveMetadata.Create("save3", "Save 3", state, 0));
        
        // Act
        var saves = snapshotStore.ListSaves();
        
        // Assert
        Assert.Equal(3, saves.Count());
    }
    
    [Fact]
    public void DeleteSave_RemovesFiles()
    {
        // Arrange
        var snapshotStore = new JsonSnapshotStore(_tempDir);
        var state = GameState.NewGame();
        var metadata = SaveMetadata.Create("test", "Test Save", state, 0);
        snapshotStore.SaveSnapshot(state, 0, metadata);
        
        // Act
        snapshotStore.DeleteSave("test");
        
        // Assert
        var saveDir = Path.Combine(_tempDir, "test");
        Assert.False(Directory.Exists(saveDir));
    }
}
```

---

## ✅ Acceptance Criteria

- [ ] Save game to named slot creates JSON file in `user://saves/{slot}/save.json`
- [ ] Load game restores exact `GameState` with correct game time
- [ ] Quick save (F5) works from any screen
- [ ] Quick load (F9) works from any screen
- [ ] Auto-save triggers every 5 minutes
- [ ] Save list UI displays all saves with metadata (name, time, events)
- [ ] Delete save removes files correctly
- [ ] Save includes game time, event count, timestamp, version
- [ ] Unit tests verify save/load roundtrip integrity
- [ ] GameState serialization handles Ulid types correctly
- [ ] StateChanged signal fires after loading a game

---

## 📦 Implementation Summary

### Files to Create:
1. ✅ `scripts/Core/Persistence/ISnapshotStore.cs`
2. ✅ `scripts/Core/Persistence/JsonSnapshotStore.cs`
3. ✅ `scripts/Core/Persistence/UlidJsonConverter.cs`
4. ✅ `scripts/Core/Domain/SaveMetadata.cs`
5. ✅ `scripts/Core/Services/SaveLoadService.cs`
6. ✅ `scripts/Core/GlobalInputHandler.cs`
7. ✅ `scenes/UI/SaveLoadMenu.tscn` (manual in Godot)
8. ✅ `scenes/UI/SaveEntry.tscn` (manual in Godot)
9. ✅ `scripts/UI/SaveEntryComponent.cs`
10. ✅ `scripts/UI/SaveLoadMenuPresenter.cs`
11. ✅ `Tests/SaveLoadTests.cs`

### Files to Modify:
1. ✅ `scripts/Core/StateStore.cs` (add `LoadState` method)
2. ✅ `scripts/App.cs` (register services, add auto-save timer)
3. ✅ `project.godot` (add input actions for F5/F9)

### Estimated Lines of Code:
- Core Persistence: ~400 lines
- Services: ~200 lines
- UI Components: ~300 lines
- Tests: ~200 lines
- **Total**: ~1100 lines

---

## 🎯 Testing Workflow

1. **Manual Test: Save Game**
   - Run game, advance time
   - Press F5 (quick save)
   - Verify `user://saves/quicksave/save.json` exists
   - Verify console shows save confirmation

2. **Manual Test: Load Game**
   - Continue playing, advance time further
   - Press F9 (quick load)
   - Verify game time reverts to saved time
   - Verify UI updates correctly

3. **Manual Test: Save Management UI**
   - Open save/load menu
   - Create new manual save
   - Verify save appears in list
   - Select save and click Load
   - Verify game loads correctly

4. **Unit Tests**
   - Run `dotnet test` in Tests directory
   - All tests should pass

---

## 🚀 Implementation Order

**Follow this sequence step-by-step:**

1. **Task 1.4.1**: Snapshot Store Infrastructure (20 min)
   - Create interfaces and metadata classes
   - Implement JSON snapshot store
   - Create Ulid converter

2. **Task 1.4.2**: Save/Load Service (25 min)
   - Create service layer
   - Add LoadState to StateStore

3. **Task 1.4.3**: UI Components (25 min)
   - Create scenes in Godot
   - Implement presenter scripts

4. **Task 1.4.4**: Hotkeys (10 min)
   - Add input actions
   - Create global input handler

5. **Task 1.4.5**: Wire Services (10 min)
   - Update App.cs
   - Add auto-save timer

6. **Task 1.4.6**: Testing
   - Write unit tests
   - Manual testing

---

**Total Estimated Time**: 90 minutes

This plan maintains alignment with the event-sourced architecture while providing a practical, user-friendly save/load system! 🎮
