using System;

namespace Outpost3.Core.Domain;

public record ProbeInFlight
{
    public Ulid Id { get; init; }
    public Ulid TargetSystemId { get; init; }
    public double ArrivalTime { get; init; }
    public double LaunchedAt { get; init; }

    public double TimeRemaining(double currentTime) => ArrivalTime - currentTime;
}