use bevy::prelude::*;
use log::info;
use crate::components::celestial_body::{CelestialBody, CelestialBodyRenderable, PreviousPosition};
use crate::components::orbital_mechanics::OrbitalState;
use crate::resources::game_state::GameState;

/// System to update celestial body positions based on orbital mechanics
pub fn update_celestial_body_positions(
    mut query: Query<(&mut CelestialBody, &mut Transform, Option<&mut PreviousPosition>)>,
    time: Res<Time>,
) {
    for (mut celestial_body, mut transform, mut previous_position) in query.iter_mut() {
        if let Some(ref mut orbital_state) = celestial_body.orbital_state {
            // Update orbital position (assuming 30 days per turn, scaled by time)
            let days_elapsed = 30.0 * time.delta_seconds() / 1.0; // 1 second = 30 days
            orbital_state.update_position(days_elapsed as f64);
            
            // Convert to Cartesian coordinates for rendering
            let cartesian = orbital_state.to_cartesian();
            
            // Scale down for rendering (convert km to game units)
            let scale_factor = 1e-6; // 1 million km = 1 game unit
            transform.translation.x = (cartesian.x * scale_factor) as f32;
            transform.translation.y = (cartesian.y * scale_factor) as f32;
            
            // Update previous position for change detection
            if let Some(ref mut prev_pos) = previous_position {
                prev_pos.angle = orbital_state.current_position.angle;
            }
        }
    }
}

/// System to update the game state's solar system positions
pub fn update_game_state_positions(
    mut game_state: ResMut<GameState>,
    time: Res<Time>,
) {
    // Update positions in the game state (30 days per turn)
    let days_elapsed = 30.0 * time.delta_seconds() / 1.0; // 1 second = 30 days
    game_state.solar_system.update_all_positions(days_elapsed as f64);
}

/// System to log significant position changes
pub fn log_position_changes(
    query: Query<(&CelestialBody, &PreviousPosition), Changed<PreviousPosition>>,
) {
    for (celestial_body, previous_position) in query.iter() {
        if let Some(ref orbital_state) = celestial_body.orbital_state {
            if orbital_state.is_significant_change(previous_position.angle, 1.0) {
                let cartesian = orbital_state.to_cartesian();
                info!("Significant position change for {}: Position ({:.0}, {:.0}) km, Angle: {:.1}°", 
                      celestial_body.name, cartesian.x, cartesian.y, orbital_state.angle_degrees());
            }
        }
    }
}
