using Godot;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the Load Game screen.
/// Will eventually contain save file management, loading, and deletion.
/// </summary>
public partial class LoadGamePresenter : Control
{
    private Button _backButton = null!;

    public override void _Ready()
    {
        GD.Print("LoadGamePresenter _Ready() called");

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
