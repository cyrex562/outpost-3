using OutpostGame.Core.Colony;

namespace OutpostGame.Core.World;

/// <summary>
/// Phase 1 placeholder terrain generator. Plain C# — no Godot dependency.
/// Rough distribution: ~70% Flat, ~20% Rough/Rocky, ~5% Crater, ~5% Impassable.
/// Impassable cells are clustered via a single short random walk of length 8 from a
/// random origin (Phase 3 will replace this with biome-aware generation).
/// </summary>
public static class TerrainGenerator
{
    public static TerrainType[,] Generate(int seed, int width, int height)
    {
        if (width <= 0 || height <= 0)
            throw new ArgumentOutOfRangeException(nameof(width), "width/height must be positive");

        var rng = new Random(seed);
        var cells = new TerrainType[width, height];

        // Pass 1: assign base terrain by roll.
        for (int x = 0; x < width; x++)
        {
            for (int y = 0; y < height; y++)
            {
                double roll = rng.NextDouble();
                cells[x, y] = roll switch
                {
                    < 0.70 => TerrainType.Flat,
                    < 0.90 => TerrainType.Rough,
                    < 0.95 => TerrainType.Crater,
                    _ => TerrainType.Impassable,
                };
            }
        }

        // Pass 2: cluster impassable via a single random walk of length 8.
        int wx = rng.Next(width);
        int wy = rng.Next(height);
        for (int step = 0; step < 8; step++)
        {
            if (wx >= 0 && wx < width && wy >= 0 && wy < height)
                cells[wx, wy] = TerrainType.Impassable;

            int dir = rng.Next(4);
            switch (dir)
            {
                case 0: wx += 1; break;
                case 1: wx -= 1; break;
                case 2: wy += 1; break;
                case 3: wy -= 1; break;
            }
        }

        return cells;
    }

    /// <summary>True if a building footprint may be placed over the given terrain cells.</summary>
    public static bool IsBuildable(TerrainType t) => t != TerrainType.Impassable;
}
