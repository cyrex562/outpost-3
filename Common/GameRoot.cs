using Godot;
using HarshRealm.Services;

namespace HarshRealm;

public partial class GameRoot : Node
{
    [Export]
    public double TurnIntervalSeconds { get; set; } = 1.5;

    private double _elapsed;
    private GameStateService? _gameState;

    public override void _Ready()
    {
        _gameState = new GameStateService();

        var loadSuccess = _gameState.TryLoadSolarSystem("res://data/solar_system_data.csv");
        if (!loadSuccess)
        {
            GD.PrintErr("Failed to load solar system data. Check the CSV at res://data/solar_system_data.csv");
            return;
        }

        GD.Print($"Game initialized. Start date: {_gameState.FormattedDate}");
    }

    public override void _Process(double delta)
    {
        if (_gameState == null)
        {
            return;
        }

        _elapsed += delta;
        if (_elapsed < TurnIntervalSeconds)
        {
            return;
        }

        _elapsed = 0;
        var updates = _gameState.ProcessTurn();
        GD.Print($"Turn {_gameState.Simulation.CurrentTurn} | Date {_gameState.FormattedDate}");

        foreach (var change in updates)
        {
            GD.Print($"  {change.BodyName}: angle={change.AngleDegrees:F2}°, distance={change.DistanceKm:F0} km");
        }
    }
}
