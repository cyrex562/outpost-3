using System;

namespace Outpost3.Core.Events;

public record SystemDiscovered(
    double Timestamp,
    Ulid SystemId,
    string SystemName
) : IGameEvent;