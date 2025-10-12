using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event emitted when the colony ship departs from Earth to begin the journey phase.
/// Marks the start of the interstellar voyage.
/// </summary>
public record ShipDepartedEvent : GameEvent
{
    /// <summary>
    /// The unique identifier of the destination star system.
    /// </summary>
    public Ulid DestinationSystemId { get; init; }

    /// <summary>
    /// The name of the colony ship.
    /// </summary>
    public string ShipName { get; init; } = string.Empty;

    /// <summary>
    /// The number of colonists aboard the ship.
    /// </summary>
    public int ColonistCount { get; init; }

    /// <summary>
    /// Creates a new ShipDepartedEvent.
    /// </summary>
    public ShipDepartedEvent()
    {
    }

    /// <summary>
    /// Creates a new ShipDepartedEvent with specified values.
    /// </summary>
    public ShipDepartedEvent(Ulid destinationSystemId, string shipName, int colonistCount)
    {
        DestinationSystemId = destinationSystemId;
        ShipName = shipName;
        ColonistCount = colonistCount;
    }
}
