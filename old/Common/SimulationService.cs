namespace HarshRealm.Services;

public sealed class SimulationService
{
    public ulong CurrentTurn { get; private set; }

    public void AdvanceTurn()
    {
        CurrentTurn += 1;
    }
}
