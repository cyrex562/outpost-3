using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

namespace OutpostGame.Rendering;

/// <summary>
/// Renders the colony grid as an iso-projected field of diamond tiles.
///
/// Phase 2 binds the view to an externally-owned <see cref="ColonySession"/>: terrain
/// is generated locally from the <see cref="SiteDefinition"/>, but the
/// <see cref="ColonyGrid"/> reference comes from <see cref="ColonyState"/> so the
/// simulation and the renderer agree on a single source of truth.
/// </summary>
public partial class ColonyGridView : Node2D
{
    public int Width { get; private set; } = 64;
    public int Height { get; private set; } = 64;
    public int Facing { get; private set; }

    private TerrainType[,] _terrain = new TerrainType[0, 0];
    private ColonyGrid _grid = new(0, 0);
    private ColonySession? _session;
    private BiomeType _biome = BiomeType.Barren;

    private Font? _font;

    public ColonyGrid Grid => _grid;
    public TerrainType[,] Terrain => _terrain;
    public ColonySession? Session => _session;

    public override void _Ready()
    {
        _font = ThemeDB.FallbackFont;
        if (_terrain.GetLength(0) == 0)
        {
            // Fallback: stand-alone preview without a session (used by the .tscn editor).
            Initialize(SiteDefinition.Default());
        }
    }

    /// <summary>Phase-1 entry point. Generates terrain and builds an internal grid;
    /// kept so the scene can render before a session is wired in.</summary>
    public void Initialize(SiteDefinition site)
    {
        Width = site.Size.Width;
        Height = site.Size.Height;
        _biome = site.Biome;
        _terrain = TerrainGenerator.Generate(site.Seed, Width, Height, site.Biome);
        _grid = new ColonyGrid(Width, Height);
        _session = null;
        QueueRedraw();
    }

    /// <summary>Phase-2 entry point. Binds to a session whose <see cref="ColonyGrid"/>
    /// becomes the rendered grid; subscribes to <see cref="ColonySession.StateChanged"/>
    /// so the view repaints when the simulation updates.</summary>
    public void Bind(ColonySession session)
    {
        if (_session != null)
            _session.StateChanged -= OnSessionStateChanged;

        _session = session;
        Width = session.Site.Size.Width;
        Height = session.Site.Size.Height;
        _biome = session.Site.Biome;
        _terrain = TerrainGenerator.Generate(session.Site.Seed, Width, Height, session.Site.Biome);
        _grid = session.State.Grid;
        session.StateChanged += OnSessionStateChanged;
        QueueRedraw();
    }

    private void OnSessionStateChanged() => QueueRedraw();

    public void SetFacing(int facing)
    {
        Facing = ((facing % 4) + 4) % 4;
        QueueRedraw();
    }

    public void CycleFacing(int delta = 1)
    {
        SetFacing(Facing + delta);
    }

    public Vector2 GridToScreen(GridPosition gp) =>
        IsoProjection.GridToScreen(gp, Width, Height, Facing);

    public GridPosition ScreenToGrid(Vector2 localPos) =>
        IsoProjection.ScreenToGrid(localPos, Width, Height, Facing);

    public TerrainType TerrainAt(GridPosition gp)
    {
        if (gp.X < 0 || gp.X >= Width || gp.Y < 0 || gp.Y >= Height)
            return TerrainType.Impassable;
        return _terrain[gp.X, gp.Y];
    }

    public bool InBounds(GridPosition gp) =>
        gp.X >= 0 && gp.X < Width && gp.Y >= 0 && gp.Y < Height;

    public Rect2 WorldExtents()
    {
        var c00 = IsoProjection.GridToScreen(new GridPosition(0, 0), Width, Height, Facing);
        var c10 = IsoProjection.GridToScreen(new GridPosition(Width - 1, 0), Width, Height, Facing);
        var c01 = IsoProjection.GridToScreen(new GridPosition(0, Height - 1), Width, Height, Facing);
        var c11 = IsoProjection.GridToScreen(new GridPosition(Width - 1, Height - 1), Width, Height, Facing);

        float minX = Mathf.Min(Mathf.Min(c00.X, c10.X), Mathf.Min(c01.X, c11.X)) - IsoProjection.TileWidth;
        float maxX = Mathf.Max(Mathf.Max(c00.X, c10.X), Mathf.Max(c01.X, c11.X)) + IsoProjection.TileWidth;
        float minY = Mathf.Min(Mathf.Min(c00.Y, c10.Y), Mathf.Min(c01.Y, c11.Y)) - IsoProjection.TileHeight;
        float maxY = Mathf.Max(Mathf.Max(c00.Y, c10.Y), Mathf.Max(c01.Y, c11.Y)) + IsoProjection.TileHeight;

        return new Rect2(minX, minY, maxX - minX, maxY - minY);
    }

    public override void _Draw()
    {
        if (Width == 0 || Height == 0) return;

        // 1) Terrain tiles in back-to-front rotated-grid order.
        for (int ry = 0; ry < Height; ry++)
        {
            for (int rx = 0; rx < Width; rx++)
            {
                var (gx, gy) = IsoProjection.RemoveFacing(rx, ry, Width, Height, Facing);
                if (gx < 0 || gx >= Width || gy < 0 || gy >= Height) continue;

                var terrain = _terrain[gx, gy];
                Color color = ColorForTerrain(terrain, _biome);

                if (_grid.IsCellOccupied(new GridPosition(gx, gy)))
                {
                    color = color.Lerp(new Color(0.3f, 0.4f, 0.9f), 0.4f);
                }

                var corners = IsoProjection.DiamondCorners(rx, ry);
                DrawColoredPolygon(corners, color);

                DrawPolyline(new[] { corners[0], corners[1], corners[2], corners[3], corners[0] },
                    new Color(0, 0, 0, 0.25f), 1f);
            }
        }

        // 2) Building footprints, color-coded by lifecycle state.
        foreach (var slot in _grid.AllSlots)
        {
            DrawBuildingSlot(slot);
        }
    }

    private void DrawBuildingSlot(BuildingSlot slot)
    {
        int minRx = int.MaxValue, minRy = int.MaxValue, maxRx = int.MinValue, maxRy = int.MinValue;
        for (int dx = 0; dx < slot.Size.Width; dx++)
        {
            for (int dy = 0; dy < slot.Size.Height; dy++)
            {
                var (rx, ry) = IsoProjection.ApplyFacing(
                    slot.Origin.X + dx, slot.Origin.Y + dy, Width, Height, Facing);
                if (rx < minRx) minRx = rx;
                if (ry < minRy) minRy = ry;
                if (rx > maxRx) maxRx = rx;
                if (ry > maxRy) maxRy = ry;
            }
        }

        Vector2 top = IsoProjection.GridToScreen(minRx, minRy) + new Vector2(0, -IsoProjection.TileHeight * 0.5f);
        Vector2 right = IsoProjection.GridToScreen(maxRx, minRy) + new Vector2(IsoProjection.TileWidth * 0.5f, 0);
        Vector2 bottom = IsoProjection.GridToScreen(maxRx, maxRy) + new Vector2(0, IsoProjection.TileHeight * 0.5f);
        Vector2 left = IsoProjection.GridToScreen(minRx, maxRy) + new Vector2(-IsoProjection.TileWidth * 0.5f, 0);
        Vector2[] poly = new[] { top, right, bottom, left };

        Color baseColor = ColorForBuilding(slot.BuildingDefinitionId);
        (Color fill, Color outline) = StyleForState(baseColor, slot.State);

        DrawColoredPolygon(poly, fill);
        DrawPolyline(new[] { top, right, bottom, left, top }, outline, 2f);

        // Construction-progress sliver at the bottom of the footprint.
        if (slot.State == BuildingState.UnderConstruction
            && BuildingRegistry.TryGet(slot.BuildingDefinitionId, out var def)
            && def is not null && def.ConstructionTurns > 0)
        {
            int total = def.ConstructionTurns;
            int done = total - slot.ConstructionTurnsRemaining;
            float pct = Math.Clamp(done / (float)total, 0f, 1f);
            DrawProgressBar(top, right, bottom, left, pct);
        }

        if (_font != null)
        {
            Vector2 center = (top + bottom) * 0.5f;
            string label = LabelFor(slot);
            var size = _font.GetStringSize(label);
            DrawString(_font, center - size * 0.5f + new Vector2(0, size.Y * 0.25f),
                label, HorizontalAlignment.Left, -1, 12, new Color(1, 1, 1));
        }
    }

    private static (Color fill, Color outline) StyleForState(Color baseColor, BuildingState state) => state switch
    {
        BuildingState.Planned =>
            (new Color(baseColor.R, baseColor.G, baseColor.B, 0.35f),
             new Color(0.9f, 0.9f, 0.9f, 0.6f)),
        BuildingState.UnderConstruction =>
            (Tint(baseColor.Lerp(new Color(1f, 0.7f, 0.2f), 0.55f), 0.65f),
             new Color(0.95f, 0.6f, 0.1f, 0.95f)),
        BuildingState.Operational =>
            (baseColor, new Color(0, 0, 0, 0.85f)),
        BuildingState.Powered =>
            (baseColor, new Color(0.2f, 1f, 0.4f, 0.95f)),
        BuildingState.Unpowered =>
            (baseColor.Lerp(new Color(0.4f, 0.4f, 0.45f), 0.6f),
             new Color(0.7f, 0.7f, 0.7f, 0.9f)),
        BuildingState.Damaged =>
            (baseColor.Lerp(new Color(1f, 0.2f, 0.2f), 0.55f),
             new Color(0.9f, 0.1f, 0.1f, 0.95f)),
        BuildingState.Destroyed =>
            (new Color(0.18f, 0.05f, 0.05f, 0.7f), new Color(0.5f, 0.05f, 0.05f, 0.9f)),
        _ => (baseColor, new Color(0, 0, 0, 0.8f)),
    };

    private void DrawProgressBar(Vector2 top, Vector2 right, Vector2 bottom, Vector2 left, float pct)
    {
        // Horizontal bar centred under the building footprint. Using the diamond's
        // edge midpoints (the old approach) slopes for non-square footprints like
        // the Ice Drill (2×3) — driving a horizontal line off the centre X keeps
        // the bar level regardless of footprint shape.
        float centerX = (left.X + right.X) * 0.5f;
        float barY    = bottom.Y - IsoProjection.TileHeight * 0.30f;
        float halfLen = Math.Max(IsoProjection.TileWidth * 0.6f,
                                  (right.X - left.X) * 0.35f);

        Vector2 start   = new Vector2(centerX - halfLen, barY);
        Vector2 end     = new Vector2(centerX + halfLen, barY);
        Vector2 fillEnd = start.Lerp(end, pct);

        DrawLine(start, end,    new Color(0, 0, 0, 0.75f), 9f);
        DrawLine(start, fillEnd, new Color(0.4f, 1f, 0.4f, 0.95f), 9f);
    }

    private static string LabelFor(BuildingSlot slot)
    {
        string name = BuildingRegistry.TryGet(slot.BuildingDefinitionId, out var def) && def is not null
            ? def.DisplayName
            : slot.BuildingDefinitionId;

        return slot.State switch
        {
            BuildingState.UnderConstruction =>
                BuildingRegistry.TryGet(slot.BuildingDefinitionId, out var d) && d is not null
                    ? $"{name} ({d.ConstructionTurns - slot.ConstructionTurnsRemaining}/{d.ConstructionTurns})"
                    : $"{name} (building)",
            BuildingState.Planned => $"{name} (planned)",
            BuildingState.Damaged => $"{name} (damaged)",
            BuildingState.Destroyed => $"{name} (destroyed)",
            _ => name,
        };
    }

    public static Color ColorForTerrain(TerrainType t, BiomeType biome = BiomeType.Barren) => biome switch
    {
        BiomeType.Rocky            => t switch
        {
            TerrainType.Flat      => new Color(0.42f, 0.37f, 0.32f),
            TerrainType.Rough     => new Color(0.27f, 0.22f, 0.17f),
            TerrainType.Slope     => new Color(0.35f, 0.30f, 0.25f),
            TerrainType.Crater    => new Color(0.18f, 0.14f, 0.11f),
            TerrainType.Impassable=> new Color(0.07f, 0.05f, 0.04f),
            _                     => new Color(0.40f, 0.35f, 0.30f),
        },
        BiomeType.Polar            => t switch
        {
            TerrainType.Flat      => new Color(0.82f, 0.88f, 0.95f),
            TerrainType.Rough     => new Color(0.60f, 0.68f, 0.80f),
            TerrainType.Slope     => new Color(0.50f, 0.62f, 0.82f),
            TerrainType.Crater    => new Color(0.65f, 0.72f, 0.85f),
            TerrainType.Impassable=> new Color(0.18f, 0.22f, 0.32f),
            _                     => new Color(0.70f, 0.78f, 0.90f),
        },
        BiomeType.Desert           => t switch
        {
            TerrainType.Flat      => new Color(0.80f, 0.65f, 0.38f),
            TerrainType.Rough     => new Color(0.58f, 0.44f, 0.24f),
            TerrainType.Slope     => new Color(0.65f, 0.50f, 0.30f),
            TerrainType.Crater    => new Color(0.44f, 0.30f, 0.18f),
            TerrainType.Impassable=> new Color(0.18f, 0.12f, 0.07f),
            _                     => new Color(0.70f, 0.56f, 0.35f),
        },
        BiomeType.Volcanic         => t switch
        {
            TerrainType.Flat      => new Color(0.25f, 0.20f, 0.20f),
            TerrainType.Rough     => new Color(0.18f, 0.13f, 0.13f),
            TerrainType.Slope     => new Color(0.22f, 0.16f, 0.14f),
            TerrainType.Crater    => new Color(0.35f, 0.10f, 0.05f),
            TerrainType.Impassable=> new Color(0.08f, 0.03f, 0.03f),
            _                     => new Color(0.20f, 0.15f, 0.15f),
        },
        BiomeType.MarginalHabitable=> t switch
        {
            TerrainType.Flat      => new Color(0.42f, 0.52f, 0.38f),
            TerrainType.Rough     => new Color(0.28f, 0.34f, 0.23f),
            TerrainType.Slope     => new Color(0.35f, 0.42f, 0.30f),
            TerrainType.Crater    => new Color(0.22f, 0.26f, 0.18f),
            TerrainType.Impassable=> new Color(0.06f, 0.08f, 0.05f),
            _                     => new Color(0.38f, 0.46f, 0.33f),
        },
        _ /* Barren */             => t switch
        {
            TerrainType.Flat      => new Color(0.55f, 0.55f, 0.55f),
            TerrainType.Rough     => new Color(0.35f, 0.35f, 0.35f),
            TerrainType.Slope     => new Color(0.45f, 0.45f, 0.40f),
            TerrainType.Crater    => new Color(0.25f, 0.20f, 0.20f),
            TerrainType.Impassable=> new Color(0.05f, 0.05f, 0.05f),
            _                     => new Color(0.50f, 0.50f, 0.50f),
        },
    };

    private static Color ColorForBuilding(string id)
    {
        int h = id.GetHashCode();
        float hue = ((h & 0xFFFF) / 65535f);
        return Color.FromHsv(hue, 0.55f, 0.85f);
    }

    private static Color Tint(Color c, float alpha) => new(c.R, c.G, c.B, alpha);
}
