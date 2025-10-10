using Godot;
using Outpost3.Core;
using System.Reflection.Emit;
using System.Text.Json;

public partial class MainUI : Control
{
    private Godot.Label _timeLabel;
    private Godot.Button _advanceButton;
    private StateStore _stateStore;

    public override void _Ready()
    {
        _stateStore = GetNode<StateStore>("/root/Main/StateStore");
        _timeLabel = GetNode<Godot.Label>("Panel/VBox/TimeLabel");
        _advanceButton = GetNode<Godot.Button>("Panel/VBox/AdvanceButton");

        _advanceButton.Pressed += OnAdvancePressed;
        _stateStore.StateChanged += OnStateChanged;

        UpdateUI();
    }

    private void OnAdvancePressed()
    {
        _stateStore.AdvanceTime(10.0);
    }

    private void OnStateChanged()
    {
        UpdateUI();
    }

    private void UpdateUI()
    {
        string stateJson = _stateStore.GetStateJson();
        var state = JsonSerializer.Deserialize<GameStateDto>(stateJson);
        _timeLabel.Text = $"Game Time: {state.game_time:F1}h";

    }

    private class GameStateDto
    {
        public double game_time { get; set; }
    }
}