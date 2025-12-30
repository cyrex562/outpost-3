use serde::{Deserialize, Serialize};
use super::{ColonyId, ResourceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildingId(pub u64);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BuildingType {
    Mine {
        resource_type: ResourceType,
        output_rate: u32,
    },
    PowerPlant {
        output_mw: u32,
        fuel_type: Option<ResourceType>,
    },
    Housing {
        capacity: u32,
        comfort_level: u8,
    },
    Factory {
        produces: ResourceType,
        consumes: Vec<ResourceType>,
        output_rate: u32,
    },
    Farm {
        output_rate: u32,
    },
    TrainStation {
        platforms: u8,
        throughput: u32,
    },
    Warehouse {
        capacity: u32,
    },
    ResearchFacility {
        research_rate: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingState {
    UnderConstruction { progress: u8 },
    Operational,
    Damaged { severity: u8 },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: BuildingId,
    pub colony_id: ColonyId,
    pub building_type: BuildingType,
    pub state: BuildingState,
    pub workers_assigned: u32,
}

impl Building {
    pub fn new(id: BuildingId, colony_id: ColonyId, building_type: BuildingType) -> Self {
        Self {
            id,
            colony_id,
            building_type,
            state: BuildingState::UnderConstruction { progress: 0 },
            workers_assigned: 0,
        }
    }

    pub fn is_operational(&self) -> bool {
        matches!(self.state, BuildingState::Operational)
    }

    pub fn power_consumption(&self) -> u32 {
        match &self.building_type {
            BuildingType::Mine { .. } => 5,
            BuildingType::Factory { .. } => 10,
            BuildingType::Farm { .. } => 2,
            BuildingType::TrainStation { .. } => 15,
            BuildingType::Warehouse { .. } => 1,
            BuildingType::ResearchFacility { .. } => 8,
            BuildingType::Housing { .. } => 3,
            BuildingType::PowerPlant { .. } => 0,
        }
    }

    pub fn power_generation(&self) -> u32 {
        if let BuildingType::PowerPlant { output_mw, .. } = self.building_type {
            if self.is_operational() {
                return output_mw;
            }
        }
        0
    }
}
