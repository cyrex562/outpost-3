using Godot;

namespace Outpost3.UI;

/// <summary>
/// Presenter for the Main Menu screen.
/// Handles navigation to different game sections.
/// </summary>
public partial class MainMenuPresenter : Control
{
    private Button _newGameButton = null!;
    private Button _loadGameButton = null!;
    private Button _settingsButton = null!;
    private Button _modsButton = null!;
    private Button _creditsButton = null!;
    private Button _exitButton = null!;

    public override void _Ready()
    {
        GD.Print("MainMenuPresenter _Ready() called");

        // Get button nodes
        _newGameButton = GetNode<Button>("CenterContainer/VBoxContainer/ButtonsContainer/NewGameButton");
        _loadGameButton = GetNode<Button>("CenterContainer/VBoxContainer/ButtonsContainer/LoadGameButton");
        _settingsButton = GetNode<Button>("CenterContainer/VBoxContainer/ButtonsContainer/SettingsButton");
        _modsButton = GetNode<Button>("CenterContainer/VBoxContainer/ButtonsContainer/ModsButton");
        _creditsButton = GetNode<Button>("CenterContainer/VBoxContainer/ButtonsContainer/CreditsButton");
        _exitButton = GetNode<Button>("CenterContainer/VBoxContainer/ButtonsContainer/ExitButton");

        // Connect signals
        _newGameButton.Pressed += OnNewGamePressed;
        _loadGameButton.Pressed += OnLoadGamePressed;
        _settingsButton.Pressed += OnSettingsPressed;
        _modsButton.Pressed += OnModsPressed;
        _creditsButton.Pressed += OnCreditsPressed;
        _exitButton.Pressed += OnExitPressed;
    }

    private void OnNewGamePressed()
    {
        GD.Print("New Game button pressed");
        GetTree().ChangeSceneToFile("res://Scenes/NewGameConfigScreen.tscn");
    }

    private void OnLoadGamePressed()
    {
        GD.Print("Load Game button pressed");
        GetTree().ChangeSceneToFile("res://Scenes/LoadGameScreen.tscn");
    }

    private void OnSettingsPressed()
    {
        GD.Print("Settings button pressed");
        GetTree().ChangeSceneToFile("res://Scenes/GameSettingsScreen.tscn");
    }

    private void OnModsPressed()
    {
        GD.Print("Mods button pressed");
        GetTree().ChangeSceneToFile("res://Scenes/ModManagementScreen.tscn");
    }

    private void OnCreditsPressed()
    {
        GD.Print("Credits button pressed");
        GetTree().ChangeSceneToFile("res://Scenes/GameCreditsScreen.tscn");
    }

    private void OnExitPressed()
    {
        GD.Print("Exit button pressed");
        GetTree().Quit();
    }
}
