use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur when working with recipes
#[derive(Error, Debug, Clone, PartialEq)]
pub enum RecipeError {
    #[error("Recipe validation failed: {0}")]
    ValidationFailed(String),

    #[error("Recipe {0} not found")]
    RecipeNotFound(String),

    #[error("Insufficient input resource: {resource}, need {needed}, have {available}")]
    InsufficientInput {
        resource: String,
        needed: f64,
        available: f64,
    },

    #[error("Insufficient storage for output: {resource}, would produce {amount}, only {available} space")]
    InsufficientStorage {
        resource: String,
        amount: f64,
        available: f64,
    },

    #[error("Building {building_type} cannot run recipe {recipe_id}")]
    IncompatibleBuilding {
        building_type: String,
        recipe_id: String,
    },

    #[error("Required deposit {resource} not found or depleted")]
    MissingDeposit { resource: String },
}

/// A production recipe that transforms inputs into outputs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recipe {
    /// Unique identifier for this recipe
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Building types that can execute this recipe (e.g., "mine", "smelter")
    pub building_types: Vec<String>,

    /// Resources consumed when recipe starts (resource_id -> quantity in metric tons)
    pub inputs: HashMap<String, f64>,

    /// Resources produced when recipe completes (resource_id -> quantity in metric tons)
    pub outputs: HashMap<String, f64>,

    /// Number of ticks required to complete this recipe
    pub processing_time_ticks: u64,

    /// If set, this recipe requires a deposit of this resource type on the celestial body
    /// Used for extraction recipes (mining, drilling, etc.)
    #[serde(default)]
    pub requires_deposit: Option<String>,

    /// Optional description for UI tooltips
    #[serde(default)]
    pub description: Option<String>,
}

impl Recipe {
    /// Validates that the recipe has sensible values
    pub fn validate(&self) -> Result<(), RecipeError> {
        // ID must not be empty
        if self.id.trim().is_empty() {
            return Err(RecipeError::ValidationFailed(
                "Recipe ID cannot be empty".to_string(),
            ));
        }

        // Name must not be empty
        if self.name.trim().is_empty() {
            return Err(RecipeError::ValidationFailed(
                "Recipe name cannot be empty".to_string(),
            ));
        }

        // Must have at least one building type
        if self.building_types.is_empty() {
            return Err(RecipeError::ValidationFailed(format!(
                "Recipe '{}' must specify at least one building type",
                self.id
            )));
        }

        // Must have at least one input
        if self.inputs.is_empty() {
            return Err(RecipeError::ValidationFailed(format!(
                "Recipe '{}' must have at least one input",
                self.id
            )));
        }

        // Must have at least one output
        if self.outputs.is_empty() {
            return Err(RecipeError::ValidationFailed(format!(
                "Recipe '{}' must have at least one output",
                self.id
            )));
        }

        // All input quantities must be positive
        for (resource, quantity) in &self.inputs {
            if *quantity <= 0.0 {
                return Err(RecipeError::ValidationFailed(format!(
                    "Recipe '{}' has non-positive input quantity for '{}': {}",
                    self.id, resource, quantity
                )));
            }
        }

        // All output quantities must be positive
        for (resource, quantity) in &self.outputs {
            if *quantity <= 0.0 {
                return Err(RecipeError::ValidationFailed(format!(
                    "Recipe '{}' has non-positive output quantity for '{}': {}",
                    self.id, resource, quantity
                )));
            }
        }

        // Processing time must be at least 1 tick
        if self.processing_time_ticks == 0 {
            return Err(RecipeError::ValidationFailed(format!(
                "Recipe '{}' has zero processing time",
                self.id
            )));
        }

        Ok(())
    }

    /// Calculate total input mass (for resource conservation checks)
    /// Virtual resources (power, labor) with zero density are excluded
    pub fn input_mass(&self, resource_densities: &HashMap<String, f64>) -> f64 {
        self.inputs
            .iter()
            .map(|(resource_id, quantity)| {
                let density = resource_densities.get(resource_id).copied().unwrap_or(1.0);
                if density > 0.0 {
                    quantity * density
                } else {
                    0.0 // Exclude virtual resources
                }
            })
            .sum()
    }

    /// Calculate total output mass (for resource conservation checks)
    /// Virtual resources (power, labor) with zero density are excluded
    pub fn output_mass(&self, resource_densities: &HashMap<String, f64>) -> f64 {
        self.outputs
            .iter()
            .map(|(resource_id, quantity)| {
                let density = resource_densities.get(resource_id).copied().unwrap_or(1.0);
                if density > 0.0 {
                    quantity * density
                } else {
                    0.0 // Exclude virtual resources
                }
            })
            .sum()
    }

    /// Check if this recipe can be executed by the given building type
    pub fn is_compatible_with_building(&self, building_type: &str) -> bool {
        self.building_types
            .iter()
            .any(|bt| bt == building_type)
    }

    /// Check if this recipe requires a specific resource deposit
    pub fn requires_deposit(&self) -> bool {
        self.requires_deposit.is_some()
    }

    /// Get the required deposit resource ID, if any
    pub fn required_deposit_resource(&self) -> Option<&str> {
        self.requires_deposit.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_recipe() -> Recipe {
        Recipe {
            id: "test_recipe".to_string(),
            name: "Test Recipe".to_string(),
            building_types: vec!["test_building".to_string()],
            inputs: {
                let mut map = HashMap::new();
                map.insert("input_resource".to_string(), 10.0);
                map
            },
            outputs: {
                let mut map = HashMap::new();
                map.insert("output_resource".to_string(), 5.0);
                map
            },
            processing_time_ticks: 5,
            requires_deposit: None,
            description: Some("A test recipe".to_string()),
        }
    }

    #[test]
    fn test_valid_recipe_passes_validation() {
        let recipe = create_valid_recipe();
        assert!(recipe.validate().is_ok());
    }

    #[test]
    fn test_empty_id_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.id = "".to_string();
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_empty_name_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.name = "".to_string();
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_empty_building_types_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.building_types = vec![];
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_empty_inputs_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.inputs = HashMap::new();
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_empty_outputs_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.outputs = HashMap::new();
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_negative_input_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.inputs.insert("bad_input".to_string(), -5.0);
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_zero_input_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.inputs.insert("bad_input".to_string(), 0.0);
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_negative_output_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.outputs.insert("bad_output".to_string(), -5.0);
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_zero_processing_time_fails_validation() {
        let mut recipe = create_valid_recipe();
        recipe.processing_time_ticks = 0;
        assert!(matches!(
            recipe.validate(),
            Err(RecipeError::ValidationFailed(_))
        ));
    }

    #[test]
    fn test_input_mass_calculation() {
        let recipe = create_valid_recipe();
        let mut densities = HashMap::new();
        densities.insert("input_resource".to_string(), 2.5); // 10 tons * 2.5 = 25
        
        let mass = recipe.input_mass(&densities);
        assert_eq!(mass, 25.0);
    }

    #[test]
    fn test_output_mass_calculation() {
        let recipe = create_valid_recipe();
        let mut densities = HashMap::new();
        densities.insert("output_resource".to_string(), 3.0); // 5 tons * 3.0 = 15
        
        let mass = recipe.output_mass(&densities);
        assert_eq!(mass, 15.0);
    }

    #[test]
    fn test_mass_calculation_excludes_virtual_resources() {
        let mut recipe = create_valid_recipe();
        recipe.inputs.insert("power".to_string(), 100.0);
        recipe.outputs.insert("labor".to_string(), 50.0);

        let mut densities = HashMap::new();
        densities.insert("input_resource".to_string(), 2.5);
        densities.insert("power".to_string(), 0.0); // Virtual resource
        densities.insert("output_resource".to_string(), 3.0);
        densities.insert("labor".to_string(), 0.0); // Virtual resource

        assert_eq!(recipe.input_mass(&densities), 25.0); // Only physical input
        assert_eq!(recipe.output_mass(&densities), 15.0); // Only physical output
    }

    #[test]
    fn test_is_compatible_with_building() {
        let recipe = create_valid_recipe();
        assert!(recipe.is_compatible_with_building("test_building"));
        assert!(!recipe.is_compatible_with_building("other_building"));
    }

    #[test]
    fn test_requires_deposit() {
        let mut recipe = create_valid_recipe();
        assert!(!recipe.requires_deposit());
        assert_eq!(recipe.required_deposit_resource(), None);

        recipe.requires_deposit = Some("iron_ore".to_string());
        assert!(recipe.requires_deposit());
        assert_eq!(recipe.required_deposit_resource(), Some("iron_ore"));
    }
}
