namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

[TestFixture]
public class ConstructionFleetTests
{
    private static SiteDefinition NewSite(int seed = 42) =>
        new(seed, BiomeType.Barren, new GridSize(32, 32));
    private static ColonySession NewSession() => new(NewSite());

    [Test]
    public void Starter_LoadoutPlacesLanderAndMultiHabitat()
    {
        var session = NewSession();
        var ids = session.State.Grid.AllSlots.Select(s => s.BuildingDefinitionId).ToList();
        Assert.That(ids, Does.Contain("lander"));
        Assert.That(ids, Does.Contain("multi_habitat"));
        Assert.That(ids.Count, Is.EqualTo(2));
    }

    [Test]
    public void Starter_BuildingsAreOperationalAndPowered()
    {
        var session = NewSession();
        foreach (var slot in session.State.Grid.AllSlots)
            Assert.That(slot.State, Is.EqualTo(BuildingState.Operational));
        // Lander contributes 10 MW; multi_habitat draws 2 MW. Net positive.
        Assert.That(session.State.Power.TotalCapacity, Is.GreaterThan(0f));
    }

    [Test]
    public void Fleet_InitialState_HasGenerousCapacity()
    {
        var session = NewSession();
        Assert.That(session.State.Fleet.BaseSlots,    Is.EqualTo(10));
        Assert.That(session.State.Fleet.FactorySlots, Is.EqualTo(0));
        Assert.That(session.State.Fleet.TotalSlots,   Is.EqualTo(10));
        Assert.That(session.State.Fleet.UsedSlots,    Is.EqualTo(0));
        Assert.That(session.State.Fleet.Available,    Is.EqualTo(10));
    }

    [Test]
    public void Operators_InitialState_HasPeopleAndRobots()
    {
        var session = NewSession();
        Assert.That(session.State.Operators.People, Is.EqualTo(10));
        Assert.That(session.State.Operators.Robots, Is.EqualTo(5));
        Assert.That(session.State.Operators.Total,  Is.EqualTo(15));
        Assert.That(session.State.Operators.Available, Is.EqualTo(15));
    }

    [Test]
    public void Fleet_AllocatesOnConstructionStart()
    {
        var session = NewSession();
        session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        // Allocation happens on the first turn (AdvanceConstruction).
        int beforeUsed = session.State.Fleet.UsedSlots;
        session.EndTurn(1);
        Assert.That(session.State.Fleet.UsedSlots, Is.GreaterThan(beforeUsed),
            "Queuing a building should consume fleet slots within one sol");
    }

    [Test]
    public void Fleet_ReleasesOnCompletion()
    {
        var session = NewSession();
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        var def = BuildingRegistry.Get("solar_array_mk1");

        session.EndTurn(def.ConstructionTurns);

        Assert.That(queued.Slot!.State, Is.EqualTo(BuildingState.Operational));
        Assert.That(session.State.Fleet.UsedSlots, Is.EqualTo(0),
            "Completed construction must release fleet slots back to the pool");
        Assert.That(session.State.Operators.Allocated, Is.EqualTo(0));
    }

    [Test]
    public void Fleet_ReleasesOnCancel()
    {
        var session = NewSession();
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        session.EndTurn(1); // allocate
        Assert.That(session.State.Fleet.UsedSlots, Is.GreaterThan(0));

        session.CancelConstruction(queued.Slot!.Id);

        Assert.That(session.State.Fleet.UsedSlots, Is.EqualTo(0),
            "Cancelling construction must release the held fleet slots");
        Assert.That(session.State.Operators.Allocated, Is.EqualTo(0));
    }

    [Test]
    public void Construction_HardStops_WhenFleetExhausted()
    {
        // Drain the fleet so allocation must fail.
        var session = NewSession();
        session.State.Fleet.TryAllocate(session.State.Fleet.Available);

        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);
        int initialRemaining = queued.Slot!.ConstructionTurnsRemaining;

        session.EndTurn(10);

        Assert.That(queued.Slot.State, Is.EqualTo(BuildingState.UnderConstruction));
        Assert.That(queued.Slot.ConstructionTurnsRemaining, Is.EqualTo(initialRemaining),
            "With no fleet available the project must not progress at all (hard stop)");
        Assert.That(queued.Slot.AllocatedFleetSlots, Is.EqualTo(0));
    }

    [Test]
    public void Construction_HardStops_WhenOperatorsExhausted()
    {
        var session = NewSession();
        // Drain operators but leave fleet available.
        session.State.Operators.TryAllocate(session.State.Operators.Available);

        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True);
        int initialRemaining = queued.Slot!.ConstructionTurnsRemaining;

        session.EndTurn(10);

        Assert.That(queued.Slot.ConstructionTurnsRemaining, Is.EqualTo(initialRemaining));
        Assert.That(queued.Slot.AllocatedFleetSlots, Is.EqualTo(0),
            "Fleet must NOT be partially allocated when operators are missing");
    }

    [Test]
    public void Allocation_RespectsCapacity_WhenPoolIsTight()
    {
        // Fill the fleet capacity so only one slot remains; queue two buildings
        // that each need 1 slot. After a sol, exactly one should be allocated
        // (the other waits for the pool to free up).
        var session = NewSession();
        session.State.Fleet.TryAllocate(9); // 1 slot left

        var first  = session.QueueConstruction("nutrient_recycler", new GridPosition(0, 0));
        var second = session.QueueConstruction("nutrient_recycler", new GridPosition(3, 0));
        Assert.That(first.Success && second.Success, Is.True);

        session.EndTurn(1);

        int allocatedSlots = (first.Slot!.AllocatedFleetSlots > 0 ? 1 : 0)
                           + (second.Slot!.AllocatedFleetSlots > 0 ? 1 : 0);
        Assert.That(allocatedSlots, Is.EqualTo(1),
            "Exactly one of the two queued buildings should be allocated when only one fleet slot is free");
        Assert.That(session.State.Fleet.UsedSlots, Is.EqualTo(10),
            "All 10 fleet slots should be in use after one of the two queued buildings allocates");
    }

    [Test]
    public void Allocation_IsDeterministic_AcrossSaveLoad()
    {
        // Queue two buildings, advance until one is allocated, then save/load.
        // The same slot should still be the allocated one after load.
        var session = NewSession();
        session.State.Fleet.TryAllocate(9);

        var first  = session.QueueConstruction("nutrient_recycler", new GridPosition(0, 0));
        var second = session.QueueConstruction("nutrient_recycler", new GridPosition(3, 0));
        session.EndTurn(1);

        Guid? allocatedId = (first.Slot!.AllocatedFleetSlots > 0) ? first.Slot.Id
                          : (second.Slot!.AllocatedFleetSlots > 0) ? second.Slot.Id
                          : null;
        Assert.That(allocatedId, Is.Not.Null,
            "At least one slot must have been allocated for this test to be meaningful");

        string json = session.CreateSave();
        var fresh = NewSession();
        fresh.ApplySave(json);
        var restoredAllocated = fresh.State.Grid.AllSlots
            .FirstOrDefault(s => s.AllocatedFleetSlots > 0
                              && s.BuildingDefinitionId == "nutrient_recycler");
        Assert.That(restoredAllocated, Is.Not.Null);
        Assert.That(restoredAllocated!.Id, Is.EqualTo(allocatedId.Value));
    }

    [Test]
    public void Prefab_RequiredOnQueueConstruction()
    {
        var session = NewSession();
        session.State.Resources.TryConsume("prefab_components",
            session.State.Resources.Get("prefab_components"));

        var result = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        Assert.That(result.Success, Is.False);
    }

    [Test]
    public void Prefab_RequiredOnUpgrade()
    {
        var session = NewSession();
        var mk1 = BuildingRegistry.Get("solar_array_mk1");
        var queued = session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        session.EndTurn(mk1.ConstructionTurns);
        Assert.That(queued.Slot!.State, Is.EqualTo(BuildingState.Operational));

        // Drain prefab; keep other resources topped up.
        session.State.Resources.Add("steel",       1000f);
        session.State.Resources.Add("electronics", 1000f);
        session.State.Resources.TryConsume("prefab_components",
            session.State.Resources.Get("prefab_components"));

        var result = session.UpgradeBuilding(queued.Slot!.Id);

        Assert.That(result.Success, Is.False);
        Assert.That(result.FailureReason, Does.Contain("Insufficient"));
    }

    [Test]
    public void PrefabFactory_ProducesPrefabComponents()
    {
        // Build a prefab factory with plenty of input materials and wait a cycle.
        var session = NewSession();
        session.State.Resources.Add("steel",      500f);
        session.State.Resources.Add("components", 500f);
        // Increase prefab cap so the freshly-produced units aren't capped out.
        session.State.Resources.SetCap("prefab_components", 1000f);

        var queued = session.QueueConstruction("prefab_factory", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True, queued.FailureReason);

        var def = BuildingRegistry.Get("prefab_factory");
        session.EndTurn(def.ConstructionTurns); // finish construction
        Assert.That(queued.Slot!.State, Is.EqualTo(BuildingState.Operational));

        // Need workers assigned for the recipe to produce.
        session.AutoAssignLabor();

        float before = session.State.Resources.Get("prefab_components");
        session.EndTurn(def.Recipe!.TurnsPerCycle + 1);
        float after  = session.State.Resources.Get("prefab_components");

        Assert.That(after, Is.GreaterThan(before),
            "Operational prefab_factory should produce prefab_components per cycle");
    }

    [Test]
    public void VehicleFactory_GrowsFleetCap()
    {
        var session = NewSession();
        session.State.Resources.Add("steel",      500f);
        session.State.Resources.Add("electronics", 200f);
        session.State.Resources.Add("components", 200f);

        var queued = session.QueueConstruction("vehicle_factory", new GridPosition(0, 0));
        Assert.That(queued.Success, Is.True, queued.FailureReason);

        var def = BuildingRegistry.Get("vehicle_factory");
        session.EndTurn(def.ConstructionTurns); // finish
        Assert.That(queued.Slot!.State, Is.EqualTo(BuildingState.Operational));
        session.AutoAssignLabor();

        int before = session.State.Fleet.TotalSlots;
        session.EndTurn(def.Recipe!.TurnsPerCycle + 1);

        Assert.That(session.State.Fleet.TotalSlots, Is.GreaterThan(before),
            "Vehicle factory should raise fleet cap each completed cycle");
    }

    [Test]
    public void Save_RoundTrip_PreservesPoolsAndAllocations()
    {
        var session = NewSession();
        session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        session.EndTurn(1); // allocate
        int fleetUsed = session.State.Fleet.UsedSlots;
        int opsAlloc  = session.State.Operators.Allocated;
        Assert.That(fleetUsed, Is.GreaterThan(0));

        string json = session.CreateSave();
        var fresh = NewSession();
        fresh.ApplySave(json);

        Assert.That(fresh.State.Fleet.UsedSlots,   Is.EqualTo(fleetUsed));
        Assert.That(fresh.State.Operators.Allocated, Is.EqualTo(opsAlloc));
        var restored = fresh.State.Grid.AllSlots
            .First(s => s.BuildingDefinitionId == "solar_array_mk1");
        Assert.That(restored.AllocatedFleetSlots, Is.GreaterThan(0));
    }

    [Test]
    public void Save_BackwardCompat_PopulatesDefaultsForPre3CSave()
    {
        // Construct a pre-3C save shape: no Fleet / Operators fields, no allocation
        // fields on the building slot. ApplySave should populate from defaults.
        var session = NewSession();
        // Build a minimal save JSON by hand (no Fleet / Operators properties).
        string minimal = """
        {
          "Version": 1,
          "ColonyName": "Test",
          "CurrentSol": 0,
          "Status": "Active",
          "DustStormTurnsRemaining": 0,
          "LowMoraleTurns": 0,
          "ThrivingTurns": 0,
          "GrowthAccumulator": 0,
          "Population": { "Count": 20, "Health": 100, "Morale": 75, "NeedsDeficitTurns": 0, "Skills": {} },
          "Labor": { "TotalWorkers": 15, "Allocations": {}, "SkillSlots": {} },
          "Resources": { "Amounts": {}, "Caps": {} },
          "Buildings": [],
          "Events": []
        }
        """;

        Assert.DoesNotThrow(() => session.ApplySave(minimal));
        Assert.That(session.State.Fleet.BaseSlots, Is.EqualTo(10),
            "Missing Fleet block should restore to SeedDefaults' base capacity");
        Assert.That(session.State.Operators.Total, Is.EqualTo(15));
    }
}
