use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::ResourceType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanetId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanetType {
    Terrestrial,
    Desert,
    Ice,
    Ocean,
    GasGiantMoon,
    Barren,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Atmosphere {
    None,
    Thin,
    Breathable,
    Toxic,
    Dense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Temperature {
    Arctic,
    Cold,
    Temperate,
    Hot,
    Scorching,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planet {
    pub id: PlanetId,
    pub name: String,
    pub planet_type: PlanetType,
    pub atmosphere: Atmosphere,
    pub temperature: Temperature,
    pub gravity: f32,
    pub size: u32,
    pub resource_richness: HashMap<ResourceType, u8>,
    pub hazard_level: u8,
    pub description: String,
}

impl Planet {
    pub fn difficulty_rating(&self) -> u8 {
        let mut difficulty = 0;

        // Atmosphere difficulty
        difficulty += match self.atmosphere {
            Atmosphere::Breathable => 0,
            Atmosphere::Thin => 2,
            Atmosphere::Dense => 2,
            Atmosphere::Toxic => 4,
            Atmosphere::None => 5,
        };

        // Temperature difficulty
        difficulty += match self.temperature {
            Temperature::Temperate => 0,
            Temperature::Cold | Temperature::Hot => 2,
            Temperature::Arctic | Temperature::Scorching => 4,
        };

        // Hazard level
        difficulty += self.hazard_level;

        difficulty.min(10)
    }

    pub fn get_resource_richness(&self, resource_type: ResourceType) -> u8 {
        *self.resource_richness.get(&resource_type).unwrap_or(&0)
    }
}
