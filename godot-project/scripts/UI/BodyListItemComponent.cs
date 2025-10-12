using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// UI component for displaying a single celestial body in the system details modal.
/// </summary>
public partial class BodyListItemComponent : PanelContainer
{
    private Label _bodyNameLabel;
    private Label _bodyTypeLabel;
    private Label _massValue;

    public override void _Ready()
    {
        // Get node references
        _bodyNameLabel = GetNode<Label>("MarginContainer/HBoxContainer/BodyNameLabel");
        _bodyTypeLabel = GetNode<Label>("MarginContainer/HBoxContainer/BodyTypeLabel");
        _massValue = GetNode<Label>("MarginContainer/HBoxContainer/MassValue");
    }

    /// <summary>
    /// Update the display with body data.
    /// </summary>
    public void SetBodyData(CelestialBody body)
    {
        _bodyNameLabel.Text = body.Name;
        _bodyTypeLabel.Text = body.BodyType;
        _massValue.Text = body.Explored ? "Explored" : "Unknown";
    }
}
