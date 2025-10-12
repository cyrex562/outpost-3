using System;

namespace Outpost3.Core.Commands;

/// <summary>
/// Command to select a star system for viewing details.
/// </summary>
public record SelectSystemCommand(Ulid SystemId) : ICommand;
