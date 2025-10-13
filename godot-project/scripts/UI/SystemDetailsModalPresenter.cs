using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the system details modal that displays detailed information about a selected star system.
/// </summary>
public partial class SystemDetailsModalPresenter : PanelContainer
{
    private Label _titleLabel;
    private Label _systemNameLabel;
    private Label _starTypeValueLabel;
    private Label _luminosityValueLabel;
    private Label _ageValueLabel;
    private Label _massValueLabel;
    private VBoxContainer _bodiesListContainer;
    private Button _closeButton;
    private Button _viewSystemMapButton;

    private PackedScene _bodyListItemScene;

    public override void _Ready()
    {
        // Get node references
        _titleLabel = GetNode<Label>("MarginContainer/VBoxContainer/HeaderHBox/TitleLabel");
        _systemNameLabel = GetNode<Label>("MarginContainer/VBoxContainer/SystemNameLabel");
        _starTypeValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/StarTypeValueLabel");
        _luminosityValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/LuminosityValueLabel");
        _ageValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/AgeValueLabel");
        _massValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/MassValueLabel");
        _bodiesListContainer = GetNode<VBoxContainer>("MarginContainer/VBoxContainer/BodiesScrollContainer/BodiesListContainer");
        _closeButton = GetNode<Button>("MarginContainer/VBoxContainer/HeaderHBox/CloseButton");
        _viewSystemMapButton = GetNode<Button>("MarginContainer/VBoxContainer/ViewSystemMapButton");

        // Load the BodyListItem scene
        _bodyListItemScene = GD.Load<PackedScene>("res://Scenes/UI/BodyListItem.tscn");

        // Wire up signals
        _closeButton.Pressed += OnCloseButtonPressed;

        // Start hidden
        Hide();
    }

    /// <summary>
    /// Display detailed information about a star system.
    /// </summary>
    public void ShowSystem(StarSystem system)
    {
        GD.Print($"ShowSystem called for: {system.Name}");
        
        _systemNameLabel.Text = system.Name;
        _starTypeValueLabel.Text = system.SpectralClass;

        // These properties don't exist yet in the domain model
        // TODO: Add when StarSystem is enhanced with full stellar properties
        _luminosityValueLabel.Text = $"{system.Luminosity:F2} L☉";
        _ageValueLabel.Text = "Unknown";
        _massValueLabel.Text = "Unknown";

        // Clear existing body items
        foreach (Node child in _bodiesListContainer.GetChildren())
        {
            child.QueueFree();
        }

        // Instantiate body list items
        foreach (var body in system.Bodies)
        {
            var bodyItem = _bodyListItemScene.Instantiate<BodyListItemComponent>();
            _bodiesListContainer.AddChild(bodyItem);
            bodyItem.SetBodyData(body);
        }

        // Make sure the modal is visible
        Visible = true;
        Show();
        
        GD.Print($"ShowSystem: Modal visibility is now {Visible}");
    }

    /// <summary>
    /// Hide the modal.
    /// </summary>
    public void HideModal()
    {
        GD.Print("HideModal called");
        Hide();
        GD.Print($"HideModal: Modal visibility is now {Visible}");
    }

    private void OnCloseButtonPressed()
    {
        GD.Print("Close button pressed");
        HideModal();
    }
}
