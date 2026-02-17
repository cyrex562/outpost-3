//! Core entity identifiers using UUIDs.
//!
//! All entity IDs follow the newtype pattern around UUID v4 for:
//! - Type safety (cannot mix SiteId with BuildingId)
//! - Network safety (globally unique across distributed systems)
//! - Serialization support

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a Galaxy (root entity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GalaxyId(pub Uuid);

impl GalaxyId {
    /// Generate a new random GalaxyId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for GalaxyId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a StarSystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StarSystemId(pub Uuid);

impl StarSystemId {
    /// Generate a new random StarSystemId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for StarSystemId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a CelestialBody (planet, moon, asteroid, station).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelestialBodyId(pub Uuid);

impl CelestialBodyId {
    /// Generate a new random CelestialBodyId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CelestialBodyId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a Site (settlement or installation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SiteId(pub Uuid);

impl SiteId {
    /// Generate a new random SiteId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SiteId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SiteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a Building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BuildingId(pub Uuid);

impl BuildingId {
    /// Generate a new random BuildingId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for BuildingId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BuildingId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_galaxy_id_uniqueness() {
        let id1 = GalaxyId::new();
        let id2 = GalaxyId::new();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_star_system_id_uniqueness() {
        let id1 = StarSystemId::new();
        let id2 = StarSystemId::new();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_celestial_body_id_uniqueness() {
        let id1 = CelestialBodyId::new();
        let id2 = CelestialBodyId::new();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_site_id_uniqueness() {
        let id1 = SiteId::new();
        let id2 = SiteId::new();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_building_id_uniqueness() {
        let id1 = BuildingId::new();
        let id2 = BuildingId::new();
        assert_ne!(id1, id2, "Generated IDs should be unique");
    }

    #[test]
    fn test_id_serialization() {
        let id = SiteId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: SiteId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_id_display() {
        let id = SiteId::new();
        let uuid_str = format!("{}", id.0);
        assert_eq!(uuid_str.len(), 36); // UUID string format
    }
}
