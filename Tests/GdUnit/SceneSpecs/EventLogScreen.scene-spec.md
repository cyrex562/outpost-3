# EventLogScreen Scene Test Specification

**Scene Path**: `res://scenes/UI/EventLogScreen.tscn`  
**Presenter**: `EventLogScreenPresenter.cs`  
**Last Updated**: 2025-10-11

## Overview

The Event Log Screen provides comprehensive event filtering, searching, and export functionality for game events.

## Components Under Test

### Search & Filter Functionality
- [ ] Search box filters events by text (description, title, etc.)
- [ ] Type filter dropdown filters by event type (All, Ship, Mechanical, Social, etc.)
- [ ] Severity filter dropdown filters by severity level
- [ ] Time filter dropdown filters by time range
- [ ] Clear filters button resets all filters
- [ ] Filters work in combination (AND logic)
- [ ] Filter state persists during session

### Event Display
- [ ] Events display in chronological order
- [ ] Event tree shows correct hierarchy
- [ ] Clicking event shows details panel
- [ ] Details panel shows all event properties
- [ ] Event count label updates correctly
- [ ] Empty state message appears when no events match filters

### Export Functionality
- [ ] Export JSON button triggers file save dialog
- [ ] Exported JSON contains all filtered events
- [ ] Exported JSON is valid and parseable
- [ ] Export YAML button triggers file save dialog
- [ ] Exported YAML contains all filtered events
- [ ] Exported YAML is valid

### Navigation
- [ ] Back button returns to previous screen
- [ ] Screen state is preserved when navigating away and back

## Node Path Tests (Automated)

```csharp
[TestCase]
public void AllRequiredNodesExist()
{
    // These @onready paths must resolve correctly:
    // - %SearchBox
    // - %TypeFilter
    // - %SeverityFilter
    // - %TimeFilter
    // - %ClearFiltersButton
    // - %EventTree
    // - %DetailsContainer
    // - %CountLabel
    // - %ExportJsonButton
    // - %ExportYamlButton
    // - %BackButton
}
```

## Integration Tests (Automated with GDUnit4)

```csharp
[TestCase]
public void Search_FiltersEventsByText()
{
    // Arrange: Add events with different descriptions
    // Act: Enter search text
    // Assert: Only matching events shown
}

[TestCase]
public void TypeFilter_ShowsOnlySelectedType()
{
    // Arrange: Add events of multiple types
    // Act: Select "Ship" from type filter
    // Assert: Only ship events shown
}

[TestCase]
public void ExportJson_ContainsFilteredEvents()
{
    // Arrange: Add events and apply filter
    // Act: Export to JSON
    // Assert: JSON contains only filtered events
}
```

## Manual Test Checklist

Date: ______ | Tester: ______ | Build: ______

### Visual Tests
- [ ] All UI elements are visible and properly aligned
- [ ] Font sizes are readable
- [ ] Colors match theme
- [ ] Icons display correctly
- [ ] Scrollbars appear when needed

### Interaction Tests
- [ ] Search box accepts keyboard input
- [ ] Dropdowns open and close properly
- [ ] Buttons respond to clicks with visual feedback
- [ ] Tree items expand/collapse smoothly
- [ ] Details panel scrolls if content is long

### Performance Tests
- [ ] Screen loads within 1 second
- [ ] Filtering 100+ events is instant
- [ ] Exporting 1000+ events completes within 5 seconds
- [ ] No lag when typing in search box

### Edge Cases
- [ ] Works with 0 events (empty state)
- [ ] Works with 1 event
- [ ] Works with 10,000+ events
- [ ] Handles very long event descriptions
- [ ] Handles events with special characters
- [ ] Handles events with null/missing properties

## Known Issues

- None currently

## Test Coverage Status

| Category | Automated | Manual | Status |
|----------|-----------|--------|--------|
| Search & Filter | ⚠️ 0/6 | ✅ 6/6 | 🟡 Manual Only |
| Event Display | ⚠️ 0/6 | ✅ 6/6 | 🟡 Manual Only |
| Export | ⚠️ 0/6 | ✅ 6/6 | 🟡 Manual Only |
| Navigation | ⚠️ 0/2 | ✅ 2/2 | 🟡 Manual Only |
| Node Paths | ❌ 0/1 | N/A | 🔴 Not Tested |

**Overall**: 🔴 Needs automated tests

## Next Steps

1. Add GDUnit4 test for scene loading and node path resolution
2. Add integration tests for search and filter functionality
3. Add export functionality tests
4. Consider adding performance benchmarks
