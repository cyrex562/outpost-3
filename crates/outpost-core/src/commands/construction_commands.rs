//! V5 Construction commands — building construction operations.
//!
//! Commands for starting, cancelling, pausing, and managing building construction.

use thiserror::Error;
use std::collections::HashMap;

use crate::domain::{
    BuildingId, ConstructionJob, GameState, Site, SiteId
};
use crate::events::EventType;

#[derive(Error, Debug)]
pub enum ConstructionError {
    #[error("Site {0} not found")]
    SiteNotFound(SiteId),

    #[error("Building definition '{0}' not found")]
    BuildingDefinitionNotFound(String),

    #[error("Insufficient resource: {resource} (need {needed}, have {available})")]
    InsufficientResource {
        resource: String,
        needed: f64,
        available: f64,
    },

    #[error("Construction job for building {0} not found")]
    ConstructionJobNotFound(BuildingId),

    #[error("Construction job for building {0} is already complete")]
    ConstructionAlreadyComplete(BuildingId),

    #[error("Invalid command: {0}")]
    InvalidCommand(String),
}

/// Command to start construction of a building at a site.
pub struct StartConstruction {
    pub site_id: SiteId,
    pub building_def_id: String,
}

impl StartConstruction {
    /// Create a new StartConstruction command.
    pub fn new(site_id: SiteId, building_def_id: String) -> Self {
        Self {
            site_id,
            building_def_id,
        }
    }

    /// Execute the command, validating and creating a construction job.
    pub fn execute(&self, game_state: &mut GameState) -> Result<Vec<EventType>, ConstructionError> {
        // Get the building definition
        let building_def = game_state
            .building_definitions
            .get(&self.building_def_id)
            .ok_or_else(|| ConstructionError::BuildingDefinitionNotFound(self.building_def_id.clone()))?;

        // Get the site
        let site = game_state
            .galaxy
            .find_site_mut(&self.site_id)
            .ok_or(ConstructionError::SiteNotFound(self.site_id))?;

        // Validate resource availability
        let required_resources = self.extract_construction_costs(building_def);
        self.validate_resources(site, &required_resources)?;

        // Deduct resources
        for (resource_id, amount) in &required_resources {
            let current = site.resources.get_resource(resource_id).unwrap_or(0);
            site.resources.set_resource(resource_id, current - (*amount as i64));
        }

        // Create construction job
        let job = ConstructionJob::new(
            self.site_id,
            self.building_def_id.clone(),
            building_def.construction_time_ticks,
            required_resources.clone(),
            game_state.clock.current_tick,
        );

        let building_id = job.id;

        // Add job to construction queue
        site.enqueue_construction(job);

        // Emit event
        Ok(vec![EventType::ConstructionQueued {
            site_id: self.site_id,
            building_id,
            building_def_id: self.building_def_id.clone(),
            tick: game_state.clock.current_tick,
        }])
    }

    /// Extract construction costs from building definition.
    fn extract_construction_costs(&self, building_def: &crate::content::BuildingDefinition) -> HashMap<String, f64> {
        building_def
            .construction_costs
            .iter()
            .map(|cost| (cost.resource_id.clone(), cost.amount))
            .collect()
    }

    /// Validate that site has sufficient resources.
    fn validate_resources(&self, site: &Site, required: &HashMap<String, f64>) -> Result<(), ConstructionError> {
        for (resource_id, needed) in required {
            let available = site.resources.get_resource(resource_id).unwrap_or(0) as f64;
            if available < *needed {
                return Err(ConstructionError::InsufficientResource {
                    resource: resource_id.clone(),
                    needed: *needed,
                    available,
                });
            }
        }
        Ok(())
    }
}

/// Command to cancel an in-progress construction job.
pub struct CancelConstruction {
    pub site_id: SiteId,
    pub building_id: BuildingId,
    pub waste_percentage: f64, // How much of committed resources are lost (0.0-1.0)
}

impl CancelConstruction {
    /// Create a new CancelConstruction command.
    pub fn new(site_id: SiteId, building_id: BuildingId) -> Self {
        Self {
            site_id,
            building_id,
            waste_percentage: 0.2, // Default 20% waste
        }
    }

    /// Execute the command, removing the job and refunding resources.
    pub fn execute(&self, game_state: &mut GameState) -> Result<Vec<EventType>, ConstructionError> {
        // Get the site
        let site = game_state
            .galaxy
            .find_site_mut(&self.site_id)
            .ok_or(ConstructionError::SiteNotFound(self.site_id))?;

        // Remove the construction job
        let job = site.construction_queue
            .remove(&self.building_id)
            .ok_or(ConstructionError::ConstructionJobNotFound(self.building_id))?;

        // Calculate refund
        let refund = job.calculate_refund(self.waste_percentage);

        // Refund resources to site
        for (resource_id, amount) in &refund {
            let current = site.resources.get_resource(resource_id).unwrap_or(0);
            site.resources.set_resource(resource_id, current + (*amount as i64));
        }

        // Emit event
        Ok(vec![EventType::ConstructionCancelled {
            site_id: self.site_id,
            building_id: self.building_id,
            refunded_resources: refund,
            tick: game_state.clock.current_tick,
        }])
    }
}

/// Command to pause construction of a building.
pub struct PauseConstruction {
    pub site_id: SiteId,
    pub building_id: BuildingId,
}

impl PauseConstruction {
    /// Create a new PauseConstruction command.
    pub fn new(site_id: SiteId, building_id: BuildingId) -> Self {
        Self {
            site_id,
            building_id,
        }
    }

    /// Execute the command, pausing the construction job.
    pub fn execute(&self, game_state: &mut GameState) -> Result<Vec<EventType>, ConstructionError> {
        // Get the site
        let site = game_state
            .galaxy
            .find_site_mut(&self.site_id)
            .ok_or(ConstructionError::SiteNotFound(self.site_id))?;

        // Get the construction job
        let job = site.construction_queue
            .get_mut(&self.building_id)
            .ok_or(ConstructionError::ConstructionJobNotFound(self.building_id))?;

        if job.is_complete() {
            return Err(ConstructionError::ConstructionAlreadyComplete(self.building_id));
        }

        // Pause the job
        job.pause();

        // Emit event
        Ok(vec![EventType::ConstructionPaused {
            site_id: self.site_id,
            building_id: self.building_id,
            tick: game_state.clock.current_tick,
        }])
    }
}

/// Command to resume construction of a paused building.
pub struct ResumeConstruction {
    pub site_id: SiteId,
    pub building_id: BuildingId,
}

impl ResumeConstruction {
    /// Create a new ResumeConstruction command.
    pub fn new(site_id: SiteId, building_id: BuildingId) -> Self {
        Self {
            site_id,
            building_id,
        }
    }

    /// Execute the command, resuming the construction job.
    pub fn execute(&self, game_state: &mut GameState) -> Result<Vec<EventType>, ConstructionError> {
        // Get the site
        let site = game_state
            .galaxy
            .find_site_mut(&self.site_id)
            .ok_or(ConstructionError::SiteNotFound(self.site_id))?;

        // Get the construction job
        let job = site.construction_queue
            .get_mut(&self.building_id)
            .ok_or(ConstructionError::ConstructionJobNotFound(self.building_id))?;

        if job.is_complete() {
            return Err(ConstructionError::ConstructionAlreadyComplete(self.building_id));
        }

        // Resume the job
        job.resume();

        // Emit event
        Ok(vec![EventType::ConstructionResumed {
            site_id: self.site_id,
            building_id: self.building_id,
            tick: game_state.clock.current_tick,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CelestialBodyId, Site};
    use crate::content::ContentLoader;

    fn setup_test_game_state() -> GameState {
        let mut game_state = GameState::new("Test Game".to_string(), 12345);
        
        // Load building definitions
        let yaml = r#"
- id: test_mine
  name: Test Mine
  description: A test mine
  category: mining
  construction_costs:
    - resource_id: steel
      amount: 100
    - resource_id: electronics
      amount: 50
  construction_time_ticks: 100
  power:
    base_mw: 5.0
    per_level_mw: 2.0
  worker_capacity: 10
  recipes: []
  maintenance_cost: {}
  upgradeable: false
  morale_bonus: 0.0
"#;
        
        let mut content_loader = ContentLoader::new();
        content_loader.load_buildings(yaml).unwrap();
        game_state.load_from_content_loader(&content_loader);
        
        // Add a test site with resources
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test Site".to_string(), 0, 100);
        site.resources.set_resource("steel", 200);
        site.resources.set_resource("electronics", 100);
        
        let site_id = site.id;
        game_state.galaxy.add_site_for_testing(site);
        
        game_state
    }

    #[test]
    fn test_start_construction_success() {
        let mut game_state = setup_test_game_state();
        let site_id = game_state.galaxy.all_sites()[0].id;
        
        let cmd = StartConstruction::new(site_id, "test_mine".to_string());
        let events = cmd.execute(&mut game_state).expect("Should succeed");
        
        assert_eq!(events.len(), 1);
        match &events[0] {
            EventType::ConstructionQueued { building_def_id, .. } => {
                assert_eq!(building_def_id, "test_mine");
            }
            _ => panic!("Expected ConstructionQueued event"),
        }
        
        // Verify construction queue has the job
        let site = game_state.galaxy.find_site(&site_id).unwrap();
        assert_eq!(site.construction_queue.len(), 1);
        
        // Verify resources were deducted
        assert_eq!(site.resources.get_resource("steel"), Some(100)); // 200 - 100
        assert_eq!(site.resources.get_resource("electronics"), Some(50)); // 100 - 50
    }

    #[test]
    fn test_start_construction_insufficient_resources() {
        let mut game_state = setup_test_game_state();
        let site_id = game_state.galaxy.all_sites()[0].id;
        
        // Drain resources
        let site = game_state.galaxy.find_site_mut(&site_id).unwrap();
        site.resources.set_resource("steel", 10); // Not enough
        
        let cmd = StartConstruction::new(site_id, "test_mine".to_string());
        let result = cmd.execute(&mut game_state);
        
        assert!(result.is_err());
        match result {
            Err(ConstructionError::InsufficientResource { resource, .. }) => {
                assert_eq!(resource, "steel");
            }
            _ => panic!("Expected InsufficientResource error"),
        }
    }

    #[test]
    fn test_start_construction_invalid_building() {
        let mut game_state = setup_test_game_state();
        let site_id = game_state.galaxy.all_sites()[0].id;
        
        let cmd = StartConstruction::new(site_id, "nonexistent_building".to_string());
        let result = cmd.execute(&mut game_state);
        
        assert!(result.is_err());
        match result {
            Err(ConstructionError::BuildingDefinitionNotFound(id)) => {
                assert_eq!(id, "nonexistent_building");
            }
            _ => panic!("Expected BuildingDefinitionNotFound error"),
        }
    }

    #[test]
    fn test_cancel_construction() {
        let mut game_state = setup_test_game_state();
        let site_id = game_state.galaxy.all_sites()[0].id;
        
        // Start construction
        let cmd = StartConstruction::new(site_id, "test_mine".to_string());
        let events = cmd.execute(&mut game_state).expect("Should succeed");
        
        let building_id = match &events[0] {
            EventType::ConstructionQueued { building_id, .. } => *building_id,
            _ => panic!("Expected ConstructionQueued event"),
        };
        
        // Cancel construction
        let cancel_cmd = CancelConstruction::new(site_id, building_id);
        let cancel_events = cancel_cmd.execute(&mut game_state).expect("Should succeed");
        
        assert_eq!(cancel_events.len(), 1);
        match &cancel_events[0] {
            EventType::ConstructionCancelled { refunded_resources, .. } => {
                // Should get most resources back (80% with default 20% waste)
                let steel_refund = refunded_resources.get("steel").unwrap();
                assert!(*steel_refund >= 79.0 && *steel_refund <= 81.0); // ~80
            }
            _ => panic!("Expected ConstructionCancelled event"),
        }
        
        // Verify queue is empty
        let site = game_state.galaxy.find_site(&site_id).unwrap();
        assert_eq!(site.construction_queue.len(), 0);
    }

    #[test]
    fn test_pause_and_resume_construction() {
        let mut game_state = setup_test_game_state();
        let site_id = game_state.galaxy.all_sites()[0].id;
        
        // Start construction
        let cmd = StartConstruction::new(site_id, "test_mine".to_string());
        let events = cmd.execute(&mut game_state).expect("Should succeed");
        
        let building_id = match &events[0] {
            EventType::ConstructionQueued { building_id, .. } => *building_id,
            _ => panic!("Expected ConstructionQueued event"),
        };
        
        // Pause construction
        let pause_cmd = PauseConstruction::new(site_id, building_id);
        let pause_events = pause_cmd.execute(&mut game_state).expect("Should succeed");
        
        assert_eq!(pause_events.len(), 1);
        assert!(matches!(pause_events[0], EventType::ConstructionPaused { .. }));
        
        // Verify job is paused
        let site = game_state.galaxy.find_site(&site_id).unwrap();
        let job = site.construction_queue.get(&building_id).unwrap();
        assert!(job.paused);
        
        // Resume construction
        let resume_cmd = ResumeConstruction::new(site_id, building_id);
        let resume_events = resume_cmd.execute(&mut game_state).expect("Should succeed");
        
        assert_eq!(resume_events.len(), 1);
        assert!(matches!(resume_events[0], EventType::ConstructionResumed { .. }));
        
        // Verify job is not paused
        let site = game_state.galaxy.find_site(&site_id).unwrap();
        let job = site.construction_queue.get(&building_id).unwrap();
        assert!(!job.paused);
    }

    // Property-based tests for resource conservation
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_construction_deducts_exact_cost(
            steel_cost in 1.0f64..1000.0,
            electronics_cost in 1.0f64..1000.0,
        ) {
            let mut game_state = GameState::new("Test Game".to_string(), 12345);
            
            // Create a building definition with variable costs
            let yaml = format!(r#"
- id: variable_building
  name: Variable Building
  description: A building with variable costs
  category: mining
  construction_costs:
    - resource_id: steel
      amount: {}
    - resource_id: electronics
      amount: {}
  construction_time_ticks: 100
  power:
    base_mw: 5.0
    per_level_mw: 2.0
  worker_capacity: 10
  recipes: []
  maintenance_cost: {{}}
  upgradeable: false
  morale_bonus: 0.0
"#, steel_cost, electronics_cost);
            
            let mut content_loader = ContentLoader::new();
            content_loader.load_buildings(&yaml).unwrap();
            game_state.load_from_content_loader(&content_loader);
            
            // Add a test site with ample resources
            let body_id = CelestialBodyId::new();
            let mut site = Site::new_settlement(body_id, "Test Site".to_string(), 0, 100);
            site.resources.set_resource("steel", (steel_cost * 2.0) as i64);
            site.resources.set_resource("electronics", (electronics_cost * 2.0) as i64);
            
            let initial_steel = site.resources.get_resource("steel").unwrap();
            let initial_electronics = site.resources.get_resource("electronics").unwrap();
            
            let site_id = site.id;
            game_state.galaxy.add_site_for_testing(site);
            
            // Start construction
            let cmd = StartConstruction::new(site_id, "variable_building".to_string());
            let result = cmd.execute(&mut game_state);
            
            prop_assert!(result.is_ok(), "Construction should succeed with sufficient resources");
            
            // Verify resources were deducted exactly
            let site = game_state.galaxy.find_site(&site_id).unwrap();
            let final_steel = site.resources.get_resource("steel").unwrap();
            let final_electronics = site.resources.get_resource("electronics").unwrap();
            
            prop_assert_eq!(initial_steel - final_steel, steel_cost as i64, "Steel deduction must match cost");
            prop_assert_eq!(initial_electronics - final_electronics, electronics_cost as i64, "Electronics deduction must match cost");
        }

        #[test]
        fn prop_cancellation_refunds_proportional_resources(
            progress_percent in 0u8..100,
            waste_pct in 0.0f64..0.5,
        ) {
            let mut game_state = setup_test_game_state();
            let site_id = game_state.galaxy.all_sites()[0].id;
            
            // Start construction
            let cmd = StartConstruction::new(site_id, "test_mine".to_string());
            let events = cmd.execute(&mut game_state).unwrap();
            
            let building_id = match &events[0] {
                EventType::ConstructionQueued { building_id, .. } => *building_id,
                _ => panic!("Expected ConstructionQueued event"),
            };
            
            // Advance progress to specific percentage
            let site = game_state.galaxy.find_site_mut(&site_id).unwrap();
            let job = site.construction_queue.get_mut(&building_id).unwrap();
            let total_ticks = job.total_ticks_required;
            job.progress_ticks = (total_ticks * progress_percent as u64) / 100;
            
            // Cancel construction
            let mut cancel_cmd = CancelConstruction::new(site_id, building_id);
            cancel_cmd.waste_percentage = waste_pct;
            let cancel_events = cancel_cmd.execute(&mut game_state).unwrap();
            
            // Verify refund calculation
            match &cancel_events[0] {
                EventType::ConstructionCancelled { refunded_resources, .. } => {
                    let steel_refund = refunded_resources.get("steel").unwrap();
                    
                    // Expected: (work_remaining / total_work) * (1 - waste) * original_cost
                    let work_remaining_ratio = (100 - progress_percent) as f64 / 100.0;
                    let expected_refund = work_remaining_ratio * (1.0 - waste_pct) * 100.0;
                    
                    // Allow 1% tolerance for rounding
                    let tolerance = expected_refund * 0.01 + 0.1;
                    prop_assert!(
                        (*steel_refund - expected_refund).abs() < tolerance,
                        "Refund {} should be close to expected {}",
                        steel_refund,
                        expected_refund
                    );
                }
                _ => panic!("Expected ConstructionCancelled event"),
            }
        }

        #[test]
        fn prop_construction_never_overcharges(
            excess_multiplier in 1.0f64..10.0,
        ) {
            let mut game_state = setup_test_game_state();
            let site_id = game_state.galaxy.all_sites()[0].id;
            
            // Get the site and record initial resources
            let site = game_state.galaxy.find_site(&site_id).unwrap();
            let initial_steel = site.resources.get_resource("steel").unwrap();
            
            // Start construction (cost is 100 steel)
            let cmd = StartConstruction::new(site_id, "test_mine".to_string());
            let result = cmd.execute(&mut game_state);
            
            prop_assert!(result.is_ok());
            
            // Verify we didn't deduct more than the cost
            let site = game_state.galaxy.find_site(&site_id).unwrap();
            let final_steel = site.resources.get_resource("steel").unwrap();
            let deducted = initial_steel - final_steel;
            
            prop_assert_eq!(deducted, 100, "Should deduct exactly the construction cost");
            prop_assert!(deducted <= initial_steel, "Cannot deduct more than available");
        }
    }
}
