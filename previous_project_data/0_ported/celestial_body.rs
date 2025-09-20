use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::components::orbital_mechanics::OrbitalState;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum CelestialBodyType {
    Star,
    Planet,
    Moon,
    Asteroid,
    Comet,
    DwarfPlanet,
}

#[derive(Component, Debug, Clone, Deserialize, Serialize)]
pub struct CelestialBody {
    pub name: String,
    pub id: Uuid,
    pub body_type: CelestialBodyType,
    pub orbital_state: Option<OrbitalState>,
    pub mass: f64,  // Mass in kg
    pub diameter: f64,  // Diameter in km
}

impl CelestialBody {
    pub fn new(name: String, body_type: CelestialBodyType, mass: f64, diameter: f64) -> Self {
        Self {
            name,
            id: Uuid::new_v4(),
            body_type,
            orbital_state: None,
            mass,
            diameter,
        }
    }
    
    pub fn with_orbital_state(mut self, orbital_state: OrbitalState) -> Self {
        self.orbital_state = Some(orbital_state);
        self
    }
    
    pub fn update_orbital_position(&mut self, days_elapsed: f64) {
        if let Some(ref mut orbital_state) = self.orbital_state {
            orbital_state.update_position(days_elapsed);
        }
    }
    
    pub fn has_significant_position_change(&self, previous_angle: f64, threshold_degrees: f64) -> bool {
        if let Some(ref orbital_state) = self.orbital_state {
            orbital_state.is_significant_change(previous_angle, threshold_degrees)
        } else {
            false
        }
    }
}

// Marker component for celestial bodies that should be rendered
#[derive(Component)]
pub struct CelestialBodyRenderable;

// Component for tracking previous positions for change detection
#[derive(Component)]
pub struct PreviousPosition {
    pub angle: f64,
}
