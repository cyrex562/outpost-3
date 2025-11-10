# Task 1.3: Event Log System - AI Agent Prompts

## Session 1.3.1: Implement FileEventStore

### Prompt 1A: Create Event Base Types

Create base event types for the event system.

**File:** `godot-project/scripts/Core/Events/GameEvent.cs`

**Requirements:**
- Create an abstract record `GameEvent` with properties:
  - `Offset` (long) - Sequential ID from EventStore
  - `GameTime` (float) - In-game time when event occurred
  - `RealTime` (DateTime) - Real-world timestamp
  - `EventType` (string) - Discriminator for serialization

**Concrete Event Types for Journey Phase:**

Create these records that inherit from `GameEvent`:

1. `ShipDepartedEvent` with properties:
   - DestinationSystemId (Ulid)
   - ShipName (string)
   - ColonistCount (int)

2. `MechanicalFailureEvent` with properties:
   - SystemAffected (string)
   - Severity (string)
   - Description (string)

3. `SocialConflictEvent` with properties:
   - ConflictType (string)
   - Description (string)
   - MoraleImpact (float)

4. `AnomalyDetectedEvent` with properties:
   - AnomalyType (string)
   - Description (string)

5. `ShipArrivedEvent` with properties:
   - SystemId (Ulid)
   - SystemName (string)
   - TravelDuration (float)

**Constraints:**
- All events must be immutable C# records
- EventType should auto-populate from the type name
- Include XML doc comments for each event type

---

### Prompt 1B: Create EventStore Interface

Define the IEventStore interface for the event persistence system.

**File:** `godot-project/scripts/Core/Events/IEventStore.cs`

**Requirements:**

Create an interface `IEventStore` with these members:

1. Method `Append(params GameEvent[] events)` that returns `long`
   - Appends one or more events to the store
   - Returns the starting offset of the first appended event

2. Method `ReadFrom(long offset = 0)` that returns `IEnumerable<GameEvent>`
   - Reads all events starting from the given offset
   - Returns empty enumerable if offset > CurrentOffset

3. Property `CurrentOffset` (long, read-only)
   - Gets the current highest offset in the store

4. Property `Count` (long, read-only)
   - Gets total count of events in the store

**Design Notes:**
- Must be thread-safe for single writer, multiple readers
- Offsets are 0-indexed and contiguous
- Add XML doc comments explaining thread-safety guarantees

---

### Prompt 1C: Implement JSON-Based FileEventStore

Implement a JSON Lines based file event store.

**File:** `godot-project/scripts/core/persistence/FileEventStore.cs`

**Requirements:**

Create a class `FileEventStore` that implements `IEventStore` with these specifications:

**File Format:**
- Each event is one line of text (JSON Lines format)
- Line format: `{offset}|{gameTime}|{eventType}|{jsonPayload}`
- Example: `0|12.5|ShipDepartedEvent|{"destinationSystemId":"alpha-centauri","shipName":"Hope","colonistCount":100}`

**Implementation Details:**

1. Constructor:
   - Accept `string filePath` parameter
   - Initialize `JsonSerializerOptions` with:
     - WriteIndented = false
     - PropertyNamingPolicy = JsonNamingPolicy.CamelCase
     - Add JsonStringEnumConverter
   - If file exists, count lines to set initial `_currentOffset`

2. `Append` method:
   - Use lock for thread safety (`_writeLock`)
   - Assign sequential offsets to each event
   - Serialize each event as: `offset|gameTime|eventType|json`
   - Append to file with immediate flush
   - Update `_currentOffset`
   - Return starting offset

3. `ReadFrom` method:
   - Implement lazy reading using `IEnumerable<GameEvent>`
   - Open file for reading
   - Skip to the requested offset line
   - Parse each line: split by '|', deserialize JSON
   - Yield return events one at a time
   - Handle polymorphic deserialization using eventType field

4. Properties:
   - `CurrentOffset` returns current `_currentOffset` value
   - `Count` counts lines in file (or 0 if file doesn't exist)

**Error Handling:**
- Throw `EventStoreException` if file is corrupted (include line number)
- Throw clear exception if disk is full
- Handle missing file gracefully (treat as empty store)

**Dependencies:**
- Use `System.Text.Json` for serialization
- Use `System.IO.File` for file operations

---

### Prompt 1D: Create Polymorphic JSON Converter

Create a custom JsonConverter for polymorphic GameEvent deserialization.

**File:** `godot-project/scripts/Core/Persistence/GameEventJsonConverter.cs`

**Requirements:**

Create a class `GameEventJsonConverter` that inherits from `JsonConverter<GameEvent>`:

**Read Method:**
1. Parse the JSON into a `JsonDocument`
2. Extract the "eventType" property value
3. Use a helper method `GetEventType(string eventTypeName)` to map type name to `Type`
4. Deserialize to the concrete type
5. Return the event

**Write Method:**
- Serialize using the runtime type of the value
- Use `JsonSerializer.Serialize(writer, value, value.GetType(), options)`

**Type Mapping:**

Create a helper method `GetEventType(string eventTypeName)` that returns `Type`:
- Use a switch expression or dictionary to map event type names to concrete types
- Include mappings for:
  - ShipDepartedEvent
  - MechanicalFailureEvent
  - SocialConflictEvent
  - AnomalyDetectedEvent
  - ShipArrivedEvent
- Throw `NotSupportedException` for unknown event types

**Integration:**
- This converter will be added to `JsonSerializerOptions.Converters` in FileEventStore

---

## Session 1.3.2: Wire EventStore to StateStore

### Prompt 2A: Integrate FileEventStore into StateStore

Update the StateStore class to use IEventStore for event persistence.

**File:** `StateStore.cs`

**Changes Required:**

1. **Constructor Dependency:**
   - Add `IEventStore eventStore` parameter to constructor
   - Store as private readonly field `_eventStore`

2. **State Enhancement:**
   - Ensure `GameState` has a `CurrentGameTime` property (float)

3. **Apply Method Changes:**
   - After running reducer to get new state and events
   - Enrich each event with metadata:
     - Set `GameTime` from `newState.CurrentGameTime`
     - Set `RealTime` to `DateTime.UtcNow`
     - Set `Offset` to `_eventStore.CurrentOffset + 1` (or let EventStore assign)
   - Call `_eventStore.Append(enrichedEvents)`
   - Update `_currentState` to new state
   - Emit StateChanged signal/event for UI

4. **Example Pattern:**
```
public void Apply(object command)
{
    // Run reducer
    var (newState, events) = ApplyReducer(_currentState, command);
    
    // Enrich events with metadata
    var enrichedEvents = events.Select(e => e with 
    { 
        GameTime = newState.CurrentGameTime,
        RealTime = DateTime.UtcNow
    }).ToArray();
    
    // Persist to event store
    _eventStore.Append(enrichedEvents);
    
    // Update current state
    _currentState = newState;
    
    // Notify observers
    EmitSignal(nameof(StateChanged));
}
```

**Testing:**
- After applying a command, verify the event appears in the EventStore file
- Restart the application and verify events persist
- Verify `CurrentOffset` increments correctly

---

## Session 1.3.3: Ship Journey Log UI

### Prompt 3A: Create Ship Journey Log Scene

Create the main scene for the Ship Journey Log screen.

**File:** `Scenes/ShipJourneyLog.tscn`

**Scene Structure:**

Create a scene with this hierarchy:

```
MarginContainer (root)
└── VBoxContainer
    ├── HBoxContainer (Header)
    │   ├── Label (text: "Ship Journey - [Ship Name]")
    │   └── Button (text: "Back to System Map")
    │
    ├── PanelContainer (Journey Progress Section)
    │   └── VBoxContainer
    │       ├── Label (name: ProgressLabel, text: "Day [X] of [Total] - [Percent]% Complete")
    │       ├── ProgressBar (name: JourneyProgress)
    │       └── Label (name: ETALabel, text: "ETA: [X] days")
    │
    ├── Label (text: "Journey Events")
    │
    ├── ScrollContainer (expand vertical, name: EventScrollContainer)
    │   └── VBoxContainer (name: EventListContainer)
    │       # Events will be added here dynamically
    │
    └── HBoxContainer (Controls)
        ├── Button (name: ExportButton, text: "Export Log")
        ├── CheckBox (name: ShowSystemEvents, text: "Show System Events")
        └── CheckBox (name: ShowMinorEvents, text: "Show Minor Events")
```

**Styling Requirements:**
- Use consistent theme throughout
- ScrollContainer should show visible scrollbar
- Events should have alternating background colors when added
- Important events (failures, conflicts) should use warning colors
- ProgressBar should be styled prominently

---

### Prompt 3B: Create Journey Event Entry Component

Create a reusable component for displaying individual journey events.

**File:** `Scenes/UI/JourneyEventEntry.tscn`

**Scene Structure:**

```
PanelContainer (root)
└── MarginContainer
    └── HBoxContainer
        ├── VBoxContainer (Timestamp Section, size_flags_h: SHRINK_BEGIN)
        │   ├── Label (name: TimeLabel, custom font: monospace, text: "Day 42")
        │   └── Label (name: GameTimeLabel, theme: dimmed, text: "15:30")
        │
        ├── TextureRect (name: EventIcon, size: 32x32)
        │
        ├── VBoxContainer (Content Section, size_flags_h: EXPAND_FILL)
        │   ├── Label (name: TitleLabel, theme: bold)
        │   ├── Label (name: DescriptionLabel, autowrap: enabled)
        │   └── VBoxContainer (name: ChoicesContainer, visible: false)
        │       # Choice buttons added dynamically
        │
        └── VBoxContainer (Metadata Section, size_flags_h: SHRINK_END)
            └── Label (name: SeverityLabel, text: "⚠️ Minor")
```

**Script Requirements:**
- Attach a script to the root PanelContainer
- Define a signal: `choice_selected(int choice_index)`
- This component will be instantiated dynamically for each event

---

### Prompt 3C: Implement Journey Log Presenter

Create the presenter script that controls the Ship Journey Log screen.

**File:** `Scripts/UI/ShipJourneyLogPresenter.cs`

**Class Structure:**

Create a partial class `ShipJourneyLogPresenter` inheriting from `Control` with:

**Exported Node References:**
- `_eventListContainer` (VBoxContainer)
- `_journeyProgress` (ProgressBar)
- `_progressLabel` (Label)
- `_etaLabel` (Label)
- `_showSystemEvents` (CheckBox)
- `_showMinorEvents` (CheckBox)
- `_exportButton` (Button)
- `_eventEntryScene` (PackedScene - the JourneyEventEntry scene)

**Private Fields:**
- `_eventStore` (IEventStore)
- `_stateStore` (StateStore)
- `_lastDisplayedOffset` (long, initialized to -1)

**Methods to Implement:**

1. **`_Ready()`:**
   - Get EventStore and StateStore from singleton/dependency injection
   - Connect to StateStore.StateChanged signal
   - Connect checkbox Toggled signals to RefreshEventList
   - Connect export button Pressed signal
   - Call initial refresh

2. **`OnStateChanged()`:**
   - Call UpdateJourneyProgress()
   - Call RefreshEventList()

3. **`UpdateJourneyProgress()`:**
   - Get current ship state from StateStore
   - Calculate progress percentage (TravelProgress: 0.0 to 1.0)
   - Calculate days elapsed and days remaining
   - Update progress bar value (0-100)
   - Update progress label: "Day {elapsed:F0} of {total:F0} - {percent:F1}% Complete"
   - Update ETA label: "ETA: {remaining:F0} days"

4. **`RefreshEventList()`:**
   - Clear existing children in _eventListContainer (QueueFree)
   - Get new events from EventStore starting at _lastDisplayedOffset + 1
   - Filter to journey-related events using IsJourneyEvent()
   - Apply checkbox filters using PassesFilters()
   - For each filtered event:
     - Instantiate _eventEntryScene
     - Cast to JourneyEventEntry
     - Call SetEvent(evt)
     - Connect ChoiceSelected signal to OnEventChoiceSelected
     - Add to _eventListContainer
   - Update _lastDisplayedOffset to current EventStore offset

5. **`IsJourneyEvent(GameEvent evt)`:**
   - Return true if event is one of:
     - ShipDepartedEvent
     - MechanicalFailureEvent
     - SocialConflictEvent
     - AnomalyDetectedEvent
     - ShipArrivedEvent

6. **`PassesFilters(GameEvent evt)`:**
   - Check _showSystemEvents checkbox state
   - Check _showMinorEvents checkbox state
   - Return true if event passes both filters
   - (Implement filtering logic based on event properties)

7. **`OnEventChoiceSelected(GameEvent evt, int choiceIndex)`:**
   - Create appropriate command based on player choice
   - Apply command to StateStore
   - Log the choice for debugging

8. **`OnExportPressed()`:**
   - Placeholder for export functionality (implement in Session 1.3.6)

---

### Prompt 3D: Implement Journey Event Entry Script

Create the script for the JourneyEventEntry component.

**File:** `Scripts/UI/JourneyEventEntry.cs`

**Class Structure:**

Create a partial class `JourneyEventEntry` inheriting from `PanelContainer`:

**Signal:**
- Define: `[Signal] public delegate void ChoiceSelectedEventHandler(int choiceIndex);`

**Exported Node References:**
- `_timeLabel` (Label)
- `_gameTimeLabel` (Label)
- `_eventIcon` (TextureRect)
- `_titleLabel` (Label)
- `_descriptionLabel` (Label)
- `_severityLabel` (Label)
- `_choicesContainer` (VBoxContainer)

**Private Fields:**
- `_event` (GameEvent)

**Methods to Implement:**

1. **`SetEvent(GameEvent evt)`:**
   - Store event in _event field
   - Format time display:
     - _timeLabel: "Day {evt.GameTime / 24.0f:F0}"
     - _gameTimeLabel: evt.RealTime.ToString("HH:mm")
   - Set icon based on event type (call GetIconForEventType)
   - Set title (call GetEventTitle)
   - Set description (call GetEventDescription)
   - Set severity label and color (call GetSeverityLabel and GetSeverityColor)
   - If event has player choices, call ShowChoices

2. **`GetEventTitle(GameEvent evt)`:**
   - Use switch expression to return appropriate title:
     - ShipDepartedEvent: "Journey Begins"
     - MechanicalFailureEvent: "Mechanical Failure: {SystemAffected}"
     - SocialConflictEvent: "Social Incident"
     - AnomalyDetectedEvent: "Anomaly Detected"
     - ShipArrivedEvent: "Arrival at Destination"
     - Default: return evt.EventType

3. **`GetEventDescription(GameEvent evt)`:**
   - Use switch expression to return appropriate description:
     - MechanicalFailureEvent: return failure.Description
     - SocialConflictEvent: return conflict.Description
     - AnomalyDetectedEvent: return anomaly.Description
     - Default: return generic message

4. **`GetSeverityLabel(GameEvent evt)`:**
   - Return appropriate severity indicator based on event type/properties
   - Examples: "⚠️ Minor", "🔴 Critical", "ℹ️ Info"

5. **`GetSeverityColor(GameEvent evt)`:**
   - Return appropriate color for the event:
     - Critical events: Colors.Red
     - Warnings: Colors.Orange
     - Info: Colors.Cyan
     - Default: Colors.White

6. **`GetIconForEventType(GameEvent evt)`:**
   - Return appropriate texture for event type
   - Use placeholder icons initially

7. **`ShowChoices(List<string> choices)`:**
   - Make _choicesContainer visible
   - For each choice string:
     - Create a Button
     - Set text to choice string
     - Connect Pressed signal to emit ChoiceSelected with index
     - Add button to _choicesContainer

---

## Session 1.3.4: Debug Event Panel

### Prompt 4A: Create Debug Panel Scene

Create an always-visible debug event panel for developers.

**File:** `Scenes/UI/DebugEventPanel.tscn`

**Scene Structure:**

```
CanvasLayer (root, layer: 100)
└── PanelContainer (anchors: top-right corner, drag-enabled)
    └── VBoxContainer
        ├── HBoxContainer (Header)
        │   ├── Label (text: "Debug: Events")
        │   ├── Button (name: PinButton, text: "📌", tooltip: "Pin panel")
        │   ├── Button (name: ClearButton, text: "🗑️", tooltip: "Clear events")
        │   └── Button (name: CloseButton, text: "❌", tooltip: "Close panel")
        │
        └── ScrollContainer (name: EventScrollContainer, custom_minimum_size: 300x200)
            └── VBoxContainer (name: DebugEventList)
                # Events added dynamically as Labels
```

**Styling Requirements:**
- Semi-transparent background (alpha: 0.9)
- Monospace font for event text
- Small font size (10-11pt)
- Color-coded text by event type
- Auto-scroll to bottom when new events arrive
- Draggable by header

**Behavior:**
- Press F3 to toggle visibility
- Pinned mode keeps panel visible when switching scenes
- Shows last 50 events only
- Updates in real-time

---

### Prompt 4B: Implement Debug Panel Script

Create the presenter script for the debug event panel.

**File:** `Scripts/UI/DebugEventPanel.cs`

**Class Structure:**

Create a partial class `DebugEventPanel` inheriting from `CanvasLayer`:

**Exported Node References:**
- `_eventList` (VBoxContainer)
- `_pinButton` (Button)
- `_clearButton` (Button)
- `_closeButton` (Button)
- `_scrollContainer` (ScrollContainer)

**Constants:**
- `MAX_EVENTS` = 50

**Private Fields:**
- `_isPinned` (bool, default: false)
- `_eventStore` (IEventStore)
- `_lastDisplayedOffset` (long, default: -1)

**Methods to Implement:**

1. **`_Ready()`:**
   - Get EventStore from singleton/service locator
   - Connect button signals:
     - _pinButton.Pressed → TogglePin
     - _clearButton.Pressed → ClearEvents
     - _closeButton.Pressed → Hide panel
   - Subscribe to StateStore.StateChanged or similar to get new event notifications
   - Start with Visible = false

2. **`_Input(InputEvent evt)`:**
   - Check if F3 key was pressed (InputEventKey, Keycode == Key.F3)
   - Toggle Visible property
   - If becoming visible, call RefreshEvents()

3. **`OnNewEvents()`:**
   - Only proceed if Visible == true
   - Call RefreshEvents()

4. **`RefreshEvents()`:**
   - Get new events from EventStore.ReadFrom(_lastDisplayedOffset + 1)
   - For each new event:
     - Create a Label
     - Set text using FormatEventForDebug(evt)
     - Override font_color theme using GetEventColor(evt)
     - Add label to _eventList
   - Trim oldest events if count exceeds MAX_EVENTS:
     - While _eventList.GetChildCount() > MAX_EVENTS:
       - Get first child and QueueFree()
   - Update _lastDisplayedOffset
   - Call CallDeferred(nameof(ScrollToBottom))

5. **`ScrollToBottom()`:**
   - Set _scrollContainer.ScrollVertical to maximum:
     - `_scrollContainer.ScrollVertical = (int)_scrollContainer.GetVScrollBar().MaxValue`

6. **`FormatEventForDebug(GameEvent evt)`:**
   - Return formatted string: "[{offset}] {gameTime:F1}h | {eventType}"
   - Example: "[42] 125.5h | MechanicalFailureEvent"

7. **`GetEventColor(GameEvent evt)`:**
   - Use switch expression to return color:
     - MechanicalFailureEvent: Colors.Orange
     - SocialConflictEvent: Colors.Red
     - AnomalyDetectedEvent: Colors.Cyan
     - ShipDepartedEvent: Colors.Green
     - ShipArrivedEvent: Colors.Green
     - Default: Colors.LightGray

8. **`TogglePin()`:**
   - Flip _isPinned boolean
   - Update button text: pinned ? "📍" : "📌"
   - If pinned, mark node to persist across scene changes (implementation depends on your scene management)

9. **`ClearEvents()`:**
   - Iterate through all children of _eventList and QueueFree them
   - Update _lastDisplayedOffset to EventStore.CurrentOffset

---

## Session 1.3.5: Global Event Log Screen

### Prompt 5A: Create Global Event Log Scene

Create a comprehensive event log screen with search and filtering.

**File:** `Scenes/EventLogScreen.tscn`

**Scene Structure:**

```
Control (root, full screen)
└── VBoxContainer
    ├── HBoxContainer (Header)
    │   ├── Label (text: "Event Log - [Session Name]")
    │   └── Button (name: BackButton, text: "Back")
    │
    ├── HBoxContainer (Filters and Controls)
    │   ├── LineEdit (name: SearchBox, placeholder_text: "Search events...")
    │   ├── OptionButton (name: TypeFilter)
    │   ├── OptionButton (name: SeverityFilter)
    │   ├── OptionButton (name: TimeFilter)
    │   ├── Button (name: ClearFiltersButton, text: "Clear Filters")
    │   ├── HSeparator
    │   ├── Button (name: ExportJsonButton, text: "Export JSON")
    │   └── Button (name: ExportYamlButton, text: "Export YAML")
    │
    └── HSplitContainer (name: MainSplit, expand vertical)
        ├── VBoxContainer (Left Panel, size_flags_h: EXPAND_FILL)
        │   ├── Label (name: CountLabel, text: "Events (0 total, 0 shown)")
        │   └── Tree (name: EventTree, columns: 4, hide_root: true)
        │       # Columns: Time | Type | Summary | Location
        │
        └── VBoxContainer (Right Panel, size_flags_h: EXPAND_FILL, custom_minimum_size_x: 300)
            ├── Label (text: "Event Details")
            └── PanelContainer
                └── ScrollContainer
                    └── VBoxContainer (name: DetailsContainer)
                        # Details populated when event selected
```

**Tree Configuration:**
- Column 0: "Time"
- Column 1: "Type"
- Column 2: "Summary"
- Column 3: "Location"

**OptionButton Items:**

TypeFilter:
- "All Types"
- "Exploration"
- "Ship"
- "Colony"
- "Economy"
- "Population"
- "Research"

SeverityFilter:
- "All"
- "Critical"
- "Important"
- "Minor"
- "Info"

TimeFilter:
- "All Time"
- "Recent (Last Hour)"
- "Current Phase"
- "Last Session"

**Styling:**
- Use Tree widget for performance
- Alternating row colors
- Color-code event types in Type column
- Details panel shows JSON when event selected

---

### Prompt 5B: Implement Global Event Log Presenter

Create the presenter script for the global event log screen.

**File:** `Scripts/UI/EventLogScreenPresenter.cs`

**Class Structure:**

Create a partial class `EventLogScreenPresenter` inheriting from `Control`:

**Exported Node References:**
- `_searchBox` (LineEdit)
- `_typeFilter` (OptionButton)
- `_severityFilter` (OptionButton)
- `_timeFilter` (OptionButton)
- `_clearFiltersButton` (Button)
- `_eventTree` (Tree)
- `_detailsContainer` (VBoxContainer)
- `_countLabel` (Label)
- `_exportJsonButton` (Button)
- `_exportYamlButton` (Button)

**Private Fields:**
- `_eventStore` (IEventStore)
- `_stateStore` (StateStore)
- `_allEvents` (List<GameEvent>)
- `_filteredEvents` (List<GameEvent>)

**Methods to Implement:**

1. **`_Ready()`:**
   - Setup Tree columns:
     - Set Columns = 4
     - Set column titles: "Time", "Type", "Summary", "Location"
   - Populate filter dropdowns with items (from scene specifications above)
   - Connect signals:
     - _searchBox.TextChanged → ApplyFilters
     - _typeFilter.ItemSelected → ApplyFilters
     - _severityFilter.ItemSelected → ApplyFilters
     - _timeFilter.ItemSelected → ApplyFilters
     - _clearFiltersButton.Pressed → ClearFilters
     - _eventTree.ItemSelected → OnEventSelected
     - _exportJsonButton.Pressed → ExportEvents("json")
     - _exportYamlButton.Pressed → ExportEvents("yaml")
   - Call LoadAllEvents()
   - Call ApplyFilters()

2. **`LoadAllEvents()`:**
   - Get all events from EventStore: `_eventStore.ReadFrom(0).ToList()`
   - Store in _allEvents
   - Print debug message with count

3. **`ApplyFilters()`:**
   - Get current filter values:
     - query = _searchBox.Text.ToLower()
     - typeIdx = _typeFilter.Selected
     - severityIdx = _severityFilter.Selected
     - timeIdx = _timeFilter.Selected
   - Filter _allEvents using LINQ:
     - Where PassesSearchFilter(e, query)
     - Where PassesTypeFilter(e, typeIdx)
     - Where PassesSeverityFilter(e, severityIdx)
     - Where PassesTimeFilter(e, timeIdx)
   - Store result in _filteredEvents
   - Call RefreshEventTree()
   - Update _countLabel: "Events ({_allEvents.Count} total, {_filteredEvents.Count} shown)"

4. **`PassesSearchFilter(GameEvent evt, string query)`:**
   - Return true if query is empty
   - Create searchable text combining event type and summary
   - Return true if searchable text contains query (case-insensitive)

5. **`PassesTypeFilter(GameEvent evt, int filterIndex)`:**
   - Return true if filterIndex == 0 (All Types)
   - Map filterIndex to event categories and check if evt matches

6. **`PassesSeverityFilter(GameEvent evt, int filterIndex)`:**
   - Return true if filterIndex == 0 (All)
   - Determine event severity and check if it matches filter

7. **`PassesTimeFilter(GameEvent evt, int filterIndex)`:**
   - Return true if filterIndex == 0 (All Time)
   - Filter based on event timestamps according to time range

8. **`ClearFilters()`:**
   - Reset _searchBox.Text to ""
   - Reset all filter dropdowns to index 0
   - Call ApplyFilters()

9. **`RefreshEventTree()`:**
   - Clear tree: _eventTree.Clear()
   - Create root: `var root = _eventTree.CreateItem()`
   - For each event in _filteredEvents:
     - Create tree item: `var item = _eventTree.CreateItem(root)`
     - Set column 0 (Time): `item.SetText(0, $"{evt.GameTime:F1}h")`
     - Set column 1 (Type): `item.SetText(1, evt.EventType.Replace("Event", ""))`
     - Set column 1 color: `item.SetCustomColor(1, GetEventTypeColor(evt))`
     - Set column 2 (Summary): `item.SetText(2, GetEventSummary(evt))`
     - Set column 3 (Location): `item.SetText(3, GetEventLocation(evt))`
     - Store event in metadata: `item.SetMetadata(0, evt)`

10. **`GetEventSummary(GameEvent evt)`:**
    - Use switch expression to return concise summary:
      - ShipDepartedEvent: "Departed for {destination}"
      - MechanicalFailureEvent: "{system}: {description}"
      - SocialConflictEvent: conflict description
      - AnomalyDetectedEvent: "{type} detected"
      - Default: event type name

11. **`GetEventLocation(GameEvent evt)`:**
    - Use switch expression to extract location:
      - ShipDepartedEvent: "Solar System"
      - ShipArrivedEvent: system name
      - Default: "-"

12. **`GetEventTypeColor(GameEvent evt)`:**
    - Return appropriate color for each event type category:
      - Exploration events: Colors.Blue
      - Ship events: Colors.Cyan
      - Colony events: Colors.Green
      - Economy events: Colors.Yellow
      - Population events: Colors.Purple
      - System events: Colors.Gray

13. **`OnEventSelected()`:**
    - Get selected tree item: `_eventTree.GetSelected()`
    - Return if null
    - Get event from metadata: `(GameEvent)selected.GetMetadata(0)`
    - Call ShowEventDetails(evt)

14. **`ShowEventDetails(GameEvent evt)`:**
    - Clear existing children in _detailsContainer
    - Add detail labels using AddDetailLabel helper:
      - "Offset": evt.Offset
      - "Game Time": formatted time
      - "Real Time": formatted datetime
      - "Type": evt.EventType
    - Call AddEventSpecificDetails(evt)
    - Add "Raw Data" section:
      - Create Label with "Raw Data:"
      - Create TextEdit with JSON serialization of event
      - Set TextEdit.Editable = false
      - Set custom minimum size
    - If CanNavigateToEvent(evt):
      - Add "Go to Location" button
      - Connect to NavigateToEvent(evt)

15. **`AddDetailLabel(string key, string value)`:**
    - Create Label with text: "{key}: {value}"
    - Add to _detailsContainer

16. **`AddEventSpecificDetails(GameEvent evt)`:**
    - Use switch expression to add type-specific details:
      - MechanicalFailureEvent: add system, severity, description
      - SocialConflictEvent: add conflict type, morale impact
      - Add similar for other event types

17. **`CanNavigateToEvent(GameEvent evt)`:**
    - Return true if event has associated scene/location
    - Examples: ShipArrivedEvent, ColonyEstablishedEvent

18. **`NavigateToEvent(GameEvent evt)`:**
    - Switch to appropriate scene based on event type
    - Pass relevant parameters (system ID, colony ID, etc.)

19. **`ExportEvents(string format)`:**
    - Placeholder for now (implement in Session 1.3.6)

---

## Session 1.3.6: Export Functionality

### Prompt 6A: Implement Event Export Service

Create a service class for exporting events to JSON and YAML formats.

**File:** `Scripts/Services/EventExporter.cs`

**Requirements:**

Create a static class `EventExporter` with these methods:

1. **`ExportToJson(IEnumerable<GameEvent> events, string filePath)`:**
   - Create JsonSerializerOptions with:
     - WriteIndented = true
     - PropertyNamingPolicy = JsonNamingPolicy.CamelCase
   - Serialize events to JSON string
   - Write to file using System.IO.File.WriteAllText
   - Log success message with count and path

2. **`ExportToYaml(IEnumerable<GameEvent> events, string filePath)`:**
   - Note: This requires the YamlDotNet NuGet package
   - Create SerializerBuilder with CamelCase naming convention
   - Build serializer
   - Serialize events to YAML string
   - Write to file
   - Log success message

**Error Handling:**
- Wrap file operations in try-catch
- Throw descriptive exceptions on failure

**Dependencies:**
- System.Text.Json (built-in)
- YamlDotNet (NuGet package - needs to be added to project)

---

### Prompt 6B: Wire Export to Event Log Screen

Implement the export functionality in the EventLogScreenPresenter.

**File:** `Scripts/UI/EventLogScreenPresenter.cs` (add to existing class)

**Method to Implement:**

Add or complete the `ExportEvents(string format)` method:

1. Create FileDialog:
   - `var fileDialog = new FileDialog()`
   - Set FileMode = FileDialog.FileModeEnum.SaveFile
   - Set Access = FileDialog.AccessEnum.Userdata
   - Set Filters based on format:
     - JSON: `new[] { "*.json ; JSON Files" }`
     - YAML: `new[] { "*.yaml ; YAML Files" }`
   - Set CurrentFile to generated filename: `event_log_{timestamp}.{format}`

2. Connect FileSelected signal:
   - In handler, wrap in try-catch:
     - If format == "json": call EventExporter.ExportToJson
     - If format == "yaml": call EventExporter.ExportToYaml
     - Pass _filteredEvents and selected path
     - On success: show notification
     - On error: show error message with exception details

3. Add FileDialog to scene tree and show:
   - AddChild(fileDialog)
   - fileDialog.PopupCentered(new Vector2I(800, 600))

**Helper Methods:**

Also implement these notification methods:

1. **`ShowNotification(string message)`:**
   - Create temporary Label or AcceptDialog
   - Display success message
   - Auto-dismiss after 3 seconds

2. **`ShowError(string message)`:**
   - Create AcceptDialog with error message
   - Show with OK button

---

### Prompt 6C: Wire Export to Journey Log Screen

Implement export functionality in the Ship Journey Log screen.

**File:** `Scripts/UI/ShipJourneyLogPresenter.cs` (add to existing class)

**Method to Complete:**

Implement the `OnExportPressed()` method (currently placeholder):

1. Create ConfirmationDialog to choose format:
   - "Export journey log as JSON or YAML?"
   - Button 1: "JSON"
   - Button 2: "YAML"

2. On format selection:
   - Get all journey events (use IsJourneyEvent filter)
   - Create FileDialog similar to EventLogScreenPresenter
   - Use EventExporter to save file
   - Show confirmation message

**Alternative Simpler Approach:**

Or, create two separate export buttons:
- "Export as JSON" button
- "Export as YAML" button
- Each directly opens FileDialog and exports

---

## Session 1.3.7: Testing and Integration

### Prompt 7A: Create FileEventStore Unit Tests

Create unit tests for the FileEventStore implementation.

**File:** `Tests/FileEventStoreTests.cs`

**Tests to Implement:**

Create a test class with these test methods:

1. **`Append_SingleEvent_WritesToFile()`:**
   - Create temp file path
   - Create FileEventStore with temp path
   - Create test event
   - Call Append
   - Assert: File exists
   - Assert: File contains 1 line
   - Assert: CurrentOffset == 0

2. **`Append_MultipleEvents_AssignsSequentialOffsets()`:**
   - Append 3 events
   - Verify file has 3 lines
   - Verify CurrentOffset == 2
   - Parse lines and verify offsets are 0, 1, 2

3. **`ReadFrom_ValidOffset_ReturnsCorrectEvents()`:**
   - Append 5 test events
   - Call ReadFrom(2)
   - Assert: Returns 3 events (indices 2, 3, 4)
   - Verify event content matches

4. **`ReadFrom_ZeroOffset_ReturnsAllEvents()`:**
   - Append 3 events
   - Call ReadFrom(0)
   - Assert: Returns all 3 events

5. **`ReadFrom_OffsetBeyondEnd_ReturnsEmpty()`:**
   - Append 2 events
   - Call ReadFrom(10)
   - Assert: Returns empty enumerable

6. **`Constructor_ExistingFile_RestoresOffset()`:**
   - Create store and append 3 events
   - Dispose store
   - Create new store with same file path
   - Assert: CurrentOffset == 2
   - Assert: Count == 3

7. **`Append_CorruptedLine_ThrowsException()`:**
   - Create store and append 1 event
   - Manually corrupt file (write invalid JSON)
   - Create new store
   - Assert: ReadFrom throws EventStoreException

**Setup/Teardown:**
- Create unique temp file path for each test
- Delete temp file in cleanup

---

### Prompt 7B: Create Integration Test for Journey Log

Create an integration test for the journey log workflow.

**File:** `Tests/JourneyLogIntegrationTests.cs`

**Test Scenario:**

Create a test that simulates a full journey with events:

1. **`JourneyLog_FullWorkflow_DisplaysEventsCorrectly()`:**
   
   **Setup:**
   - Create FileEventStore with temp file
   - Create StateStore with EventStore
   - Create mock GameState with ship journey in progress
   
   **Actions:**
   - Append test events in sequence:
     - ShipDepartedEvent
     - 2-3 MechanicalFailureEvents
     - SocialConflictEvent
     - AnomalyDetectedEvent
     - ShipArrivedEvent
   
   **Assertions:**
   - Verify all events persist to file
   - Verify ReadFrom returns events in order
   - Verify events can be filtered by type
   - Verify export to JSON produces valid file
   - Verify export to YAML produces valid file
   
   **Note:** This may need to be a manual test or require Godot test framework if testing UI components.

---

### Prompt 7C: Add Event Store to Global Dependency Injection

Wire the EventStore into your application's dependency injection or service locator.

**File:** `Scripts/App.cs` (or your bootstrap file)

**Requirements:**

In your application bootstrap code:

1. Create FileEventStore instance:
   - Use path in Godot's user directory: `user://events.log`
   - Store as singleton/global service

2. Pass EventStore to StateStore:
   - When creating StateStore, inject the EventStore instance

3. Make EventStore accessible to UI:
   - Add to service locator/DI container
   - Or expose via autoload singleton in Godot

4. Initialize Debug Panel:
   - Create DebugEventPanel as autoload
   - Pass EventStore reference
   - Set initial visibility to false

**Example Pattern:**

```csharp
public partial class App : Node
{
    public override void _Ready()
    {
        // Create event store
        var eventsPath = ProjectSettings.GlobalizePath("user://events.log");
        var eventStore = new FileEventStore(eventsPath);
        
        // Create state store with event store
        var initialState = GameState.CreateDefault();
        var stateStore = new StateStore(eventStore, initialState);
        
        // Register in service locator
        ServiceLocator.Register<IEventStore>(eventStore);
        ServiceLocator.Register<StateStore>(stateStore);
        
        // Create and add debug panel
        var debugPanel = GD.Load<PackedScene>("res://Scenes/UI/DebugEventPanel.tscn").Instantiate<DebugEventPanel>();
        AddChild(debugPanel);
    }
}
```

---

## Final Checklist

After implementing all prompts, verify these items:

### FileEventStore:
- [ ] Appends events successfully
- [ ] Persists to file immediately
- [ ] Reads events from any offset
- [ ] Handles file corruption gracefully
- [ ] Restores offset on restart

### Ship Journey Log:
- [ ] Displays journey events in real-time
- [ ] Updates progress bar correctly
- [ ] Filters work (system events, minor events)
- [ ] Player choices create commands
- [ ] Export creates valid files

### Debug Panel:
- [ ] F3 toggles visibility
- [ ] Shows last 50 events
- [ ] Color codes by type
- [ ] Auto-scrolls to bottom
- [ ] Clear button works
- [ ] Pin mode persists

### Global Event Log:
- [ ] Loads all events on open
- [ ] Search finds events correctly
- [ ] All filters work independently
- [ ] Event selection shows details
- [ ] Navigation buttons work
- [ ] Export JSON works
- [ ] Export YAML works
- [ ] Handles 1000+ events smoothly

### Integration:
- [ ] Events persist across app restarts
- [ ] All three event displays show same events
- [ ] StateStore correctly appends to EventStore
- [ ] No duplicate events
- [ ] Event offsets are contiguous