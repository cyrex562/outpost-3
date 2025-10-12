using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when a probe arrives at its target star system.
/// </summary>
public record ProbeArrived : GameEvent
{
    /// <summary>
    /// The unique identifier of the probe.
    /// </summary>
    public Ulid ProbeId { get; init; }

    /// <summary>
    /// The unique identifier of the target star system.
    /// </summary>
    public Ulid TargetSystemId { get; init; }

    /// <summary>
    /// Creates a new ProbeArrived event.
    /// </summary>
    public ProbeArrived()
    {
    }

    /// <summary>
    /// Creates a new ProbeArrived event with specified values.
    /// </summary>
    public ProbeArrived(Ulid probeId, Ulid targetSystemId)
    {
        ProbeId = probeId;
        TargetSystemId = targetSystemId;
    }
}