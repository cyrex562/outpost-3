namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when game time advances by a tick.
/// </summary>
public record TimeAdvanced : GameEvent
{
    /// <summary>
    /// The time delta that was advanced.
    /// </summary>
    public double Dt { get; init; }

    /// <summary>
    /// Creates a new TimeAdvanced event.
    /// </summary>
    public TimeAdvanced()
    {
    }

    /// <summary>
    /// Creates a new TimeAdvanced event with the specified delta.
    /// </summary>
    public TimeAdvanced(double dt)
    {
        Dt = dt;
    }
}