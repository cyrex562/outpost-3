namespace Outpost3.Core.Events;

public record TimeAdvanced(double Timestamp, double Dt) : IGameEvent;