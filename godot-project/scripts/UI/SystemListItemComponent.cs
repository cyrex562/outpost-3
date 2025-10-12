using System;
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// UI component for a single star system in the systems list.
/// </summary>
public partial class SystemListItemComponent : PanelContainer
{
    private ColorRect _starColorIndicator;
    private Label _systemNameLabel;
    private Label _distanceValue;
    private Label _bodiesValue;

    [Signal]
    public delegate void SystemClickedEventHandler(string systemId);

    private Ulid _systemId;
    private bool _isSelected;

    public Ulid SystemId => _systemId;

    public override void _Ready()
    {
        // Get node references
        _starColorIndicator = GetNode<ColorRect>("MarginContainer/VBoxContainer/HeaderHBox/StarColorIndicator");
        _systemNameLabel = GetNode<Label>("MarginContainer/VBoxContainer/HeaderHBox/SystemNameLabel");
        _distanceValue = GetNode<Label>("MarginContainer/VBoxContainer/DistanceHBox/DistanceValue");
        _bodiesValue = GetNode<Label>("MarginContainer/VBoxContainer/BodiesHBox/BodiesValue");

        GuiInput += OnGuiInput;
    }

    /// <summary>
    /// Sets the star system data to display.
    /// </summary>
    /// <param name="system">The star system to display.</param>
    public void SetSystemData(StarSystem system)
    {
        _systemId = system.Id;

        _systemNameLabel.Text = system.Name;
        _distanceValue.Text = system.SpectralClass;
        _bodiesValue.Text = system.Bodies.Count.ToString();

        // Set star color based on spectral class
        _starColorIndicator.Color = GetStarColorFromSpectralClass(system.SpectralClass);
    }

    /// <summary>
    /// Sets the visual selection state of this item.
    /// </summary>
    /// <param name="selected">True if this item should appear selected.</param>
    public void SetSelected(bool selected)
    {
        _isSelected = selected;

        // Visual feedback
        var styleBox = new StyleBoxFlat();
        styleBox.BgColor = selected
            ? new Color(0.3f, 0.5f, 0.7f, 0.4f)
            : new Color(0.15f, 0.15f, 0.15f, 0.3f);
        styleBox.BorderColor = selected
            ? new Color(0.5f, 0.7f, 1.0f)
            : new Color(0.3f, 0.3f, 0.3f);
        var borderWidth = selected ? 2 : 1;
        styleBox.BorderWidthLeft = borderWidth;
        styleBox.BorderWidthRight = borderWidth;
        styleBox.BorderWidthTop = borderWidth;
        styleBox.BorderWidthBottom = borderWidth;
        AddThemeStyleboxOverride("panel", styleBox);
    }

    private void OnGuiInput(InputEvent @event)
    {
        if (@event is InputEventMouseButton mouseEvent
            && mouseEvent.Pressed
            && mouseEvent.ButtonIndex == MouseButton.Left)
        {
            EmitSignal(SignalName.SystemClicked, _systemId.ToString());
        }
    }

    /// <summary>
    /// Maps spectral class to a representative color.
    /// Spectral classes: O (blue), B (blue-white), A (white), F (yellow-white), G (yellow), K (orange), M (red)
    /// </summary>
    private Color GetStarColorFromSpectralClass(string spectralClass)
    {
        if (string.IsNullOrEmpty(spectralClass))
        {
            return new Color(0.8f, 0.8f, 0.8f); // Default gray
        }

        // Take first character for classification
        var classChar = spectralClass.ToUpper()[0];

        return classChar switch
        {
            'O' => new Color(0.4f, 0.6f, 1.0f),    // Blue
            'B' => new Color(0.6f, 0.7f, 1.0f),    // Blue-white
            'A' => new Color(0.9f, 0.9f, 1.0f),    // White
            'F' => new Color(1.0f, 0.95f, 0.7f),   // Yellow-white
            'G' => new Color(1.0f, 0.95f, 0.4f),   // Yellow (like our Sun)
            'K' => new Color(1.0f, 0.7f, 0.3f),    // Orange
            'M' => new Color(1.0f, 0.4f, 0.2f),    // Red
            _ => new Color(0.8f, 0.8f, 0.8f)       // Unknown
        };
    }
}
