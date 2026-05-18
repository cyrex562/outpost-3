//! Power Grid Tick Processor — MVP.5.
//!
//! Each tick, this processor:
//! 1. Aggregates generation and consumption from operational buildings.
//! 2. Updates `site.power_grid` totals.
//! 3. Computes `site.power_efficiency` (1.0 = surplus, <1.0 = brownout).
//! 4. Emits `PowerGridUpdatedV5` events.

use crate::content::{BuildingCategory, BuildingDefinition, ContentLoader};
use crate::domain::{GameState, Site};
use crate::events::EventType;
use std::collections::HashMap;

/// Essential building categories exempt from brownout efficiency penalty.
/// These buildings always operate at full efficiency regardless of power status.
pub fn is_essential(category: &BuildingCategory) -> bool {
    matches!(
        category,
        BuildingCategory::Housing | BuildingCategory::Infrastructure
    )
}

/// Compute the power efficiency for a specific building during a brownout.
///
/// Essential buildings (Housing, Infrastructure) always get 1.0.
/// Other buildings are penalized proportional to the power shortfall.
pub fn building_power_efficiency(
    building_def_id: &str,
    site_power_efficiency: f64,
    defs: &HashMap<String, BuildingDefinition>,
) -> f64 {
    if site_power_efficiency >= 1.0 {
        return 1.0;
    }
    match defs.get(building_def_id) {
        Some(def) if is_essential(&def.category) => 1.0,
        _ => site_power_efficiency,
    }
}

/// Compute the power generation (MW) for a site from all operational buildings.
pub fn compute_generation(
    site: &Site,
    defs: &HashMap<String, BuildingDefinition>,
) -> f64 {
    site.buildings.values()
        .filter(|b| b.state.can_operate())
        .filter_map(|b| defs.get(&b.building_def_id))
        .filter_map(|def| def.power_output_mw)
        .sum()
}

/// Compute the power consumption (MW) for a site from all operational buildings.
pub fn compute_consumption(
    site: &Site,
    defs: &HashMap<String, BuildingDefinition>,
) -> f64 {
    site.buildings.values()
        .filter(|b| b.state.can_operate())
        .filter_map(|b| {
            let def = defs.get(&b.building_def_id)?;
            def.power.as_ref().map(|p| p.base_mw)
        })
        .sum()
}

/// Process the power grid for all sites in the game state.
///
/// Updates `site.power_grid`, `site.power_efficiency`, and returns events.
pub fn process_power_tick(
    state: &mut GameState,
    _content: &ContentLoader,
) -> Vec<EventType> {
    let mut events = Vec::new();
    let tick = state.current_tick();

    for system in state.galaxy.systems.values_mut() {
        for site in system.sites.values_mut() {
            let generation = compute_generation(site, &state.building_definitions);
            let consumption = compute_consumption(site, &state.building_definitions);

            let gen_mw = (generation * 10.0).round() / 10.0;
            let con_mw = (consumption * 10.0).round() / 10.0;

            site.power_grid.update_generation(gen_mw.round() as u32);
            site.power_grid.update_consumption(con_mw.round() as u32);

            let has_brownout = site.power_grid.has_brownout();

            site.power_efficiency = if has_brownout && con_mw > 0.0 {
                (gen_mw / con_mw).clamp(0.5, 1.0)
            } else {
                1.0
            };

            events.push(EventType::PowerGridUpdatedV5 {
                site_id: site.id,
                generation_mw: gen_mw,
                consumption_mw: con_mw,
                has_brownout,
                power_efficiency: site.power_efficiency,
                tick,
            });
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{BuildingCategory, BuildingDefinition};
    use crate::domain::{CelestialBodyId, Site};
    use crate::domain::building_instance::{BuildingInstance, BuildingState};
    use std::collections::HashMap;

    fn make_site() -> Site {
        Site::new_settlement(CelestialBodyId::new(), "Test Site".to_string(), 0, 50)
    }

    fn make_generator(id: &str, output_mw: f64) -> BuildingDefinition {
        BuildingDefinition {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            category: BuildingCategory::PowerGeneration,
            construction_costs: vec![],
            construction_time_ticks: 0,
            power: None,
            power_output_mw: Some(output_mw),
            worker_capacity: 0,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        }
    }

    fn make_consumer(id: &str, base_mw: f64, cat: BuildingCategory) -> BuildingDefinition {
        use crate::content::building_def::PowerRequirement;
        BuildingDefinition {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            category: cat,
            construction_costs: vec![],
            construction_time_ticks: 0,
            power: Some(PowerRequirement { base_mw, per_level_mw: 0.0 }),
            power_output_mw: None,
            worker_capacity: 0,
            storage_capacity: None,
            housing_capacity: None,
            recipes: vec![],
            maintenance_cost: HashMap::new(),
            upgradeable: false,
            upgrade_to: None,
            morale_bonus: 0.0,
        }
    }

    fn add_building(site: &mut Site, def_id: &str, state: BuildingState) -> BuildingInstance {
        let mut b = BuildingInstance::new_operational(site.id, def_id.to_string(), 0);
        b.state = state;
        let id = b.id;
        site.buildings.insert(id, b.clone());
        b
    }

    #[test]
    fn test_no_buildings_zero_generation_consumption() {
        let site = make_site();
        let defs = HashMap::new();
        assert_eq!(compute_generation(&site, &defs), 0.0);
        assert_eq!(compute_consumption(&site, &defs), 0.0);
    }

    #[test]
    fn test_generation_sums_operational_power_buildings() {
        let mut site = make_site();
        let mut defs = HashMap::new();
        defs.insert("solar".to_string(), make_generator("solar", 50.0));
        defs.insert("wind".to_string(),  make_generator("wind", 30.0));

        add_building(&mut site, "solar", BuildingState::Operational);
        add_building(&mut site, "wind",  BuildingState::Operational);
        // Third building under construction — should NOT count
        add_building(&mut site, "solar", BuildingState::UnderConstruction { progress: 50 });

        let gen = compute_generation(&site, &defs);
        assert!((gen - 80.0).abs() < 0.01, "Expected 80 MW, got {}", gen);
    }

    #[test]
    fn test_paused_buildings_do_not_consume_power() {
        let mut site = make_site();
        let mut defs = HashMap::new();
        defs.insert("smelter".to_string(),
            make_consumer("smelter", 12.0, BuildingCategory::Manufacturing));

        add_building(&mut site, "smelter", BuildingState::Paused);
        let con = compute_consumption(&site, &defs);
        assert_eq!(con, 0.0, "Paused building must not consume power");
    }

    #[test]
    fn test_power_efficiency_is_1_when_surplus() {
        let mut site = make_site();
        let mut defs = HashMap::new();
        defs.insert("solar".to_string(),   make_generator("solar", 100.0));
        defs.insert("smelter".to_string(), make_consumer("smelter", 50.0, BuildingCategory::Manufacturing));

        add_building(&mut site, "solar",   BuildingState::Operational);
        add_building(&mut site, "smelter", BuildingState::Operational);

        site.power_grid.update_generation(100);
        site.power_grid.update_consumption(50);
        assert!(!site.power_grid.has_brownout());

        let efficiency = if site.power_grid.has_brownout() {
            (100.0_f64 / 50.0).clamp(0.5, 1.0)
        } else { 1.0 };
        assert!((efficiency - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_brownout_reduces_efficiency() {
        let mut site = make_site();
        // 60 MW generation vs 100 MW consumption → efficiency = 0.6
        site.power_grid.update_generation(60);
        site.power_grid.update_consumption(100);
        assert!(site.power_grid.has_brownout());

        let gen = 60.0_f64;
        let con = 100.0_f64;
        let eff = (gen / con).clamp(0.5, 1.0);
        assert!((eff - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_brownout_clamped_to_50_percent_minimum() {
        // Even if 10 MW vs 100 MW, efficiency floor is 0.5
        let gen = 10.0_f64;
        let con = 100.0_f64;
        let eff = (gen / con).clamp(0.5, 1.0);
        assert!((eff - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_essential_buildings_get_full_efficiency_during_brownout() {
        let mut defs = HashMap::new();
        defs.insert("habitat".to_string(), make_consumer("habitat", 20.0, BuildingCategory::Housing));
        defs.insert("smelter".to_string(), make_consumer("smelter", 20.0, BuildingCategory::Manufacturing));

        let site_efficiency = 0.5;

        let habitat_eff = building_power_efficiency("habitat", site_efficiency, &defs);
        assert!((habitat_eff - 1.0).abs() < 0.001, "Essential building must have 1.0 efficiency");

        let smelter_eff = building_power_efficiency("smelter", site_efficiency, &defs);
        assert!((smelter_eff - 0.5).abs() < 0.001, "Non-essential building should get site efficiency");
    }

    #[test]
    fn test_infrastructure_is_essential() {
        let mut defs = HashMap::new();
        defs.insert("water_plant".to_string(), make_consumer("water_plant", 10.0, BuildingCategory::Infrastructure));

        let eff = building_power_efficiency("water_plant", 0.6, &defs);
        assert!((eff - 1.0).abs() < 0.001, "Infrastructure must be exempt from brownout");
    }

    #[test]
    fn test_no_brownout_returns_full_efficiency_for_all() {
        let mut defs = HashMap::new();
        defs.insert("smelter".to_string(), make_consumer("smelter", 20.0, BuildingCategory::Manufacturing));

        let eff = building_power_efficiency("smelter", 1.0, &defs);
        assert!((eff - 1.0).abs() < 0.001);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use crate::domain::{CelestialBodyId, Site};
    use std::collections::HashMap;

    fn make_site() -> Site {
        Site::new_settlement(CelestialBodyId::new(), "PropTest Site".to_string(), 0, 50)
    }

    proptest! {
        #[test]
        fn power_efficiency_always_in_valid_range(
            generation in 0u32..=1000,
            consumption in 1u32..=1000,
        ) {
            let mut site = make_site();
            site.power_grid.update_generation(generation);
            site.power_grid.update_consumption(consumption);
            let has_brownout = site.power_grid.has_brownout();

            let gen = generation as f64;
            let con = consumption as f64;

            let efficiency = if has_brownout && con > 0.0 {
                (gen / con).clamp(0.5, 1.0)
            } else {
                1.0
            };

            prop_assert!(efficiency >= 0.5 && efficiency <= 1.0,
                "Efficiency must be in [0.5, 1.0], got {}", efficiency);
        }

        #[test]
        fn no_brownout_when_generation_exceeds_consumption(
            surplus in 1u32..=500,
            consumption in 0u32..=500,
        ) {
            let mut site = make_site();
            let generation = consumption + surplus;
            site.power_grid.update_generation(generation);
            site.power_grid.update_consumption(consumption);
            prop_assert!(!site.power_grid.has_brownout(),
                "No brownout when generation ({}) > consumption ({})", generation, consumption);
        }

        #[test]
        fn brownout_when_consumption_exceeds_generation(
            deficit in 1u32..=500,
            generation in 0u32..=500,
        ) {
            let mut site = make_site();
            let consumption = generation + deficit;
            site.power_grid.update_generation(generation);
            site.power_grid.update_consumption(consumption);
            prop_assert!(site.power_grid.has_brownout(),
                "Brownout expected when consumption ({}) > generation ({})", consumption, generation);
        }

        #[test]
        fn building_power_efficiency_always_clamped(
            site_eff in 0.0f64..=1.0,
        ) {
            let defs: HashMap<String, _> = HashMap::new();
            let eff = building_power_efficiency("unknown", site_eff, &defs);
            prop_assert!(eff >= 0.0 && eff <= 1.0,
                "Building power efficiency must be in [0.0, 1.0], got {}", eff);
        }
    }
}
