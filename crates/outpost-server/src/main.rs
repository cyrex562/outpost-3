use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use outpost_server::config::AppConfig;
use outpost_server::db;
use outpost_server::event_store::EventStore;
use outpost_server::http;
use outpost_server::services::SimulationService;
use outpost_core::domain::GameState;

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

    // Initialize simulation service
    let initial_game_state = GameState::new("Outpost 3".to_string(), 42);
    let simulation_service = web::Data::new(SimulationService::new(initial_game_state));

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
