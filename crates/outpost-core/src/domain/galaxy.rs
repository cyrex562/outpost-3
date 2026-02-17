//! Galaxy — root container for all star systems.
//!
//! The galaxy is the top-level entity in the game hierarchy.
//! It contains all star systems accessible to the player.

use crate::domain::{GalaxyId, StarSystem, StarSystemId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The galaxy containing all star systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Galaxy {
    /// Unique identifier.
    pub id: GalaxyId,
    /// Galaxy name (e.g., "Milky Way", "Andromeda").
    pub name: String,
    /// Procedural seed for galaxy generation.
    pub seed: u64,
    /// All star systems in the galaxy.
    pub systems: HashMap<StarSystemId, StarSystem>,
}

impl Galaxy {
    /// Create a new galaxy.
    pub fn new(name: String, seed: u64) -> Self {
        Self {
            id: GalaxyId::new(),
            name,
            seed,
            systems: HashMap::new(),
        }
    }

    /// Add a star system to the galaxy.
    pub fn add_system(&mut self, system: StarSystem) -> StarSystemId {
        let id = system.id;
        self.systems.insert(id, system);
        id
    }

    /// Get a star system by ID.
    pub fn get_system(&self, id: &StarSystemId) -> Option<&StarSystem> {
        self.systems.get(id)
    }

    /// Get a mutable star system by ID.
    pub fn get_system_mut(&mut self, id: &StarSystemId) -> Option<&mut StarSystem> {
        self.systems.get_mut(id)
    }

    /// Remove a star system from the galaxy.
    pub fn remove_system(&mut self, id: &StarSystemId) -> Option<StarSystem> {
        self.systems.remove(id)
    }

    /// Get all star systems.
    pub fn all_systems(&self) -> impl Iterator<Item = &StarSystem> {
        self.systems.values()
    }

    /// Get all explored star systems.
    pub fn explored_systems(&self) -> impl Iterator<Item = &StarSystem> {
        self.systems.values().filter(|s| s.explored)
    }

    /// Get all unexplored star systems.
    pub fn unexplored_systems(&self) -> impl Iterator<Item = &StarSystem> {
        self.systems.values().filter(|s| !s.explored)
    }

    /// Count star systems.
    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    /// Count explored systems.
    pub fn explored_count(&self) -> usize {
        self.explored_systems().count()
    }

    /// Count unexplored systems.
    pub fn unexplored_count(&self) -> usize {
        self.unexplored_systems().count()
    }

    /// Calculate total population across all systems.
    pub fn total_population(&self) -> u64 {
        self.systems.values().map(|s| s.total_population()).sum()
    }

    /// Calculate total site count across all systems.
    pub fn total_sites(&self) -> usize {
        self.systems.values().map(|s| s.site_count()).sum()
    }

    /// Calculate total celestial body count across all systems.
    pub fn total_bodies(&self) -> usize {
        self.systems.values().map(|s| s.body_count()).sum()
    }
    
    /// Find a site by ID across all systems.
    pub fn find_site(&self, site_id: &crate::domain::SiteId) -> Option<&crate::domain::Site> {
        for system in self.systems.values() {
            if let Some(site) = system.get_site(site_id) {
                return Some(site);
            }
        }
        None
    }
    
    /// Find a mutable site by ID across all systems.
    pub fn find_site_mut(&mut self, site_id: &crate::domain::SiteId) -> Option<&mut crate::domain::Site> {
        for system in self.systems.values_mut() {
            if let Some(site) = system.get_site_mut(site_id) {
                return Some(site);
            }
        }
        None
    }
    
    /// Get all sites across all systems.
    pub fn all_sites(&self) -> Vec<&crate::domain::Site> {
        self.systems
            .values()
            .flat_map(|system| system.all_sites())
            .collect()
    }
    
    /// For testing: add a site directly (bypassing normal add_site_to_body flow).
    #[cfg(test)]
    pub fn add_site_for_testing(&mut self, site: crate::domain::Site) {
        // Find or create a default system
        let system_id = if let Some(system_id) = self.systems.keys().next().copied() {
            system_id
        } else {
            let system = StarSystem::new(self.id, "Test System".to_string(), 0);
            self.add_system(system)
        };
        
        let system = self.systems.get_mut(&system_id).unwrap();
        system.add_site(site);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_galaxy() {
        let galaxy = Galaxy::new("Milky Way".to_string(), 12345);

        assert_eq!(galaxy.name, "Milky Way");
        assert_eq!(galaxy.seed, 12345);
        assert_eq!(galaxy.system_count(), 0);
    }

    #[test]
    fn test_add_and_get_system() {
        let mut galaxy = Galaxy::new("Test Galaxy".to_string(), 999);
        let system = StarSystem::new(galaxy.id, "Sol".to_string(), 42);

        let system_id = galaxy.add_system(system);
        assert_eq!(galaxy.system_count(), 1);

        let retrieved = galaxy.get_system(&system_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Sol");
    }

    #[test]
    fn test_remove_system() {
        let mut galaxy = Galaxy::new("Test Galaxy".to_string(), 999);
        let system = StarSystem::new(galaxy.id, "Alpha Centauri".to_string(), 100);

        let system_id = galaxy.add_system(system);
        assert_eq!(galaxy.system_count(), 1);

        let removed = galaxy.remove_system(&system_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Alpha Centauri");
        assert_eq!(galaxy.system_count(), 0);
    }

    #[test]
    fn test_explored_vs_unexplored() {
        let mut galaxy = Galaxy::new("Test".to_string(), 1);

        let mut system1 = StarSystem::new(galaxy.id, "System A".to_string(), 1);
        system1.mark_explored();

        let system2 = StarSystem::new(galaxy.id, "System B".to_string(), 2);
        let system3 = StarSystem::new(galaxy.id, "System C".to_string(), 3);

        galaxy.add_system(system1);
        galaxy.add_system(system2);
        galaxy.add_system(system3);

        assert_eq!(galaxy.system_count(), 3);
        assert_eq!(galaxy.explored_count(), 1);
        assert_eq!(galaxy.unexplored_count(), 2);
    }

    #[test]
    fn test_total_statistics() {
        use crate::domain::{
            Atmosphere, BodyType, CelestialBody, CelestialBodyId, Site, Temperature,
        };
        use std::collections::HashMap;

        let mut galaxy = Galaxy::new("Test".to_string(), 1);

        // Create system 1 with bodies and sites
        let mut system1 = StarSystem::new(galaxy.id, "System 1".to_string(), 1);
        
        let body1 = CelestialBody::new(
            system1.id,
            "Planet A".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::None,
            Temperature::Cold,
            1.0,
            5000.0,
            0.8,
        );
        let body1_id = system1.add_body(body1);

        system1.add_site(Site::new_settlement(
            body1_id,
            "Settlement 1".to_string(),
            0,
            100,
        ));
        system1.add_site(Site::new_settlement(
            body1_id,
            "Settlement 2".to_string(),
            10,
            200,
        ));

        // Create system 2 with bodies and sites
        let mut system2 = StarSystem::new(galaxy.id, "System 2".to_string(), 2);
        
        let body2 = CelestialBody::new(
            system2.id,
            "Planet B".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Breathable { composition: HashMap::new() },
            Temperature::Temperate,
            1.0,
            6000.0,
            1.0,
        );
        let body2_id = system2.add_body(body2);

        system2.add_site(Site::new_installation(
            body2_id,
            "Mine".to_string(),
            20,
            50,
        ));

        galaxy.add_system(system1);
        galaxy.add_system(system2);

        assert_eq!(galaxy.total_sites(), 3);
        assert_eq!(galaxy.total_bodies(), 2);
        assert_eq!(galaxy.total_population(), 350);
    }

    #[test]
    fn test_all_systems_iterator() {
        let mut galaxy = Galaxy::new("Test".to_string(), 1);

        galaxy.add_system(StarSystem::new(galaxy.id, "System A".to_string(), 1));
        galaxy.add_system(StarSystem::new(galaxy.id, "System B".to_string(), 2));
        galaxy.add_system(StarSystem::new(galaxy.id, "System C".to_string(), 3));

        let names: Vec<_> = galaxy.all_systems().map(|s| &s.name).collect();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&&"System A".to_string()));
        assert!(names.contains(&&"System B".to_string()));
        assert!(names.contains(&&"System C".to_string()));
    }
}
