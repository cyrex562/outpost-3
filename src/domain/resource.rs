use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Add, AddAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    // Currency
    Credits,

    // Energy
    Energy,

    // Energy Systems
    SolarPower,
    NuclearFuel,
    BatteryPack,
    HydrogenCell,

    // Raw materials - Basic
    IronOre,
    CopperOre,
    RareMetals,
    Water,
    Timber,
    Food,
    Nutrients,

    // Raw materials - Minerals
    Nickel,
    Cobalt,
    Zinc,
    Silver,
    Gold,
    Lithium,
    Graphite,

    // Raw materials - Advanced
    Coal,
    Coke,
    Limestone,
    Uranium,
    Silicon,
    Titanium,
    Platinum,
    Aluminum,
    CrystalOre,
    Gas,
    Oil,
    Oxygen,

    // Processed goods - Basic
    Steel,
    Electronics,
    Machinery,
    CopperWire,

    // Processed goods - Advanced
    Concrete,
    Plastics,
    Fuel,
    Chemicals,
    AdvancedComponents,
    Medicine,

    // Specialized resources
    Research,
    ConsumerGoods,
    Luxuries,
}

impl ResourceType {
    pub fn name(&self) -> &'static str {
        match self {
            ResourceType::Credits => "Credits",
            ResourceType::Energy => "Energy",
            ResourceType::SolarPower => "Solar Power",
            ResourceType::NuclearFuel => "Nuclear Fuel",
            ResourceType::BatteryPack => "Battery Pack",
            ResourceType::HydrogenCell => "Hydrogen Cell",
            ResourceType::IronOre => "Iron Ore",
            ResourceType::CopperOre => "Copper Ore",
            ResourceType::RareMetals => "Rare Metals",
            ResourceType::Water => "Water",
            ResourceType::Timber => "Timber",
            ResourceType::Food => "Food",
            ResourceType::Nutrients => "Nutrients",
            ResourceType::Coal => "Coal",
            ResourceType::Coke => "Coke",
            ResourceType::Limestone => "Limestone",
            ResourceType::Uranium => "Uranium",
            ResourceType::Silicon => "Silicon",
            ResourceType::Titanium => "Titanium",
            ResourceType::Platinum => "Platinum",
            ResourceType::Aluminum => "Aluminum",
            ResourceType::Nickel => "Nickel",
            ResourceType::Cobalt => "Cobalt",
            ResourceType::Zinc => "Zinc",
            ResourceType::Silver => "Silver",
            ResourceType::Gold => "Gold",
            ResourceType::Lithium => "Lithium",
            ResourceType::Graphite => "Graphite",
            ResourceType::CrystalOre => "Crystal Ore",
            ResourceType::Gas => "Gas",
            ResourceType::Oil => "Oil",
            ResourceType::Steel => "Steel",
            ResourceType::Electronics => "Electronics",
            ResourceType::Machinery => "Machinery",
            ResourceType::CopperWire => "Copper Wire",
            ResourceType::Concrete => "Concrete",
            ResourceType::Plastics => "Plastics",
            ResourceType::Fuel => "Fuel",
            ResourceType::Chemicals => "Chemicals",
            ResourceType::AdvancedComponents => "Advanced Components",
            ResourceType::Medicine => "Medicine",
            ResourceType::Research => "Research",
            ResourceType::ConsumerGoods => "Consumer Goods",
            ResourceType::Luxuries => "Luxuries",
            ResourceType::Oxygen => "Oxygen",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            ResourceType::Credits => "Currency",
            ResourceType::Energy
            | ResourceType::SolarPower
            | ResourceType::NuclearFuel
            | ResourceType::BatteryPack
            | ResourceType::HydrogenCell => "Energy",
            ResourceType::IronOre
            | ResourceType::CopperOre
            | ResourceType::RareMetals
            | ResourceType::Water
            | ResourceType::Timber
            | ResourceType::Food
            | ResourceType::Nutrients => "Raw Materials - Basic",
            ResourceType::Nickel
            | ResourceType::Cobalt
            | ResourceType::Zinc
            | ResourceType::Silver
            | ResourceType::Gold
            | ResourceType::Lithium
            | ResourceType::Graphite => "Raw Materials - Minerals",
            ResourceType::Coal
            | ResourceType::Coke
            | ResourceType::Limestone
            | ResourceType::Uranium
            | ResourceType::Silicon
            | ResourceType::Titanium
            | ResourceType::Platinum
            | ResourceType::Aluminum
            | ResourceType::CrystalOre
            | ResourceType::Gas
            | ResourceType::Oil
            | ResourceType::Oxygen => "Raw Materials - Advanced",
            ResourceType::Steel
            | ResourceType::Electronics
            | ResourceType::Machinery
            | ResourceType::CopperWire => "Processed Goods - Basic",
            ResourceType::Concrete
            | ResourceType::Plastics
            | ResourceType::Fuel
            | ResourceType::Chemicals
            | ResourceType::AdvancedComponents
            | ResourceType::Medicine => "Processed Goods - Advanced",
            ResourceType::Research | ResourceType::ConsumerGoods | ResourceType::Luxuries => {
                "Specialized"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Resources {
    resources: HashMap<ResourceType, i64>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn starting_resources() -> Self {
        let mut resources = Self::new();
        resources.set(ResourceType::Credits, 10000);
        resources.set(ResourceType::Energy, 100);
        resources.set(ResourceType::IronOre, 50);
        resources.set(ResourceType::Food, 200);
        resources.set(ResourceType::Water, 500);
        resources.set(ResourceType::Steel, 50);
        resources.set(ResourceType::Electronics, 20);
        resources
    }

    pub fn get(&self, resource_type: ResourceType) -> i64 {
        *self.resources.get(&resource_type).unwrap_or(&0)
    }

    pub fn set(&mut self, resource_type: ResourceType, quantity: i64) {
        self.resources.insert(resource_type, quantity);
    }

    pub fn add_resource(&mut self, resource_type: ResourceType, quantity: i64) {
        let current = self.get(resource_type);
        self.set(resource_type, current + quantity);
    }

    pub fn subtract(&mut self, resource_type: ResourceType, quantity: i64) -> bool {
        let current = self.get(resource_type);
        if current >= quantity {
            self.set(resource_type, current - quantity);
            true
        } else {
            false
        }
    }

    pub fn has_enough(&self, resource_type: ResourceType, quantity: i64) -> bool {
        self.get(resource_type) >= quantity
    }

    pub fn can_afford(&self, costs: &Resources) -> bool {
        for (resource_type, quantity) in &costs.resources {
            if !self.has_enough(*resource_type, *quantity) {
                return false;
            }
        }
        true
    }

    pub fn consume(&mut self, costs: &Resources) -> bool {
        if !self.can_afford(costs) {
            return false;
        }

        for (resource_type, quantity) in &costs.resources {
            self.subtract(*resource_type, *quantity);
        }
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ResourceType, &i64)> {
        self.resources.iter()
    }
}

impl Add for Resources {
    type Output = Resources;

    fn add(self, other: Resources) -> Resources {
        let mut result = self.clone();
        for (resource_type, quantity) in other.resources {
            let current = result.get(resource_type);
            result.set(resource_type, current + quantity);
        }
        result
    }
}

impl AddAssign for Resources {
    fn add_assign(&mut self, other: Resources) {
        for (resource_type, quantity) in other.resources {
            let current = self.get(resource_type);
            self.set(resource_type, current + quantity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_operations() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 100);

        assert_eq!(resources.get(ResourceType::Credits), 100);

        resources.add_resource(ResourceType::Credits, 50);
        assert_eq!(resources.get(ResourceType::Credits), 150);

        assert!(resources.subtract(ResourceType::Credits, 100));
        assert_eq!(resources.get(ResourceType::Credits), 50);

        assert!(!resources.subtract(ResourceType::Credits, 100));
        assert_eq!(resources.get(ResourceType::Credits), 50);
    }

    #[test]
    fn test_can_afford() {
        let mut resources = Resources::new();
        resources.set(ResourceType::Credits, 100);
        resources.set(ResourceType::IronOre, 50);

        let mut costs = Resources::new();
        costs.set(ResourceType::Credits, 50);
        costs.set(ResourceType::IronOre, 25);

        assert!(resources.can_afford(&costs));

        costs.set(ResourceType::Credits, 150);
        assert!(!resources.can_afford(&costs));
    }

    #[test]
    fn test_mineral_resource_types() {
        // Test that all 7 new mineral types have correct names
        assert_eq!(ResourceType::Nickel.name(), "Nickel");
        assert_eq!(ResourceType::Cobalt.name(), "Cobalt");
        assert_eq!(ResourceType::Zinc.name(), "Zinc");
        assert_eq!(ResourceType::Silver.name(), "Silver");
        assert_eq!(ResourceType::Gold.name(), "Gold");
        assert_eq!(ResourceType::Lithium.name(), "Lithium");
        assert_eq!(ResourceType::Graphite.name(), "Graphite");
    }

    #[test]
    fn test_mineral_categories() {
        // Test that all minerals are categorized correctly as "Raw Materials - Minerals"
        assert_eq!(ResourceType::Nickel.category(), "Raw Materials - Minerals");
        assert_eq!(ResourceType::Cobalt.category(), "Raw Materials - Minerals");
        assert_eq!(ResourceType::Zinc.category(), "Raw Materials - Minerals");
        assert_eq!(ResourceType::Silver.category(), "Raw Materials - Minerals");
        assert_eq!(ResourceType::Gold.category(), "Raw Materials - Minerals");
        assert_eq!(ResourceType::Lithium.category(), "Raw Materials - Minerals");
        assert_eq!(
            ResourceType::Graphite.category(),
            "Raw Materials - Minerals"
        );
    }

    #[test]
    fn test_mineral_operations() {
        // Test that minerals work with resource operations like other resources
        let mut resources = Resources::new();
        resources.set(ResourceType::Nickel, 100);
        resources.set(ResourceType::Gold, 50);

        assert_eq!(resources.get(ResourceType::Nickel), 100);
        assert_eq!(resources.get(ResourceType::Gold), 50);

        // Test add_resource
        resources.add_resource(ResourceType::Nickel, 25);
        assert_eq!(resources.get(ResourceType::Nickel), 125);

        // Test subtract
        assert!(resources.subtract(ResourceType::Gold, 30));
        assert_eq!(resources.get(ResourceType::Gold), 20);

        // Test insufficient resources
        assert!(!resources.subtract(ResourceType::Gold, 50));
        assert_eq!(resources.get(ResourceType::Gold), 20);
    }

    #[test]
    fn test_mineral_afford_and_consume() {
        // Test that minerals work with affordability and consumption checks
        let mut resources = Resources::new();
        resources.set(ResourceType::Lithium, 100);
        resources.set(ResourceType::Cobalt, 50);

        let mut costs = Resources::new();
        costs.set(ResourceType::Lithium, 60);
        costs.set(ResourceType::Cobalt, 30);

        // Check we can afford
        assert!(resources.can_afford(&costs));

        // Consume and verify amounts
        assert!(resources.consume(&costs));
        assert_eq!(resources.get(ResourceType::Lithium), 40);
        assert_eq!(resources.get(ResourceType::Cobalt), 20);

        // Try to consume more than we have
        let mut expensive_costs = Resources::new();
        expensive_costs.set(ResourceType::Lithium, 50);
        expensive_costs.set(ResourceType::Cobalt, 30);

        assert!(!resources.can_afford(&expensive_costs));
        assert!(!resources.consume(&expensive_costs));
    }

    #[test]
    fn test_energy_resource_types() {
        // Test that all 4 new energy types have correct names
        assert_eq!(ResourceType::SolarPower.name(), "Solar Power");
        assert_eq!(ResourceType::NuclearFuel.name(), "Nuclear Fuel");
        assert_eq!(ResourceType::BatteryPack.name(), "Battery Pack");
        assert_eq!(ResourceType::HydrogenCell.name(), "Hydrogen Cell");
    }

    #[test]
    fn test_energy_categories() {
        // Test that all energy types are categorized as "Energy"
        assert_eq!(ResourceType::Energy.category(), "Energy");
        assert_eq!(ResourceType::SolarPower.category(), "Energy");
        assert_eq!(ResourceType::NuclearFuel.category(), "Energy");
        assert_eq!(ResourceType::BatteryPack.category(), "Energy");
        assert_eq!(ResourceType::HydrogenCell.category(), "Energy");
    }

    #[test]
    fn test_energy_distinct_from_base_energy() {
        // Test that energy types are distinct from base Energy
        assert_ne!(ResourceType::SolarPower, ResourceType::Energy);
        assert_ne!(ResourceType::NuclearFuel, ResourceType::Energy);
        assert_ne!(ResourceType::BatteryPack, ResourceType::Energy);
        assert_ne!(ResourceType::HydrogenCell, ResourceType::Energy);

        // But all are in the same category
        assert_eq!(ResourceType::Energy.category(), "Energy");
        assert_eq!(ResourceType::SolarPower.category(), "Energy");
        assert_eq!(ResourceType::NuclearFuel.category(), "Energy");
        assert_eq!(ResourceType::BatteryPack.category(), "Energy");
        assert_eq!(ResourceType::HydrogenCell.category(), "Energy");
    }

    #[test]
    fn test_energy_operations() {
        // Test that energy types work with resource operations
        let mut resources = Resources::new();
        resources.set(ResourceType::SolarPower, 500);
        resources.set(ResourceType::BatteryPack, 100);
        resources.set(ResourceType::NuclearFuel, 50);
        resources.set(ResourceType::HydrogenCell, 25);

        // Verify all energy types can be tracked
        assert_eq!(resources.get(ResourceType::SolarPower), 500);
        assert_eq!(resources.get(ResourceType::BatteryPack), 100);
        assert_eq!(resources.get(ResourceType::NuclearFuel), 50);
        assert_eq!(resources.get(ResourceType::HydrogenCell), 25);

        // Test operations
        resources.add_resource(ResourceType::SolarPower, 100);
        assert_eq!(resources.get(ResourceType::SolarPower), 600);

        assert!(resources.subtract(ResourceType::BatteryPack, 40));
        assert_eq!(resources.get(ResourceType::BatteryPack), 60);

        assert!(resources.has_enough(ResourceType::NuclearFuel, 40));
        assert!(!resources.has_enough(ResourceType::NuclearFuel, 100));
    }

    #[test]
    fn test_energy_mixed_afford() {
        // Test that energy types can be mixed with other resources in affordability checks
        let mut resources = Resources::new();
        resources.set(ResourceType::Energy, 200);
        resources.set(ResourceType::SolarPower, 100);
        resources.set(ResourceType::BatteryPack, 50);
        resources.set(ResourceType::Credits, 5000);

        let mut costs = Resources::new();
        costs.set(ResourceType::Energy, 150);
        costs.set(ResourceType::SolarPower, 30);
        costs.set(ResourceType::BatteryPack, 20);
        costs.set(ResourceType::Credits, 1000);

        // Should be able to afford the mixed cost
        assert!(resources.can_afford(&costs));

        // Consume and verify
        assert!(resources.consume(&costs));
        assert_eq!(resources.get(ResourceType::Energy), 50);
        assert_eq!(resources.get(ResourceType::SolarPower), 70);
        assert_eq!(resources.get(ResourceType::BatteryPack), 30);
        assert_eq!(resources.get(ResourceType::Credits), 4000);
    }
}
