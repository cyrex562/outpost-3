use serde::{Deserialize, Serialize};

/// Category of resource - determines tier and processing requirements
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceCategory {
    // Tier 0: Raw Materials
    MetallicOre,
    NonMetallicMineral,
    Hydrocarbon,
    AtmosphericGas,
    Hydrospheric,
    Agricultural,
    Forestry,
    Aquaculture,
    Biological,
    
    // Tier 1: Processed Materials
    Metal,
    NonMetallicMaterial,
    Chemical,
    BiologicalProduct,
    EnergyStorage,
    LifeSupport,
    
    // Tier 2: Components
    Component,
    Electronics,
    MechanicalParts,
    
    // Tier 3: Finished Goods
    ConsumerGoods,
    IndustrialEquipment,
    Vehicle,
    
    // Special
    Currency,
    Data,
}

/// Physical phase of a resource
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourcePhase {
    Solid,
    Liquid,
    Gas,
    Plasma,
    Virtual, // For data, currency, etc.
}

/// Storage requirements for a resource
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageRequirements {
    /// Phase determines storage type
    pub phase: ResourcePhase,
    
    /// Temperature requirements (Kelvin)
    pub min_temperature_k: Option<f64>,
    pub max_temperature_k: Option<f64>,
    
    /// Pressure requirements (atmospheres)
    pub min_pressure_atm: Option<f64>,
    pub max_pressure_atm: Option<f64>,
    
    /// Is hazardous
    pub hazardous: bool,
    
    /// Special handling notes
    pub special_handling: Option<String>,
}

/// Complete definition of a resource type loaded from YAML
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceDefinition {
    /// Unique identifier (e.g., "iron_ore", "steel_ingot")
    pub id: String,
    
    /// Display name
    pub name: String,
    
    /// Detailed description
    pub description: String,
    
    /// Category determines tier and processing
    pub category: ResourceCategory,
    
    /// Storage requirements
    pub storage: StorageRequirements,
    
    /// Density (kg/m³) for volume calculations
    pub density_kg_per_m3: f64,
    
    /// Base market value (abstract currency)
    pub base_value: f64,
    
    /// Can be traded on markets
    pub tradeable: bool,
    
    /// Can be extracted from deposits
    pub extractable: bool,
    
    /// Is consumed by population
    pub consumable: bool,
    
    /// Stack size for discrete items (0 = continuous/bulk)
    pub stack_size: u32,
}

impl ResourceDefinition {
    /// Validate that this resource definition is complete and consistent
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Resource ID cannot be empty".to_string());
        }
        
        if self.name.is_empty() {
            return Err(format!("Resource {} has empty name", self.id));
        }
        
        // Virtual resources can have zero density; physical resources must have positive density
        if self.storage.phase != ResourcePhase::Virtual && self.density_kg_per_m3 <= 0.0 {
            return Err(format!("Resource {} has invalid density", self.id));
        }
        
        if self.base_value < 0.0 {
            return Err(format!("Resource {} has negative base value", self.id));
        }
        
        Ok(())
    }
    
    /// Calculate volume in m³ from mass in kg
    pub fn volume_from_mass(&self, mass_kg: f64) -> f64 {
        mass_kg / self.density_kg_per_m3
    }
    
    /// Calculate mass in kg from volume in m³
    pub fn mass_from_volume(&self, volume_m3: f64) -> f64 {
        volume_m3 * self.density_kg_per_m3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_definition_validation() {
        let valid_resource = ResourceDefinition {
            id: "iron_ore".to_string(),
            name: "Iron Ore".to_string(),
            description: "Raw iron ore extracted from deposits".to_string(),
            category: ResourceCategory::MetallicOre,
            storage: StorageRequirements {
                phase: ResourcePhase::Solid,
                min_temperature_k: None,
                max_temperature_k: None,
                min_pressure_atm: None,
                max_pressure_atm: None,
                hazardous: false,
                special_handling: None,
            },
            density_kg_per_m3: 5000.0,
            base_value: 10.0,
            tradeable: true,
            extractable: true,
            consumable: false,
            stack_size: 0,
        };

        assert!(valid_resource.validate().is_ok());
    }

    #[test]
    fn test_volume_calculations() {
        let resource = ResourceDefinition {
            id: "water".to_string(),
            name: "Water".to_string(),
            description: "H2O".to_string(),
            category: ResourceCategory::Hydrospheric,
            storage: StorageRequirements {
                phase: ResourcePhase::Liquid,
                min_temperature_k: Some(273.0),
                max_temperature_k: Some(373.0),
                min_pressure_atm: None,
                max_pressure_atm: None,
                hazardous: false,
                special_handling: None,
            },
            density_kg_per_m3: 1000.0,
            base_value: 1.0,
            tradeable: true,
            extractable: true,
            consumable: true,
            stack_size: 0,
        };

        assert_eq!(resource.volume_from_mass(1000.0), 1.0);
        assert_eq!(resource.mass_from_volume(1.0), 1000.0);
    }
}
