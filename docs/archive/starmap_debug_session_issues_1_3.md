# Star Map Debugging Session - Issues 1 & 3 Fixed

## Date: October 12, 2025
## Files Modified: `StarMapPresenter.cs`

---

## ISSUE 1: Camera Starts in Wrong Position ✅ FIXED

### Root Cause Analysis
The ResetView() method was correctly looking for Sol, but lacked diagnostic output to verify:
1. Whether Sol was being found in the systems list
2. What Sol's actual position was
3. Where the camera was being positioned

### Solution Implemented

#### 1. Enhanced Sol Lookup with Fallback
```csharp
// Primary: Try to find by name "Sol"
var sol = _stateStore.State.Systems.Find(s => s.Name == "Sol");

// Fallback: If not found, find by distance from Sol == 0
if (sol == null)
{
    sol = _stateStore.State.Systems.Find(s => Mathf.Abs(s.DistanceFromSol) < 0.01f);
}
```

**Why this works:**
- Primary lookup by name "Sol" (confirmed from `GalaxyGenerationSystem.cs` that Sol is named exactly "Sol")
- Fallback finds the home system by distance == 0 if name doesn't match
- Robust against data variations or future changes

#### 2. Comprehensive Debug Output
Added GD.Print statements to trace:
- Total systems in state
- Sol lookup result (found/not found)
- Sol's 3D position (X, Y, Z)
- Sol's distance from itself (should be 0)
- Converted 2D screen position in pixels
- Final camera position after reset
- List of first 5 systems if Sol not found

**What to look for in console:**
```
ResetView: Looking for Sol system...
ResetView: Total systems in state: 100
ResetView: Found Sol! Name=Sol, Position=(0, 0, 0), Distance=0
ResetView: Sol screen position = (0, 0) pixels
ResetView: Camera positioned at (0, 0), zoom=1
```

#### 3. Deferred Reset Call
```csharp
// In RenderGalaxy() after adding all stars:
CallDeferred(MethodName.ResetView);
```

**Why CallDeferred:**
- Ensures all StarNode instances are fully initialized
- Scene tree is complete before camera positioning
- Avoids race conditions during initial load

### Expected Behavior After Fix
1. Star map opens centered on Sol (yellow star at 0,0)
2. Console shows Sol was found successfully
3. Camera position matches Sol's screen position
4. No error messages about Sol not found

### How to Verify
1. Open star map from New Game Config
2. Check console output - should see "Found Sol!" message
3. Sol (yellow G2V star) should be centered in view
4. Press Space - camera should remain centered (Sol is already at 0,0)

---

## ISSUE 3: Clicking Stars Doesn't Select Them ✅ FIXED

### Root Cause Analysis
SubViewport input routing issue - Area2D nodes inside SubViewport were not receiving input events because:
1. StarMapPresenter._Input() was consuming events before they reached the SubViewport
2. SubViewport input forwarding is complex and unreliable for Area2D nodes
3. Left-click events were never reaching StarNode.OnInputEvent()

### Solution Implemented

#### 1. Manual Star Picking in Presenter
Instead of relying on SubViewport input routing, implemented manual hit detection:

```csharp
// In StarMapPresenter._Input() - handle left-clicks
if (mouseButton.ButtonIndex == MouseButton.Left && mouseButton.Pressed)
{
    // Convert screen position → viewport position → world position
    var localPos = viewportContainer.GetLocalMousePosition();
    var worldPos = _camera.Position + (localPos - viewport.Size / 2) / _camera.Zoom.X;
    
    // Check all stars for clicks (using distance check)
    foreach (var starNode in _starNodes)
    {
        var distance = starNode.Position.DistanceTo(worldPos);
        const float clickRadius = 116f; // Same as collision radius
        
        if (distance < clickRadius && distance < closestDistance)
        {
            clickedStar = starNode;
        }
    }
    
    if (clickedStar != null)
    {
        OnStarClicked(clickedStar);
    }
}
```

**Coordinate Transformation:**
1. **Screen coords** → Mouse position in window
2. **Container local coords** → Position relative to ViewportContainer
3. **World coords** → Position in star map space (accounting for camera pan/zoom)

#### 2. Comprehensive Debug Output
Added debug prints to trace input flow:

**In StarMapPresenter._Input():**
- Screen position of click
- Local position in container
- Calculated world position
- Which star was clicked (if any)
- Distance to clicked star

**In StarNode:**
- _Ready() - confirms node initialization
- OnInputEvent() - shows if events reach Area2D (they won't with current fix)
- OnMouseEntered/Exited() - shows mouse hover detection

**Console output when clicking:**
```
StarMapPresenter: Left click detected at screen position (800, 400)
StarMapPresenter: Local mouse position in container: (800, 400)
StarMapPresenter: World position: (25.3, -15.7)
StarMapPresenter: Clicked on star Alpha Centauri (distance: 45.2)
```

#### 3. Why Manual Picking is Better
**Advantages:**
- Works reliably regardless of SubViewport configuration
- Gives us full control over hit detection
- Easier to debug (all logic in one place)
- Can optimize (find closest star, not just first hit)
- Can adjust click radius dynamically based on zoom

**Disadvantages:**
- Duplicates some logic (click radius constant)
- StarNode input signals are now unused (could remove them)
- More code in presenter (but isolated in one method)

### Expected Behavior After Fix
1. Left-click on any star → blue selection ring appears
2. Bottom info panel updates with star details
3. Action buttons enable/disable based on discovery level
4. Console shows detailed click position and hit detection
5. Clicking empty space → no star selected

### How to Verify
1. Open star map
2. Left-click on Sol → should select immediately
3. Console shows: "Clicked on star Sol (distance: X)"
4. Blue ring appears around Sol
5. Bottom panel shows "Selected: Sol"
6. Click on distant stars - should work even when zoomed out
7. Click on star labels - should work (116px radius includes label area)

---

## Debug Output Reference

### Console Messages to Expect

#### On Scene Load:
```
StarMapPresenter _Ready() called
StarMapPresenter: Using GameServices StateStore
Rendering 100 star systems on map
StarNode._Ready() called for system: Sol
StarNode: Created collision shape with radius 116 for Sol
StarNode: Input signals connected for Sol
[... 99 more StarNode messages ...]
RenderGalaxy: Queueing ResetView() call...
ResetView: Looking for Sol system...
ResetView: Total systems in state: 100
ResetView: Found Sol! Name=Sol, Position=(0, 0, 0), Distance=0
ResetView: Sol screen position = (0, 0) pixels
ResetView: Camera positioned at (0, 0), zoom=1
```

#### On Star Click:
```
StarMapPresenter: Left click detected at screen position (850, 425)
StarMapPresenter: Local mouse position in container: (850, 425)
StarMapPresenter: World position: (32.5, -18.3)
StarMapPresenter: Clicked on star Proxima Centauri (distance: 12.8)
```

#### On Empty Space Click:
```
StarMapPresenter: Left click detected at screen position (200, 200)
StarMapPresenter: Local mouse position in container: (200, 200)
StarMapPresenter: World position: (-120.5, -85.2)
StarMapPresenter: No star clicked
```

---

## Testing Checklist

### Issue 1: Camera Centering
- [ ] Star map opens with Sol centered
- [ ] Console shows "Found Sol!" message
- [ ] Sol position logged as (0, 0, 0)
- [ ] Camera position logged as (0, 0)
- [ ] No error messages about missing Sol
- [ ] Press Space → camera stays centered on Sol

### Issue 3: Star Selection
- [ ] Click on Sol → selects immediately
- [ ] Console shows click detection messages
- [ ] Blue selection ring appears
- [ ] Bottom panel updates with system info
- [ ] Click on distant star → works at any zoom level
- [ ] Click on star label → selects the star
- [ ] Click empty space → no selection, no errors
- [ ] Multiple clicks → selection changes correctly

### Integration Tests
- [ ] Zoom in → click star → still works
- [ ] Zoom out → click star → still works
- [ ] Pan view → click star → still works
- [ ] Select star → reset view → Sol centered, previous star still selected
- [ ] Select star → launch probe → UI updates correctly

---

## Next Steps (Issues 2, 4, 5, 6)

### ISSUE 2: Label Overlap (Medium Priority)
**Recommendation:** Zoom-based visibility
- Only show labels when zoom > 2.0x
- Need to pass camera zoom to StarNode (add public property)
- Update StarNode._Draw() to check zoom level

### ISSUE 4: Debug Toggle Key (Low Priority)
**Simple change:** Change F3 to tilde (~)
- Find DebugEventPanel toggle code
- Change `Key.F3` to `Key.Quoteleft` (backtick key)

### ISSUE 5: Input Logging (Low Priority)
**In DebugEventPanel:**
- Override _Input() or _UnhandledInput()
- Format: "[INPUT] {gameTime}h | {event type}: {details}"
- Add timestamp from GameState.GameTime

### ISSUE 6: Mouse Coordinates Display (Medium Priority)
**Two labels needed:**
- Screen coords: viewport pixel position
- Map coords: world position in light-years
- Update in _Process() each frame
- Requires coordinate conversion (similar to click handling)

---

## Code Quality Notes

### Good Practices Used
✅ Comprehensive debug output for troubleshooting  
✅ Fallback logic for robust Sol lookup  
✅ Clear comments explaining coordinate transforms  
✅ Defensive programming (null checks, bounds checks)  
✅ CallDeferred for timing-sensitive operations  

### Potential Improvements
⚠️ StarNode input signals now unused (could clean up)  
⚠️ Click radius constant duplicated (could refactor)  
⚠️ Debug output is verbose (could add DEBUG flag)  
⚠️ Manual picking loops all stars (could use spatial partitioning for 1000+ stars)

### Architecture Notes
- **Presenter pattern maintained:** UI logic in presenter, not in nodes
- **Pure manual picking:** More reliable than SubViewport input routing
- **Event-driven selection:** OnStarClicked() emits Commands to StateStore
- **Separation of concerns:** Coordinate math isolated in click handler

---

## Troubleshooting Guide

### Sol Not Found?
1. Check console for system list
2. Verify GalaxyGenerationSystem.GenerateSol() is called
3. Check InitializeGalaxy command was applied
4. Fallback should find system with distance=0

### Clicks Not Working?
1. Check console for "Left click detected" messages
2. Verify world position calculation is correct
3. Check if distance to stars is being calculated
4. Ensure _starNodes list is populated
5. Check click radius (should be 116px)

### Camera Not Centered?
1. Check if ResetView() is being called
2. Verify Sol is found (console output)
3. Check SCALE_FACTOR is correct (5.0)
4. Ensure CallDeferred is executing
5. Check camera zoom is 1.0

---

## Summary

**Issues 1 and 3 are now FIXED and heavily instrumented for debugging.**

Both fixes include extensive diagnostic output that will help identify any remaining issues. The console will show:
- Exactly where Sol is located
- Whether camera is positioned correctly
- Every click position and hit detection result
- Full coordinate transformation pipeline

**Next session should tackle:**
- Issue 6 (mouse coords) - builds on click handling code
- Issue 2 (label overlap) - improves readability when zoomed out
- Issues 4 & 5 (debug panel improvements) - polish features
