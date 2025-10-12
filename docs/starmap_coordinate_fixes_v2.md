# Star Map Coordinate and Rendering Fixes

**Date:** October 12, 2025  
**Status:** ✅ Complete

## Issues Addressed

### 1. Camera Not Centering on Sol
**Problem:** When the star map loaded, the camera was centering on the bounding box center of all stars, not on Sol (the home system).

**Solution:** Modified `ResetView()` to:
- Always position camera at `Vector2.Zero` (Sol's world position)
- Calculate zoom based on star field bounds to fit all stars
- Use 40% padding to ensure stars aren't at screen edges

### 2. Inaccurate Coordinate Display
**Problem:** Map coordinates were showing incorrect values and not properly reflecting Sol-relative positions.

**Solution:**
- Sol is guaranteed to be at `Vector3.Zero` in `GalaxyGenerationSystem`
- All star positions are stored relative to Sol
- Coordinate display now clearly shows Sol-relative light-year positions
- Improved comments to clarify coordinate system

### 3. Blurry/Pixelated Font Rendering
**Problem:** Star labels were rendering blurry or pixelated at different zoom levels due to text scaling.

**Solution:** Implemented proper screen-space font rendering:
```csharp
// Counter-scale to render text at screen resolution
DrawSetTransform(worldLabelOffset, 0.0f, new Vector2(inverseZoom, inverseZoom));
DrawString(font, Vector2.Zero, labelText, HorizontalAlignment.Left, -1, fontSize, fontColor);
DrawSetTransform(Vector2.Zero, 0.0f, Vector2.One);
```
- Uses `DrawSetTransform()` to counter-scale text rendering
- Fixed font size of 14 pixels in screen space
- Crisp rendering at all zoom levels

### 4. Sol Not Visually Distinctive
**Problem:** Hard to identify Sol among other stars, especially when zoomed out.

**Solution:** Added special rendering for Sol:
- Golden ring (`Color(1.0f, 0.84f, 0.0f)`) around Sol
- Golden text label for Sol
- Sol label always visible regardless of zoom level
- Added `OriginMarker` class to draw crosshairs at world origin

## Code Changes

### Files Modified
1. **`StarMapPresenter.cs`**
   - `ResetView()` - Center on Sol instead of bounding box
   - `UpdateMouseCoordinates()` - Clarified Sol-relative coordinate display
   - `_Ready()` - Improved initialization comments
   - `StarNode._Draw()` - Fixed font rendering with transform counter-scaling
   - `StarNode._Draw()` - Added Sol special rendering
   - Added `OriginMarker` class for visual origin marker

### Key Implementation Details

#### Sol-Centric Coordinate System
```
World Coordinates: Pixels at BASE_PIXELS_PER_LY scale (2.0 px/LY)
- Sol is at (0, 0) in world space
- All other stars positioned relative to Sol
- Camera operates in world space

Light-Year Coordinates: Game units
- Direct conversion: LY = WorldPos / BASE_PIXELS_PER_LY
- All coordinates are Sol-relative
```

#### Zoom Behavior
- Camera always centers on Sol (0, 0) when reset
- Zoom calculated to fit all stars with 40% padding
- Zoom range: 0.1x to 20.0x
- Zoom step: 0.1 for smooth increments

#### Font Rendering
- Fixed 14px screen-space font size
- Transform counter-scaling prevents blur
- Labels show at zoom ≥ 3.0x or when selected
- Sol label always visible

## Testing Recommendations

1. **Initial Load**
   - Camera should be centered on Sol
   - All stars should be visible
   - Sol should have golden ring and label

2. **Zoom In/Out**
   - Text should remain crisp at all zoom levels
   - Labels appear/disappear at appropriate zoom threshold
   - Sol label always visible

3. **Panning**
   - Right-click drag should pan smoothly
   - Coordinate display should update correctly
   - When mouse is at Sol, coordinates should show (0.0 LY, 0.0 LY)

4. **Reset View (Space Key)**
   - Camera returns to Sol center
   - Zoom adjusts to show all stars
   - Consistent behavior

## References

Examined Godot camera panning/zooming examples:
- https://gist.github.com/thygrrr/8288cabeb5cd25031ce6132c4a886311
- https://gist.github.com/Tam/202ae5fd472f55c69ba0ad3da2ff8e77

Key takeaways applied:
- Transform counter-scaling for crisp rendering
- Proper screen-to-world coordinate conversion
- Zoom-to-cursor mechanics

## Architecture Compliance

✅ **Pure UI Presenter Pattern**
- No business logic in presenter
- Only emits commands to StateStore
- Subscribes to state changes
- All rendering logic is pure (deterministic from state)

✅ **Event-Sourced Architecture**
- Selection changes emit `SelectSystemCommand`
- State updates trigger re-render
- No direct state mutation

## Next Steps

Consider future enhancements:
1. Minimap showing full galaxy with viewport indicator
2. Star clustering visualization
3. Navigation history (breadcrumb trail)
4. Bookmark/favorite systems
5. Distance/travel time indicators
