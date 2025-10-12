# Star Map Debugging Enhancements

## Date: 2025-10-12

## Issues Being Debugged

1. **Coordinate transformation accuracy** - Screen/world/map coordinate conversion
2. **Font redraw timing** - Labels not being redrawn on pan/zoom
3. **Click detection accuracy** - Finding closest star for debugging

## Debug Features Added

### 1. Closest Star Debugging ✅

**Added to click detection:**
- Shows **closest star regardless of click threshold** 
- Displays star's world, map, and screen coordinates
- Shows distance in both world pixels and screen pixels

**Output Example:**
```
StarMapPresenter: Closest star: Alpha Centauri
  World pos: (6.5, 2.3) px
  Map pos: (1.3, 0.5) LY  
  Screen pos: (1350.2, 750.8) px
  Distance from click: 8.2 px (16.4 screen px)
```

### 2. Label Redraw Debugging ✅

**Added to StarNode.UpdateZoomLevel():**
- Logs when zoom level changes trigger redraw
- Shows old zoom → new zoom transition

**Added to StarNode._Draw():**
- Logs when _Draw is called for stars that should show labels
- Shows actual label drawing with position and inverse zoom scale

**Output Example:**
```
StarNode.UpdateZoomLevel: Sol zoom 1.00 -> 2.00 - triggering redraw
StarNode._Draw: Sol redrawing at zoom 2.00 (should show label)
StarNode._Draw: Sol drawing label at (15.0, 0.0) with inverseZoom 0.50
```

### 3. Pan Redraw Triggers ✅

**Problem:** Panning didn't trigger star redraws - only zoom did

**Solution:**
- Added `TriggerStarRedraws()` method
- Pan detection triggers redraw when camera moves > 1 pixel
- Added debug logging for pan events

**Added Methods:**
```csharp
private void TriggerStarRedraws()
{
    GD.Print($"StarMapPresenter: TriggerStarRedraws - forcing redraw of {_starNodes.Count} stars");
    foreach (var starNode in _starNodes)
    {
        starNode.QueueRedraw();
    }
}
```

**Output Example:**
```
StarMapPresenter: Pan - camera moved to (45.2, -23.1)
StarMapPresenter: TriggerStarRedraws - forcing redraw of 100 stars
```

### 4. Coordinate Transform Verification ✅

**Added to ScreenToWorld():**
- Verifies screen→world→screen roundtrip accuracy
- Logs errors if conversion isn't symmetric

**Added to UpdateMouseCoordinates():**
- Detailed debugging when mouse is near screen center
- Shows complete coordinate transformation pipeline

**Output Example:**
```
DEBUG: Mouse near screen center:
  Screen pos: (1280.0, 800.0)
  Viewport size: (2560, 1600)
  Screen center: (1280.0, 800.0)
  Camera pos: (0.0, 0.0)
  Camera zoom: 1.00
  World pos: (0.0, 0.0)
  Map pos: (0.0, 0.0) LY
```

### 5. Enhanced UpdateStarZoomLevels Logging ✅

**Added to UpdateStarZoomLevels():**
- Shows how many stars are being notified of zoom changes
- Helps verify all stars get update messages

**Output Example:**
```
StarMapPresenter: UpdateStarZoomLevels to 2.00 - notifying 100 stars
```

---

## Testing Guide

### 1. Coordinate Accuracy Testing

**Steps:**
1. Load star map
2. Move mouse to screen center
3. Check debug output shows world pos (0.0, 0.0) and map pos (0.0, 0.0) LY
4. Move mouse around screen center - coordinates should change smoothly
5. Look for any "ScreenToWorld verification failed!" errors

**Expected:** 
- Screen center = world (0,0) = map (0,0) LY when camera at origin
- No roundtrip conversion errors

### 2. Label Redraw Testing

**Steps:**
1. Zoom to 2.0x (above label threshold)
2. Watch debug output for "redrawing at zoom" messages
3. Pan around with right-click drag
4. Watch debug output for "TriggerStarRedraws" messages
5. Verify labels appear and reposition correctly

**Expected:**
- Zoom changes show "UpdateZoomLevel" and "_Draw" messages for multiple stars
- Pan events show "TriggerStarRedraws" messages
- Labels should reposition smoothly during pan

### 3. Click Detection Testing

**Steps:**
1. Click various locations on the map
2. Check debug output shows closest star info regardless of whether it was clicked
3. Verify world/map/screen coordinates look reasonable
4. Test at different zoom levels

**Expected:**
- Always see "Closest star: [Name]" with coordinates
- Distance calculations should make sense (closer stars have smaller distances)
- Screen pixel distances should be reasonable (< viewport size)

---

## Known Issues to Watch For

### 1. Coordinate Transform Issues
- **Screen center not mapping to (0,0) world:** Check camera position
- **Roundtrip conversion errors:** Check viewport size calculations
- **Map coordinates wrong scale:** Check SCALE_FACTOR division

### 2. Label Redraw Issues  
- **Labels not redrawing on pan:** Check if TriggerStarRedraws is being called
- **Labels not redrawing on zoom:** Check if UpdateStarZoomLevels is being called
- **Labels wrong size:** Check inverseZoom calculation in _Draw

### 3. Click Detection Issues
- **Closest star coordinates don't match visual:** Check star positioning in CreateStarNode
- **Distance calculations wrong:** Check world coordinate system consistency
- **Screen distances huge:** Check zoom scaling in distance calculations

---

## Debug Output Patterns

### Normal Operation
```
StarMapPresenter: UpdateStarZoomLevels to 1.50 - notifying 100 stars
StarNode.UpdateZoomLevel: Sol zoom 1.00 -> 1.50 - triggering redraw
StarMapPresenter: Pan - camera moved to (12.3, -5.7)
StarMapPresenter: TriggerStarRedraws - forcing redraw of 100 stars
```

### Click Detection
```
StarMapPresenter: Left click detected at screen position (1350.2, 750.8)
StarMapPresenter: Local mouse position in container: (1350.2, 750.8)
StarMapPresenter: World position: (70.2, -50.8) px | Map: (14.0, -10.2) LY
StarMapPresenter: Closest star: Proxima Centauri
  World pos: (6.5, 2.3) px
  Map pos: (1.3, 0.5) LY  
  Screen pos: (1306.5, 804.6) px
  Distance from click: 65.2 px (65.2 screen px)
StarMapPresenter: ✗ No star within 6.7 px (checked 100 stars)
```

### Label Drawing
```
StarNode._Draw: Sol redrawing at zoom 2.00 (should show label)
StarNode._Draw: Sol drawing label at (4.0, 0.0) with inverseZoom 0.50
StarNode._Draw: Alpha Centauri redrawing at zoom 2.00 (should show label)
StarNode._Draw: Alpha Centauri drawing label at (3.0, 3.0) with inverseZoom 0.50
```

---

## Files Modified

**`godot-project/scripts/UI/StarMapPresenter.cs`:**
- Enhanced click detection with closest star debugging (~20 lines)
- Added TriggerStarRedraws() method (~10 lines)
- Enhanced UpdateStarZoomLevels() logging (~3 lines)  
- Enhanced UpdateMouseCoordinates() with center debugging (~15 lines)
- Enhanced ScreenToWorld() with roundtrip verification (~10 lines)
- Enhanced StarNode.UpdateZoomLevel() logging (~3 lines)
- Enhanced StarNode._Draw() logging (~5 lines)

**Total:** ~66 lines of debugging code added

---

## Next Steps

1. **Test the enhanced debugging** - Run the star map and observe debug output
2. **Identify coordinate issues** - Look for patterns in the coordinate debugging
3. **Verify redraw timing** - Confirm labels redraw on both pan and zoom
4. **Fix any identified issues** - Based on what the debugging reveals

The enhanced debugging should make it much easier to identify exactly where the coordinate transformation and redraw issues are occurring.