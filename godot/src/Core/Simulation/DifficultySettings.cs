namespace OutpostGame.Core.Simulation;

using OutpostGame.Core.Colony;

/// <summary>
/// Scaling constants for each <see cref="DifficultyPreset"/>.
/// All values are pure multipliers relative to Normal (1.0).
/// </summary>
public static class DifficultySettings
{
    /// <summary>
    /// Multiplier applied to all starting resource amounts in <see cref="ColonySession"/>.
    /// </summary>
    public static float ResourceMultiplier(DifficultyPreset d) => d switch
    {
        DifficultyPreset.Sandbox => 3.0f,
        DifficultyPreset.Easy   => 1.5f,
        DifficultyPreset.Normal => 1.0f,
        DifficultyPreset.Hard   => 0.70f,
        DifficultyPreset.Brutal => 0.50f,
        _                       => 1.0f,
    };

    /// <summary>
    /// Multiplier applied to population survival needs and building maintenance costs.
    /// Sandbox = 0 (needs never deplete resources).
    /// </summary>
    public static float ConsumptionMultiplier(DifficultyPreset d) => d switch
    {
        DifficultyPreset.Sandbox => 0.0f,
        DifficultyPreset.Easy   => 0.70f,
        DifficultyPreset.Normal => 1.0f,
        DifficultyPreset.Hard   => 1.30f,
        DifficultyPreset.Brutal => 1.60f,
        _                       => 1.0f,
    };

    /// <summary>
    /// How many sols between strategic-event rolls.
    /// 0 means events never fire (Sandbox).
    /// </summary>
    public static int StrategicEventInterval(DifficultyPreset d) => d switch
    {
        DifficultyPreset.Sandbox => 0,
        DifficultyPreset.Easy   => 60,
        DifficultyPreset.Normal => 30,
        DifficultyPreset.Hard   => 20,
        DifficultyPreset.Brutal => 15,
        _                       => 30,
    };

    /// <summary>
    /// Skill distribution for the starting labour pool (15 workers on Normal).
    /// Returns a dictionary of SkillType → count whose values sum to
    /// <paramref name="totalWorkers"/>.
    /// </summary>
    public static Dictionary<SkillType, int> StartingSkills(int totalWorkers)
    {
        // Baseline fractions — Laborer-heavy, with a few specialists.
        var raw = new Dictionary<SkillType, float>
        {
            [SkillType.Laborer]   = 0.40f,
            [SkillType.Engineer]  = 0.20f,
            [SkillType.Operator]  = 0.15f,
            [SkillType.Farmer]    = 0.12f,
            [SkillType.Medic]     = 0.07f,
            [SkillType.Scientist] = 0.06f,
        };

        var result = new Dictionary<SkillType, int>();
        int assigned = 0;
        foreach (var (skill, frac) in raw.OrderByDescending(kv => kv.Value))
        {
            int count = (int)MathF.Round(frac * totalWorkers);
            result[skill] = count;
            assigned += count;
        }

        // Rounding errors — give remainder to Laborers.
        result[SkillType.Laborer] += totalWorkers - assigned;
        return result;
    }
}
