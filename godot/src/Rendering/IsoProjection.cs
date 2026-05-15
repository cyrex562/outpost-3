using Godot;
using OutpostGame.Core.Colony;

namespace OutpostGame.Rendering;

/// <summary>
/// Isometric projection helpers.
///
/// The colony grid is conceptually a flat square grid (X,Y). We render it with an
/// isometric diamond projection. The "4-direction camera" is implemented at the
/// projection level: rather than rotating the Camera2D (which would visually rotate
/// the diamonds), we re-project grid coordinates through one of four facing transforms
/// (N/E/S/W), so the visual stays a clean iso view at any facing.
///
/// Facing index:
///   0 = North (default)   (gx, gy) -> ( gx,  gy)
///   1 = East              (gx, gy) -> ( gy,  W-1-gx)
///   2 = South             (gx, gy) -> (W-1-gx, H-1-gy)
///   3 = West              (gx, gy) -> (H-1-gy, gx)
///
/// where (W,H) is the grid size. This rotates the *logical* grid 90° clockwise
/// per facing step before projecting to iso screen space.
/// </summary>
public static class IsoProjection
{
    public const float TileWidth = 64f;
    public const float TileHeight = 32f;

    /// <summary>Rotate a grid coord by the facing (0..3) inside a WxH grid.</summary>
    public static (int x, int y) ApplyFacing(int gx, int gy, int width, int height, int facing)
    {
        facing = ((facing % 4) + 4) % 4;
        return facing switch
        {
            0 => (gx, gy),
            1 => (gy, width - 1 - gx),
            2 => (width - 1 - gx, height - 1 - gy),
            3 => (height - 1 - gy, gx),
            _ => (gx, gy),
        };
    }

    /// <summary>Inverse of ApplyFacing — used when converting a screen-derived grid coord
    /// back into the underlying model coord.</summary>
    public static (int x, int y) RemoveFacing(int rx, int ry, int width, int height, int facing)
    {
        facing = ((facing % 4) + 4) % 4;
        return facing switch
        {
            0 => (rx, ry),
            1 => (width - 1 - ry, rx),
            2 => (width - 1 - rx, height - 1 - ry),
            3 => (ry, height - 1 - rx),
            _ => (rx, ry),
        };
    }

    /// <summary>Project a (post-facing) integer grid coord to iso screen-space.</summary>
    public static Vector2 GridToScreen(int rx, int ry)
    {
        return new Vector2(
            (rx - ry) * TileWidth * 0.5f,
            (rx + ry) * TileHeight * 0.5f);
    }

    /// <summary>Project a (post-facing) fractional grid coord to iso screen-space.</summary>
    public static Vector2 GridToScreenF(float rx, float ry)
    {
        return new Vector2(
            (rx - ry) * TileWidth * 0.5f,
            (rx + ry) * TileHeight * 0.5f);
    }

    /// <summary>Inverse of GridToScreen. Returns the fractional rotated-grid coord.</summary>
    public static Vector2 ScreenToGridF(Vector2 world)
    {
        float rx = (world.X / (TileWidth * 0.5f) + world.Y / (TileHeight * 0.5f)) * 0.5f;
        float ry = (world.Y / (TileHeight * 0.5f) - world.X / (TileWidth * 0.5f)) * 0.5f;
        return new Vector2(rx, ry);
    }

    /// <summary>Convert a world position to a model-grid cell, taking facing into account.</summary>
    public static GridPosition ScreenToGrid(Vector2 world, int width, int height, int facing)
    {
        var f = ScreenToGridF(world);
        int rx = Mathf.FloorToInt(f.X);
        int ry = Mathf.FloorToInt(f.Y);
        var (gx, gy) = RemoveFacing(rx, ry, width, height, facing);
        return new GridPosition(gx, gy);
    }

    /// <summary>Convert a model-grid cell to its iso screen position, taking facing into account.</summary>
    public static Vector2 GridToScreen(GridPosition gp, int width, int height, int facing)
    {
        var (rx, ry) = ApplyFacing(gp.X, gp.Y, width, height, facing);
        return GridToScreen(rx, ry);
    }

    /// <summary>The four diamond corner offsets relative to the cell's top corner.</summary>
    public static Vector2[] DiamondCorners(int rx, int ry)
    {
        // Cell center in screen space; the diamond's top corner sits half a tile above.
        Vector2 center = GridToScreen(rx, ry);
        return new[]
        {
            center + new Vector2(0, -TileHeight * 0.5f),   // top
            center + new Vector2(TileWidth * 0.5f, 0),     // right
            center + new Vector2(0, TileHeight * 0.5f),    // bottom
            center + new Vector2(-TileWidth * 0.5f, 0),    // left
        };
    }
}
