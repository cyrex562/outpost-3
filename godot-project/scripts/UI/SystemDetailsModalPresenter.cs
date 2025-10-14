using System;
using Godot;
using Outpost3.Core.Domain;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the system details modal that displays detailed information about a selected star system.
/// </summary>
public partial class GalaxyMapSystemDetailsModalPresenter : PanelContainer
{
	private Label? _titleLabel;
	private Label? _systemNameLabel;
	private Label? _starTypeValueLabel;
	private Label? _luminosityValueLabel;
	private Label? _ageValueLabel;
	private Label? _massValueLabel;
	private VBoxContainer? _bodiesListContainer;
	private Button? _closeButton;
	private Button? _viewSystemMapButton;

	private PackedScene? _bodyListItemScene;

	public override void _Ready()
	{
		// Get node references
		_titleLabel = GetNode<Label>("MarginContainer/VBoxContainer/HeaderHBox/TitleLabel");
		if (_titleLabel == null)
		{
			throw new InvalidOperationException("TitleLabel node not found");
		}
		_systemNameLabel = GetNode<Label>("MarginContainer/VBoxContainer/SystemNameLabel");
		if (_systemNameLabel == null)
		{
			throw new InvalidOperationException("SystemNameLabel node not found");
		}
		_starTypeValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/StarTypeValueLabel");
		if (_starTypeValueLabel == null)
		{
			throw new InvalidOperationException("StarTypeValueLabel node not found");
		}
		_luminosityValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/LuminosityValueLabel");
		if (_luminosityValueLabel == null)
		{
			throw new InvalidOperationException("LuminosityValueLabel node not found");
		}
		_ageValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/AgeValueLabel");
		if (_ageValueLabel == null)
		{
			throw new InvalidOperationException("AgeValueLabel node not found");
		}
		_massValueLabel = GetNode<Label>("MarginContainer/VBoxContainer/PropertiesGrid/MassValueLabel");
		if (_massValueLabel == null)
		{
			throw new InvalidOperationException("MassValueLabel node not found");
		}
		_bodiesListContainer = GetNode<VBoxContainer>("MarginContainer/VBoxContainer/BodiesScrollContainer/BodiesListContainer");
		if (_bodiesListContainer == null)
		{
			throw new InvalidOperationException("BodiesListContainer node not found");
		}
		_closeButton = GetNode<Button>("MarginContainer/VBoxContainer/HeaderHBox/CloseButton");
		if (_closeButton == null)
		{
			throw new InvalidOperationException("CloseButton node not found");
		}
		_viewSystemMapButton = GetNode<Button>("MarginContainer/VBoxContainer/ViewSystemMapButton");
		if (_viewSystemMapButton == null)
		{
			throw new InvalidOperationException("ViewSystemMapButton node not found");
		}

		// Load the BodyListItem scene
		_bodyListItemScene = GD.Load<PackedScene>("res://Scenes/UI/BodyListItem.tscn");
		if (_bodyListItemScene == null)
		{
			throw new InvalidOperationException("Failed to load BodyListItem scene");
		}

		// Wire up signals
		_closeButton.Pressed += OnCloseButtonPressed;
		_viewSystemMapButton.Pressed += OnViewSystemMapPressed;

		// Start hidden
		Hide();
	}

	/// <summary>
	/// Display detailed information about a star system.
	/// </summary>
	public void ShowSystem(StarSystem system)
	{
		GD.Print($"ShowSystem called for: {system.Name}");

		if (_systemNameLabel == null || _starTypeValueLabel == null ||
			_luminosityValueLabel == null || _ageValueLabel == null || _massValueLabel == null)
		{
			GD.PrintErr("UI controls not properly initialized");
			return;
		}

		_systemNameLabel.Text = system.Name;
		_starTypeValueLabel.Text = system.SpectralClass;

		// Enable the View System Map button
		if (_viewSystemMapButton != null)
		{
			_viewSystemMapButton.Disabled = false;
			_viewSystemMapButton.Text = "View System Map";
		}

		// Clear existing body items
		if (_bodiesListContainer != null)
		{
			foreach (Node child in _bodiesListContainer.GetChildren())
			{
				child.QueueFree();
			}

			// Instantiate body list items
			if (_bodyListItemScene != null)
			{
				foreach (var body in system.Bodies)
				{
					var bodyItem = _bodyListItemScene.Instantiate<BodyListItemComponent>();
					_bodiesListContainer.AddChild(bodyItem);
					bodyItem.SetBodyData(body);
				}
			}
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

	private void OnViewSystemMapPressed()
	{
		GD.Print("View System Map button pressed");

		// Hide the modal first
		HideModal();

		// Navigate to the star system map scene
		// Note: The selected system is already set in the state, so the map will load it
		GetTree().Root.GetNode<Outpost3.GameServices>("/root/GameServices").StateStore.ApplyCommand(
			new Outpost3.Core.Commands.PushScreen(Outpost3.Core.Domain.ScreenId.StarSystemMap)
		);

		GetTree().ChangeSceneToFile("res://Scenes/UI/StarSystemMapScreen.tscn");
	}
}
