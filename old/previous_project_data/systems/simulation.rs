use bevy::prelude::*;
use log::info;
use crate::components::simulation::Simulation;
use crate::resources::game_state::GameState;

/// System to process simulation turns
pub fn process_simulation_turn(
    mut game_state: ResMut<GameState>,
    mut simulation: ResMut<Simulation>,
) {
    // Process the turn
    game_state.process_turn();
    simulation.process_turn();
    
    info!("=== Turn {} ===", simulation.current_turn);
    info!("Game date after turn: {}", game_state.get_formatted_date());
    
    // Display some sample body positions
    let bodies = game_state.solar_system.get_all_bodies();
    let sample_bodies = ["Earth", "Mars", "Jupiter", "Saturn"];
    
    for body_name in sample_bodies.iter() {
        if let Some(body) = bodies.get(*body_name) {
            if let Some(ref orbital_state) = body.orbital_state {
                let cartesian = orbital_state.to_cartesian();
                info!("  {}: Position ({:.0}, {:.0}) km, Angle: {:.1}°", 
                      body_name, cartesian.x, cartesian.y, orbital_state.angle_degrees());
            }
        }
    }
}

/// System to handle end turn input
pub fn handle_end_turn_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut turn_events: EventWriter<TurnEndEvent>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter) {
        turn_events.send(TurnEndEvent);
    }
}

/// Event for when a turn ends
#[derive(Event)]
pub struct TurnEndEvent;

/// System to process turn end events
pub fn process_turn_end_events(
    mut turn_events: EventReader<TurnEndEvent>,
    mut game_state: ResMut<GameState>,
    mut simulation: ResMut<Simulation>,
) {
    for _event in turn_events.read() {
        info!("End turn requested!");
        // Process the turn directly here instead of calling the function
        game_state.process_turn();
        simulation.process_turn();
        
        // Keep the simulation in GameState in sync
        game_state.simulation.current_turn = simulation.current_turn;
        
        info!("=== Turn {} ===", simulation.current_turn);
        info!("Game date after turn: {}", game_state.get_formatted_date());
        
        // Display some sample body positions
        let bodies = game_state.solar_system.get_all_bodies();
        let sample_bodies = ["Earth", "Mars", "Jupiter", "Saturn"];
        
        for body_name in sample_bodies.iter() {
            if let Some(body) = bodies.get(*body_name) {
                if let Some(ref orbital_state) = body.orbital_state {
                    let cartesian = orbital_state.to_cartesian();
                    info!("  {}: Position ({:.0}, {:.0}) km, Angle: {:.1}°", 
                          body_name, cartesian.x, cartesian.y, orbital_state.angle_degrees());
                }
            }
        }
    }
}
