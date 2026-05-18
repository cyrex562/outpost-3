//! Building queries — helpers for working with BuildingInstances and BuildingDefinitions.
//!
//! These are pure functions that combine BuildingInstance state with BuildingDefinition data.

use crate::content::{BuildingDefinition, ProductionRecipe};
use crate::domain::BuildingInstance;

/// Get the power consumption of a building instance.
/// Returns MW consumed (positive value), or 0.0 if no power requirement.
pub fn power_consumption(
    instance: &BuildingInstance,
    definition: &BuildingDefinition,
) -> f64 {
    if let Some(power_req) = &definition.power {
        let base = power_req.base_mw;
        let per_level = power_req.per_level_mw * (instance.level as f64 - 1.0);
        (base + per_level) * instance.overall_efficiency(definition.worker_capacity)
    } else {
        0.0
    }
}

/// Get the power generation of a building instance.
/// Returns MW generated, or 0.0 if no power generation.
pub fn power_generation(
    instance: &BuildingInstance,
    definition: &BuildingDefinition,
) -> f64 {
    definition.power_output_mw.unwrap_or(0.0) * instance.overall_efficiency(definition.worker_capacity)
}

/// Get the net power balance (generation - consumption).
/// Positive = net producer, negative = net consumer.
pub fn net_power(
    instance: &BuildingInstance,
    definition: &BuildingDefinition,
) -> f64 {
    power_generation(instance, definition) - power_consumption(instance, definition)
}

/// Check if a building instance is currently producing (operational + has recipe).
pub fn is_producing(instance: &BuildingInstance, _definition: &BuildingDefinition) -> bool {
    instance.state.is_operational() && instance.has_recipe()
}

/// Get the active recipe ID for a building instance.
pub fn active_recipe_id(instance: &BuildingInstance) -> Option<&str> {
    instance.get_recipe_id()
}

/// Calculate production rate multiplier based on instance state.
/// Combines health, labor, and state efficiency.
pub fn production_multiplier(
    instance: &BuildingInstance,
    definition: &BuildingDefinition,
) -> f64 {
    instance.overall_efficiency(definition.worker_capacity)
}

/// Get construction progress as percentage (0-100).
/// Returns None if not under construction.
pub fn construction_progress(
    instance: &BuildingInstance,
    definition: &BuildingDefinition,
    current_tick: u64,
) -> Option<u8> {
    if let Some(started_at) = instance.construction_started_at {
        let elapsed = current_tick.saturating_sub(started_at);
        let total_time = definition.construction_time_ticks;
        
        if total_time == 0 {
            return Some(100); // Instant construction
        }
        
        let progress = ((elapsed as f64 / total_time as f64) * 100.0).min(100.0) as u8;
        Some(progress)
    } else {
        None
    }
}

/// Check if construction is complete.
pub fn is_construction_complete(
    instance: &BuildingInstance,
    definition: &BuildingDefinition,
    current_tick: u64,
) -> bool {
    if let Some(progress) = construction_progress(instance, definition, current_tick) {
        progress >= 100
    } else {
        false
    }
}

/// Calculate maintenance cost per tick based on definition and instance state.
pub fn maintenance_cost_per_tick(
    instance: &BuildingInstance,
    definition: &BuildingDefinition,
) -> std::collections::HashMap<String, f64> {
    let mut costs = definition.maintenance_cost.clone();
    
    // Scale by health - damaged buildings cost more to maintain
    let health_factor = 1.0 + (1.0 - (instance.health as f64 / 100.0)) * 0.5; // Up to 50% extra cost
    
    for amount in costs.values_mut() {
        *amount *= health_factor;
    }
    
    costs
}

/// Check if a building can be upgraded.
pub fn can_upgrade(instance: &BuildingInstance, definition: &BuildingDefinition) -> bool {
    definition.upgradeable
        && definition.upgrade_to.is_some()
        && instance.state.is_operational()
        && instance.health >= 75 // Must be in good condition
}

/// Get the total morale bonus from a building.
pub fn morale_bonus(instance: &BuildingInstance, definition: &BuildingDefinition) -> f64 {
    if instance.state.can_operate() {
        definition.morale_bonus * instance.efficiency()
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::building_def::{BuildingCategory, ConstructionCost, PowerRequirement};
    use crate::domain::{BuildingInstance, SiteId};
    use std::collections::HashMap;

    fn test_building_def() -> BuildingDefinition {
        BuildingDefinition {
            id: "test_mine".to_string(),
            name: "Test Mine".to_string(),
            description: "A test mine".to_string(),
            category: BuildingCategory::Mining,
            construction_costs: vec![ConstructionCost {
                resource_id: "steel".to_string(),
                amount: 100.0,
            }],
            construction_time_ticks: 100,
            power: Some(PowerRequirement {
                base_mw: 5.0,
                per_level_mw: 1.0,
            }),
            power_output_mw: None,
            worker_capacity: 10,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::from([
                ("steel".to_string(), 1.0),
                ("power".to_string(), 0.5),
            ]),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        }
    }

    #[test]
    fn test_power_consumption() {
        let def = test_building_def();
        let instance = BuildingInstance::new_operational(
            SiteId::new(),
            "test_mine".to_string(),
            0,
        );

        // Full health, no workers
        assert_eq!(power_consumption(&instance, &def), 0.0); // No labor = 0 consumption

        // With workers
        let mut instance_with_workers = instance.clone();
        instance_with_workers.assign_workers(10, 10);
        assert_eq!(power_consumption(&instance_with_workers, &def), 5.0); // Full power
    }

    #[test]
    fn test_construction_progress() {
        let def = test_building_def();
        let instance = BuildingInstance::new(SiteId::new(), "test_mine".to_string(), 0);

        // At tick 0 (just started)
        assert_eq!(construction_progress(&instance, &def, 0), Some(0));

        // At tick 50 (halfway)
        assert_eq!(construction_progress(&instance, &def, 50), Some(50));

        // At tick 100 (complete)
        assert_eq!(construction_progress(&instance, &def, 100), Some(100));

        // At tick 150 (overtime, capped)
        assert_eq!(construction_progress(&instance, &def, 150), Some(100));
    }

    #[test]
    fn test_maintenance_cost_scaling() {
        let def = test_building_def();
        
        // Full health
        let instance_healthy = BuildingInstance::new_operational(
            SiteId::new(),
            "test_mine".to_string(),
            0,
        );
        let cost_healthy = maintenance_cost_per_tick(&instance_healthy, &def);
        assert_eq!(cost_healthy.get("steel"), Some(&1.0));

        // Damaged (50% health)
        let mut instance_damaged = instance_healthy.clone();
        instance_damaged.damage(50);
        let cost_damaged = maintenance_cost_per_tick(&instance_damaged, &def);
        assert!(cost_damaged.get("steel").unwrap() > &1.0); // Higher cost
        assert!(cost_damaged.get("steel").unwrap() <= &1.5); // Max 50% increase
    }

    #[test]
    fn test_can_upgrade() {
        let mut def = test_building_def();
        def.upgradeable = true;
        def.upgrade_to = Some("advanced_mine".to_string());

        // Healthy, operational
        let instance_ok = BuildingInstance::new_operational(
            SiteId::new(),
            "test_mine".to_string(),
            0,
        );
        assert!(can_upgrade(&instance_ok, &def));

        // Too damaged
        let mut instance_damaged = instance_ok.clone();
        instance_damaged.damage(30); // 70% health
        assert!(!can_upgrade(&instance_damaged, &def));

        // Paused
        let mut instance_paused = instance_ok.clone();
        instance_paused.pause();
        assert!(!can_upgrade(&instance_paused, &def));
    }
}
