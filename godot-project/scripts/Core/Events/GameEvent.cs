using System;

namespace Outpost3.Core.Events;

/// <summary>
/// Base abstract record for all game events in the event-sourced system.
/// Events are immutable and represent facts that have occurred in the game.
/// </summary>
public abstract record GameEvent : IGameEvent
{
    /// <summary>
    /// Sequential ID assigned by the EventStore when persisted.
    /// Used for ordering and replay.
    /// </summary>
    public long Offset { get; init; }

    /// <summary>
    /// In-game time when this event occurred (game ticks/seconds).
    /// </summary>
    public float GameTime { get; init; }

    /// <summary>
    /// Real-world timestamp when this event was created.
    /// </summary>
    public DateTime RealTime { get; init; }

    /// <summary>
    /// Type discriminator for serialization and deserialization.
    /// Automatically populated from the concrete type name.
    /// </summary>
    public string EventType { get; init; }

    /// <summary>
    /// Protected constructor to ensure EventType is set automatically.
    /// </summary>
    protected GameEvent()
    {
        EventType = GetType().Name;
        RealTime = DateTime.UtcNow;
    }
}
