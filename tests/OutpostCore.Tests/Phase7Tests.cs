namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Simulation;
using OutpostGame.Core.World;

/// <summary>Phase 7: biome-aware terrain generation and starting conditions.</summary>
[TestFixture]
public class Phase7Tests
{
    private const int Seed = 99;
    private const int W = 100;
    private const int H = 100;

    private static (Dictionary<TerrainType, int> counts, int total) CountTiles(BiomeType biome)
    {
        var terrain = TerrainGenerator.Generate(Seed, W, H, biome);
        var counts = new Dictionary<TerrainType, int>();
        foreach (TerrainType t in Enum.GetValues<TerrainType>())
            counts[t] = 0;

        for (int x = 0; x < W; x++)
            for (int y = 0; y < H; y++)
                counts[terrain[x, y]]++;

        return (counts, W * H);
    }

    private static float Pct(int count, int total) => count * 100f / total;

    // ── Terrain distribution ──────────────────────────────────────────────────

    [Test]
    public void Terrain_Barren_FlatDominates()
    {
        var (c, total) = CountTiles(BiomeType.Barren);
        Assert.That(Pct(c[TerrainType.Flat], total), Is.GreaterThan(50f),
            "Barren should be >50% flat");
    }

    [Test]
    public void Terrain_Desert_HasMostFlat()
    {
        var (cDesert, tDesert) = CountTiles(BiomeType.Desert);
        var (cBarren, tBarren) = CountTiles(BiomeType.Barren);
        Assert.That(Pct(cDesert[TerrainType.Flat], tDesert),
            Is.GreaterThan(Pct(cBarren[TerrainType.Flat], tBarren)),
            "Desert should have more flat tiles than Barren");
    }

    [Test]
    public void Terrain_Rocky_HasMoreRoughThanBarren()
    {
        var (cRocky, tRocky)   = CountTiles(BiomeType.Rocky);
        var (cBarren, tBarren) = CountTiles(BiomeType.Barren);
        Assert.That(Pct(cRocky[TerrainType.Rough], tRocky),
            Is.GreaterThan(Pct(cBarren[TerrainType.Rough], tBarren)),
            "Rocky should have more rough tiles than Barren");
    }

    [Test]
    public void Terrain_Polar_HasSlopeTiles()
    {
        var (c, total) = CountTiles(BiomeType.Polar);
        Assert.That(Pct(c[TerrainType.Slope], total), Is.GreaterThan(8f),
            "Polar should have >8% slope tiles");
    }

    [Test]
    public void Terrain_Volcanic_HasMostImpassable()
    {
        var results = Enum.GetValues<BiomeType>()
            .Select(b => { var (counts, t) = CountTiles(b); return (b, pct: Pct(counts[TerrainType.Impassable], t)); })
            .OrderByDescending(x => x.pct)
            .ToList();

        Assert.That(results.First().b, Is.EqualTo(BiomeType.Volcanic),
            "Volcanic should have the most impassable tiles");
    }

    [Test]
    public void Terrain_Volcanic_ImpassableExceedsBarren()
    {
        var (cV, tV) = CountTiles(BiomeType.Volcanic);
        var (cB, tB) = CountTiles(BiomeType.Barren);
        Assert.That(Pct(cV[TerrainType.Impassable], tV),
            Is.GreaterThan(Pct(cB[TerrainType.Impassable], tB) * 2),
            "Volcanic impassable% should be more than 2× Barren impassable%");
    }

    [Test]
    public void Terrain_AllBiomes_ProduceValidGrids()
    {
        foreach (BiomeType biome in Enum.GetValues<BiomeType>())
        {
            Assert.DoesNotThrow(
                () => TerrainGenerator.Generate(Seed, 32, 32, biome),
                $"Generate should not throw for biome {biome}");
        }
    }

    [Test]
    public void Terrain_DefaultBiome_MatchesBarren()
    {
        var withDefault = TerrainGenerator.Generate(Seed, W, H);
        var withBarren  = TerrainGenerator.Generate(Seed, W, H, BiomeType.Barren);

        for (int x = 0; x < W; x++)
            for (int y = 0; y < H; y++)
                Assert.That(withDefault[x, y], Is.EqualTo(withBarren[x, y]),
                    $"Default biome mismatch at ({x},{y})");
    }

    // ── Biome starting conditions ─────────────────────────────────────────────

    private static ColonySession NewSession(BiomeType biome) =>
        new(new SiteDefinition(42, biome, new GridSize(32, 32)));

    [Test]
    public void BiomeStart_Polar_HasMoreIceThanBarren()
    {
        float polarIce  = NewSession(BiomeType.Polar).State.Resources.Get("ice");
        float barrenIce = NewSession(BiomeType.Barren).State.Resources.Get("ice");
        Assert.That(polarIce, Is.GreaterThan(barrenIce));
    }

    [Test]
    public void BiomeStart_Volcanic_HasMoreUraniumThanBarren()
    {
        float volcanicU = NewSession(BiomeType.Volcanic).State.Resources.Get("uranium_fuel");
        float barrenU   = NewSession(BiomeType.Barren).State.Resources.Get("uranium_fuel");
        Assert.That(volcanicU, Is.GreaterThan(barrenU));
    }

    [Test]
    public void BiomeStart_Rocky_HasMoreSteelThanBarren()
    {
        float rockySteel  = NewSession(BiomeType.Rocky).State.Resources.Get("steel");
        float barrenSteel = NewSession(BiomeType.Barren).State.Resources.Get("steel");
        Assert.That(rockySteel, Is.GreaterThan(barrenSteel));
    }

    [Test]
    public void BiomeStart_MarginalHabitable_HasMoreNutrientsThanBarren()
    {
        float margN  = NewSession(BiomeType.MarginalHabitable).State.Resources.Get("nutrients");
        float barnN  = NewSession(BiomeType.Barren).State.Resources.Get("nutrients");
        Assert.That(margN, Is.GreaterThan(barnN));
    }

    [Test]
    public void BiomeStart_MarginalHabitable_HasLargerStartingPop()
    {
        int margPop  = NewSession(BiomeType.MarginalHabitable).State.Population.Count;
        int barrenPop= NewSession(BiomeType.Barren).State.Population.Count;
        Assert.That(margPop, Is.GreaterThan(barrenPop));
    }

    [Test]
    public void BiomeStart_Desert_HasLessWaterThanBarren()
    {
        float desertW = NewSession(BiomeType.Desert).State.Resources.Get("water");
        float barrenW = NewSession(BiomeType.Barren).State.Resources.Get("water");
        Assert.That(desertW, Is.LessThan(barrenW));
    }

    [Test]
    public void BiomeStart_AllBiomes_CanConstructSession()
    {
        foreach (BiomeType biome in Enum.GetValues<BiomeType>())
        {
            Assert.DoesNotThrow(() =>
            {
                var s = NewSession(biome);
                s.EndTurn(5);
            }, $"Session should not throw for biome {biome}");
        }
    }
}
