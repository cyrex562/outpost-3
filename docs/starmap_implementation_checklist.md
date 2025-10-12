# Star Map Implementation Checklist

## Coordinate System Fixes - 2025-10-12

### ✅ Implemented

#### 1. Coordinate Transformation Helpers
- [x] Created `ScreenToWorld()` method
- [x] Created `WorldToScreen()` method
- [x] Updated `UpdateMouseCoordinates()` to use helpers
- [x] Changed coordinate precision from F0 to F1 for debugging

#### 2. Click Detection
- [x] Fixed threshold to 10 screen pixels
- [x] Convert to world space: `clickRadius = 10 / zoom`
- [x] Nearest-neighbor search algorithm
- [x] Show map coordinates in debug output (light-years)
- [x] Show selected star's map coordinates

#### 3. Rendering with Zoom
- [x] Calculate `inverseZoom = 1.0 / zoom`
- [x] Scale star size: `size = baseSize * inverseZoom`
- [x] Scale selection ring: `radius = SELECTION_RING_SIZE * inverseZoom`
- [x] Scale label offset: `labelPos = labelOffset * inverseZoom`
- [x] Redraw on zoom change via `UpdateZoomLevel()` → `QueueRedraw()`

#### 4. Debug Output
- [x] Camera initialization logging
- [x] Sol position verification in CreateStarNode()
- [x] ResetView shows Sol at screen center calculation
- [x] Click detection shows world and map coordinates
- [x] Selected star shows map coordinates

### 🧪 Testing Required

#### Coordinate Accuracy
- [ ] Load star map
- [ ] Verify debug log shows "Camera initialized at (0, 0)"
- [ ] Verify debug log shows "Sol at ... -> relative pos (0.0, 0.0) px"
- [ ] Place mouse at screen center
- [ ] Verify coordinate display shows "Map: (0.0 LY, 0.0 LY)" or very close
- [ ] Move mouse and verify coordinates change smoothly

#### Click Detection
- [ ] Zoom to 1.0x
- [ ] Click directly on Sol (should be at center)
- [ ] Verify selection and debug log: "✓ Clicked Sol at map (0.0, 0.0) LY"
- [ ] Click on nearby star
- [ ] Verify debug shows map coordinates in LY
- [ ] Zoom to 4.0x
- [ ] Click on stars - should still work with precise targeting
- [ ] Click between stars - should show "✗ No star within X px"

#### Visual Consistency
- [ ] Start at 1.0x zoom
- [ ] Observe star sizes and label positions
- [ ] Zoom to 2.0x - stars should appear same visual size
- [ ] Zoom to 4.0x - stars should still appear same visual size
- [ ] Zoom to 0.5x - stars should still appear same visual size
- [ ] Labels should not grow/shrink
- [ ] Labels should stay at same distance from stars visually

#### Sol Centering
- [ ] Load map - Sol should be at screen center
- [ ] Pan away from Sol
- [ ] Press Space or Reset button
- [ ] Verify camera returns to Sol at center
- [ ] Verify debug log: "Sol (0,0) should be at screen center: (X, Y)"
- [ ] Verify X ≈ viewport width / 2, Y ≈ viewport height / 2

### 📊 Expected Debug Output Examples

#### On Load:
```
StarMapPresenter: Camera initialized at (0, 0), zoom 1.0
CreateStarNode: Sol found at raw position (7.88, -39.42, 1.23)
CreateStarNode: Sol at world pos (7.88, -39.42) -> relative pos (0.0, 0.0) px
ResetView: Centering camera on Sol at (0,0)
ResetView: Camera now at (0.0, 0.0), zoom 1.00
ResetView: Sol (0,0) should be at screen center: (1280.0, 800.0)
```

#### On Mouse Move:
```
Screen: (1280.0, 800.0)
Map: (0.0 LY, 0.0 LY)
```

#### On Click:
```
StarMapPresenter: Left click detected at screen position (1350.2, 750.8)
StarMapPresenter: Local mouse position in container: (1350.2, 750.8)
StarMapPresenter: World position: (70.2, -50.8) px | Map: (14.0, -10.2) LY
StarMapPresenter: Camera at (0.0, 0.0), zoom 1.00
StarMapPresenter: ✓ Clicked Proxima Centauri at map (1.3, 0.4) LY, distance=8.2 px
```

### 🐛 Known Issues to Watch For

1. **Sol not at (0,0):**
   - Check debug: "Sol at world pos (...) -> relative pos (...)"
   - Should show relative pos (0.0, 0.0) or very close

2. **Screen coordinates wrong:**
   - Check viewport size in debug output
   - Screen center should be (viewportWidth/2, viewportHeight/2)

3. **Stars scaling:**
   - If stars grow with zoom, `inverseZoom` is not being applied
   - Check `_Draw()` method has `size = baseSize * inverseZoom`

4. **Click detection fails:**
   - Check debug output for world position calculation
   - Verify `clickRadiusWorldPixels = 10 / zoom`
   - At 1x zoom: should be 10px, at 2x: should be 5px

### 📝 Files Modified

- `godot-project/scripts/UI/StarMapPresenter.cs`
  - Added ScreenToWorld/WorldToScreen helpers (~10 lines)
  - Updated UpdateMouseCoordinates (~5 lines changed)
  - Updated _Input click detection (~20 lines changed)
  - Updated StarNode._Draw() rendering (~15 lines changed)
  - Added debug output throughout (~15 lines added)

### 🎯 Success Criteria

All of these must be true:
- ✅ Sol appears at screen center on load
- ✅ Mouse at screen center shows map coords ≈ (0.0, 0.0) LY
- ✅ Stars appear same visual size at all zoom levels
- ✅ Labels appear same visual size at all zoom levels
- ✅ Click detection works reliably at all zoom levels
- ✅ Debug output shows accurate coordinates in all spaces

### 🔄 Regression Tests

Make sure these still work:
- [ ] Pan with middle mouse drag
- [ ] Zoom with mouse wheel
- [ ] Reset view with Space key
- [ ] Star selection shows info panel
- [ ] Launch Probe button works
- [ ] Back button returns to New Game Config
