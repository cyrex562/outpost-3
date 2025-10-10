using System;

namespace Outpost3.Core.Events;

public record ProbeArrived(
    double Timestamp,
    Ulid ProbeId,
    Ulid TargetSystemId
) : IGameEvent;