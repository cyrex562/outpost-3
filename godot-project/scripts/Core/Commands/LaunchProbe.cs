using System;

namespace Outpost3.Core.Commands;

public record LaunchProbe(Ulid TargetSystemId) : ICommand;