//! Building Instance — V5 data-driven building system.
//!
//! Buildings are now data-driven: instances reference BuildingDefinition by ID.
//! All building stats (costs, power, recipes) come from ContentLoader, not hardcoded enums.

use crate::domain::ids::{BuildingId, SiteId};
use serde::{Deserialize, Serialize};

/// Current operational state of a building instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingState {
    /// Under construction with progress percentage (0-100).
    UnderConstruction { progress: u8 },
    /// Fully operational and producing/consuming.
    Operational,
    /// Temporarily paused by player.
    Paused,
    /// Damaged and operating at reduced efficiency.
    Damaged { severity: u8 },
    /// Destroyed and cannot operate (can be repaired or demolished).
    Destroyed,
}

impl BuildingState {
    pub fn is_operational(&self) -> bool {
        matches!(self, BuildingState::Operational)
    }

    pub fn is_under_construction(&self) -> bool {
        matches!(self, BuildingState::UnderConstruction { .. })
    }

    pub fn can_operate(&self) -> bool {
        matches!(
            self,
            BuildingState::Operational | BuildingState::Damaged { .. }
        )
    }
}

/// A building instance at a specific site.
///
/// This is a lightweight reference to a BuildingDefinition.
/// All static data (costs, recipes, power) comes from the definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingInstance {
    /// Unique identifier for this building instance.
    pub id: BuildingId,
    
    /// Site where this building is located.
    pub site_id: SiteId,
    
    /// Reference to BuildingDefinition by ID (e.g., "solar_array_mk1").
    pub building_def_id: String,
    
    /// Current operational state.
    pub state: BuildingState,
    
    /// Number of workers currently assigned.
    pub workers_assigned: u32,
    
    /// Upgrade level (1 = base, 2+ = upgraded).
    pub level: u8,
    
    /// Selected recipe index (for buildings with multiple recipes).
    pub active_recipe_index: usize,
    
    /// Health percentage (0-100). Affects efficiency.
    pub health: u8,
    
    /// Tick when construction started (for UnderConstruction state).
    pub construction_started_at: Option<u64>,
    
    /// Tick when last maintained (for maintenance scheduling).
    pub last_maintained_at: u64,
}

impl BuildingInstance {
    /// Create a new building instance in construction state.
    pub fn new(
        site_id: SiteId,
        building_def_id: String,
        current_tick: u64,
    ) -> Self {
        Self {
            id: BuildingId::new(),
            site_id,
            building_def_id,
            state: BuildingState::UnderConstruction { progress: 0 },
            workers_assigned: 0,
            level: 1,
            active_recipe_index: 0,
            health: 100,
            construction_started_at: Some(current_tick),
            last_maintained_at: current_tick,
        }
    }

    /// Create a new building instance that's already operational.
    /// Used for testing or scenario setup.
    pub fn new_operational(
        site_id: SiteId,
        building_def_id: String,
        current_tick: u64,
    ) -> Self {
        Self {
            id: BuildingId::new(),
            site_id,
            building_def_id,
            state: BuildingState::Operational,
            workers_assigned: 0,
            level: 1,
            active_recipe_index: 0,
            health: 100,
            construction_started_at: None,
            last_maintained_at: current_tick,
        }
    }

    /// Complete construction and make operational.
    pub fn complete_construction(& mut self) {
        self.state = BuildingState::Operational;
        self.construction_started_at = None;
    }

    /// Pause this building.
    pub fn pause(&mut self) {
        if self.state.can_operate() {
            self.state = BuildingState::Paused;
        }
    }

    /// Resume this building.
    pub fn resume(&mut self) {
        if matches!(self.state, BuildingState::Paused) {
            self.state = BuildingState::Operational;
        }
    }

    /// Toggle pause state.
    pub fn toggle_pause(&mut self) {
        match self.state {
            BuildingState::Operational => self.pause(),
            BuildingState::Paused => self.resume(),
            _ => {}
        }
    }

    /// Apply damage to this building.
    pub fn damage(&mut self, amount: u8) {
        self.health = self.health.saturating_sub(amount);
        
        if self.health == 0 {
            self.state = BuildingState::Destroyed;
        } else if self.state.can_operate() && self.health < 100 {
            let severity = ((100 - self.health) as f64 / 10.0).ceil() as u8;
            self.state = BuildingState::Damaged { severity: severity.min(10) };
        }
    }

    /// Repair this building.
    pub fn repair(&mut self, amount: u8) {
        self.health = (self.health + amount).min(100);
        
        if self.health == 100 {
            self.state = BuildingState::Operational;
        } else if self.health > 0 {
            let severity = ((100 - self.health) as f64 / 10.0).ceil() as u8;
            self.state = BuildingState::Damaged { severity: severity.min(10) };
        }
    }

    /// Get efficiency multiplier based on health and state (0.0 to 1.0).
    pub fn efficiency(&self) -> f64 {
        match self.state {
            BuildingState::UnderConstruction { .. } => 0.0,
            BuildingState::Paused => 0.0,
            BuildingState::Destroyed => 0.0,
            BuildingState::Operational => (self.health as f64) / 100.0,
            BuildingState::Damaged { severity } => {
                let health_factor = (self.health as f64) / 100.0;
                let damage_factor = 1.0 - (severity as f64 * 0.05); // 5% per severity level
                health_factor * damage_factor.max(0.1) // Minimum 10% efficiency
            }
        }
    }

    /// Assign workers to this building.
    pub fn assign_workers(&mut self, count: u32, max_capacity: u32) {
        self.workers_assigned = count.min(max_capacity);
    }

    /// Calculate labor efficiency (based on workers assigned vs capacity).
    pub fn labor_efficiency(&self, worker_capacity: u32) -> f64 {
        if worker_capacity == 0 {
            return 1.0; // Buildings with no workers always at 100%
        }
        (self.workers_assigned as f64 / worker_capacity as f64).min(1.0)
    }

    /// Get overall efficiency (health * labor * state).
    pub fn overall_efficiency(&self, worker_capacity: u32) -> f64 {
        self.efficiency() * self.labor_efficiency(worker_capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_building_starts_under_construction() {
        let building = BuildingInstance::new(
            SiteId::new(),
            "solar_array_mk1".to_string(),
            0,
        );
        
        assert!(matches!(building.state, BuildingState::UnderConstruction { progress: 0 }));
        assert_eq!(building.level, 1);
        assert_eq!(building.health, 100);
    }

    #[test]
    fn test_complete_construction() {
        let mut building = BuildingInstance::new(
            SiteId::new(),
            "solar_array_mk1".to_string(),
            0,
        );
        
        building.complete_construction();
        assert_eq!(building.state, BuildingState::Operational);
        assert!(building.construction_started_at.is_none());
    }

    #[test]
    fn test_pause_resume() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "iron_mine".to_string(),
            0,
        );
        
        building.pause();
        assert_eq!(building.state, BuildingState::Paused);
        
        building.resume();
        assert_eq!(building.state, BuildingState::Operational);
    }

    #[test]
    fn test_damage_and_repair() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "smelter".to_string(),
            0,
        );
        
        // Apply moderate damage
        building.damage(30);
        assert_eq!(building.health, 70);
        assert!(matches!(building.state, BuildingState::Damaged { .. }));
        
        // Apply severe damage
        building.damage(70);
        assert_eq!(building.health, 0);
        assert_eq!(building.state, BuildingState::Destroyed);
        
        // Repair partially
        building.repair(60);
        assert_eq!(building.health, 60);
        assert!(matches!(building.state, BuildingState::Damaged { .. }));
        
        // Repair fully
        building.repair(40);
        assert_eq!(building.health, 100);
        assert_eq!(building.state, BuildingState::Operational);
    }

    #[test]
    fn test_efficiency_calculation() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "basic_habitat".to_string(),
            0,
        );
        
        // Full health, operational
        assert_eq!(building.efficiency(), 1.0);
        
        // Damaged
        building.damage(50);
        building.state = BuildingState::Damaged { severity: 5 };
        assert!(building.efficiency() < 1.0);
        assert!(building.efficiency() >= 0.1); // Minimum 10%
        
        // Paused
        building.state = BuildingState::Paused;
        assert_eq!(building.efficiency(), 0.0);
    }

    #[test]
    fn test_labor_efficiency() {
        let mut building = BuildingInstance::new_operational(
            SiteId::new(),
            "iron_mine".to_string(),
            0,
        );
        
        // No workers assigned, capacity 10
        assert_eq!(building.labor_efficiency(10), 0.0);
        
        // 5 workers assigned, capacity 10
        building.assign_workers(5, 10);
        assert_eq!(building.labor_efficiency(10), 0.5);
        
        // 10 workers assigned, capacity 10
        building.assign_workers(10, 10);
        assert_eq!(building.labor_efficiency(10), 1.0);
        
        // Over-assign (capped at capacity)
        building.assign_workers(15, 10);
        assert_eq!(building.workers_assigned, 10);
        assert_eq!(building.labor_efficiency(10), 1.0);
    }

    #[test]
    fn test_building_with_no_workers() {
        let building = BuildingInstance::new_operational(
            SiteId::new(),
            "solar_array_mk1".to_string(),
            0,
        );
        
        // Buildings with 0 worker capacity should always have 100% labor efficiency
        assert_eq!(building.labor_efficiency(0), 1.0);
    }
}
