use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use std::fs as std_fs;

use outpost_server::config::AppConfig;
use outpost_server::db;
use outpost_server::event_store::EventStore;
use outpost_server::http;
use outpost_server::services::SimulationService;
use outpost_core::domain::GameState;
use outpost_core::content::ContentLoader;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("Starting Outpost 3 server...");

    // Load configuration
    let config = AppConfig::load().expect("Failed to load configuration");
    let bind_address = format!("{}:{}", config.server.host, config.server.port);

    info!("Binding to: {}", bind_address);

    // Initialize database
    let db_pool = db::pool::create_db_pool(&config.database.path);
    db::migrations::run_migrations(&db_pool).expect("Failed to run migrations");

    // Initialize event store
    let event_store = web::Data::new(EventStore::new(db_pool.clone()));

    // Load content definitions from YAML files
    info!("Loading content definitions...");
    let mut content_loader = ContentLoader::new();
    
    // Load buildings
    if let Ok(yaml_content) = std_fs::read_to_string("content/buildings/basic_buildings.yaml") {
        match content_loader.load_buildings(&yaml_content) {
            Ok(_) => info!("Loaded building definitions"),
            Err(e) => warn!("Failed to load buildings: {}", e),
        }
    }
    
    // Load resources
    if let Ok(yaml_content) = std_fs::read_to_string("content/resources/basic_resources.yaml") {
        match content_loader.load_resources(&yaml_content) {
            Ok(_) => info!("Loaded resource definitions"),
            Err(e) => warn!("Failed to load resources: {}", e),
        }
    }
    
    // Load recipes
    if let Ok(yaml_content) = std_fs::read_to_string("content/recipes.yaml") {
        match content_loader.load_recipes(&yaml_content) {
            Ok(_) => info!("Loaded recipe definitions"),
            Err(e) => warn!("Failed to load recipes: {}", e),
        }
    }
    
    let (building_count, resource_count, event_count, tech_count, recipe_count) = content_loader.stats();
    info!("Content loaded: {} buildings, {} resources, {} events, {} techs, {} recipes", 
          building_count, resource_count, event_count, tech_count, recipe_count);

    // Initialize simulation service with content
    let mut initial_game_state = GameState::new("Outpost 3".to_string(), 42);
    initial_game_state.load_from_content_loader(&content_loader);
    let simulation_service = web::Data::new(SimulationService::new(initial_game_state, content_loader));

    // Initialize Tera templates
    let tera = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to initialize Tera"),
    );

    info!("Server initialized successfully");

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(event_store.clone())
            .app_data(simulation_service.clone())
            .app_data(tera.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .configure(http::routes::configure)
            // Serve static CSS/JS files
            .service(fs::Files::new("/static", "crates/outpost-server/static").show_files_listing())
    })
    .bind(&bind_address)?
    .run()
    .await
}
