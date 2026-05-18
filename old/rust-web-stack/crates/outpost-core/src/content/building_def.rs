use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Category of building - determines behavior and UI grouping
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingCategory {
    PowerGeneration,
    Mining,
    Refining,
    Manufacturing,
    Farming,
    Storage,
    Housing,
    Services,
    Entertainment,
    Education,
    Healthcare,
    Infrastructure,
}

/// Power requirement specification for a building
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerRequirement {
    /// Base power consumption in megawatts
    pub base_mw: f64,
    /// Additional power per operational level
    pub per_level_mw: f64,
}

/// Input requirement for a production recipe
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeInput {
    pub resource_id: String,
    pub amount_per_tick: f64,
}

/// Output produced by a production recipe
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeOutput {
    pub resource_id: String,
    pub amount_per_tick: f64,
}

/// A production recipe that a building can perform
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductionRecipe {
    pub id: String,
    pub name: String,
    pub description: String,
    pub inputs: Vec<RecipeInput>,
    pub outputs: Vec<RecipeOutput>,
    /// Processing time in ticks
    pub duration_ticks: u64,
}

/// Construction cost for a building
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructionCost {
    pub resource_id: String,
    pub amount: f64,
}

/// Complete definition of a building type loaded from YAML
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingDefinition {
    /// Unique identifier (e.g., "solar_array_mk1")
    pub id: String,
    
    /// Display name
    pub name: String,
    
    /// Detailed description for UI
    pub description: String,
    
    /// Category determines behavior and grouping
    pub category: BuildingCategory,
    
    /// Construction costs
    pub construction_costs: Vec<ConstructionCost>,
    
    /// Time to construct in ticks
    pub construction_time_ticks: u64,
    
    /// Power requirement (consumption or generation if negative)
    pub power: Option<PowerRequirement>,
    
    /// Power generation capacity (MW) - only for power plants
    pub power_output_mw: Option<f64>,
    
    /// Worker capacity
    pub worker_capacity: u32,
    
    /// Storage capacity (for warehouses)
    pub storage_capacity: Option<f64>,
    
    /// Housing capacity (for housing buildings)
    pub housing_capacity: Option<u32>,
    
    /// Production recipes this building can perform
    pub recipes: Vec<ProductionRecipe>,
    
    /// Maintenance cost per tick
    pub maintenance_cost: HashMap<String, f64>,
    
    /// Can be upgraded
    pub upgradeable: bool,
    
    /// Next tier building ID if upgradeable
    pub upgrade_to: Option<String>,
    
    /// Morale bonus provided by this building
    pub morale_bonus: f64,
}

impl BuildingDefinition {
    /// Validate that this building definition is complete and consistent
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Building ID cannot be empty".to_string());
        }
        
        if self.name.is_empty() {
            return Err(format!("Building {} has empty name", self.id));
        }
        
        if self.construction_time_ticks == 0 {
            return Err(format!("Building {} has zero construction time", self.id));
        }
        
        // Validate that power plants have power_output_mw
        if self.category == BuildingCategory::PowerGeneration && self.power_output_mw.is_none() {
            return Err(format!("Power generation building {} missing power_output_mw", self.id));
        }
        
        // Validate recipes
        for recipe in &self.recipes {
            if recipe.id.is_empty() {
                return Err(format!("Building {} has recipe with empty ID", self.id));
            }
            if recipe.inputs.is_empty() && recipe.outputs.is_empty() {
                return Err(format!("Recipe {} has no inputs or outputs", recipe.id));
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_building_definition_validation() {
        let valid_building = BuildingDefinition {
            id: "solar_array".to_string(),
            name: "Solar Array".to_string(),
            description: "Generates power from sunlight".to_string(),
            category: BuildingCategory::PowerGeneration,
            construction_costs: vec![
                ConstructionCost {
                    resource_id: "steel".to_string(),
                    amount: 100.0,
                }
            ],
            construction_time_ticks: 100,
            power: None,
            power_output_mw: Some(10.0),
            worker_capacity: 2,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::new(),
            upgradeable: true,
            upgrade_to: Some("solar_array_mk2".to_string()),
            morale_bonus: 0.0,
        };

        assert!(valid_building.validate().is_ok());
    }

    #[test]
    fn test_building_definition_missing_name() {
        let invalid_building = BuildingDefinition {
            id: "test".to_string(),
            name: "".to_string(),
            description: "Test".to_string(),
            category: BuildingCategory::Manufacturing,
            construction_costs: vec![],
            construction_time_ticks: 100,
            power: None,
            power_output_mw: None,
            worker_capacity: 0,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        };

        assert!(invalid_building.validate().is_err());
    }
}
