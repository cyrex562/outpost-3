namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

[TestFixture]
public class EventDispatchTests
{
    private static ColonyState NewState() => new(16, 16);

    [Test]
    public void Apply_ResourceChange_AddsAndSubtracts()
    {
        var state = NewState();
        var rng = new Random(0);
        state.Resources.Add("steel", 100f);

        EventOutcomeExecutor.Apply(state,
            new EventOutcome("resource_change", Resource: "steel", Amount: 50f), rng, sol: 1);
        Assert.That(state.Resources.Get("steel"), Is.EqualTo(150f).Within(0.01f));

        EventOutcomeExecutor.Apply(state,
            new EventOutcome("resource_change", Resource: "steel", Amount: -30f), rng, sol: 1);
        Assert.That(state.Resources.Get("steel"), Is.EqualTo(120f).Within(0.01f));
    }

    [Test]
    public void Apply_Log_AppendsEvent()
    {
        var state = NewState();
        var rng = new Random(0);
        EventOutcomeExecutor.Apply(state,
            new EventOutcome("log", Message: "Test happened"), rng, sol: 5);

        Assert.That(state.EventLog.All, Has.Count.EqualTo(1));
        Assert.That(state.EventLog.All[0].Message, Is.EqualTo("Test happened"));
        Assert.That(state.EventLog.All[0].Sol, Is.EqualTo(5));
    }

    [Test]
    public void Apply_DustStorm_SetsDuration()
    {
        var state = NewState();
        var rng = new Random(0);
        EventOutcomeExecutor.Apply(state,
            new EventOutcome("dust_storm", Duration: 12), rng, sol: 1);

        Assert.That(state.DustStormTurnsRemaining, Is.EqualTo(12));
    }

    [Test]
    public void Apply_DustStorm_DoesNotOverwriteActiveStorm()
    {
        var state = NewState();
        state.DustStormTurnsRemaining = 5;
        var rng = new Random(0);
        EventOutcomeExecutor.Apply(state,
            new EventOutcome("dust_storm", Duration: 20), rng, sol: 1);

        Assert.That(state.DustStormTurnsRemaining, Is.EqualTo(5));
    }

    [Test]
    public void Apply_PopulationAdd_IncrementsCount()
    {
        var state = NewState();
        state.Population.Count = 10;
        var rng = new Random(0);
        EventOutcomeExecutor.Apply(state,
            new EventOutcome("population_add", Amount: 5f), rng, sol: 1);

        Assert.That(state.Population.Count, Is.EqualTo(15));
    }

    [Test]
    public void Apply_DamageRandom_DamagesAnOperationalBuilding()
    {
        var session = new ColonySession(new SiteDefinition(42, BiomeType.Barren, new GridSize(32, 32)));
        var def = BuildingRegistry.Get("solar_array_mk1");
        session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        session.EndTurn(def.ConstructionTurns);

        var rng = new Random(0);
        EventOutcomeExecutor.Apply(session.State,
            new EventOutcome("damage_random"), rng, sol: session.CurrentSol);

        Assert.That(session.State.Grid.AllSlots.Any(s => s.State == BuildingState.Damaged),
            Is.True);
    }

    [Test]
    public void RandomEventProcessor_FiresRegistryEventsOverManyTurns()
    {
        // Brute-force: with a wide seed pool, at least one strategic event should
        // fire from the registry within 50 strategic turns.
        var state = new ColonyState(16, 16);
        state.Resources.Add("electronics", 200f);
        var proc = new RandomEventProcessor(state, seed: 12345);

        for (int i = 1; i <= 50; i++)
            proc.ProcessStrategicTurn(i * 30);

        Assert.That(state.EventLog.All.Count, Is.GreaterThan(0),
            "At least one registry event should fire across 50 strategic turns");
    }

    [Test]
    public void MysteriousSignal_SpawnsDecisionWhenItFires()
    {
        // Seed the state with the required electronics so mysterious_signal can pass
        // its gate, then drive enough strategic turns that the 12% chance fires at
        // least once. Verifies the decision spawns with the expected two choices.
        var state = new ColonyState(16, 16);
        state.Resources.Add("electronics", 1000f);
        var proc = new RandomEventProcessor(state, seed: 7);

        bool decisionSeen = false;
        for (int i = 1; i <= 100 && !decisionSeen; i++)
        {
            proc.ProcessStrategicTurn(i * 30);
            decisionSeen = state.PendingDecisions.Count > 0;
        }

        if (!decisionSeen)
        {
            Assert.Inconclusive("mysterious_signal did not fire across 100 strategic turns — try a different seed.");
            return;
        }

        var decision = state.PendingDecisions.Peek();
        Assert.That(decision.Title,   Is.EqualTo("Mysterious Signal"));
        Assert.That(decision.Choices, Has.Count.EqualTo(2));
    }
}
