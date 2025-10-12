using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when a new star system is discovered.
/// </summary>
public record SystemDiscovered : GameEvent
{
    /// <summary>
    /// The unique identifier of the discovered star system.
    /// </summary>
    public Ulid SystemId { get; init; }

    /// <summary>
    /// The name of the discovered star system.
    /// </summary>
    public string SystemName { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new SystemDiscovered event.
    /// </summary>
    public SystemDiscovered()
    {
    }

    /// <summary>
    /// Creates a new SystemDiscovered event with specified values.
    /// </summary>
    public SystemDiscovered(Ulid systemId, string systemName)
    {
        SystemId = systemId;
        SystemName = systemName;
    }
}