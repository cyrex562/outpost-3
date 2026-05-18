//! StarSystem — a star with orbiting celestial bodies.
//!
//! Star systems are containers for planets, moons, asteroids, and stations.
//! Each system has a procedural seed for generating consistent content.

use crate::domain::{CelestialBody, CelestialBodyId, GalaxyId, Site, SiteId, StarSystemId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A star system containing celestial bodies and sites.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarSystem {
    /// Unique identifier.
    pub id: StarSystemId,
    /// Parent galaxy.
    pub galaxy_id: GalaxyId,
    /// System name (e.g., "Sol", "Alpha Centauri").
    pub name: String,
    /// Procedural seed for system generation.
    pub seed: u64,
    /// Celestial bodies in this system (planets, moons, asteroids, stations).
    pub bodies: HashMap<CelestialBodyId, CelestialBody>,
    /// Sites across all bodies in this system.
    pub sites: HashMap<SiteId, Site>,
    /// Whether this system has been explored by the player.
    pub explored: bool,
}

impl StarSystem {
    /// Create a new star system.
    pub fn new(galaxy_id: GalaxyId, name: String, seed: u64) -> Self {
        Self {
            id: StarSystemId::new(),
            galaxy_id,
            name,
            seed,
            bodies: HashMap::new(),
            sites: HashMap::new(),
            explored: false,
        }
    }

    /// Add a celestial body to this system.
    pub fn add_body(&mut self, body: CelestialBody) -> CelestialBodyId {
        let id = body.id;
        self.bodies.insert(id, body);
        id
    }

    /// Get a celestial body by ID.
    pub fn get_body(&self, id: &CelestialBodyId) -> Option<&CelestialBody> {
        self.bodies.get(id)
    }

    /// Get a mutable celestial body by ID.
    pub fn get_body_mut(&mut self, id: &CelestialBodyId) -> Option<&mut CelestialBody> {
        self.bodies.get_mut(id)
    }

    /// Remove a celestial body from this system.
    pub fn remove_body(&mut self, id: &CelestialBodyId) -> Option<CelestialBody> {
        self.bodies.remove(id)
    }

    /// Get all celestial bodies.
    pub fn all_bodies(&self) -> impl Iterator<Item = &CelestialBody> {
        self.bodies.values()
    }

    /// Count celestial bodies in this system.
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    /// Add a site to this system.
    pub fn add_site(&mut self, site: Site) -> SiteId {
        let id = site.id;
        self.sites.insert(id, site);
        id
    }

    /// Get a site by ID.
    pub fn get_site(&self, id: &SiteId) -> Option<&Site> {
        self.sites.get(id)
    }

    /// Get a mutable site by ID.
    pub fn get_site_mut(&mut self, id: &SiteId) -> Option<&mut Site> {
        self.sites.get_mut(id)
    }

    /// Remove a site from this system.
    pub fn remove_site(&mut self, id: &SiteId) -> Option<Site> {
        self.sites.remove(id)
    }

    /// Get all sites in this system.
    pub fn all_sites(&self) -> impl Iterator<Item = &Site> {
        self.sites.values()
    }

    /// Get all sites on a specific celestial body.
    pub fn sites_on_body<'a>(&'a self, body_id: &'a CelestialBodyId) -> impl Iterator<Item = &'a Site> + 'a {
        self.sites.values().filter(move |site| &site.body_id == body_id)
    }

    /// Count sites in this system.
    pub fn site_count(&self) -> usize {
        self.sites.len()
    }

    /// Count sites on a specific celestial body.
    pub fn site_count_on_body(&self, body_id: &CelestialBodyId) -> usize {
        self.sites_on_body(body_id).count()
    }

    /// Mark this system as explored.
    pub fn mark_explored(&mut self) {
        self.explored = true;
    }

    /// Calculate total population across all sites.
    pub fn total_population(&self) -> u64 {
        self.sites.values().map(|site| site.population_count() as u64).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Atmosphere, BodyType, CelestialBody, Temperature};

    #[test]
    fn test_create_star_system() {
        let galaxy_id = GalaxyId::new();
        let system = StarSystem::new(galaxy_id, "Sol".to_string(), 42);

        assert_eq!(system.name, "Sol");
        assert_eq!(system.seed, 42);
        assert_eq!(system.galaxy_id, galaxy_id);
        assert!(!system.explored);
        assert_eq!(system.body_count(), 0);
        assert_eq!(system.site_count(), 0);
    }

    #[test]
    fn test_add_and_get_body() {
        let galaxy_id = GalaxyId::new();
        let mut system = StarSystem::new(galaxy_id, "Test System".to_string(), 123);

        let body = CelestialBody::new(
            system.id,
            "Earth".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Breathable { composition: HashMap::new() },
            Temperature::Temperate,
            1.0,
            6371.0,
            1.0,
        );

        let body_id = system.add_body(body);
        assert_eq!(system.body_count(), 1);

        let retrieved = system.get_body(&body_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Earth");
    }

    #[test]
    fn test_remove_body() {
        let galaxy_id = GalaxyId::new();
        let mut system = StarSystem::new(galaxy_id, "Test System".to_string(), 123);

        let body = CelestialBody::new(
            system.id,
            "Mars".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Thin { composition: HashMap::new() },
            Temperature::Cold,
            0.38,
            3389.5,
            0.107,
        );

        let body_id = system.add_body(body);
        assert_eq!(system.body_count(), 1);

        let removed = system.remove_body(&body_id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Mars");
        assert_eq!(system.body_count(), 0);
    }

    #[test]
    fn test_add_and_get_site() {
        let galaxy_id = GalaxyId::new();
        let mut system = StarSystem::new(galaxy_id, "Test System".to_string(), 123);

        let body_id = CelestialBodyId::new();
        let site = Site::new_settlement(
            body_id,
            "Landing Site".to_string(),
            0,
            100,
        );

        let site_id = system.add_site(site);
        assert_eq!(system.site_count(), 1);

        let retrieved = system.get_site(&site_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Landing Site");
    }

    #[test]
    fn test_sites_on_body() {
        let galaxy_id = GalaxyId::new();
        let mut system = StarSystem::new(galaxy_id, "Test System".to_string(), 123);

        let body1_id = CelestialBodyId::new();
        let body2_id = CelestialBodyId::new();

        // Add sites on body1
        system.add_site(Site::new_settlement(body1_id, "Site A".to_string(), 0, 100));
        system.add_site(Site::new_settlement(body1_id, "Site B".to_string(), 10, 50));

        // Add site on body2
        system.add_site(Site::new_settlement(body2_id, "Site C".to_string(), 20, 75));

        assert_eq!(system.site_count(), 3);
        assert_eq!(system.site_count_on_body(&body1_id), 2);
        assert_eq!(system.site_count_on_body(&body2_id), 1);

        let body1_sites: Vec<_> = system.sites_on_body(&body1_id).collect();
        assert_eq!(body1_sites.len(), 2);
    }

    #[test]
    fn test_total_population() {
        let galaxy_id = GalaxyId::new();
        let mut system = StarSystem::new(galaxy_id, "Test System".to_string(), 123);

        let body_id = CelestialBodyId::new();
        system.add_site(Site::new_settlement(body_id, "Site A".to_string(), 0, 100));
        system.add_site(Site::new_settlement(body_id, "Site B".to_string(), 10, 200));
        system.add_site(Site::new_installation(body_id, "Mine".to_string(), 20, 25));

        assert_eq!(system.total_population(), 325);
    }

    #[test]
    fn test_mark_explored() {
        let galaxy_id = GalaxyId::new();
        let mut system = StarSystem::new(galaxy_id, "Unknown".to_string(), 999);

        assert!(!system.explored);
        system.mark_explored();
        assert!(system.explored);
    }

    #[test]
    fn test_all_bodies_iterator() {
        let galaxy_id = GalaxyId::new();
        let mut system = StarSystem::new(galaxy_id, "Test".to_string(), 1);

        system.add_body(CelestialBody::new(
            system.id,
            "Planet A".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::None,
            Temperature::Cold,
            1.0,
            5000.0,
            0.8,
        ));

        system.add_body(CelestialBody::new(
            system.id,
            "Planet B".to_string(),
            BodyType::GasGiant,
            Atmosphere::Crushing { composition: HashMap::new() },
            Temperature::Frozen,
            2.5,
            50000.0,
            100.0,
        ));

        let names: Vec<_> = system.all_bodies().map(|b| &b.name).collect();
        assert!(names.contains(&&"Planet A".to_string()));
        assert!(names.contains(&&"Planet B".to_string()));
    }
}
