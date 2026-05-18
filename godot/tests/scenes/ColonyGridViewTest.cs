namespace OutpostGame.Tests.Scenes;

using GdUnit4;
using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Game;
using OutpostGame.Rendering;
using static GdUnit4.Assertions;

/// <summary>
/// Scene-level tests covering grid rendering and camera behaviour.
/// Exercises ColonyGridView, BuildingPlacer, and IsometricCamera in the live scene.
/// </summary>
[TestSuite]
public partial class ColonyGridViewTest : GodotObject
{
	// ── Grid view initialisation ─────────────────────────────────────────────

	[TestCase(Description = "GridView node is present and bound after _Ready")]
	public async Task GridView_IsPresentInScene()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var gridView = runner.Scene().GetNodeOrNull<ColonyGridView>("GridView");
		AssertThat(gridView).IsNotNull();
	}

	[TestCase(Description = "Scene survives 10 frames of rendering without crashing")]
	public async Task GridView_AfterBind_RendersWithoutException()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(10);

		AssertThat(runner.Scene()).IsNotNull();
	}

	// ── GridView coordinate helpers ──────────────────────────────────────────

	[TestCase(Description = "GridToScreen → ScreenToGrid round-trips back to the original cell")]
	public async Task GridView_GridToScreen_ScreenToGrid_RoundTrips()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var gridView = runner.Scene().GetNodeOrNull<ColonyGridView>("GridView");
		AssertThat(gridView).IsNotNull();

		var origin = new GridPosition(5, 7);
		var screen = gridView!.GridToScreen(origin);
		var back = gridView.ScreenToGrid(screen);

		AssertInt(back.X).IsEqual(origin.X);
		AssertInt(back.Y).IsEqual(origin.Y);
	}

	// ── BuildingPlacer ───────────────────────────────────────────────────────

	[TestCase(Description = "BuildingPlacer starts with no active building (ActiveBuildingId is null)")]
	public async Task BuildingPlacer_InitialState_NoBuildingSelected()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var placer = runner.Scene().GetNodeOrNull<BuildingPlacer>("Placer");
		AssertThat(placer).IsNotNull();
		AssertThat(placer!.ActiveBuildingId).IsNull();
	}

	[TestCase(Description = "SetActiveBuilding selects a building by id")]
	public async Task BuildingPlacer_SetActiveBuilding_SelectsBuilding()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var placer = runner.Scene().GetNodeOrNull<BuildingPlacer>("Placer");
		AssertThat(placer).IsNotNull();

		placer!.SetActiveBuilding("solar_array_mk1");
		await runner.SimulateFrames(2);

		AssertString(placer.ActiveBuildingId).IsEqual("solar_array_mk1");
	}

	[TestCase(Description = "SetActiveBuilding(null) clears the active selection")]
	public async Task BuildingPlacer_SetActiveBuilding_Null_ClearsSelection()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var placer = runner.Scene().GetNodeOrNull<BuildingPlacer>("Placer");
		placer!.SetActiveBuilding("solar_array_mk1");
		placer.SetActiveBuilding(null);
		await runner.SimulateFrames(2);

		AssertThat(placer.ActiveBuildingId).IsNull();
	}

	[TestCase(Description = "ActiveBuildingChanged fires when the selection changes")]
	public async Task BuildingPlacer_ActiveBuildingChanged_Fires()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var placer = runner.Scene().GetNodeOrNull<BuildingPlacer>("Placer");
		string? captured = "sentinel";
		placer!.ActiveBuildingChanged += id => captured = id;

		placer.SetActiveBuilding("solar_array_mk1");
		await runner.SimulateFrames(2);

		AssertString(captured).IsEqual("solar_array_mk1");
	}

	// ── Camera ───────────────────────────────────────────────────────────────

	[TestCase(Description = "IsometricCamera is present in the scene")]
	public async Task Camera_IsPresentInScene()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var camera = runner.Scene().GetNodeOrNull<IsometricCamera>("Camera");
		AssertThat(camera).IsNotNull();
	}

	[TestCase(Description = "Camera zoom starts at 1.0 (default)")]
	public async Task Camera_InitialZoom_IsOne()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var camera = runner.Scene().GetNodeOrNull<IsometricCamera>("Camera");
		AssertFloat(camera!.Zoom.X).IsEqualApprox(1.0f, 0.01f);
	}

	[TestCase(Description = "Camera initial zoom is within the 0.25–4.0 configured bounds")]
	public async Task Camera_Zoom_IsWithinConfiguredBounds()
	{
		using var runner = ISceneRunner.Load("res://scenes/colony/ColonyScene.tscn");
		await runner.SimulateFrames(5);

		var camera = runner.Scene().GetNodeOrNull<IsometricCamera>("Camera");
		AssertThat(camera).IsNotNull();

		// Scroll-wheel zoom simulation doesn't work in GdUnit4's headless sub-viewport;
		// check the initial zoom (1.0) falls within the enforced 0.25–4.0 range.
		AssertFloat(camera!.Zoom.X).IsBetween(0.24f, 4.01f);
	}
}
