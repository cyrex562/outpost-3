using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;
using OutpostGame.Rendering;

namespace OutpostGame.Game;

/// <summary>
/// Phase 2 entry point. Creates a <see cref="ColonySession"/> and binds it to the
/// rendered grid view, the placer, and the HUD. Turn processing is driven by
/// <see cref="ColonySession.EndTurn"/> calls coming from the HUD buttons.
/// </summary>
public partial class ColonyScene : Node2D
{
    [Export] public NodePath GridViewPath { get; set; } = new("GridView");
    [Export] public NodePath CameraPath { get; set; } = new("Camera");
    [Export] public NodePath PlacerPath { get; set; } = new("Placer");
    [Export] public NodePath HudPath { get; set; } = new("Hud");

    [Export] public int Seed { get; set; } = 12345;
    [Export] public int GridWidth { get; set; } = 64;
    [Export] public int GridHeight { get; set; } = 64;

    public ColonySession? Session { get; private set; }

    public override void _Ready()
    {
        var gridView = GetNode<ColonyGridView>(GridViewPath);
        var camera = GetNode<IsometricCamera>(CameraPath);
        var placer = GetNode<BuildingPlacer>(PlacerPath);
        var hud = GetNode<ColonyHud>(HudPath);

        var site = new SiteDefinition(
            Seed,
            BiomeType.Barren,
            new GridSize(GridWidth, GridHeight));

        Session = new ColonySession(site);

        gridView.Bind(Session);
        camera.GridView = gridView;
        placer.GridView = gridView;
        placer.Camera = camera;
        placer.Session = Session;
        hud.Bind(Session, placer);

        // Center the camera on the grid.
        var extents = gridView.WorldExtents();
        camera.Position = extents.Position + extents.Size * 0.5f;
    }
}
