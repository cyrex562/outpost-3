//! Resource mapping layer — bridges string-based resource IDs (V5) with ResourceType enum (legacy).
//!
//! This module provides bidirectional mapping between:
//! - String-based resource IDs used in YAML recipes (e.g., "iron_ore", "water")
//! - ResourceType enum used in the legacy Resources system
//!
//! This allows the data-driven recipe system to work with the existing Site/Resources infrastructure.

use crate::domain::ResourceType;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum MappingError {
    #[error("No ResourceType mapping found for string ID: {0}")]
    StringIdNotFound(String),

    #[error("No string ID mapping found for ResourceType: {0:?}")]
    ResourceTypeNotFound(ResourceType),

    #[error("Resource ID '{0}' is a virtual resource (power, labor) with no physical storage")]
    VirtualResource(String),
}

/// Check if a resource ID is virtual (power, labor) and has no physical representation.
pub fn is_virtual_resource(resource_id: &str) -> bool {
    matches!(resource_id, "power" | "labor")
}

/// Convert a string resource ID to ResourceType enum.
///
/// Returns None for virtual resources (power, labor) which have no enum representation.
pub fn string_to_resource_type(id: &str) -> Result<ResourceType, MappingError> {
    // Check for virtual resources first
    if is_virtual_resource(id) {
        return Err(MappingError::VirtualResource(id.to_string()));
    }

    // Direct mapping from string IDs to ResourceType enum
    let resource_type = match id {
        // Currency
        "credits" => ResourceType::Credits,

        // Energy
        "energy" => ResourceType::Energy,
        "solar_power" => ResourceType::SolarPower,
        "nuclear_fuel" => ResourceType::NuclearFuel,
        "battery_pack" => ResourceType::BatteryPack,
        "hydrogen_cell" => ResourceType::HydrogenCell,

        // Raw materials - Basic
        "iron_ore" => ResourceType::IronOre,
        "copper_ore" => ResourceType::CopperOre,
        "rare_metals" | "rare_earth_ore" | "rare_earth_metals" => ResourceType::RareMetals,
        "water" => ResourceType::Water,
        "timber" => ResourceType::Timber,
        "food" | "food_rations" => ResourceType::Food,
        "nutrients" => ResourceType::Nutrients,

        // Raw materials - Minerals
        "nickel" => ResourceType::Nickel,
        "cobalt" => ResourceType::Cobalt,
        "zinc" => ResourceType::Zinc,
        "silver" => ResourceType::Silver,
        "gold" => ResourceType::Gold,
        "lithium" => ResourceType::Lithium,
        "graphite" => ResourceType::Graphite,

        // Raw materials - Advanced
        "coal" => ResourceType::Coal,
        "coke" => ResourceType::Coke,
        "limestone" => ResourceType::Limestone,
        "uranium" | "uranium_ore" | "uranium_fuel_rod" => ResourceType::Uranium,
        "silicon" | "silicon_ore" | "silicon_wafer" | "silicon_wafers" => ResourceType::Silicon,
        "titanium" | "titanium_alloys" => ResourceType::Titanium,
        "platinum" => ResourceType::Platinum,
        "aluminum" | "aluminum_ingots" => ResourceType::Aluminum,
        "crystal_ore" => ResourceType::CrystalOre,
        "gas" => ResourceType::Gas,
        "oil" => ResourceType::Oil,
        "oxygen" => ResourceType::Oxygen,

        // Processed goods - Basic
        "steel" | "iron" | "iron_ingots" => ResourceType::Steel,
        "electronics" => ResourceType::Electronics,
        "machinery" | "machine_parts" => ResourceType::Machinery,
        "copper_wire" | "copper" | "copper_ingots" => ResourceType::CopperWire,

        // Processed goods - Advanced
        "concrete" | "regolith" => ResourceType::Concrete,
        "plastics" | "polymers" => ResourceType::Plastics,
        "fuel" => ResourceType::Fuel,
        "chemicals" | "carbon_compounds" | "carbon_fiber" => ResourceType::Chemicals,
        "advanced_components" | "structural_components" | "construction_materials" => {
            ResourceType::AdvancedComponents
        }
        "medicine" | "medical_supplies" => ResourceType::Medicine,

        // Specialized resources
        "research" | "research_data" => ResourceType::Research,
        "consumer_goods" => ResourceType::ConsumerGoods,
        "luxuries" => ResourceType::Luxuries,

        // Additional mappings for recipe-specific resources
        "ice" => ResourceType::Water, // Ice melts to water
        "hydrogen" => ResourceType::HydrogenCell, // Hydrogen stored as cells

        // Fallback
        _ => return Err(MappingError::StringIdNotFound(id.to_string())),
    };

    Ok(resource_type)
}

/// Convert a ResourceType enum to its canonical string ID.
pub fn resource_type_to_string(resource_type: ResourceType) -> &'static str {
    match resource_type {
        // Currency
        ResourceType::Credits => "credits",

        // Energy
        ResourceType::Energy => "energy",
        ResourceType::SolarPower => "solar_power",
        ResourceType::NuclearFuel => "nuclear_fuel",
        ResourceType::BatteryPack => "battery_pack",
        ResourceType::HydrogenCell => "hydrogen_cell",

        // Raw materials - Basic
        ResourceType::IronOre => "iron_ore",
        ResourceType::CopperOre => "copper_ore",
        ResourceType::RareMetals => "rare_earth_metals",
        ResourceType::Water => "water",
        ResourceType::Timber => "timber",
        ResourceType::Food => "food",
        ResourceType::Nutrients => "nutrients",

        // Raw materials - Minerals
        ResourceType::Nickel => "nickel",
        ResourceType::Cobalt => "cobalt",
        ResourceType::Zinc => "zinc",
        ResourceType::Silver => "silver",
        ResourceType::Gold => "gold",
        ResourceType::Lithium => "lithium",
        ResourceType::Graphite => "graphite",

        // Raw materials - Advanced
        ResourceType::Coal => "coal",
        ResourceType::Coke => "coke",
        ResourceType::Limestone => "limestone",
        ResourceType::Uranium => "uranium",
        ResourceType::Silicon => "silicon",
        ResourceType::Titanium => "titanium",
        ResourceType::Platinum => "platinum",
        ResourceType::Aluminum => "aluminum",
        ResourceType::CrystalOre => "crystal_ore",
        ResourceType::Gas => "gas",
        ResourceType::Oil => "oil",
        ResourceType::Oxygen => "oxygen",

        // Processed goods - Basic
        ResourceType::Steel => "iron_ingots",
        ResourceType::Electronics => "electronics",
        ResourceType::Machinery => "machine_parts",
        ResourceType::CopperWire => "copper_ingots",

        // Processed goods - Advanced
        ResourceType::Concrete => "regolith",
        ResourceType::Plastics => "polymers",
        ResourceType::Fuel => "fuel",
        ResourceType::Chemicals => "carbon_compounds",
        ResourceType::AdvancedComponents => "structural_components",
        ResourceType::Medicine => "medical_supplies",

        // Specialized resources
        ResourceType::Research => "research_data",
        ResourceType::ConsumerGoods => "consumer_goods",
        ResourceType::Luxuries => "luxuries",
    }
}

/// Build a complete bidirectional mapping between string IDs and ResourceType.
/// Useful for validation and debugging.
pub fn build_full_mapping() -> HashMap<String, ResourceType> {
    let mut map = HashMap::new();

    // Add all primary mappings
    let types = [
        ResourceType::Credits,
        ResourceType::Energy,
        ResourceType::SolarPower,
        ResourceType::NuclearFuel,
        ResourceType::BatteryPack,
        ResourceType::HydrogenCell,
        ResourceType::IronOre,
        ResourceType::CopperOre,
        ResourceType::RareMetals,
        ResourceType::Water,
        ResourceType::Timber,
        ResourceType::Food,
        ResourceType::Nutrients,
        ResourceType::Nickel,
        ResourceType::Cobalt,
        ResourceType::Zinc,
        ResourceType::Silver,
        ResourceType::Gold,
        ResourceType::Lithium,
        ResourceType::Graphite,
        ResourceType::Coal,
        ResourceType::Coke,
        ResourceType::Limestone,
        ResourceType::Uranium,
        ResourceType::Silicon,
        ResourceType::Titanium,
        ResourceType::Platinum,
        ResourceType::Aluminum,
        ResourceType::CrystalOre,
        ResourceType::Gas,
        ResourceType::Oil,
        ResourceType::Oxygen,
        ResourceType::Steel,
        ResourceType::Electronics,
        ResourceType::Machinery,
        ResourceType::CopperWire,
        ResourceType::Concrete,
        ResourceType::Plastics,
        ResourceType::Fuel,
        ResourceType::Chemicals,
        ResourceType::AdvancedComponents,
        ResourceType::Medicine,
        ResourceType::Research,
        ResourceType::ConsumerGoods,
        ResourceType::Luxuries,
    ];

    for resource_type in &types {
        let string_id = resource_type_to_string(*resource_type);
        map.insert(string_id.to_string(), *resource_type);
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_resources() {
        assert!(is_virtual_resource("power"));
        assert!(is_virtual_resource("labor"));
        assert!(!is_virtual_resource("iron_ore"));
        assert!(!is_virtual_resource("water"));
    }

    #[test]
    fn test_virtual_resource_mapping_fails() {
        assert!(matches!(
            string_to_resource_type("power"),
            Err(MappingError::VirtualResource(_))
        ));
        assert!(matches!(
            string_to_resource_type("labor"),
            Err(MappingError::VirtualResource(_))
        ));
    }

    #[test]
    fn test_basic_string_to_enum() {
        assert_eq!(
            string_to_resource_type("iron_ore").unwrap(),
            ResourceType::IronOre
        );
        assert_eq!(
            string_to_resource_type("water").unwrap(),
            ResourceType::Water
        );
        assert_eq!(
            string_to_resource_type("food").unwrap(),
            ResourceType::Food
        );
        assert_eq!(
            string_to_resource_type("oxygen").unwrap(),
            ResourceType::Oxygen
        );
    }

    #[test]
    fn test_alias_mapping() {
        // iron_ingots maps to Steel
        assert_eq!(
            string_to_resource_type("iron_ingots").unwrap(),
            ResourceType::Steel
        );
        assert_eq!(
            string_to_resource_type("iron").unwrap(),
            ResourceType::Steel
        );
        assert_eq!(
            string_to_resource_type("steel").unwrap(),
            ResourceType::Steel
        );

        // food_rations maps to Food
        assert_eq!(
            string_to_resource_type("food_rations").unwrap(),
            ResourceType::Food
        );

        // ice maps to Water
        assert_eq!(
            string_to_resource_type("ice").unwrap(),
            ResourceType::Water
        );
    }

    #[test]
    fn test_enum_to_string() {
        assert_eq!(resource_type_to_string(ResourceType::IronOre), "iron_ore");
        assert_eq!(resource_type_to_string(ResourceType::Water), "water");
        assert_eq!(resource_type_to_string(ResourceType::Food), "food");
        assert_eq!(resource_type_to_string(ResourceType::Steel), "iron_ingots");
    }

    #[test]
    fn test_roundtrip_mapping() {
        // For primary IDs, roundtrip should work
        let resource_type = ResourceType::IronOre;
        let string_id = resource_type_to_string(resource_type);
        let back_to_enum = string_to_resource_type(string_id).unwrap();
        assert_eq!(resource_type, back_to_enum);

        // Test a few more
        for resource_type in &[
            ResourceType::Water,
            ResourceType::Oxygen,
            ResourceType::Electronics,
            ResourceType::Uranium,
        ] {
            let string_id = resource_type_to_string(*resource_type);
            let back = string_to_resource_type(string_id).unwrap();
            assert_eq!(*resource_type, back);
        }
    }

    #[test]
    fn test_unknown_string_id() {
        let result = string_to_resource_type("unknown_resource_xyz");
        assert!(matches!(result, Err(MappingError::StringIdNotFound(_))));
    }

    #[test]
    fn test_recipe_resources_coverage() {
        // Test that all resources used in our recipes can be mapped
        let recipe_resources = vec![
            "iron_ore",
            "copper_ore",
            "silicon_ore",
            "ice",
            "regolith",
            "uranium_ore",
            "carbon_compounds",
            "rare_earth_ore",
            "iron_ingots",
            "copper_ingots",
            "silicon_wafers",
            "aluminum_ingots",
            "uranium_fuel_rod",
            "rare_earth_metals",
            "polymers",
            "titanium_alloys",
            "structural_components",
            "machine_parts",
            "electronics",
            "advanced_components",
            "water",
            "oxygen",
            "hydrogen",
            "food_rations",
            "nutrients",
            "medical_supplies",
        ];

        for resource_id in recipe_resources {
            let result = string_to_resource_type(resource_id);
            assert!(
                result.is_ok(),
                "Recipe resource '{}' should map to ResourceType",
                resource_id
            );
        }
    }

    #[test]
    fn test_build_full_mapping() {
        let mapping = build_full_mapping();

        // Should have entries for all non-virtual resources
        assert!(mapping.len() > 40, "Should have many resource mappings");

        // Spot check a few
        assert_eq!(mapping.get("iron_ore"), Some(&ResourceType::IronOre));
        assert_eq!(mapping.get("water"), Some(&ResourceType::Water));
        assert_eq!(mapping.get("oxygen"), Some(&ResourceType::Oxygen));
    }
}
