use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use crate::simulation::simulation::Simulation;
use crate::universe::solar_system_manager::SolarSystemManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub simulation: Simulation,
    pub solar_system: SolarSystemManager,
    // Add other game-specific state here that is not part of the core simulation.
}

impl GameState {
    /// Creates a new game state with a fresh simulation and solar system.
    pub fn new() -> Self {
        let start_date = NaiveDate::from_ymd_opt(2070, 1, 1).unwrap();
        Self {
            simulation: Simulation::new(),
            solar_system: SolarSystemManager::new(start_date),
        }
    }
    
    /// Loads the solar system data from CSV
    pub fn load_solar_system_data(&mut self, csv_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.solar_system.load_from_csv(std::path::Path::new(csv_path))
    }
    
    /// Updates the game world (called at the beginning of each turn)
    pub fn update_world(&mut self) {
        // Update orbital positions (30 days per turn)
        self.solar_system.update_all_positions(30.0);
    }
    
    /// Advances the game state by processing a simulation turn.
    pub fn process_turn(&mut self) {
        // Update world state (orbital positions, etc.)
        self.update_world();
        
        // Process simulation turn
        self.simulation.process_turn();
    }
    
    /// Gets the current game date
    pub fn get_game_date(&self) -> NaiveDate {
        self.solar_system.get_game_date()
    }
    
    /// Gets formatted game date string
    pub fn get_formatted_date(&self) -> String {
        self.solar_system.get_formatted_date()
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }   
}