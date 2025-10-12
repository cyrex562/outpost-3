ROLE: You are implementing the append-only, ordered, thread-safe InMemoryEventStore.

CONTEXT:
- Interface in src/Game.App/EventStore.cs:
  - long Append(ReadOnlySpan<object> events)
  - IAsyncEnumerable<object> ReadFrom(long offset, CancellationToken ct = default)
  - long CurrentOffset { get; }
- Use Channel<T> to fan-out events to readers.
- Offsets start at 0 and are contiguous.

DELIVERABLES:
1) Implement InMemoryEventStore.cs in src/Game.App/.
2) Unit tests in src/Game.Tests/ with xUnit:
   - Append 3 events => offsets 0..2
   - ReadFrom(1) yields last two events in order
   - Concurrent append/read determinism

CONSTRAINTS:
- No blocking on reads (backpressure optional)
- Pure ordering guarantees; no filtering by type
