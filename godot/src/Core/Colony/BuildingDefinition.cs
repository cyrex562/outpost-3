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
    string Description = ""
);
