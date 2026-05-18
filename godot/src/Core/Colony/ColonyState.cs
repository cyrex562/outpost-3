namespace OutpostGame.Core.Colony;

using OutpostGame.Core.Simulation;

public sealed class ColonyState
{
    public Guid Id { get; } = Guid.NewGuid();
    public string Name { get; set; } = "New Colony";
    public ColonyGrid Grid { get; }
    public ResourceStore Resources { get; } = new();
    public PopulationGroup Population { get; } = new();
    public LaborPool Labor { get; } = new();
    public PowerGrid Power { get; } = new();
    public ConstructionFleet Fleet { get; } = new();
    public OperatorPool Operators { get; } = new();
    public ColonyEventLog EventLog { get; } = new();
    public TurnManager TurnManager { get; } = new();

    /// <summary>Sols remaining in the current dust storm (0 = no storm).</summary>
    public int DustStormTurnsRemaining { get; set; }

    public ColonyStatus Status { get; set; } = ColonyStatus.Active;

    /// <summary>Consecutive turns morale has been below the critical threshold.</summary>
    public int LowMoraleTurns { get; set; }

    /// <summary>Consecutive turns the thriving conditions have been met.</summary>
    public int ThrivingTurns { get; set; }

    /// <summary>
    /// Signed change in resource amounts during the most recent <see
    /// cref="ColonyTurnProcessor.ProcessTurn"/>. Empty until the first turn runs.
    /// Drives the HUD's per-resource trend arrows.
    /// </summary>
    public IReadOnlyDictionary<string, float> ResourceDeltas { get; set; }
        = new Dictionary<string, float>();

    /// <summary>
    /// FIFO queue of player-choice events awaiting a decision. The HUD shows a
    /// modal for <see cref="CurrentDecision"/>; resolution is via
    /// <see cref="OutpostGame.Core.Simulation.ColonySession.AnswerDecision"/>.
    /// </summary>
    public Queue<PendingDecision> PendingDecisions { get; } = new();

    public PendingDecision? CurrentDecision =>
        PendingDecisions.Count > 0 ? PendingDecisions.Peek() : null;

    public ColonyState(int gridWidth = 64, int gridHeight = 64)
    {
        Grid = new ColonyGrid(gridWidth, gridHeight);
    }
}

public enum ColonyEventSeverity { Info, Warning, Critical }

public sealed record ColonyEvent(ColonyEventSeverity Severity, string Message, int Sol);

public sealed record EventChoice(string Label, string ConsequenceSummary, Action<ColonyState> Apply);

public sealed class PendingDecision
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public string Title { get; init; } = "";
    public string Description { get; init; } = "";
    public IReadOnlyList<EventChoice> Choices { get; init; } = Array.Empty<EventChoice>();
    public int Sol { get; init; }
}

public sealed class ColonyEventLog
{
    private readonly List<ColonyEvent> _events = new();
    public void Add(ColonyEvent evt) => _events.Add(evt);
    public void Clear() => _events.Clear();
    public IReadOnlyList<ColonyEvent> All => _events;
    public IEnumerable<ColonyEvent> Since(int sol) => _events.Where(e => e.Sol >= sol);
}
