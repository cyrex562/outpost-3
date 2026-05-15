namespace OutpostGame.Core.Colony;

/// <summary>
/// Static registry of all building definitions for the colony simulation.
/// Definitions are ported from content/buildings/basic_buildings.yaml.
/// Note: the source YAML does not include grid footprint sizes; sensible defaults
/// are used here per building class. Construction durations are taken from
/// construction_time_ticks; power values from power_output_mw / power.base_mw.
/// </summary>
public static class BuildingRegistry
{
    private static readonly Dictionary<string, BuildingDefinition> _all = new();

    static BuildingRegistry()
    {
        // Solar Array Mk1 — power generation
        Register(new BuildingDefinition(
            Id: "solar_array_mk1",
            DisplayName: "Solar Array Mk1",
            Size: new GridSize(2, 2),                                      // default: small power array
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 50, ["electronics"] = 20 },
            ConstructionTurns: 120,
            PowerConsumption: 0f,
            PowerProduction: 10.0f,
            LaborRequired: 2,
            Recipe: null,
            MaintenanceCost: new Dictionary<string, float> { ["electronics"] = 0.1f },
            IsEssential: false,
            Description: "Converts sunlight into electrical energy."
        ));

        // Iron Mine — extraction
        Register(new BuildingDefinition(
            Id: "iron_mine",
            DisplayName: "Iron Mine",
            Size: new GridSize(3, 3),                                      // default: medium extractor
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 100, ["electronics"] = 10 },
            ConstructionTurns: 200,
            PowerConsumption: 5.0f,
            PowerProduction: 0f,
            LaborRequired: 10,
            Recipe: new ProductionRecipe(
                Inputs: new Dictionary<string, float>(),
                Outputs: new Dictionary<string, float> { ["iron_ore"] = 10.0f },
                TurnsPerCycle: 10
            ),
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 0.5f },
            IsEssential: false,
            Description: "Extracts iron ore from surface or underground deposits."
        ));

        // Smelter — refining
        Register(new BuildingDefinition(
            Id: "smelter",
            DisplayName: "Smelter",
            Size: new GridSize(3, 3),                                      // default: industrial refinery
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 150, ["electronics"] = 30 },
            ConstructionTurns: 250,
            PowerConsumption: 12.0f,
            PowerProduction: 0f,
            LaborRequired: 8,
            Recipe: new ProductionRecipe(
                Inputs: new Dictionary<string, float> { ["iron_ore"] = 10.0f },
                Outputs: new Dictionary<string, float> { ["steel"] = 5.0f },
                TurnsPerCycle: 5
            ),
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 1.0f },
            IsEssential: false,
            Description: "Refines raw ore into usable metal ingots."
        ));

        // Basic Habitat — housing (essential)
        Register(new BuildingDefinition(
            Id: "basic_habitat",
            DisplayName: "Basic Habitat",
            Size: new GridSize(3, 3),                                      // default: residential block
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 200, ["electronics"] = 50 },
            ConstructionTurns: 300,
            PowerConsumption: 3.0f,
            PowerProduction: 0f,
            LaborRequired: 1,                                              // default: minimal upkeep crew (YAML worker_capacity=0)
            Recipe: null,
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 0.5f, ["electronics"] = 0.2f },
            IsEssential: true,
            Description: "Standard living quarters for colonists."
        ));

        // Warehouse — storage
        Register(new BuildingDefinition(
            Id: "warehouse",
            DisplayName: "Warehouse",
            Size: new GridSize(3, 3),                                      // default: medium storage
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 100 },
            ConstructionTurns: 150,
            PowerConsumption: 1.0f,
            PowerProduction: 0f,
            LaborRequired: 3,
            Recipe: null,
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 0.2f },
            IsEssential: false,
            Description: "Bulk storage facility for solid resources and manufactured goods."
        ));

        // Hydroponics Bay — food production
        Register(new BuildingDefinition(
            Id: "hydroponics_bay",
            DisplayName: "Hydroponics Bay",
            Size: new GridSize(3, 3),                                      // default: medium farm
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 80, ["electronics"] = 40 },
            ConstructionTurns: 180,
            PowerConsumption: 4.0f,
            PowerProduction: 0f,
            LaborRequired: 6,
            Recipe: new ProductionRecipe(
                Inputs: new Dictionary<string, float> { ["water"] = 5.0f, ["nutrients"] = 1.0f },
                Outputs: new Dictionary<string, float> { ["food"] = 8.0f },
                TurnsPerCycle: 100
            ),
            MaintenanceCost: new Dictionary<string, float> { ["electronics"] = 0.3f },
            IsEssential: false,
            Description: "Indoor farming facility using nutrient solutions."
        ));

        // Fabricator — manufacturing
        // YAML defines two recipes; pick the structural components recipe as the primary recipe.
        Register(new BuildingDefinition(
            Id: "fabricator",
            DisplayName: "Fabricator",
            Size: new GridSize(3, 3),                                      // default: medium factory
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 180, ["electronics"] = 60 },
            ConstructionTurns: 280,
            PowerConsumption: 8.0f,
            PowerProduction: 0f,
            LaborRequired: 12,
            Recipe: new ProductionRecipe(
                Inputs: new Dictionary<string, float> { ["steel"] = 5.0f },
                Outputs: new Dictionary<string, float> { ["components"] = 3.0f },
                TurnsPerCycle: 8
            ),
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 0.8f, ["electronics"] = 0.5f },
            IsEssential: false,
            Description: "Advanced manufacturing facility for machine parts and electronics."
        ));

        // Nuclear Reactor — high-output power
        Register(new BuildingDefinition(
            Id: "nuclear_reactor",
            DisplayName: "Nuclear Reactor",
            Size: new GridSize(4, 4),                                      // default: large power plant
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 300, ["electronics"] = 100, ["components"] = 50 },
            ConstructionTurns: 500,
            PowerConsumption: 0f,
            PowerProduction: 50.0f,
            LaborRequired: 8,
            Recipe: new ProductionRecipe(
                Inputs: new Dictionary<string, float> { ["uranium_fuel"] = 0.1f },
                Outputs: new Dictionary<string, float>(),
                TurnsPerCycle: 1
            ),
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 2.0f, ["electronics"] = 1.0f },
            IsEssential: false,
            Description: "High-output fission reactor for reliable power generation."
        ));

        // Water Purifier — water (essential)
        Register(new BuildingDefinition(
            Id: "water_purifier",
            DisplayName: "Water Purifier",
            Size: new GridSize(2, 3),                                      // default: small infrastructure
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 120, ["electronics"] = 40 },
            ConstructionTurns: 200,
            PowerConsumption: 6.0f,
            PowerProduction: 0f,
            LaborRequired: 4,
            Recipe: new ProductionRecipe(
                Inputs: new Dictionary<string, float> { ["ice"] = 10.0f },
                Outputs: new Dictionary<string, float> { ["water"] = 8.0f },
                TurnsPerCycle: 5
            ),
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 0.4f, ["electronics"] = 0.3f },
            IsEssential: true,
            Description: "Processes ice or contaminated water into clean, potable water."
        ));

        // Life Support System — atmosphere (essential)
        Register(new BuildingDefinition(
            Id: "life_support_system",
            DisplayName: "Life Support System",
            Size: new GridSize(3, 3),                                      // default: medium infrastructure
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 250, ["electronics"] = 80, ["components"] = 40 },
            ConstructionTurns: 350,
            PowerConsumption: 10.0f,
            PowerProduction: 0f,
            LaborRequired: 6,
            Recipe: new ProductionRecipe(
                Inputs: new Dictionary<string, float> { ["water"] = 5.0f },
                Outputs: new Dictionary<string, float> { ["oxygen"] = 10.0f },
                TurnsPerCycle: 10
            ),
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 1.0f, ["electronics"] = 0.8f },
            IsEssential: true,
            Description: "Generates breathable atmosphere and maintains temperature control."
        ));

        // Large Warehouse — Tier-2 bulk storage
        Register(new BuildingDefinition(
            Id: "large_warehouse",
            DisplayName: "Large Warehouse",
            Size: new GridSize(4, 4),                                      // default: large storage
            ConstructionCost: new Dictionary<string, int> { ["steel"] = 300, ["components"] = 50 },
            ConstructionTurns: 300,
            PowerConsumption: 2.0f,
            PowerProduction: 0f,
            LaborRequired: 6,
            Recipe: null,
            MaintenanceCost: new Dictionary<string, float> { ["steel"] = 0.5f, ["electronics"] = 0.1f },
            IsEssential: false,
            Description: "High-capacity storage complex for bulk solid resources."
        ));
    }

    private static void Register(BuildingDefinition def) => _all[def.Id] = def;

    public static BuildingDefinition Get(string id) =>
        _all.TryGetValue(id, out var def)
            ? def
            : throw new KeyNotFoundException($"Unknown building: {id}");

    public static bool TryGet(string id, out BuildingDefinition? def) =>
        _all.TryGetValue(id, out def);

    public static IReadOnlyDictionary<string, BuildingDefinition> All => _all;
}
