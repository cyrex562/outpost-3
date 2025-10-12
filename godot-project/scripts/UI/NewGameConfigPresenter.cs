using Godot;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the New Game Configuration screen.
/// Will eventually contain player name, difficulty, and starting condition inputs.
/// </summary>
public partial class NewGameConfigPresenter : Control
{
    private Button _backButton = null!;
    private Button _nextButton = null!;

    public override void _Ready()
    {
        GD.Print("NewGameConfigPresenter _Ready() called");

        // Get button nodes
        _backButton = GetNode<Button>("MarginContainer/VBoxContainer/ButtonsContainer/BackButton");
        _nextButton = GetNode<Button>("MarginContainer/VBoxContainer/ButtonsContainer/NextButton");

        // Connect signals
        _backButton.Pressed += OnBackPressed;
        _nextButton.Pressed += OnNextPressed;
    }

    private void OnBackPressed()
    {
        GD.Print("Back button pressed - returning to Main Menu");
        GetTree().ChangeSceneToFile("res://Scenes/MainMenuScreen.tscn");
    }

    private void OnNextPressed()
    {
        GD.Print("Next button pressed - initializing new galaxy and going to Star System Selection");

        // Initialize a new galaxy via GameServices autoload
        var gameServices = GetNode<GameServices>("/root/GameServices");
        gameServices.InitializeNewGalaxy(seed: 42, starCount: 100);

        // Navigate to star map for system selection
        GetTree().ChangeSceneToFile("res://Scenes/UI/StarMapScreen.tscn");
    }
}
