# Star Map Screen Improvements

## Summary
Fixed multiple issues with the StarMapScreen to improve usability and visual clarity.

## Changes Made

### 1. Resolution Update (project.godot)
- **Changed**: Set game window resolution to 2560x1600
- **File**: `godot-project/project.godot`
- **Details**: Added `[display]` section with viewport_width and viewport_height settings
- **Note**: Future work needed to scale UI elements for different resolutions

### 2. Reset View Centers on Sol (StarMapPresenter.cs)
- **Changed**: `ResetView()` method now finds Sol system and centers camera on its actual position
- **Previous**: Always centered on Vector2.Zero regardless of Sol's actual position
- **Current**: Searches for Sol in the Systems list and uses its Position (scaled to pixels)
- **Files**: `godot-project/scripts/UI/StarMapPresenter.cs`

### 3. Auto-Center on Scene Load
- **Changed**: `RenderGalaxy()` now calls `ResetView()` directly instead of using CallDeferred
- **Effect**: Camera immediately centers on Sol when the star map loads
- **Files**: `godot-project/scripts/UI/StarMapPresenter.cs`

### 4. Zoom Centers on Mouse Cursor
- **Changed**: `ZoomCamera()` method now accepts mouse position parameter
- **Implementation**: 
  - Calculates world position under cursor before zoom
  - Applies zoom
  - Adjusts camera position to keep world position under cursor constant
- **Effect**: Zoom feels natural and intuitive, focusing on where the user is looking
- **Files**: `godot-project/scripts/UI/StarMapPresenter.cs`

### 5. Improved Star/Label Click Detection
- **Changed**: Increased collision radius for StarNode Area2D
- **Added**: `LABEL_COLLISION_EXTENSION` constant (100f pixels)
- **Previous**: Collision shape only covered star circle (16f radius)
- **Current**: Collision shape extends to cover label area (116f total radius)
- **Effect**: Clicking on star labels now properly selects the star system
- **Files**: `godot-project/scripts/UI/StarMapPresenter.cs`

### 6. Increased Maximum Zoom Level
- **Changed**: `MAX_ZOOM` constant from 3.0f to 10.0f
- **Reason**: At 3.0x zoom, stars were still too clustered to see individually
- **Effect**: Users can now zoom in much closer to distinguish individual stars
- **Files**: `godot-project/scripts/UI/StarMapPresenter.cs`

## Technical Details

### Zoom Algorithm
The zoom-on-cursor implementation uses this approach:
1. Calculate world position at mouse cursor BEFORE zoom: `worldPosBefore = cameraPos + (mousePos - viewportCenter) / oldZoom`
2. Apply new zoom level to camera
3. Calculate world position at mouse cursor AFTER zoom: `worldPosAfter = cameraPos + (mousePos - viewportCenter) / newZoom`
4. Adjust camera position: `cameraPos += worldPosBefore - worldPosAfter`

This ensures the world coordinates under the mouse cursor remain constant during zoom operations.

### Sol Position Lookup
The reset view now searches for the "Sol" system by name:
```csharp
var sol = _stateStore.State.Systems.Find(s => s.Name == "Sol");
if (sol != null)
{
    solPosition = new Vector2(sol.Position.X, sol.Position.Y) * SCALE_FACTOR;
}
```

According to the test files, Sol should always be at Position (0, 0, 0), but this implementation is robust to any position.

## Testing Recommendations

1. **Resolution**: Launch the game and verify the window is 2560x1600
2. **Reset View**: 
   - Open star map
   - Pan and zoom around
   - Press Space or click Reset
   - Verify Sol is centered in view
3. **Scene Load**: Open star map and verify it immediately shows Sol centered
4. **Zoom on Cursor**:
   - Hover over a distant star
   - Zoom in with mouse wheel
   - Verify zoom centers on the star under cursor
5. **Label Clicks**: 
   - Zoom in to see star labels clearly
   - Click directly on label text
   - Verify the star becomes selected
6. **Max Zoom**: 
   - Zoom in to maximum (10.0x)
   - Verify individual stars are distinguishable

## Future Improvements

1. **Responsive UI**: Scale UI elements (buttons, labels, fonts) based on window resolution
2. **Dynamic Label Collision**: Calculate collision radius based on actual label width instead of fixed constant
3. **Zoom Smoothing**: Add smooth animation for zoom transitions
4. **Label LOD**: Hide labels at low zoom levels to reduce clutter, show at high zoom
5. **Minimap**: Add minimap showing full galaxy with current viewport highlighted
