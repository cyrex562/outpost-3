namespace OutpostGame.Core.Simulation;

using OutpostGame.Core.Colony;

/// <summary>Serialisable snapshot of a full colony session. Version-tagged for future migrations.</summary>
public sealed class ColonySaveData
{
    public int Version { get; init; } = 1;
    public string ColonyName { get; init; } = "New Colony";
    public int CurrentSol { get; init; }
    public ColonyStatus Status { get; init; }
    public int DustStormTurnsRemaining { get; init; }
    public int LowMoraleTurns { get; init; }
    public int ThrivingTurns { get; init; }
    public float GrowthAccumulator { get; init; }

    public PopulationSaveData Population { get; init; } = new();
    public LaborSaveData Labor { get; init; } = new();
    public ResourceSaveData Resources { get; init; } = new();
    public List<BuildingSlotSaveData> Buildings { get; init; } = new();
    public List<EventSaveData> Events { get; init; } = new();

    // Phase 3C — construction fleet + operator pools. Nullable so pre-3C
    // saves deserialise cleanly and ApplySave populates from defaults.
    public ConstructionFleetSaveData? Fleet { get; init; }
    public OperatorPoolSaveData? Operators { get; init; }
}

public sealed class ConstructionFleetSaveData
{
    public int BaseSlots { get; init; }
    public int FactorySlots { get; init; }
    public int TechBonus { get; init; }
    public int UsedSlots { get; init; }
}

public sealed class OperatorPoolSaveData
{
    public int People { get; init; }
    public int Robots { get; init; }
    public int Allocated { get; init; }
}

public sealed class PopulationSaveData
{
    public int Count { get; init; }
    public float Health { get; init; }
    public float Morale { get; init; }
    public int NeedsDeficitTurns { get; init; }
    /// <summary>SkillType.ToString() → worker count.</summary>
    public Dictionary<string, int> Skills { get; init; } = new();
}

public sealed class LaborSaveData
{
    public int TotalWorkers { get; init; }
    /// <summary>Slot ID (string) → worker count.</summary>
    public Dictionary<string, int> Allocations { get; init; } = new();
    /// <summary>Slot ID (string) → SkillType name (e.g. "Engineer").</summary>
    public Dictionary<string, string> SkillSlots { get; init; } = new();
}

public sealed class ResourceSaveData
{
    public Dictionary<string, float> Amounts { get; init; } = new();
    public Dictionary<string, float> Caps { get; init; } = new();
}

public sealed class BuildingSlotSaveData
{
    public string Id { get; init; } = "";
    public string DefinitionId { get; init; } = "";
    public int OriginX { get; init; }
    public int OriginY { get; init; }
    public int SizeWidth { get; init; }
    public int SizeHeight { get; init; }
    public BuildingState State { get; init; }
    public int ConstructionTurnsRemaining { get; init; }
    public int AssignedWorkers { get; init; }
    public int ProductionCycleProgress { get; init; }

    // Phase 3C — held fleet / operator allocation during construction.
    public int AllocatedFleetSlots { get; init; }
    public int AllocatedOperators { get; init; }
    public int AllocatedAtSol { get; init; } = -1;
}

public sealed class EventSaveData
{
    public ColonyEventSeverity Severity { get; init; }
    public string Message { get; init; } = "";
    public int Sol { get; init; }
}
