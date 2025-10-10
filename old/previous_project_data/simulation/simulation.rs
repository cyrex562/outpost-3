use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug,Serialize,Deserialize,Clone)]
pub struct Simulation {
    pub current_turn: u64,
    // placeholder for other elements
    // pub solar_system: SolarSystem,
    // pub factions: HashMap<Uuid, Faction>,
    // pub structures: HashMap<Uuid, StructureType>,
    // pub events_log: Vec<NarrativeEvent>,
}

impl Simulation {
    pub fn new() -> Self {
        info!("Initializing simulation");
        Simulation {
            current_turn: 0,
        }
    }
    
    pub fn process_turn(&mut self) {
        self.current_turn += 1;
        info!("Processing turn {}", self.current_turn);
        // TODO: implement additional turn logic:
        // - process faction orders
        // - update resource production/consumption
        // - move spacecraft
        // - generate events
        // - update celestial body positions
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}