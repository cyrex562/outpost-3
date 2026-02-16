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
    
    let (buildings, resources, events, techs) = loader.stats();
    
    println!("Loaded content:");
    println!("  - Buildings: {}", buildings);
    println!("  - Resources: {}", resources);
    println!("  - Events: {}", events);
    println!("  - Technologies: {}", techs);
    
    assert!(buildings > 0, "No buildings loaded");
    assert!(resources > 0, "No resources loaded");
    assert!(events > 0, "No events loaded");
    assert!(techs > 0, "No techs loaded");
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
