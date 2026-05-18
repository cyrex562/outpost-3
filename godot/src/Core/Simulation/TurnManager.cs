namespace OutpostGame.Core.Simulation;

public sealed class WaitDirective
{
    public int WaitUntilSol { get; init; }
    public bool InterruptOnEvent { get; init; } = true;
}

public sealed class TurnManager
{
    public int CurrentSol { get; private set; }
    public int StrategicTurnsPerColonyTurn { get; set; } = 30;
    public WaitDirective? ActiveWait { get; private set; }

    public event Action<int>? TurnAdvanced;         // sol number
    public event Action<int>? StrategicTurnAdvanced; // month number
    public event Action? WaitInterrupted;

    public void Advance(int turns = 1)
    {
        for (int i = 0; i < turns; i++)
        {
            CurrentSol++;
            TurnAdvanced?.Invoke(CurrentSol);

            if (CurrentSol % StrategicTurnsPerColonyTurn == 0)
                StrategicTurnAdvanced?.Invoke(CurrentSol / StrategicTurnsPerColonyTurn);
        }
    }

    public void BeginWait(WaitDirective directive) => ActiveWait = directive;

    public void InterruptWait()
    {
        ActiveWait = null;
        WaitInterrupted?.Invoke();
    }

    public bool ShouldContinueWait() =>
        ActiveWait != null && CurrentSol < ActiveWait.WaitUntilSol;

    /// <summary>Sets the sol counter without firing any events — for save/load restore.</summary>
    public void RestoreSol(int sol) => CurrentSol = sol;
}
