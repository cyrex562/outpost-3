use bevy::prelude::*;
use crate::components::simulation::Simulation;
use crate::resources::game_state::GameState;
use crate::systems::{
    initialization::*,
    simulation::*,
    solar_system::*,
    ui::*,
    camera::*,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app
            // Add events
            .add_event::<TurnEndEvent>()
            
            // Add startup systems with proper ordering
            .add_systems(Startup, (
                setup_camera,
                initialize_game_state,
                spawn_celestial_bodies.after(initialize_game_state),
                setup_ui,
            ))
            
            // Add update systems
            .add_systems(Update, (
                // Camera systems
                camera_controller,
                
                // Simulation systems
                handle_end_turn_input,
                process_turn_end_events,
                
                // Solar system systems
                update_celestial_body_positions,
                update_game_state_positions,
                log_position_changes,
                
                // UI systems
                update_ui_displays,
                handle_end_turn_button,
            ));
    }
}
