using Godot;
using System;
using System.Linq;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;
using Outpost3.Core;
using Outpost3.Services;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the Ship Journey Log screen.
/// Displays journey progress and events for the current ship journey.
/// </summary>
public partial class ShipJourneyLogPresenter : Control
{
    // Exported node references
    [Export] private VBoxContainer _eventListContainer;
    [Export] private ProgressBar _journeyProgress;
    [Export] private Label _progressLabel;
    [Export] private Label _etaLabel;
    [Export] private CheckBox _showSystemEvents;
    [Export] private CheckBox _showMinorEvents;
    [Export] private Button _exportButton;
    [Export] private PackedScene _eventEntryScene;

    // Private fields
    private IEventStore _eventStore;
    private StateStore _stateStore;
    private long _lastDisplayedOffset = -1;

    public override void _Ready()
    {
        // Get services from singleton/dependency injection
        // TODO: Replace with actual service locator/DI pattern
        _eventStore = GetEventStore();
        _stateStore = GetStateStore();

        if (_eventStore == null)
        {
            GD.PrintErr("ShipJourneyLogPresenter: EventStore not found!");
            return;
        }

        if (_stateStore == null)
        {
            GD.PrintErr("ShipJourneyLogPresenter: StateStore not found!");
            return;
        }

        // Connect to StateStore signals
        _stateStore.StateChanged += OnStateChanged;

        // Connect checkbox signals
        if (_showSystemEvents != null)
        {
            _showSystemEvents.Toggled += (_) => RefreshEventList();
        }

        if (_showMinorEvents != null)
        {
            _showMinorEvents.Toggled += (_) => RefreshEventList();
        }

        // Connect export button
        if (_exportButton != null)
        {
            _exportButton.Pressed += OnExportPressed;
        }

        // Initial refresh
        UpdateJourneyProgress();
        RefreshEventList();
    }

    public override void _ExitTree()
    {
        // Disconnect signals
        if (_stateStore != null)
        {
            _stateStore.StateChanged -= OnStateChanged;
        }
    }

    /// <summary>
    /// Called when StateStore state changes.
    /// </summary>
    private void OnStateChanged()
    {
        UpdateJourneyProgress();
        RefreshEventList();
    }

    /// <summary>
    /// Updates the journey progress display.
    /// </summary>
    private void UpdateJourneyProgress()
    {
        if (_stateStore == null) return;

        var state = _stateStore.State;
        
        // TODO: Get actual ship journey data from state
        // For now, use placeholder values
        float travelProgress = 0.0f; // 0.0 to 1.0
        float totalDays = 100.0f;
        
        // Check if state has journey data
        // This will depend on your actual GameState structure
        // Example:
        // if (state.CurrentShipJourney != null)
        // {
        //     travelProgress = state.CurrentShipJourney.TravelProgress;
        //     totalDays = state.CurrentShipJourney.TotalTravelTime / 24.0f;
        // }

        float daysElapsed = totalDays * travelProgress;
        float daysRemaining = totalDays - daysElapsed;
        float percentComplete = travelProgress * 100.0f;

        // Update progress bar (0-100 scale)
        if (_journeyProgress != null)
        {
            _journeyProgress.Value = percentComplete;
        }

        // Update progress label
        if (_progressLabel != null)
        {
            _progressLabel.Text = $"Day {daysElapsed:F0} of {totalDays:F0} - {percentComplete:F1}% Complete";
        }

        // Update ETA label
        if (_etaLabel != null)
        {
            _etaLabel.Text = $"ETA: {daysRemaining:F0} days";
        }
    }

    /// <summary>
    /// Refreshes the event list display.
    /// </summary>
    private void RefreshEventList()
    {
        if (_eventListContainer == null || _eventStore == null) return;

        // Clear existing children
        foreach (Node child in _eventListContainer.GetChildren())
        {
            child.QueueFree();
        }

        // Get new events from EventStore
        var newEvents = _eventStore.ReadFrom(_lastDisplayedOffset + 1)
            .Where(IsJourneyEvent)
            .Where(PassesFilters)
            .ToList();

        // Create entry for each event
        foreach (var evt in newEvents)
        {
            if (_eventEntryScene == null)
            {
                GD.PrintErr("ShipJourneyLogPresenter: _eventEntryScene is null!");
                continue;
            }

            // Instantiate event entry
            var entryInstance = _eventEntryScene.Instantiate();
            if (entryInstance is not JourneyEventEntry entry)
            {
                GD.PrintErr($"ShipJourneyLogPresenter: Instantiated scene is not JourneyEventEntry, got {entryInstance.GetType().Name}");
                entryInstance.QueueFree();
                continue;
            }

            // Set event data
            entry.SetEvent(evt);

            // Connect choice signal
            entry.ChoiceSelected += (choiceIndex) => OnEventChoiceSelected(evt, choiceIndex);

            // Add to container
            _eventListContainer.AddChild(entry);
        }

        // Update last displayed offset
        if (_eventStore.CurrentOffset >= 0)
        {
            _lastDisplayedOffset = _eventStore.CurrentOffset;
        }
    }

    /// <summary>
    /// Checks if an event is a journey-related event.
    /// </summary>
    private bool IsJourneyEvent(GameEvent evt)
    {
        return evt switch
        {
            ShipDepartedEvent => true,
            MechanicalFailureEvent => true,
            SocialConflictEvent => true,
            AnomalyDetectedEvent => true,
            ShipArrivedEvent => true,
            _ => false
        };
    }

    /// <summary>
    /// Checks if an event passes the current filter settings.
    /// </summary>
    private bool PassesFilters(GameEvent evt)
    {
        // Check system events filter
        bool showSystemEvents = _showSystemEvents?.ButtonPressed ?? true;
        if (!showSystemEvents && IsSystemEvent(evt))
        {
            return false;
        }

        // Check minor events filter
        bool showMinorEvents = _showMinorEvents?.ButtonPressed ?? true;
        if (!showMinorEvents && IsMinorEvent(evt))
        {
            return false;
        }

        return true;
    }

    /// <summary>
    /// Determines if an event is a system-level event.
    /// </summary>
    private bool IsSystemEvent(GameEvent evt)
    {
        return evt switch
        {
            // Add logic to identify system events
            // For now, treat anomalies as system events
            AnomalyDetectedEvent => true,
            _ => false
        };
    }

    /// <summary>
    /// Determines if an event is a minor event.
    /// </summary>
    private bool IsMinorEvent(GameEvent evt)
    {
        return evt switch
        {
            MechanicalFailureEvent failure => failure.Severity == "Minor",
            SocialConflictEvent conflict => conflict.MoraleImpact > -0.2f,
            _ => false
        };
    }

    /// <summary>
    /// Called when a player makes a choice in an event.
    /// </summary>
    private void OnEventChoiceSelected(GameEvent evt, int choiceIndex)
    {
        GD.Print($"ShipJourneyLogPresenter: Choice {choiceIndex} selected for event {evt.EventType} at offset {evt.Offset}");

        // TODO: Create appropriate command based on player choice
        // This will depend on your command structure
        // Example:
        // var command = CreateChoiceCommand(evt, choiceIndex);
        // if (command != null && _stateStore != null)
        // {
        //     _stateStore.ApplyCommand(command);
        // }
    }

    /// <summary>
    /// Called when the export button is pressed.
    /// </summary>
    private void OnExportPressed()
    {
        if (_eventStore == null)
        {
            ShowNotification("EventStore not available.");
            return;
        }

        // Get all journey events
        var journeyEvents = _eventStore.ReadFrom(0)
            .Where(IsJourneyEvent)
            .ToList();

        if (journeyEvents.Count == 0)
        {
            ShowNotification("No journey events to export.");
            return;
        }

        // Create confirmation dialog to choose format
        var formatDialog = new ConfirmationDialog
        {
            DialogText = $"Export {journeyEvents.Count} journey events as JSON or YAML?",
            Title = "Choose Export Format",
            OkButtonText = "JSON",
            CancelButtonText = "YAML"
        };

        // Connect signals
        formatDialog.Confirmed += () => ExportJourneyEvents(journeyEvents, "json");
        formatDialog.Canceled += () => ExportJourneyEvents(journeyEvents, "yaml");

        AddChild(formatDialog);
        formatDialog.PopupCentered();
    }

    /// <summary>
    /// Exports journey events to the specified format.
    /// </summary>
    private void ExportJourneyEvents(System.Collections.Generic.List<GameEvent> events, string format)
    {
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
            fileDialog.CurrentFile = EventExporter.GenerateExportFilename("journey_log", "json");
        }
        else if (format == "yaml")
        {
            fileDialog.Filters = new[] { "*.yaml ; YAML Files" };
            fileDialog.CurrentFile = EventExporter.GenerateExportFilename("journey_log", "yaml");
        }

        // Connect file selected signal
        fileDialog.FileSelected += (selectedPath) => OnJourneyFileSelected(selectedPath, format, events);

        // Add to scene tree and show
        AddChild(fileDialog);
        fileDialog.PopupCentered(new Vector2I(800, 600));
    }

    /// <summary>
    /// Called when a file is selected for journey export.
    /// </summary>
    private void OnJourneyFileSelected(string filePath, string format, System.Collections.Generic.List<GameEvent> events)
    {
        try
        {
            if (format == "json")
            {
                EventExporter.ExportToJson(events, filePath);
                ShowNotification($"Successfully exported {events.Count} journey events to JSON.");
            }
            else if (format == "yaml")
            {
                EventExporter.ExportToYaml(events, filePath);
                ShowNotification($"Successfully exported {events.Count} journey events to YAML.");
            }
        }
        catch (NotImplementedException ex)
        {
            ShowError($"Export failed: {ex.Message}");
        }
        catch (Exception ex)
        {
            ShowError($"Export failed: {ex.Message}");
            GD.PrintErr($"ShipJourneyLogPresenter: Export error - {ex}");
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
    /// Shows a temporary notification message.
    /// </summary>
    private void ShowNotification(string message)
    {
        // Simple notification using AcceptDialog
        var dialog = new AcceptDialog
        {
            DialogText = message,
            Title = "Info"
        };
        AddChild(dialog);
        dialog.PopupCentered();
    }

    /// <summary>
    /// Gets the EventStore instance from service locator.
    /// TODO: Replace with actual service locator pattern.
    /// </summary>
    private IEventStore GetEventStore()
    {
        // Placeholder - replace with actual service locator
        // Example: return ServiceLocator.Get<IEventStore>();
        
        // For now, try to find it in the scene tree
        var app = GetNodeOrNull("/root/App");
        if (app != null)
        {
            // Use reflection or a known method to get the service
            var result = app.Call("GetEventStore");
            if (result.VariantType != Variant.Type.Nil)
            {
                return result.AsGodotObject() as IEventStore;
            }
        }

        GD.PrintErr("ShipJourneyLogPresenter: Could not find EventStore. Implement service locator pattern.");
        return null;
    }

    /// <summary>
    /// Gets the StateStore instance from service locator.
    /// TODO: Replace with actual service locator pattern.
    /// </summary>
    private StateStore GetStateStore()
    {
        // Placeholder - replace with actual service locator
        // Example: return ServiceLocator.Get<StateStore>();
        
        // For now, try to find it in the scene tree
        var app = GetNodeOrNull("/root/App");
        if (app != null)
        {
            // StateStore is a Node, so we can find it directly
            var result = app.Call("GetStateStore");
            if (result.VariantType != Variant.Type.Nil)
            {
                return result.AsGodotObject() as StateStore;
            }
        }

        GD.PrintErr("ShipJourneyLogPresenter: Could not find StateStore. Implement service locator pattern.");
        return null;
    }
}
