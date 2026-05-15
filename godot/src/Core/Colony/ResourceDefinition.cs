namespace OutpostGame.Core.Colony;

public sealed record ResourceDefinition(
    string Id,
    string DisplayName,
    ResourceTier Tier,
    ResourceCategory Category,
    float BaseWeight,
    string Description = ""
);
