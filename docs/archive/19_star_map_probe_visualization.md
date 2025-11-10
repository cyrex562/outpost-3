# Star Map: Probe Visualization & Time Controls

**Date**: 2025-10-12  
**Status**: ✅ Complete

## Overview

Enhanced the Star Map screen with:
1. **Real-time probe visualization** - Shows probes in flight moving from Sol to target systems
2. **Time controls** - Pause, slow down, and speed up game time
3. **System details modal** - View detailed information about selected star systems

## Features Implemented

### 1. Probe Visualization

**Implementation**: `ProbeNode` class in `StarMapPresenter.cs`

- Renders probes as cyan triangular markers pointing towards their target
- Interpolates position based on travel progress (launch time → arrival time)
- Shows progress trail from Sol to current position
- Updates in real-time as game time advances
- Uses inverse zoom scaling to remain visible at all zoom levels

**Visual Design**:
- Triangle shape (8px base size, scales with zoom)
- Cyan color (#33CCFF) with white outline
- Semi-transparent progress trail behind probe
- Points in direction of travel

### 2. Time Controls

**UI Elements**:
- **Pause Button** (⏸): Pause/resume time advancement
- **Slow Down Button** (◀): Decrease time scale by 0.5x (minimum 0.25x)
- **Speed Up Button** (▶): Increase time scale by 2x (maximum 32x)
- **Time Speed Label**: Shows current time scale and status

**Implementation**:
- Auto-advance timer runs in `_Process()` at configurable speed
- Advances time every 1 second of real-time (scaled by time multiplier)
- Time scale: 0.25x, 0.5x, 1x, 2x, 4x, 8x, 16x, 32x
- Pausing stops time advancement but allows camera/UI interaction

**UI Position**: Top-right corner of screen (overlay panel)

### 3. System Details Modal

**Implementation**: Wired up `OnViewSystemPressed()` to show `SystemDetailsModalPresenter`

- Displays star system properties (name, spectral class, luminosity, age, mass)
- Shows list of all celestial bodies in the system
- Modal loads from `res://Scenes/UI/SystemDetailsModal.tscn`
- Automatically centers on screen
- Close button to dismiss

**Integration**:
- Modal instantiated in `_Ready()`
- Connected to "View System Details" button
- Only enabled when a system is selected

## Architecture

### Data Flow

```
User clicks "Launch Probe"
  └─> LaunchProbe command → StateStore
      └─> ProbeLaunched event
          └─> GameState.ProbesInFlight updated
              └─> StateChanged event
                  └─> StarMapPresenter.OnStateChanged()
                      └─> RenderProbes() creates ProbeNode instances
                      
Every frame:
  _Process(delta)
    └─> Auto-advance timer accumulates
        └─> Every 1 second: AdvanceTime command
            └─> TimeAdvanced event
                └─> Probe positions update
                    └─> ProbeNode.UpdatePosition() called
```

### Component Structure

```
StarMapPresenter (Control)
├── ViewportContainer
│   └── SubViewport
│       ├── Camera2D
│       ├── StarsContainer (Node2D)
│       │   ├── OriginMarker
│       │   └── StarNode × N
│       └── ProbesContainer (Node2D)
│           └── ProbeNode × N
├── TimeControls (PanelContainer) [created dynamically]
│   └── Pause/Slow/Fast buttons
└── SystemDetailsModal (added as child)
```

## Technical Details

### Probe Position Calculation

```csharp
// Progress from 0.0 (just launched) to 1.0 (arrived)
var elapsed = currentGameTime - probe.LaunchedAt;
var progress = elapsed / totalTravelTime;

// Interpolate from Sol (0,0) to target system position
Position = Vector2.Zero.Lerp(targetWorldPos, progress);
```

### Time Scale Management

```csharp
// Auto-advance every frame
_autoAdvanceTimer += delta * _timeScale;
if (_autoAdvanceTimer >= 1.0)
{
    var hours = _autoAdvanceTimer;
    _autoAdvanceTimer = 0;
    ApplyCommand(new AdvanceTime(hours));
}
```

### Zoom-Invariant Rendering

All visual elements (probes, trails, UI indicators) use inverse zoom scaling:

```csharp
float inverseZoom = 1.0f / Mathf.Max(camera.Zoom.X, 0.1f);
var visualSize = baseSize * inverseZoom;
```

This ensures probes and stars remain visible and proportional at all zoom levels.

## User Experience

### Launching a Probe

1. Select a star system by clicking on it
2. Click "Launch Probe" button
3. Cyan triangle appears at Sol, pointing towards target
4. Probe automatically moves towards target as time advances
5. Progress trail shows path traveled
6. On arrival: probe disappears, system gets scanned

### Viewing System Details

1. Select a star system
2. Click "View System Details" button
3. Modal appears showing:
   - Star name and spectral class
   - Physical properties (luminosity, age, mass)
   - List of celestial bodies with properties
4. Click close button or outside modal to dismiss

### Time Control

1. **Default**: Time runs at 1x speed
2. **Pause**: Click ⏸ to freeze time (camera still works)
3. **Resume**: Click ⏸ again to resume
4. **Speed Up**: Click ▶ repeatedly to increase speed (up to 32x)
5. **Slow Down**: Click ◀ to decrease speed (down to 0.25x)
6. Label shows current speed: "Time: 2.0x (Running)" or "Time: PAUSED"

## Testing Checklist

- [ ] Launch probe to nearby system - verify triangle appears
- [ ] Pause time - verify probe stops moving
- [ ] Resume time - verify probe continues
- [ ] Speed up time - verify probe moves faster
- [ ] Zoom in/out - verify probe remains visible and proportional
- [ ] Click "View System Details" - verify modal appears
- [ ] Modal shows correct system information
- [ ] Modal close button works
- [ ] Multiple probes in flight render correctly
- [ ] Probe disappears on arrival at target system

## Known Limitations

1. **No ETA display**: Probe doesn't show time remaining (could add label)
2. **No probe selection**: Can't click on probes for details
3. **No cancellation**: Can't recall a probe once launched
4. **No probe list panel**: No UI listing all probes in flight
5. **Time controls not persisted**: Time scale resets to 1x on scene reload

## Future Enhancements

### Short-term
- Add ETA label above probes (shows "ETA: 42h")
- Show probe count in UI: "Probes: 3 in flight"
- Persist time scale preference in game state

### Long-term
- Probe management panel (list all probes, show details, ability to cancel)
- Multiple probe types (fast scouts, detailed survey probes)
- Probe failure chance based on distance
- Probe waypoints (route through multiple systems)
- Probe communications delay visualization

## Files Modified

- `godot-project/scripts/UI/StarMapPresenter.cs`
  - Added `ProbeNode` class for visual representation
  - Added time control UI creation
  - Added `RenderProbes()` and `UpdateProbePositions()` methods
  - Wired up system details modal
  - Added auto-advance timer in `_Process()`

## Code Statistics

- **Lines added**: ~250
- **New classes**: 1 (`ProbeNode`)
- **New methods**: 7 (time controls, probe rendering)
- **UI elements added**: 4 (3 buttons + 1 label)

---

## Summary

The star map now provides real-time visual feedback for probe missions and gives players control over game time flow. Players can:

✅ **See probes moving** towards target systems  
✅ **Control time flow** with pause and speed controls  
✅ **View system details** in a dedicated modal  

This creates a much more engaging exploration experience and sets the foundation for future features like probe management and mission planning.
