namespace OutpostGame.Core.Colony;

using OutpostGame.Core.Simulation;

public sealed class ColonyState
{
    public Guid Id { get; } = Guid.NewGuid();
    public string Name { get; set; } = "New Colony";
    public ColonyGrid Grid { get; } = new(64, 64);
    public ResourceStore Resources { get; } = new();
    public PopulationGroup Population { get; } = new();
    public LaborPool Labor { get; } = new();
    public PowerGrid Power { get; } = new();
    public ColonyEventLog EventLog { get; } = new();
    public TurnManager TurnManager { get; } = new();
}

public enum ColonyEventSeverity { Info, Warning, Critical }

public sealed record ColonyEvent(ColonyEventSeverity Severity, string Message, int Sol);

public sealed class ColonyEventLog
{
    private readonly List<ColonyEvent> _events = new();
    public void Add(ColonyEvent evt) => _events.Add(evt);
    public IReadOnlyList<ColonyEvent> All => _events;
    public IEnumerable<ColonyEvent> Since(int sol) => _events.Where(e => e.Sol >= sol);
}
