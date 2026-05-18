using OutpostGame.Core.Colony;

namespace OutpostGame.Core.World;

/// <summary>
/// Typed handoff between planet map (Phase 3) and colony scene.
/// Phase 1 only consumes Seed / BiomeType / Size; StartingDeposits reserved for Phase 3.
/// Plain C# record — must not depend on Godot.
/// </summary>
public sealed record SiteDefinition(
    int Seed,
    BiomeType Biome,
    GridSize Size,
    DifficultyPreset Difficulty = DifficultyPreset.Normal,
    IReadOnlyDictionary<string, int>? StartingDeposits = null)
{
    public static SiteDefinition Default(int seed = 12345) =>
        new(seed, BiomeType.Barren, new GridSize(64, 64));
}
