namespace Outpost3.Core.Commands;

public record AdvanceTime(double Dt) : ICommand;