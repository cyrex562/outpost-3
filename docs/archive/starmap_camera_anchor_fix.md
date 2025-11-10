# Star Map Camera Anchor Mode Fix

**Date:** October 12, 2025  
**Status:** ✅ Complete

## Issue

Sol was rendering at the **top-left corner** of the viewport instead of the **center**, even though:
- Sol's world position was correctly set to (0, 0)
- Camera position was set to (0, 0)
- Mouse at viewport center showed correct map coordinates (0, 0)

## Root Cause

The Camera2D was using **ANCHOR_MODE_FIXED_TOP_LEFT** (`anchor_mode = 0`) instead of **ANCHOR_MODE_DRAG_CENTER** (`anchor_mode = 1`).

### Understanding Camera2D Anchor Modes

In Godot's Camera2D:

**ANCHOR_MODE_FIXED_TOP_LEFT (0):**
- `camera.Position` defines the **top-left corner** of the viewport
- Used for platformers, side-scrollers
- World position (0,0) appears at screen position (0,0)

**ANCHOR_MODE_DRAG_CENTER (1):**
- `camera.Position` defines the **center** of the viewport
- Used for top-down games, strategy games, space maps
- World position (0,0) appears at screen center

## Solution

Changed Camera2D to use centered anchor mode:

### Code Changes

1. **StarMapPresenter.cs** - Set anchor mode in `_Ready()`:
```csharp
_camera.AnchorMode = Camera2D.AnchorModeEnum.DragCenter;
_camera.Position = Vector2.Zero; // Now centers view on Sol at world (0,0)
```

2. **StarMapScreen.tscn** - Updated scene default:
```gdscene
[node name="Camera2D" type="Camera2D" parent="ViewportContainer/SubViewport"]
anchor_mode = 1  # Changed from 0 to 1 (DragCenter)
```

3. **Simplified CreateStarNode()** - Removed redundant Sol position lookup:
```csharp
// Direct conversion: 3D position -> 2D world position
var systemPos = new Vector2(system.Position.X, system.Position.Y);
var worldPos = systemPos * BASE_PIXELS_PER_LY;
starNode.Position = worldPos;
```

Since Sol is guaranteed to be at `Vector3.Zero` from `GalaxyGenerationSystem`, this directly places Sol at world (0,0).

## Coordinate System Overview

```
Galaxy Generation (GalaxyGenerationSystem):
  Sol.Position = Vector3.Zero
  Other stars have Vector3 positions relative to Sol in light-years

2D World Space (StarMapPresenter):
  Sol world pos = (0, 0) pixels
  Other stars = system.Position.XY * BASE_PIXELS_PER_LY
  Scale: 2 pixels = 1 light-year at zoom 1.0x

Camera2D (with AnchorMode.DragCenter):
  camera.Position = (0, 0) -> Sol appears at viewport center
  camera.Position = (100, 50) -> World position (100, 50) appears at center
  
Screen Space:
  Viewport center = (viewport.Size / 2)
  screen = center + (world - camera.Position) * zoom
```

## Testing

1. **Load Star Map** - Sol should appear at viewport center with golden ring
2. **Hover Over Sol** - Map coordinates should show (0.0 LY, 0.0 LY)
3. **Reset View (Space)** - Camera returns to centering on Sol
4. **Pan Around** - Can see Sol move as you pan
5. **Zoom In/Out** - Sol stays at center when zooming if not moving mouse

## Impact

- ✅ Sol now renders at viewport center on load
- ✅ Camera position (0,0) correctly centers on world origin
- ✅ Panning and zooming work intuitively around the center
- ✅ Mouse coordinates accurately reflect Sol-relative positions
- ✅ Font rendering remains crisp (previous fix)

## Related Files

- `godot-project/scripts/UI/StarMapPresenter.cs`
- `godot-project/scenes/UI/StarMapScreen.tscn`
- `godot-project/scripts/Core/Systems/GalaxyGenerationSystem.cs`

## Architecture Compliance

✅ **Pure Presenter Pattern** - No business logic, only rendering and command emission  
✅ **Event-Sourced** - State changes through commands  
✅ **Deterministic** - Same seed produces same galaxy positions
