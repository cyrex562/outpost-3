using Godot;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the Mod Management screen.
/// Will eventually contain mod detection, enable/disable, load order, and compatibility checking.
/// </summary>
public partial class ModManagementPresenter : Control
{
    private Button _backButton = null!;

    public override void _Ready()
    {
        GD.Print("ModManagementPresenter _Ready() called");

        // Get button node
        _backButton = GetNode<Button>("MarginContainer/VBoxContainer/BackButton");

        // Connect signal
        _backButton.Pressed += OnBackPressed;
    }

    private void OnBackPressed()
    {
        GD.Print("Back button pressed - returning to Main Menu");
        GetTree().ChangeSceneToFile("res://Scenes/MainMenuScreen.tscn");
    }
}
