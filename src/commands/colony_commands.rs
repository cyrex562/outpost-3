use thiserror::Error;
use tracing::info;

use super::Command;
use crate::domain::*;
use crate::events::EventType;
use crate::utils::logging::OperationContext;

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
            return Err(ColonyError::InvalidCommand(
                "Colony name cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        info!(
            colony_id = ?self.colony_id,
            planet_id = ?self.planet_id,
            name = %self.name,
            "Colony founded"
        );

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
            BuildingType::Refinery { .. } => {
                cost.set(ResourceType::Credits, 1200);
                cost.set(ResourceType::Steel, 35);
                cost.set(ResourceType::Machinery, 12);
            }
            BuildingType::CommercialZone { .. } => {
                cost.set(ResourceType::Credits, 800);
                cost.set(ResourceType::Steel, 20);
                cost.set(ResourceType::Concrete, 25);
            }
            BuildingType::MedicalFacility { .. } => {
                cost.set(ResourceType::Credits, 1000);
                cost.set(ResourceType::Steel, 15);
                cost.set(ResourceType::Electronics, 10);
                cost.set(ResourceType::Medicine, 5);
            }
            BuildingType::SolarPowerPlant { .. } => {
                cost.set(ResourceType::Credits, 800);
                cost.set(ResourceType::Steel, 20);
                cost.set(ResourceType::Electronics, 15);
            }
            BuildingType::NuclearPowerPlant { .. } => {
                cost.set(ResourceType::Credits, 5000);
                cost.set(ResourceType::Steel, 60);
                cost.set(ResourceType::Machinery, 20);
                cost.set(ResourceType::AdvancedComponents, 25);
                cost.set(ResourceType::Uranium, 10);
            }
            BuildingType::SolarArray { .. } => {
                cost.set(ResourceType::Credits, 1200);
                cost.set(ResourceType::Steel, 25);
                cost.set(ResourceType::Electronics, 20);
                cost.set(ResourceType::SolarPower, 5);
            }
            BuildingType::BatteryStorage { .. } => {
                cost.set(ResourceType::Credits, 800);
                cost.set(ResourceType::Steel, 30);
                cost.set(ResourceType::Lithium, 15);
                cost.set(ResourceType::Cobalt, 10);
            }
            BuildingType::HydrogenPlant { .. } => {
                cost.set(ResourceType::Credits, 1500);
                cost.set(ResourceType::Steel, 40);
                cost.set(ResourceType::Machinery, 15);
                cost.set(ResourceType::HydrogenCell, 8);
            }
            BuildingType::ArcFurnace { .. } => {
                cost.set(ResourceType::Credits, 1200);
                cost.set(ResourceType::Steel, 35);
                cost.set(ResourceType::Machinery, 12);
                cost.set(ResourceType::Oxygen, 5);
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

        // Task 7.6.5: Multi-step operation with operation_id correlation
        let op_ctx = OperationContext::new("construct_building");

        info!(
            operation_id = %op_ctx.operation_id(),
            building_id = ?self.building_id,
            colony_id = ?self.colony_id,
            building_type = ?self.building_type,
            cost = ?cost,
            "Building construction started"
        );

        let mut events = vec![];

        // Building construction started
        events.push(EventType::BuildingConstructionStarted {
            building_id: self.building_id,
            colony_id: self.colony_id,
            building_type: self.building_type.clone(),
        });

        // Consume resources
        for (resource_type, amount) in cost.iter() {
            info!(
                operation_id = %op_ctx.operation_id(),
                resource_type = ?resource_type,
                amount = amount,
                "Consuming resource for construction"
            );
            events.push(EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: *resource_type,
                amount: *amount,
            });
        }

        op_ctx.log_complete(&format!("{} events generated", events.len()));

        Ok(events)
    }
}

pub struct AdvanceTurn {
    pub current_turn: u64,
    pub buildings: Vec<(ColonyId, BuildingType)>,
}

impl Command for AdvanceTurn {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        // Task 7.6.5: Multi-step operation with operation_id correlation
        let op_ctx = OperationContext::new("advance_turn");

        info!(
            operation_id = %op_ctx.operation_id(),
            turn_number = self.current_turn,
            building_count = self.buildings.len(),
            "Advancing turn"
        );

        let mut events = vec![];

        // Process each building's production
        for (colony_id, building_type) in &self.buildings {
            match building_type {
                BuildingType::Mine {
                    resource_type,
                    output_rate,
                } => {
                    info!(
                        operation_id = %op_ctx.operation_id(),
                        colony_id = ?colony_id,
                        resource_type = ?resource_type,
                        amount = output_rate,
                        "Mine producing resources"
                    );
                    events.push(EventType::ResourcesProduced {
                        colony_id: *colony_id,
                        resource_type: *resource_type,
                        amount: *output_rate as i64,
                    });
                }
                BuildingType::Farm { output_rate } => {
                    info!(
                        operation_id = %op_ctx.operation_id(),
                        colony_id = ?colony_id,
                        resource_type = ?ResourceType::Food,
                        amount = output_rate,
                        "Farm producing food"
                    );
                    events.push(EventType::ResourcesProduced {
                        colony_id: *colony_id,
                        resource_type: ResourceType::Food,
                        amount: *output_rate as i64,
                    });
                }
                BuildingType::PowerPlant { output_mw, .. } => {
                    events.push(EventType::ResourcesProduced {
                        colony_id: *colony_id,
                        resource_type: ResourceType::Energy,
                        amount: *output_mw as i64,
                    });
                }
                BuildingType::SolarArray { output_mw } => {
                    events.push(EventType::ResourcesProduced {
                        colony_id: *colony_id,
                        resource_type: ResourceType::Energy,
                        amount: *output_mw as i64,
                    });
                }
                BuildingType::BatteryStorage { charge_rate, .. } => {
                    events.push(EventType::ResourcesProduced {
                        colony_id: *colony_id,
                        resource_type: ResourceType::BatteryPack,
                        amount: *charge_rate as i64,
                    });
                }
                BuildingType::HydrogenPlant { output_rate } => {
                    events.push(EventType::ResourcesProduced {
                        colony_id: *colony_id,
                        resource_type: ResourceType::HydrogenCell,
                        amount: *output_rate as i64,
                    });
                }
                BuildingType::ArcFurnace { output_rate } => {
                    events.push(EventType::ResourcesProduced {
                        colony_id: *colony_id,
                        resource_type: ResourceType::Steel,
                        amount: *output_rate as i64,
                    });
                }
                _ => {}
            }
        }

        // Advance turn
        events.push(EventType::TurnAdvanced {
            turn_number: self.current_turn + 1,
        });

        op_ctx.log_complete(&format!(
            "Turn advanced to {}, {} events generated",
            self.current_turn + 1,
            events.len()
        ));

        Ok(events)
    }
}

pub struct AllocateLabor {
    pub colony_id: ColonyId,
    pub building_id: BuildingId,
    pub workers_to_allocate: u32,
    pub available_unemployed: u32,
    pub building_capacity: u32,
    pub current_workers: u32,
}

impl Command for AllocateLabor {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.workers_to_allocate > self.available_unemployed {
            return Err(ColonyError::InvalidCommand(
                "Not enough unemployed workers available".to_string(),
            ));
        }

        if self.current_workers + self.workers_to_allocate > self.building_capacity {
            return Err(ColonyError::InvalidCommand(
                "Building worker capacity exceeded".to_string(),
            ));
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        info!(
            colony_id = ?self.colony_id,
            building_id = ?self.building_id,
            workers_allocated = self.workers_to_allocate,
            "Labor allocated to building"
        );

        Ok(vec![EventType::LaborAllocated {
            colony_id: self.colony_id,
            building_id: self.building_id,
            workers_allocated: self.workers_to_allocate,
        }])
    }
}

pub struct DeallocateLabor {
    pub colony_id: ColonyId,
    pub building_id: BuildingId,
    pub workers_to_deallocate: u32,
    pub current_workers: u32,
}

impl Command for DeallocateLabor {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        if self.workers_to_deallocate > self.current_workers {
            return Err(ColonyError::InvalidCommand(
                "Cannot deallocate more workers than currently assigned".to_string(),
            ));
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        info!(
            colony_id = ?self.colony_id,
            building_id = ?self.building_id,
            workers_deallocated = self.workers_to_deallocate,
            "Labor deallocated from building"
        );

        Ok(vec![EventType::LaborDeallocated {
            colony_id: self.colony_id,
            building_id: self.building_id,
            workers_deallocated: self.workers_to_deallocate,
        }])
    }
}

pub struct ChangeBuildingState {
    pub building_id: BuildingId,
    pub colony_id: ColonyId,
    pub new_state: BuildingState,
}

impl Command for ChangeBuildingState {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        info!(
            building_id = ?self.building_id,
            colony_id = ?self.colony_id,
            new_state = ?self.new_state,
            "Building state changed"
        );

        Ok(vec![EventType::BuildingStateChanged {
            building_id: self.building_id,
            colony_id: self.colony_id,
            new_state: self.new_state,
        }])
    }
}

pub struct UpgradeBuilding {
    pub building_id: BuildingId,
    pub colony_id: ColonyId,
    pub current_level: u8,
    pub available_resources: Resources,
    pub upgrade_cost: Resources,
}

impl Command for UpgradeBuilding {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Check if already at max level
        if self.current_level >= 5 {
            return Err(ColonyError::InvalidCommand(format!(
                "Building is already at max level ({})",
                self.current_level
            )));
        }

        // Check if enough resources
        if !self.available_resources.can_afford(&self.upgrade_cost) {
            return Err(ColonyError::InvalidCommand(format!(
                "Cannot afford upgrade cost"
            )));
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        // Task 7.6.5: Multi-step operation with operation_id correlation
        let op_ctx = OperationContext::new("upgrade_building");

        info!(
            operation_id = %op_ctx.operation_id(),
            building_id = ?self.building_id,
            colony_id = ?self.colony_id,
            current_level = self.current_level,
            new_level = self.current_level + 1,
            upgrade_cost = ?self.upgrade_cost,
            "Building upgraded"
        );

        let mut events = vec![];

        // Consume resources for upgrade
        for (resource_type, amount) in self.upgrade_cost.iter() {
            info!(
                operation_id = %op_ctx.operation_id(),
                resource_type = ?resource_type,
                amount = amount,
                "Consuming resource for upgrade"
            );
            events.push(EventType::ResourcesConsumed {
                colony_id: self.colony_id,
                resource_type: *resource_type,
                amount: *amount,
            });
        }

        // Create upgrade event
        events.push(EventType::BuildingUpgraded {
            building_id: self.building_id,
            colony_id: self.colony_id,
            new_level: self.current_level + 1,
        });

        op_ctx.log_complete(&format!(
            "Building upgraded to level {}",
            self.current_level + 1
        ));

        Ok(events)
    }
}

pub struct RepairBuilding {
    pub building_id: BuildingId,
    pub colony_id: ColonyId,
    pub current_state: BuildingState,
    pub available_resources: Resources,
}

impl RepairBuilding {
    fn repair_cost(damage_severity: u8) -> Resources {
        let mut cost = Resources::new();

        // Base repair cost scales with damage severity (1-100)
        let base_cost = (damage_severity as i64) * 10;
        cost.set(ResourceType::Credits, base_cost);
        cost.set(ResourceType::Steel, damage_severity as i64 / 10 + 1);

        // More severe damage requires electronics for diagnostic equipment
        if damage_severity > 50 {
            cost.set(ResourceType::Electronics, damage_severity as i64 / 25);
        }

        cost
    }
}

impl Command for RepairBuilding {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Check if building is damaged
        match self.current_state {
            BuildingState::Damaged { severity } => {
                if severity == 0 {
                    return Err(ColonyError::InvalidCommand(
                        "Building is not damaged".to_string(),
                    ));
                }

                // Check if enough resources for repair
                let repair_cost = Self::repair_cost(severity);
                if !self.available_resources.can_afford(&repair_cost) {
                    return Err(ColonyError::InvalidCommand(
                        "Cannot afford repair cost".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ColonyError::InvalidCommand(
                    "Building is not damaged".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        let mut events = vec![];

        if let BuildingState::Damaged { severity } = self.current_state {
            // Task 7.6.5: Multi-step operation with operation_id correlation
            let op_ctx = OperationContext::new("repair_building");
            let repair_cost = Self::repair_cost(severity);

            info!(
                operation_id = %op_ctx.operation_id(),
                building_id = ?self.building_id,
                colony_id = ?self.colony_id,
                damage_severity = severity,
                repair_cost = ?repair_cost,
                "Building repaired"
            );

            // Consume resources for repair
            for (resource_type, amount) in repair_cost.iter() {
                if *amount > 0 {
                    info!(
                        operation_id = %op_ctx.operation_id(),
                        resource_type = ?resource_type,
                        amount = amount,
                        "Consuming resource for repair"
                    );
                    events.push(EventType::ResourcesConsumed {
                        colony_id: self.colony_id,
                        resource_type: *resource_type,
                        amount: *amount,
                    });
                }
            }

            // Create repair event
            events.push(EventType::BuildingRepaired {
                building_id: self.building_id,
                colony_id: self.colony_id,
            });

            op_ctx.log_complete(&format!("Building repaired, severity was {}", severity));
        }

        Ok(events)
    }
}

pub struct SetRecipe {
    pub building_id: BuildingId,
    pub colony_id: ColonyId,
    pub recipe_index: u32,
    pub current_recipe_index: u32,
}

impl Command for SetRecipe {
    type Error = ColonyError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Validate that recipe_index is different from current
        if self.recipe_index == self.current_recipe_index {
            return Err(ColonyError::InvalidCommand(
                "New recipe must be different from current recipe".to_string(),
            ));
        }

        // Validate that recipe_index is reasonable (0-99, allowing for future expansion)
        if self.recipe_index > 99 {
            return Err(ColonyError::InvalidCommand(
                "Recipe index out of valid range".to_string(),
            ));
        }

        Ok(())
    }

    fn execute(&self) -> Result<Vec<EventType>, Self::Error> {
        self.validate()?;

        info!(
            building_id = ?self.building_id,
            colony_id = ?self.colony_id,
            old_recipe_index = self.current_recipe_index,
            new_recipe_index = self.recipe_index,
            "Building recipe changed"
        );

        Ok(vec![EventType::BuildingRecipeChanged {
            building_id: self.building_id,
            colony_id: self.colony_id,
            old_recipe_index: self.current_recipe_index,
            new_recipe_index: self.recipe_index,
        }])
    }
}
