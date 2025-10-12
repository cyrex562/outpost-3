using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event fired when a star system is scanned by a probe.
/// </summary>
public record SystemScanned : GameEvent
{
    public Ulid SystemId { get; init; }
    public string SystemName { get; init; } = "";

    /// <summary>
    /// Parameterless constructor for deserialization.
    /// </summary>
    public SystemScanned()
    {
    }

    /// <summary>
    /// Constructor with parameters.
    /// </summary>
    public SystemScanned(Ulid systemId, string systemName)
    {
        SystemId = systemId;
        SystemName = systemName;
    }
}
