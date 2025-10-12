using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Event fired when a star system is selected by the player.
/// </summary>
public record SystemSelected : GameEvent
{
    /// <summary>
    /// The unique identifier of the selected star system.
    /// </summary>
    public Ulid SystemId { get; init; }

    /// <summary>
    /// Creates a new SystemSelected event.
    /// </summary>
    /// <param name="systemId">The ID of the system that was selected.</param>
    public SystemSelected(Ulid systemId)
    {
        SystemId = systemId;
    }

    /// <summary>
    /// Parameterless constructor for deserialization.
    /// </summary>
    public SystemSelected() { }
}
