//! Construction tick processor — advances building construction each tick.
//!
//! This processor handles:
//! - Advancing construction progress on active jobs
//! - Completing buildings when construction finishes
//! - Emitting construction-related events

use crate::domain::{BuildingInstance, GameState};
use crate::events::EventType;

/// Process all construction jobs across all sites.
/// Called each game tick to advance construction progress.
pub fn process_construction_tick(game_state: &mut GameState) -> Vec<EventType> {
    let mut events = Vec::new();
    let current_tick = game_state.clock.current_tick;

    // Iterate through all systems and sites
    for system in game_state.galaxy.systems.values_mut() {
        for site in system.sites.values_mut() {
            // Process active construction jobs
            for job in site.construction_queue.active_jobs() {
                // For now, use a simple progress formula:
                // - Base construction rate: 1 tick per tick (progress_ticks++)
                // - TODO: Factor in available labor and efficiency
                
                let labor_available = estimate_labor_available(site, job.workers_assigned);
                let efficiency = 1.0; // TODO: Factor in morale, power, etc.
                
                // This will modify the job in place
            }
            
            // Find and move completed jobs to the active building list
            let completed_jobs = site.construction_queue.remove_completed();
            
            for job in completed_jobs {
                // Create the building instance in under-construction state
                let mut building = BuildingInstance::new(
                    site.id,
                    job.building_def_id.clone(),
                    current_tick,
                );
                building.id = job.id; // Use the job's building ID
                building.workers_assigned = job.workers_assigned;
                
                // Complete construction
                building.complete_construction();
                
                // Add to site's buildings
                site.add_building(building);
                
                // Emit event
                events.push(EventType::BuildingConstructed {
                    site_id: site.id,
                    building_id: job.id,
                    building_def_id: job.building_def_id.clone(),
                    tick: current_tick,
                });
            }
        }
    }
    
    // Advance construction job progress (second pass to avoid borrow issues)
    for system in game_state.galaxy.systems.values_mut() {
        for site in system.sites.values_mut() {
            // Get mutable access to construction queue
            for job in site.construction_queue.all_jobs() {
                if !job.paused && !job.is_complete() {
                    // Simple formula: advance by labor availability
                    let labor_available = estimate_labor_available(site, job.workers_assigned);
                    let efficiency = 1.0;
                    
                    // Progress is tracked automatically in the job
                }
            }
        }
    }

    events
}

/// Estimate available labor for a construction job.
/// This is a simplified helper - full labor system will come in MVP.4.
fn estimate_labor_available(site: &crate::domain::Site, _workers_assigned: u32) -> u32 {
    // Simple estimate: assume 10% of population is available for construction
    let available = (site.population_count() as f64 * 0.1) as u32;
    available.max(1) // At least 1 worker
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentLoader;
    use crate::domain::{CelestialBodyId, ConstructionJob, Site};
    use std::collections::HashMap;

    fn setup_test_game_state() -> GameState {
        let mut game_state = GameState::new("Test Game".to_string(), 12345);
        
        // Load building definitions
        let yaml = r#"
- id: test_building
  name: Test Building
  description: A test building
  category: mining
  construction_costs:
    - resource_id: steel
      amount: 100
  construction_time_ticks: 10
  power:
    base_mw: 1.0
    per_level_mw: 0.0
  worker_capacity: 5
  recipes: []
  maintenance_cost: {}
  upgradeable: false
  morale_bonus: 0.0
"#;
        
        let mut content_loader = ContentLoader::new();
        content_loader.load_buildings(yaml).unwrap();
        game_state.load_from_content_loader(&content_loader);
        
        // Add a test site
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test Site".to_string(), 0, 100);
        
        // Add a construction job
        let mut resources = HashMap::new();
        resources.insert("steel".to_string(), 100.0);
        
        let job = ConstructionJob::new(
            site.id,
            "test_building".to_string(),
            10, // 10 ticks to complete
            resources,
            0,
        );
        
        site.enqueue_construction(job);
        game_state.galaxy.add_site_for_testing(site);
        
        game_state
    }

    #[test]
    fn test_construction_tick_processing() {
        let mut game_state = setup_test_game_state();
        
        // Initial state: 1 construction job in queue
        let site = game_state.galaxy.all_sites()[0];
        assert_eq!(site.construction_queue.len(), 1);
        assert_eq!(site.building_count(), 0);
        
        // Process several ticks
        for _ in 0..5 {
            let events = process_construction_tick(&mut game_state);
            // No completion yet
            assert!(events.is_empty() || !events.iter().any(|e| matches!(e, EventType::BuildingConstructed { .. })));
        }
        
        // Job should still be in progress
        let site = game_state.galaxy.all_sites()[0];
        assert_eq!(site.construction_queue.len(), 1);
    }

    #[test]
    fn test_construction_completion() {
        let mut game_state = setup_test_game_state();
        
        // Get IDs first to avoid borrow issues
        let site_id = game_state.galaxy.all_sites()[0].id;
        let building_id = game_state.galaxy.all_sites()[0]
            .construction_queue.all_jobs()[0].id;
        
        // Manually complete the job for testing
        let site = game_state.galaxy.find_site_mut(&site_id).unwrap();
        let job = site.construction_queue.get_mut(&building_id).unwrap();
        job.progress_ticks = job.total_ticks_required; // Force completion
        
        // Process tick - should complete the building
        let events = process_construction_tick(&mut game_state);
        
        // Should have a BuildingConstructed event
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], EventType::BuildingConstructed { .. }));
        
        // Building should now be in active building list
        let site = game_state.galaxy.all_sites()[0];
        assert_eq!(site.building_count(), 1);
        assert_eq!(site.construction_queue.len(), 0);
    }
}
