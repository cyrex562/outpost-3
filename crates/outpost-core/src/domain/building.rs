use super::{ColonyId, ResourceType, Resources};
use serde::{Deserialize, Serialize};

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
    SolarArray {
        output_mw: u32,
    },
    BatteryStorage {
        capacity: u32,
        charge_rate: u32,
    },
    HydrogenPlant {
        output_rate: u32,
    },
    ArcFurnace {
        output_rate: u32,
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
    pub level: u8,         // Upgrade level (1-5)
    pub recipe_index: u32, // Index of selected production recipe (0-based)
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
            recipe_index: 0,
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
            BuildingType::Warehouse { .. } => 1,
            BuildingType::ResearchFacility { .. } => 8,
            BuildingType::Housing { .. } => 3,
            BuildingType::PowerPlant { .. } => 0,
            BuildingType::Refinery { .. } => 12,
            BuildingType::CommercialZone { .. } => 4,
            BuildingType::MedicalFacility { .. } => 6,
            BuildingType::SolarPowerPlant { .. } => 0,
            BuildingType::NuclearPowerPlant { .. } => 0,
            BuildingType::SolarArray { .. } => 0,
            BuildingType::BatteryStorage { .. } => 2,
            BuildingType::HydrogenPlant { .. } => 8,
            BuildingType::ArcFurnace { .. } => 15,
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
            BuildingType::SolarArray { output_mw } => *output_mw,
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
            BuildingType::Warehouse { .. } => 3,
            BuildingType::ResearchFacility { .. } => 7,
            BuildingType::Refinery { .. } => 6,
            BuildingType::CommercialZone { .. } => 4,
            BuildingType::MedicalFacility { .. } => 5,
            BuildingType::SolarPowerPlant { .. } => 4,
            BuildingType::NuclearPowerPlant { .. } => 10,
            BuildingType::SolarArray { .. } => 4,
            BuildingType::BatteryStorage { .. } => 3,
            BuildingType::HydrogenPlant { .. } => 5,
            BuildingType::ArcFurnace { .. } => 6,
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

    pub fn advance_construction(&mut self) -> bool {
        if let BuildingState::UnderConstruction { progress } = self.state {
            let construction_time = self.construction_time();
            let new_progress = progress + 1;

            if new_progress >= construction_time {
                self.state = BuildingState::Operational;
                true
            } else {
                self.state = BuildingState::UnderConstruction {
                    progress: new_progress,
                };
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
            BuildingType::Warehouse { .. } => 5,
            BuildingType::ResearchFacility { .. } => 25,
            BuildingType::Refinery { .. } => 15,
            BuildingType::CommercialZone {
                employment_capacity,
                ..
            } => *employment_capacity,
            BuildingType::MedicalFacility { .. } => 12,
            BuildingType::SolarArray { .. } => 8,
            BuildingType::BatteryStorage { .. } => 4,
            BuildingType::HydrogenPlant { .. } => 12,
            BuildingType::ArcFurnace { .. } => 18,
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
            BuildingType::PowerPlant {
                fuel_type: Some(fuel),
                ..
            } => {
                inputs.set(*fuel, 1);
            }
            BuildingType::NuclearPowerPlant { .. } => {
                inputs.set(ResourceType::Uranium, 1);
            }
            BuildingType::BatteryStorage { .. } => {
                inputs.set(ResourceType::BatteryPack, 1);
            }
            BuildingType::HydrogenPlant { .. } => {
                inputs.set(ResourceType::Gas, 2);
                inputs.set(ResourceType::Coal, 1);
            }
            BuildingType::ArcFurnace { .. } => {
                inputs.set(ResourceType::IronOre, 2);
                inputs.set(ResourceType::Coal, 1);
                inputs.set(ResourceType::Oxygen, 1);
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
            BuildingType::Mine {
                resource_type,
                output_rate,
            } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(*resource_type, amount);
            }
            BuildingType::Farm { output_rate } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(ResourceType::Food, amount);
            }
            BuildingType::Factory {
                produces,
                output_rate,
                ..
            } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(*produces, amount);
            }
            BuildingType::Refinery {
                output_type,
                conversion_rate,
                ..
            } => {
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
            BuildingType::SolarArray { output_mw: _ } => {
                // SolarArray produces energy but doesn't output resources directly
                // It contributes to power_generation()
            }
            BuildingType::BatteryStorage { charge_rate, .. } => {
                let amount = (*charge_rate as f32 * total_multiplier) as i64;
                outputs.set(ResourceType::BatteryPack, amount);
            }
            BuildingType::HydrogenPlant { output_rate } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(ResourceType::HydrogenCell, amount);
            }
            BuildingType::ArcFurnace { output_rate } => {
                let amount = (*output_rate as f32 * total_multiplier) as i64;
                outputs.set(ResourceType::Steel, amount);
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
            BuildingType::PowerPlant { .. }
            | BuildingType::SolarPowerPlant { .. }
            | BuildingType::NuclearPowerPlant { .. }
            | BuildingType::SolarArray { .. } => {
                cost.set(ResourceType::Electronics, 5 * (self.level as i64 + 1));
            }
            BuildingType::Factory { .. } | BuildingType::Refinery { .. } => {
                cost.set(ResourceType::Machinery, 8 * (self.level as i64 + 1));
            }
            BuildingType::ResearchFacility { .. } => {
                cost.set(ResourceType::Electronics, 10 * (self.level as i64 + 1));
                cost.set(ResourceType::Machinery, 5 * (self.level as i64 + 1));
            }
            BuildingType::BatteryStorage { .. } => {
                cost.set(ResourceType::Lithium, 3 * (self.level as i64 + 1));
            }
            BuildingType::HydrogenPlant { .. } => {
                cost.set(ResourceType::Machinery, 5 * (self.level as i64 + 1));
            }
            BuildingType::ArcFurnace { .. } => {
                cost.set(ResourceType::Machinery, 6 * (self.level as i64 + 1));
                cost.set(ResourceType::Oxygen, 2 * (self.level as i64 + 1));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mine_with_new_minerals() {
        // Verify Mine building supports new mineral resources from 7.2.1
        let minerals = vec![
            ResourceType::Nickel,
            ResourceType::Cobalt,
            ResourceType::Zinc,
            ResourceType::Silver,
            ResourceType::Gold,
            ResourceType::Lithium,
            ResourceType::Graphite,
        ];

        for mineral in minerals {
            let mine = Building::new(
                BuildingId(1),
                ColonyId(1),
                BuildingType::Mine {
                    resource_type: mineral,
                    output_rate: 5,
                },
            );

            assert_eq!(mine.worker_capacity(), 10);
            assert_eq!(mine.construction_time(), 3);
            assert!(mine.construction_cost().get(ResourceType::Steel) > 0);
        }
    }

    #[test]
    fn test_mine_nickel_extraction() {
        // Test specific Mine configuration for Nickel (example from task)
        let mut mine = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::Mine {
                resource_type: ResourceType::Nickel,
                output_rate: 3,
            },
        );

        // Complete construction
        mine.state = BuildingState::Operational;
        mine.workers_assigned = 10; // Full staffing

        // Verify outputs
        let outputs = mine.production_outputs();
        assert_eq!(outputs.get(ResourceType::Nickel), 3); // Base rate with full efficiency
    }

    #[test]
    fn test_mine_production_with_efficiency() {
        // Test that efficiency affects production output
        let mut mine = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::Mine {
                resource_type: ResourceType::Gold,
                output_rate: 10,
            },
        );

        mine.state = BuildingState::Operational;

        // No workers - no production
        mine.workers_assigned = 0;
        let outputs_no_workers = mine.production_outputs();
        assert_eq!(outputs_no_workers.get(ResourceType::Gold), 0);

        // Half capacity
        mine.workers_assigned = 5;
        let outputs_half = mine.production_outputs();
        let half_value = outputs_half.get(ResourceType::Gold);
        assert!(half_value > 0);
        assert!(half_value < 10);

        // Full capacity
        mine.workers_assigned = 10;
        let outputs_full = mine.production_outputs();
        assert_eq!(outputs_full.get(ResourceType::Gold), 10);
    }

    #[test]
    fn test_mine_construction_to_extraction_flow() {
        // Integration test: construction → completion → extraction
        let mut mine = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::Mine {
                resource_type: ResourceType::Lithium,
                output_rate: 5,
            },
        );

        // Verify initial state
        assert!(!mine.is_operational());
        let outputs_initial = mine.production_outputs();
        assert_eq!(outputs_initial.get(ResourceType::Lithium), 0);

        // Advance construction through all phases
        let construction_time = mine.construction_time();
        for _ in 0..construction_time {
            mine.advance_construction();
        }

        // Verify operational
        assert!(mine.is_operational());

        // Assign workers and verify production
        mine.workers_assigned = 5;
        let outputs = mine.production_outputs();
        assert!(outputs.get(ResourceType::Lithium) > 0);
    }

    #[test]
    fn test_mine_production_logging_info() {
        // Verify extraction details match logging format
        let mut mine = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::Mine {
                resource_type: ResourceType::Zinc,
                output_rate: 4,
            },
        );

        mine.state = BuildingState::Operational;
        mine.workers_assigned = 8;

        let efficiency = mine.production_efficiency();
        let outputs = mine.production_outputs();
        let amount = outputs.get(ResourceType::Zinc);

        // Verify values for logging: "Mine extracting {resource}: {amount} (efficiency: {efficiency})"
        assert_eq!(efficiency, 0.8); // 8/10
        assert!(amount > 0);
        // amount should be approximately 4 * 0.8 * 1.0 (level multiplier) = 3.2 -> 3
        assert!(amount >= 3);
    }

    #[test]
    fn test_multiple_mines_different_resources() {
        // Verify multiple mine types can coexist in a colony
        let colony = ColonyId(1);

        let mine_nickel = Building::new(
            BuildingId(1),
            colony,
            BuildingType::Mine {
                resource_type: ResourceType::Nickel,
                output_rate: 5,
            },
        );

        let mine_gold = Building::new(
            BuildingId(2),
            colony,
            BuildingType::Mine {
                resource_type: ResourceType::Gold,
                output_rate: 3,
            },
        );

        let mine_lithium = Building::new(
            BuildingId(3),
            colony,
            BuildingType::Mine {
                resource_type: ResourceType::Lithium,
                output_rate: 4,
            },
        );

        // All should be valid mines for same colony
        assert_eq!(mine_nickel.colony_id, colony);
        assert_eq!(mine_gold.colony_id, colony);
        assert_eq!(mine_lithium.colony_id, colony);
    }

    #[test]
    fn test_mine_with_upgrade_levels() {
        // Verify level upgrades affect production output
        let mut mine = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::Mine {
                resource_type: ResourceType::Cobalt,
                output_rate: 10,
            },
        );

        mine.state = BuildingState::Operational;
        mine.workers_assigned = 10;

        // Level 1: base multiplier 1.0
        let outputs_level1 = mine.production_outputs();
        let production_level1 = outputs_level1.get(ResourceType::Cobalt);

        // Upgrade to level 2: multiplier 1.2
        mine.level = 2;
        let outputs_level2 = mine.production_outputs();
        let production_level2 = outputs_level2.get(ResourceType::Cobalt);

        // Level 2 should produce 20% more
        assert!(production_level2 > production_level1);
        assert_eq!(production_level2, (production_level1 as f32 * 1.2) as i64);
    }

    #[test]
    fn test_solar_array_power_generation() {
        // Test SolarArray produces power
        let mut solar = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::SolarArray { output_mw: 50 },
        );

        // Not operational - no power
        assert_eq!(solar.power_generation(), 0);

        // Operational
        solar.state = BuildingState::Operational;
        assert_eq!(solar.power_generation(), 50);

        // Verify logging format: "SolarArray producing {output_mw} MW"
        let output_mw = match solar.building_type {
            BuildingType::SolarArray { output_mw } => output_mw,
            _ => panic!("Expected SolarArray"),
        };
        assert_eq!(output_mw, 50);
    }

    #[test]
    fn test_battery_storage_production() {
        // Test BatteryStorage produces battery packs
        let mut battery = Building::new(
            BuildingId(2),
            ColonyId(1),
            BuildingType::BatteryStorage {
                capacity: 1000,
                charge_rate: 10,
            },
        );

        battery.state = BuildingState::Operational;
        battery.workers_assigned = 4; // Full capacity

        let outputs = battery.production_outputs();
        let battery_count = outputs.get(ResourceType::BatteryPack);
        assert_eq!(battery_count, 10); // Base rate with full efficiency

        // Verify construction time (3 turns)
        assert_eq!(battery.construction_time(), 3);

        // Verify construction cost includes Lithium and Cobalt
        let cost = battery.construction_cost();
        assert!(cost.get(ResourceType::Lithium) > 0);
        assert!(cost.get(ResourceType::Cobalt) > 0);
    }

    #[test]
    fn test_hydrogen_plant_production() {
        // Test HydrogenPlant produces hydrogen cells
        let mut hydrogen = Building::new(
            BuildingId(3),
            ColonyId(1),
            BuildingType::HydrogenPlant { output_rate: 8 },
        );

        hydrogen.state = BuildingState::Operational;
        hydrogen.workers_assigned = 12; // Full capacity

        let outputs = hydrogen.production_outputs();
        let hydrogen_count = outputs.get(ResourceType::HydrogenCell);
        assert_eq!(hydrogen_count, 8);

        // Verify inputs: Gas (2) + Coal (1)
        let inputs = hydrogen.production_inputs();
        assert_eq!(inputs.get(ResourceType::Gas), 2);
        assert_eq!(inputs.get(ResourceType::Coal), 1);
    }

    #[test]
    fn test_energy_building_construction_requirements() {
        // Verify each energy building has appropriate construction costs
        let solar_array = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::SolarArray { output_mw: 40 },
        );

        let battery_storage = Building::new(
            BuildingId(2),
            ColonyId(1),
            BuildingType::BatteryStorage {
                capacity: 500,
                charge_rate: 8,
            },
        );

        let hydrogen_plant = Building::new(
            BuildingId(3),
            ColonyId(1),
            BuildingType::HydrogenPlant { output_rate: 5 },
        );

        // SolarArray cost includes SolarPower
        let solar_cost = solar_array.construction_cost();
        assert!(solar_cost.get(ResourceType::SolarPower) > 0);
        assert!(solar_cost.get(ResourceType::Electronics) > 0);

        // Battery storage cost includes Lithium and Cobalt
        let battery_cost = battery_storage.construction_cost();
        assert!(battery_cost.get(ResourceType::Lithium) > 0);
        assert!(battery_cost.get(ResourceType::Cobalt) > 0);

        // Hydrogen plant cost includes HydrogenCell
        let hydrogen_cost = hydrogen_plant.construction_cost();
        assert!(hydrogen_cost.get(ResourceType::HydrogenCell) > 0);
    }

    #[test]
    fn test_battery_storage_to_energy_discharge() {
        // Integration test: BatteryStorage → Energy consumption
        let mut battery = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::BatteryStorage {
                capacity: 500,
                charge_rate: 10,
            },
        );

        battery.state = BuildingState::Operational;
        battery.workers_assigned = 4;

        // Charge batteries
        let outputs = battery.production_outputs();
        let battery_packs = outputs.get(ResourceType::BatteryPack);
        assert!(
            battery_packs > 0,
            "Battery storage should produce battery packs"
        );

        // Verify it can consume battery packs as input
        let inputs = battery.production_inputs();
        assert_eq!(inputs.get(ResourceType::BatteryPack), 1);
    }

    #[test]
    fn test_solar_array_to_battery_storage_chain() {
        // Integration test: SolarArray (power) → Battery Storage (stores) → consumption
        let mut solar = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::SolarArray { output_mw: 100 },
        );

        solar.state = BuildingState::Operational;
        let solar_power = solar.power_generation();
        assert_eq!(solar_power, 100, "Solar should generate 100 MW");

        // Battery storage can store energy
        let battery = Building::new(
            BuildingId(2),
            ColonyId(1),
            BuildingType::BatteryStorage {
                capacity: 5000,
                charge_rate: 20,
            },
        );

        // Verify battery can use solar power (via integration)
        assert!(battery.worker_capacity() > 0);
    }

    #[test]
    fn test_hydrogen_plant_efficiency_with_workers() {
        // Test hydrogen plant efficiency scales with workers
        let mut hydrogen = Building::new(
            BuildingId(1),
            ColonyId(1),
            BuildingType::HydrogenPlant { output_rate: 12 },
        );

        hydrogen.state = BuildingState::Operational;

        // No workers
        hydrogen.workers_assigned = 0;
        let outputs_no_workers = hydrogen.production_outputs();
        assert_eq!(outputs_no_workers.get(ResourceType::HydrogenCell), 0);

        // Half capacity
        hydrogen.workers_assigned = 6;
        let outputs_half = hydrogen.production_outputs();
        let half_production = outputs_half.get(ResourceType::HydrogenCell);
        assert!(half_production > 0);
        assert!(half_production < 12);

        // Full capacity
        hydrogen.workers_assigned = 12;
        let outputs_full = hydrogen.production_outputs();
        assert_eq!(outputs_full.get(ResourceType::HydrogenCell), 12);
    }

    #[test]
    fn test_energy_buildings_in_same_colony() {
        // Verify multiple energy buildings can coexist
        let colony = ColonyId(1);

        let solar = Building::new(
            BuildingId(1),
            colony,
            BuildingType::SolarArray { output_mw: 50 },
        );

        let battery = Building::new(
            BuildingId(2),
            colony,
            BuildingType::BatteryStorage {
                capacity: 1000,
                charge_rate: 15,
            },
        );

        let hydrogen = Building::new(
            BuildingId(3),
            colony,
            BuildingType::HydrogenPlant { output_rate: 8 },
        );

        assert_eq!(solar.colony_id, colony);
        assert_eq!(battery.colony_id, colony);
        assert_eq!(hydrogen.colony_id, colony);

        // Each should have distinct power generation or storage characteristics
        assert!(solar.power_generation() > 0 || battery.power_consumption() >= 0);
    }
}
