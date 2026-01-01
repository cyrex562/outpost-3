use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use crate::domain::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_id: u64,
    pub timestamp: DateTime<Utc>,
    pub turn_number: u64,
    pub event_type: EventType,
}

impl GameEvent {
    pub fn new(event_id: u64, turn_number: u64, event_type: EventType) -> Self {
        Self {
            event_id,
            timestamp: Utc::now(),
            turn_number,
            event_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventType {
    // Colony events
    ColonyFounded {
        colony_id: ColonyId,
        planet_id: PlanetId,
        name: String,
        starting_resources: Resources,
    },
    BuildingConstructionStarted {
        building_id: BuildingId,
        colony_id: ColonyId,
        building_type: BuildingType,
    },
    BuildingConstructionCompleted {
        building_id: BuildingId,
        colony_id: ColonyId,
    },
    BuildingStateChanged {
        building_id: BuildingId,
        colony_id: ColonyId,
        new_state: BuildingState,
    },
    ResourcesExtracted {
        colony_id: ColonyId,
        resource_type: ResourceType,
        amount: i64,
    },
    ResourcesConsumed {
        colony_id: ColonyId,
        resource_type: ResourceType,
        amount: i64,
    },
    ResourcesProduced {
        colony_id: ColonyId,
        resource_type: ResourceType,
        amount: i64,
    },
    PopulationChanged {
        colony_id: ColonyId,
        new_population: u64,
    },
    PowerGridUpdated {
        colony_id: ColonyId,
        generation: u32,
        consumption: u32,
    },
    PopulationGrew {
        colony_id: ColonyId,
        old_population: u64,
        new_population: u64,
    },
    LaborAllocated {
        colony_id: ColonyId,
        building_id: BuildingId,
        workers_allocated: u32,
    },
    LaborDeallocated {
        colony_id: ColonyId,
        building_id: BuildingId,
        workers_deallocated: u32,
    },
    BuildingDamaged {
        building_id: BuildingId,
        colony_id: ColonyId,
        damage_severity: u8,
    },
    BuildingRepaired {
        building_id: BuildingId,
        colony_id: ColonyId,
    },
    BuildingUpgraded {
        building_id: BuildingId,
        colony_id: ColonyId,
        new_level: u8,
    },

    // Wormhole events
    PlanetDiscovered {
        planet_id: PlanetId,
        planet: Planet,
    },
    GateConstructionStarted {
        gate_id: WormholeId,
        source_planet: PlanetId,
        destination_planet: PlanetId,
    },
    GateConstructionCompleted {
        gate_id: WormholeId,
    },
    GateActivated {
        gate_id: WormholeId,
    },
    GateDeactivated {
        gate_id: WormholeId,
    },

    // Train events
    TrainPurchased {
        train_id: TrainId,
        colony_id: ColonyId,
        train_type: TrainType,
        cost: i64,
    },
    TrainAssignedToRoute {
        train_id: TrainId,
        route_id: RouteId,
    },
    TrainDispatched {
        train_id: TrainId,
        route_id: RouteId,
        origin: PlanetId,
        destination: PlanetId,
    },
    TrainArrived {
        train_id: TrainId,
        planet_id: PlanetId,
    },

    // Simulation events
    TurnAdvanced {
        turn_number: u64,
    },
}
