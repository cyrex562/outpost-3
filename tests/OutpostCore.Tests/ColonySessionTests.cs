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
        Assert.That(session.State.Grid.AllSlots.Count(), Is.EqualTo(1));
    }

    [Test]
    public void QueueConstruction_FailsOnInsufficientResources()
    {
        var session = NewSession();
        // Drain steel.
        session.State.Resources.TryConsume("steel", session.State.Resources.Get("steel"));

        var result = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        Assert.That(result.Success, Is.False);
        Assert.That(session.State.Grid.AllSlots, Is.Empty);
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
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);
        var slot = queued.Slot!;
        var def = BuildingRegistry.Get("solar_array_mk1");

        session.EndTurn(def.ConstructionTurns);

        Assert.That(slot.State, Is.EqualTo(BuildingState.Operational));
        Assert.That(slot.ConstructionTurnsRemaining, Is.LessThanOrEqualTo(0));
        Assert.That(session.State.Power.TotalCapacity,
            Is.EqualTo(def.PowerProduction).Within(0.01f));
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
}
