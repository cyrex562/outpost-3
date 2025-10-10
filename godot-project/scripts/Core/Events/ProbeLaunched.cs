using System;

namespace Outpost3.Core.Events;

public record ProbeLaunched(
    double Timestamp,
    Ulid ProbeId,
    Ulid TargetSystemId,
    double Eta
) : IGameEvent;