using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when the colony ship arrives at its destination star system.
/// Marks the end of the journey phase and the transition to the settlement phase.
/// </summary>
public record ShipArrivedEvent : GameEvent
{
    /// <summary>
    /// The unique identifier of the star system where the ship has arrived.
    /// </summary>
    public Ulid SystemId { get; init; }

    /// <summary>
    /// The name of the star system where the ship has arrived.
    /// </summary>
    public string SystemName { get; init; } = string.Empty;

    /// <summary>
    /// The total duration of the journey in game time units (seconds/ticks).
    /// </summary>
    public float TravelDuration { get; init; }

    /// <summary>
    /// Creates a new ShipArrivedEvent.
    /// </summary>
    public ShipArrivedEvent()
    {
    }

    /// <summary>
    /// Creates a new ShipArrivedEvent with specified values.
    /// </summary>
    public ShipArrivedEvent(Ulid systemId, string systemName, float travelDuration)
    {
        SystemId = systemId;
        SystemName = systemName;
        TravelDuration = travelDuration;
    }
}
