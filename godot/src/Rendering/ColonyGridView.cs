using Godot;
using OutpostGame.Core.Colony;
using OutpostGame.Core.World;

namespace OutpostGame.Rendering;

/// <summary>
/// Renders the colony grid as an iso-projected field of diamond tiles.
///
/// Rendering approach (chosen for Phase 1 because Godot is unavailable at write time
/// and this requires no asset pipeline): a single Node2D whose `_Draw()` paints each
/// cell as a colored diamond polygon. Buildings are drawn after tiles, back-to-front
/// in rotated-grid Y order, with a centered "WxH ID" text label. This satisfies the
/// 1.4 sprite-sorting requirement without YSort scene nodes.
///
/// "View rotation" is implemented by re-projecting grid coordinates through one of
/// four facing transforms — see IsoProjection.cs. The actual Camera2D never rotates.
/// </summary>
public partial class ColonyGridView : Node2D
{
    public int Width { get; private set; } = 64;
    public int Height { get; private set; } = 64;
    public int Facing { get; private set; }

    private TerrainType[,] _terrain = new TerrainType[0, 0];
    private ColonyGrid _grid = new(0, 0);

    // Cached font for label drawing.
    private Font? _font;

    public ColonyGrid Grid => _grid;
    public TerrainType[,] Terrain => _terrain;

    public override void _Ready()
    {
        _font = ThemeDB.FallbackFont;
        if (_terrain.GetLength(0) == 0)
        {
            Initialize(SiteDefinition.Default());
        }
    }

    public void Initialize(SiteDefinition site)
    {
        Width = site.Size.Width;
        Height = site.Size.Height;
        _terrain = TerrainGenerator.Generate(site.Seed, Width, Height);
        _grid = new ColonyGrid(Width, Height);
        QueueRedraw();
    }

    public void SetFacing(int facing)
    {
        Facing = ((facing % 4) + 4) % 4;
        QueueRedraw();
    }

    public void CycleFacing(int delta = 1)
    {
        SetFacing(Facing + delta);
    }

    /// <summary>Convert a model grid position to local screen coords (this Node2D space).</summary>
    public Vector2 GridToScreen(GridPosition gp) =>
        IsoProjection.GridToScreen(gp, Width, Height, Facing);

    /// <summary>Convert a local-space world position to a model grid position.</summary>
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
        // Compute the bounding box of all four grid corners projected to screen space.
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

        // 1) Draw terrain tiles in back-to-front order of the rotated grid (sum rx+ry).
        // Iterate model space but sort by rotated y for correct overlap.
        for (int ry = 0; ry < Height; ry++)
        {
            for (int rx = 0; rx < Width; rx++)
            {
                // Inverse-rotate to find which model cell sits at this rotated coord.
                var (gx, gy) = IsoProjection.RemoveFacing(rx, ry, Width, Height, Facing);
                if (gx < 0 || gx >= Width || gy < 0 || gy >= Height) continue;

                var terrain = _terrain[gx, gy];
                Color color = ColorForTerrain(terrain);

                bool occupied = _grid.IsCellOccupied(new GridPosition(gx, gy));
                if (occupied)
                {
                    // Tint blue when a building sits here.
                    color = color.Lerp(new Color(0.3f, 0.4f, 0.9f), 0.5f);
                }

                var corners = IsoProjection.DiamondCorners(rx, ry);
                DrawColoredPolygon(corners, color);

                // Thin outline for readability.
                DrawPolyline(new[] { corners[0], corners[1], corners[2], corners[3], corners[0] },
                    new Color(0, 0, 0, 0.25f), 1f);
            }
        }

        // 2) Draw building footprints and labels.
        foreach (var slot in _grid.AllSlots)
        {
            DrawBuildingSlot(slot);
        }
    }

    private void DrawBuildingSlot(BuildingSlot slot)
    {
        // Find the bounding diamond of the slot's cells (in rotated space).
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

        // Build a diamond polygon spanning the rotated bounding box.
        Vector2 top = IsoProjection.GridToScreen(minRx, minRy) + new Vector2(0, -IsoProjection.TileHeight * 0.5f);
        Vector2 right = IsoProjection.GridToScreen(maxRx, minRy) + new Vector2(IsoProjection.TileWidth * 0.5f, 0);
        Vector2 bottom = IsoProjection.GridToScreen(maxRx, maxRy) + new Vector2(0, IsoProjection.TileHeight * 0.5f);
        Vector2 left = IsoProjection.GridToScreen(minRx, maxRy) + new Vector2(-IsoProjection.TileWidth * 0.5f, 0);

        Color fill = ColorForBuilding(slot.BuildingDefinitionId);
        DrawColoredPolygon(new[] { top, right, bottom, left }, fill);
        DrawPolyline(new[] { top, right, bottom, left, top }, new Color(0, 0, 0, 0.8f), 2f);

        // Centered "WxH ID" label.
        if (_font != null)
        {
            Vector2 center = (top + bottom) * 0.5f;
            string text = $"{slot.Size.Width}x{slot.Size.Height} {slot.BuildingDefinitionId}";
            var size = _font.GetStringSize(text);
            DrawString(_font, center - size * 0.5f + new Vector2(0, size.Y * 0.25f),
                text, HorizontalAlignment.Left, -1, 12, new Color(1, 1, 1));
        }
    }

    public static Color ColorForTerrain(TerrainType t) => t switch
    {
        TerrainType.Flat => new Color(0.55f, 0.55f, 0.55f),
        TerrainType.Rough => new Color(0.35f, 0.35f, 0.35f),
        TerrainType.Slope => new Color(0.45f, 0.45f, 0.40f),
        TerrainType.Crater => new Color(0.25f, 0.20f, 0.20f),
        TerrainType.Impassable => new Color(0.05f, 0.05f, 0.05f),
        _ => new Color(0.5f, 0.5f, 0.5f),
    };

    private static Color ColorForBuilding(string id)
    {
        // Deterministic-ish color from id, biased toward saturated mids.
        int h = id.GetHashCode();
        float hue = ((h & 0xFFFF) / 65535f);
        return Color.FromHsv(hue, 0.55f, 0.85f);
    }
}
