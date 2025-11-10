# Main.tscn Cleanup & GameServices Autoload Migration

**Date**: 2025-01-XX  
**Status**: ✅ Complete

## Summary

Migrated the game initialization logic from the old `Main.tscn` demo scene to a proper **autoload singleton pattern** using `GameServices.cs`. The old `Main.tscn` and `App.cs` files have been removed and replaced with a cleaner architecture.

---

## Changes Made

### 1. Created GameServices Autoload Singleton

**File**: `godot-project/Autoload/GameServices.cs`

- Converted `App.cs` into a proper autoload singleton
- Initializes all core services on game startup:
  - `EventStore` (FileEventStore → `user://events.log`)
  - `StateStore` (event-sourced game state)
  - `SnapshotStore` (JsonSnapshotStore → `user://saves`)
  - `SaveLoadService` (save/load game functionality)
  - `GlobalInputHandler` (quick save/load shortcuts)
  - Auto-save timer (5 minute interval)
  - Debug panel (F3 to toggle, debug builds only)
- Persists across all scene changes
- Provides public properties for accessing services:
  - `GameServices.EventStore`
  - `GameServices.StateStore`
  - `GameServices.SaveLoadService`
- **New method**: `InitializeNewGalaxy(seed, starCount)` - called when starting a new game

### 2. Registered GameServices in project.godot

**File**: `godot-project/project.godot`

Added autoload configuration:
```ini
[autoload]

GameServices="*res://Autoload/GameServices.cs"
```

The `*` prefix means the singleton is instantiated immediately on game startup.

### 3. Updated NewGameConfigPresenter

**File**: `godot-project/scripts/UI/NewGameConfigPresenter.cs`

Modified `OnNextPressed()` to initialize the galaxy when starting a new game:
```csharp
private void OnNextPressed()
{
    var gameServices = GetNode<GameServices>("/root/GameServices");
    gameServices.InitializeNewGalaxy(seed: 42, starCount: 100);
    GetTree().ChangeSceneToFile("res://Scenes/UI/StarMapScreen.tscn");
}
```

### 4. Updated StarMapPresenter

**File**: `godot-project/scripts/UI/StarMapPresenter.cs`

Simplified StateStore access using the autoload:
```csharp
// Before (tried to find Main/StateStore)
_stateStore = GetNodeOrNull<StateStore>("/root/Main/StateStore");

// After (uses GameServices autoload)
var gameServices = GetNodeOrNull<GameServices>("/root/GameServices");
if (gameServices != null)
{
    _stateStore = gameServices.StateStore;
}
```

Removed standalone mode (temporary StateStore creation) since GameServices is always available.

### 5. Updated Screen Navigation Paths

**Files**:
- `godot-project/scripts/UI/EventLogScreenPresenter.cs`
- `godot-project/scripts/UI/SaveLoadMenuPresenter.cs`

Changed navigation targets:
- `res://scenes/Main.tscn` → `res://Scenes/MainMenuScreen.tscn`
- Save/load now goes to `res://Scenes/UI/StarMapScreen.tscn` (gameplay screen)

### 6. Deleted Obsolete Files

Removed:
- ❌ `godot-project/Scenes/Main.tscn` (old demo scene)
- ❌ `godot-project/scripts/App.cs` (replaced by GameServices)

---

## Architecture Benefits

### Before (Main.tscn approach)
- ❌ Services tied to a specific scene
- ❌ StateStore lost on scene change
- ❌ Manual initialization required
- ❌ Couldn't access services from other scenes easily
- ❌ Confusion between "main menu" and "main scene"

### After (GameServices autoload)
- ✅ Services available globally via `/root/GameServices`
- ✅ StateStore persists across all scene changes
- ✅ Automatic initialization on game startup
- ✅ Clean separation of concerns
- ✅ Follows Godot best practices
- ✅ Easy dependency injection for presenters

---

## Usage Pattern

All presenters can now access core services like this:

```csharp
public override void _Ready()
{
    var gameServices = GetNode<GameServices>("/root/GameServices");
    
    // Access services
    var stateStore = gameServices.StateStore;
    var eventStore = gameServices.EventStore;
    var saveLoadService = gameServices.SaveLoadService;
    
    // Subscribe to state changes
    stateStore.StateChanged += OnStateChanged;
}
```

---

## Game Flow

1. **Game Launch** → `MainMenuScreen.tscn` loads
   - GameServices autoload initializes in background
   - EventStore loads existing events from `user://events.log`
   - StateStore rebuilds state from events

2. **New Game** → User clicks "New Game" → `NewGameConfigScreen.tscn`
   - User clicks "Next"
   - `NewGameConfigPresenter` calls `gameServices.InitializeNewGalaxy(42, 100)`
   - Navigates to `StarMapScreen.tscn`

3. **Load Game** → User clicks "Load Game" → `LoadGameScreen.tscn`
   - User selects save file
   - `SaveLoadService` loads snapshot + events
   - Navigates to `StarMapScreen.tscn`

4. **Gameplay** → `StarMapScreen.tscn` (or other gameplay screens)
   - All screens access StateStore via GameServices
   - Auto-save triggers every 5 minutes
   - State persists across screen transitions

---

## Testing

To verify the cleanup:

1. Launch the game → should start at `MainMenuScreen`
2. Click "New Game" → should initialize galaxy
3. Check console for `GameServices: Galaxy initialized with 100 stars`
4. Star map should render 100 stars correctly
5. Close star map → should return to New Game Config
6. Navigate to Main Menu → GameServices should still be loaded
7. Quick save (F5) → should work
8. Quick load (F9) → should work

---

## Future Work

- [ ] Move galaxy initialization parameters to NewGameConfigScreen UI (seed input, star count slider)
- [ ] Add loading screen between Main Menu and Star Map
- [ ] Implement proper save slot UI in LoadGameScreen
- [ ] Add Ship Configuration screen after Star System Selection
- [ ] Integrate GameHUD into gameplay screens (time controls, resource display)

---

## References

- **Godot Autoload Pattern**: https://docs.godotengine.org/en/stable/tutorials/scripting/singletons_autoload.html
- **Event-Sourced Architecture**: `docs/05_architecture.md`
- **Screen Flow Spec**: `docs/02_screens.md`
- **Copilot Instructions**: `.github/copilot-instructions.md`
