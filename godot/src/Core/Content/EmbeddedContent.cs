namespace OutpostGame.Core.Content;

/// <summary>
/// Bundled JSON content used to seed the building / resource registries at
/// startup. These strings are the authoritative source for Phase 3.1 — the
/// matching files under <c>content/</c> exist as human-readable mirrors for
/// modders and as the target for Phase 3.2's runtime file loader. Until that
/// loader ships, edits here are what the simulation actually sees.
/// </summary>
internal static class EmbeddedContent
{
    public const string BuildingsJson = """
{
  "buildings": [
    {
      "id": "lander",
      "displayName": "Lander",
      "description": "The colony ship's crew lander, repurposed as a command, life-support, and storage hub. Cannot be rebuilt if lost.",
      "category": "Habitat",
      "size": { "width": 3, "height": 3 },
      "constructionCost": {},
      "constructionTurns": 0,
      "powerConsumption": 0.0,
      "powerProduction": 10.0,
      "laborRequired": 1,
      "isEssential": true,
      "primarySkill": "Operator",
      "housingCapacity": 10,
      "storageCapacity": 200,
      "requiredFleetSlots": 0,
      "requiredOperators": 0
    },
    {
      "id": "multi_habitat",
      "displayName": "Multi-Purpose Habitat",
      "description": "Modular pressurised crew quarters with integrated life support. Cannot be rebuilt if lost.",
      "category": "Habitat",
      "size": { "width": 3, "height": 3 },
      "constructionCost": {},
      "constructionTurns": 0,
      "powerConsumption": 2.0,
      "powerProduction": 0.0,
      "laborRequired": 1,
      "isEssential": true,
      "primarySkill": "Operator",
      "housingCapacity": 20,
      "recipe": {
        "inputs": { "water": 2.0 },
        "outputs": { "oxygen": 3.0 },
        "turnsPerCycle": 10
      },
      "requiredFleetSlots": 0,
      "requiredOperators": 0
    },
    {
      "id": "solar_array_mk1",
      "displayName": "Solar Array Mk1",
      "description": "Converts sunlight into electrical energy.",
      "category": "Power",
      "size": { "width": 2, "height": 2 },
      "constructionCost": { "prefab_components": 3, "steel": 50, "electronics": 20 },
      "constructionTurns": 40,
      "powerConsumption": 0.0,
      "powerProduction": 10.0,
      "laborRequired": 2,
      "isEssential": false,
      "isSolar": true,
      "primarySkill": "Operator",
      "maintenanceCost": { "electronics": 0.1 },
      "upgradesTo": "solar_array_mk2"
    },
    {
      "id": "solar_array_mk2",
      "displayName": "Solar Array Mk2",
      "description": "Upgraded solar array — double the output, sturdier panels.",
      "category": "Power",
      "size": { "width": 2, "height": 2 },
      "constructionCost": { "prefab_components": 3, "steel": 100, "electronics": 60 },
      "constructionTurns": 70,
      "powerConsumption": 0.0,
      "powerProduction": 20.0,
      "laborRequired": 2,
      "isEssential": false,
      "isSolar": true,
      "primarySkill": "Operator",
      "maintenanceCost": { "electronics": 0.2 }
    },
    {
      "id": "iron_mine",
      "displayName": "Iron Mine",
      "description": "Extracts iron ore from surface or underground deposits.",
      "category": "Production",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 100, "electronics": 10 },
      "constructionTurns": 70,
      "powerConsumption": 5.0,
      "powerProduction": 0.0,
      "laborRequired": 10,
      "isEssential": false,
      "primarySkill": "Laborer",
      "recipe": {
        "inputs": {},
        "outputs": { "iron_ore": 10.0 },
        "turnsPerCycle": 10
      },
      "maintenanceCost": { "steel": 0.5 }
    },
    {
      "id": "smelter",
      "displayName": "Smelter",
      "description": "Refines raw ore into usable metal ingots.",
      "category": "Production",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 150, "electronics": 30 },
      "constructionTurns": 85,
      "powerConsumption": 12.0,
      "powerProduction": 0.0,
      "laborRequired": 8,
      "isEssential": false,
      "primarySkill": "Engineer",
      "recipe": {
        "inputs": { "iron_ore": 10.0 },
        "outputs": { "steel": 5.0 },
        "turnsPerCycle": 5
      },
      "maintenanceCost": { "steel": 1.0 }
    },
    {
      "id": "basic_habitat",
      "displayName": "Basic Habitat",
      "description": "Standard living quarters for colonists.",
      "category": "Habitat",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 200, "electronics": 50 },
      "constructionTurns": 100,
      "powerConsumption": 3.0,
      "powerProduction": 0.0,
      "laborRequired": 1,
      "isEssential": true,
      "primarySkill": "Laborer",
      "maintenanceCost": { "steel": 0.5, "electronics": 0.2 },
      "housingCapacity": 20,
      "upgradesTo": "basic_habitat_mk2"
    },
    {
      "id": "basic_habitat_mk2",
      "displayName": "Habitat Mk2",
      "description": "Pressurised modular habitat with expanded crew quarters.",
      "category": "Habitat",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 12, "steel": 400, "electronics": 120, "components": 30 },
      "constructionTurns": 170,
      "powerConsumption": 4.0,
      "powerProduction": 0.0,
      "laborRequired": 2,
      "isEssential": true,
      "primarySkill": "Laborer",
      "maintenanceCost": { "steel": 0.8, "electronics": 0.4 },
      "housingCapacity": 40
    },
    {
      "id": "warehouse",
      "displayName": "Warehouse",
      "description": "Bulk storage facility for solid resources and manufactured goods.",
      "category": "Storage",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 100 },
      "constructionTurns": 50,
      "powerConsumption": 1.0,
      "powerProduction": 0.0,
      "laborRequired": 3,
      "isEssential": false,
      "primarySkill": "Laborer",
      "maintenanceCost": { "steel": 0.2 },
      "storageCapacity": 500
    },
    {
      "id": "hydroponics_bay",
      "displayName": "Hydroponics Bay",
      "description": "Indoor farming facility using nutrient solutions.",
      "category": "Production",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 80, "electronics": 40 },
      "constructionTurns": 60,
      "powerConsumption": 4.0,
      "powerProduction": 0.0,
      "laborRequired": 6,
      "isEssential": false,
      "primarySkill": "Farmer",
      "recipe": {
        "inputs": { "water": 5.0, "nutrients": 1.0 },
        "outputs": { "food": 8.0 },
        "turnsPerCycle": 100
      },
      "maintenanceCost": { "electronics": 0.3 }
    },
    {
      "id": "fabricator",
      "displayName": "Fabricator",
      "description": "Advanced manufacturing facility for machine parts and electronics.",
      "category": "Production",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 180, "electronics": 60 },
      "constructionTurns": 95,
      "powerConsumption": 8.0,
      "powerProduction": 0.0,
      "laborRequired": 12,
      "isEssential": false,
      "primarySkill": "Engineer",
      "recipe": {
        "inputs": { "steel": 5.0 },
        "outputs": { "components": 3.0 },
        "turnsPerCycle": 8
      },
      "maintenanceCost": { "steel": 0.8, "electronics": 0.5 }
    },
    {
      "id": "vehicle_factory",
      "displayName": "Vehicle Factory",
      "description": "Assembles new construction vehicles. Each completed cycle adds one fleet equipment slot to the colony's pool.",
      "category": "Production",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 200, "electronics": 80, "components": 40 },
      "constructionTurns": 100,
      "powerConsumption": 5.0,
      "powerProduction": 0.0,
      "laborRequired": 4,
      "isEssential": false,
      "primarySkill": "Engineer",
      "recipe": {
        "inputs": { "steel": 3.0, "components": 1.0 },
        "outputs": {},
        "turnsPerCycle": 60
      },
      "maintenanceCost": { "steel": 0.5, "electronics": 0.3 }
    },
    {
      "id": "prefab_factory",
      "displayName": "Prefab Factory",
      "description": "Assembles prefab structural kits required by nearly every constructible building.",
      "category": "Production",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 180, "electronics": 60 },
      "constructionTurns": 90,
      "powerConsumption": 6.0,
      "powerProduction": 0.0,
      "laborRequired": 5,
      "isEssential": false,
      "primarySkill": "Engineer",
      "recipe": {
        "inputs": { "steel": 4.0, "components": 1.0 },
        "outputs": { "prefab_components": 2.0 },
        "turnsPerCycle": 8
      },
      "maintenanceCost": { "steel": 0.5, "electronics": 0.3 }
    },
    {
      "id": "nuclear_reactor",
      "displayName": "Nuclear Reactor",
      "description": "High-output fission reactor for reliable power generation.",
      "category": "Power",
      "size": { "width": 4, "height": 4 },
      "constructionCost": { "prefab_components": 12, "steel": 300, "electronics": 100, "components": 50 },
      "constructionTurns": 170,
      "powerConsumption": 0.0,
      "powerProduction": 50.0,
      "laborRequired": 8,
      "isEssential": false,
      "primarySkill": "Engineer",
      "recipe": {
        "inputs": { "uranium_fuel": 0.1 },
        "outputs": {},
        "turnsPerCycle": 1
      },
      "maintenanceCost": { "steel": 2.0, "electronics": 1.0 }
    },
    {
      "id": "water_purifier",
      "displayName": "Water Purifier",
      "description": "Processes ice or contaminated water into clean, potable water.",
      "category": "LifeSupport",
      "size": { "width": 2, "height": 3 },
      "constructionCost": { "prefab_components": 4, "steel": 120, "electronics": 40 },
      "constructionTurns": 70,
      "powerConsumption": 6.0,
      "powerProduction": 0.0,
      "laborRequired": 4,
      "isEssential": true,
      "primarySkill": "Operator",
      "recipe": {
        "inputs": { "ice": 10.0 },
        "outputs": { "water": 8.0 },
        "turnsPerCycle": 5
      },
      "maintenanceCost": { "steel": 0.4, "electronics": 0.3 }
    },
    {
      "id": "life_support_system",
      "displayName": "Life Support System",
      "description": "Generates breathable atmosphere and maintains temperature control.",
      "category": "LifeSupport",
      "size": { "width": 3, "height": 3 },
      "constructionCost": { "prefab_components": 6, "steel": 250, "electronics": 80, "components": 40 },
      "constructionTurns": 120,
      "powerConsumption": 10.0,
      "powerProduction": 0.0,
      "laborRequired": 6,
      "isEssential": true,
      "primarySkill": "Operator",
      "recipe": {
        "inputs": { "water": 5.0 },
        "outputs": { "oxygen": 10.0 },
        "turnsPerCycle": 10
      },
      "maintenanceCost": { "steel": 1.0, "electronics": 0.8 }
    },
    {
      "id": "large_warehouse",
      "displayName": "Large Warehouse",
      "description": "High-capacity storage complex for bulk solid resources.",
      "category": "Storage",
      "size": { "width": 4, "height": 4 },
      "constructionCost": { "prefab_components": 12, "steel": 300, "components": 50 },
      "constructionTurns": 100,
      "powerConsumption": 2.0,
      "powerProduction": 0.0,
      "laborRequired": 6,
      "isEssential": false,
      "primarySkill": "Laborer",
      "maintenanceCost": { "steel": 0.5, "electronics": 0.1 },
      "storageCapacity": 1500
    },
    {
      "id": "ice_drill",
      "displayName": "Ice Drill",
      "description": "Bores into permafrost to extract raw ice for water processing.",
      "category": "Production",
      "size": { "width": 2, "height": 3 },
      "constructionCost": { "prefab_components": 4, "steel": 80, "electronics": 15 },
      "constructionTurns": 50,
      "powerConsumption": 4.0,
      "powerProduction": 0.0,
      "laborRequired": 4,
      "isEssential": false,
      "primarySkill": "Laborer",
      "recipe": {
        "inputs": {},
        "outputs": { "ice": 8.0 },
        "turnsPerCycle": 5
      },
      "maintenanceCost": { "steel": 0.3 }
    },
    {
      "id": "nutrient_recycler",
      "displayName": "Nutrient Recycler",
      "description": "Processes organic waste and regolith into hydroponic nutrient solution.",
      "category": "Production",
      "size": { "width": 2, "height": 2 },
      "constructionCost": { "prefab_components": 3, "steel": 60, "electronics": 20 },
      "constructionTurns": 40,
      "powerConsumption": 2.0,
      "powerProduction": 0.0,
      "laborRequired": 3,
      "isEssential": false,
      "primarySkill": "Farmer",
      "recipe": {
        "inputs": {},
        "outputs": { "nutrients": 5.0 },
        "turnsPerCycle": 10
      },
      "maintenanceCost": { "steel": 0.1, "electronics": 0.1 }
    }
  ]
}
""";

    public const string EventsJson = """
{
  "events": [
    {
      "id": "dust_storm",
      "title": "Dust Storm",
      "description": "Atmospheric dust reduces solar output to a fraction of normal for several sols.",
      "severity": "Warning",
      "trigger": { "type": "Strategic", "probability": 0.25, "minSol": 0 },
      "outcomes": [
        { "kind": "dust_storm", "duration": 10 },
        { "kind": "log", "message": "Dust storm! Solar output at 20% for 10 sols." }
      ]
    },
    {
      "id": "equipment_failure",
      "title": "Equipment Failure",
      "description": "A random operational building suffers catastrophic damage.",
      "severity": "Warning",
      "trigger": { "type": "Strategic", "probability": 0.18, "minSol": 0 },
      "outcomes": [
        { "kind": "damage_random" },
        { "kind": "log", "message": "Equipment failure — production halted on a building." }
      ]
    },
    {
      "id": "supply_drop",
      "title": "Supply Drop",
      "description": "An automated supply mission arrives at the colony.",
      "severity": "Info",
      "trigger": { "type": "Strategic", "probability": 0.06, "minSol": 0 },
      "outcomes": [
        { "kind": "resource_change", "resource": "steel",       "amount": 200.0 },
        { "kind": "resource_change", "resource": "electronics", "amount": 100.0 },
        { "kind": "resource_change", "resource": "nutrients",   "amount":  50.0 },
        { "kind": "log", "message": "Supply drop! +200 steel, +100 electronics, +50 nutrients." }
      ]
    },
    {
      "id": "mysterious_signal",
      "title": "Mysterious Signal",
      "description": "Long-range sensors detect an encoded transmission of unknown origin drifting from beyond the colony perimeter.",
      "severity": "Warning",
      "trigger": {
        "type": "Strategic", "probability": 0.12, "minSol": 0,
        "requiredResources": { "electronics": 50.0 }
      },
      "choices": [
        {
          "label": "Investigate the signal",
          "consequenceSummary": "−50 electronics, possible components recovered",
          "outcomes": [
            { "kind": "resource_change", "resource": "electronics", "amount": -50.0 },
            { "kind": "resource_change", "resource": "components",  "amount": 120.0 },
            { "kind": "log", "message": "Signal decoded — recovered 120 components from a derelict module." }
          ]
        },
        {
          "label": "Ignore — focus on colony work",
          "consequenceSummary": "no effect",
          "outcomes": [
            { "kind": "log", "message": "Mysterious signal faded into static. Crew refocused on routine tasks." }
          ]
        }
      ]
    }
  ]
}
""";

    public const string TechJson = """
{
  "tech": [
    {
      "id": "basic_construction",
      "displayName": "Basic Construction",
      "category": "Engineering",
      "tier": 1,
      "researchCost": 100.0,
      "researchTurns": 50,
      "prerequisites": [],
      "unlocks": {
        "buildings": ["basic_habitat", "warehouse"],
        "bonuses": ["construction_speed_10_percent"]
      },
      "description": "Foundational construction techniques for simple buildings."
    },
    {
      "id": "metallurgy",
      "displayName": "Metallurgy",
      "category": "Engineering",
      "tier": 1,
      "researchCost": 150.0,
      "researchTurns": 60,
      "prerequisites": [],
      "unlocks": {
        "buildings": ["iron_mine", "smelter"]
      },
      "description": "Ore extraction and refining processes."
    },
    {
      "id": "improved_solar",
      "displayName": "Improved Solar",
      "category": "Engineering",
      "tier": 2,
      "researchCost": 400.0,
      "researchTurns": 100,
      "prerequisites": ["basic_construction"],
      "unlocks": {
        "buildings": ["solar_array_mk2"]
      },
      "description": "Higher-efficiency photovoltaics that double Mk1 output."
    },
    {
      "id": "expanded_logistics",
      "displayName": "Expanded Logistics",
      "category": "Engineering",
      "tier": 2,
      "researchCost": 450.0,
      "researchTurns": 110,
      "prerequisites": ["basic_construction"],
      "unlocks": {
        "bonuses": ["fleet_cap_plus_5"]
      },
      "description": "Streamlined dispatch and depots raise the colony's construction fleet capacity by 5."
    },
    {
      "id": "habitat_engineering",
      "displayName": "Habitat Engineering",
      "category": "Engineering",
      "tier": 2,
      "researchCost": 500.0,
      "researchTurns": 120,
      "prerequisites": ["basic_construction"],
      "unlocks": {
        "buildings": ["basic_habitat_mk2"],
        "bonuses": ["housing_capacity_plus_50_percent"]
      },
      "description": "Pressurised modular construction for larger, more comfortable habitats."
    },
    {
      "id": "advanced_materials",
      "displayName": "Advanced Materials",
      "category": "Physics",
      "tier": 2,
      "researchCost": 600.0,
      "researchTurns": 140,
      "prerequisites": ["metallurgy"],
      "unlocks": {
        "resources": ["carbon_fiber", "rare_earth_metals"]
      },
      "description": "Carbon composites and rare-earth alloys for high-end manufacturing."
    },
    {
      "id": "fusion_basics",
      "displayName": "Fusion Fundamentals",
      "category": "Physics",
      "tier": 3,
      "researchCost": 2000.0,
      "researchTurns": 300,
      "prerequisites": ["improved_solar", "advanced_materials"],
      "unlocks": {
        "bonuses": ["fusion_research_unlocked"]
      },
      "description": "Theoretical foundation for fusion power generation."
    }
  ]
}
""";

    public const string ResourcesJson = """
{
  "resources": [
    { "id": "iron_ore",             "displayName": "Iron Ore",             "tier": "Raw",      "category": "Mineral",      "baseWeight":   10.0, "description": "Raw iron ore extracted from deposits." },
    { "id": "copper_ore",           "displayName": "Copper Ore",           "tier": "Raw",      "category": "Mineral",      "baseWeight":   12.0, "description": "Chalcopyrite and other copper-bearing minerals." },
    { "id": "silicon_ore",          "displayName": "Silicon Ore",          "tier": "Raw",      "category": "Mineral",      "baseWeight":    8.0, "description": "Silicate minerals processed into pure silicon." },
    { "id": "ice",                  "displayName": "Ice",                  "tier": "Raw",      "category": "Chemical",     "baseWeight":    2.0, "description": "Frozen water from polar regions and shadowed craters." },
    { "id": "regolith",             "displayName": "Regolith",             "tier": "Raw",      "category": "Mineral",      "baseWeight":    1.0, "description": "Loose surface material covering bedrock." },
    { "id": "uranium_ore",          "displayName": "Uranium Ore",          "tier": "Raw",      "category": "Energy",       "baseWeight":  150.0, "description": "Radioactive uranium-bearing ore." },
    { "id": "carbon_compounds",     "displayName": "Carbon Compounds",     "tier": "Raw",      "category": "Chemical",     "baseWeight":   15.0, "description": "Organic carbon deposits including hydrocarbons." },
    { "id": "rare_earth_ore",       "displayName": "Rare Earth Ore",       "tier": "Raw",      "category": "Mineral",      "baseWeight":   80.0, "description": "Ore containing lanthanides and other rare earth elements." },

    { "id": "iron",                 "displayName": "Iron",                 "tier": "Refined",  "category": "Manufactured", "baseWeight":   30.0, "description": "Refined iron metal." },
    { "id": "steel",                "displayName": "Steel",                "tier": "Refined",  "category": "Manufactured", "baseWeight":   50.0, "description": "Iron-carbon alloy for structural use." },
    { "id": "copper",               "displayName": "Copper",               "tier": "Refined",  "category": "Manufactured", "baseWeight":   40.0, "description": "Refined copper metal." },
    { "id": "silicon_wafer",        "displayName": "Silicon Wafer",        "tier": "Refined",  "category": "Manufactured", "baseWeight":  120.0, "description": "High-purity silicon processed into wafers." },
    { "id": "water",                "displayName": "Water",                "tier": "Refined",  "category": "Chemical",     "baseWeight":    1.0, "description": "H2O - essential for life support and agriculture." },
    { "id": "uranium_fuel_rod",     "displayName": "Uranium Fuel Rod",     "tier": "Refined",  "category": "Energy",       "baseWeight": 5000.0, "description": "Enriched uranium fuel rod for nuclear reactors." },
    { "id": "carbon_fiber",         "displayName": "Carbon Fiber",         "tier": "Refined",  "category": "Manufactured", "baseWeight":  180.0, "description": "High-strength carbon composite material." },
    { "id": "rare_earth_metals",    "displayName": "Rare Earth Metals",    "tier": "Refined",  "category": "Manufactured", "baseWeight":  500.0, "description": "Refined lanthanides for advanced electronics." },

    { "id": "prefab_components",    "displayName": "Prefab Components",    "tier": "Refined",  "category": "Manufactured", "baseWeight":   80.0, "description": "Modular structural kits used in nearly every constructible building." },

    { "id": "structural_components","displayName": "Structural Components","tier": "Advanced", "category": "Manufactured", "baseWeight":  150.0, "description": "Prefabricated beams, panels, and supports." },
    { "id": "electronics",          "displayName": "Electronics",          "tier": "Advanced", "category": "Manufactured", "baseWeight":  200.0, "description": "Electronic components, circuits, and control systems." },
    { "id": "machine_parts",        "displayName": "Machine Parts",        "tier": "Advanced", "category": "Manufactured", "baseWeight":  120.0, "description": "Mechanical components including motors, pumps, and valves." },
    { "id": "construction_materials","displayName": "Construction Materials","tier": "Advanced", "category": "Manufactured", "baseWeight":   80.0, "description": "Mixed building materials including fasteners and sealants." },

    { "id": "oxygen",               "displayName": "Oxygen",               "tier": "Refined",  "category": "Chemical",     "baseWeight":    5.0, "description": "O2 gas essential for life support." },
    { "id": "food",                 "displayName": "Food",                 "tier": "Refined",  "category": "Biological",   "baseWeight":   20.0, "description": "Processed food for human consumption." },
    { "id": "medical_supplies",     "displayName": "Medical Supplies",     "tier": "Refined",  "category": "Biological",   "baseWeight":  300.0, "description": "Pharmaceuticals, bandages, and diagnostic equipment." },
    { "id": "nutrients",            "displayName": "Nutrients",            "tier": "Refined",  "category": "Biological",   "baseWeight":   15.0, "description": "Mineral nutrients for hydroponic farming." },

    { "id": "credits",              "displayName": "Credits",              "tier": "Virtual",  "category": "Virtual",      "baseWeight":    1.0, "description": "Universal currency." },
    { "id": "research_data",        "displayName": "Research Data",        "tier": "Virtual",  "category": "Virtual",      "baseWeight":  100.0, "description": "Scientific data used for technological advancement." }
  ]
}
""";
}
