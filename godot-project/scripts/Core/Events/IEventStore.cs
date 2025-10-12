using System.Collections.Generic;

namespace Outpost3.Core.Events;

/// <summary>
/// Interface for event persistence storage in the event-sourced system.
/// Provides append-only storage with sequential offset-based retrieval.
/// </summary>
/// <remarks>
/// Thread-Safety Guarantees:
/// - Designed for single-writer, multiple-readers pattern
/// - Append operations are serialized and thread-safe
/// - Read operations are thread-safe and can be concurrent
/// - Offsets are 0-indexed and guaranteed to be contiguous
/// - No gaps in offset sequence - each append increments by 1 per event
/// 
/// Event Store Semantics:
/// - Events are immutable once appended
/// - Offsets are assigned sequentially starting from 0
/// - The store is append-only; no updates or deletes
/// - Events maintain their order and can be replayed deterministically
/// </remarks>
public interface IEventStore
{
    /// <summary>
    /// Appends one or more events to the store with sequential offsets.
    /// </summary>
    /// <param name="events">The events to append to the store.</param>
    /// <returns>
    /// The starting offset of the first appended event.
    /// For example, if CurrentOffset is 5 before the call, and 3 events are appended,
    /// this method returns 6 (the offset of the first new event).
    /// </returns>
    /// <remarks>
    /// Thread-Safety: This method is thread-safe for single-writer access.
    /// Multiple simultaneous calls will be serialized internally.
    /// Each event will be assigned a sequential offset starting from CurrentOffset + 1.
    /// The Offset property of each event will be set during this operation.
    /// </remarks>
    /// <exception cref="System.ArgumentNullException">Thrown if events is null.</exception>
    /// <exception cref="System.ArgumentException">Thrown if events array is empty.</exception>
    /// <exception cref="EventStoreException">Thrown if the append operation fails (e.g., disk full, I/O error).</exception>
    long Append(params GameEvent[] events);

    /// <summary>
    /// Reads all events starting from the specified offset.
    /// </summary>
    /// <param name="offset">
    /// The offset to start reading from (inclusive). Defaults to 0 to read all events.
    /// </param>
    /// <returns>
    /// An enumerable of events starting at the specified offset.
    /// Returns an empty enumerable if offset is greater than CurrentOffset.
    /// Events are returned in offset order (oldest to newest).
    /// </returns>
    /// <remarks>
    /// Thread-Safety: This method is thread-safe and can be called concurrently
    /// with other ReadFrom calls or with Append operations.
    /// 
    /// Performance: This method uses deferred execution (yield return pattern).
    /// Events are streamed from storage rather than loading all into memory.
    /// Suitable for reading large event logs.
    /// 
    /// Consistency: Reads are consistent with respect to completed Append operations.
    /// Events that are being appended may or may not be visible during concurrent reads.
    /// </remarks>
    /// <exception cref="System.ArgumentOutOfRangeException">Thrown if offset is negative.</exception>
    /// <exception cref="EventStoreException">Thrown if the read operation encounters a corrupted event or I/O error.</exception>
    IEnumerable<GameEvent> ReadFrom(long offset = 0);

    /// <summary>
    /// Gets the current highest offset in the store.
    /// </summary>
    /// <value>
    /// The offset of the most recently appended event.
    /// Returns -1 if the store is empty (no events have been appended).
    /// </value>
    /// <remarks>
    /// Thread-Safety: Reading this property is thread-safe.
    /// The value may change if concurrent Append operations are in progress.
    /// </remarks>
    long CurrentOffset { get; }

    /// <summary>
    /// Gets the total count of events in the store.
    /// </summary>
    /// <value>
    /// The total number of events that have been appended.
    /// Equal to CurrentOffset + 1 (since offsets are 0-indexed).
    /// Returns 0 if the store is empty.
    /// </value>
    /// <remarks>
    /// Thread-Safety: Reading this property is thread-safe.
    /// The value may change if concurrent Append operations are in progress.
    /// </remarks>
    long Count { get; }
}
