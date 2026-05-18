namespace OutpostGame.Core.Colony;

public sealed record ColonistNeeds(
    float FoodPerSol,
    float WaterPerSol,
    float OxygenPerSol,
    float HousingUnitsRequired
);

/// <summary>
/// Most-recent per-need satisfaction values (0..1) from the last
/// <see cref="PopulationGroup.ApplyNeedsSatisfaction"/> call. Surfaced on
/// <see cref="PopulationGroup.LastNeeds"/> so the HUD can render a breakdown.
/// </summary>
public readonly record struct NeedsSatisfaction(float Food, float Water, float Oxygen, float Housing)
{
    public static NeedsSatisfaction Full => new(1f, 1f, 1f, 1f);
}

public sealed class PopulationGroup
{
    public int Count { get; set; }
    public Dictionary<SkillType, int> Skills { get; init; } = new();
    public ColonistNeeds Needs { get; init; } = new(0.5f, 0.3f, 0.4f, 1.0f);
    public float Health { get; set; } = 100f;
    public float Morale { get; set; } = 75f;
    public int NeedsDeficitTurns { get; set; }

    public NeedsSatisfaction LastNeeds { get; private set; } = NeedsSatisfaction.Full;

    public void ApplyNeedsSatisfaction(float foodMet, float waterMet, float oxygenMet, float housingMet)
    {
        LastNeeds = new NeedsSatisfaction(foodMet, waterMet, oxygenMet, housingMet);
        float satisfaction = (foodMet + waterMet + oxygenMet + housingMet) / 4f;
        if (satisfaction < 0.5f)
        {
            NeedsDeficitTurns++;
            Health -= (1f - satisfaction) * 2f;
            Morale -= (1f - satisfaction) * 1.5f;
        }
        else
        {
            NeedsDeficitTurns = 0;
            Health = Math.Min(100f, Health + satisfaction * 0.5f);
            Morale = Math.Min(100f, Morale + (satisfaction - 0.5f) * 1f);
        }
        Health = Math.Clamp(Health, 0f, 100f);
        Morale = Math.Clamp(Morale, 0f, 100f);
    }

    public int ComputeDeaths()
    {
        if (Health <= 0f) return Math.Max(1, Count / 10);
        if (NeedsDeficitTurns > 30) return Math.Max(1, Count / 50);
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
    private readonly Dictionary<Guid, int>       _allocations = new();  // SlotId → workers assigned
    private readonly Dictionary<Guid, SkillType> _slotSkills  = new();  // SlotId → skill type in use

    public int TotalWorkers { get; set; }
    public int AllocatedWorkers => _allocations.Values.Sum();
    public int IdleWorkers => Math.Max(0, TotalWorkers - AllocatedWorkers);

    public bool Assign(Guid slotId, int count)
    {
        if (count > IdleWorkers + _allocations.GetValueOrDefault(slotId, 0)) return false;
        _allocations[slotId] = count;
        return true;
    }

    /// <summary>Assign workers and record the skill type being used at this slot.</summary>
    public bool AssignWithSkill(Guid slotId, int count, SkillType skill)
    {
        if (!Assign(slotId, count)) return false;
        _slotSkills[slotId] = skill;
        return true;
    }

    public void Deallocate(Guid slotId)
    {
        _allocations.Remove(slotId);
        _slotSkills.Remove(slotId);
    }

    public void ClearSlotSkill(Guid slotId) => _slotSkills.Remove(slotId);

    public SkillType? GetSlotSkill(Guid slotId) =>
        _slotSkills.TryGetValue(slotId, out var s) ? s : null;

    public IReadOnlyDictionary<Guid, int>       AllAllocations  => _allocations;
    public IReadOnlyDictionary<Guid, SkillType> AllSkillSlots   => _slotSkills;

    public void RestoreAllocations(IReadOnlyDictionary<Guid, int> allocations)
    {
        _allocations.Clear();
        foreach (var kv in allocations) _allocations[kv.Key] = kv.Value;
    }

    public void RestoreSkillSlots(IReadOnlyDictionary<Guid, SkillType> skillSlots)
    {
        _slotSkills.Clear();
        foreach (var kv in skillSlots) _slotSkills[kv.Key] = kv.Value;
    }

    /// <summary>
    /// Production efficiency for a slot, combining fill-rate, skill match, and morale.
    /// </summary>
    public float Efficiency(Guid slotId, int required, float moraleModifier,
                            SkillType? requiredSkill = null)
    {
        int assigned = _allocations.GetValueOrDefault(slotId, 0);
        float fillRate = Math.Min((float)assigned / Math.Max(1, required), 1.0f);
        float skillMod = requiredSkill.HasValue
            ? SkillEfficiencyModifier(_slotSkills.TryGetValue(slotId, out var s) ? s : null,
                                      requiredSkill.Value)
            : 1.0f;
        return fillRate * skillMod * moraleModifier;
    }

    /// <summary>
    /// Efficiency modifier based on whether the assigned skill matches what the building needs.
    /// Perfect match → 1.0; generalist Laborer → 0.80; wrong specialist → 0.65.
    /// </summary>
    private static float SkillEfficiencyModifier(SkillType? assigned, SkillType required)
    {
        if (assigned == null)                   return 0.80f; // no skill recorded → generalist
        if (assigned.Value == required)         return 1.00f; // perfect match
        if (assigned.Value == SkillType.Laborer) return 0.80f; // generalist can do anything
        return 0.65f;                                          // wrong specialist
    }
}
