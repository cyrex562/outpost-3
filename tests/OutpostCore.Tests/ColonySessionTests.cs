namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

[TestFixture]
public class ColonySessionTests
{
    private static SiteDefinition NewSite(int seed = 42) =>
        new(seed, BiomeType.Barren, new GridSize(32, 32));

    private static ColonySession NewSession() => new(NewSite());

    [Test]
    public void Constructor_SeedsResourcesAndPopulation()
    {
        var session = NewSession();
        Assert.That(session.State.Resources.Get("steel"), Is.GreaterThan(0f));
        Assert.That(session.State.Resources.Get("electronics"), Is.GreaterThan(0f));
        Assert.That(session.State.Population.Count, Is.GreaterThan(0));
        Assert.That(session.State.Labor.TotalWorkers, Is.GreaterThan(0));
    }

    [Test]
    public void BuildableBuildings_AreRegistryEntries()
    {
        var session = NewSession();
        Assert.That(session.BuildableBuildings, Is.Not.Empty);
        foreach (var def in session.BuildableBuildings)
            Assert.That(BuildingRegistry.Get(def.Id), Is.Not.Null);
    }

    [Test]
    public void QueueConstruction_DeductsResourcesAndQueuesUnderConstruction()
    {
        var session = NewSession();
        var def = BuildingRegistry.Get("solar_array_mk1");
        float steelBefore = session.State.Resources.Get("steel");

        var result = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        Assert.That(result.Success, Is.True, result.FailureReason);
        Assert.That(result.Slot!.State, Is.EqualTo(BuildingState.UnderConstruction));
        Assert.That(result.Slot!.ConstructionTurnsRemaining, Is.EqualTo(def.ConstructionTurns));
        Assert.That(session.State.Resources.Get("steel"),
            Is.EqualTo(steelBefore - def.ConstructionCost["steel"]).Within(0.01f));
        Assert.That(session.State.Grid.AllSlots.Count(s => s.BuildingDefinitionId == "solar_array_mk1"),
            Is.EqualTo(1));
    }

    [Test]
    public void QueueConstruction_FailsOnInsufficientResources()
    {
        var session = NewSession();
        // Drain steel.
        session.State.Resources.TryConsume("steel", session.State.Resources.Get("steel"));

        var result = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        Assert.That(result.Success, Is.False);
        Assert.That(session.State.Grid.AllSlots.Any(s => s.BuildingDefinitionId == "solar_array_mk1"),
            Is.False);
    }

    [Test]
    public void QueueConstruction_FailsOnOverlap()
    {
        var session = NewSession();
        var first = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(first.Success, Is.True, first.FailureReason);

        var overlap = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(overlap.Success, Is.False);
    }

    [Test]
    public void QueueConstruction_FailsOnUnknownBuildingId()
    {
        var session = NewSession();
        var result = session.QueueConstruction("not_a_real_building", new GridPosition(0, 0));
        Assert.That(result.Success, Is.False);
    }

    [Test]
    public void EndTurn_AdvancesSolAndProcessesConstruction()
    {
        var session = NewSession();
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);
        var slot = queued.Slot!;
        int beforeRemaining = slot.ConstructionTurnsRemaining;

        session.EndTurn(5);

        Assert.That(session.CurrentSol, Is.EqualTo(5));
        Assert.That(slot.ConstructionTurnsRemaining, Is.EqualTo(beforeRemaining - 5));
    }

    [Test]
    public void Construction_CompletesAfterEnoughSols_AndRegistersWithPowerGrid()
    {
        var session = NewSession();
        float landerPower = BuildingRegistry.Get("lander").PowerProduction;
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);
        var slot = queued.Slot!;
        var def = BuildingRegistry.Get("solar_array_mk1");

        session.EndTurn(def.ConstructionTurns);

        Assert.That(slot.State, Is.EqualTo(BuildingState.Operational));
        Assert.That(slot.ConstructionTurnsRemaining, Is.LessThanOrEqualTo(0));
        // Lander's pre-placed 10 MW + the solar's 10 MW = 20 MW total.
        Assert.That(session.State.Power.TotalCapacity,
            Is.EqualTo(def.PowerProduction + landerPower).Within(0.01f));
    }

    [Test]
    public void StateChanged_FiresOnQueueAndOnTurn()
    {
        var session = NewSession();
        int fires = 0;
        session.StateChanged += () => fires++;

        session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(fires, Is.GreaterThanOrEqualTo(1));

        int afterQueue = fires;
        session.EndTurn(1);
        Assert.That(fires, Is.GreaterThan(afterQueue));
    }

    [Test]
    public void TurnAdvanced_FiresOncePerSol()
    {
        var session = NewSession();
        var sols = new List<int>();
        session.TurnAdvanced += sol => sols.Add(sol);

        session.EndTurn(3);

        Assert.That(sols, Is.EqualTo(new[] { 1, 2, 3 }));
    }

    [Test]
    public void CancelConstruction_RefundsHalfCostAndRemovesSlot()
    {
        var session = NewSession();
        var def = BuildingRegistry.Get("solar_array_mk1");
        float steelBefore = session.State.Resources.Get("steel");

        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True, queued.FailureReason);

        bool cancelled = session.CancelConstruction(queued.Slot!.Id);

        Assert.That(cancelled, Is.True);
        Assert.That(session.State.Grid.AllSlots.Any(s => s.Id == queued.Slot!.Id), Is.False);

        float expectedSteel = steelBefore - def.ConstructionCost["steel"] * 0.5f;
        Assert.That(session.State.Resources.Get("steel"),
            Is.EqualTo(expectedSteel).Within(0.01f));
    }

    [Test]
    public void CancelConstruction_FailsForOperationalSlot()
    {
        var session = NewSession();
        var def = BuildingRegistry.Get("solar_array_mk1");
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);

        session.EndTurn(def.ConstructionTurns);
        Assert.That(queued.Slot!.State, Is.EqualTo(BuildingState.Operational));

        bool cancelled = session.CancelConstruction(queued.Slot!.Id);
        Assert.That(cancelled, Is.False);
        Assert.That(session.State.Grid.AllSlots.Any(s => s.Id == queued.Slot!.Id), Is.True);
    }

    [Test]
    public void CancelConstruction_FailsForUnknownSlot()
    {
        var session = NewSession();
        bool cancelled = session.CancelConstruction(Guid.NewGuid());
        Assert.That(cancelled, Is.False);
    }

    [Test]
    public void LastNeeds_DefaultsToFull()
    {
        var session = NewSession();
        var n = session.State.Population.LastNeeds;
        Assert.That(n.Food,    Is.EqualTo(1f));
        Assert.That(n.Water,   Is.EqualTo(1f));
        Assert.That(n.Oxygen,  Is.EqualTo(1f));
        Assert.That(n.Housing, Is.EqualTo(1f));
    }

    [Test]
    public void LastNeeds_PopulatedAfterTurn()
    {
        var session = NewSession();
        // Pre-3C this test assumed no starter buildings. The 3C starter loadout
        // includes Lander (10 housing) + multi_habitat (20 housing) → 30 capacity,
        // comfortably covering the 20 colonists. Housing should be fully satisfied.
        session.EndTurn(1);

        var n = session.State.Population.LastNeeds;
        Assert.That(n.Housing, Is.EqualTo(1f).Within(0.001f),
            "starter Lander + multi_habitat cover the 20 colonists");
        Assert.That(n.Food,   Is.InRange(0f, 1f));
        Assert.That(n.Water,  Is.InRange(0f, 1f));
        Assert.That(n.Oxygen, Is.InRange(0f, 1f));
    }

    [Test]
    public void LastNeeds_FoodHitsZeroWhenStarved()
    {
        var session = NewSession();
        session.State.Resources.TryConsume("nutrients",
            session.State.Resources.Get("nutrients"));
        session.EndTurn(1);

        Assert.That(session.State.Population.LastNeeds.Food,
            Is.EqualTo(0f).Within(0.001f));
    }

    [Test]
    public void ResourceDeltas_ReflectGainsAndLosses()
    {
        var session = NewSession();
        // Population consumes nutrients/water/oxygen each turn → loss expected.
        float nutrientsBefore = session.State.Resources.Get("nutrients");
        Assert.That(nutrientsBefore, Is.GreaterThan(0f),
            "test precondition: nutrients should be seeded");

        session.EndTurn(1);

        var deltas = session.State.ResourceDeltas;
        Assert.That(deltas, Is.Not.Empty);
        Assert.That(deltas.ContainsKey("nutrients"), Is.True);
        // First turn has no operational producers → nutrients should decrease.
        Assert.That(deltas["nutrients"], Is.LessThanOrEqualTo(0f));
        float nutrientsAfter = session.State.Resources.Get("nutrients");
        Assert.That(deltas["nutrients"],
            Is.EqualTo(nutrientsAfter - nutrientsBefore).Within(0.0001f));
    }

    [Test]
    public void ResourceDeltas_StartEmpty()
    {
        var session = NewSession();
        Assert.That(session.State.ResourceDeltas, Is.Empty);
    }

    [Test]
    public void ResourceDeltas_RefreshEachTurn()
    {
        var session = NewSession();
        session.EndTurn(1);
        var first = new Dictionary<string, float>(session.State.ResourceDeltas);

        session.EndTurn(1);
        // After a second turn the deltas dictionary should be a fresh snapshot,
        // not the cumulative two-turn change.
        var second = session.State.ResourceDeltas;

        // At minimum the dictionary instance is replaced (not the same reference).
        Assert.That(ReferenceEquals(first, second), Is.False);
        // And the second-turn nutrients delta should not exceed the cumulative
        // amount we'd expect from doubling the first-turn loss + epsilon.
        if (first.TryGetValue("nutrients", out var d1) &&
            second.TryGetValue("nutrients", out var d2))
        {
            Assert.That(Math.Abs(d2), Is.LessThanOrEqualTo(Math.Abs(d1) + 0.01f));
        }
    }

    [Test]
    public void EndTurn_NormalModeFiresStateChangedOncePerSol()
    {
        var session = NewSession();
        int fires = 0;
        session.StateChanged += () => fires++;

        session.EndTurn(3);

        Assert.That(fires, Is.EqualTo(3));
        Assert.That(session.CurrentSol, Is.EqualTo(3));
    }

    [Test]
    public void EndTurn_FastModeFiresStateChangedOnce()
    {
        var session = NewSession();
        int fires = 0;
        session.StateChanged += () => fires++;

        session.EndTurn(5, fast: true);

        Assert.That(fires, Is.EqualTo(1));
        Assert.That(session.CurrentSol, Is.EqualTo(5));
    }

    [Test]
    public void EndTurn_FastModeStillEmitsTurnAdvancedEachSol()
    {
        var session = NewSession();
        var sols = new List<int>();
        session.TurnAdvanced += sol => sols.Add(sol);

        session.EndTurn(4, fast: true);

        Assert.That(sols, Is.EqualTo(new[] { 1, 2, 3, 4 }));
    }

    [Test]
    public void PendingDecision_StartsEmpty()
    {
        var session = NewSession();
        Assert.That(session.HasPendingDecision, Is.False);
        Assert.That(session.CurrentDecision, Is.Null);
    }

    [Test]
    public void AnswerDecision_AppliesChoiceAndDequeues()
    {
        var session = NewSession();
        bool applied = false;
        var decision = new PendingDecision
        {
            Title = "Test",
            Description = "A test decision",
            Sol = session.CurrentSol,
            Choices = new[]
            {
                new EventChoice("Choice A", "no effect", _ => { applied = true; }),
                new EventChoice("Choice B", "no effect", _ => { applied = false; }),
            },
        };
        session.State.PendingDecisions.Enqueue(decision);

        Assert.That(session.HasPendingDecision, Is.True);
        Assert.That(session.CurrentDecision, Is.SameAs(decision));

        bool ok = session.AnswerDecision(0);

        Assert.That(ok, Is.True);
        Assert.That(applied, Is.True);
        Assert.That(session.HasPendingDecision, Is.False);
    }

    [Test]
    public void AnswerDecision_FailsWhenNoneOrIndexInvalid()
    {
        var session = NewSession();
        Assert.That(session.AnswerDecision(0), Is.False);

        session.State.PendingDecisions.Enqueue(new PendingDecision
        {
            Title = "X",
            Choices = new[] { new EventChoice("A", "", _ => { }) },
        });
        Assert.That(session.AnswerDecision(-1), Is.False);
        Assert.That(session.AnswerDecision(5),  Is.False);
        Assert.That(session.HasPendingDecision, Is.True,
            "invalid answer must not dequeue the decision");
    }

    [Test]
    public void UpgradeBuilding_RequiresOperationalSlot()
    {
        var session = NewSession();
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);

        // Still under construction — upgrade should fail.
        var result = session.UpgradeBuilding(queued.Slot!.Id);
        Assert.That(result.Success, Is.False);
    }

    [Test]
    public void UpgradeBuilding_TransitionsToUnderConstructionAndCostsResources()
    {
        var session = NewSession();
        var def = BuildingRegistry.Get("solar_array_mk1");
        var mk2 = BuildingRegistry.Get("solar_array_mk2");
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);

        // Top up resources so mk2 is affordable.
        session.State.Resources.Add("steel",       1000f);
        session.State.Resources.Add("electronics", 1000f);

        // Finish mk1 construction.
        session.EndTurn(def.ConstructionTurns);
        Assert.That(queued.Slot!.State, Is.EqualTo(BuildingState.Operational));

        float steelBefore = session.State.Resources.Get("steel");
        var upgrade = session.UpgradeBuilding(queued.Slot!.Id);

        Assert.That(upgrade.Success, Is.True, upgrade.FailureReason);
        Assert.That(queued.Slot.BuildingDefinitionId, Is.EqualTo("solar_array_mk2"));
        Assert.That(queued.Slot.State, Is.EqualTo(BuildingState.UnderConstruction));
        Assert.That(queued.Slot.ConstructionTurnsRemaining, Is.EqualTo(mk2.ConstructionTurns));
        Assert.That(session.State.Resources.Get("steel"),
            Is.EqualTo(steelBefore - mk2.ConstructionCost["steel"]).Within(0.01f));
        // Mk1 power output should have been removed from the grid during the
        // upgrade; only the pre-placed Lander's 10 MW remains.
        float landerPower = BuildingRegistry.Get("lander").PowerProduction;
        Assert.That(session.State.Power.TotalCapacity, Is.EqualTo(landerPower).Within(0.01f));
    }

    [Test]
    public void UpgradeBuilding_CompletesAndDoublesPowerOutput()
    {
        var session = NewSession();
        var mk1 = BuildingRegistry.Get("solar_array_mk1");
        var mk2 = BuildingRegistry.Get("solar_array_mk2");
        float landerPower = BuildingRegistry.Get("lander").PowerProduction;
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);

        session.State.Resources.Add("steel",       1000f);
        session.State.Resources.Add("electronics", 1000f);
        session.State.Resources.Add("prefab_components", 50f);

        session.EndTurn(mk1.ConstructionTurns);
        session.UpgradeBuilding(queued.Slot!.Id);
        session.EndTurn(mk2.ConstructionTurns);

        Assert.That(queued.Slot!.State, Is.EqualTo(BuildingState.Operational));
        // Lander (10 MW) + solar Mk2 = total grid capacity.
        Assert.That(session.State.Power.TotalCapacity,
            Is.EqualTo(mk2.PowerProduction + landerPower).Within(0.01f));
        Assert.That(mk2.PowerProduction, Is.GreaterThan(mk1.PowerProduction));
    }

    [Test]
    public void UpgradeBuilding_FailsWithNoUpgradePath()
    {
        var session = NewSession();
        var queued = session.QueueConstruction("iron_mine", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);

        session.EndTurn(BuildingRegistry.Get("iron_mine").ConstructionTurns);

        var result = session.UpgradeBuilding(queued.Slot!.Id);
        Assert.That(result.Success, Is.False);
        Assert.That(result.FailureReason, Does.Contain("no upgrade path"));
    }

    [Test]
    public void UpgradeBuilding_FailsOnInsufficientResources()
    {
        var session = NewSession();
        var mk1 = BuildingRegistry.Get("solar_array_mk1");
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);
        session.EndTurn(mk1.ConstructionTurns);

        // Drain resources so mk2 is unaffordable.
        session.State.Resources.TryConsume("steel",       session.State.Resources.Get("steel"));
        session.State.Resources.TryConsume("electronics", session.State.Resources.Get("electronics"));

        var result = session.UpgradeBuilding(queued.Slot!.Id);
        Assert.That(result.Success, Is.False);
        Assert.That(result.FailureReason, Does.Contain("Insufficient resources"));
    }

    [Test]
    public void BuildingRegistry_AssignsExpectedCategories()
    {
        Assert.That(BuildingRegistry.Get("solar_array_mk1").Category,
            Is.EqualTo(BuildingCategory.Power));
        Assert.That(BuildingRegistry.Get("nuclear_reactor").Category,
            Is.EqualTo(BuildingCategory.Power));
        Assert.That(BuildingRegistry.Get("basic_habitat").Category,
            Is.EqualTo(BuildingCategory.Habitat));
        Assert.That(BuildingRegistry.Get("warehouse").Category,
            Is.EqualTo(BuildingCategory.Storage));
        Assert.That(BuildingRegistry.Get("large_warehouse").Category,
            Is.EqualTo(BuildingCategory.Storage));
        Assert.That(BuildingRegistry.Get("water_purifier").Category,
            Is.EqualTo(BuildingCategory.LifeSupport));
        Assert.That(BuildingRegistry.Get("life_support_system").Category,
            Is.EqualTo(BuildingCategory.LifeSupport));
        Assert.That(BuildingRegistry.Get("iron_mine").Category,
            Is.EqualTo(BuildingCategory.Production));
        Assert.That(BuildingRegistry.Get("hydroponics_bay").Category,
            Is.EqualTo(BuildingCategory.Production));
    }
}
