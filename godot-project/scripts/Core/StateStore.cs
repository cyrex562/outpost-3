using Godot;
using Outpost3.Core.Domain;
using Outpost3.Core.Commands;
using Outpost3.Core.Systems;
using System.Windows.Input;

namespace Outpost3.Core;

public partial class StateStore : Node
{
    private GameState _state = GameState.NewGame();

    [Signal]
    public delegate void StateChangedEventHandler();

    public GameState State => _state;

    public override void _Ready()
    {
        GD.Print("StateStore initialized");
    }

    public void ApplyCommand(Core.Commands.ICommand command)
    {
        GD.Print($"Applying command: {command.GetType().Name}");

        var (newState, events) = TimeSystem.Reduce(_state, command);

        _state = newState;

        foreach (var evt in events)
        {
            GD.Print($"Event: {evt.GetType().Name}");
        }

        EmitSignal(SignalName.StateChanged);
    }
}