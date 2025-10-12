# Save/Load UI Scene Creation Guide

## Instructions for Creating Save/Load Scenes in Godot Editor

### Scene 1: SaveEntry.tscn

**Location**: `godot-project/scenes/UI/SaveEntry.tscn`

**Steps**:
1. In Godot, create a new scene with root node type: `PanelContainer`
2. Name the root node: `SaveEntry`
3. Attach script: `res://scripts/UI/SaveEntryComponent.cs`
4. Set custom minimum size: `(0, 80)`

**Node Hierarchy**:
```
SaveEntry (PanelContainer) [Script: SaveEntryComponent.cs]
└── MarginContainer
    └── HBoxContainer (separation: 15)
        ├── VBoxContainer (size_flags_h: EXPAND_FILL, separation: 5)
        │   ├── SaveNameLabel (Label, custom_font_size: 16)
        │   ├── HBoxContainer
        │   │   ├── Label (text: "Game Time:")
        │   │   └── GameTimeLabel (Label, text: "Day 0, 0.0h")
        │   ├── HBoxContainer
        │   │   ├── Label (text: "Saved:")
        │   │   └── SaveTimeLabel (Label, text: "2025-10-11 14:30")
        │   └── HBoxContainer
        │       ├── Label (text: "Version:")
        │       └── VersionLabel (Label, text: "0.1.4")
        └── VBoxContainer (size_flags_h: SHRINK_END, separation: 5)
            └── EventCountLabel (Label, text: "0 events")
```

**Export Connections** (in SaveEntry root node script):
- SaveNameLabel → first Label in left VBoxContainer
- GameTimeLabel → second label in first HBoxContainer
- SaveTimeLabel → second label in second HBoxContainer
- VersionLabel → second label in third HBoxContainer
- EventCountLabel → Label in right VBoxContainer

**Styling**:
- MarginContainer: Set all margins to 10
- SaveNameLabel: Make text bold (theme override)
- All info labels (GameTimeLabel, etc.): Use slightly dimmed color

---

### Scene 2: SaveLoadMenu.tscn

**Location**: `godot-project/scenes/UI/SaveLoadMenu.tscn`

**Steps**:
1. Create new scene with root: `Control`
2. Name root: `SaveLoadMenu`
3. Attach script: `res://scripts/UI/SaveLoadMenuPresenter.cs`
4. Set anchors to fill screen (all anchors: 0, 0, 1, 1)

**Node Hierarchy**:
```
SaveLoadMenu (Control) [Script: SaveLoadMenuPresenter.cs]
└── PanelContainer (anchors: fill, margins: 50 on all sides)
    └── MarginContainer (margin: 20 on all sides)
        └── VBoxContainer (separation: 10)
            ├── Label (text: "Save Game Files", custom_font_size: 24, horizontal_alignment: CENTER)
            ├── HSeparator
            ├── HBoxContainer (Controls)
            │   ├── NewSaveButton (Button, text: "New Save")
            │   ├── RefreshButton (Button, text: "Refresh")
            │   └── BackButton (Button, text: "Back to Menu")
            ├── ScrollContainer (size_flags_v: EXPAND_FILL, custom_minimum_size: (0, 400))
            │   └── SaveListContainer (VBoxContainer)
            ├── HSeparator
            └── HBoxContainer (separation: 10)
                ├── LoadButton (Button, text: "Load Selected", disabled: true)
                ├── DeleteButton (Button, text: "Delete Selected", disabled: true)
                └── DetailsLabel (Label, size_flags_h: EXPAND_FILL, text: "No saves found")
```

**Export Connections** (in SaveLoadMenu root node script):
- SaveListContainer → VBoxContainer inside ScrollContainer
- NewSaveButton → first button in controls HBoxContainer
- RefreshButton → second button in controls HBoxContainer
- BackButton → third button in controls HBoxContainer
- LoadButton → first button in bottom HBoxContainer
- DeleteButton → second button in bottom HBoxContainer
- DetailsLabel → Label in bottom HBoxContainer
- SaveEntryScene → Load the SaveEntry.tscn as PackedScene resource

**Styling**:
- Use consistent theme
- Main title should be prominent
- Buttons should have consistent sizing
- ScrollContainer should have visible scrollbar

---

## Quick Creation Checklist

### SaveEntry.tscn
- [ ] Create PanelContainer root named "SaveEntry"
- [ ] Attach SaveEntryComponent.cs script
- [ ] Build node hierarchy as specified
- [ ] Export all required node references
- [ ] Set custom minimum size (0, 80)
- [ ] Style labels appropriately
- [ ] Save scene to `scenes/UI/SaveEntry.tscn`

### SaveLoadMenu.tscn
- [ ] Create Control root named "SaveLoadMenu"
- [ ] Attach SaveLoadMenuPresenter.cs script
- [ ] Build node hierarchy as specified
- [ ] Export all required node references
- [ ] Load SaveEntry.tscn as PackedScene export
- [ ] Set anchors to fill screen
- [ ] Configure button states (LoadButton and DeleteButton disabled by default)
- [ ] Save scene to `scenes/UI/SaveLoadMenu.tscn`

---

## Testing the Scenes

After creating both scenes:

1. **Test SaveEntry standalone**:
   - Open SaveEntry.tscn
   - Add sample text to labels manually
   - Run scene (F6) to verify layout

2. **Test SaveLoadMenu**:
   - Open SaveLoadMenu.tscn
   - Verify all node references are exported correctly
   - The scene won't fully work until wired into App.cs
   - But you should be able to run it and see the UI

---

## Alternative: Quick Scene Creation

If you prefer, I can provide a minimal .tscn text file you can copy-paste directly into the Godot editor after creating empty scene files.
