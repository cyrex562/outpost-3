namespace Outpost3.Core.Events;

/// <summary>
/// Marker interface for all game events in the event-sourced system.
/// All concrete events should inherit from GameEvent which implements this interface.
/// This interface allows for polymorphic collections and flexibility for future event types.
/// </summary>
public interface IGameEvent
{
}   