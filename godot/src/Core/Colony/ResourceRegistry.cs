namespace OutpostGame.Core.Colony;

/// <summary>
/// Static registry of all resource definitions for the colony simulation.
/// Definitions are ported from content/resources/basic_resources.yaml.
/// </summary>
public static class ResourceRegistry
{
    private static readonly Dictionary<string, ResourceDefinition> _all = new();

    static ResourceRegistry()
    {
        // ========================================================================
        // TIER 0: RAW MATERIALS
        // ========================================================================
        Register(new ResourceDefinition("iron_ore", "Iron Ore",
            ResourceTier.Raw, ResourceCategory.Mineral, 10.0f,
            "Raw iron ore extracted from deposits."));
        Register(new ResourceDefinition("copper_ore", "Copper Ore",
            ResourceTier.Raw, ResourceCategory.Mineral, 12.0f,
            "Chalcopyrite and other copper-bearing minerals."));
        Register(new ResourceDefinition("silicon_ore", "Silicon Ore",
            ResourceTier.Raw, ResourceCategory.Mineral, 8.0f,
            "Silicate minerals processed into pure silicon."));
        Register(new ResourceDefinition("ice", "Ice",
            ResourceTier.Raw, ResourceCategory.Chemical, 2.0f,
            "Frozen water from polar regions and shadowed craters."));
        Register(new ResourceDefinition("regolith", "Regolith",
            ResourceTier.Raw, ResourceCategory.Mineral, 1.0f,
            "Loose surface material covering bedrock."));
        Register(new ResourceDefinition("uranium_ore", "Uranium Ore",
            ResourceTier.Raw, ResourceCategory.Energy, 150.0f,
            "Radioactive uranium-bearing ore."));
        Register(new ResourceDefinition("carbon_compounds", "Carbon Compounds",
            ResourceTier.Raw, ResourceCategory.Chemical, 15.0f,
            "Organic carbon deposits including hydrocarbons."));
        Register(new ResourceDefinition("rare_earth_ore", "Rare Earth Ore",
            ResourceTier.Raw, ResourceCategory.Mineral, 80.0f,
            "Ore containing lanthanides and other rare earth elements."));

        // ========================================================================
        // TIER 1: REFINED MATERIALS
        // ========================================================================
        Register(new ResourceDefinition("iron", "Iron",
            ResourceTier.Refined, ResourceCategory.Manufactured, 30.0f,
            "Refined iron metal."));
        Register(new ResourceDefinition("steel", "Steel",
            ResourceTier.Refined, ResourceCategory.Manufactured, 50.0f,
            "Iron-carbon alloy for structural use."));
        Register(new ResourceDefinition("copper", "Copper",
            ResourceTier.Refined, ResourceCategory.Manufactured, 40.0f,
            "Refined copper metal."));
        Register(new ResourceDefinition("silicon_wafer", "Silicon Wafer",
            ResourceTier.Refined, ResourceCategory.Manufactured, 120.0f,
            "High-purity silicon processed into wafers."));
        Register(new ResourceDefinition("water", "Water",
            ResourceTier.Refined, ResourceCategory.Chemical, 1.0f,
            "H2O - essential for life support and agriculture."));
        Register(new ResourceDefinition("uranium_fuel_rod", "Uranium Fuel Rod",
            ResourceTier.Refined, ResourceCategory.Energy, 5000.0f,
            "Enriched uranium fuel rod for nuclear reactors."));
        Register(new ResourceDefinition("carbon_fiber", "Carbon Fiber",
            ResourceTier.Refined, ResourceCategory.Manufactured, 180.0f,
            "High-strength carbon composite material."));
        Register(new ResourceDefinition("rare_earth_metals", "Rare Earth Metals",
            ResourceTier.Refined, ResourceCategory.Manufactured, 500.0f,
            "Refined lanthanides for advanced electronics."));

        // ========================================================================
        // TIER 2: MANUFACTURED COMPONENTS
        // ========================================================================
        Register(new ResourceDefinition("structural_components", "Structural Components",
            ResourceTier.Advanced, ResourceCategory.Manufactured, 150.0f,
            "Prefabricated beams, panels, and supports."));
        Register(new ResourceDefinition("electronics", "Electronics",
            ResourceTier.Advanced, ResourceCategory.Manufactured, 200.0f,
            "Electronic components, circuits, and control systems."));
        Register(new ResourceDefinition("machine_parts", "Machine Parts",
            ResourceTier.Advanced, ResourceCategory.Manufactured, 120.0f,
            "Mechanical components including motors, pumps, and valves."));
        Register(new ResourceDefinition("construction_materials", "Construction Materials",
            ResourceTier.Advanced, ResourceCategory.Manufactured, 80.0f,
            "Mixed building materials including fasteners and sealants."));

        // ========================================================================
        // TIER 3: CONSUMABLES
        // ========================================================================
        Register(new ResourceDefinition("oxygen", "Oxygen",
            ResourceTier.Refined, ResourceCategory.Chemical, 5.0f,
            "O2 gas essential for life support."));
        Register(new ResourceDefinition("food", "Food",
            ResourceTier.Refined, ResourceCategory.Biological, 20.0f,
            "Processed food for human consumption."));
        Register(new ResourceDefinition("medical_supplies", "Medical Supplies",
            ResourceTier.Refined, ResourceCategory.Biological, 300.0f,
            "Pharmaceuticals, bandages, and diagnostic equipment."));
        Register(new ResourceDefinition("nutrients", "Nutrients",
            ResourceTier.Refined, ResourceCategory.Biological, 15.0f,
            "Mineral nutrients for hydroponic farming."));

        // ========================================================================
        // SPECIAL: VIRTUAL RESOURCES
        // ========================================================================
        Register(new ResourceDefinition("credits", "Credits",
            ResourceTier.Virtual, ResourceCategory.Virtual, 1.0f,
            "Universal currency."));
        Register(new ResourceDefinition("research_data", "Research Data",
            ResourceTier.Virtual, ResourceCategory.Virtual, 100.0f,
            "Scientific data used for technological advancement."));
    }

    private static void Register(ResourceDefinition def) => _all[def.Id] = def;

    public static ResourceDefinition Get(string id) =>
        _all.TryGetValue(id, out var def)
            ? def
            : throw new KeyNotFoundException($"Unknown resource: {id}");

    public static bool TryGet(string id, out ResourceDefinition? def) =>
        _all.TryGetValue(id, out def);

    public static IReadOnlyDictionary<string, ResourceDefinition> All => _all;
}
