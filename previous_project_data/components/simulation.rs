use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct Simulation {
    pub current_turn: u64,
}

impl Simulation {
    pub fn new() -> Self {
        Simulation {
            current_turn: 0,
        }
    }
    
    pub fn process_turn(&mut self) {
        self.current_turn += 1;
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}
