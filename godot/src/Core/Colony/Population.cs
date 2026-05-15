namespace OutpostGame.Core.Colony;

public sealed record ColonistNeeds(
    float FoodPerSol,
    float WaterPerSol,
    float OxygenPerSol,
    float HousingUnitsRequired
);

public sealed class PopulationGroup
{
    public int Count { get; set; }
    public Dictionary<SkillType, int> Skills { get; init; } = new();
    public ColonistNeeds Needs { get; init; } = new(0.5f, 0.3f, 0.4f, 1.0f);
    public float Health { get; set; } = 100f;
    public float Morale { get; set; } = 75f;
    private int _needsDeficitTurns;

    public void ApplyNeedsSatisfaction(float foodMet, float waterMet, float oxygenMet, float housingMet)
    {
        float satisfaction = (foodMet + waterMet + oxygenMet + housingMet) / 4f;
        if (satisfaction < 0.5f)
        {
            _needsDeficitTurns++;
            Health -= (1f - satisfaction) * 2f;
            Morale -= (1f - satisfaction) * 1.5f;
        }
        else
        {
            _needsDeficitTurns = 0;
            Health = Math.Min(100f, Health + satisfaction * 0.5f);
            Morale = Math.Min(100f, Morale + (satisfaction - 0.5f) * 1f);
        }
        Health = Math.Clamp(Health, 0f, 100f);
        Morale = Math.Clamp(Morale, 0f, 100f);
    }

    public int ComputeDeaths()
    {
        if (Health <= 0f) return Math.Max(1, Count / 10);
        if (_needsDeficitTurns > 30) return Math.Max(1, Count / 50);
        return 0;
    }

    public float MoraleModifier => Morale switch
    {
        >= 80f => 1.2f,
        >= 60f => 1.0f,
        >= 40f => 0.85f,
        >= 20f => 0.65f,
        _ => 0.4f
    };
}

public sealed class LaborPool
{
    private readonly Dictionary<Guid, int> _allocations = new();   // SlotId -> workers assigned

    public int TotalWorkers { get; set; }
    public int AllocatedWorkers => _allocations.Values.Sum();
    public int IdleWorkers => Math.Max(0, TotalWorkers - AllocatedWorkers);

    public bool Assign(Guid slotId, int count)
    {
        if (count > IdleWorkers + _allocations.GetValueOrDefault(slotId, 0)) return false;
        _allocations[slotId] = count;
        return true;
    }

    public void Deallocate(Guid slotId) => _allocations.Remove(slotId);

    public float Efficiency(Guid slotId, int required, float moraleModifier)
    {
        int assigned = _allocations.GetValueOrDefault(slotId, 0);
        return Math.Min((float)assigned / Math.Max(1, required), 1.0f) * moraleModifier;
    }
}
