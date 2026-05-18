namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

/// <summary>Phase 6: save/load round-trip serialisation.</summary>
[TestFixture]
public class Phase6Tests
{
    private static ColonySession NewSession() =>
        new(new SiteDefinition(42, BiomeType.Barren, new GridSize(32, 32)));

    /// <summary>Advance the session to a mid-game state with buildings and events.</summary>
    private static ColonySession MidGameSession()
    {
        var s = NewSession();
        // Enough survival resources that nothing starves during construction.
        foreach (var r in new[] { "nutrients", "water", "oxygen" })
        {
            s.State.Resources.SetCap(r, 100_000f);
            s.State.Resources.Add(r, 99_000f);
        }

        // Build two buildings.
        s.QueueConstruction("solar_array_mk1",  new GridPosition(0, 0));
        s.QueueConstruction("nutrient_recycler", new GridPosition(5, 0));
        s.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);
        s.AutoAssignLabor();
        s.EndTurn(10);
        return s;
    }

    // ── Round-trip helpers ───────────────────────────────────────────────────

    private static ColonySession RoundTrip(ColonySession original)
    {
        string json = original.CreateSave();
        var restored = NewSession();   // fresh session
        restored.ApplySave(json);
        return restored;
    }

    // ── Sol / turn counter ───────────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesCurrentSol()
    {
        var s = MidGameSession();
        int expectedSol = s.CurrentSol;

        var r = RoundTrip(s);
        Assert.That(r.CurrentSol, Is.EqualTo(expectedSol));
    }

    // ── Resources ─────────────────────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesResourceAmounts()
    {
        var s = MidGameSession();
        float steelBefore = s.State.Resources.Get("steel");
        float waterBefore = s.State.Resources.Get("water");

        var r = RoundTrip(s);

        Assert.That(r.State.Resources.Get("steel"), Is.EqualTo(steelBefore).Within(0.01f));
        Assert.That(r.State.Resources.Get("water"), Is.EqualTo(waterBefore).Within(0.01f));
    }

    [Test]
    public void SaveLoad_PreservesResourceCaps()
    {
        var s = MidGameSession();
        float capBefore = s.State.Resources.Cap("water");

        var r = RoundTrip(s);

        Assert.That(r.State.Resources.Cap("water"), Is.EqualTo(capBefore).Within(0.01f));
    }

    // ── Buildings ─────────────────────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesBuildingCount()
    {
        var s = MidGameSession();
        int countBefore = s.State.Grid.AllSlots.Count();

        var r = RoundTrip(s);

        Assert.That(r.State.Grid.AllSlots.Count(), Is.EqualTo(countBefore));
    }

    [Test]
    public void SaveLoad_PreservesBuildingIds()
    {
        var s = MidGameSession();
        var idsBefore = s.State.Grid.AllSlots.Select(b => b.Id).OrderBy(id => id).ToList();

        var r = RoundTrip(s);
        var idsAfter = r.State.Grid.AllSlots.Select(b => b.Id).OrderBy(id => id).ToList();

        Assert.That(idsAfter, Is.EqualTo(idsBefore));
    }

    [Test]
    public void SaveLoad_PreservesBuildingStates()
    {
        var s = MidGameSession();
        var statesBefore = s.State.Grid.AllSlots
            .OrderBy(b => b.Id)
            .Select(b => (b.BuildingDefinitionId, b.State))
            .ToList();

        var r = RoundTrip(s);
        var statesAfter = r.State.Grid.AllSlots
            .OrderBy(b => b.Id)
            .Select(b => (b.BuildingDefinitionId, b.State))
            .ToList();

        Assert.That(statesAfter, Is.EqualTo(statesBefore));
    }

    [Test]
    public void SaveLoad_RebuildsPowerGrid()
    {
        var s = MidGameSession();
        float capacityBefore = s.State.Power.TotalCapacity;

        var r = RoundTrip(s);

        Assert.That(r.State.Power.TotalCapacity, Is.EqualTo(capacityBefore).Within(0.01f));
    }

    // ── Population ────────────────────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesPopulation()
    {
        var s = MidGameSession();
        int popBefore    = s.State.Population.Count;
        float moraleBefore = s.State.Population.Morale;
        float healthBefore = s.State.Population.Health;

        var r = RoundTrip(s);

        Assert.That(r.State.Population.Count,  Is.EqualTo(popBefore));
        Assert.That(r.State.Population.Morale, Is.EqualTo(moraleBefore).Within(0.01f));
        Assert.That(r.State.Population.Health, Is.EqualTo(healthBefore).Within(0.01f));
    }

    // ── Labor ─────────────────────────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesLaborAllocations()
    {
        var s = MidGameSession();
        var allocBefore = s.State.Labor.AllAllocations
            .OrderBy(kv => kv.Key)
            .ToDictionary(kv => kv.Key, kv => kv.Value);

        var r = RoundTrip(s);
        var allocAfter = r.State.Labor.AllAllocations
            .OrderBy(kv => kv.Key)
            .ToDictionary(kv => kv.Key, kv => kv.Value);

        Assert.That(allocAfter, Is.EqualTo(allocBefore));
    }

    // ── Colony status ─────────────────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesColonyStatus()
    {
        var s = NewSession();
        s.State.Status       = ColonyStatus.Critical;
        s.State.LowMoraleTurns = 7;

        var r = RoundTrip(s);

        Assert.That(r.State.Status,        Is.EqualTo(ColonyStatus.Critical));
        Assert.That(r.State.LowMoraleTurns, Is.EqualTo(7));
    }

    // ── Event log ─────────────────────────────────────────────────────────────

    [Test]
    public void SaveLoad_PreservesEventLog()
    {
        var s = MidGameSession();
        int eventCountBefore = s.State.EventLog.All.Count;

        var r = RoundTrip(s);

        Assert.That(r.State.EventLog.All.Count, Is.EqualTo(eventCountBefore));
    }

    // ── Game continues after load ─────────────────────────────────────────────

    [Test]
    public void SaveLoad_SessionContinuesAfterLoad()
    {
        var s = MidGameSession();
        int solBefore = s.CurrentSol;

        var r = RoundTrip(s);
        r.EndTurn(5);

        Assert.That(r.CurrentSol, Is.EqualTo(solBefore + 5));
    }

    [Test]
    public void SaveLoad_JsonIsReadableString()
    {
        var s = MidGameSession();
        string json = s.CreateSave();

        Assert.That(json, Does.Contain("\"CurrentSol\""));
        Assert.That(json, Does.Contain("\"Buildings\""));
        Assert.That(json, Does.Contain("solar_array_mk1"));
    }
}
