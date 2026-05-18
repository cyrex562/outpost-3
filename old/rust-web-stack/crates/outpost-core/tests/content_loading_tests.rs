/// Integration tests for the data-driven content system
/// These tests verify that YAML content files can be loaded and validated

use outpost_core::content::{ContentLoader, BuildingCategory, ResourceCategory, ResourcePhase};

#[test]
fn test_load_basic_buildings() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/buildings/basic_buildings.yaml")
        .expect("Failed to read buildings YAML file");
    
    let result = loader.load_buildings(&yaml_content);
    assert!(result.is_ok(), "Failed to load buildings: {:?}", result.err());
    
    let stats = loader.stats();
    assert!(stats.0 > 0, "No buildings were loaded");
    
    // Verify specific buildings
    let solar_array = loader.get_building("solar_array_mk1");
    assert!(solar_array.is_some(), "Solar array not found");
    
    if let Some(building) = solar_array {
        assert_eq!(building.name, "Solar Array Mk1");
        assert_eq!(building.category, BuildingCategory::PowerGeneration);
        assert!(building.power_output_mw.is_some());
        assert_eq!(building.power_output_mw.unwrap(), 10.0);
    }
    
    let mine = loader.get_building("iron_mine");
    assert!(mine.is_some(), "Iron mine not found");
    
    if let Some(building) = mine {
        assert_eq!(building.name, "Iron Mine");
        assert_eq!(building.category, BuildingCategory::Mining);
        assert!(!building.recipes.is_empty(), "Mine should have recipes");
        assert_eq!(building.recipes[0].id, "extract_iron_ore");
    }
    
    let habitat = loader.get_building("basic_habitat");
    assert!(habitat.is_some(), "Habitat not found");
    
    if let Some(building) = habitat {
        assert!(building.housing_capacity.is_some());
        assert_eq!(building.housing_capacity.unwrap(), 20);
        assert_eq!(building.morale_bonus, 5.0);
    }
}

#[test]
fn test_load_basic_resources() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/resources/basic_resources.yaml")
        .expect("Failed to read resources YAML file");
    
    let result = loader.load_resources(&yaml_content);
    assert!(result.is_ok(), "Failed to load resources: {:?}", result.err());
    
    let stats = loader.stats();
    assert!(stats.1 > 0, "No resources were loaded");
    
    // Verify specific resources
    let iron_ore = loader.get_resource("iron_ore");
    assert!(iron_ore.is_some(), "Iron ore not found");
    
    if let Some(resource) = iron_ore {
        assert_eq!(resource.name, "Iron Ore");
        assert_eq!(resource.category, ResourceCategory::MetallicOre);
        assert_eq!(resource.storage.phase, ResourcePhase::Solid);
        assert!(resource.extractable);
        assert!(!resource.consumable);
    }
    
    let water = loader.get_resource("water");
    assert!(water.is_some(), "Water not found");
    
    if let Some(resource) = water {
        assert_eq!(resource.storage.phase, ResourcePhase::Liquid);
        assert!(resource.consumable);
        assert_eq!(resource.density_kg_per_m3, 1000.0);
    }
    
    let steel = loader.get_resource("steel");
    assert!(steel.is_some(), "Steel not found");
    
    if let Some(resource) = steel {
        assert_eq!(resource.category, ResourceCategory::Metal);
        assert!(!resource.extractable);
        assert!(resource.tradeable);
    }
    
    let food = loader.get_resource("food");
    assert!(food.is_some(), "Food not found");
    
    if let Some(resource) = food {
        assert_eq!(resource.category, ResourceCategory::BiologicalProduct);
        assert!(resource.consumable);
        assert!(resource.storage.max_temperature_k.is_some());
    }
}

#[test]
fn test_mvp_resource_coverage() {
    // Test that all MVP resources from Task 3.1 are defined
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/resources/basic_resources.yaml")
        .expect("Failed to read resources YAML file");
    
    loader.load_resources(&yaml_content).expect("Failed to load resources");
    
    // Required RAW materials (Tier 0) - 8 resources
    let raw_resources = vec![
        "iron_ore", "copper_ore", "silicon_ore", "ice", 
        "regolith", "uranium_ore", "carbon_compounds", "rare_earth_ore"
    ];
    for resource_id in raw_resources {
        assert!(loader.get_resource(resource_id).is_some(), 
            "Missing raw resource: {}", resource_id);
        let resource = loader.get_resource(resource_id).unwrap();
        assert!(resource.extractable, "{} should be extractable", resource_id);
    }
    
    // Required REFINED materials (Tier 1) - 8 resources
    let refined_resources = vec![
        "iron", "steel", "copper", "silicon_wafer",
        "water", "uranium_fuel_rod", "carbon_fiber", "rare_earth_metals"
    ];
    for resource_id in refined_resources {
        assert!(loader.get_resource(resource_id).is_some(), 
            "Missing refined resource: {}", resource_id);
        let resource = loader.get_resource(resource_id).unwrap();
        assert!(!resource.extractable, "{} should not be extractable (refined)", resource_id);
    }
    
    // Required MANUFACTURED components (Tier 2) - 4 resources
    let manufactured_resources = vec![
        "structural_components", "electronics", 
        "machine_parts", "construction_materials"
    ];
    for resource_id in manufactured_resources {
        assert!(loader.get_resource(resource_id).is_some(), 
            "Missing manufactured resource: {}", resource_id);
    }
    
    // Required CONSUMABLES (Tier 3) - 4 resources
    let consumable_resources = vec![
        "food", "oxygen", "medical_supplies", "nutrients"
    ];
    for resource_id in consumable_resources {
        assert!(loader.get_resource(resource_id).is_some(), 
            "Missing consumable resource: {}", resource_id);
    }
    
    // Virtual resources (currency, data)
    assert!(loader.get_resource("credits").is_some(), "Missing credits");
    assert!(loader.get_resource("research_data").is_some(), "Missing research_data");
    
    // Verify total count (should be at least 26)
    let stats = loader.stats();
    assert!(stats.1 >= 26, "Expected at least 26 resources, found {}", stats.1);
    
    println!("✓ All {} MVP resources loaded successfully", stats.1);
}

#[test]
fn test_load_recipes() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/recipes.yaml")
        .expect("Failed to read recipes YAML file");
    
    let result = loader.load_recipes(&yaml_content);
    assert!(result.is_ok(), "Failed to load recipes: {:?}", result.err());
    
    let stats = loader.stats();
    assert!(stats.4 > 0, "No recipes were loaded");
    
    // Verify extraction recipes
    let mine_iron = loader.get_recipe("mine_iron");
    assert!(mine_iron.is_some(), "mine_iron recipe not found");
    
    if let Some(recipe) = mine_iron {
        assert_eq!(recipe.name, "Mine Iron Ore");
        assert!(recipe.requires_deposit());
        assert_eq!(recipe.required_deposit_resource(), Some("iron_ore"));
        assert!(recipe.inputs.contains_key("labor"));
        assert!(recipe.inputs.contains_key("power"));
        assert!(recipe.outputs.contains_key("iron_ore"));
        assert_eq!(recipe.processing_time_ticks, 1);
        assert!(recipe.is_compatible_with_building("mine"));
    }
    
    // Verify refining recipes
    let smelt_iron = loader.get_recipe("smelt_iron");
    assert!(smelt_iron.is_some(), "smelt_iron recipe not found");
    
    if let Some(recipe) = smelt_iron {
        assert_eq!(recipe.name, "Smelt Iron");
        assert!(!recipe.requires_deposit());
        assert!(recipe.inputs.contains_key("iron_ore"));
        assert!(recipe.inputs.contains_key("power"));
        assert!(recipe.outputs.contains_key("iron_ingots"));
        assert_eq!(recipe.processing_time_ticks, 2);
        assert!(recipe.is_compatible_with_building("smelter"));
    }
    
    // Verify manufacturing recipes
    let fabricate = loader.get_recipe("fabricate_structural_components");
    assert!(fabricate.is_some(), "fabricate_structural_components recipe not found");
    
    if let Some(recipe) = fabricate {
        assert_eq!(recipe.name, "Fabricate Structural Components");
        assert!(recipe.inputs.contains_key("iron_ingots"));
        assert!(recipe.inputs.contains_key("aluminum_ingots"));
        assert!(recipe.outputs.contains_key("structural_components"));
        assert!(recipe.is_compatible_with_building("fabricator"));
    }
    
    // Verify life support recipes
    let purify_water = loader.get_recipe("purify_water");
    assert!(purify_water.is_some(), "purify_water recipe not found");
    
    if let Some(recipe) = purify_water {
        assert!(recipe.inputs.contains_key("ice"));
        assert!(recipe.outputs.contains_key("water"));
    }
    
    let generate_oxygen = loader.get_recipe("generate_oxygen");
    assert!(generate_oxygen.is_some(), "generate_oxygen recipe not found");
    
    if let Some(recipe) = generate_oxygen {
        assert!(recipe.inputs.contains_key("water"));
        assert!(recipe.outputs.contains_key("oxygen"));
        assert!(recipe.outputs.contains_key("hydrogen"));
    }
    
    // Verify that all loaded recipes pass validation
    for (id, recipe) in loader.all_recipes() {
        assert!(recipe.validate().is_ok(), "Recipe {} failed validation", id);
    }
    
    println!("✓ All {} recipes loaded and validated successfully", stats.4);
}

#[test]
fn test_recipe_coverage() {
    // Test that we have recipes for key MVP production chains
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/recipes.yaml")
        .expect("Failed to read recipes YAML file");
    
    loader.load_recipes(&yaml_content).expect("Failed to load recipes");
    
    // Extraction recipes (raw materials)
    let extraction_recipes = vec![
        "mine_iron", "mine_copper", "mine_silicon",
        "extract_ice", "harvest_regolith",
    ];
    for recipe_id in extraction_recipes {
        assert!(loader.get_recipe(recipe_id).is_some(), 
            "Missing extraction recipe: {}", recipe_id);
    }
    
    // Refining recipes (raw → refined)
    let refining_recipes = vec![
        "smelt_iron", "smelt_copper", "purify_water",
        "refine_silicon", "generate_oxygen",
    ];
    for recipe_id in refining_recipes {
        assert!(loader.get_recipe(recipe_id).is_some(), 
            "Missing refining recipe: {}", recipe_id);
    }
    
    // Manufacturing recipes (refined → components)
    let manufacturing_recipes = vec![
        "fabricate_structural_components",
        "fabricate_machine_parts",
        "fabricate_electronics",
    ];
    for recipe_id in manufacturing_recipes {
        assert!(loader.get_recipe(recipe_id).is_some(), 
            "Missing manufacturing recipe: {}", recipe_id);
    }
    
    // Life support recipes
    let life_support_recipes = vec![
        "purify_water", "generate_oxygen", "grow_food",
    ];
    for recipe_id in life_support_recipes {
        assert!(loader.get_recipe(recipe_id).is_some(), 
            "Missing life support recipe: {}", recipe_id);
    }
    
    // At least 20 recipes should be defined for MVP
    let stats = loader.stats();
    assert!(stats.4 >= 20, "Expected at least 20 MVP recipes, got {}", stats.4);
    
    println!("✓ Recipe coverage test passed: {} recipes covering all production chains", stats.4);
}

#[test]
fn test_load_narrative_events() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/events/narrative_events.yaml")
        .expect("Failed to read events YAML file");
    
    let result = loader.load_events(&yaml_content);
    assert!(result.is_ok(), "Failed to load events: {:?}", result.err());
    
    let stats = loader.stats();
    assert!(stats.2 > 0, "No events were loaded");
    
    // Verify specific events
    let first_colonists = loader.get_event("first_colonists_arrive");
    assert!(first_colonists.is_some(), "First colonists event not found");
    
    if let Some(event) = first_colonists {
        assert_eq!(event.title, "The First Colonists");
        assert!(!event.triggers.is_empty());
        assert!(!event.choices.is_empty());
        assert_eq!(event.choices.len(), 2);
        assert!(!event.repeatable);
    }
    
    let mineral_deposit = loader.get_event("mineral_deposit_discovered");
    assert!(mineral_deposit.is_some(), "Mineral deposit event not found");
    
    if let Some(event) = mineral_deposit {
        assert!(event.repeatable);
        assert!(event.cooldown_ticks.is_some());
        assert_eq!(event.choices.len(), 3);
    }
    
    let morale_crisis = loader.get_event("morale_crisis");
    assert!(morale_crisis.is_some(), "Morale crisis event not found");
    
    if let Some(event) = morale_crisis {
        assert_eq!(event.choices.len(), 3);
        assert!(event.repeatable);
    }
}

#[test]
fn test_load_all_content() {
    let mut loader = ContentLoader::new();
    
    // Load all buildings
    let buildings_yaml = std::fs::read_to_string("../../content/buildings/basic_buildings.yaml")
        .expect("Failed to read buildings");
    loader.load_buildings(&buildings_yaml).expect("Failed to load buildings");
    
    // Load all resources
    let resources_yaml = std::fs::read_to_string("../../content/resources/basic_resources.yaml")
        .expect("Failed to read resources");
    loader.load_resources(&resources_yaml).expect("Failed to load resources");
    
    // Load all events
    let events_yaml = std::fs::read_to_string("../../content/events/narrative_events.yaml")
        .expect("Failed to read events");
    loader.load_events(&events_yaml).expect("Failed to load events");
    
    // Load all techs
    let techs_yaml = std::fs::read_to_string("../../content/tech/basic_tech_tree.yaml")
        .expect("Failed to read techs");
    loader.load_techs(&techs_yaml).expect("Failed to load techs");
    
    let (buildings, resources, events, techs, recipes) = loader.stats();
    
    println!("Loaded content:");
    println!("  - Buildings: {}", buildings);
    println!("  - Resources: {}", resources);
    println!("  - Events: {}", events);
    println!("  - Technologies: {}", techs);
    println!("  - Recipes: {}", recipes);
    
    assert!(buildings > 0, "No buildings loaded");
    assert!(resources > 0, "No resources loaded");
    assert!(events > 0, "No events loaded");
    assert!(techs > 0, "No techs loaded");
}

#[test]
fn test_building_validation() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/buildings/basic_buildings.yaml")
        .expect("Failed to read buildings");
    
    // All buildings should pass validation
    let result = loader.load_buildings(&yaml_content);
    assert!(result.is_ok(), "Building validation failed: {:?}", result.err());
    
    // Verify each building individually
    for (id, building) in loader.all_buildings() {
        let validation = building.validate();
        assert!(validation.is_ok(), "Building {} failed validation: {:?}", id, validation.err());
    }
}

#[test]
fn test_resource_validation() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/resources/basic_resources.yaml")
        .expect("Failed to read resources");
    
    // All resources should pass validation
    let result = loader.load_resources(&yaml_content);
    assert!(result.is_ok(), "Resource validation failed: {:?}", result.err());
    
    // Verify each resource individually
    for (id, resource) in loader.all_resources() {
        let validation = resource.validate();
        assert!(validation.is_ok(), "Resource {} failed validation: {:?}", id, validation.err());
    }
}

#[test]
fn test_event_validation() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/events/narrative_events.yaml")
        .expect("Failed to read events");
    
    // All events should pass validation
    let result = loader.load_events(&yaml_content);
    assert!(result.is_ok(), "Event validation failed: {:?}", result.err());
    
    // Verify each event individually
    for (id, event) in loader.all_events() {
        let validation = event.validate();
        assert!(validation.is_ok(), "Event {} failed validation: {:?}", id, validation.err());
    }
}

#[test]
fn test_recipe_outputs() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/buildings/basic_buildings.yaml")
        .expect("Failed to read buildings");
    loader.load_buildings(&yaml_content).unwrap();
    
    let smelter = loader.get_building("smelter").expect("Smelter not found");
    assert!(!smelter.recipes.is_empty());
    
    let smelt_iron = &smelter.recipes[0];
    assert_eq!(smelt_iron.id, "smelt_iron");
    assert_eq!(smelt_iron.inputs.len(), 1);
    assert_eq!(smelt_iron.outputs.len(), 1);
    
    // Verify conversion: 10 iron ore -> 5 steel
    assert_eq!(smelt_iron.inputs[0].resource_id, "iron_ore");
    assert_eq!(smelt_iron.inputs[0].amount_per_tick, 10.0);
    assert_eq!(smelt_iron.outputs[0].resource_id, "steel");
    assert_eq!(smelt_iron.outputs[0].amount_per_tick, 5.0);
}

#[test]
fn test_building_upgrades() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/buildings/basic_buildings.yaml")
        .expect("Failed to read buildings");
    loader.load_buildings(&yaml_content).unwrap();
    
    let solar_array = loader.get_building("solar_array_mk1").unwrap();
    assert!(solar_array.upgradeable);
    assert_eq!(solar_array.upgrade_to, Some("solar_array_mk2".to_string()));
    
    let iron_mine = loader.get_building("iron_mine").unwrap();
    assert!(iron_mine.upgradeable);
    assert_eq!(iron_mine.upgrade_to, Some("iron_mine_mk2".to_string()));
}

#[test]
fn test_resource_volume_calculations() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/resources/basic_resources.yaml")
        .expect("Failed to read resources");
    loader.load_resources(&yaml_content).unwrap();
    
    let water = loader.get_resource("water").unwrap();
    
    // 1000 kg of water = 1 m³ (density = 1000 kg/m³)
    assert_eq!(water.volume_from_mass(1000.0), 1.0);
    assert_eq!(water.mass_from_volume(1.0), 1000.0);
    
    let steel = loader.get_resource("steel").unwrap();
    
    // Steel is denser: 7850 kg/m³
    assert_eq!(steel.density_kg_per_m3, 7850.0);
    assert!((steel.volume_from_mass(7850.0) - 1.0).abs() < 0.001);
}

#[test]
fn test_load_basic_tech_tree() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/tech/basic_tech_tree.yaml")
        .expect("Failed to read tech YAML file");
    
    let result = loader.load_techs(&yaml_content);
    assert!(result.is_ok(), "Failed to load techs: {:?}", result.err());
    
    let stats = loader.stats();
    assert!(stats.3 > 0, "No techs were loaded");
    
    // Verify tier 1 tech with no prerequisites
    let basic_construction = loader.get_tech("basic_construction");
    assert!(basic_construction.is_some(), "Basic construction tech not found");
    
    if let Some(tech) = basic_construction {
        assert_eq!(tech.name, "Basic Construction");
        assert_eq!(tech.tier, 1);
        assert!(tech.prerequisites.is_empty());
        assert!(tech.has_unlocks());
        assert!(tech.unlocks.buildings.contains(&"basic_habitat".to_string()));
    }
    
    // Verify tier 2 tech with prerequisites
    let advanced_materials = loader.get_tech("advanced_materials");
    assert!(advanced_materials.is_some(), "Advanced materials tech not found");
    
    if let Some(tech) = advanced_materials {
        assert_eq!(tech.tier, 2);
        assert!(tech.has_prerequisites());
        assert!(tech.prerequisites.contains(&"resource_extraction".to_string()));
        assert!(tech.unlocks.buildings.contains(&"smelter".to_string()));
        assert!(tech.unlocks.resources.contains(&"steel".to_string()));
    }
    
    // Verify tier 3 tech with resource costs
    let automation = loader.get_tech("automation");
    assert!(automation.is_some(), "Automation tech not found");
    
    if let Some(tech) = automation {
        assert_eq!(tech.tier, 3);
        assert!(!tech.resource_costs.is_empty());
        assert_eq!(tech.resource_costs[0].resource_id, "electronics");
        assert_eq!(tech.resource_costs[0].amount, 50.0);
    }
}

#[test]
fn test_tech_validation() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/tech/basic_tech_tree.yaml")
        .expect("Failed to read tech file");
    
    loader.load_techs(&yaml_content)
        .expect("Tech validation failed");
    
    // All loaded techs should be valid
    for tech in loader.all_techs().values() {
        assert!(tech.validate().is_ok(), "Tech {} failed validation", tech.id);
        assert!(!tech.id.is_empty());
        assert!(!tech.name.is_empty());
        assert!(tech.research_cost > 0.0);
        assert!(tech.research_time_ticks > 0);
        assert!(tech.tier >= 1);
    }
}

#[test]
fn test_tech_prerequisites_graph() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/tech/basic_tech_tree.yaml")
        .expect("Failed to read tech file");
    
    loader.load_techs(&yaml_content).unwrap();
    
    // Verify prerequisite relationships form a valid DAG
    // Fusion basics requires both improved solar and advanced materials
    let fusion_basics = loader.get_tech("fusion_basics").unwrap();
    assert_eq!(fusion_basics.prerequisites.len(), 2);
    assert!(fusion_basics.prerequisites.contains(&"improved_solar".to_string()));
    assert!(fusion_basics.prerequisites.contains(&"advanced_materials".to_string()));
    
    // Improved solar requires basic power (tier 1)
    let improved_solar = loader.get_tech("improved_solar").unwrap();
    assert_eq!(improved_solar.prerequisites.len(), 1);
    assert_eq!(improved_solar.prerequisites[0], "basic_power");
    
    // Basic power has no prerequisites (tier 1)
    let basic_power = loader.get_tech("basic_power").unwrap();
    assert!(basic_power.prerequisites.is_empty());
}

#[test]
fn test_tech_unlock_count() {
    let mut loader = ContentLoader::new();
    
    let yaml_content = std::fs::read_to_string("../../content/tech/basic_tech_tree.yaml")
        .expect("Failed to read tech file");
    
    loader.load_techs(&yaml_content).unwrap();
    
    let basic_construction = loader.get_tech("basic_construction").unwrap();
    assert!(basic_construction.unlock_count() > 0);
    
    let advanced_materials = loader.get_tech("advanced_materials").unwrap();
    // Should unlock buildings and resources
    assert!(advanced_materials.unlock_count() >= 3);
    
    let genetic_engineering = loader.get_tech("genetic_engineering").unwrap();
    // Should unlock bonuses and events
    assert!(genetic_engineering.has_unlocks());
}

