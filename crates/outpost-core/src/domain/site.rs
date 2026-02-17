//! Site — settlements and installations on celestial bodies.
//!
//! Sites are the primary locations where colonists live and work.
//! This entity replaces the pre-V5 Colony concept with more granular types.

use crate::domain::{
    BuildingInstance, CelestialBodyId, ConstructionQueue, Population, PowerGrid, Resources, SiteId,
};
use crate::domain::ids::BuildingId; // V5 BuildingId with UUID
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of site: settlement or installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SiteType {
    /// Permanent settlement with full population and building set.
    Settlement,
    /// Specialized installation with crew (mines, refineries, military, research).
    Installation,
}

/// A site on a celestial body (settlement or installation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    /// Unique identifier.
    pub id: SiteId,
    /// Parent celestial body.
    pub body_id: CelestialBodyId,
    /// Site name (e.g., "Landing Site Prime", "Mining Installation #3").
    pub name: String,
    /// Type of site.
    pub site_type: SiteType,
    /// Timestamp when site was founded (tick count, not wall-clock time).
    pub founded_at_tick: u64,
    /// Buildings present at this site (V5: full BuildingInstance with state).
    pub buildings: HashMap<BuildingId, BuildingInstance>,
    /// Construction queue for buildings being built.
    pub construction_queue: ConstructionQueue,
    /// Resource stockpile.
    pub resources: Resources,
    /// Population (settlers for settlements, crew for installations).
    pub population: Population,
    /// Power generation and distribution.
    pub power_grid: PowerGrid,
    /// Pollution level (0.0 = pristine, 1.0+ = heavily polluted).
    pub pollution_level: f64,
    /// Morale modifier specific to this site (-1.0 to 1.0).
    /// Combines with colony-wide morale factors.
    pub local_morale_modifier: f64,
}

impl Site {
    /// Create a new settlement.
    pub fn new_settlement(
        body_id: CelestialBodyId,
        name: String,
        founded_at_tick: u64,
        initial_population: u32,
    ) -> Self {
        Self {
            id: SiteId::new(),
            body_id,
            name,
            site_type: SiteType::Settlement,
            founded_at_tick,
            buildings: HashMap::new(),
            construction_queue: ConstructionQueue::new(),
            resources: Resources::starting_resources(),
            population: Population::new(initial_population as u64),
            power_grid: PowerGrid::new(),
            pollution_level: 0.0,
            local_morale_modifier: 0.0,
        }
    }

    /// Create a new installation.
    pub fn new_installation(
        body_id: CelestialBodyId,
        name: String,
        founded_at_tick: u64,
        crew_size: u32,
    ) -> Self {
        Self {
            id: SiteId::new(),
            body_id,
            name,
            site_type: SiteType::Installation,
            founded_at_tick,
            buildings: HashMap::new(),
            construction_queue: ConstructionQueue::new(),
            resources: Resources::starting_resources(),
            population: Population::new(crew_size as u64),
            power_grid: PowerGrid::new(),
            pollution_level: 0.0,
            local_morale_modifier: 0.0,
        }
    }

    /// Add a building instance to this site.
    pub fn add_building(&mut self, building: BuildingInstance) {
        self.buildings.insert(building.id, building);
    }

    /// Remove a building from this site.
    pub fn remove_building(&mut self, building_id: &BuildingId) -> Option<BuildingInstance> {
        self.buildings.remove(building_id)
    }

    /// Check if a building exists at this site.
    pub fn has_building(&self, building_id: &BuildingId) -> bool {
        self.buildings.contains_key(building_id)
    }
    
    /// Get a building instance by ID.
    pub fn get_building(&self, building_id: &BuildingId) -> Option<&BuildingInstance> {
        self.buildings.get(building_id)
    }
    
    /// Get a mutable building instance by ID.
    pub fn get_building_mut(&mut self, building_id: &BuildingId) -> Option<&mut BuildingInstance> {
        self.buildings.get_mut(building_id)
    }

    /// Get the number of buildings at this site.
    pub fn building_count(&self) -> usize {
        self.buildings.len()
    }
    
    /// Get all building instances at this site.
    pub fn all_buildings(&self) -> impl Iterator<Item = &BuildingInstance> {
        self.buildings.values()
    }
    
    /// Get all operational buildings at this site.
    pub fn operational_buildings(&self) -> impl Iterator<Item = &BuildingInstance> {
        self.buildings.values().filter(|b| b.state.is_operational())
    }

    /// Add pollution to the site.
    pub fn add_pollution(&mut self, amount: f64) {
        self.pollution_level = (self.pollution_level + amount).max(0.0);
    }

    /// Reduce pollution (e.g., via environmental cleanup).
    pub fn reduce_pollution(&mut self, amount: f64) {
        self.pollution_level = (self.pollution_level - amount).max(0.0);
    }

    /// Calculate the effective morale for this site.
    /// Combines base population morale with the local site modifier.
    pub fn effective_morale(&self) -> f64 {
        let base_morale = self.population.morale as f64;
        let modified = base_morale + (self.local_morale_modifier * 10.0);
        modified.clamp(0.0, 100.0)
    }

    /// Adjust the local morale modifier for this site.
    pub fn adjust_local_morale(&mut self, delta: f64) {
        self.local_morale_modifier = (self.local_morale_modifier + delta).clamp(-1.0, 1.0);
    }

    /// Returns true if this is a settlement (vs installation).
    pub fn is_settlement(&self) -> bool {
        self.site_type == SiteType::Settlement
    }

    /// Returns true if this is an installation (vs settlement).
    pub fn is_installation(&self) -> bool {
        self.site_type == SiteType::Installation
    }
    
    /// Get storage capacity for a specific resource type.
    /// Returns the maximum amount that can be stored.
    /// Default capacity is 1000 tons; warehouses and specialized storage increase this.
    pub fn get_storage_capacity(&self, _resource_id: &str) -> f64 {
        // TODO: Calculate from buildings once BuildingDefinition includes storage_capacity
        // For now, return a default capacity
        10000.0
    }
    
    /// Get resource amount by string ID (V5 interface for production system).
    /// Uses the resource mapping layer to convert string IDs to ResourceType.
    pub fn get_resource_by_id(&self, resource_id: &str) -> Result<f64, crate::domain::MappingError> {
        use crate::domain::resource_mapping::{string_to_resource_type, is_virtual_resource};
        
        // Virtual resources (power, labor) are not stored
        if is_virtual_resource(resource_id) {
            return Ok(0.0);
        }
        
        // Map string ID to ResourceType
        let resource_type = string_to_resource_type(resource_id)?;
        
        // Get from Resources (returns i64, convert to f64)
        Ok(self.resources.get(resource_type) as f64)
    }
    
    /// Set resource amount by string ID (V5 interface for production system).
    /// Uses the resource mapping layer to convert string IDs to ResourceType.
    pub fn set_resource_by_id(&mut self, resource_id: &str, amount: f64) -> Result<(), crate::domain::MappingError> {
        use crate::domain::resource_mapping::{string_to_resource_type, is_virtual_resource};
        
        // Virtual resources (power, labor) are not stored
        if is_virtual_resource(resource_id) {
            return Ok(());
        }
        
        // Map string ID to ResourceType
        let resource_type = string_to_resource_type(resource_id)?;
        
        // Set in Resources (convert f64 to i64, rounding)
        self.resources.set(resource_type, amount.round() as i64);
        
        Ok(())
    }
    
    /// Adjust resource amount by delta (add or subtract).
    /// Convenience method for production/consumption.
    pub fn adjust_resource_by_id(&mut self, resource_id: &str, delta: f64) -> Result<(), crate::domain::MappingError> {
        let current = self.get_resource_by_id(resource_id)?;
        self.set_resource_by_id(resource_id, current + delta)
    }

    /// Get the total population count.
    pub fn population_count(&self) -> u32 {
        self.population.total as u32
    }
    
    // Construction queue management methods
    
    /// Get the construction queue.
    pub fn get_construction_queue(&self) -> &ConstructionQueue {
        &self.construction_queue
    }
    
    /// Get mutable access to the construction queue.
    pub fn get_construction_queue_mut(&mut self) -> &mut ConstructionQueue {
        &mut self.construction_queue
    }
    
    /// Add a construction job to the queue.
    pub fn enqueue_construction(&mut self, job: crate::domain::ConstructionJob) {
        self.construction_queue.enqueue(job);
    }
    
    /// Get the number of jobs in the construction queue.
    pub fn construction_jobs_count(&self) -> usize {
        self.construction_queue.len()
    }
    
    /// Check if there are any construction jobs in progress.
    pub fn has_active_construction(&self) -> bool {
        !self.construction_queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_settlement() {
        let body_id = CelestialBodyId::new();
        let site = Site::new_settlement(
            body_id,
            "Landing Site Prime".to_string(),
            0,
            100,
        );

        assert_eq!(site.name, "Landing Site Prime");
        assert_eq!(site.site_type, SiteType::Settlement);
        assert!(site.is_settlement());
        assert!(!site.is_installation());
        assert_eq!(site.population_count(), 100);
        assert_eq!(site.building_count(), 0);
    }

    #[test]
    fn test_create_installation() {
        let body_id = CelestialBodyId::new();
        let site = Site::new_installation(
            body_id,
            "Mining Outpost".to_string(),
            100,
            25,
        );

        assert_eq!(site.name, "Mining Outpost");
        assert_eq!(site.site_type, SiteType::Installation);
        assert!(site.is_installation());
        assert!(!site.is_settlement());
        assert_eq!(site.population_count(), 25);
    }

    #[test]
    fn test_building_management() {
        let body_id = CelestialBodyId::new();
        let site_id = SiteId::new();
        let mut site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);

        let building = BuildingInstance::new_operational(
            site_id,
            "test_building".to_string(),
            0,
        );
        let building_id = building.id;
        assert_eq!(site.building_count(), 0);
        
        site.add_building(building);
        assert_eq!(site.building_count(), 1);
        assert!(site.has_building(&building_id));

        let removed = site.remove_building(&building_id);
        assert!(removed.is_some());
        assert_eq!(site.building_count(), 0);
        assert!(!site.has_building(&building_id));
    }

    #[test]
    fn test_pollution() {
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);

        assert_eq!(site.pollution_level, 0.0);

        site.add_pollution(0.5);
        assert!((site.pollution_level - 0.5).abs() < 1e-10);

        site.add_pollution(0.3);
        assert!((site.pollution_level - 0.8).abs() < 1e-10);

        site.reduce_pollution(0.2);
        assert!((site.pollution_level - 0.6).abs() < 1e-10);

        // Pollution can't go negative
        site.reduce_pollution(10.0);
        assert_eq!(site.pollution_level, 0.0);
    }

    #[test]
    fn test_morale_modifier() {
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);

        // Base morale (from population)
        let base_morale = site.population.morale;
        assert_eq!(site.effective_morale(), base_morale as f64);

        // Positive local modifier
        site.adjust_local_morale(0.2);
        assert_eq!(site.local_morale_modifier, 0.2);
        assert_eq!(site.effective_morale(), base_morale as f64 + 2.0);

        // Negative local modifier
        site.adjust_local_morale(-0.5);
        assert_eq!(site.local_morale_modifier, -0.3);
        assert_eq!(site.effective_morale(), base_morale as f64 - 3.0);

        // Clamping
        site.adjust_local_morale(-10.0);
        assert_eq!(site.local_morale_modifier, -1.0);

        site.adjust_local_morale(20.0);
        assert_eq!(site.local_morale_modifier, 1.0);
    }

    #[test]
    fn test_effective_morale_bounds() {
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);

        // Set population morale very low
        site.population.adjust_morale(-100.0);
        assert_eq!(site.population.morale, 0.0);

        // Even with positive modifier, effective morale shouldn't go negative
        site.adjust_local_morale(-1.0);
        let effective = site.effective_morale();
        assert!(effective >= 0.0, "Morale should not be negative");

        // Set population morale very high
        site.population.morale = 100.0;
        site.adjust_local_morale(1.0);
        let effective = site.effective_morale();
        assert!(effective <= 100.0, "Morale should not exceed 100");
    }
    
    #[test]
    fn test_resource_mapping_interface() {
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);
        
        // Starting resources include some iron ore
        let iron = site.get_resource_by_id("iron_ore").unwrap();
        assert_eq!(iron, 50.0); // From starting_resources()
        
        // Set resource using string ID
        site.set_resource_by_id("iron_ore", 100.0).unwrap();
        assert_eq!(site.get_resource_by_id("iron_ore").unwrap(), 100.0);
        
        // Adjust resource (add)
        site.adjust_resource_by_id("iron_ore", 25.0).unwrap();
        assert_eq!(site.get_resource_by_id("iron_ore").unwrap(), 125.0);
        
        // Adjust resource (subtract)
        site.adjust_resource_by_id("iron_ore", -50.0).unwrap();
        assert_eq!(site.get_resource_by_id("iron_ore").unwrap(), 75.0);
    }
    
    #[test]
    fn test_resource_alias_mapping() {
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);
        
        // "iron_ingots" is an alias for Steel in the mapping
        site.set_resource_by_id("iron_ingots", 50.0).unwrap();
        
        // Reading via different aliases should give same value
        assert_eq!(site.get_resource_by_id("iron_ingots").unwrap(), 50.0);
        assert_eq!(site.get_resource_by_id("steel").unwrap(), 50.0);
        assert_eq!(site.get_resource_by_id("iron").unwrap(), 50.0);
    }
    
    #[test]
    fn test_virtual_resources_ignored() {
        let body_id = CelestialBodyId::new();
        let mut site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);
        
        // Virtual resources (power, labor) should return 0 and not error
        assert_eq!(site.get_resource_by_id("power").unwrap(), 0.0);
        assert_eq!(site.get_resource_by_id("labor").unwrap(), 0.0);
        
        // Setting virtual resources should be a no-op
        site.set_resource_by_id("power", 1000.0).unwrap();
        assert_eq!(site.get_resource_by_id("power").unwrap(), 0.0);
    }
    
    #[test]
    fn test_unknown_resource_id_errors() {
        let body_id = CelestialBodyId::new();
        let site = Site::new_settlement(body_id, "Test".to_string(), 0, 100);
        
        // Unknown resource ID should return error
        let result = site.get_resource_by_id("unknown_resource_xyz");
        assert!(result.is_err());
    }
}
