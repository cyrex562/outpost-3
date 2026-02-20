//! Labor commands — AssignLabor and DeallocateLabor.
//!
//! These commands allow players to assign workers to building slots,
//! affecting building efficiency via the labor efficiency formula:
//!   efficiency = min(assigned / required, 1.0) * morale_modifier

use crate::domain::{BuildingId, SiteId, SkillCategory};
use crate::domain::game_state::GameState;
use serde::{Deserialize, Serialize};

/// Error types for labor assignment commands.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum LaborError {
    #[error("Site {0} not found")]
    SiteNotFound(String),
    #[error("Building {0} not found")]
    BuildingNotFound(String),
    #[error("Not enough workers of skill {skill:?} available (need {needed}, have {available})")]
    InsufficientWorkers {
        skill: SkillCategory,
        needed: u32,
        available: u32,
    },
    #[error("Cannot assign more than building capacity ({capacity} slots)")]
    ExceedsCapacity { capacity: u32 },
    #[error("Building is not operational")]
    BuildingNotOperational,
    #[error("Cannot deallocate more workers than assigned ({assigned} assigned, tried to remove {requested})")]
    TooManyDeallocated { assigned: u32, requested: u32 },
}

/// Assign a number of skilled workers to a building.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignLabor {
    pub site_id: SiteId,
    pub building_id: BuildingId,
    /// Skill type of workers to assign.
    pub skill: SkillCategory,
    /// Number of workers to assign.
    pub count: u32,
}

impl AssignLabor {
    pub fn execute(&self, state: &mut GameState) -> Result<(), LaborError> {
        // Find the site
        let site = state.galaxy.find_site_mut(&self.site_id)
            .ok_or_else(|| LaborError::SiteNotFound(self.site_id.to_string()))?;

        // Validate building exists and is operational
        let building = site.buildings.get(&self.building_id)
            .ok_or_else(|| LaborError::BuildingNotFound(self.building_id.to_string()))?;

        if !building.state.can_operate() {
            return Err(LaborError::BuildingNotOperational);
        }

        // Check building's worker capacity from definition
        let building_def_id = building.building_def_id.clone();
        let current_assigned = building.workers_assigned;
        let worker_capacity = state.building_definitions
            .get(&building_def_id)
            .map(|def| def.worker_capacity)
            .unwrap_or(0);

        if current_assigned + self.count > worker_capacity {
            return Err(LaborError::ExceedsCapacity { capacity: worker_capacity });
        }

        // Check available skilled workers
        let available = site.population.available_by_skill(self.skill);
        if available < self.count {
            return Err(LaborError::InsufficientWorkers {
                skill: self.skill,
                needed: self.count,
                available,
            });
        }

        // Allocate workers from population pool
        site.population.allocate_skilled_workers(self.skill, self.count);

        // Assign to building
        let building = site.buildings.get_mut(&self.building_id).unwrap();
        building.workers_assigned += self.count;

        Ok(())
    }
}

/// Remove workers from a building and return them to the available pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeallocateLabor {
    pub site_id: SiteId,
    pub building_id: BuildingId,
    /// Skill type of workers to return.
    pub skill: SkillCategory,
    /// Number of workers to remove.
    pub count: u32,
}

impl DeallocateLabor {
    pub fn execute(&self, state: &mut GameState) -> Result<(), LaborError> {
        // Find the site
        let site = state.galaxy.find_site_mut(&self.site_id)
            .ok_or_else(|| LaborError::SiteNotFound(self.site_id.to_string()))?;

        // Validate building exists
        let building = site.buildings.get(&self.building_id)
            .ok_or_else(|| LaborError::BuildingNotFound(self.building_id.to_string()))?;

        let current_assigned = building.workers_assigned;
        if self.count > current_assigned {
            return Err(LaborError::TooManyDeallocated {
                assigned: current_assigned,
                requested: self.count,
            });
        }

        // Remove workers from building
        let building = site.buildings.get_mut(&self.building_id).unwrap();
        building.workers_assigned -= self.count;

        // Return workers to population pool
        site.population.deallocate_skilled_workers(self.skill, self.count);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SkillCategory;

    #[test]
    fn test_assign_labor_insufficient_workers() {
        // Tests the domain logic via Population directly
        let mut pop = crate::domain::Population::new(100);
        let skill = SkillCategory::Engineer;
        let available = pop.available_by_skill(skill);

        // Try to allocate way more than available
        let result = pop.allocate_skilled_workers(skill, available + 100);
        assert!(!result, "Should fail when not enough workers");
    }

    #[test]
    fn test_assign_and_deallocate_labor() {
        let mut pop = crate::domain::Population::new(100);
        let skill = SkillCategory::Laborer;

        let initial = pop.available_by_skill(skill);
        assert!(initial > 0);

        // Assign 5 workers
        assert!(pop.allocate_skilled_workers(skill, 5));
        assert_eq!(pop.available_by_skill(skill), initial - 5);
        assert_eq!(pop.employed, 5);

        // Return them
        pop.deallocate_skilled_workers(skill, 5);
        assert_eq!(pop.available_by_skill(skill), initial);
        assert_eq!(pop.employed, 0);
    }

    #[test]
    fn test_labor_error_insufficient_workers() {
        let err = LaborError::InsufficientWorkers {
            skill: SkillCategory::Engineer,
            needed: 10,
            available: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("Engineer"));
        assert!(msg.contains("10"));
        assert!(msg.contains("3"));
    }
}

