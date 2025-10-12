using Godot;
using System;
using Outpost3.Core.Events;
using Outpost3.Core.Persistence;
using Outpost3.Core;

namespace Outpost3.UI;

/// <summary>
/// Debug panel for displaying real-time event stream.
/// Press F3 to toggle visibility.
/// </summary>
public partial class DebugEventPanel : CanvasLayer
{
    // Exported node references
    private VBoxContainer _eventList = null!;
    private Button _pinButton = null!;
    private Button _clearButton = null!;
    private Button _closeButton = null!;
    private ScrollContainer _scrollContainer = null!;

    // Constants
    private const int MAX_EVENTS = 50;

    // Private fields
    private bool _isPinned = false;
    private IEventStore? _eventStore;
    private StateStore? _stateStore;
    private long _lastDisplayedOffset = -1;

    public override void _Ready()
    {
        // Get node references
        _eventList = GetNode<VBoxContainer>("PanelContainer/VBoxContainer/EventScrollContainer/DebugEventList");
        _pinButton = GetNode<Button>("PanelContainer/VBoxContainer/Header/PinButton");
        _clearButton = GetNode<Button>("PanelContainer/VBoxContainer/Header/ClearButton");
        _closeButton = GetNode<Button>("PanelContainer/VBoxContainer/Header/CloseButton");
        _scrollContainer = GetNode<ScrollContainer>("PanelContainer/VBoxContainer/EventScrollContainer");

        // Get services
        _eventStore = GetEventStore();
        _stateStore = GetStateStore();

        // Connect button signals
        _pinButton.Pressed += TogglePin;
        _clearButton.Pressed += ClearEvents;
        _closeButton.Pressed += () => Visible = false;

        // Subscribe to state changes for new events
        if (_stateStore != null)
        {
            _stateStore.StateChanged += OnNewEvents;
        }

        // Start hidden
        Visible = false;
    }

    public override void _ExitTree()
    {
        // Disconnect signals
        if (_stateStore != null)
        {
            _stateStore.StateChanged -= OnNewEvents;
        }
    }

    public override void _Input(InputEvent evt)
    {
        // Check for F3 key press
        if (evt is InputEventKey keyEvent && keyEvent.Pressed && !keyEvent.Echo)
        {
            if (keyEvent.Keycode == Key.F3)
            {
                Visible = !Visible;
                if (Visible)
                {
                    RefreshEvents();
                }
                GetViewport().SetInputAsHandled();
            }
        }
    }

    /// <summary>
    /// Called when StateStore emits StateChanged signal.
    /// </summary>
    private void OnNewEvents()
    {
        // Only refresh if visible
        if (!Visible) return;

        RefreshEvents();
    }

    /// <summary>
    /// Refreshes the event list from the EventStore.
    /// </summary>
    private void RefreshEvents()
    {
        if (_eventStore == null || _eventList == null) return;

        // Get new events from EventStore
        var newEvents = _eventStore.ReadFrom(_lastDisplayedOffset + 1);

        foreach (var evt in newEvents)
        {
            // Create label for event
            var label = new Label
            {
                Text = FormatEventForDebug(evt),
                AutowrapMode = TextServer.AutowrapMode.WordSmart
            };

            // Override font color based on event type
            label.AddThemeColorOverride("font_color", GetEventColor(evt));

            // Add to list
            _eventList.AddChild(label);
        }

        // Trim oldest events if exceeds max
        while (_eventList.GetChildCount() > MAX_EVENTS)
        {
            var firstChild = _eventList.GetChild(0);
            _eventList.RemoveChild(firstChild);
            firstChild.QueueFree();
        }

        // Update last displayed offset
        if (_eventStore.CurrentOffset >= 0)
        {
            _lastDisplayedOffset = _eventStore.CurrentOffset;
        }

        // Scroll to bottom
        CallDeferred(MethodName.ScrollToBottom);
    }

    /// <summary>
    /// Scrolls the event list to the bottom.
    /// </summary>
    private void ScrollToBottom()
    {
        if (_scrollContainer == null) return;

        var vScrollBar = _scrollContainer.GetVScrollBar();
        if (vScrollBar != null)
        {
            _scrollContainer.ScrollVertical = (int)vScrollBar.MaxValue;
        }
    }

    /// <summary>
    /// Formats an event for debug display.
    /// </summary>
    private string FormatEventForDebug(GameEvent evt)
    {
        return $"[{evt.Offset}] {evt.GameTime:F1}h | {evt.EventType}";
    }

    /// <summary>
    /// Gets the color for an event based on its type.
    /// </summary>
    private Color GetEventColor(GameEvent evt)
    {
        return evt switch
        {
            MechanicalFailureEvent => Colors.Orange,
            SocialConflictEvent => Colors.Red,
            AnomalyDetectedEvent => Colors.Cyan,
            ShipDepartedEvent => Colors.Green,
            ShipArrivedEvent => Colors.Green,
            _ => Colors.LightGray
        };
    }

    /// <summary>
    /// Toggles the pin state of the panel.
    /// </summary>
    private void TogglePin()
    {
        _isPinned = !_isPinned;
        _pinButton.Text = _isPinned ? "📍" : "📌";

        // TODO: Implement persistence across scene changes
        // This would require integration with your scene management system
        if (_isPinned)
        {
            GD.Print("DebugEventPanel: Pinned mode enabled (persistence not yet implemented)");
        }
    }

    /// <summary>
    /// Clears all events from the display.
    /// </summary>
    private void ClearEvents()
    {
        if (_eventList == null) return;

        // Remove all children
        foreach (Node child in _eventList.GetChildren())
        {
            _eventList.RemoveChild(child);
            child.QueueFree();
        }

        // Update offset to current
        if (_eventStore != null)
        {
            _lastDisplayedOffset = _eventStore.CurrentOffset;
        }
    }

    /// <summary>
    /// Gets the EventStore instance from GameServices autoload.
    /// </summary>
    private IEventStore? GetEventStore()
    {
        var gameServices = GetNodeOrNull<GameServices>("/root/GameServices");
        if (gameServices != null)
        {
            return gameServices.EventStore;
        }

        GD.PrintErr("DebugEventPanel: Could not find GameServices autoload!");
        return null;
    }

    /// <summary>
    /// Gets the StateStore instance from GameServices autoload.
    /// </summary>
    private StateStore? GetStateStore()
    {
        var gameServices = GetNodeOrNull<GameServices>("/root/GameServices");
        if (gameServices != null)
        {
            return gameServices.StateStore;
        }

        GD.PrintErr("DebugEventPanel: Could not find GameServices autoload!");
        return null;
    }
}
