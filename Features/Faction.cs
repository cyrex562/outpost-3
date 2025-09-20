using Godot;
using System;

namespace Outpost3.Features;

public partial class Faction : Node
{
    public Guid FactionId { get; set; } = Guid.NewGuid();
    public string FactionName { get; set; } = string.Empty;
}