namespace OutpostGame.Tests;

using OutpostGame.Core.Colony;
using OutpostGame.Core.Content;

[TestFixture]
public class ContentLoaderTests
{
    private static readonly string[] ExpectedBuildingIds =
    {
        "solar_array_mk1", "nuclear_reactor",
        "iron_mine", "ice_drill", "nutrient_recycler",
        "smelter", "fabricator",
        "water_purifier", "life_support_system", "hydroponics_bay",
        "basic_habitat", "warehouse", "large_warehouse",
    };

    private static readonly string[] ExpectedResourceIds =
    {
        "iron_ore", "copper_ore", "silicon_ore", "ice", "regolith",
        "uranium_ore", "carbon_compounds", "rare_earth_ore",
        "iron", "steel", "copper", "silicon_wafer", "water",
        "uranium_fuel_rod", "carbon_fiber", "rare_earth_metals",
        "structural_components", "electronics", "machine_parts",
        "construction_materials", "oxygen", "food", "medical_supplies",
        "nutrients", "credits", "research_data",
    };

    [Test]
    public void BuildingRegistry_ResolvesAllExpectedIds()
    {
        foreach (var id in ExpectedBuildingIds)
        {
            Assert.That(BuildingRegistry.TryGet(id, out var def), Is.True,
                $"Building '{id}' should resolve in registry");
            Assert.That(def, Is.Not.Null);
            Assert.That(def!.DisplayName, Is.Not.Empty);
        }
    }

    [Test]
    public void ResourceRegistry_ResolvesAllExpectedIds()
    {
        foreach (var id in ExpectedResourceIds)
        {
            Assert.That(ResourceRegistry.TryGet(id, out var def), Is.True,
                $"Resource '{id}' should resolve in registry");
            Assert.That(def, Is.Not.Null);
            Assert.That(def!.DisplayName, Is.Not.Empty);
        }
    }

    [Test]
    public void LoadBuildings_ParsesMinimalDefinition()
    {
        var json = """
        {
          "buildings": [
            {
              "id": "test_widget",
              "displayName": "Test Widget",
              "category": "Storage",
              "size": { "width": 1, "height": 1 },
              "constructionCost": { "steel": 10 },
              "constructionTurns": 5,
              "powerConsumption": 1.0,
              "powerProduction": 0.0,
              "laborRequired": 0,
              "isEssential": false
            }
          ]
        }
        """;

        var defs = ContentLoader.LoadBuildings(json);
        Assert.That(defs.Count, Is.EqualTo(1));
        var def = defs["test_widget"];
        Assert.That(def.Category,          Is.EqualTo(BuildingCategory.Storage));
        Assert.That(def.Size.Width,        Is.EqualTo(1));
        Assert.That(def.ConstructionTurns, Is.EqualTo(5));
        Assert.That(def.PowerConsumption,  Is.EqualTo(1.0f));
        Assert.That(def.Recipe,            Is.Null);
        Assert.That(def.MaintenanceCost,   Is.Null);
    }

    [Test]
    public void LoadBuildings_ParsesRecipeAndMaintenance()
    {
        var json = """
        {
          "buildings": [
            {
              "id": "test_factory",
              "displayName": "Test Factory",
              "category": "Production",
              "size": { "width": 2, "height": 2 },
              "constructionCost": { "steel": 10 },
              "constructionTurns": 5,
              "powerConsumption": 2.0,
              "powerProduction": 0.0,
              "laborRequired": 4,
              "isEssential": false,
              "primarySkill": "Engineer",
              "recipe": {
                "inputs":  { "iron_ore": 2.0 },
                "outputs": { "steel": 1.0 },
                "turnsPerCycle": 3
              },
              "maintenanceCost": { "steel": 0.1 }
            }
          ]
        }
        """;

        var def = ContentLoader.LoadBuildings(json)["test_factory"];
        Assert.That(def.PrimarySkill,           Is.EqualTo(SkillType.Engineer));
        Assert.That(def.Recipe,                 Is.Not.Null);
        Assert.That(def.Recipe!.TurnsPerCycle,  Is.EqualTo(3));
        Assert.That(def.Recipe.Inputs["iron_ore"],  Is.EqualTo(2.0f));
        Assert.That(def.Recipe.Outputs["steel"],    Is.EqualTo(1.0f));
        Assert.That(def.MaintenanceCost!["steel"], Is.EqualTo(0.1f));
    }

    [Test]
    public void LoadBuildings_RejectsDuplicateIds()
    {
        var json = """
        {
          "buildings": [
            { "id": "dup", "displayName": "A", "category": "Storage",
              "size": { "width": 1, "height": 1 }, "constructionCost": {},
              "constructionTurns": 1, "laborRequired": 0, "isEssential": false },
            { "id": "dup", "displayName": "B", "category": "Storage",
              "size": { "width": 1, "height": 1 }, "constructionCost": {},
              "constructionTurns": 1, "laborRequired": 0, "isEssential": false }
          ]
        }
        """;

        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadBuildings(json));
    }

    [Test]
    public void LoadBuildings_RejectsMissingId()
    {
        var json = """
        {
          "buildings": [
            { "displayName": "Nameless", "category": "Storage",
              "size": { "width": 1, "height": 1 }, "constructionCost": {},
              "constructionTurns": 1, "laborRequired": 0, "isEssential": false }
          ]
        }
        """;

        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadBuildings(json));
    }

    [Test]
    public void LoadBuildings_RejectsInvalidSize()
    {
        var json = """
        {
          "buildings": [
            { "id": "tiny", "displayName": "Tiny", "category": "Storage",
              "size": { "width": 0, "height": 1 }, "constructionCost": {},
              "constructionTurns": 1, "laborRequired": 0, "isEssential": false }
          ]
        }
        """;

        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadBuildings(json));
    }

    [Test]
    public void LoadBuildings_RejectsInvalidEnum()
    {
        var json = """
        {
          "buildings": [
            { "id": "x", "displayName": "X", "category": "Bogus",
              "size": { "width": 1, "height": 1 }, "constructionCost": {},
              "constructionTurns": 1, "laborRequired": 0, "isEssential": false }
          ]
        }
        """;

        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadBuildings(json));
    }

    [Test]
    public void LoadResources_ParsesMinimalDefinition()
    {
        var json = """
        {
          "resources": [
            { "id": "test_dust", "displayName": "Test Dust",
              "tier": "Raw", "category": "Mineral", "baseWeight": 5.0 }
          ]
        }
        """;

        var def = ContentLoader.LoadResources(json)["test_dust"];
        Assert.That(def.Tier,       Is.EqualTo(ResourceTier.Raw));
        Assert.That(def.Category,   Is.EqualTo(ResourceCategory.Mineral));
        Assert.That(def.BaseWeight, Is.EqualTo(5.0f));
        Assert.That(def.Description, Is.EqualTo(""));
    }

    [Test]
    public void LoadResources_RejectsDuplicateIds()
    {
        var json = """
        {
          "resources": [
            { "id": "dup", "displayName": "A", "tier": "Raw",
              "category": "Mineral", "baseWeight": 1 },
            { "id": "dup", "displayName": "B", "tier": "Raw",
              "category": "Mineral", "baseWeight": 1 }
          ]
        }
        """;

        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadResources(json));
    }

    [Test]
    public void LoadBuildings_FailsOnMalformedJson()
    {
        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadBuildings("{ not valid json"));
    }

    [Test]
    public void TechRegistry_LoadsExpectedTechs()
    {
        Assert.That(TechRegistry.All.Count, Is.GreaterThan(0));
        Assert.That(TechRegistry.TryGet("basic_construction", out var def), Is.True);
        Assert.That(def!.Tier, Is.EqualTo(1));
        Assert.That(def.Prerequisites, Is.Empty);
        Assert.That(def.Unlocks.Buildings, Does.Contain("basic_habitat"));
    }

    [Test]
    public void TechRegistry_PreservesPrerequisiteChain()
    {
        var fusion = TechRegistry.Get("fusion_basics");
        Assert.That(fusion.Prerequisites, Does.Contain("improved_solar"));
        Assert.That(fusion.Prerequisites, Does.Contain("advanced_materials"));
        Assert.That(fusion.Tier, Is.EqualTo(3));
    }

    [Test]
    public void LoadTech_RejectsUnknownPrerequisite()
    {
        var json = """
        {
          "tech": [
            { "id": "branch", "displayName": "Branch", "category": "Engineering",
              "tier": 2, "researchCost": 10, "researchTurns": 5,
              "prerequisites": ["nope"], "unlocks": {} }
          ]
        }
        """;
        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadTech(json));
    }

    [Test]
    public void LoadTech_RejectsTierZero()
    {
        var json = """
        {
          "tech": [
            { "id": "x", "displayName": "X", "category": "Engineering",
              "tier": 0, "researchCost": 1, "researchTurns": 1,
              "prerequisites": [], "unlocks": {} }
          ]
        }
        """;
        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadTech(json));
    }

    [Test]
    public void EventRegistry_LoadsExpectedEvents()
    {
        Assert.That(EventRegistry.All.Count, Is.GreaterThan(0));
        Assert.That(EventRegistry.TryGet("dust_storm",        out _), Is.True);
        Assert.That(EventRegistry.TryGet("equipment_failure", out _), Is.True);
        Assert.That(EventRegistry.TryGet("supply_drop",       out _), Is.True);
        Assert.That(EventRegistry.TryGet("mysterious_signal", out _), Is.True);
    }

    [Test]
    public void EventRegistry_ParsesTriggerAndOutcomes()
    {
        var drop = EventRegistry.Get("supply_drop");
        Assert.That(drop.Trigger.Type,        Is.EqualTo(EventTriggerType.Strategic));
        Assert.That(drop.Trigger.Probability, Is.EqualTo(0.06f).Within(0.001f));
        Assert.That(drop.Severity,            Is.EqualTo(ColonyEventSeverity.Info));
        Assert.That(drop.Outcomes,            Has.Count.GreaterThan(0));
        Assert.That(drop.Outcomes.Any(o => o.Kind == "resource_change"
                                        && o.Resource == "steel"), Is.True);
    }

    [Test]
    public void EventRegistry_ParsesChoicesWithOutcomes()
    {
        var signal = EventRegistry.Get("mysterious_signal");
        Assert.That(signal.Choices,        Has.Count.EqualTo(2));
        Assert.That(signal.Choices[0].Outcomes, Has.Count.GreaterThan(0));
        Assert.That(signal.Trigger.RequiredResources["electronics"], Is.EqualTo(50.0f));
    }

    [Test]
    public void LoadEvents_RejectsUnknownOutcomeKind()
    {
        var json = """
        {
          "events": [
            { "id": "x", "title": "X", "severity": "Info",
              "trigger": { "type": "Strategic", "probability": 0.1, "minSol": 0 },
              "outcomes": [ { "kind": "do_a_barrel_roll" } ] }
          ]
        }
        """;
        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadEvents(json));
    }

    [Test]
    public void LoadEvents_RejectsOutOfRangeProbability()
    {
        var json = """
        {
          "events": [
            { "id": "x", "title": "X", "severity": "Info",
              "trigger": { "type": "Strategic", "probability": 1.5, "minSol": 0 },
              "outcomes": [] }
          ]
        }
        """;
        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadEvents(json));
    }

    [Test]
    public void LoadTech_RejectsDuplicateIds()
    {
        var json = """
        {
          "tech": [
            { "id": "dup", "displayName": "A", "category": "Engineering",
              "tier": 1, "researchCost": 1, "researchTurns": 1,
              "prerequisites": [], "unlocks": {} },
            { "id": "dup", "displayName": "B", "category": "Engineering",
              "tier": 1, "researchCost": 1, "researchTurns": 1,
              "prerequisites": [], "unlocks": {} }
          ]
        }
        """;
        Assert.Throws<ContentLoader.ContentValidationException>(
            () => ContentLoader.LoadTech(json));
    }
}
