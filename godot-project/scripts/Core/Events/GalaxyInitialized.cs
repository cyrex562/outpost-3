namespace Outpost3.Core.Events;

/// <summary>
/// Event fired when the galaxy is initialized at game start.
/// </summary>
public record GalaxyInitialized : GameEvent
{
    public int StarCount { get; init; }
    public int Seed { get; init; }

    /// <summary>
    /// Parameterless constructor for deserialization.
    /// </summary>
    public GalaxyInitialized()
    {
    }

    /// <summary>
    /// Constructor with parameters.
    /// </summary>
    public GalaxyInitialized(int starCount, int seed)
    {
        StarCount = starCount;
        Seed = seed;
    }
}
