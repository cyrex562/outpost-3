namespace OutpostGame.Core.Colony;

public sealed record ProductionRecipe(
    IReadOnlyDictionary<string, float> Inputs,   // ResourceId -> amount per cycle
    IReadOnlyDictionary<string, float> Outputs,  // ResourceId -> amount per cycle
    int TurnsPerCycle
);

public sealed record BuildingDefinition(
    string Id,
    string DisplayName,
    GridSize Size,                                // Grid footprint, e.g. (2, 2)
    IReadOnlyDictionary<string, int> ConstructionCost,
    int ConstructionTurns,
    float PowerConsumption,
    float PowerProduction,
    int LaborRequired,
    ProductionRecipe? Recipe,
    IReadOnlyDictionary<string, float>? MaintenanceCost,
    bool IsEssential,                             // Essential buildings stay powered during brownout
    bool IsSolar = false,                         // Solar producers lose output during dust storms
    SkillType PrimarySkill = SkillType.Laborer,   // Skill type that gives full efficiency bonus
    string Description = "",
    BuildingCategory Category = BuildingCategory.Production,
    int HousingCapacity = 0,                      // Colonists this building can house
    float StorageCapacity = 0f,                   // Storage capacity contributed to the colony cap
    string? UpgradesTo = null,                    // Building ID this building upgrades into (mk1 → mk2)
    int RequiredFleetSlots = 1,                   // Fleet equipment slots held during construction
    int RequiredOperators = 1                     // Operators (people/robots) consumed alongside fleet
);
