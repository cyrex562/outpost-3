ROLE: Implement single-writer StateStore orchestrating reducers and event appends.

DO:
- Owns current immutable GameState.
- Apply(command): routes to reducers, returns (events, newState), appends to EventStore, publishes to ProjectionEngine.
- Advance(TimeSpan dt): emits TimeAdvanced event via reducers.

TESTS:
- Determinism (same inputs -> same outputs)
- Replay equals live state
