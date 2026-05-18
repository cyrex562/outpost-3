namespace OutpostGame.Core.Colony;

public enum TechCategory
{
    Engineering,
    Physics,
    LifeSciences,
    Computing,
    Social,
    Exploration,
}

public sealed record TechUnlocks(
    IReadOnlyList<string> Buildings,
    IReadOnlyList<string> Resources,
    IReadOnlyList<string> Bonuses
);

public sealed record TechDefinition(
    string Id,
    string DisplayName,
    TechCategory Category,
    int Tier,
    float ResearchCost,
    int ResearchTurns,
    IReadOnlyList<string> Prerequisites,
    TechUnlocks Unlocks,
    string Description = ""
);
