use super::{BuildingDefinition, ResourceDefinition, EventDefinition, TechDefinition};
use crate::domain::Recipe;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("Failed to parse YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),
    
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("Duplicate ID found: {0}")]
    DuplicateId(String),
}

/// Loads and validates game content from YAML strings (I/O-free)
/// File loading should be done by the server layer, which then passes YAML strings to this loader
#[derive(Clone)]
pub struct ContentLoader {
    buildings: HashMap<String, BuildingDefinition>,
    resources: HashMap<String, ResourceDefinition>,
    events: HashMap<String, EventDefinition>,
    techs: HashMap<String, TechDefinition>,
    recipes: HashMap<String, Recipe>,
}

impl ContentLoader {
    /// Create a new empty content loader
    pub fn new() -> Self {
        Self {
            buildings: HashMap::new(),
            resources: HashMap::new(),
            events: HashMap::new(),
            techs: HashMap::new(),
            recipes: HashMap::new(),
        }
    }
    
    /// Load a building definition from a YAML string
    pub fn load_building(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let building: BuildingDefinition = serde_yaml::from_str(yaml_content)?;
        
        building.validate()
            .map_err(|e| ContentError::ValidationError(e))?;
        
        if self.buildings.contains_key(&building.id) {
            return Err(ContentError::DuplicateId(building.id.clone()));
        }
        
        self.buildings.insert(building.id.clone(), building);
        Ok(())
    }
    
    /// Load multiple building definitions from a YAML string
    pub fn load_buildings(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let buildings: Vec<BuildingDefinition> = serde_yaml::from_str(yaml_content)?;
        
        for building in buildings {
            building.validate()
                .map_err(|e| ContentError::ValidationError(e))?;
            
            if self.buildings.contains_key(&building.id) {
                return Err(ContentError::DuplicateId(building.id.clone()));
            }
            
            self.buildings.insert(building.id.clone(), building);
        }
        
        Ok(())
    }
    
    /// Load a resource definition from a YAML string
    pub fn load_resource(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let resource: ResourceDefinition = serde_yaml::from_str(yaml_content)?;
        
        resource.validate()
            .map_err(|e| ContentError::ValidationError(e))?;
        
        if self.resources.contains_key(&resource.id) {
            return Err(ContentError::DuplicateId(resource.id.clone()));
        }
        
        self.resources.insert(resource.id.clone(), resource);
        Ok(())
    }
    
    /// Load multiple resource definitions from a YAML string
    pub fn load_resources(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let resources: Vec<ResourceDefinition> = serde_yaml::from_str(yaml_content)?;
        
        for resource in resources {
            resource.validate()
                .map_err(|e| ContentError::ValidationError(e))?;
            
            if self.resources.contains_key(&resource.id) {
                return Err(ContentError::DuplicateId(resource.id.clone()));
            }
            
            self.resources.insert(resource.id.clone(), resource);
        }
        
        Ok(())
    }
    
    /// Load an event definition from a YAML string
    pub fn load_event(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let event: EventDefinition = serde_yaml::from_str(yaml_content)?;
        
        event.validate()
            .map_err(|e| ContentError::ValidationError(e))?;
        
        if self.events.contains_key(&event.id) {
            return Err(ContentError::DuplicateId(event.id.clone()));
        }
        
        self.events.insert(event.id.clone(), event);
        Ok(())
    }
    
    /// Load multiple event definitions from a YAML string
    pub fn load_events(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let events: Vec<EventDefinition> = serde_yaml::from_str(yaml_content)?;
        
        for event in events {
            event.validate()
                .map_err(|e| ContentError::ValidationError(e))?;
            
            if self.events.contains_key(&event.id) {
                return Err(ContentError::DuplicateId(event.id.clone()));
            }
            
            self.events.insert(event.id.clone(), event);
        }
        
        Ok(())
    }
    
    /// Get a building definition by ID
    pub fn get_building(&self, id: &str) -> Option<&BuildingDefinition> {
        self.buildings.get(id)
    }
    
    /// Get a resource definition by ID
    pub fn get_resource(&self, id: &str) -> Option<&ResourceDefinition> {
        self.resources.get(id)
    }
    
    /// Get an event definition by ID
    pub fn get_event(&self, id: &str) -> Option<&EventDefinition> {
        self.events.get(id)
    }
    
    /// Get all building definitions
    pub fn all_buildings(&self) -> &HashMap<String, BuildingDefinition> {
        &self.buildings
    }
    
    /// Get all resource definitions
    pub fn all_resources(&self) -> &HashMap<String, ResourceDefinition> {
        &self.resources
    }
    
    /// Get all event definitions
    pub fn all_events(&self) -> &HashMap<String, EventDefinition> {
        &self.events
    }
    
    /// Get all building definitions as an iterator of values
    pub fn all_building_defs(&self) -> impl Iterator<Item = &BuildingDefinition> {
        self.buildings.values()
    }

    /// Get all resource definitions as an iterator of values
    pub fn all_resource_defs(&self) -> impl Iterator<Item = &ResourceDefinition> {
        self.resources.values()
    }

    /// Get all event definitions as an iterator of values
    pub fn all_event_defs(&self) -> impl Iterator<Item = &EventDefinition> {
        self.events.values()
    }

    /// Get all tech definitions as an iterator of values
    pub fn all_tech_defs(&self) -> impl Iterator<Item = &TechDefinition> {
        self.techs.values()
    }

    /// Load a tech definition from a YAML string
    pub fn load_tech(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let tech: TechDefinition = serde_yaml::from_str(yaml_content)?;
        
        tech.validate()
            .map_err(|e| ContentError::ValidationError(e))?;
        
        if self.techs.contains_key(&tech.id) {
            return Err(ContentError::DuplicateId(tech.id.clone()));
        }
        
        self.techs.insert(tech.id.clone(), tech);
        Ok(())
    }
    
    /// Load multiple tech definitions from a YAML string
    pub fn load_techs(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let techs: Vec<TechDefinition> = serde_yaml::from_str(yaml_content)?;
        
        for tech in techs {
            tech.validate()
                .map_err(|e| ContentError::ValidationError(e))?;
            
            if self.techs.contains_key(&tech.id) {
                return Err(ContentError::DuplicateId(tech.id.clone()));
            }
            
            self.techs.insert(tech.id.clone(), tech);
        }
        
        Ok(())
    }
    
    /// Get a tech definition by ID
    pub fn get_tech(&self, id: &str) -> Option<&TechDefinition> {
        self.techs.get(id)
    }
    
    /// Get all tech definitions
    pub fn all_techs(&self) -> &HashMap<String, TechDefinition> {
        &self.techs
    }
    
    /// Load a recipe from a YAML string
    pub fn load_recipe(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        let recipe: Recipe = serde_yaml::from_str(yaml_content)?;
        
        recipe.validate()
            .map_err(|e| ContentError::ValidationError(e.to_string()))?;
        
        if self.recipes.contains_key(&recipe.id) {
            return Err(ContentError::DuplicateId(recipe.id.clone()));
        }
        
        self.recipes.insert(recipe.id.clone(), recipe);
        Ok(())
    }
    
    /// Load multiple recipes from a YAML string
    pub fn load_recipes(&mut self, yaml_content: &str) -> Result<(), ContentError> {
        #[derive(serde::Deserialize)]
        struct RecipeFile {
            recipes: Vec<Recipe>,
        }
        
        let file: RecipeFile = serde_yaml::from_str(yaml_content)?;
        
        for recipe in file.recipes {
            recipe.validate()
                .map_err(|e| ContentError::ValidationError(e.to_string()))?;
            
            if self.recipes.contains_key(&recipe.id) {
                return Err(ContentError::DuplicateId(recipe.id.clone()));
            }
            
            self.recipes.insert(recipe.id.clone(), recipe);
        }
        
        Ok(())
    }
    
    /// Get a recipe by ID
    pub fn get_recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.get(id)
    }
    
    /// Get all recipes
    pub fn all_recipes(&self) -> &HashMap<String, Recipe> {
        &self.recipes
    }
    
    /// Get count of loaded content
    pub fn stats(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.buildings.len(),
            self.resources.len(),
            self.events.len(),
            self.techs.len(),
            self.recipes.len(),
        )
    }
}

impl Default for ContentLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::building_def::{BuildingCategory, ConstructionCost};
    use crate::content::resource_def::{ResourceCategory, ResourcePhase, StorageRequirements};

    #[test]
    fn test_content_loader_buildings() {
        let mut loader = ContentLoader::new();
        
        let building = BuildingDefinition {
            id: "test_building".to_string(),
            name: "Test Building".to_string(),
            description: "A test".to_string(),
            category: BuildingCategory::Manufacturing,
            construction_costs: vec![
                ConstructionCost {
                    resource_id: "steel".to_string(),
                    amount: 100.0,
                }
            ],
            construction_time_ticks: 100,
            power: None,
            power_output_mw: None,
            worker_capacity: 5,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        };
        
        let yaml = serde_yaml::to_string(&building).unwrap();
        let result = loader.load_building(&yaml);
        assert!(result.is_ok());
        assert_eq!(loader.all_buildings().len(), 1);
    }

    #[test]
    fn test_content_loader_resources() {
        let mut loader = ContentLoader::new();
        
        let resource = ResourceDefinition {
            id: "test_resource".to_string(),
            name: "Test Resource".to_string(),
            description: "A test".to_string(),
            category: ResourceCategory::Metal,
            storage: StorageRequirements {
                phase: ResourcePhase::Solid,
                min_temperature_k: None,
                max_temperature_k: None,
                min_pressure_atm: None,
                max_pressure_atm: None,
                hazardous: false,
                special_handling: None,
            },
            density_kg_per_m3: 7500.0,
            base_value: 50.0,
            tradeable: true,
            extractable: false,
            consumable: false,
            stack_size: 0,
        };
        
        let yaml = serde_yaml::to_string(&resource).unwrap();
        let result = loader.load_resource(&yaml);
        assert!(result.is_ok());
        assert_eq!(loader.all_resources().len(), 1);
    }

    #[test]
    fn test_load_events_yaml() {
        let yaml = std::fs::read_to_string("../../content/events/narrative_events.yaml")
            .expect("Failed to read narrative_events.yaml");
        let mut loader = ContentLoader::new();
        let result = loader.load_events(&yaml);
        assert!(result.is_ok(), "Failed to load events: {:?}", result.err());
        assert!(
            loader.all_events().len() >= 15,
            "Expected at least 15 events, got {}",
            loader.all_events().len()
        );
    }

    #[test]
    fn test_event_category_coverage() {
        let yaml = std::fs::read_to_string("../../content/events/narrative_events.yaml")
            .expect("Failed to read narrative_events.yaml");
        let mut loader = ContentLoader::new();
        loader.load_events(&yaml).unwrap();

        use crate::content::EventCategory;
        let events = loader.all_events();
        let has_disaster  = events.values().any(|e| e.category == EventCategory::Disaster);
        let has_discovery = events.values().any(|e| e.category == EventCategory::Discovery);
        let has_social    = events.values().any(|e| e.category == EventCategory::Social);
        let has_technical = events.values().any(|e| e.category == EventCategory::Technical);
        let has_economic  = events.values().any(|e| e.category == EventCategory::Economic);

        assert!(has_disaster,  "No Disaster events found");
        assert!(has_discovery, "No Discovery events found");
        assert!(has_social,    "No Social events found");
        assert!(has_technical, "No Technical events found");
        assert!(has_economic,  "No Economic events found");
    }

    #[test]
    fn test_all_events_have_valid_choices() {
        let yaml = std::fs::read_to_string("../../content/events/narrative_events.yaml")
            .expect("Failed to read narrative_events.yaml");
        let mut loader = ContentLoader::new();
        loader.load_events(&yaml).unwrap();

        for (id, event) in loader.all_events() {
            assert!(
                !event.choices.is_empty(),
                "Event '{}' has no choices",
                id
            );
            for choice in &event.choices {
                assert!(
                    !choice.id.is_empty(),
                    "Event '{}' has a choice with empty ID",
                    id
                );
            }
        }
    }
}
