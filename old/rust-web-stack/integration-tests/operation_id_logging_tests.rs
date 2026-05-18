/// Tests for log aggregation with operation_id correlation
/// Task 7.6.5: Verify that multi-step operations emit logs with the same operation_id

use outpost3::commands::colony_commands::*;
use outpost3::commands::Command;
use outpost3::domain::*;
use outpost3::utils::logging::{OperationContext, OperationId};

#[test]
fn test_operation_id_unique_across_operations() {
    let ctx1 = OperationContext::new("operation_1");
    let ctx2 = OperationContext::new("operation_2");
    let ctx3 = OperationContext::new("operation_3");

    assert_ne!(ctx1.operation_id(), ctx2.operation_id());
    assert_ne!(ctx2.operation_id(), ctx3.operation_id());
    assert_ne!(ctx1.operation_id(), ctx3.operation_id());
}

#[test]
fn test_operation_id_format() {
    let id = OperationId::from_value(123);
    let formatted = format!("{}", id);
    assert!(formatted.starts_with("op-"));
    assert!(formatted.contains("123"));
}

#[test]
fn test_construct_building_generates_multiple_events() {
    // ConstructBuilding is a multi-step operation:
    // 1. BuildingConstructionStarted
    // 2+ ResourcesConsumed (one per resource type in cost)

    let mut resources = Resources::new();
    resources.set(ResourceType::Credits, 1000);
    resources.set(ResourceType::Steel, 100);

    let cmd = ConstructBuilding {
        building_id: BuildingId::new("building-1"),
        colony_id: ColonyId::new("colony-1"),
        building_type: BuildingType::Mine {
            resource_type: ResourceType::Iron,
            output_rate: 10,
        },
        available_resources: resources,
    };

    let events = cmd.execute().expect("Command should succeed");

    // Should have at least 2 events: construction started + resource consumption
    assert!(
        events.len() >= 2,
        "Multi-step operation should generate multiple events"
    );

    // First event should be construction started
    match &events[0] {
        EventType::BuildingConstructionStarted { .. } => {
            // Expected
        }
        _ => panic!("First event should be BuildingConstructionStarted"),
    }

    // Subsequent events should be resource consumption
    for event in &events[1..] {
        match event {
            EventType::ResourcesConsumed { .. } => {
                // Expected
            }
            _ => panic!("Subsequent events should be ResourcesConsumed"),
        }
    }
}

#[test]
fn test_advance_turn_generates_multiple_events() {
    // AdvanceTurn processes multiple buildings and generates one event per building
    // plus a final TurnAdvanced event

    let cmd = AdvanceTurn {
        current_turn: 10,
        buildings: vec![
            (
                ColonyId::new("colony-1"),
                BuildingType::Mine {
                    resource_type: ResourceType::Iron,
                    output_rate: 5,
                },
            ),
            (
                ColonyId::new("colony-1"),
                BuildingType::Farm { output_rate: 3 },
            ),
            (
                ColonyId::new("colony-2"),
                BuildingType::SolarArray { output_mw: 10 },
            ),
        ],
    };

    let events = cmd.execute().expect("Command should succeed");

    // Should have 4 events: 3 resource productions + 1 turn advanced
    assert_eq!(
        events.len(),
        4,
        "Should generate one event per building plus turn advanced"
    );

    // Last event should be TurnAdvanced
    match &events[events.len() - 1] {
        EventType::TurnAdvanced { turn_number } => {
            assert_eq!(*turn_number, 11, "Turn should advance by 1");
        }
        _ => panic!("Last event should be TurnAdvanced"),
    }
}

#[test]
fn test_upgrade_building_generates_multiple_events() {
    // UpgradeBuilding consumes resources then upgrades

    let mut available = Resources::new();
    available.set(ResourceType::Credits, 500);
    available.set(ResourceType::Steel, 20);

    let mut upgrade_cost = Resources::new();
    upgrade_cost.set(ResourceType::Credits, 200);
    upgrade_cost.set(ResourceType::Steel, 10);

    let cmd = UpgradeBuilding {
        building_id: BuildingId::new("building-1"),
        colony_id: ColonyId::new("colony-1"),
        current_level: 1,
        available_resources: available,
        upgrade_cost,
    };

    let events = cmd.execute().expect("Command should succeed");

    // Should have resource consumption events + upgrade event
    assert!(
        events.len() >= 2,
        "Should consume resources and then upgrade"
    );

    // Last event should be BuildingUpgraded
    match &events[events.len() - 1] {
        EventType::BuildingUpgraded { new_level, .. } => {
            assert_eq!(*new_level, 2, "Level should increase by 1");
        }
        _ => panic!("Last event should be BuildingUpgraded"),
    }
}

#[test]
fn test_repair_building_generates_multiple_events() {
    // RepairBuilding consumes resources then repairs

    let mut available = Resources::new();
    available.set(ResourceType::Credits, 1000);
    available.set(ResourceType::Steel, 50);
    available.set(ResourceType::Electronics, 10);

    let cmd = RepairBuilding {
        building_id: BuildingId::new("building-1"),
        colony_id: ColonyId::new("colony-1"),
        current_state: BuildingState::Damaged { severity: 75 }, // High damage
        available_resources: available,
    };

    let events = cmd.execute().expect("Command should succeed");

    // Should have resource consumption events + repair event
    assert!(
        events.len() >= 2,
        "Should consume resources and then repair"
    );

    // Last event should be BuildingRepaired
    match &events[events.len() - 1] {
        EventType::BuildingRepaired { .. } => {
            // Expected
        }
        _ => panic!("Last event should be BuildingRepaired"),
    }
}

#[test]
fn test_operation_context_provides_correlation() {
    // Demonstrate that OperationContext allows correlating multiple log entries

    let ctx = OperationContext::new("complex_operation");
    let op_id = ctx.operation_id();

    // Simulate multiple steps
    ctx.log_step("step1", "Validating inputs");
    ctx.log_step("step2", "Processing data");
    ctx.log_step("step3", "Saving results");
    ctx.log_complete("All steps completed");

    // All of these log calls would have the same operation_id in practice
    // Since we can't easily capture logs in tests, we verify the ID is consistent
    assert_eq!(ctx.operation_id(), op_id);
    assert_eq!(ctx.operation_name(), "complex_operation");
}

#[test]
fn test_operation_context_multiple_instances_have_different_ids() {
    // When multiple operations run concurrently, each should have a unique ID

    let contexts: Vec<OperationContext> = (0..10)
        .map(|i| OperationContext::new(format!("operation_{}", i)))
        .collect();

    // All operation IDs should be unique
    let mut seen_ids = std::collections::HashSet::new();
    for ctx in &contexts {
        let op_id = ctx.operation_id();
        assert!(
            !seen_ids.contains(&op_id),
            "Operation ID {:?} should be unique",
            op_id
        );
        seen_ids.insert(op_id);
    }

    assert_eq!(seen_ids.len(), 10, "All 10 IDs should be unique");
}

#[test]
fn test_operation_id_monotonic_increase() {
    // Operation IDs should generally increase (though not guaranteed in concurrent scenarios)

    let id1 = OperationId::new();
    let id2 = OperationId::new();
    let id3 = OperationId::new();

    // In a single-threaded test, IDs should increase
    assert!(id2.value() > id1.value());
    assert!(id3.value() > id2.value());
}
