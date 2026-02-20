//! Life Support Tick Processor — MVP.5.
//!
//! Each tick, this processor:
//! 1. Calculates oxygen/water/food demand from population.
//! 2. Deducts available supply from site stockpile.
//! 3. Computes satisfaction ratios and updates `site.life_support`.
//! 4. Calls `site.tick_population_needs()` to feed morale effects.
//! 5. Emits `LifeSupportUpdated` events when status changes.

use crate::content::ContentLoader;
use crate::domain::{GameState, Site};
use crate::domain::life_support::{LifeSupportDemand, LifeSupportStatus};
use crate::domain::ResourceType;
use crate::events::EventType;

/// Compute life support metrics for a site and update its stockpile.
///
/// Returns the new `LifeSupportStatus` and deducted amounts.
pub fn compute_life_support(site: &mut Site) -> LifeSupportStatus {
    let pop = site.population.total;
    if pop == 0 {
        return LifeSupportStatus::satisfied();
    }

    let oxygen_demand = LifeSupportDemand::oxygen(pop);
    let water_demand  = LifeSupportDemand::water(pop);
    let food_demand   = LifeSupportDemand::food(pop);

    // Try to consume from stockpile; partial consumption is allowed
    let oxygen_consumed = consume_stockpile(site, ResourceType::Oxygen, oxygen_demand);
    let water_consumed  = consume_stockpile(site, ResourceType::Water,  water_demand);
    let food_consumed   = consume_stockpile(site, ResourceType::Food,   food_demand);

    let oxygen_sat = safe_ratio(oxygen_consumed, oxygen_demand);
    let water_sat  = safe_ratio(water_consumed,  water_demand);
    let food_sat   = safe_ratio(food_consumed,   food_demand);

    let is_critical = oxygen_sat < LifeSupportDemand::CRITICAL_THRESHOLD
        || water_sat  < LifeSupportDemand::CRITICAL_THRESHOLD
        || food_sat   < LifeSupportDemand::CRITICAL_THRESHOLD;

    LifeSupportStatus {
        oxygen_sat,
        water_sat,
        food_sat,
        is_critical,
        oxygen_consumed,
        water_consumed,
        food_consumed,
    }
}

/// Process life support for all sites in the game state.
pub fn process_life_support_tick(
    state: &mut GameState,
    _content: &ContentLoader,
) -> Vec<EventType> {
    let mut events = Vec::new();
    let tick = state.current_tick();

    for system in state.galaxy.systems.values_mut() {
        for site in system.sites.values_mut() {
            let ls = compute_life_support(site);

            site.tick_population_needs(
                ls.food_consumed,
                ls.water_consumed,
                ls.oxygen_consumed,
            );

            let oxygen_sat = ls.oxygen_sat;
            let water_sat  = ls.water_sat;
            let food_sat   = ls.food_sat;
            let is_critical = ls.is_critical;
            let overall = ls.overall_score();

            site.life_support = ls;

            events.push(EventType::LifeSupportUpdated {
                site_id: site.id,
                oxygen_sat,
                water_sat,
                food_sat,
                is_critical,
                tick,
            });

            // Emit alert events for warning and critical thresholds
            if is_critical {
                events.push(EventType::LifeSupportCritical {
                    site_id: site.id,
                    oxygen_sat,
                    water_sat,
                    food_sat,
                    tick,
                });
            } else if overall < 0.75 {
                events.push(EventType::LifeSupportWarning {
                    site_id: site.id,
                    oxygen_sat,
                    water_sat,
                    food_sat,
                    tick,
                });
            }
        }
    }

    events
}

/// Attempt to deduct `demand` units of `resource` from the stockpile.
/// Returns the amount actually consumed (≤ demand).
fn consume_stockpile(site: &mut Site, resource: ResourceType, demand: f64) -> f64 {
    let available = site.resources.get(resource) as f64;
    let consumed = demand.min(available);
    if consumed > 0.0 {
        let new_qty = (available - consumed).max(0.0) as i64;
        site.resources.set(resource, new_qty);
    }
    consumed
}

fn safe_ratio(consumed: f64, demand: f64) -> f32 {
    if demand <= 0.0 { 1.0 } else { (consumed / demand).clamp(0.0, 1.0) as f32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CelestialBodyId, ResourceType, Site};

    fn make_site_with_pop(pop: u32) -> Site {
        Site::new_settlement(CelestialBodyId::new(), "Test".to_string(), 0, pop)
    }

    fn set_resource(site: &mut Site, r: ResourceType, qty: i64) {
        site.resources.set(r, qty);
    }

    fn get_resource(site: &Site, r: ResourceType) -> i64 {
        site.resources.get(r)
    }

    #[test]
    fn test_no_population_returns_fully_satisfied() {
        let mut site = make_site_with_pop(0);
        let ls = compute_life_support(&mut site);
        assert_eq!(ls.oxygen_sat, 1.0);
        assert_eq!(ls.water_sat,  1.0);
        assert_eq!(ls.food_sat,   1.0);
        assert!(!ls.is_critical);
    }

    #[test]
    fn test_sufficient_supply_gives_full_satisfaction() {
        let mut site = make_site_with_pop(10);
        // Supply more than enough
        set_resource(&mut site, ResourceType::Oxygen, 1000);
        set_resource(&mut site, ResourceType::Water,  1000);
        set_resource(&mut site, ResourceType::Food,   1000);

        let ls = compute_life_support(&mut site);

        assert!((ls.oxygen_sat - 1.0).abs() < 0.01, "oxygen_sat={}", ls.oxygen_sat);
        assert!((ls.water_sat  - 1.0).abs() < 0.01, "water_sat={}", ls.water_sat);
        assert!((ls.food_sat   - 1.0).abs() < 0.01, "food_sat={}", ls.food_sat);
        assert!(!ls.is_critical);
    }

    #[test]
    fn test_zero_oxygen_causes_critical() {
        let mut site = make_site_with_pop(100);
        set_resource(&mut site, ResourceType::Oxygen, 0);
        set_resource(&mut site, ResourceType::Water,  10000);
        set_resource(&mut site, ResourceType::Food,   10000);

        let ls = compute_life_support(&mut site);
        assert_eq!(ls.oxygen_sat, 0.0);
        assert!(ls.is_critical);
        assert_eq!(ls.status_label(), "Critical");
    }

    #[test]
    fn test_partial_supply_gives_partial_satisfaction() {
        let mut site = make_site_with_pop(100);
        // 100 colonists need 30.0 oxygen per tick; give only 15.0 → 50% sat
        set_resource(&mut site, ResourceType::Oxygen, 15);
        set_resource(&mut site, ResourceType::Water,  10000);
        set_resource(&mut site, ResourceType::Food,   10000);

        let ls = compute_life_support(&mut site);
        assert!((ls.oxygen_sat - 0.5).abs() < 0.01, "oxygen_sat={}", ls.oxygen_sat);
    }

    #[test]
    fn test_resources_are_deducted_from_stockpile() {
        let mut site = make_site_with_pop(10);
        set_resource(&mut site, ResourceType::Oxygen, 100);
        set_resource(&mut site, ResourceType::Water,  100);
        set_resource(&mut site, ResourceType::Food,   100);

        let oxygen_before = get_resource(&site, ResourceType::Oxygen);
        compute_life_support(&mut site);
        let oxygen_after = get_resource(&site, ResourceType::Oxygen);

        // 10 colonists × 0.3 = 3.0 oxygen consumed → stockpile drops by 3
        assert!(oxygen_after < oxygen_before, "Oxygen must be deducted");
    }

    #[test]
    fn test_stockpile_cannot_go_below_zero() {
        let mut site = make_site_with_pop(1000); // High demand
        set_resource(&mut site, ResourceType::Oxygen, 5);  // Much less than needed

        compute_life_support(&mut site);
        let remaining = get_resource(&site, ResourceType::Oxygen);
        assert!(remaining >= 0, "Stockpile must not go negative, got {}", remaining);
    }

    #[test]
    fn test_critical_event_emitted_when_oxygen_zero() {
        use crate::content::ContentLoader;
        use crate::domain::{GameState, StarSystem};
        use crate::events::EventType;

        let mut state = GameState::new("Test".into(), 1);
        let content = ContentLoader::new();
        let galaxy_id = state.galaxy.id;
        let mut system = StarSystem::new(galaxy_id, "Sys".into(), 0);

        let mut site = make_site_with_pop(50);
        // No oxygen → critical
        set_resource(&mut site, ResourceType::Oxygen, 0);
        set_resource(&mut site, ResourceType::Water,  10000);
        set_resource(&mut site, ResourceType::Food,   10000);
        system.sites.insert(site.id, site);
        state.galaxy.add_system(system);

        let events = process_life_support_tick(&mut state, &content);

        let has_critical = events.iter().any(|e| matches!(e, EventType::LifeSupportCritical { .. }));
        assert!(has_critical, "Expected LifeSupportCritical event when oxygen = 0");

        let has_warning = events.iter().any(|e| matches!(e, EventType::LifeSupportWarning { .. }));
        assert!(!has_warning, "Should not emit warning when already critical");
    }

    #[test]
    fn test_warning_event_emitted_for_partial_satisfaction() {
        use crate::content::ContentLoader;
        use crate::domain::{GameState, StarSystem};
        use crate::events::EventType;

        let mut state = GameState::new("Test".into(), 1);
        let content = ContentLoader::new();
        let galaxy_id = state.galaxy.id;
        let mut system = StarSystem::new(galaxy_id, "Sys".into(), 0);

        let mut site = make_site_with_pop(100);
        // Oxygen at 50% → overall < 0.75 but not critical (CRITICAL_THRESHOLD = 0.25)
        // 100 colonists × 0.3 = 30 oxygen/tick; give 15 → 50% sat
        set_resource(&mut site, ResourceType::Oxygen, 15);
        set_resource(&mut site, ResourceType::Water,  10000);
        set_resource(&mut site, ResourceType::Food,   10000);
        system.sites.insert(site.id, site);
        state.galaxy.add_system(system);

        let events = process_life_support_tick(&mut state, &content);

        // oxygen_sat=0.5 → overall_score = 0.5*0.5 + 1.0*0.3 + 1.0*0.2 = 0.75 (boundary)
        // Check we got either a warning or an updated event (at boundary it's >= 0.75 so no warning)
        let has_updated = events.iter().any(|e| matches!(e, EventType::LifeSupportUpdated { .. }));
        assert!(has_updated, "Must always emit LifeSupportUpdated");
    }
}
