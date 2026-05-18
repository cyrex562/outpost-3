namespace OutpostGame.Tests.Scenes;

using GdUnit4;
using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Game;
using static GdUnit4.Assertions;

/// <summary>
/// Scene-level tests for ColonyScene. These run inside Godot's runtime via the
/// GdUnit4 editor plugin — open the GdUnit4 inspector panel and click Run All.
/// </summary>
[TestSuite]
public partial class ColonySceneTest : GodotObject
{
    // Use GetNode<ColonyScene>(".") rather than a direct C# cast of runner.Scene()
    // so Godot's instance cache returns the live object (avoids a fresh wrapper
    // with null fields in cases where the managed instance was GC'd between
    // _Ready() and the test assertion).
    private static ColonyScene GetScene(ISceneRunner runner) =>
        runner.Scene().GetNode<ColonyScene>(".");

    // ── Startup ──────────────────────────────────────────────────────────────

    [TestCase(Description = "Scene loads without errors and all child nodes exist")]
    public async Task ColonyScene_Loads_AllNodesPresent()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = runner.Scene();
        AssertThat(scene).IsNotNull();
        AssertThat(scene.GetNodeOrNull("GridView")).IsNotNull();
        AssertThat(scene.GetNodeOrNull("Placer")).IsNotNull();
        AssertThat(scene.GetNodeOrNull("Camera")).IsNotNull();
        AssertThat(scene.GetNodeOrNull("Hud")).IsNotNull();
    }

    [TestCase(Description = "Session is initialised at sol 0 with seed-default resources and workers")]
    public async Task ColonyScene_InitialState_MatchesSeedDefaults()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        AssertThat(scene.Session).IsNotNull();

        var state = scene.Session.State;
        AssertInt(state.TurnManager.CurrentSol).IsEqual(0);
        AssertInt(state.Population.Count).IsEqual(20);
        AssertInt(state.Labor.TotalWorkers).IsEqual(15);
        AssertFloat(state.Resources.Get("steel")).IsGreater(0f);
        AssertFloat(state.Resources.Get("electronics")).IsGreater(0f);
    }

    // ── Turn advancement ─────────────────────────────────────────────────────

    [TestCase(Description = "EndTurn(1) advances the sol counter to 1")]
    public async Task EndTurn_AdvancesSolCounter()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.EndTurn(1);
        await runner.SimulateFrames(2);

        AssertInt(scene.Session.CurrentSol).IsEqual(1);
    }

    [TestCase(Description = "EndTurn fires StateChanged so HUD is updated")]
    public async Task EndTurn_FiresStateChanged_HudRefreshes()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        bool stateChangedFired = false;
        scene.Session.StateChanged += () => stateChangedFired = true;

        scene.Session.EndTurn(1);
        await runner.SimulateFrames(2);

        AssertBool(stateChangedFired).IsTrue();
    }

    // ── Construction placement ───────────────────────────────────────────────

    [TestCase(Description = "QueueConstruction places a slot in UnderConstruction state")]
    public async Task QueueConstruction_PlacesSlot_UnderConstruction()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        var result = scene.Session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        AssertBool(result.Success).IsTrue();
        AssertThat(result.Slot).IsNotNull();
        AssertThat(result.Slot!.State).IsEqual(BuildingState.UnderConstruction);
    }

    [TestCase(Description = "Placing a second building on an occupied cell returns failure")]
    public async Task QueueConstruction_OccupiedCell_Fails()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        var second = scene.Session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        AssertBool(second.Success).IsFalse();
        AssertString(second.FailureReason).IsNotEmpty();
    }

    // ── Construction completion ──────────────────────────────────────────────

    [TestCase(Description = "After ConstructionTurns, the building slot becomes Operational")]
    public async Task Construction_CompletesAfterCorrectTurns()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        int ctTurns = BuildingRegistry.Get("solar_array_mk1").ConstructionTurns;
        scene.Session.EndTurn(ctTurns);
        await runner.SimulateFrames(2);

        var slot = scene.Session.State.Grid.AllSlots.First();
        AssertThat(slot.State).IsEqual(BuildingState.Operational);
    }

    // ── Power grid ───────────────────────────────────────────────────────────

    [TestCase(Description = "Completed solar array registers power production")]
    public async Task SolarArray_WhenOperational_RegistersPowerProduction()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        scene.Session.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);
        await runner.SimulateFrames(2);

        AssertFloat(scene.Session.State.Power.TotalCapacity).IsGreater(0f);
    }

    [TestCase(Description = "Without a generator, all non-essential buildings report unpowered")]
    public async Task WithoutGenerator_NonEssentialBuildings_AreUnpowered()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.QueueConstruction("iron_mine", new GridPosition(0, 0));
        scene.Session.EndTurn(BuildingRegistry.Get("iron_mine").ConstructionTurns);
        await runner.SimulateFrames(2);

        var slot = scene.Session.State.Grid.AllSlots.First();
        AssertBool(scene.Session.State.Power.IsPowered(slot.Id)).IsFalse();
    }

    // ── Building repair ──────────────────────────────────────────────────────

    [TestCase(Description = "RepairBuilding restores a damaged slot to Operational")]
    public async Task RepairBuilding_RestoresDamagedSlotToOperational()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.QueueConstruction("nutrient_recycler", new GridPosition(0, 0));
        scene.Session.EndTurn(BuildingRegistry.Get("nutrient_recycler").ConstructionTurns);

        var slot = scene.Session.State.Grid.AllSlots.First();
        slot.State = BuildingState.Damaged;
        scene.Session.State.Power.Unregister(slot.Id);

        bool ok = scene.Session.RepairBuilding(slot.Id);
        await runner.SimulateFrames(2);

        AssertBool(ok).IsTrue();
        AssertThat(slot.State).IsEqual(BuildingState.Operational);
    }

    [TestCase(Description = "RepairBuilding returns false when steel is insufficient")]
    public async Task RepairBuilding_FailsWithInsufficientResources()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));
        scene.Session.EndTurn(BuildingRegistry.Get("solar_array_mk1").ConstructionTurns);

        var slot = scene.Session.State.Grid.AllSlots.First();
        slot.State = BuildingState.Damaged;
        scene.Session.State.Power.Unregister(slot.Id);

        // Drain all steel so repair cost cannot be met.
        scene.Session.State.Resources.TryConsume("steel",
            scene.Session.State.Resources.Get("steel"));

        bool ok = scene.Session.RepairBuilding(slot.Id);
        await runner.SimulateFrames(2);

        AssertBool(ok).IsFalse();
        AssertThat(slot.State).IsEqual(BuildingState.Damaged);
    }

    // ── Event log ────────────────────────────────────────────────────────────

    [TestCase(Description = "Queuing construction appends a 'Construction started' event")]
    public async Task QueueConstruction_AddsEventLogEntry()
    {
        using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
        await runner.SimulateFrames(5);

        var scene = GetScene(runner);
        scene.Session.QueueConstruction("solar_array_mk1", new GridPosition(0, 0));

        var log = scene.Session.State.EventLog.All;
        AssertThat(log).IsNotEmpty();
        AssertBool(log.Any(e => e.Message.Contains("Construction started"))).IsTrue();
    }
}
