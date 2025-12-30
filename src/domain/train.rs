use serde::{Deserialize, Serialize};
use super::{PlanetId, ResourceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainType {
    Freight,
    Passenger,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainSize {
    Small,
    Medium,
    Large,
    Massive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainState {
    Idle,
    InTransit,
    Loading,
    Unloading,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Train {
    pub id: TrainId,
    pub name: String,
    pub train_type: TrainType,
    pub size: TrainSize,
    pub speed: u32,
    pub capacity: u32,
    pub current_planet_id: Option<PlanetId>,
    pub state: TrainState,
}

impl Train {
    pub fn new(id: TrainId, name: String, train_type: TrainType, size: TrainSize) -> Self {
        let (speed, capacity) = match size {
            TrainSize::Small => (100, 50),
            TrainSize::Medium => (80, 200),
            TrainSize::Large => (60, 500),
            TrainSize::Massive => (40, 1000),
        };

        Self {
            id,
            name,
            train_type,
            size,
            speed,
            capacity,
            current_planet_id: None,
            state: TrainState::Idle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: RouteId,
    pub name: String,
    pub origin_planet_id: PlanetId,
    pub destination_planet_id: PlanetId,
    pub active: bool,
}

impl Route {
    pub fn new(id: RouteId, name: String, origin: PlanetId, destination: PlanetId) -> Self {
        Self {
            id,
            name,
            origin_planet_id: origin,
            destination_planet_id: destination,
            active: true,
        }
    }
}
