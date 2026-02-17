//! CelestialBody — planets, moons, asteroids, and orbital stations.
//!
//! This entity extends the pre-V5 Planet concept to include all types
//! of celestial bodies that can host sites or provide resources.

use crate::domain::{CelestialBodyId, StarSystemId, ResourceDeposit};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of celestial body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BodyType {
    /// Rocky terrestrial planet.
    TerrestrialPlanet,
    /// Gas giant planet.
    GasGiant,
    /// Ice giant planet.
    IceGiant,
    /// Natural satellite orbiting a planet.
    Moon,
    /// Rocky body in asteroid belt.
    Asteroid,
    /// Icy body from outer system.
    Comet,
    /// Artificial orbital station.
    OrbitalStation,
    /// Small rocky body.
    Dwarf,
}

/// Atmospheric composition and pressure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Atmosphere {
    /// No atmosphere (vacuum).
    None,
    /// Trace atmosphere (< 0.01 bar).
    Trace,
    /// Thin atmosphere (0.01-0.5 bar).
    Thin { composition: HashMap<String, f64> },
    /// Earth-like atmosphere (0.5-2 bar).
    Breathable { composition: HashMap<String, f64> },
    /// Dense atmosphere (2-10 bar).
    Dense { composition: HashMap<String, f64> },
    /// Crushing atmosphere (> 10 bar).
    Crushing { composition: HashMap<String, f64> },
    /// Toxic or corrosive atmosphere.
    Toxic { composition: HashMap<String, f64> },
}

impl Atmosphere {
    /// Return atmospheric pressure in bars.
    pub fn pressure_bars(&self) -> f64 {
        match self {
            Atmosphere::None => 0.0,
            Atmosphere::Trace => 0.005,
            Atmosphere::Thin { .. } => 0.25,
            Atmosphere::Breathable { .. } => 1.0,
            Atmosphere::Dense { .. } => 5.0,
            Atmosphere::Crushing { .. } => 50.0,
            Atmosphere::Toxic { .. } => 1.0,
        }
    }

    /// Returns true if humans can breathe without life support.
    pub fn is_breathable(&self) -> bool {
        matches!(self, Atmosphere::Breathable { .. })
    }
}

/// Temperature classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Temperature {
    /// Below -100°C
    Frozen,
    /// -100°C to 0°C
    Cold,
    /// 0°C to 30°C (human-comfortable)
    Temperate,
    /// 30°C to 60°C
    Hot,
    /// Above 60°C
    Scorching,
}

/// Environmental hazard level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum HazardLevel {
    /// Minimal hazards, easy colonization.
    Minimal,
    /// Low hazards, routine precautions needed.
    Low,
    /// Moderate hazards, significant infrastructure required.
    Moderate,
    /// High hazards, expensive mitigation needed.
    High,
    /// Extreme hazards, only specialized installations viable.
    Extreme,
}

/// A celestial body that can host sites or provide resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CelestialBody {
    /// Unique identifier.
    pub id: CelestialBodyId,
    /// Parent star system.
    pub star_system_id: StarSystemId,
    /// Body name (e.g., "Earth", "Luna", "Ceres").
    pub name: String,
    /// Type of celestial body.
    pub body_type: BodyType,
    /// Atmospheric properties.
    pub atmosphere: Atmosphere,
    /// Temperature classification.
    pub temperature: Temperature,
    /// Surface gravity in Earth-G (1.0 = Earth gravity).
    pub gravity: f64,
    /// Mean radius in kilometers.
    pub radius_km: f64,
    /// Mass in Earth masses (1.0 = Earth mass).
    pub mass_earth: f64,
    /// Day length in hours (rotation period).
    pub day_length_hours: f64,
    /// Year length in Earth days (orbital period, 0 for stations).
    pub year_length_days: f64,
    /// Axial tilt in degrees.
    pub axial_tilt_degrees: f64,
    /// Resource richness per resource type (0.0 = none, 1.0 = abundant).
    /// This is used for procedural generation and can be replaced by deposits.
    #[deprecated(note = "Use resource_deposits instead for detailed extraction tracking")]
    pub resource_richness: HashMap<String, f64>,
    /// Resource deposits available for extraction.
    pub resource_deposits: Vec<ResourceDeposit>,
    /// Overall hazard level for colonization.
    pub hazard_level: HazardLevel,
    /// Optional parent body ID (for moons orbiting planets).
    pub parent_body_id: Option<CelestialBodyId>,
}

impl CelestialBody {
    /// Create a new celestial body.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        star_system_id: StarSystemId,
        name: String,
        body_type: BodyType,
        atmosphere: Atmosphere,
        temperature: Temperature,
        gravity: f64,
        radius_km: f64,
        mass_earth: f64,
    ) -> Self {
        Self {
            id: CelestialBodyId::new(),
            star_system_id,
            name,
            body_type,
            atmosphere,
            temperature,
            gravity,
            radius_km,
            mass_earth,
            day_length_hours: 24.0, // Default to Earth-like
            year_length_days: 365.25, // Default to Earth-like
            axial_tilt_degrees: 23.5, // Default to Earth-like
            resource_richness: HashMap::new(),
            resource_deposits: Vec::new(),
            hazard_level: HazardLevel::Moderate,
            parent_body_id: None,
        }
    }

    /// Set day length (rotation period).
    pub fn with_day_length(mut self, hours: f64) -> Self {
        self.day_length_hours = hours;
        self
    }

    /// Set year length (orbital period).
    pub fn with_year_length(mut self, days: f64) -> Self {
        self.year_length_days = days;
        self
    }

    /// Set axial tilt.
    pub fn with_axial_tilt(mut self, degrees: f64) -> Self {
        self.axial_tilt_degrees = degrees;
        self
    }

    /// Set resource richness.
    pub fn with_resource_richness(mut self, richness: HashMap<String, f64>) -> Self {
        self.resource_richness = richness;
        self
    }

    /// Set hazard level.
    pub fn with_hazard_level(mut self, level: HazardLevel) -> Self {
        self.hazard_level = level;
        self
    }

    /// Set parent body (for moons).
    pub fn with_parent_body(mut self, parent_id: CelestialBodyId) -> Self {
        self.parent_body_id = Some(parent_id);
        self
    }

    /// Calculate difficulty rating for colonization (0.0 = easiest, 1.0+ = very difficult).
    pub fn difficulty_rating(&self) -> f64 {
        let mut difficulty = 0.0;

        // Hostile atmosphere increases difficulty
        difficulty += match &self.atmosphere {
            Atmosphere::None => 0.3,
            Atmosphere::Trace => 0.2,
            Atmosphere::Toxic { .. } => 0.4,
            Atmosphere::Crushing { .. } => 0.5,
            Atmosphere::Breathable { .. } => 0.0,
            Atmosphere::Thin { .. } => 0.1,
            Atmosphere::Dense { .. } => 0.2,
        };

        // Extreme temperatures increase difficulty
        difficulty += match self.temperature {
            Temperature::Frozen => 0.3,
            Temperature::Scorching => 0.3,
            Temperature::Cold => 0.1,
            Temperature::Hot => 0.1,
            Temperature::Temperate => 0.0,
        };

        // Extreme gravity increases difficulty
        let gravity_deviation = (self.gravity - 1.0).abs();
        difficulty += gravity_deviation * 0.2;

        // Hazard level contributes
        difficulty += match self.hazard_level {
            HazardLevel::Minimal => 0.0,
            HazardLevel::Low => 0.1,
            HazardLevel::Moderate => 0.2,
            HazardLevel::High => 0.4,
            HazardLevel::Extreme => 0.6,
        };

        difficulty
    }

    /// Returns true if this body can host surface sites.
    pub fn can_host_surface_sites(&self) -> bool {
        matches!(
            self.body_type,
            BodyType::TerrestrialPlanet | BodyType::Moon | BodyType::Asteroid | BodyType::Dwarf
        )
    }

    /// Returns true if this is a moon orbiting another body.
    pub fn is_moon(&self) -> bool {
        self.body_type == BodyType::Moon && self.parent_body_id.is_some()
    }

    /// Returns true if this is an artificial orbital station.
    pub fn is_station(&self) -> bool {
        self.body_type == BodyType::OrbitalStation
    }

    /// Add a resource deposit to this body.
    pub fn add_deposit(&mut self, deposit: ResourceDeposit) {
        self.resource_deposits.push(deposit);
    }

    /// Add multiple resource deposits at once.
    pub fn add_deposits(&mut self, deposits: Vec<ResourceDeposit>) {
        self.resource_deposits.extend(deposits);
    }

    /// Get all deposits of a specific resource type.
    pub fn get_deposits(&self, resource_id: &str) -> Vec<&ResourceDeposit> {
        self.resource_deposits
            .iter()
            .filter(|d| d.resource_id == resource_id)
            .collect()
    }

    /// Get mutable reference to all deposits of a specific resource type.
    pub fn get_deposits_mut(&mut self, resource_id: &str) -> Vec<&mut ResourceDeposit> {
        self.resource_deposits
            .iter_mut()
            .filter(|d| d.resource_id == resource_id)
            .collect()
    }

    /// Calculate total remaining quantity for a specific resource across all deposits.
    pub fn total_remaining_resource(&self, resource_id: &str) -> f64 {
        self.resource_deposits
            .iter()
            .filter(|d| d.resource_id == resource_id)
            .map(|d| d.remaining_quantity)
            .sum()
    }

    /// Check if body has extractable resources (non-depleted deposits).
    pub fn has_extractable_resources(&self) -> bool {
        self.resource_deposits.iter().any(|d| !d.is_depleted())
    }

    /// Get list of all resource types present in deposits.
    pub fn available_resources(&self) -> Vec<String> {
        let mut resources: Vec<_> = self.resource_deposits
            .iter()
            .filter(|d| !d.is_depleted())
            .map(|d| d.resource_id.clone())
            .collect();
        resources.sort();
        resources.dedup();
        resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_terrestrial_planet() {
        let system_id = StarSystemId::new();
        let body = CelestialBody::new(
            system_id,
            "Test Planet".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Breathable {
                composition: [("N2".to_string(), 0.78), ("O2".to_string(), 0.21)]
                    .iter()
                    .cloned()
                    .collect(),
            },
            Temperature::Temperate,
            1.0,
            6371.0,
            1.0,
        );

        assert_eq!(body.name, "Test Planet");
        assert_eq!(body.body_type, BodyType::TerrestrialPlanet);
        assert_eq!(body.gravity, 1.0);
        assert!(body.can_host_surface_sites());
        assert!(!body.is_moon());
    }

    #[test]
    fn test_create_moon() {
        let system_id = StarSystemId::new();
        let parent_id = CelestialBodyId::new();
        let moon = CelestialBody::new(
            system_id,
            "Luna".to_string(),
            BodyType::Moon,
            Atmosphere::None,
            Temperature::Cold,
            0.166,
            1737.4,
            0.0123,
        )
        .with_parent_body(parent_id);

        assert_eq!(moon.body_type, BodyType::Moon);
        assert!(moon.is_moon());
        assert_eq!(moon.parent_body_id, Some(parent_id));
    }

    #[test]
    fn test_atmosphere_pressure() {
        let breathable = Atmosphere::Breathable {
            composition: HashMap::new(),
        };
        assert_eq!(breathable.pressure_bars(), 1.0);
        assert!(breathable.is_breathable());

        let none = Atmosphere::None;
        assert_eq!(none.pressure_bars(), 0.0);
        assert!(!none.is_breathable());
    }

    #[test]
    fn test_difficulty_rating() {
        let system_id = StarSystemId::new();

        // Earth-like: should be easy
        let earth_like = CelestialBody::new(
            system_id,
            "Earth-like".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Breathable {
                composition: HashMap::new(),
            },
            Temperature::Temperate,
            1.0,
            6371.0,
            1.0,
        )
        .with_hazard_level(HazardLevel::Minimal);

        assert!(
            earth_like.difficulty_rating() < 0.5,
            "Earth-like should be easy"
        );

        // Hostile world: should be difficult
        let hostile = CelestialBody::new(
            system_id,
            "Hostile".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Toxic {
                composition: HashMap::new(),
            },
            Temperature::Scorching,
            2.5,
            8000.0,
            3.0,
        )
        .with_hazard_level(HazardLevel::Extreme);

        assert!(
            hostile.difficulty_rating() > 1.0,
            "Hostile world should be difficult"
        );
    }

    #[test]
    fn test_builder_pattern() {
        let system_id = StarSystemId::new();
        let body = CelestialBody::new(
            system_id,
            "Test".to_string(),
            BodyType::TerrestrialPlanet,
            Atmosphere::Thin {
                composition: HashMap::new(),
            },
            Temperature::Cold,
            0.8,
            5000.0,
            0.5,
        )
        .with_day_length(20.0)
        .with_year_length(400.0)
        .with_axial_tilt(15.0)
        .with_hazard_level(HazardLevel::Low)
        .with_resource_richness(
            [("iron".to_string(), 0.8), ("copper".to_string(), 0.6)]
                .iter()
                .cloned()
                .collect(),
        );

        assert_eq!(body.day_length_hours, 20.0);
        assert_eq!(body.year_length_days, 400.0);
        assert_eq!(body.axial_tilt_degrees, 15.0);
        assert_eq!(body.hazard_level, HazardLevel::Low);
        assert_eq!(body.resource_richness.get("iron"), Some(&0.8));
    }
}
