use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod domain;
mod events;
mod commands;
mod queries;
mod services;
mod web as web_layer;
mod db;
mod simulation;
mod utils;

use crate::config::AppConfig;
use crate::db::pool::create_db_pool;
use crate::events::store::EventStore;

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
    let db_pool = create_db_pool(&config.database.path);
    db::migrations::run_migrations(&db_pool).expect("Failed to run migrations");

    // Initialize event store
    let event_store = web::Data::new(EventStore::new(db_pool.clone()));

    // Initialize Tera templates
    let tera = web::Data::new(
        tera::Tera::new("templates/**/*.html").expect("Failed to initialize Tera"),
    );

    info!("Server initialized successfully");

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(event_store.clone())
            .app_data(tera.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .configure(web_layer::routes::configure)
            .service(fs::Files::new("/static", "./static").show_files_listing())
    })
    .bind(&bind_address)?
    .run()
    .await
}
