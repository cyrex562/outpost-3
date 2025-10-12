namespace Outpost3.Core.Commands;

/// <summary>
/// Command to initialize the galaxy with procedurally generated stars.
/// </summary>
public record InitializeGalaxy(int Seed, int StarCount = 100) : ICommand;
