using Godot;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the Game Credits screen.
/// Displays developer information and attributions.
/// </summary>
public partial class GameCreditsPresenter : Control
{
    private Button _backButton = null!;

    public override void _Ready()
    {
        GD.Print("GameCreditsPresenter _Ready() called");

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
