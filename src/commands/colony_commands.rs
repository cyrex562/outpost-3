use thiserror::Error;

use crate::domain::*;
use crate::events::EventType;
use super::Command;

#[derive(Error, Debug)]
pub enum ColonyError {
    #[error("Insufficient resources: {resource_type:?}")]
    InsufficientResources { resource_type: ResourceType },

    #[error("Building {building_id:?} not found")]
    BuildingNotFound { building_id: BuildingId },

    #[error("Invalid command: {0}")]
    InvalidCommand(String),
}

pub struct FoundColony {
    pub colony_id: ColonyId,
    pub planet_id: PlanetId,
    pub name: String,
}

impl Command for FoundColony {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.name.is_empty() {
            return Err(ColonyError::InvalidCommand("Colony name cannot be empty".to_string()));
        }
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        Ok(vec![EventType::ColonyFounded {
            colony_id: self.colony_id,
            planet_id: self.planet_id,
            name: self.name.clone(),
            starting_resources: Resources::starting_resources(),
        }])
    }
}

pub struct ConstructBuilding {
    pub building_id: BuildingId,
    pub colony_id: ColonyId,
    pub building_type: BuildingType,
    pub available_resources: Resources,
}

impl ConstructBuilding {
    fn construction_cost(&self) -> Resources {
        let mut cost = Resources::new();

        match &self.building_type {
            BuildingType::Mine { .. } => {
                cost.set(ResourceType::Credits, 500);
                cost.set(ResourceType::Steel, 10);
            }
            BuildingType::PowerPlant { .. } => {
                cost.set(ResourceType::Credits, 1000);
                cost.set(ResourceType::Steel, 20);
                cost.set(ResourceType::Electronics, 5);
            }
            BuildingType::Housing { .. } => {
                cost.set(ResourceType::Credits, 300);
                cost.set(ResourceType::Steel, 5);
            }
            BuildingType::Farm { .. } => {
                cost.set(ResourceType::Credits, 400);
            }
            BuildingType::Factory { .. } => {
                cost.set(ResourceType::Credits, 800);
                cost.set(ResourceType::Steel, 15);
                cost.set(ResourceType::Electronics, 3);
            }
            BuildingType::TrainStation { .. } => {
                cost.set(ResourceType::Credits, 2000);
                cost.set(ResourceType::Steel, 30);
            }
            BuildingType::Warehouse { .. } => {
                cost.set(ResourceType::Credits, 600);
                cost.set(ResourceType::Steel, 10);
            }
            BuildingType::ResearchFacility { .. } => {
                cost.set(ResourceType::Credits, 1500);
                cost.set(ResourceType::Steel, 10);
                cost.set(ResourceType::Electronics, 10);
            }
        }

        cost
    }
}

impl Command for ConstructBuilding {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        let cost = self.construction_cost();

        if !self.available_resources.can_afford(&cost) {
            return Err(ColonyError::InsufficientResources {
                resource_type: ResourceType::Credits, // Simplified
            });
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        let cost = self.construction_cost();
        let mut events = vec![];

        // Building construction started
        events.push(EventType::BuildingConstructionStarted {
            building_id: self.building_id,
            colony_id: self.colony_id,
            building_type: self.building_type.clone(),
        });

        // Consume resources
        for (resource_type, amount) in cost.iter() {
            events.push(EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: *resource_type,
                amount: *amount,
            });
        }

        Ok(events)
    }
}

pub struct AdvanceTurn {
    pub current_turn: u64,
}

impl Command for AdvanceTurn {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        Ok(vec![EventType::TurnAdvanced {
            turn_number: self.current_turn + 1,
        }])
    }
}
