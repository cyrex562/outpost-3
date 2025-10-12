# Manual Testing Guide: Save/Load System (Session 1.4)

**Purpose**: Validate that the save/load system works correctly in the Godot editor and in builds.

**Date Created**: 2025-10-12  
**Session**: 1.4 - Save/Load System  
**Prerequisites**: All Session 1.4 implementation tasks completed (Tasks 1.4.1 through 1.4.5)

---

## Overview

This document provides step-by-step instructions to manually test the complete save/load functionality, including:
- Quick save/load hotkeys (F5/F9)
- Save file management UI
- Auto-save functionality
- File system persistence
- State restoration accuracy

---

## Pre-Test Setup

### 1. Build and Run

```powershell
# From project root
cd godot-project
dotnet build
```

### 2. Launch Godot Editor

- Open Godot 4.x
- Load project at `godot-project/project.godot`
- Press **F5** to run the project (or click "Run Project")

### 3. Locate Save Files

Save files are stored in the Godot user data directory:

**Windows**:
```
%APPDATA%\Godot\app_userdata\Outpost3\saves\
```

**macOS**:
```
~/Library/Application Support/Godot/app_userdata/Outpost3/saves/
```

**Linux**:
```
~/.local/share/godot/app_userdata/Outpost3/saves/
```

You can also access this in Godot via:
- **Project > Open User Data Folder** (then navigate to `saves/`)

---

## Test Cases

### Test 1: Quick Save (F5)

**Objective**: Verify that pressing F5 creates a quick save file.

#### Steps:
1. Launch the game
2. Start a new colony (if not already started)
3. Note the current game state:
   - Current tick: `______`
   - Colony credits: `______`
   - Population: `______`
   - Morale: `______`
4. Press **F5** (Quick Save)
5. Check console output for confirmation message: `"Quick save created"`

#### Expected Results:
- ✅ Console shows "Quick save created"
- ✅ File exists at `user://saves/quicksave/save.json`
- ✅ No errors or exceptions thrown

#### Verification:
Open `saves/quicksave/save.json` in a text editor and verify:
- `"SaveSlot": "quicksave"`
- `"DisplayName": "Quick Save"`
- `"State"` contains `CurrentTick`, `Colony`, etc.
- `"SaveTime"` is a valid ISO 8601 timestamp

---

### Test 2: Quick Load (F9)

**Objective**: Verify that pressing F9 restores the quick save.

#### Steps:
1. After completing Test 1, advance the game state:
   - Issue several commands (e.g., advance tick multiple times)
   - Note the new current tick: `______`
2. Press **F9** (Quick Load)
3. Check console output for confirmation: `"Quick save loaded"`
4. Verify game state matches the state from step 3 in Test 1

#### Expected Results:
- ✅ Console shows "Quick save loaded"
- ✅ Current tick restored to saved value
- ✅ Colony credits, population, morale match saved values
- ✅ No errors or exceptions thrown

#### Verification:
Compare current state with values noted in Test 1, step 3. They should be **identical**.

---

### Test 3: Save File Management UI

**Objective**: Verify the save/load menu displays saves and allows saving/loading/deleting.

#### Steps:
1. Open the Save/Load Menu (button in main UI)
2. Verify the save list displays at least one save (the quicksave from Test 1)
3. Click **"New Save"** button
4. Enter a custom save name: `"Manual Test Save 1"`
5. Click **"Save"**
6. Verify new save appears in the list
7. Select the new save from the list
8. Note the displayed metadata:
   - Display name: `______`
   - Save time: `______`
   - Game time (ticks): `______`
   - Total events: `______`

#### Expected Results:
- ✅ Save list displays all saves (quicksave + manual save)
- ✅ Each entry shows display name, save time, game time
- ✅ New save dialog accepts input and creates save
- ✅ Metadata is accurate and readable

#### Verification:
Check file system:
- `saves/quicksave/save.json` exists
- `saves/manual_test_save_1/save.json` exists (slot name normalized)

---

### Test 4: Load from Save List

**Objective**: Verify loading a save from the UI restores state.

#### Steps:
1. In the Save/Load Menu, select "Manual Test Save 1"
2. Click **"Load"** button
3. Confirm the load action in the dialog (if prompted)
4. Verify the game returns to the main screen
5. Check that state matches the save:
   - Current tick: `______`
   - Colony stats: `______`

#### Expected Results:
- ✅ Load button triggers state restoration
- ✅ Game state matches the saved state
- ✅ Console shows "Loaded save: manual_test_save_1"
- ✅ No errors during load

---

### Test 5: Delete Save

**Objective**: Verify deleting a save removes it from the file system.

#### Steps:
1. In the Save/Load Menu, select "Manual Test Save 1"
2. Click **"Delete"** button
3. Confirm deletion in the dialog
4. Verify the save is removed from the list
5. Check the file system

#### Expected Results:
- ✅ Save disappears from the UI list
- ✅ Directory `saves/manual_test_save_1/` is deleted
- ✅ Console shows "Deleted save: manual_test_save_1"

#### Verification:
Navigate to `user://saves/` and confirm `manual_test_save_1/` directory no longer exists.

---

### Test 6: Auto-Save

**Objective**: Verify auto-save triggers every 5 minutes.

#### Steps:
1. Start the game
2. Let the game run for at least 5 minutes (can speed up by modifying timer interval for testing)
3. After 5 minutes, check console output

#### Expected Results:
- ✅ Console shows "Auto-save created" after 5 minutes
- ✅ File exists at `saves/autosave/save.json`
- ✅ Auto-save updates every 5 minutes

#### Verification:
- Check `saves/autosave/save.json` modification timestamp
- Load the autosave via the menu and verify it contains recent state

---

### Test 7: Save Slot Naming

**Objective**: Verify save slot names are normalized correctly.

#### Steps:
1. Open Save/Load Menu
2. Create saves with various names:
   - `"Test Save!"`
   - `"Save #123"`
   - `"Colony Alpha-9"`
3. Check the file system for directory names

#### Expected Results:
- ✅ Special characters are replaced with underscores
- ✅ Directories follow pattern: `test_save_`, `save__123`, `colony_alpha_9`
- ✅ All saves load correctly despite name normalization

---

### Test 8: Multiple Save Slots

**Objective**: Verify multiple saves can coexist and be managed independently.

#### Steps:
1. Create 5 different saves with unique names
2. Verify all 5 appear in the save list
3. Load each save sequentially and verify state restoration
4. Delete 2 saves
5. Verify remaining 3 saves still work

#### Expected Results:
- ✅ All saves listed correctly
- ✅ Each save restores correct state
- ✅ Deleting saves doesn't affect others
- ✅ No file corruption or conflicts

---

### Test 9: Error Handling

**Objective**: Verify graceful error handling for edge cases.

#### Steps:
1. **Missing Save**: Try to load a non-existent save (manually delete a save file while game is running, then try to load it)
2. **Corrupted Save**: Manually corrupt `save.json` (add invalid JSON), then try to load
3. **No Saves**: Delete all saves, verify UI shows empty state

#### Expected Results:
- ✅ Missing save shows error message: "Save not found"
- ✅ Corrupted save shows error message: "Failed to load save"
- ✅ Empty save list shows placeholder text: "No saves found"
- ✅ Game doesn't crash on any error condition

---

### Test 10: State Accuracy (Determinism)

**Objective**: Verify loaded state is byte-for-byte identical to saved state.

#### Steps:
1. Start a new colony
2. Issue exactly 10 commands (e.g., advance tick 10 times)
3. Note exact state values:
   - Current tick: `______`
   - Total events: `______`
   - Credits: `______`
4. Save the game as "Determinism Test"
5. Issue 5 more commands
6. Load "Determinism Test"
7. Verify state matches step 3 exactly

#### Expected Results:
- ✅ All state values match exactly
- ✅ Event count is correct
- ✅ No drift or data loss
- ✅ Loading is deterministic (repeated loads give same result)

---

## Performance Tests

### Test 11: Large Save Files

**Objective**: Verify save/load performance with many events.

#### Steps:
1. Run the game and issue 1000+ commands (advance tick 1000 times)
2. Save the game
3. Measure time to save (check console timestamps)
4. Load the game
5. Measure time to load

#### Expected Results:
- ✅ Save completes in < 1 second
- ✅ Load completes in < 2 seconds
- ✅ No memory leaks or performance degradation

---

## Acceptance Criteria Checklist

Use this checklist to validate Session 1.4 is complete:

- [ ] **Quick Save (F5)** creates save file successfully
- [ ] **Quick Load (F9)** restores exact game state
- [ ] **Save/Load Menu** displays all saves with metadata
- [ ] **New Save** button creates named save
- [ ] **Load button** restores selected save
- [ ] **Delete button** removes save and files
- [ ] **Auto-save** triggers every 5 minutes
- [ ] **Save files** stored in correct user directory
- [ ] **State restoration** is 100% accurate (deterministic)
- [ ] **Error handling** prevents crashes on invalid saves
- [ ] **Multiple saves** can coexist without conflicts
- [ ] **Performance** is acceptable (< 2 seconds for load)

---

## Troubleshooting

### Issue: "Quick save created" but no file found

**Solution**: 
- Check Godot console for actual file path (search for "user://")
- Use **Project > Open User Data Folder** to locate saves directory
- Verify `App.cs` initializes `JsonSnapshotStore` with correct path

### Issue: Load fails with "Save not found"

**Solution**:
- Verify save slot name matches directory name (check for normalization)
- Ensure `save.json` file exists in `saves/{slot}/` directory
- Check console for detailed error messages

### Issue: Loaded state doesn't match saved state

**Solution**:
- Verify `StateStore.LoadState()` is called in `SaveLoadService.LoadGame()`
- Check that `StateChanged` signal is emitted after load
- Ensure no commands are issued between load and verification

### Issue: F5/F9 hotkeys don't work

**Solution**:
- Verify `project.godot` contains `quick_save` and `quick_load` input actions
- Check `GlobalInputHandler` is added as child node in `App.cs`
- Ensure input actions are mapped to `Key.F5` and `Key.F9`

### Issue: Auto-save doesn't trigger

**Solution**:
- Verify `Timer` is created and started in `App.cs`
- Check timer interval is set to `300.0` (5 minutes in seconds)
- Verify `OnAutoSave()` is connected to `timer.Timeout` signal

---

## Test Log Template

Use this template to record test results:

```
Test Date: __________
Tester: __________
Build Version: __________

| Test # | Test Name              | Result | Notes                          |
|--------|------------------------|--------|--------------------------------|
| 1      | Quick Save (F5)        | ☐ Pass | ☐ Fail |                                |
| 2      | Quick Load (F9)        | ☐ Pass | ☐ Fail |                                |
| 3      | Save Management UI     | ☐ Pass | ☐ Fail |                                |
| 4      | Load from Save List    | ☐ Pass | ☐ Fail |                                |
| 5      | Delete Save            | ☐ Pass | ☐ Fail |                                |
| 6      | Auto-Save              | ☐ Pass | ☐ Fail |                                |
| 7      | Save Slot Naming       | ☐ Pass | ☐ Fail |                                |
| 8      | Multiple Save Slots    | ☐ Pass | ☐ Fail |                                |
| 9      | Error Handling         | ☐ Pass | ☐ Fail |                                |
| 10     | State Accuracy         | ☐ Pass | ☐ Fail |                                |
| 11     | Large Save Files       | ☐ Pass | ☐ Fail |                                |

Overall Status: ☐ All Pass | ☐ Some Failures

Notes:
_________________________________________________________________
_________________________________________________________________
_________________________________________________________________
```

---

## Next Steps After Testing

Once all tests pass:

1. ✅ Mark Session 1.4 as **COMPLETE** in `docs/07_roadmap.md`
2. Update `docs/06_implementation_plan_1.md` with actual completion date
3. Document any bugs found in a GitHub issue (if using issue tracker)
4. Proceed to **Session 1.5**: Pause/Resume System (see roadmap)

---

## References

- **Implementation Plan**: `docs/09_feature_1_4.md`
- **Unit Tests**: `Tests/SaveLoadTests.cs`
- **Architecture**: `docs/05_architecture.md`
- **Roadmap**: `docs/07_roadmap.md`
