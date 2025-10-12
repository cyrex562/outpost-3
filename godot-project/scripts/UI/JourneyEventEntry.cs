using Godot;
using System;
using System.Collections.Generic;
using Outpost3.Core.Events;

namespace Outpost3.UI;

/// <summary>
/// Reusable component for displaying individual journey events.
/// Dynamically populates event details and handles player choices.
/// </summary>
public partial class JourneyEventEntry : PanelContainer
{
    [Signal]
    public delegate void ChoiceSelectedEventHandler(int choiceIndex);

    // Node references
    private Label _timeLabel;
    private Label _gameTimeLabel;
    private TextureRect _eventIcon;
    private Label _titleLabel;
    private Label _descriptionLabel;
    private Label _severityLabel;
    private VBoxContainer _choicesContainer;

    // Private fields
    private GameEvent _event;

    public override void _Ready()
    {
        // Get node references
        _timeLabel = GetNode<Label>("MarginContainer/HBoxContainer/TimestampSection/TimeLabel");
        _gameTimeLabel = GetNode<Label>("MarginContainer/HBoxContainer/TimestampSection/GameTimeLabel");
        _eventIcon = GetNode<TextureRect>("MarginContainer/HBoxContainer/EventIcon");
        _titleLabel = GetNode<Label>("MarginContainer/HBoxContainer/ContentSection/TitleLabel");
        _descriptionLabel = GetNode<Label>("MarginContainer/HBoxContainer/ContentSection/DescriptionLabel");
        _severityLabel = GetNode<Label>("MarginContainer/HBoxContainer/MetadataSection/SeverityLabel");
        _choicesContainer = GetNode<VBoxContainer>("MarginContainer/HBoxContainer/ContentSection/ChoicesContainer");
    }

    /// <summary>
    /// Sets the event to display in this entry.
    /// </summary>
    /// <param name="evt">The game event to display.</param>
    public void SetEvent(GameEvent evt)
    {
        if (evt == null)
        {
            GD.PrintErr("JourneyEventEntry.SetEvent: event is null");
            return;
        }

        _event = evt;

        // Format time display
        var dayNumber = evt.GameTime / 24.0f;
        _timeLabel.Text = $"Day {dayNumber:F0}";
        _gameTimeLabel.Text = evt.RealTime.ToString("HH:mm");

        // Set icon based on event type
        GetIconForEventType(evt);

        // Set title
        _titleLabel.Text = GetEventTitle(evt);

        // Set description
        _descriptionLabel.Text = GetEventDescription(evt);

        // Set severity label and color
        _severityLabel.Text = GetSeverityLabel(evt);
        var severityColor = GetSeverityColor(evt);
        _severityLabel.AddThemeColorOverride("font_color", severityColor);

        // TODO: Handle player choices if needed
        // ShowChoices(choices);
    }

    /// <summary>
    /// Gets the appropriate title for an event.
    /// </summary>
    private string GetEventTitle(GameEvent evt)
    {
        return evt switch
        {
            ShipDepartedEvent => "Journey Begins",
            MechanicalFailureEvent failure => $"Mechanical Failure: {failure.SystemAffected}",
            SocialConflictEvent => "Social Incident",
            AnomalyDetectedEvent => "Anomaly Detected",
            ShipArrivedEvent => "Arrival at Destination",
            _ => evt.EventType
        };
    }

    /// <summary>
    /// Gets the appropriate description for an event.
    /// </summary>
    private string GetEventDescription(GameEvent evt)
    {
        return evt switch
        {
            ShipDepartedEvent departure => 
                $"The {departure.ShipName} has departed with {departure.ColonistCount} colonists aboard.",
            MechanicalFailureEvent failure => failure.Description,
            SocialConflictEvent conflict => conflict.Description,
            AnomalyDetectedEvent anomaly => anomaly.Description,
            ShipArrivedEvent arrival => 
                $"Successfully arrived at {arrival.SystemName} after {arrival.TravelDuration:F1} days of travel.",
            _ => "Event occurred."
        };
    }

    /// <summary>
    /// Gets the severity label for an event.
    /// </summary>
    private string GetSeverityLabel(GameEvent evt)
    {
        return evt switch
        {
            MechanicalFailureEvent failure => failure.Severity switch
            {
                "Critical" => "🔴 Critical",
                "Major" => "🟠 Major",
                "Minor" => "⚠️ Minor",
                _ => "ℹ️ Info"
            },
            SocialConflictEvent conflict => conflict.MoraleImpact switch
            {
                < -0.5f => "🔴 Severe",
                < -0.2f => "🟠 Moderate",
                < 0 => "⚠️ Minor",
                _ => "ℹ️ Info"
            },
            ShipDepartedEvent => "✅ Mission Start",
            ShipArrivedEvent => "✅ Arrived",
            AnomalyDetectedEvent => "🔵 Discovery",
            _ => "ℹ️ Info"
        };
    }

    /// <summary>
    /// Gets the color for an event based on its severity.
    /// </summary>
    private Color GetSeverityColor(GameEvent evt)
    {
        return evt switch
        {
            MechanicalFailureEvent failure => failure.Severity switch
            {
                "Critical" => Colors.Red,
                "Major" => Colors.Orange,
                "Minor" => Colors.Yellow,
                _ => Colors.Cyan
            },
            SocialConflictEvent conflict => conflict.MoraleImpact switch
            {
                < -0.5f => Colors.Red,
                < -0.2f => Colors.Orange,
                < 0 => Colors.Yellow,
                _ => Colors.Cyan
            },
            ShipDepartedEvent => Colors.Green,
            ShipArrivedEvent => Colors.Green,
            AnomalyDetectedEvent => Colors.Cyan,
            _ => Colors.White
        };
    }

    /// <summary>
    /// Sets the icon for an event type.
    /// Currently uses placeholder logic - icons should be loaded from resources.
    /// </summary>
    private void GetIconForEventType(GameEvent evt)
    {
        // TODO: Load actual icons from resources
        // For now, the TextureRect will remain empty
        // Future implementation:
        // var iconPath = evt switch
        // {
        //     ShipDepartedEvent => "res://assets/icons/ship_departure.png",
        //     MechanicalFailureEvent => "res://assets/icons/mechanical_failure.png",
        //     SocialConflictEvent => "res://assets/icons/social_conflict.png",
        //     AnomalyDetectedEvent => "res://assets/icons/anomaly.png",
        //     ShipArrivedEvent => "res://assets/icons/ship_arrival.png",
        //     _ => "res://assets/icons/default_event.png"
        // };
        // _eventIcon.Texture = GD.Load<Texture2D>(iconPath);
    }

    /// <summary>
    /// Shows player choice buttons for events that require decisions.
    /// </summary>
    /// <param name="choices">List of choice descriptions.</param>
    public void ShowChoices(List<string> choices)
    {
        if (choices == null || choices.Count == 0)
        {
            _choicesContainer.Visible = false;
            return;
        }

        // Clear existing choice buttons
        foreach (Node child in _choicesContainer.GetChildren())
        {
            child.QueueFree();
        }

        // Create a button for each choice
        for (int i = 0; i < choices.Count; i++)
        {
            var button = new Button
            {
                Text = choices[i]
            };

            // Capture the index for the lambda
            int choiceIndex = i;
            button.Pressed += () => OnChoiceButtonPressed(choiceIndex);

            _choicesContainer.AddChild(button);
        }

        _choicesContainer.Visible = true;
    }

    /// <summary>
    /// Called when a choice button is pressed.
    /// </summary>
    private void OnChoiceButtonPressed(int choiceIndex)
    {
        EmitSignal(SignalName.ChoiceSelected, choiceIndex);
    }
}
