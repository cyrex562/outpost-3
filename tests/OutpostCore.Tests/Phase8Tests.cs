namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

/// <summary>Phase 8: skill-based labor efficiency and difficulty presets.</summary>
[TestFixture]
public class Phase8Tests
{
    private static ColonySession NewSession(
        DifficultyPreset difficulty = DifficultyPreset.Normal,
        BiomeType biome = BiomeType.Barren) =>
        new(new SiteDefinition(42, biome, new GridSize(32, 32), difficulty));

    // ── Skill distribution ────────────────────────────────────────────────────

    [Test]
    public void SeedDefaults_InitialisesSkillDistribution()
    {
        var s = NewSession();
        int total = s.State.Population.Skills.Values.Sum();
        Assert.That(total, Is.EqualTo(s.State.Labor.TotalWorkers),
            "Skill counts should sum to TotalWorkers");
    }

    [Test]
    public void SeedDefaults_HasLaborersAsLargestGroup()
    {
        var s = NewSession();
        var skills = s.State.Population.Skills;
        Assert.That(skills.ContainsKey(SkillType.Laborer), Is.True);
        foreach (var kv in skills)
            Assert.That(skills[SkillType.Laborer], Is.GreaterThanOrEqualTo(kv.Value),
                $"Laborers ({skills[SkillType.Laborer]}) should be >= {kv.Key} ({kv.Value})");
    }

    [Test]
    public void SeedDefaults_HasAllSkillTypes()
    {
        var s = NewSession();
        foreach (SkillType skill in Enum.GetValues<SkillType>())
            Assert.That(s.State.Population.Skills.ContainsKey(skill), Is.True,
                $"Skills dictionary should contain {skill}");
    }

    // ── LaborPool skill tracking ──────────────────────────────────────────────

    [Test]
    public void LaborPool_AssignWithSkill_RecordsSkill()
    {
        var s = NewSession();
        s.State.Labor.TotalWorkers = 10;

        var slotId = Guid.NewGuid();
        s.State.Labor.AssignWithSkill(slotId, 5, SkillType.Engineer);

        Assert.That(s.State.Labor.GetSlotSkill(slotId), Is.EqualTo(SkillType.Engineer));
    }

    [Test]
    public void LaborPool_Deallocate_ClearsSkill()
    {
        var s = NewSession();
        s.State.Labor.TotalWorkers = 10;
        var slotId = Guid.NewGuid();
        s.State.Labor.AssignWithSkill(slotId, 5, SkillType.Farmer);
        s.State.Labor.Deallocate(slotId);

        Assert.That(s.State.Labor.GetSlotSkill(slotId), Is.Null);
    }

    // ── Skill efficiency in production ────────────────────────────────────────

    [Test]
    public void SkillMatch_GivesFullEfficiency_ComparedToMismatch()
    {
        // Two sessions — identical setup except one gets Engineer workers on the
        // smelter (which needs Engineers) and one gets Farmers.
        var sEngineer = SetupSmelterSession(SkillType.Engineer);
        var sFarmer   = SetupSmelterSession(SkillType.Farmer);

        // Run enough turns for at least one production cycle (TurnsPerCycle = 5).
        sEngineer.EndTurn(5);
        sFarmer.EndTurn(5);

        float steelEngineer = sEngineer.State.Resources.Get("steel");
        float steelFarmer   = sFarmer.State.Resources.Get("steel");

        Assert.That(steelEngineer, Is.GreaterThan(steelFarmer),
            "Engineer workers on the smelter should produce more steel than Farmer workers");
    }

    [Test]
    public void SkillMatch_GivesFullEfficiency_ComparedToLaborer()
    {
        var sEngineer = SetupSmelterSession(SkillType.Engineer);
        var sLaborer  = SetupSmelterSession(SkillType.Laborer);

        sEngineer.EndTurn(5);
        sLaborer.EndTurn(5);

        float steelEngineer = sEngineer.State.Resources.Get("steel");
        float steelLaborer  = sLaborer.State.Resources.Get("steel");

        // Engineer (1.0×) beats Laborer (0.8×)
        Assert.That(steelEngineer, Is.GreaterThan(steelLaborer),
            "Engineers should outperform generalist Laborers on the smelter");
    }

    /// <summary>
    /// Creates a session with a powered smelter that has exactly enough iron ore
    /// for several cycles, staffed with <paramref name="workerSkill"/> workers.
    /// Uses Sandbox difficulty so no random malfunction events can damage buildings.
    /// </summary>
    private static ColonySession SetupSmelterSession(SkillType workerSkill)
    {
        // Sandbox: no random events, generous resources, zero consumption pressure.
        var s = NewSession(DifficultyPreset.Sandbox);
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            s.State.Resources.SetCap(r, 100_000f);
            s.State.Resources.Add(r, 99_000f);
        }

        // Enough iron ore for several smelter cycles.
        s.State.Resources.SetCap("iron_ore", 10_000f);
        s.State.Resources.Add("iron_ore", 5_000f);
        s.State.Resources.SetCap("steel", 10_000f);

        // Two solar arrays (2×10 MW = 20 MW) so the smelter (12 MW) gets full power.
        s.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        s.QueueConstruction("solar_array_mk1", new GridPosition(3, 0));
        s.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);

        s.QueueConstruction("smelter", new GridPosition(6, 0));
        s.EndTurn(BuildingRegistry.Get("smelter").ConstructionTurns);

        // Assign the smelter's required workers using the specified skill.
        var def    = BuildingRegistry.Get("smelter");
        var smelter = s.State.Grid.AllSlots.First(sl => sl.BuildingDefinitionId == "smelter");
        s.State.Labor.AssignWithSkill(smelter.Id, def.LaborRequired, workerSkill);

        // Zero out steel produced during construction so we measure only post-setup.
        s.State.Resources.TryConsume("steel", s.State.Resources.Get("steel"));

        return s;
    }

    // ── AutoAssignLabor skill awareness ───────────────────────────────────────

    [Test]
    public void AutoAssign_SetsSkillForEachBuilding()
    {
        var s = NewSession();
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            s.State.Resources.SetCap(r, 100_000f);
            s.State.Resources.Add(r, 99_000f);
        }
        s.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        s.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);

        s.State.Labor.TotalWorkers = 20;
        s.AutoAssignLabor();

        var solarSlot = s.State.Grid.AllSlots.First(sl => sl.BuildingDefinitionId == "solar_array_mk1");
        Assert.That(s.State.Labor.GetSlotSkill(solarSlot.Id), Is.Not.Null,
            "AutoAssignLabor should record a skill type for each assigned slot");
    }

    [Test]
    public void AutoAssign_PrefersPrimarySkill_WhenAvailable()
    {
        var s = NewSession();
        // Give the colony plenty of Operators specifically.
        s.State.Population.Skills[SkillType.Operator] = 10;
        s.State.Population.Skills[SkillType.Laborer]  = 2;
        s.State.Labor.TotalWorkers = 12;

        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            s.State.Resources.SetCap(r, 100_000f);
            s.State.Resources.Add(r, 99_000f);
        }
        s.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        s.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);

        s.AutoAssignLabor();

        var solarSlot = s.State.Grid.AllSlots.First(sl => sl.BuildingDefinitionId == "solar_array_mk1");
        // solar_array_mk1 PrimarySkill = Operator
        Assert.That(s.State.Labor.GetSlotSkill(solarSlot.Id), Is.EqualTo(SkillType.Operator),
            "AutoAssign should assign Operator workers to the solar array when available");
    }

    // ── Difficulty: starting resources ────────────────────────────────────────

    [Test]
    public void Difficulty_Sandbox_StartsWithMoreResourcesThanNormal()
    {
        float sandboxSteel = NewSession(DifficultyPreset.Sandbox).State.Resources.Get("steel");
        float normalSteel  = NewSession(DifficultyPreset.Normal).State.Resources.Get("steel");
        Assert.That(sandboxSteel, Is.GreaterThan(normalSteel));
    }

    [Test]
    public void Difficulty_Brutal_StartsWithFewerResourcesThanNormal()
    {
        float brutalSteel = NewSession(DifficultyPreset.Brutal).State.Resources.Get("steel");
        float normalSteel = NewSession(DifficultyPreset.Normal).State.Resources.Get("steel");
        Assert.That(brutalSteel, Is.LessThan(normalSteel));
    }

    [Test]
    public void Difficulty_ResourceOrder_SandboxGtEasyGtNormalGtHardGtBrutal()
    {
        float[] steel = new[]
        {
            DifficultyPreset.Sandbox, DifficultyPreset.Easy,
            DifficultyPreset.Normal, DifficultyPreset.Hard, DifficultyPreset.Brutal,
        }.Select(d => NewSession(d).State.Resources.Get("steel")).ToArray();

        for (int i = 0; i < steel.Length - 1; i++)
            Assert.That(steel[i], Is.GreaterThanOrEqualTo(steel[i + 1]),
                $"Steel at index {i} ({steel[i]}) should be >= index {i+1} ({steel[i+1]})");
    }

    // ── Difficulty: consumption ────────────────────────────────────────────────

    [Test]
    public void Difficulty_Sandbox_NeedsNeverDepleteResources()
    {
        var s = NewSession(DifficultyPreset.Sandbox);
        float nutrientsBefore = s.State.Resources.Get("nutrients");

        s.EndTurn(50);

        float nutrientsAfter = s.State.Resources.Get("nutrients");
        Assert.That(nutrientsAfter, Is.GreaterThanOrEqualTo(nutrientsBefore * 0.99f),
            "On Sandbox difficulty, population needs should not deplete resources");
    }

    [Test]
    public void Difficulty_Hard_DepletesResourcesFasterThanNormal()
    {
        // Create two sessions with identical resources, advance the same number of turns.
        var sHard   = CreateSessionWithFixedResources(DifficultyPreset.Hard);
        var sNormal = CreateSessionWithFixedResources(DifficultyPreset.Normal);

        sHard.EndTurn(10);
        sNormal.EndTurn(10);

        float nutrientsHard   = sHard.State.Resources.Get("nutrients");
        float nutrientsNormal = sNormal.State.Resources.Get("nutrients");

        Assert.That(nutrientsHard, Is.LessThan(nutrientsNormal),
            "Hard difficulty should consume nutrients faster than Normal");
    }

    private static ColonySession CreateSessionWithFixedResources(DifficultyPreset difficulty)
    {
        var s = NewSession(difficulty);
        // Override resources to identical starting values regardless of difficulty multiplier.
        s.State.Resources.SetCap("nutrients", 100_000f);
        s.State.Resources.SetCap("water", 100_000f);
        s.State.Resources.SetCap("oxygen", 100_000f);
        s.State.Resources.RestoreFromSnapshot(
            new Dictionary<string, float>
            {
                ["nutrients"] = 10_000f,
                ["water"]     = 10_000f,
                ["oxygen"]    = 10_000f,
            },
            new Dictionary<string, float>
            {
                ["nutrients"] = 100_000f,
                ["water"]     = 100_000f,
                ["oxygen"]    = 100_000f,
            });
        return s;
    }

    // ── Difficulty: event frequency ───────────────────────────────────────────

    [Test]
    public void Difficulty_Sandbox_NoEventsAfterManyStrategicTurns()
    {
        var s = NewSession(DifficultyPreset.Sandbox);
        // Advance past several intervals of Normal (every 30) and Hard (every 20).
        s.EndTurn(300);

        // None of the random-event-driven log entries should fire.
        // - "Dust storm", "Equipment failure", "Supply drop" come from
        //   RandomEventProcessor outcomes.
        // - The random arrival event logs "colonist(s) arrived — population now …"
        //   (em-dash). Distinct from ProcessPopulationGrowth's
        //   "new colonist(s) arrived (pop now …)" which is *not* a random
        //   event and DOES fire under Sandbox once the starter habitats give
        //   room to grow.
        bool hasRandomEvent = s.State.EventLog.All.Any(e =>
            e.Message.Contains("Dust storm") ||
            e.Message.Contains("Equipment failure") ||
            e.Message.Contains("Supply drop") ||
            e.Message.Contains("colonist(s) arrived — population now"));
        Assert.That(hasRandomEvent, Is.False,
            "Sandbox difficulty should suppress random events");
    }

    [Test]
    public void Difficulty_Hard_EventsFireAtInterval20()
    {
        // Confirm DifficultySettings returns interval 20 for Hard.
        Assert.That(DifficultySettings.StrategicEventInterval(DifficultyPreset.Hard), Is.EqualTo(20));
    }

    [Test]
    public void Difficulty_Normal_EventsFireAtInterval30()
    {
        Assert.That(DifficultySettings.StrategicEventInterval(DifficultyPreset.Normal), Is.EqualTo(30));
    }

    [Test]
    public void Difficulty_Brutal_EventsFireAtInterval15()
    {
        Assert.That(DifficultySettings.StrategicEventInterval(DifficultyPreset.Brutal), Is.EqualTo(15));
    }

    // ── Save/load preserves skills ────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesSkillDistribution()
    {
        var s = NewSession();
        var skillsBefore = new Dictionary<SkillType, int>(s.State.Population.Skills);

        string json = s.CreateSave();
        var r = NewSession();
        r.ApplySave(json);

        foreach (var kv in skillsBefore)
            Assert.That(r.State.Population.Skills.GetValueOrDefault(kv.Key, 0),
                Is.EqualTo(kv.Value), $"Skill {kv.Key} should be preserved");
    }

    [Test]
    public void SaveLoad_PreservesSlotSkillAssignments()
    {
        var s = NewSession();
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            s.State.Resources.SetCap(r, 100_000f);
            s.State.Resources.Add(r, 99_000f);
        }
        s.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        s.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);
        s.State.Labor.TotalWorkers = 20;
        s.AutoAssignLabor();

        var slotId = s.State.Grid.AllSlots.First().Id;
        SkillType? skillBefore = s.State.Labor.GetSlotSkill(slotId);

        string json = s.CreateSave();
        var restored = NewSession();
        restored.ApplySave(json);

        Assert.That(restored.State.Labor.GetSlotSkill(slotId), Is.EqualTo(skillBefore),
            "Slot skill assignment should survive save/load");
    }

    // ── BuildingDefinition PrimarySkill ───────────────────────────────────────

    [Test]
    public void BuildingDefinition_PrimarySkill_AssignedCorrectly()
    {
        Assert.That(BuildingRegistry.Get("smelter").PrimarySkill,       Is.EqualTo(SkillType.Engineer));
        Assert.That(BuildingRegistry.Get("fabricator").PrimarySkill,    Is.EqualTo(SkillType.Engineer));
        Assert.That(BuildingRegistry.Get("solar_array_mk1").PrimarySkill, Is.EqualTo(SkillType.Operator));
        Assert.That(BuildingRegistry.Get("hydroponics_bay").PrimarySkill, Is.EqualTo(SkillType.Farmer));
        Assert.That(BuildingRegistry.Get("iron_mine").PrimarySkill,     Is.EqualTo(SkillType.Laborer));
    }
}
