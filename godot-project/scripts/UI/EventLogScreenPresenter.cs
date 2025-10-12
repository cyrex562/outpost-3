using Godot;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;
using Outpost3.Core;
using Outpost3.Services;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the global event log screen.
/// Provides comprehensive event viewing, filtering, and export capabilities.
/// </summary>
public partial class EventLogScreenPresenter : Control
{
    // Exported node references
    [Export] private LineEdit _searchBox;
    [Export] private OptionButton _typeFilter;
    [Export] private OptionButton _severityFilter;
    [Export] private OptionButton _timeFilter;
    [Export] private Button _clearFiltersButton;
    [Export] private Tree _eventTree;
    [Export] private VBoxContainer _detailsContainer;
    [Export] private Label _countLabel;
    [Export] private Button _exportJsonButton;
    [Export] private Button _exportYamlButton;
    [Export] private Button _backButton;

    // Private fields
    private IEventStore _eventStore;
    private StateStore _stateStore;
    private List<GameEvent> _allEvents = new();
    private List<GameEvent> _filteredEvents = new();

    public override void _Ready()
    {
        // Get node references if not set via exports
        _searchBox ??= GetNode<LineEdit>("VBoxContainer/FiltersAndControls/SearchBox");
        _typeFilter ??= GetNode<OptionButton>("VBoxContainer/FiltersAndControls/TypeFilter");
        _severityFilter ??= GetNode<OptionButton>("VBoxContainer/FiltersAndControls/SeverityFilter");
        _timeFilter ??= GetNode<OptionButton>("VBoxContainer/FiltersAndControls/TimeFilter");
        _clearFiltersButton ??= GetNode<Button>("VBoxContainer/FiltersAndControls/ClearFiltersButton");
        _eventTree ??= GetNode<Tree>("VBoxContainer/MainSplit/LeftPanel/EventTree");
        _detailsContainer ??= GetNode<VBoxContainer>("VBoxContainer/MainSplit/RightPanel/DetailsPanel/DetailsScrollContainer/DetailsContainer");
        _countLabel ??= GetNode<Label>("VBoxContainer/MainSplit/LeftPanel/CountLabel");
        _exportJsonButton ??= GetNode<Button>("VBoxContainer/FiltersAndControls/ExportJsonButton");
        _exportYamlButton ??= GetNode<Button>("VBoxContainer/FiltersAndControls/ExportYamlButton");
        _backButton ??= GetNode<Button>("VBoxContainer/Header/BackButton");

        // Get services
        _eventStore = GetEventStore();
        _stateStore = GetStateStore();

        // Setup Tree columns
        _eventTree.Columns = 4;
        _eventTree.SetColumnTitle(0, "Time");
        _eventTree.SetColumnTitle(1, "Type");
        _eventTree.SetColumnTitle(2, "Summary");
        _eventTree.SetColumnTitle(3, "Location");

        // Connect signals
        _searchBox.TextChanged += (_) => ApplyFilters();
        _typeFilter.ItemSelected += (_) => ApplyFilters();
        _severityFilter.ItemSelected += (_) => ApplyFilters();
        _timeFilter.ItemSelected += (_) => ApplyFilters();
        _clearFiltersButton.Pressed += ClearFilters;
        _eventTree.ItemSelected += OnEventSelected;
        _exportJsonButton.Pressed += () => ExportEvents("json");
        _exportYamlButton.Pressed += () => ExportEvents("yaml");
        _backButton.Pressed += () => GetTree().ChangeSceneToFile("res://scenes/Main.tscn");

        // Load and display events
        LoadAllEvents();
        ApplyFilters();
    }

    /// <summary>
    /// Loads all events from the EventStore.
    /// </summary>
    private void LoadAllEvents()
    {
        if (_eventStore == null)
        {
            GD.PrintErr("EventLogScreenPresenter: EventStore is null");
            return;
        }

        _allEvents = _eventStore.ReadFrom(0).ToList();
        GD.Print($"EventLogScreenPresenter: Loaded {_allEvents.Count} events");
    }

    /// <summary>
    /// Applies all active filters to the event list.
    /// </summary>
    private void ApplyFilters()
    {
        if (_allEvents == null) return;

        var query = _searchBox?.Text?.ToLower() ?? "";
        var typeIdx = _typeFilter?.Selected ?? 0;
        var severityIdx = _severityFilter?.Selected ?? 0;
        var timeIdx = _timeFilter?.Selected ?? 0;

        _filteredEvents = _allEvents
            .Where(e => PassesSearchFilter(e, query))
            .Where(e => PassesTypeFilter(e, typeIdx))
            .Where(e => PassesSeverityFilter(e, severityIdx))
            .Where(e => PassesTimeFilter(e, timeIdx))
            .ToList();

        RefreshEventTree();

        if (_countLabel != null)
        {
            _countLabel.Text = $"Events ({_allEvents.Count} total, {_filteredEvents.Count} shown)";
        }
    }

    /// <summary>
    /// Checks if an event passes the search filter.
    /// </summary>
    private bool PassesSearchFilter(GameEvent evt, string query)
    {
        if (string.IsNullOrEmpty(query)) return true;

        var searchable = $"{evt.EventType} {GetEventSummary(evt)}".ToLower();
        return searchable.Contains(query);
    }

    /// <summary>
    /// Checks if an event passes the type filter.
    /// </summary>
    private bool PassesTypeFilter(GameEvent evt, int filterIndex)
    {
        if (filterIndex == 0) return true; // All Types

        return filterIndex switch
        {
            1 => IsExplorationType(evt),  // Exploration
            2 => IsShipType(evt),          // Ship
            3 => IsColonyType(evt),        // Colony
            4 => IsEconomyType(evt),       // Economy
            5 => IsPopulationType(evt),    // Population
            6 => IsResearchType(evt),      // Research
            _ => true
        };
    }

    /// <summary>
    /// Checks if an event passes the severity filter.
    /// </summary>
    private bool PassesSeverityFilter(GameEvent evt, int filterIndex)
    {
        if (filterIndex == 0) return true; // All

        var severity = GetEventSeverity(evt);
        return filterIndex switch
        {
            1 => severity == "Critical",
            2 => severity == "Important",
            3 => severity == "Minor",
            4 => severity == "Info",
            _ => true
        };
    }

    /// <summary>
    /// Checks if an event passes the time filter.
    /// </summary>
    private bool PassesTimeFilter(GameEvent evt, int filterIndex)
    {
        if (filterIndex == 0) return true; // All Time

        var now = DateTime.UtcNow;
        return filterIndex switch
        {
            1 => (now - evt.RealTime).TotalHours <= 1,  // Recent (Last Hour)
            2 => IsCurrentPhase(evt),                    // Current Phase
            3 => IsLastSession(evt),                     // Last Session
            _ => true
        };
    }

    /// <summary>
    /// Clears all filters.
    /// </summary>
    private void ClearFilters()
    {
        if (_searchBox != null) _searchBox.Text = "";
        if (_typeFilter != null) _typeFilter.Selected = 0;
        if (_severityFilter != null) _severityFilter.Selected = 0;
        if (_timeFilter != null) _timeFilter.Selected = 0;
        ApplyFilters();
    }

    /// <summary>
    /// Refreshes the event tree display.
    /// </summary>
    private void RefreshEventTree()
    {
        if (_eventTree == null) return;

        _eventTree.Clear();
        var root = _eventTree.CreateItem();

        foreach (var evt in _filteredEvents)
        {
            var item = _eventTree.CreateItem(root);
            
            // Column 0: Time
            item.SetText(0, $"{evt.GameTime:F1}h");
            
            // Column 1: Type
            var typeName = evt.EventType.Replace("Event", "");
            item.SetText(1, typeName);
            item.SetCustomColor(1, GetEventTypeColor(evt));
            
            // Column 2: Summary
            item.SetText(2, GetEventSummary(evt));
            
            // Column 3: Location
            item.SetText(3, GetEventLocation(evt));
            
            // Store event offset in metadata (use offset to look up event later)
            item.SetMetadata(0, evt.Offset);
        }
    }

    /// <summary>
    /// Gets a concise summary for an event.
    /// </summary>
    private string GetEventSummary(GameEvent evt)
    {
        return evt switch
        {
            ShipDepartedEvent departure => $"Departed for {departure.DestinationSystemId}",
            MechanicalFailureEvent failure => $"{failure.SystemAffected}: {failure.Description}",
            SocialConflictEvent conflict => conflict.Description,
            AnomalyDetectedEvent anomaly => $"{anomaly.AnomalyType} detected",
            ShipArrivedEvent arrival => $"Arrived at {arrival.SystemName}",
            _ => evt.EventType
        };
    }

    /// <summary>
    /// Gets the location associated with an event.
    /// </summary>
    private string GetEventLocation(GameEvent evt)
    {
        return evt switch
        {
            ShipDepartedEvent => "Solar System",
            ShipArrivedEvent arrival => arrival.SystemName,
            _ => "-"
        };
    }

    /// <summary>
    /// Gets the color for an event type.
    /// </summary>
    private Color GetEventTypeColor(GameEvent evt)
    {
        return evt switch
        {
            AnomalyDetectedEvent => Colors.Blue,          // Exploration
            ShipDepartedEvent => Colors.Cyan,             // Ship
            ShipArrivedEvent => Colors.Cyan,              // Ship
            MechanicalFailureEvent => Colors.Cyan,        // Ship
            SocialConflictEvent => Colors.Purple,         // Population
            _ => Colors.Gray                              // System
        };
    }

    /// <summary>
    /// Called when an event is selected in the tree.
    /// </summary>
    private void OnEventSelected()
    {
        var selected = _eventTree.GetSelected();
        if (selected == null) return;

        // Get the offset from metadata and find the event
        var offset = selected.GetMetadata(0).AsInt64();
        var evt = _filteredEvents.FirstOrDefault(e => e.Offset == offset);
        
        if (evt != null)
        {
            ShowEventDetails(evt);
        }
    }

    /// <summary>
    /// Displays detailed information about an event.
    /// </summary>
    private void ShowEventDetails(GameEvent evt)
    {
        if (_detailsContainer == null) return;

        // Clear existing children
        foreach (Node child in _detailsContainer.GetChildren())
        {
            child.QueueFree();
        }

        // Add basic details
        AddDetailLabel("Offset", evt.Offset.ToString());
        AddDetailLabel("Game Time", $"{evt.GameTime:F1} hours");
        AddDetailLabel("Real Time", evt.RealTime.ToString("yyyy-MM-dd HH:mm:ss"));
        AddDetailLabel("Type", evt.EventType);

        // Add event-specific details
        AddEventSpecificDetails(evt);

        // Add raw data section
        var rawLabel = new Label { Text = "Raw Data:" };
        rawLabel.AddThemeColorOverride("font_color", Colors.Gray);
        _detailsContainer.AddChild(rawLabel);

        var jsonOptions = new JsonSerializerOptions 
        { 
            WriteIndented = true,
            PropertyNamingPolicy = JsonNamingPolicy.CamelCase
        };
        var json = JsonSerializer.Serialize(evt, evt.GetType(), jsonOptions);

        var textEdit = new TextEdit
        {
            Text = json,
            Editable = false,
            CustomMinimumSize = new Vector2(0, 150),
            SyntaxHighlighter = null
        };
        _detailsContainer.AddChild(textEdit);
    }

    /// <summary>
    /// Adds a detail label to the details container.
    /// </summary>
    private void AddDetailLabel(string key, string value)
    {
        var label = new Label { Text = $"{key}: {value}" };
        _detailsContainer?.AddChild(label);
    }

    /// <summary>
    /// Adds event-specific detail fields.
    /// </summary>
    private void AddEventSpecificDetails(GameEvent evt)
    {
        switch (evt)
        {
            case MechanicalFailureEvent failure:
                AddDetailLabel("System Affected", failure.SystemAffected);
                AddDetailLabel("Severity", failure.Severity);
                AddDetailLabel("Description", failure.Description);
                break;
            case SocialConflictEvent conflict:
                AddDetailLabel("Conflict Type", conflict.ConflictType);
                AddDetailLabel("Morale Impact", conflict.MoraleImpact.ToString("F2"));
                AddDetailLabel("Description", conflict.Description);
                break;
            case AnomalyDetectedEvent anomaly:
                AddDetailLabel("Anomaly Type", anomaly.AnomalyType);
                AddDetailLabel("Description", anomaly.Description);
                break;
            case ShipDepartedEvent departure:
                AddDetailLabel("Ship Name", departure.ShipName);
                AddDetailLabel("Colonist Count", departure.ColonistCount.ToString());
                AddDetailLabel("Destination", departure.DestinationSystemId.ToString());
                break;
            case ShipArrivedEvent arrival:
                AddDetailLabel("System Name", arrival.SystemName);
                AddDetailLabel("Travel Duration", $"{arrival.TravelDuration:F1} days");
                break;
        }
    }

    /// <summary>
    /// Exports events to file.
    /// </summary>
    private void ExportEvents(string format)
    {
        if (_filteredEvents == null || _filteredEvents.Count == 0)
        {
            ShowNotification("No events to export.");
            return;
        }

        var fileDialog = new FileDialog
        {
            FileMode = FileDialog.FileModeEnum.SaveFile,
            Access = FileDialog.AccessEnum.Filesystem,
            UseNativeDialog = false
        };

        // Set file filters based on format
        if (format == "json")
        {
            fileDialog.Filters = new[] { "*.json ; JSON Files" };
            fileDialog.CurrentFile = EventExporter.GenerateExportFilename("event_log", "json");
        }
        else if (format == "yaml")
        {
            fileDialog.Filters = new[] { "*.yaml ; YAML Files" };
            fileDialog.CurrentFile = EventExporter.GenerateExportFilename("event_log", "yaml");
        }

        // Connect file selected signal
        fileDialog.FileSelected += (selectedPath) => OnFileSelectedForExport(selectedPath, format);

        // Add to scene tree and show
        AddChild(fileDialog);
        fileDialog.PopupCentered(new Vector2I(800, 600));
    }

    /// <summary>
    /// Called when a file is selected for export.
    /// </summary>
    private void OnFileSelectedForExport(string filePath, string format)
    {
        try
        {
            if (format == "json")
            {
                EventExporter.ExportToJson(_filteredEvents, filePath);
                ShowNotification($"Successfully exported {_filteredEvents.Count} events to JSON.");
            }
            else if (format == "yaml")
            {
                EventExporter.ExportToYaml(_filteredEvents, filePath);
                ShowNotification($"Successfully exported {_filteredEvents.Count} events to YAML.");
            }
        }
        catch (NotImplementedException ex)
        {
            ShowError($"Export failed: {ex.Message}");
        }
        catch (Exception ex)
        {
            ShowError($"Export failed: {ex.Message}");
            GD.PrintErr($"EventLogScreenPresenter: Export error - {ex}");
        }
    }

    /// <summary>
    /// Shows an error message dialog.
    /// </summary>
    private void ShowError(string message)
    {
        var dialog = new AcceptDialog
        {
            DialogText = message,
            Title = "Export Error"
        };
        AddChild(dialog);
        dialog.PopupCentered();
    }

    /// <summary>
    /// Shows a notification message.
    /// </summary>
    private void ShowNotification(string message)
    {
        var dialog = new AcceptDialog
        {
            DialogText = message,
            Title = "Info"
        };
        AddChild(dialog);
        dialog.PopupCentered();
    }

    // Type categorization helpers
    private bool IsExplorationType(GameEvent evt) => evt is AnomalyDetectedEvent;
    private bool IsShipType(GameEvent evt) => evt is ShipDepartedEvent or ShipArrivedEvent or MechanicalFailureEvent;
    private bool IsColonyType(GameEvent evt) => false; // TODO: Add colony events
    private bool IsEconomyType(GameEvent evt) => false; // TODO: Add economy events
    private bool IsPopulationType(GameEvent evt) => evt is SocialConflictEvent;
    private bool IsResearchType(GameEvent evt) => false; // TODO: Add research events

    /// <summary>
    /// Gets the severity level of an event.
    /// </summary>
    private string GetEventSeverity(GameEvent evt)
    {
        return evt switch
        {
            MechanicalFailureEvent failure => failure.Severity,
            SocialConflictEvent conflict => conflict.MoraleImpact switch
            {
                < -0.5f => "Critical",
                < -0.2f => "Important",
                < 0 => "Minor",
                _ => "Info"
            },
            ShipDepartedEvent => "Important",
            ShipArrivedEvent => "Important",
            AnomalyDetectedEvent => "Info",
            _ => "Info"
        };
    }

    /// <summary>
    /// Checks if event is from current phase.
    /// </summary>
    private bool IsCurrentPhase(GameEvent evt)
    {
        // TODO: Implement based on actual game phase tracking
        return true;
    }

    /// <summary>
    /// Checks if event is from last session.
    /// </summary>
    private bool IsLastSession(GameEvent evt)
    {
        // TODO: Implement based on actual session tracking
        // For now, check if from last 24 hours
        return (DateTime.UtcNow - evt.RealTime).TotalHours <= 24;
    }

    /// <summary>
    /// Gets the EventStore instance.
    /// </summary>
    private IEventStore GetEventStore()
    {
        var app = GetNodeOrNull("/root/App");
        if (app != null)
        {
            var result = app.Call("GetEventStore");
            if (result.VariantType != Variant.Type.Nil)
            {
                return result.AsGodotObject() as IEventStore;
            }
        }

        GD.PrintErr("EventLogScreenPresenter: Could not find EventStore");
        return null;
    }

    /// <summary>
    /// Gets the StateStore instance.
    /// </summary>
    private StateStore GetStateStore()
    {
        var app = GetNodeOrNull("/root/App");
        if (app != null)
        {
            var result = app.Call("GetStateStore");
            if (result.VariantType != Variant.Type.Nil)
            {
                return result.AsGodotObject() as StateStore;
            }
        }

        GD.PrintErr("EventLogScreenPresenter: Could not find StateStore");
        return null;
    }
}
