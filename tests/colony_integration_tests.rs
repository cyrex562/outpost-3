use outpost3::commands::*;
use outpost3::domain::production_chain::ProductionChains;
use outpost3::domain::*;
use outpost3::events::*;

#[test]
fn test_multi_turn_production_workflow() {
    // Create a colony
    let colony_id = ColonyId(1);
    let planet_id = PlanetId(1);
    let mut colony = Colony::new(colony_id, planet_id, "Production Colony".to_string());

    // Give the colony some initial resources
    colony.resources.set(ResourceType::Credits, 10000);
    colony.resources.set(ResourceType::Steel, 100);
    colony.resources.set(ResourceType::Machinery, 50);
    colony.resources.set(ResourceType::IronOre, 200);

    // Create a mine building
    let mine = Building::new(
        BuildingId(1),
        colony_id,
        BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
    );

    // Verify mine starts under construction
    assert!(matches!(
        mine.state,
        BuildingState::UnderConstruction { progress: 0 }
    ));

    // Simulate construction progress over multiple turns
    let mut mine = mine.clone();
    let construction_time = mine.construction_time();
    assert_eq!(construction_time, 3); // Mines take 3 turns

    // Turn 1: Still under construction
    let completed = mine.advance_construction();
    assert!(!completed);
    assert!(matches!(
        mine.state,
        BuildingState::UnderConstruction { progress: 1 }
    ));

    // Turn 2: Still under construction
    let completed = mine.advance_construction();
    assert!(!completed);
    assert!(matches!(
        mine.state,
        BuildingState::UnderConstruction { progress: 2 }
    ));

    // Turn 3: Construction completes
    let completed = mine.advance_construction();
    assert!(completed);
    assert!(matches!(mine.state, BuildingState::Operational));

    // Allocate workers to the mine
    colony.population.housing_capacity = 500;
    let allocated = colony.population.allocate_workers(5);
    assert!(allocated);
    mine.workers_assigned = 5;

    // Calculate production efficiency (5 workers out of 10 capacity = 50%)
    let efficiency = mine.production_efficiency();
    assert_eq!(efficiency, 0.5);

    // Get production outputs
    let outputs = mine.production_outputs();
    let iron_produced = outputs.get(ResourceType::IronOre);
    assert_eq!(iron_produced, 5); // 10 * 0.5 efficiency = 5

    // Apply production to colony
    colony
        .resources
        .add_resource(ResourceType::IronOre, iron_produced);
    assert_eq!(colony.resources.get(ResourceType::IronOre), 205);

    // Verify population stats
    assert_eq!(colony.population.employed, 5);
    assert_eq!(colony.population.unemployed, 95);
    assert_eq!(colony.population.unemployment_rate(), 95.0);
}

#[test]
fn test_power_shortage_scenario() {
    let colony_id = ColonyId(1);
    let planet_id = PlanetId(1);
    let mut colony = Colony::new(colony_id, planet_id, "Power Test Colony".to_string());

    // Create a power plant
    let power_plant = Building {
        id: BuildingId(1),
        colony_id,
        building_type: BuildingType::PowerPlant {
            output_mw: 50,
            fuel_type: None,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    };

    // Create energy-consuming buildings
    let mine = Building {
        id: BuildingId(2),
        colony_id,
        building_type: BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    };

    let factory = Building {
        id: BuildingId(3),
        colony_id,
        building_type: BuildingType::Factory {
            produces: ResourceType::Steel,
            consumes: vec![ResourceType::IronOre],
            output_rate: 5,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    };

    // Calculate power grid status
    let mut total_generation = 0u32;
    let mut total_consumption = 0u32;

    total_generation += power_plant.power_generation();
    total_consumption += mine.power_consumption();
    total_consumption += factory.power_consumption();

    colony.power_grid.update_generation(total_generation);
    colony.power_grid.update_consumption(total_consumption);

    // Verify initial state - should have surplus
    assert_eq!(colony.power_grid.total_generation, 50);
    assert_eq!(colony.power_grid.total_consumption, 15); // 5 + 10
    assert!(!colony.power_grid.has_brownout());
    assert_eq!(colony.power_grid.power_surplus(), 35);

    // Add more energy-consuming buildings to cause brownout
    let refinery = Building {
        id: BuildingId(4),
        colony_id,
        building_type: BuildingType::Refinery {
            input_type: ResourceType::IronOre,
            output_type: ResourceType::Steel,
            conversion_rate: 0.8,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    };

    let research_facility = Building {
        id: BuildingId(5),
        colony_id,
        building_type: BuildingType::ResearchFacility { research_rate: 5 },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    };

    let train_station = Building {
        id: BuildingId(6),
        colony_id,
        building_type: BuildingType::TrainStation {
            platforms: 2,
            throughput: 100,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    };

    total_consumption += refinery.power_consumption(); // +12
    total_consumption += research_facility.power_consumption(); // +8
    total_consumption += train_station.power_consumption(); // +15

    colony.power_grid.update_consumption(total_consumption);

    // Now we should have brownout
    assert_eq!(colony.power_grid.total_consumption, 50); // 5 + 10 + 12 + 8 + 15
    assert!(!colony.power_grid.has_brownout()); // Exactly balanced
    assert_eq!(colony.power_grid.net_power(), 0);

    // Add one more building to trigger brownout
    let medical_facility = Building {
        id: BuildingId(7),
        colony_id,
        building_type: BuildingType::MedicalFacility {
            treatment_capacity: 100,
            morale_bonus: 5.0,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    };

    total_consumption += medical_facility.power_consumption(); // +6
    colony.power_grid.update_consumption(total_consumption);

    assert!(colony.power_grid.has_brownout());
    assert_eq!(colony.power_grid.power_deficit(), 6);
    assert_eq!(colony.power_grid.net_power(), -6);

    // Verify we can't support additional load (not even 0 since we're already in brownout)
    assert!(!colony.power_grid.can_support_additional_load(10));
    assert!(!colony.power_grid.can_support_additional_load(1));
}

#[test]
fn test_population_growth_and_labor_allocation() {
    let colony_id = ColonyId(1);
    let planet_id = PlanetId(1);
    let mut colony = Colony::new(colony_id, planet_id, "Growth Colony".to_string());

    // Initial state
    assert_eq!(colony.population.total, 100);
    assert_eq!(colony.population.employed, 0);
    assert_eq!(colony.population.unemployed, 100);
    assert_eq!(colony.population.housing_capacity, 100);

    // Add housing to allow growth
    colony.population.add_housing(200);
    assert_eq!(colony.population.housing_capacity, 300);

    // Simulate population growth over multiple turns
    let initial_pop = colony.population.total;
    colony.population.grow();
    assert!(colony.population.total > initial_pop);
    let turn1_pop = colony.population.total;

    // Grow again
    colony.population.grow();
    assert!(colony.population.total > turn1_pop);

    // Create buildings and allocate labor
    let mut buildings = vec![];

    // Mine
    buildings.push(Building {
        id: BuildingId(1),
        colony_id,
        building_type: BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    });

    // Farm
    buildings.push(Building {
        id: BuildingId(2),
        colony_id,
        building_type: BuildingType::Farm { output_rate: 15 },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    });

    // Factory
    buildings.push(Building {
        id: BuildingId(3),
        colony_id,
        building_type: BuildingType::Factory {
            produces: ResourceType::Steel,
            consumes: vec![ResourceType::IronOre],
            output_rate: 5,
        },
        state: BuildingState::Operational,
        workers_assigned: 0,
        level: 1,
        recipe_index: 0,
    });

    // Allocate workers to each building
    for building in &mut buildings {
        let capacity = building.worker_capacity();
        let workers_to_allocate = (capacity / 2).min(colony.population.unemployed as u32);

        if workers_to_allocate > 0 {
            assert!(colony
                .population
                .allocate_workers(workers_to_allocate as u64));
            building.workers_assigned = workers_to_allocate;
        }
    }

    // Verify labor allocation
    assert!(colony.population.employed > 0);
    assert_eq!(
        colony.population.total,
        colony.population.employed + colony.population.unemployed
    );

    // Verify production efficiency
    for building in &buildings {
        let efficiency = building.production_efficiency();
        assert!(efficiency > 0.0 && efficiency <= 1.0);

        if building.workers_assigned > 0 {
            assert!(efficiency > 0.0);
        }
    }

    // Test deallocating workers
    let initial_employed = colony.population.employed;
    let workers_to_remove = 5;

    assert!(colony
        .population
        .deallocate_workers(workers_to_remove as u64));
    assert_eq!(
        colony.population.employed,
        initial_employed - workers_to_remove as u64
    );
    assert_eq!(
        colony.population.unemployed,
        colony.population.total - colony.population.employed
    );
}

#[test]
fn test_complete_production_chain() {
    let colony_id = ColonyId(1);
    let planet_id = PlanetId(1);
    let mut colony = Colony::new(colony_id, planet_id, "Production Chain Colony".to_string());

    // Set up initial resources (clear starting resources first)
    colony.resources = Resources::new();
    colony.resources.set(ResourceType::IronOre, 100);
    colony.resources.set(ResourceType::CopperOre, 50);

    // Create a refinery to convert IronOre to Steel
    let mut refinery = Building {
        id: BuildingId(1),
        colony_id,
        building_type: BuildingType::Refinery {
            input_type: ResourceType::IronOre,
            output_type: ResourceType::Steel,
            conversion_rate: 1.0,
        },
        state: BuildingState::Operational,
        workers_assigned: 15, // Full capacity
        level: 1,
        recipe_index: 0,
    };

    // Check production requirements
    let inputs = refinery.production_inputs();
    assert_eq!(inputs.get(ResourceType::IronOre), 2);

    // Verify we can produce
    assert!(refinery.can_produce(&colony.resources));

    // Get outputs
    let outputs = refinery.production_outputs();
    assert_eq!(outputs.get(ResourceType::Steel), 2); // 2 * 1.0 * 1.0 efficiency

    // Simulate production turn
    let inputs = refinery.production_inputs();
    for (resource_type, amount) in inputs.iter() {
        colony.resources.subtract(*resource_type, *amount);
    }

    let outputs = refinery.production_outputs();
    for (resource_type, amount) in outputs.iter() {
        colony.resources.add_resource(*resource_type, *amount);
    }

    // Verify resources changed correctly
    assert_eq!(colony.resources.get(ResourceType::IronOre), 98);
    assert_eq!(colony.resources.get(ResourceType::Steel), 2);

    // Test production chain lookup
    let recipe = ProductionChains::get_recipe(ResourceType::IronOre, ResourceType::Steel);
    assert!(recipe.is_some());

    let recipe = recipe.unwrap();
    assert_eq!(recipe.inputs.len(), 1);
    assert_eq!(recipe.inputs[0].0, ResourceType::IronOre);
    assert_eq!(recipe.outputs.len(), 1);
    assert_eq!(recipe.outputs[0].0, ResourceType::Steel);
}

#[test]
fn test_building_construction_with_commands() {
    let colony_id = ColonyId(1);
    let planet_id = PlanetId(1);
    let mut colony = Colony::new(colony_id, planet_id, "Command Test Colony".to_string());

    // Set up resources for construction
    colony.resources.set(ResourceType::Credits, 5000);
    colony.resources.set(ResourceType::Steel, 100);
    colony.resources.set(ResourceType::Machinery, 50);

    // Test ConstructBuilding command
    let building_type = BuildingType::Mine {
        resource_type: ResourceType::IronOre,
        output_rate: 10,
    };

    let construct_cmd = ConstructBuilding {
        building_id: BuildingId(1),
        colony_id,
        building_type: building_type.clone(),
        available_resources: colony.resources.clone(),
    };

    // Validate command
    assert!(construct_cmd.validate().is_ok());

    // Execute command
    let events = construct_cmd.execute().unwrap();
    assert!(!events.is_empty());

    // Verify BuildingConstructionStarted event
    assert!(events
        .iter()
        .any(|e| matches!(e, EventType::BuildingConstructionStarted { .. })));

    // Verify resource consumption events
    assert!(events
        .iter()
        .any(|e| matches!(e, EventType::ResourcesConsumed { .. })));
}

#[test]
fn test_labor_allocation_commands() {
    let colony_id = ColonyId(1);
    let building_id = BuildingId(1);

    // Test valid allocation
    let allocate_cmd = AllocateLabor {
        colony_id,
        building_id,
        workers_to_allocate: 5,
        available_unemployed: 50,
        building_capacity: 10,
        current_workers: 0,
    };

    assert!(allocate_cmd.validate().is_ok());
    let events = allocate_cmd.execute().unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        EventType::LaborAllocated {
            workers_allocated: 5,
            ..
        }
    ));

    // Test allocation beyond capacity
    let invalid_allocate = AllocateLabor {
        colony_id,
        building_id,
        workers_to_allocate: 15,
        available_unemployed: 50,
        building_capacity: 10,
        current_workers: 0,
    };

    assert!(invalid_allocate.validate().is_err());

    // Test allocation with insufficient workers
    let insufficient_workers = AllocateLabor {
        colony_id,
        building_id,
        workers_to_allocate: 10,
        available_unemployed: 5,
        building_capacity: 20,
        current_workers: 0,
    };

    assert!(insufficient_workers.validate().is_err());

    // Test deallocation
    let deallocate_cmd = DeallocateLabor {
        colony_id,
        building_id,
        workers_to_deallocate: 3,
        current_workers: 5,
    };

    assert!(deallocate_cmd.validate().is_ok());
    let events = deallocate_cmd.execute().unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(
        events[0],
        EventType::LaborDeallocated {
            workers_deallocated: 3,
            ..
        }
    ));
}

#[test]
fn test_building_upgrade_system() {
    let colony_id = ColonyId(1);
    let building_id = BuildingId(1);

    // Create a level 1 mine
    let mut mine = Building {
        id: building_id,
        colony_id,
        building_type: BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
        state: BuildingState::Operational,
        workers_assigned: 10,
        level: 1,
        recipe_index: 0,
    };

    // Verify initial state
    assert_eq!(mine.level, 1);
    assert_eq!(mine.max_level(), 5);
    assert!(mine.can_upgrade());

    // Check initial production (level 1)
    let base_outputs = mine.production_outputs();
    let base_iron = base_outputs.get(ResourceType::IronOre);
    assert_eq!(base_iron, 10); // 10 output * 1.0 efficiency * 1.0 level multiplier

    // Check upgrade cost for level 2
    let upgrade_cost_l2 = mine.upgrade_cost();
    assert_eq!(upgrade_cost_l2.get(ResourceType::Credits), 2000); // 1000 * (1 + 1)
    assert_eq!(upgrade_cost_l2.get(ResourceType::Steel), 20); // 10 * (1 + 1)
                                                              // Mine doesn't require Machinery for upgrades

    // Set up resources for upgrade
    let mut resources = Resources::new();
    resources.set(ResourceType::Credits, 10000);
    resources.set(ResourceType::Steel, 100);
    resources.set(ResourceType::Machinery, 50);

    // Test UpgradeBuilding command for level 1 -> 2
    let upgrade_cmd = UpgradeBuilding {
        building_id,
        colony_id,
        current_level: mine.level,
        available_resources: resources.clone(),
        upgrade_cost: upgrade_cost_l2.clone(),
    };

    // Validate and execute
    assert!(upgrade_cmd.validate().is_ok());
    let events = upgrade_cmd.execute().unwrap();
    assert!(!events.is_empty());

    // Verify BuildingUpgraded event
    assert!(events
        .iter()
        .any(|e| matches!(e, EventType::BuildingUpgraded { new_level: 2, .. })));

    // Verify resource consumption events
    let resource_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, EventType::ResourcesConsumed { .. }))
        .collect();
    assert_eq!(resource_events.len(), 2); // Credits, Steel (Mine doesn't need Machinery)

    // Apply upgrade
    mine.upgrade();
    assert_eq!(mine.level, 2);
    assert_eq!(mine.level_multiplier(), 1.2); // 1.0 + 0.2 * (2 - 1)

    // Check production after upgrade to level 2 (20% bonus)
    let upgraded_outputs = mine.production_outputs();
    let upgraded_iron = upgraded_outputs.get(ResourceType::IronOre);
    assert_eq!(upgraded_iron, 12); // 10 * 1.0 efficiency * 1.2 level multiplier

    // Upgrade to level 3
    let upgrade_cost_l3 = mine.upgrade_cost();
    assert_eq!(upgrade_cost_l3.get(ResourceType::Credits), 3000); // 1000 * (2 + 1)
    assert_eq!(upgrade_cost_l3.get(ResourceType::Steel), 30); // 10 * (2 + 1)

    mine.upgrade();
    assert_eq!(mine.level, 3);
    assert_eq!(mine.level_multiplier(), 1.4); // 1.0 + 0.2 * (3 - 1)

    let l3_outputs = mine.production_outputs();
    assert_eq!(l3_outputs.get(ResourceType::IronOre), 14); // 10 * 1.0 * 1.4

    // Upgrade to level 4
    mine.upgrade();
    assert_eq!(mine.level, 4);
    assert_eq!(mine.level_multiplier(), 1.6); // 1.0 + 0.2 * (4 - 1)

    let l4_outputs = mine.production_outputs();
    assert_eq!(l4_outputs.get(ResourceType::IronOre), 16); // 10 * 1.0 * 1.6

    // Upgrade to level 5 (max)
    mine.upgrade();
    assert_eq!(mine.level, 5);
    assert_eq!(mine.level_multiplier(), 1.8); // 1.0 + 0.2 * (5 - 1)
    assert!(!mine.can_upgrade()); // At max level

    let l5_outputs = mine.production_outputs();
    assert_eq!(l5_outputs.get(ResourceType::IronOre), 18); // 10 * 1.0 * 1.8

    // Verify can't upgrade past max level
    let empty_cost = mine.upgrade_cost();
    assert_eq!(empty_cost.get(ResourceType::Credits), 0);
    assert_eq!(empty_cost.get(ResourceType::Steel), 0);
}

#[test]
fn test_upgrade_with_insufficient_resources() {
    let colony_id = ColonyId(1);
    let building_id = BuildingId(1);

    let mine = Building {
        id: building_id,
        colony_id,
        building_type: BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
        state: BuildingState::Operational,
        workers_assigned: 10,
        level: 1,
        recipe_index: 0,
    };

    let upgrade_cost = mine.upgrade_cost();

    // Not enough resources
    let mut insufficient_resources = Resources::new();
    insufficient_resources.set(ResourceType::Credits, 100); // Need 2000
    insufficient_resources.set(ResourceType::Steel, 5); // Need 20

    let upgrade_cmd = UpgradeBuilding {
        building_id,
        colony_id,
        current_level: 1,
        available_resources: insufficient_resources,
        upgrade_cost,
    };

    // Should fail validation
    assert!(upgrade_cmd.validate().is_err());
}

#[test]
fn test_upgrade_at_max_level() {
    let colony_id = ColonyId(1);
    let building_id = BuildingId(1);

    let max_level_mine = Building {
        id: building_id,
        colony_id,
        building_type: BuildingType::Mine {
            resource_type: ResourceType::IronOre,
            output_rate: 10,
        },
        state: BuildingState::Operational,
        workers_assigned: 10,
        level: 5,
        recipe_index: 0,
    };

    assert!(!max_level_mine.can_upgrade());

    let mut resources = Resources::new();
    resources.set(ResourceType::Credits, 10000);
    resources.set(ResourceType::Steel, 100);

    let upgrade_cost = max_level_mine.upgrade_cost(); // Should be empty

    let upgrade_cmd = UpgradeBuilding {
        building_id,
        colony_id,
        current_level: 5,
        available_resources: resources,
        upgrade_cost,
    };

    // Should fail validation due to max level
    assert!(upgrade_cmd.validate().is_err());
}

#[test]
fn test_building_recipe_switching_validation() {
    // Test SetRecipe command validation
    let set_recipe = SetRecipe {
        building_id: BuildingId(1),
        colony_id: ColonyId(1),
        recipe_index: 0,
        current_recipe_index: 0,
    };

    // Should fail if recipe_index equals current_recipe_index
    assert!(set_recipe.validate().is_err());

    // Should succeed if recipe_index differs
    let set_recipe_valid = SetRecipe {
        building_id: BuildingId(1),
        colony_id: ColonyId(1),
        recipe_index: 1,
        current_recipe_index: 0,
    };
    assert!(set_recipe_valid.validate().is_ok());

    // Should fail if recipe_index is out of valid range
    let set_recipe_invalid_range = SetRecipe {
        building_id: BuildingId(1),
        colony_id: ColonyId(1),
        recipe_index: 100,
        current_recipe_index: 0,
    };
    assert!(set_recipe_invalid_range.validate().is_err());
}

#[test]
fn test_building_recipe_switching_event_generation() {
    // Test that SetRecipe command generates correct event
    let set_recipe = SetRecipe {
        building_id: BuildingId(1),
        colony_id: ColonyId(1),
        recipe_index: 2,
        current_recipe_index: 0,
    };

    let events = set_recipe.execute().expect("Should execute successfully");
    assert_eq!(events.len(), 1);

    match &events[0] {
        EventType::BuildingRecipeChanged {
            building_id,
            colony_id,
            old_recipe_index,
            new_recipe_index,
        } => {
            assert_eq!(*building_id, BuildingId(1));
            assert_eq!(*colony_id, ColonyId(1));
            assert_eq!(*old_recipe_index, 0);
            assert_eq!(*new_recipe_index, 2);
        }
        _ => panic!("Expected BuildingRecipeChanged event"),
    }
}

#[test]
fn test_building_recipe_index_initialization() {
    // Test that new buildings initialize with recipe_index 0
    let building = Building::new(
        BuildingId(1),
        ColonyId(1),
        BuildingType::Farm { output_rate: 5 },
    );

    assert_eq!(building.recipe_index, 0);
    assert_eq!(building.level, 1);
}

#[test]
fn test_switch_recipe_mid_game() {
    // Test switching recipe while building is operational
    let mut building = Building::new(
        BuildingId(1),
        ColonyId(1),
        BuildingType::Farm { output_rate: 5 },
    );

    // Set building to operational (simulate construction completion)
    building.state = BuildingState::Operational;
    assert!(building.is_operational());

    // Initial recipe is 0 (traditional farm: Water → Food)
    assert_eq!(building.recipe_index, 0);

    // Create command to switch to recipe 1 (hydroponic: Water + Nutrients + Energy → Food)
    let set_recipe = SetRecipe {
        building_id: building.id,
        colony_id: building.colony_id,
        recipe_index: 1,
        current_recipe_index: building.recipe_index,
    };

    assert!(set_recipe.validate().is_ok());
    let events = set_recipe.execute().expect("Should execute successfully");
    assert_eq!(events.len(), 1);

    // Verify event contains correct recipe change
    match &events[0] {
        EventType::BuildingRecipeChanged {
            old_recipe_index,
            new_recipe_index,
            ..
        } => {
            assert_eq!(*old_recipe_index, 0);
            assert_eq!(*new_recipe_index, 1);
        }
        _ => panic!("Expected BuildingRecipeChanged event"),
    }
}

#[test]
fn test_recipe_switching_sequence() {
    // Test multiple sequential recipe changes
    let building_id = BuildingId(1);
    let colony_id = ColonyId(1);

    // Start with recipe 0
    let mut current_recipe = 0u32;

    // Switch 0 → 1
    let cmd1 = SetRecipe {
        building_id,
        colony_id,
        recipe_index: 1,
        current_recipe_index: current_recipe,
    };
    assert!(cmd1.validate().is_ok());
    let events = cmd1.execute().expect("Should execute");
    assert_eq!(events.len(), 1);
    current_recipe = 1;

    // Switch 1 → 2 (chemical synthesis: Chemicals + Energy → Food)
    let cmd2 = SetRecipe {
        building_id,
        colony_id,
        recipe_index: 2,
        current_recipe_index: current_recipe,
    };
    assert!(cmd2.validate().is_ok());
    let events = cmd2.execute().expect("Should execute");
    assert_eq!(events.len(), 1);
    current_recipe = 2;

    // Try to switch back to 1
    let cmd3 = SetRecipe {
        building_id,
        colony_id,
        recipe_index: 1,
        current_recipe_index: current_recipe,
    };
    assert!(cmd3.validate().is_ok());
    let events = cmd3.execute().expect("Should execute");
    assert_eq!(events.len(), 1);

    // Verify no duplicate switches
    let dup_cmd = SetRecipe {
        building_id,
        colony_id,
        recipe_index: 1,
        current_recipe_index: 1,
    };
    assert!(dup_cmd.validate().is_err());
}

#[test]
fn test_recipe_logging_format() {
    // Test that recipe switching events are properly formatted
    let set_recipe = SetRecipe {
        building_id: BuildingId(42),
        colony_id: ColonyId(7),
        recipe_index: 1,
        current_recipe_index: 0,
    };

    let events = set_recipe.execute().expect("Should execute");
    match &events[0] {
        EventType::BuildingRecipeChanged {
            building_id,
            colony_id,
            old_recipe_index,
            new_recipe_index,
        } => {
            // Verify format matches logging spec: "Building {id} switched recipe: {old} -> {new}"
            let log_msg = format!(
                "Building {:?} switched recipe: {} -> {}",
                building_id, old_recipe_index, new_recipe_index
            );
            assert!(log_msg.contains("Building"));
            assert!(log_msg.contains("switched recipe"));
            assert!(log_msg.contains("0 -> 1"));
            assert_eq!(*colony_id, ColonyId(7));
        }
        _ => panic!("Expected BuildingRecipeChanged event"),
    }
}

#[test]
fn test_multiple_buildings_independent_recipes() {
    // Test that multiple buildings have independent recipe indices
    let colony_id = ColonyId(1);

    let building1 = Building::new(
        BuildingId(1),
        colony_id,
        BuildingType::Farm { output_rate: 5 },
    );

    let building2 = Building::new(
        BuildingId(2),
        colony_id,
        BuildingType::Farm { output_rate: 5 },
    );

    assert_eq!(building1.recipe_index, 0);
    assert_eq!(building2.recipe_index, 0);

    // Switch recipe for building 1
    let cmd1 = SetRecipe {
        building_id: building1.id,
        colony_id,
        recipe_index: 2,
        current_recipe_index: building1.recipe_index,
    };
    assert!(cmd1.validate().is_ok());

    // Building 2 still on recipe 0
    let cmd2_attempt = SetRecipe {
        building_id: building2.id,
        colony_id,
        recipe_index: 1,
        current_recipe_index: building2.recipe_index,
    };
    assert!(cmd2_attempt.validate().is_ok());
}
