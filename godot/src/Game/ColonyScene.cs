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
/// Save files are stored in slots under <c>user://saves/</c> via
/// <see cref="SaveSlotManager"/>.
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

    /// <summary>Last slot used for a manual save — Ctrl+S quicksaves use this if set,
    /// otherwise falls back to the quicksave slot.</summary>
    public string LastUsedSlotId { get; private set; } = SaveSlotManager.QuicksaveId;

    /// <summary>Autosave cadence in sols.</summary>
    public const int AutosaveInterval = 10;

    // Lazy-initialised so the property is always non-null even when accessed
    // from a GdUnit4 sub-viewport where Godot lifecycle callbacks (_EnterTree /
    // _Ready) may fire on a different managed-object instance than the one
    // returned by ISceneRunner.Scene().
    private ColonySession? _session;
    public ColonySession Session => _session ??= NewSession();

    private ColonySession NewSession()
    {
        var site = GameContext.ConsumePendingSite()
            ?? new SiteDefinition(Seed, BiomeType.Barren, new GridSize(GridWidth, GridHeight));
        var session = new ColonySession(site);

        var name = GameContext.ConsumePendingColonyName();
        if (!string.IsNullOrWhiteSpace(name))
            session.State.Name = name;

        session.TurnAdvanced += OnTurnAdvancedAutosave;
        return session;
    }

    public override void _Ready()
    {
        // Make sure registries are populated from res://content/ before any
        // session creation reads BuildingRegistry / ResourceRegistry.
        ContentBootstrap.EnsureLoaded();

        // Touch the getter so the session is created with the final exported
        // property values (Seed / GridWidth / GridHeight) before binding.
        _ = Session;

        var gridView = GetNode<ColonyGridView>(GridViewPath);
        var camera = GetNode<IsometricCamera>(CameraPath);
        var placer = GetNode<BuildingPlacer>(PlacerPath);
        var hud = GetNode<ColonyHud>(HudPath);

        gridView.Bind(Session);
        camera.GridView = gridView;
        placer.GridView = gridView;
        placer.Camera = camera;
        placer.Session = Session;
        hud.Bind(Session, placer);

        // Center the camera on the grid.
        var extents = gridView.WorldExtents();
        camera.Position = extents.Position + extents.Size * 0.5f;

        // If the main menu requested a save load (Continue or slot selection),
        // apply it now.
        var slotToLoad = GameContext.LoadSlotOnReady;
        if (!string.IsNullOrEmpty(slotToLoad))
        {
            GameContext.LoadSlotOnReady = null;
            LoadFromSlot(slotToLoad);
            LastUsedSlotId = slotToLoad;
        }
    }

    public bool SaveToSlot(string slotId)
    {
        bool ok = SaveSlotManager.Save(slotId, Session);
        if (ok) LastUsedSlotId = slotId;
        return ok;
    }

    public bool LoadFromSlot(string slotId)
    {
        bool ok = SaveSlotManager.Load(slotId, Session);
        if (ok) LastUsedSlotId = slotId;
        return ok;
    }

    public bool QuickSave() => SaveToSlot(SaveSlotManager.QuicksaveId);

    public bool DeleteSlot(string slotId) => SaveSlotManager.Delete(slotId);

    private void OnTurnAdvancedAutosave(int sol)
    {
        if (sol <= 0) return;
        if (sol % AutosaveInterval != 0) return;
        // Skip autosave on terminal states to avoid spamming a dead save.
        if (Session.State.Status is ColonyStatus.Destroyed or ColonyStatus.Abandoned) return;
        SaveSlotManager.Save(SaveSlotManager.AutosaveId, Session);
    }
}
