use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

// Import the types from the old codebase structure
// These will need to be defined as components as well
#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PersonType {
    Colonist,
    Engineer,
    Scientist,
    Administrator,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BuildingType {
    Habitation,
    PowerPlant,
    Factory,
    ResearchLab,
    Spaceport,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Energy,
    Materials,
    Food,
    Research,
    Population,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub id: Uuid,
    pub name: String,
    pub population: HashMap<PersonType, u32>,
    pub buildings: HashMap<BuildingType, u32>,
    pub resources: HashMap<ResourceType, u64>,
}

impl Settlement {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            population: HashMap::new(),
            buildings: HashMap::new(),
            resources: HashMap::new(),
        }
    }
}

// Component for tracking settlement resources
#[derive(Component)]
pub struct SettlementResources {
    pub resources: HashMap<ResourceType, u64>,
}

// Component for tracking settlement population
#[derive(Component)]
pub struct SettlementPopulation {
    pub population: HashMap<PersonType, u32>,
}

// Component for tracking settlement buildings
#[derive(Component)]
pub struct SettlementBuildings {
    pub buildings: HashMap<BuildingType, u32>,
}
