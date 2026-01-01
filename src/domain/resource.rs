use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Add, AddAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    // Currency
    Credits,

    // Energy
    Energy,

    // Raw materials - Basic
    IronOre,
    CopperOre,
    RareMetals,
    Water,
    Timber,
    Food,

    // Raw materials - Advanced
    Coal,
    Uranium,
    Silicon,
    Titanium,
    Platinum,
    Aluminum,
    CrystalOre,
    Gas,
    Oil,

    // Processed goods - Basic
    Steel,
    Electronics,
    Machinery,

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
            ResourceType::IronOre => "Iron Ore",
            ResourceType::CopperOre => "Copper Ore",
            ResourceType::RareMetals => "Rare Metals",
            ResourceType::Water => "Water",
            ResourceType::Timber => "Timber",
            ResourceType::Food => "Food",
            ResourceType::Coal => "Coal",
            ResourceType::Uranium => "Uranium",
            ResourceType::Silicon => "Silicon",
            ResourceType::Titanium => "Titanium",
            ResourceType::Platinum => "Platinum",
            ResourceType::Aluminum => "Aluminum",
            ResourceType::CrystalOre => "Crystal Ore",
            ResourceType::Gas => "Gas",
            ResourceType::Oil => "Oil",
            ResourceType::Steel => "Steel",
            ResourceType::Electronics => "Electronics",
            ResourceType::Machinery => "Machinery",
            ResourceType::Concrete => "Concrete",
            ResourceType::Plastics => "Plastics",
            ResourceType::Fuel => "Fuel",
            ResourceType::Chemicals => "Chemicals",
            ResourceType::AdvancedComponents => "Advanced Components",
            ResourceType::Medicine => "Medicine",
            ResourceType::Research => "Research",
            ResourceType::ConsumerGoods => "Consumer Goods",
            ResourceType::Luxuries => "Luxuries",
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            ResourceType::Credits => "Currency",
            ResourceType::Energy => "Energy",
            ResourceType::IronOre | ResourceType::CopperOre | ResourceType::RareMetals |
            ResourceType::Water | ResourceType::Timber | ResourceType::Food => "Raw Materials - Basic",
            ResourceType::Coal | ResourceType::Uranium | ResourceType::Silicon |
            ResourceType::Titanium | ResourceType::Platinum | ResourceType::Aluminum |
            ResourceType::CrystalOre | ResourceType::Gas | ResourceType::Oil => "Raw Materials - Advanced",
            ResourceType::Steel | ResourceType::Electronics | ResourceType::Machinery => "Processed Goods - Basic",
            ResourceType::Concrete | ResourceType::Plastics | ResourceType::Fuel |
            ResourceType::Chemicals | ResourceType::AdvancedComponents | ResourceType::Medicine => "Processed Goods - Advanced",
            ResourceType::Research | ResourceType::ConsumerGoods | ResourceType::Luxuries => "Specialized",
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
}
