using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when a probe is launched toward a target star system.
/// </summary>
public record ProbeLaunched : GameEvent
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
    /// The estimated time of arrival at the target system.
    /// </summary>
    public double Eta { get; init; }

    /// <summary>
    /// Creates a new ProbeLaunched event.
    /// </summary>
    public ProbeLaunched()
    {
    }

    /// <summary>
    /// Creates a new ProbeLaunched event with specified values.
    /// </summary>
    public ProbeLaunched(Ulid probeId, Ulid targetSystemId, double eta)
    {
        ProbeId = probeId;
        TargetSystemId = targetSystemId;
        Eta = eta;
    }
}