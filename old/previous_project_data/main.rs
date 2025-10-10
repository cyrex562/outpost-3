use env_logger::{Builder, Env};
use log::info;

mod buildings;
mod events;
mod faction;
mod game;
mod game_state;
mod maps;
mod population;
mod procedural_generation;
mod production;
mod resources;
mod simulation;
mod structures;
mod units;
mod universe;
mod ui;

fn main() {
    println!("Starting Harsh Realm...");
    
    // Initialize the logger
    Builder::from_env(Env::default().default_filter_or("info")).init();
    info!("Starting Harsh Realm prototype simulation.");
    
    println!("Creating game state...");
    // Create a new game state
    let mut game_state = game_state::GameState::new();
    
    println!("Loading solar system data...");
    // Load solar system data from CSV
    match game_state.load_solar_system_data("data/solar_system_data.csv") {
        Ok(()) => {
            println!("Successfully loaded solar system data");
            info!("Successfully loaded solar system data");
        }
        Err(e) => {
            eprintln!("Failed to load solar system data: {}", e);
            return;
        }
    }
    
    info!("Game state created successfully");
    info!("Initial game date: {}", game_state.get_formatted_date());
    
    println!("Initializing UI...");
    // Initialize UI
    let mut ui = match ui::UI::new() {
        Ok(ui) => {
            println!("UI initialized successfully");
            info!("UI initialized successfully");
            ui
        }
        Err(e) => {
            eprintln!("Failed to initialize UI: {}", e);
            return;
        }
    };
    
    println!("Getting event pump...");
    // Get event pump for handling input
    let sdl_context = match sdl2::init() {
        Ok(context) => {
            println!("SDL2 initialized successfully");
            context
        }
        Err(e) => {
            eprintln!("Failed to initialize SDL2: {}", e);
            return;
        }
    };
    
    let mut event_pump = match sdl_context.event_pump() {
        Ok(pump) => {
            println!("Event pump created successfully");
            pump
        }
        Err(e) => {
            eprintln!("Failed to get event pump: {}", e);
            return;
        }
    };
    
    info!("Starting game loop...");
    println!("Starting game loop...");
    
    // Main game loop
    'game_loop: loop {
        // Handle events
        match ui.handle_events(&mut event_pump) {
            Ok(ui_event) => {
                match ui_event {
                    ui::UIEvent::Quit => {
                        println!("Quit requested");
                        info!("Quit requested");
                        break 'game_loop;
                    }
                    ui::UIEvent::EndTurn => {
                        println!("End turn clicked!");
                        info!("=== Turn {} ===", game_state.simulation.current_turn + 1);
                        game_state.process_turn();
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
                    ui::UIEvent::Continue => {
                        // Continue with normal game loop
                    }
                }
            }
            Err(e) => {
                eprintln!("Error handling events: {}", e);
                break 'game_loop;
            }
        }
        
        // Render the frame
        if let Err(e) = ui.render(&game_state) {
            eprintln!("Error rendering frame: {}", e);
            break 'game_loop;
        }
        
        // Cap frame rate
        std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
    }
    
    info!("Game loop ended.");
    println!("Game loop ended.");
}
