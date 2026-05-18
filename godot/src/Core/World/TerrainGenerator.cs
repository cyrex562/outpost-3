using OutpostGame.Core.Colony;

namespace OutpostGame.Core.World;

/// <summary>
/// Biome-aware terrain generator. Plain C# — no Godot dependency.
/// Each <see cref="BiomeType"/> produces a characteristic terrain distribution and
/// impassable-cluster density; the two-pass approach (random roll + random walk) is kept
/// from Phase 1 but parameterised per-biome.
/// </summary>
public static class TerrainGenerator
{
    public static TerrainType[,] Generate(
        int seed, int width, int height,
        BiomeType biome = BiomeType.Barren)
    {
        if (width <= 0 || height <= 0)
            throw new ArgumentOutOfRangeException(nameof(width), "width/height must be positive");

        var rng = new Random(seed);
        var cells = new TerrainType[width, height];
        var t = Thresholds(biome);

        // Pass 1: assign base terrain by roll.
        for (int x = 0; x < width; x++)
        {
            for (int y = 0; y < height; y++)
            {
                double roll = rng.NextDouble();
                cells[x, y] = roll switch
                {
                    var r when r < t.Flat     => TerrainType.Flat,
                    var r when r < t.Rough    => TerrainType.Rough,
                    var r when r < t.Crater   => TerrainType.Crater,
                    var r when r < t.Slope    => TerrainType.Slope,
                    _                         => TerrainType.Impassable,
                };
            }
        }

        // Pass 2: cluster impassable terrain via random walk(s).
        int steps = ClusterSteps(biome);
        int wx = rng.Next(width);
        int wy = rng.Next(height);
        for (int step = 0; step < steps; step++)
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

    // ── Biome parameters ─────────────────────────────────────────────────────

    private readonly struct BiomeThresholds(double flat, double rough, double crater, double slope)
    {
        public readonly double Flat   = flat;
        public readonly double Rough  = rough;
        public readonly double Crater = crater;
        public readonly double Slope  = slope;
        // Impassable fills the remainder to 1.0
    }

    /// <summary>
    /// Cumulative probability thresholds for terrain type selection.
    /// Rolls are assigned: &lt;Flat → Flat, &lt;Rough → Rough, &lt;Crater → Crater,
    /// &lt;Slope → Slope, else → Impassable.
    /// </summary>
    private static BiomeThresholds Thresholds(BiomeType biome) => biome switch
    {
        // Flat  Rough  Crater  Slope   (Impassable = remainder)
        BiomeType.Barren            => new(0.60, 0.85, 0.95, 0.95),  // 60/25/10/0/5
        BiomeType.Rocky             => new(0.45, 0.80, 0.92, 0.92),  // 45/35/12/0/8
        BiomeType.Polar             => new(0.55, 0.75, 0.80, 0.95),  // 55/20/5/15/5
        BiomeType.Desert            => new(0.70, 0.90, 0.95, 0.95),  // 70/20/5/0/5
        BiomeType.Volcanic          => new(0.30, 0.60, 0.80, 0.80),  // 30/30/20/0/20
        BiomeType.MarginalHabitable => new(0.65, 0.85, 0.93, 0.93),  // 65/20/8/0/7
        _                           => new(0.60, 0.85, 0.95, 0.95),
    };

    /// <summary>Length of the impassable-clustering random walk in Pass 2.</summary>
    private static int ClusterSteps(BiomeType biome) => biome switch
    {
        BiomeType.Barren            => 8,
        BiomeType.Rocky             => 12,
        BiomeType.Polar             => 6,
        BiomeType.Desert            => 4,
        BiomeType.Volcanic          => 20,
        BiomeType.MarginalHabitable => 6,
        _                           => 8,
    };
}
