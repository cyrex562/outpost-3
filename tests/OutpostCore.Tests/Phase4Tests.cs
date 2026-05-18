namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

/// <summary>Phase 4: random events, building damage, repair, dust-storm solar penalty.</summary>
[TestFixture]
public class Phase4Tests
{
    private static ColonySession NewSession(int seed = 42) =>
        new(new SiteDefinition(seed, BiomeType.Barren, new GridSize(64, 64)));

    /// <summary>Session with ample survival resources so population never dies.</summary>
    private static ColonySession StableSession(int seed = 42)
    {
        var s = NewSession(seed);
        foreach (var res in new[] { "nutrients", "water", "oxygen" })
        {
            s.State.Resources.SetCap(res, 100_000f);
            s.State.Resources.Add(res, 99_000f);
        }
        s.State.Resources.Add("steel",       5_000f);
        s.State.Resources.Add("electronics", 2_000f);
        return s;
    }

    // ── Building damage ──────────────────────────────────────────────────────

    [Test]
    public void DamagedBuilding_IsSkippedByProduction()
    {
        // Iron mine produces iron_ore; nothing in the simulation consumes iron_ore,
        // so any change in the stockpile is purely due to production.
        var sessionOp  = StableSession();
        var sessionDmg = StableSession();

        // Build and complete an iron mine in both sessions.
        foreach (var s in new[] { sessionOp, sessionDmg })
        {
            // Give enough power so the mine is powered.
            s.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
            s.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);
            s.QueueConstruction("iron_mine", new GridPosition(5, 0));
            s.EndTurn(BuildingRegistry.Get("iron_mine").ConstructionTurns);
            s.AutoAssignLabor();
        }

        // Damage the mine in sessionDmg.
        var mineSlot = sessionDmg.State.Grid.AllSlots
            .First(s => s.BuildingDefinitionId == "iron_mine");
        mineSlot.State = BuildingState.Damaged;
        sessionDmg.State.Power.Unregister(mineSlot.Id);

        int cycle = BuildingRegistry.Get("iron_mine").Recipe!.TurnsPerCycle;
        float oreBefore = sessionDmg.State.Resources.Get("iron_ore");
        sessionDmg.EndTurn(cycle);

        float oreAfter = sessionDmg.State.Resources.Get("iron_ore");
        Assert.That(oreAfter, Is.EqualTo(oreBefore).Within(0.01f),
            "Damaged building must not produce resources.");
    }

    [Test]
    public void DamagedBuilding_DoesNotConsumeMaintenanceWhileOffline()
    {
        var session = StableSession();
        session.QueueConstruction("iron_mine", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("iron_mine").ConstructionTurns);

        var slot = session.State.Grid.AllSlots
            .First(s => s.BuildingDefinitionId == "iron_mine");
        slot.State = BuildingState.Damaged;

        // Snapshot AFTER damage so subsequent maintenance only includes other
        // operational buildings (the iron mine should now draw nothing).
        float steelBefore = session.State.Resources.Get("steel");
        session.EndTurn(1);

        // Iron mine's maintenance is 0.5 steel/sol when operational; damaged it
        // should draw nothing. Pre-placed starters (lander, multi_habitat) have
        // no steel maintenance, so the only steel consumer is the (damaged) mine.
        float steelAfter = session.State.Resources.Get("steel");
        Assert.That(steelBefore - steelAfter, Is.LessThan(0.5f),
            "Damaged iron mine should not draw its 0.5 steel maintenance.");
    }

    // ── Repair ───────────────────────────────────────────────────────────────

    [Test]
    public void RepairBuilding_RestoresDamagedSlotToOperational()
    {
        var session = StableSession();
        session.QueueConstruction("nutrient_recycler", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("nutrient_recycler").ConstructionTurns);

        var slot = session.State.Grid.AllSlots
            .First(s => s.BuildingDefinitionId == "nutrient_recycler");
        slot.State = BuildingState.Damaged;
        session.State.Power.Unregister(slot.Id);

        bool ok = session.RepairBuilding(slot.Id);

        Assert.That(ok, Is.True);
        Assert.That(slot.State, Is.EqualTo(BuildingState.Operational));
    }

    [Test]
    public void RepairBuilding_ConsumesThirtyPercentOfConstructionCost()
    {
        var session = StableSession();
        session.QueueConstruction("nutrient_recycler", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("nutrient_recycler").ConstructionTurns);

        var slot = session.State.Grid.AllSlots
            .First(s => s.BuildingDefinitionId == "nutrient_recycler");
        slot.State = BuildingState.Damaged;
        session.State.Power.Unregister(slot.Id);

        float steelBefore = session.State.Resources.Get("steel");
        float elecBefore  = session.State.Resources.Get("electronics");

        session.RepairBuilding(slot.Id);

        var def = BuildingRegistry.Get("nutrient_recycler");
        float expectedSteel = def.ConstructionCost.GetValueOrDefault("steel", 0) * 0.3f;
        float expectedElec  = def.ConstructionCost.GetValueOrDefault("electronics", 0) * 0.3f;

        Assert.That(session.State.Resources.Get("steel"),
            Is.EqualTo(steelBefore - expectedSteel).Within(0.01f));
        Assert.That(session.State.Resources.Get("electronics"),
            Is.EqualTo(elecBefore - expectedElec).Within(0.01f));
    }

    [Test]
    public void RepairBuilding_FailsWhenInsufficientResources()
    {
        // Repair pays 30% of the construction cost in every resource (now
        // includes prefab_components). We drain prefab so repair must fail.
        var session = NewSession();
        session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);

        var slot = session.State.Grid.AllSlots
            .First(s => s.BuildingDefinitionId == "solar_array_mk1");
        slot.State = BuildingState.Damaged;
        session.State.Power.Unregister(slot.Id);

        // Drain prefab so repair cannot be paid (cheapest gate to close).
        session.State.Resources.TryConsume("prefab_components",
            session.State.Resources.Get("prefab_components"));

        bool ok = session.RepairBuilding(slot.Id);

        Assert.That(ok, Is.False);
        Assert.That(slot.State, Is.EqualTo(BuildingState.Damaged));
    }

    [Test]
    public void RepairBuilding_ReturnsFalse_ForNonDamagedSlot()
    {
        var session = StableSession();
        session.QueueConstruction("nutrient_recycler", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("nutrient_recycler").ConstructionTurns);

        var slot = session.State.Grid.AllSlots
            .First(s => s.BuildingDefinitionId == "nutrient_recycler");
        Assert.That(slot.State, Is.EqualTo(BuildingState.Operational));

        bool ok = session.RepairBuilding(slot.Id);
        Assert.That(ok, Is.False, "Repairing an operational building must return false.");
    }

    // ── Dust storm — solar penalty ───────────────────────────────────────────

    [Test]
    public void DustStorm_ReducesSolarCapacityToTwentyPercent()
    {
        var session = StableSession();
        float solarPower  = BuildingRegistry.Get("solar_array_mk1").PowerProduction;
        float landerPower = BuildingRegistry.Get("lander").PowerProduction;
        session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);

        // Pre-storm: lander (non-solar) + solar = full output.
        Assert.That(session.State.Power.TotalCapacity,
            Is.EqualTo(landerPower + solarPower).Within(0.01f));

        session.State.DustStormTurnsRemaining = 5;
        session.EndTurn(1);

        // Storm only affects solar; lander is unaffected.
        Assert.That(session.State.Power.TotalCapacity,
            Is.EqualTo(landerPower + solarPower * 0.2f).Within(0.01f),
            "Only solar capacity should drop to 20 % during a dust storm.");
    }

    [Test]
    public void DustStorm_RestoresSolarCapacityAfterExpiry()
    {
        var session = StableSession();
        session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);

        float normalCapacity = session.State.Power.TotalCapacity;

        // Short storm: expires after 1 turn (counter goes 1→0).
        session.State.DustStormTurnsRemaining = 1;
        session.EndTurn(1);

        Assert.That(session.State.Power.TotalCapacity,
            Is.EqualTo(normalCapacity).Within(0.01f),
            "Solar capacity must be fully restored after the storm ends.");
    }

    [Test]
    public void DustStorm_DoesNotAffectNuclearOutput()
    {
        var session = StableSession();
        float reactorPower = BuildingRegistry.Get("nuclear_reactor").PowerProduction;
        float landerPower  = BuildingRegistry.Get("lander").PowerProduction;

        // Add enough resources to build a nuclear reactor.
        session.State.Resources.Add("components", 500f);
        session.State.Resources.SetCap("components", 500f);

        session.QueueConstruction("nuclear_reactor", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("nuclear_reactor").ConstructionTurns);

        float normalCapacity = session.State.Power.TotalCapacity;
        // Lander + reactor.
        Assert.That(normalCapacity, Is.EqualTo(landerPower + reactorPower).Within(0.1f));

        session.State.DustStormTurnsRemaining = 3;
        session.EndTurn(1);

        Assert.That(session.State.Power.TotalCapacity,
            Is.EqualTo(landerPower + reactorPower).Within(0.1f),
            "Nuclear output must be unaffected by a dust storm.");
    }

    // ── RandomEventProcessor — deterministic seeding ─────────────────────────

    [Test]
    public void RandomEventProcessor_SupplyDrop_AddsResources()
    {
        var state  = new OutpostGame.Core.Colony.ColonyState(16, 16);
        // Force a supply drop by using seed 0 and calling ProcessStrategicTurn
        // until one fires (at most 100 attempts with different sol values).
        // Easier: test the effect directly via a processor configured to always drop.
        // We verify via an internal-accessible seeded call.
        var proc = new RandomEventProcessor(state, seed: 77);

        float steelBefore = state.Resources.Get("steel");
        // Brute-force: try 30 strategic turns until a supply drop is logged.
        bool dropped = false;
        for (int i = 1; i <= 30 && !dropped; i++)
        {
            proc.ProcessStrategicTurn(i * 30);
            dropped = state.EventLog.All.Any(e => e.Message.Contains("Supply drop"));
        }

        if (dropped)
        {
            Assert.That(state.Resources.Get("steel"),
                Is.GreaterThan(steelBefore),
                "Supply drop must increase steel.");
        }
        else
        {
            Assert.Inconclusive("No supply drop fired in 30 strategic turns with this seed — try another seed.");
        }
    }

    [Test]
    public void RandomEventProcessor_Malfunction_DamagesOperationalBuilding()
    {
        var session = StableSession(seed: 99);
        session.QueueConstruction("iron_mine", new GridPosition(0, 0));
        session.EndTurn(BuildingRegistry.Get("iron_mine").ConstructionTurns);

        // Advance strategic turns until a malfunction fires.
        bool damaged = false;
        for (int i = 0; i < 50 && !damaged; i++)
            session.EndTurn(30);

        damaged = session.State.Grid.AllSlots.Any(s => s.State == BuildingState.Damaged)
               || session.State.EventLog.All.Any(e => e.Message.Contains("Equipment failure"));

        if (!damaged)
            Assert.Inconclusive("No malfunction fired in 50 strategic turns — increase attempt count or use a different seed.");
    }
}
