using Godot;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the Game Settings screen.
/// Will eventually contain audio, graphics, controls, gameplay, and accessibility settings.
/// </summary>
public partial class GameSettingsPresenter : Control
{
    private Button _backButton = null!;

    public override void _Ready()
    {
        GD.Print("GameSettingsPresenter _Ready() called");

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
