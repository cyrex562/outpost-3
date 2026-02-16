use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    BuildingRecipeChanged {
        building_id: BuildingId,
        colony_id: ColonyId,
        old_recipe_index: u32,
        new_recipe_index: u32,
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

    // Market events
    MarketPriceChanged {
        resource: ResourceType,
        old_price: f64,
        new_price: f64,
    },

    // Trading events
    ResourceTraded {
        colony_id: ColonyId,
        resource_type: ResourceType,
        quantity: i64,
        price: f64,
        side: TradeSide,
    },

    // Economy events
    EconomySnapshot {
        colony_id: ColonyId,
        gdp: i64,
        income: i64,
        expenses: i64,
        net_worth: i64,
    },

    // Banking events
    LoanIssued {
        loan_id: LoanId,
        colony_id: ColonyId,
        principal: f64,
        interest_rate: f64,
        term_turns: u32,
    },
    LoanPaymentMade {
        loan_id: LoanId,
        colony_id: ColonyId,
        amount: f64,
        principal_paid: f64,
        interest_paid: f64,
        remaining_principal: f64,
    },
    LoanPaidOff {
        loan_id: LoanId,
        colony_id: ColonyId,
    },

    // Rail events
    RailConstructionStarted {
        segment_id: SegmentId,
        colony_id: ColonyId,
        from: HexCoord,
        to: HexCoord,
        rail_type: RailType,
    },
    RailConstructionCompleted {
        segment_id: SegmentId,
        colony_id: ColonyId,
    },
    RailUpgraded {
        segment_id: SegmentId,
        colony_id: ColonyId,
        new_type: RailType,
        new_count: u8,
    },
    RailRemoved {
        segment_id: SegmentId,
        colony_id: ColonyId,
    },

    // Station events
    StationBuilt {
        station_id: StationId,
        colony_id: ColonyId,
        building_id: BuildingId,
        connected_segments: Vec<SegmentId>,
    },
    StationRemoved {
        station_id: StationId,
        colony_id: ColonyId,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TradeSide {
    Buy,
    Sell,
}
