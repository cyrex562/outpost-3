use serde::{Deserialize, Serialize};
use super::{ColonyId, ResourceType, Resources, ProductionChains};

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
    Refinery {
        input_type: ResourceType,
        output_type: ResourceType,
        conversion_rate: f32,
    },
    CommercialZone {
        size: u32,
        employment_capacity: u32,
    },
    MedicalFacility {
        treatment_capacity: u32,
        morale_bonus: f32,
    },
    SolarPowerPlant {
        output_mw: u32,
    },
    NuclearPowerPlant {
        output_mw: u32,
        waste_rate: u32,
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
    pub level: u8, // Upgrade level (1-5)
}

impl Building {
    pub fn new(id: BuildingId, colony_id: ColonyId, building_type: BuildingType) -> Self {
        Self {
            id,
            colony_id,
            building_type,
            state: BuildingState::UnderConstruction { progress: 0 },
            workers_assigned: 0,
            level: 1,
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
            BuildingType::Refinery { .. } => 12,
            BuildingType::CommercialZone { .. } => 4,
            BuildingType::MedicalFacility { .. } => 6,
            BuildingType::SolarPowerPlant { .. } => 0,
            BuildingType::NuclearPowerPlant { .. } => 0,
        }
    }

    pub fn power_generation(&self) -> u32 {
        if !self.is_operational() {
            return 0;
        }
        match &self.building_type {
            BuildingType::PowerPlant { output_mw, .. } => *output_mw,
            BuildingType::SolarPowerPlant { output_mw } => *output_mw,
            BuildingType::NuclearPowerPlant { output_mw, .. } => *output_mw,
            _ => 0,
        }
    }

    pub fn construction_time(&self) -> u8 {
        match &self.building_type {
            BuildingType::Mine { .. } => 3,
            BuildingType::PowerPlant { .. } => 5,
            BuildingType::Housing { .. } => 2,
            BuildingType::Factory { .. } => 6,
            BuildingType::Farm { .. } => 2,
            BuildingType::TrainStation { .. } => 8,
            BuildingType::Warehouse { .. } => 3,
            BuildingType::ResearchFacility { .. } => 7,
            BuildingType::Refinery { .. } => 6,
            BuildingType::CommercialZone { .. } => 4,
            BuildingType::MedicalFacility { .. } => 5,
            BuildingType::SolarPowerPlant { .. } => 4,
            BuildingType::NuclearPowerPlant { .. } => 10,
        }
    }

    pub fn construction_cost(&self) -> Resources {
        let mut cost = Resources::new();
        match &self.building_type {
            BuildingType::Mine { .. } => {
                cost.set(ResourceType::Credits, 500);
                cost.set(ResourceType::Steel, 20);
                cost.set(ResourceType::Machinery, 5);
            }
            BuildingType::PowerPlant { .. } => {
                cost.set(ResourceType::Credits, 1000);
                cost.set(ResourceType::Steel, 30);
                cost.set(ResourceType::Machinery, 10);
            }
            BuildingType::Housing { .. } => {
                cost.set(ResourceType::Credits, 300);
                cost.set(ResourceType::Steel, 10);
                cost.set(ResourceType::Concrete, 15);
            }
            BuildingType::Factory { .. } => {
                cost.set(ResourceType::Credits, 1500);
                cost.set(ResourceType::Steel, 40);
                cost.set(ResourceType::Machinery, 15);
                cost.set(ResourceType::Electronics, 10);
            }
            BuildingType::Farm { .. } => {
                cost.set(ResourceType::Credits, 200);
                cost.set(ResourceType::Steel, 5);
            }
            BuildingType::TrainStation { .. } => {
                cost.set(ResourceType::Credits, 2000);
                cost.set(ResourceType::Steel, 50);
                cost.set(ResourceType::Concrete, 30);
                cost.set(ResourceType::Electronics, 15);
            }
            BuildingType::Warehouse { .. } => {
                cost.set(ResourceType::Credits, 400);
                cost.set(ResourceType::Steel, 15);
                cost.set(ResourceType::Concrete, 20);
            }
            BuildingType::ResearchFacility { .. } => {
                cost.set(ResourceType::Credits, 1800);
                cost.set(ResourceType::Steel, 25);
                cost.set(ResourceType::Electronics, 20);
                cost.set(ResourceType::AdvancedComponents, 10);
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
        }
        cost
    }

    pub fn advance_construction(&mut self) -> bool {
        if let BuildingState::UnderConstruction { progress } = self.state {
            let construction_time = self.construction_time();
            let new_progress = progress + 1;

            if new_progress >= construction_time {
                self.state = BuildingState::Operational;
                true
            } else {
                self.state = BuildingState::UnderConstruction { progress: new_progress };
                false
            }
        } else {
            false
        }
    }

    pub fn construction_progress_percentage(&self) -> f32 {
        if let BuildingState::UnderConstruction { progress } = self.state {
            (progress as f32 / self.construction_time() as f32) * 100.0
        } else {
            100.0
        }
    }

    pub fn worker_capacity(&self) -> u32 {
        match &self.building_type {
            BuildingType::Mine { .. } => 10,
            BuildingType::Factory { .. } => 20,
            BuildingType::Farm { .. } => 15,
            BuildingType::TrainStation { .. } => 5,
            BuildingType::Warehouse { .. } => 5,
            BuildingType::ResearchFacility { .. } => 25,
            BuildingType::Refinery { .. } => 15,
            BuildingType::CommercialZone { employment_capacity, .. } => *employment_capacity,
            BuildingType::MedicalFacility { .. } => 12,
            _ => 0,
        }
    }

    /// Get the resource inputs required for production (per turn)
    pub fn production_inputs(&self) -> Resources {
        let mut inputs = Resources::new();

        match &self.building_type {
            BuildingType::Factory { consumes, .. } => {
                for resource in consumes {
                    inputs.set(*resource, 1);
                }
            }
            BuildingType::Refinery { input_type, .. } => {
                inputs.set(*input_type, 2);
            }
            BuildingType::PowerPlant { fuel_type: Some(fuel), .. } => {
                inputs.set(*fuel, 1);
            }
            BuildingType::NuclearPowerPlant { .. } => {
                inputs.set(ResourceType::Uranium, 1);
            }
            _ => {}
        }

        inputs
    }

    /// Get the resource outputs produced (per turn) when operational
    pub fn production_outputs(&self) -> Resources {
        let mut outputs = Resources::new();

        if !self.is_operational() {
            return outputs;
        }

        // Calculate efficiency based on worker allocation
        let efficiency = if self.worker_capacity() > 0 {
            (self.workers_assigned as f32 / self.worker_capacity() as f32).min(1.0)
        } else {
            1.0
        };

        // Apply level multiplier (20% bonus per level above 1)
        let level_bonus = self.level_multiplier();
        let total_multiplier = efficiency * level_bonus;

        match &self.building_type {
            BuildingType::Mine { resource_type, output_rate } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(*resource_type, amount);
            }
            BuildingType::Farm { output_rate } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(ResourceType::Food, amount);
            }
            BuildingType::Factory { produces, output_rate, .. } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(*produces, amount);
            }
            BuildingType::Refinery { output_type, conversion_rate, .. } => {
                let amount = (2.0 * conversion_rate * total_multiplier) as i64;
                outputs.set(*output_type, amount);
            }
            BuildingType::ResearchFacility { research_rate } => {
                let amount = (*research_rate as f32 * total_multiplier) as i64;
                outputs.set(ResourceType::Research, amount);
            }
            BuildingType::MedicalFacility { .. } => {
                let amount = (2.0 * total_multiplier) as i64;
                outputs.set(ResourceType::Medicine, amount);
            }
            BuildingType::CommercialZone { .. } => {
                let amount = (5.0 * total_multiplier) as i64;
                outputs.set(ResourceType::ConsumerGoods, amount);
                outputs.set(ResourceType::Credits, (10.0 * total_multiplier) as i64);
            }
            _ => {}
        }

        outputs
    }

    /// Check if the building can produce with available resources
    pub fn can_produce(&self, available_resources: &Resources) -> bool {
        if !self.is_operational() {
            return false;
        }

        let inputs = self.production_inputs();
        available_resources.can_afford(&inputs)
    }

    /// Get production efficiency based on workers (0.0 to 1.0)
    pub fn production_efficiency(&self) -> f32 {
        if self.worker_capacity() == 0 {
            1.0
        } else {
            (self.workers_assigned as f32 / self.worker_capacity() as f32).min(1.0)
        }
    }

    /// Get the maximum upgrade level
    pub fn max_level(&self) -> u8 {
        5
    }

    /// Check if building can be upgraded
    pub fn can_upgrade(&self) -> bool {
        self.level < self.max_level() && self.is_operational()
    }

    /// Get the cost to upgrade to the next level
    pub fn upgrade_cost(&self) -> Resources {
        let mut cost = Resources::new();

        if !self.can_upgrade() {
            return cost;
        }

        // Base costs that scale with level
        let base_credits = 1000 * (self.level as i64 + 1);
        let base_steel = 10 * (self.level as i64 + 1);

        cost.set(ResourceType::Credits, base_credits);
        cost.set(ResourceType::Steel, base_steel);

        // Building-specific upgrade costs
        match &self.building_type {
            BuildingType::PowerPlant { .. } |
            BuildingType::SolarPowerPlant { .. } |
            BuildingType::NuclearPowerPlant { .. } => {
                cost.set(ResourceType::Electronics, 5 * (self.level as i64 + 1));
            }
            BuildingType::Factory { .. } |
            BuildingType::Refinery { .. } => {
                cost.set(ResourceType::Machinery, 8 * (self.level as i64 + 1));
            }
            BuildingType::ResearchFacility { .. } => {
                cost.set(ResourceType::Electronics, 10 * (self.level as i64 + 1));
                cost.set(ResourceType::Machinery, 5 * (self.level as i64 + 1));
            }
            _ => {}
        }

        cost
    }

    /// Get the production multiplier based on level (1.0 at level 1, +20% per level)
    pub fn level_multiplier(&self) -> f32 {
        1.0 + (0.2 * (self.level as f32 - 1.0))
    }

    /// Upgrade the building to the next level
    pub fn upgrade(&mut self) -> bool {
        if self.can_upgrade() {
            self.level += 1;
            true
        } else {
            false
        }
    }
}
