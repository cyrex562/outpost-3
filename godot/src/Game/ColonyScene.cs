using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.World;
using OutpostGame.Rendering;

namespace OutpostGame.Game;

/// <summary>
/// Phase 1 entry point. Wires up GridView, IsometricCamera, and BuildingPlacer.
/// No simulation hook-up yet (Phase 2). Placements are stored on the
/// <see cref="OutpostGame.Core.Colony.ColonyGrid"/> only.
/// </summary>
public partial class ColonyScene : Node2D
{
    [Export] public NodePath GridViewPath { get; set; } = new("GridView");
    [Export] public NodePath CameraPath { get; set; } = new("Camera");
    [Export] public NodePath PlacerPath { get; set; } = new("Placer");

    [Export] public int Seed { get; set; } = 12345;
    [Export] public int GridWidth { get; set; } = 64;
    [Export] public int GridHeight { get; set; } = 64;

    public override void _Ready()
    {
        var gridView = GetNode<ColonyGridView>(GridViewPath);
        var camera = GetNode<IsometricCamera>(CameraPath);
        var placer = GetNode<BuildingPlacer>(PlacerPath);

        var site = new SiteDefinition(
            Seed,
            BiomeType.Barren,
            new GridSize(GridWidth, GridHeight));
        gridView.Initialize(site);

        camera.GridView = gridView;
        placer.GridView = gridView;
        placer.Camera = camera;

        // Center the camera on the grid.
        var extents = gridView.WorldExtents();
        camera.Position = extents.Position + extents.Size * 0.5f;
    }
}
