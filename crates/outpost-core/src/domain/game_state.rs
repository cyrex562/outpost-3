//! GameState — root of all game state.
//!
//! GameState is the top-level entity containing the galaxy and all game metadata.
//! All state changes are applied via events, never direct mutation.

use crate::content::{BuildingDefinition, ResourceDefinition, EventDefinition, TechDefinition};
use crate::domain::{Galaxy, EventLog};
use crate::simulation::GameClock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Per-site event engine state: which events have fired and when cooldowns expire.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventEngineState {
    /// Events that are non-repeatable and have already fired for a specific site.
    /// Key: `"{site_id}:{event_id}"`
    pub fired_non_repeatable: HashSet<String>,
    /// Cooldown expiry tick for events per site.
    /// Key: `"{site_id}:{event_id}"`, value: tick at which cooldown ends.
    pub cooldown_until: HashMap<String, u64>,
    /// Pending player choices: events that have fired but await player resolution.
    /// Key: `"{site_id}:{event_id}"`, value: event id.
    pub pending_choices: Vec<PendingEventChoice>,
}

/// An event that has fired and is awaiting a player choice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingEventChoice {
    pub site_id: String,
    pub event_id: String,
    pub fired_at_tick: u64,
    pub title: String,
    pub description: String,
}

/// The root game state containing all entities and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    /// The galaxy containing all star systems.
    pub galaxy: Galaxy,
    /// Game clock managing simulation time.
    pub clock: GameClock,
    /// Building definitions loaded from content files.
    #[serde(skip)]
    pub building_definitions: HashMap<String, BuildingDefinition>,
    /// Resource definitions loaded from content files.
    #[serde(skip)]
    pub resource_definitions: HashMap<String, ResourceDefinition>,
    /// Event definitions loaded from content files.
    #[serde(skip)]
    pub event_definitions: HashMap<String, EventDefinition>,
    /// Tech definitions loaded from content files.
    #[serde(skip)]
    pub tech_definitions: HashMap<String, TechDefinition>,
    /// Event engine runtime state (cooldowns, fired set, pending choices).
    pub event_engine: EventEngineState,
    /// Ordered log of gameplay events that have fired (for UI ticker and /events page).
    pub event_log: EventLog,
}

impl GameState {
    /// Create a new game state with an empty galaxy.
    pub fn new(galaxy_name: String, galaxy_seed: u64) -> Self {
        Self {
            galaxy: Galaxy::new(galaxy_name, galaxy_seed),
            clock: GameClock::new(),
            building_definitions: HashMap::new(),
            resource_definitions: HashMap::new(),
            event_definitions: HashMap::new(),
            tech_definitions: HashMap::new(),
            event_engine: EventEngineState::default(),
            event_log: EventLog::new(),
        }
    }

    /// Create a new game state with a pre-initialized galaxy.
    pub fn with_galaxy(galaxy: Galaxy) -> Self {
        Self {
            galaxy,
            clock: GameClock::new(),
            building_definitions: HashMap::new(),
            resource_definitions: HashMap::new(),
            event_definitions: HashMap::new(),
            tech_definitions: HashMap::new(),
            event_engine: EventEngineState::default(),
            event_log: EventLog::new(),
        }
    }

    /// Load content definitions from a ContentLoader.
    /// Called by the server layer after loading YAML files.
    pub fn load_content(
        &mut self,
        buildings: HashMap<String, BuildingDefinition>,
        resources: HashMap<String, ResourceDefinition>,
        events: HashMap<String, EventDefinition>,
        techs: HashMap<String, TechDefinition>,
    ) {
        self.building_definitions = buildings;
        self.resource_definitions = resources;
        self.event_definitions = events;
        self.tech_definitions = techs;
    }
    
    /// Load content definitions from a ContentLoader by cloning.
    /// Alternative to load_content() for convenience.
    pub fn load_from_content_loader(&mut self, loader: &crate::content::ContentLoader) {
        self.building_definitions = loader.all_buildings().clone();
        self.resource_definitions = loader.all_resources().clone();
        self.event_definitions = loader.all_events().clone();
        self.tech_definitions = loader.all_techs().clone();
    }

    /// Get a building definition by ID.
    pub fn get_building_def(&self, id: &str) -> Option<&BuildingDefinition> {
        self.building_definitions.get(id)
    }

    /// Get a resource definition by ID.
    pub fn get_resource_def(&self, id: &str) -> Option<&ResourceDefinition> {
        self.resource_definitions.get(id)
    }

    /// Get an event definition by ID.
    pub fn get_event_def(&self, id: &str) -> Option<&EventDefinition> {
        self.event_definitions.get(id)
    }

    /// Get a tech definition by ID.
    pub fn get_tech_def(&self, id: &str) -> Option<&TechDefinition> {
        self.tech_definitions.get(id)
    }

    /// Advance the simulation by one tick.
    pub fn advance_tick(&mut self) {
        self.clock.advance();
    }

    /// Advance the simulation by multiple ticks.
    pub fn advance_ticks(&mut self, count: u64) {
        self.clock.advance_by(count);
    }

    /// Set the game speed multiplier.
    pub fn set_speed(&mut self, multiplier: u32) -> Result<(), String> {
        self.clock.set_speed(multiplier)
    }

    /// Pause the simulation.
    pub fn pause(&mut self) {
        self.clock.pause();
    }

    /// Resume the simulation.
    pub fn resume(&mut self) {
        self.clock.resume();
    }

    /// Toggle pause state.
    pub fn toggle_pause(&mut self) {
        self.clock.toggle_pause();
    }

    // Convenience accessors for backward compatibility
    
    /// Get the current tick count.
    pub fn current_tick(&self) -> u64 {
        self.clock.current_tick
    }

    /// Check if the simulation is paused.
    pub fn is_paused(&self) -> bool {
        self.clock.paused
    }

    /// Get the current speed multiplier.
    pub fn speed_multiplier(&self) -> u32 {
        self.clock.speed_multiplier
    }

    /// Get total population across all systems.
    pub fn total_population(&self) -> u64 {
        self.galaxy.total_population()
    }

    /// Get total site count across all systems.
    pub fn total_sites(&self) -> usize {
        self.galaxy.total_sites()
    }

    /// Get total celestial body count across all systems.
    pub fn total_bodies(&self) -> usize {
        self.galaxy.total_bodies()
    }

    /// Get total star system count.
    pub fn total_systems(&self) -> usize {
        self.galaxy.system_count()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new("Milky Way".to_string(), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        Atmosphere, BodyType, CelestialBody, Site, StarSystem, Temperature,
    };
    use std::collections::HashMap;

    #[test]
    fn test_create_game_state() {
        let state = GameState::new("Test Galaxy".to_string(), 42);

        assert_eq!(state.galaxy.name, "Test Galaxy");
        assert_eq!(state.galaxy.seed, 42);
        assert_eq!(state.current_tick(), 0);
        assert_eq!(state.speed_multiplier(), 1);
        assert!(!state.is_paused());
    }

    #[test]
    fn test_default_game_state() {
        let state = GameState::default();

        assert_eq!(state.galaxy.name, "Milky Way");
        assert_eq!(state.current_tick(), 0);
        assert!(!state.is_paused());
    }

    #[test]
    fn test_advance_tick() {
        let mut state = GameState::new("Test".to_string(), 1);
        assert_eq!(state.current_tick(), 0);

        state.advance_tick();
        assert_eq!(state.current_tick(), 1);

        state.advance_tick();
        assert_eq!(state.current_tick(), 2);
    }

    #[test]
    fn test_advance_ticks() {
        let mut state = GameState::new("Test".to_string(), 1);
        assert_eq!(state.current_tick(), 0);

        state.advance_ticks(10);
        assert_eq!(state.current_tick(), 10);

        state.advance_ticks(5);
        assert_eq!(state.current_tick(), 15);
    }

    #[test]
    fn test_pause_prevents_tick_advance() {
        let mut state = GameState::new("Test".to_string(), 1);
        
        state.pause();
        assert!(state.is_paused());

        state.advance_tick();
        assert_eq!(state.current_tick(), 0, "Paused state should not advance");

        state.advance_ticks(10);
        assert_eq!(state.current_tick(), 0, "Paused state should not advance");

        state.resume();
        assert!(!state.is_paused());

        state.advance_tick();
        assert_eq!(state.current_tick(), 1, "Resumed state should advance");
    }

    #[test]
    fn test_toggle_pause() {
        let mut state = GameState::new("Test".to_string(), 1);
        assert!(!state.is_paused());

        state.toggle_pause();
        assert!(state.is_paused());

        state.toggle_pause();
        assert!(!state.is_paused());
    }

    #[test]
    fn test_set_speed() {
        let mut state = GameState::new("Test".to_string(), 1);
        assert_eq!(state.speed_multiplier(), 1);

        state.set_speed(5).unwrap();
        assert_eq!(state.speed_multiplier(), 5);

        // Invalid speeds now return errors
        assert!(state.set_speed(0).is_err());
        assert!(state.set_speed(100).is_err());
        
        // Speed should be unchanged after error
        assert_eq!(state.speed_multiplier(), 5);
    }

    #[test]
    fn test_statistics() {
        let mut state = GameState::new("Test".to_string(), 1);

        // Create and add a system
        let mut system = StarSystem::new(state.galaxy.id, "Sol".to_string(), 42);

        // Add a body
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

        // Add sites
        system.add_site(Site::new_settlement(
            body_id,
            "City 1".to_string(),
            0,
            1000,
        ));
        system.add_site(Site::new_settlement(
            body_id,
            "City 2".to_string(),
            10,
            500,
        ));

        state.galaxy.add_system(system);

        assert_eq!(state.total_systems(), 1);
        assert_eq!(state.total_bodies(), 1);
        assert_eq!(state.total_sites(), 2);
        assert_eq!(state.total_population(), 1500);
    }

    #[test]
    fn test_with_galaxy() {
        let galaxy = Galaxy::new("Custom Galaxy".to_string(), 999);
        let state = GameState::with_galaxy(galaxy);

        assert_eq!(state.galaxy.name, "Custom Galaxy");
        assert_eq!(state.galaxy.seed, 999);
        assert_eq!(state.current_tick(), 0);
    }

    #[test]
    fn test_serialization() {
        let state = GameState::new("Test".to_string(), 123);
        
        // Serialize to JSON
        let json = serde_json::to_string(&state).unwrap();
        
        // Deserialize back
        let deserialized: GameState = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.galaxy.name, state.galaxy.name);
        assert_eq!(deserialized.galaxy.seed, state.galaxy.seed);
        assert_eq!(deserialized.current_tick(), state.current_tick());
        assert_eq!(deserialized.is_paused(), state.is_paused());
    }

    // Property-based tests for hierarchy integrity
    mod proptests {
        use super::*;
        use crate::domain::{CelestialBodyId, GalaxyId, StarSystemId};
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_id_uniqueness(galaxy_count in 1usize..10) {
                // Generate multiple galaxies and verify IDs are unique
                let galaxies: Vec<_> = (0..galaxy_count)
                    .map(|i| Galaxy::new(format!("Galaxy {}", i), i as u64))
                    .collect();

                let ids: std::collections::HashSet<_> = galaxies.iter().map(|g| g.id).collect();
                prop_assert_eq!(ids.len(), galaxy_count, "All galaxy IDs should be unique");
            }

            #[test]
            fn test_system_uniqueness(system_count in 1usize..10) {
                // Generate multiple systems and verify IDs are unique
                let galaxy_id = GalaxyId::new();
                let systems: Vec<_> = (0..system_count)
                    .map(|i| StarSystem::new(galaxy_id, format!("System {}", i), i as u64))
                    .collect();

                let ids: std::collections::HashSet<_> = systems.iter().map(|s| s.id).collect();
                prop_assert_eq!(ids.len(), system_count, "All system IDs should be unique");
            }

            #[test]
            fn test_body_uniqueness(body_count in 1usize..10) {
                // Generate multiple celestial bodies and verify IDs are unique
                use crate::domain::{Atmosphere, BodyType, CelestialBody, Temperature};

                let system_id = StarSystemId::new();
                let bodies: Vec<_> = (0..body_count)
                    .map(|i| CelestialBody::new(
                        system_id,
                        format!("Body {}", i),
                        BodyType::TerrestrialPlanet,
                        Atmosphere::None,
                        Temperature::Cold,
                        1.0,
                        5000.0,
                        1.0,
                    ))
                    .collect();

                let ids: std::collections::HashSet<_> = bodies.iter().map(|b| b.id).collect();
                prop_assert_eq!(ids.len(), body_count, "All body IDs should be unique");
            }

            #[test]
            fn test_site_uniqueness(site_count in 1usize..10) {
                // Generate multiple sites and verify IDs are unique
                use crate::domain::Site;

                let body_id = CelestialBodyId::new();
                let sites: Vec<_> = (0..site_count)
                    .map(|i| Site::new_settlement(
                        body_id,
                        format!("Site {}", i),
                        i as u64,
                        100,
                    ))
                    .collect();

                let ids: std::collections::HashSet<_> = sites.iter().map(|s| s.id).collect();
                prop_assert_eq!(ids.len(), site_count, "All site IDs should be unique");
            }

            #[test]
            fn test_tick_advance_monotonic(advances in prop::collection::vec(1u64..100, 1..10)) {
                // Verify that ticks always increase monotonically
                let mut state = GameState::new("Test".to_string(), 1);
                let mut last_tick = 0u64;

                for advance_count in advances {
                    state.advance_ticks(advance_count);
                    prop_assert!(state.current_tick() > last_tick, "Ticks should increase monotonically");
                    last_tick = state.current_tick();
                }
            }

            #[test]
            fn test_pause_prevents_advance(tick_count in 1u64..100) {
                // Verify paused state prevents tick advancement
                let mut state = GameState::new("Test".to_string(), 1);
                state.pause();

                state.advance_ticks(tick_count);
                prop_assert_eq!(state.current_tick(), 0, "Paused state should not advance");

                state.resume();
                state.advance_ticks(tick_count);
                prop_assert_eq!(state.current_tick(), tick_count, "Resumed state should advance");
            }

            #[test]
            fn test_speed_multiplier_bounds(speed in 0u32..1000) {
                // Verify speed multiplier is always >= 1 and <= max
                let mut state = GameState::new("Test".to_string(), 1);
                let _ = state.set_speed(speed); // May fail for invalid values
                prop_assert!(state.speed_multiplier() >= 1, "Speed multiplier should be at least 1");
                prop_assert!(state.speed_multiplier() <= 10, "Speed multiplier should not exceed max");
            }

            #[test]
            fn test_hierarchy_consistency(
                system_count in 1usize..5,
                bodies_per_system in 1usize..3,
                sites_per_body in 1usize..3,
            ) {
                // Build a complete hierarchy and verify referential integrity
                use crate::domain::{Atmosphere, BodyType, CelestialBody, Site, Temperature};

                let mut state = GameState::new("Test".to_string(), 1);

                for sys_idx in 0..system_count {
                    let mut system = StarSystem::new(
                        state.galaxy.id,
                        format!("System{}", sys_idx),
                        sys_idx as u64,
                    );

                    for body_idx in 0..bodies_per_system {
                        let body = CelestialBody::new(
                            system.id,
                            format!("Body{}_{}", sys_idx, body_idx),
                            BodyType::TerrestrialPlanet,
                            Atmosphere::None,
                            Temperature::Cold,
                            1.0,
                            5000.0,
                            1.0,
                        );
                        let body_id = system.add_body(body);

                        for site_idx in 0..sites_per_body {
                            let site = Site::new_settlement(
                                body_id,
                                format!("Site{}_{}_{}", sys_idx, body_idx, site_idx),
                                0,
                                100,
                            );
                            system.add_site(site);
                        }
                    }

                    state.galaxy.add_system(system);
                }

                // Verify counts
                prop_assert_eq!(state.galaxy.system_count(), system_count);
                prop_assert_eq!(state.total_bodies(), system_count * bodies_per_system);
                prop_assert_eq!(state.total_sites(), system_count * bodies_per_system * sites_per_body);

                // Verify all sites reference valid bodies
                for system in state.galaxy.all_systems() {
                    for site in system.all_sites() {
                        prop_assert!(
                            system.get_body(&site.body_id).is_some(),
                            "Site should reference a valid body in its system"
                        );
                    }
                }
            }
        }
    }
}
