use bevy::prelude::*;
use log::{info, error, warn};
use crate::resources::game_state::GameState;
use crate::components::celestial_body::{CelestialBodyRenderable, CelestialBodyType, PreviousPosition};
use crate::components::simulation::Simulation;

/// System to initialize the game state
pub fn initialize_game_state(mut commands: Commands) {
    info!("Initializing game state...");
    
    let mut game_state = GameState::new();
    
    // Load solar system data
    match game_state.load_solar_system_data("data/solar_system_data.csv") {
        Ok(()) => {
            info!("Successfully loaded solar system data");
        }
        Err(e) => {
            error!("Failed to load solar system data: {}", e);
            // Continue with empty solar system rather than crashing
        }
    }
    
    commands.insert_resource(game_state);
    
    // Also insert the simulation as a separate resource for systems that need it
    let simulation = Simulation::new();
    commands.insert_resource(simulation);
    
    info!("Game state initialized successfully");
}

/// System to set up the camera
pub fn setup_camera(mut commands: Commands) {
    // Add a 3D camera for the solar system view
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 100.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    
    // Add a 2D camera for the UI
    commands.spawn(Camera2dBundle::default());
}

/// System to spawn celestial body entities from the game state
pub fn spawn_celestial_bodies(
    mut commands: Commands,
    game_state: Res<GameState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("Spawning celestial body entities...");
    
    let bodies = game_state.solar_system.get_all_bodies();
    if bodies.is_empty() {
        warn!("No celestial bodies found to spawn");
        return;
    }
    
    for (name, celestial_body) in bodies {
        let mut entity_commands = commands.spawn((
            celestial_body.clone(),
            CelestialBodyRenderable,
            Transform::from_xyz(0.0, 0.0, 0.0),
            GlobalTransform::default(),
        ));
        
        // Add orbital state as a separate component if it exists
        if let Some(orbital_state) = &celestial_body.orbital_state {
            entity_commands.insert(orbital_state.clone());
            
            // Add previous position tracking
            if let Some(previous_angle) = game_state.solar_system.previous_positions.get(name) {
                entity_commands.insert(PreviousPosition { angle: *previous_angle });
            }
        }
        
        // Add visual representation based on body type
        match celestial_body.body_type {
            CelestialBodyType::Star => {
                entity_commands.insert((
                    meshes.add(Sphere::new(10.0)),
                    materials.add(StandardMaterial {
                        base_color: Color::rgb(1.0, 1.0, 0.8), // Light yellow
                        emissive: Color::rgb(0.5, 0.5, 0.4),
                        ..default()
                    }),
                ));
            }
            CelestialBodyType::Planet => {
                entity_commands.insert((
                    meshes.add(Sphere::new(2.0)),
                    materials.add(StandardMaterial {
                        base_color: Color::rgb(0.4, 0.6, 1.0), // Blue
                        ..default()
                    }),
                ));
            }
            CelestialBodyType::Moon => {
                entity_commands.insert((
                    meshes.add(Sphere::new(0.5)),
                    materials.add(StandardMaterial {
                        base_color: Color::rgb(0.7, 0.7, 0.7), // Gray
                        ..default()
                    }),
                ));
            }
            CelestialBodyType::Asteroid => {
                entity_commands.insert((
                    meshes.add(Sphere::new(0.2)),
                    materials.add(StandardMaterial {
                        base_color: Color::rgb(0.6, 0.4, 0.2), // Brown
                        ..default()
                    }),
                ));
            }
            CelestialBodyType::Comet => {
                entity_commands.insert((
                    meshes.add(Sphere::new(0.3)),
                    materials.add(StandardMaterial {
                        base_color: Color::rgb(0.8, 0.8, 0.9), // Light blue
                        ..default()
                    }),
                ));
            }
            CelestialBodyType::DwarfPlanet => {
                entity_commands.insert((
                    meshes.add(Sphere::new(1.0)),
                    materials.add(StandardMaterial {
                        base_color: Color::rgb(0.5, 0.3, 0.1), // Dark brown
                        ..default()
                    }),
                ));
            }
        }
    }
    
    info!("Celestial body entities spawned");
}
