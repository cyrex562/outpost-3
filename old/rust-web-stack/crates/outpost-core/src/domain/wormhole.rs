use serde::{Deserialize, Serialize};
use super::PlanetId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WormholeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateState {
    UnderConstruction { progress: u8 },
    Active,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WormholeGate {
    pub id: WormholeId,
    pub source_planet_id: PlanetId,
    pub destination_planet_id: PlanetId,
    pub state: GateState,
    pub throughput_capacity: u32,
}

impl WormholeGate {
    pub fn new(
        id: WormholeId,
        source_planet_id: PlanetId,
        destination_planet_id: PlanetId,
    ) -> Self {
        Self {
            id,
            source_planet_id,
            destination_planet_id,
            state: GateState::UnderConstruction { progress: 0 },
            throughput_capacity: 10,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, GateState::Active)
    }
}
