use actix_files as fs;
use actix_web::{middleware, web, App, HttpServer};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use std::fs as std_fs;
use std::sync::Arc;

use outpost_server::config::AppConfig;
use outpost_server::db;
use outpost_server::db::{SqliteGameRepository, GameRepository, SharedRepository};
use outpost_server::event_store::EventStore;
use outpost_server::http;
use outpost_server::services::SimulationService;
use outpost_core::domain::{GameState, StarSystem, CelestialBody, BodyType, Atmosphere, Temperature};
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
    db::migrations::run_migrations(&db_pool).expect("Failed to run legacy migrations");

    // Run V5 schema migrations
    {
        let conn = db_pool.get().expect("Failed to get DB connection for V5 migrations");
        db::v5_migrations::run_v5_migrations(&conn).expect("Failed to run V5 migrations");
    }

    // Build shared repository
    let repo: SharedRepository = Arc::new(SqliteGameRepository::new(db_pool.clone()));

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

    // Load gameplay event definitions
    if let Ok(yaml_content) = std_fs::read_to_string("content/events/narrative_events.yaml") {
        match content_loader.load_events(&yaml_content) {
            Ok(_) => info!("Loaded event definitions"),
            Err(e) => warn!("Failed to load events: {}", e),
        }
    }
    
    let (building_count, resource_count, event_count, tech_count, recipe_count) = content_loader.stats();
    info!("Content loaded: {} buildings, {} resources, {} events, {} techs, {} recipes", 
          building_count, resource_count, event_count, tech_count, recipe_count);

    // Initialize simulation service with content
    let mut initial_game_state = GameState::new("Outpost 3".to_string(), 42);
    initial_game_state.load_from_content_loader(&content_loader);

    // Sync content definitions from YAML into the DB (does not overwrite 'db'-sourced records).
    info!("Syncing content definitions to database...");
    for def in content_loader.all_building_defs() {
        if let Err(e) = repo.upsert_building_def(def, "yaml") {
            warn!("Failed to sync building def '{}': {}", def.id, e);
        }
    }
    for def in content_loader.all_resource_defs() {
        if let Err(e) = repo.upsert_resource_def(def, "yaml") {
            warn!("Failed to sync resource def '{}': {}", def.id, e);
        }
    }
    for def in content_loader.all_event_defs() {
        if let Err(e) = repo.upsert_event_def(def, "yaml") {
            warn!("Failed to sync event def '{}': {}", def.id, e);
        }
    }
    for recipe in content_loader.all_recipes().values() {
        if let Err(e) = repo.upsert_recipe(recipe, "yaml") {
            warn!("Failed to sync recipe '{}': {}", recipe.id, e);
        }
    }
    info!("Content sync complete");

    // Try loading saved game state from DB; fall back to seeding fresh data.
    let loaded_from_db = match repo.load_game_state() {
        Ok(Some(mut saved_state)) => {
            // Re-attach content definitions (they are #[serde(skip)]).
            saved_state.load_from_content_loader(&content_loader);
            info!("Resumed game from DB at tick {}", saved_state.clock.current_tick);
            initial_game_state = saved_state;
            true
        }
        Ok(None) => {
            info!("No saved game found — seeding fresh development data");
            false
        }
        Err(e) => {
            warn!("Failed to load saved game state: {} — seeding fresh data", e);
            false
        }
    };

    if !loaded_from_db {
        seed_dev_data(&mut initial_game_state);
        // Persist the initial seed so it survives restart.
        if let Err(e) = repo.save_game_state(&initial_game_state) {
            warn!("Failed to persist initial game state: {}", e);
        }
    }

    let simulation_service = web::Data::new(
        SimulationService::with_repo(initial_game_state, content_loader, repo.clone()),
    );

    // Start the background tick loop (shares Arc state with the service above)
    simulation_service.as_ref().clone().start_tick_loop();

    // Make the repository available to handlers via Actix Data.
    // Use web::Data::new so the type is Data<SharedRepository> (= Data<Arc<dyn GameRepository>>),
    // matching what handlers declare in their parameters.
    let repo_data = web::Data::new(repo);

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
            .app_data(repo_data.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            // Static files must be registered before the catch-all scope("") in routes
            .service(fs::Files::new("/static", "crates/outpost-server/static"))
            .configure(http::routes::configure)
    })
    .bind(&bind_address)?
    .run()
    .await
}

/// Populate game state with development/test data so there's something to browse on first run.
/// Uses fixed UUIDs so that IDs are stable across server restarts.
fn seed_dev_data(state: &mut GameState) {
    use outpost_core::domain::{CelestialBodyId, Site, SiteId};
    use uuid::Uuid;

    macro_rules! fixed_uuid { ($s:expr) => { Uuid::parse_str($s).unwrap() } }

    let galaxy_id = state.galaxy.id;

    // ── Sol System ─────────────────────────────────────────────────────────
    let mut sol = StarSystem::new(galaxy_id, "Sol".to_string(), 1001);
    sol.id = outpost_core::domain::StarSystemId(fixed_uuid!("10000000-0000-0000-0000-000000000001"));
    sol.explored = true;

    let mut earth = CelestialBody::new(
        sol.id, "Earth".to_string(), BodyType::TerrestrialPlanet,
        Atmosphere::Breathable { composition: std::collections::HashMap::new() },
        Temperature::Temperate, 1.0, 6371.0, 1.0,
    );
    earth.id = CelestialBodyId(fixed_uuid!("20000000-0000-0000-0000-000000000001"));
    let earth_id = earth.id;
    sol.add_body(earth);

    let mut mars = CelestialBody::new(
        sol.id, "Mars".to_string(), BodyType::TerrestrialPlanet,
        Atmosphere::Thin { composition: std::collections::HashMap::new() },
        Temperature::Cold, 0.38, 3389.5, 0.107,
    );
    mars.id = CelestialBodyId(fixed_uuid!("20000000-0000-0000-0000-000000000002"));
    let mars_id = mars.id;
    sol.add_body(mars);

    let mut jupiter = CelestialBody::new(
        sol.id, "Jupiter".to_string(), BodyType::GasGiant,
        Atmosphere::Crushing { composition: std::collections::HashMap::new() },
        Temperature::Frozen, 2.53, 71492.0, 317.8,
    );
    jupiter.id = CelestialBodyId(fixed_uuid!("20000000-0000-0000-0000-000000000003"));
    sol.add_body(jupiter);

    let mut earth_base = Site::new_settlement(earth_id, "New Earth Base".to_string(), 0, 250);
    earth_base.id = SiteId(fixed_uuid!("30000000-0000-0000-0000-000000000001"));
    sol.add_site(earth_base);

    let mut ares_station = Site::new_installation(mars_id, "Ares Mining Station".to_string(), 10, 45);
    ares_station.id = SiteId(fixed_uuid!("30000000-0000-0000-0000-000000000002"));
    sol.add_site(ares_station);

    state.galaxy.add_system(sol);

    // ── Alpha Centauri ─────────────────────────────────────────────────────
    let mut alpha_cen = StarSystem::new(galaxy_id, "Alpha Centauri".to_string(), 2002);
    alpha_cen.id = outpost_core::domain::StarSystemId(fixed_uuid!("10000000-0000-0000-0000-000000000002"));
    alpha_cen.explored = true;

    let mut proxima_b = CelestialBody::new(
        alpha_cen.id, "Proxima b".to_string(), BodyType::TerrestrialPlanet,
        Atmosphere::Dense { composition: std::collections::HashMap::new() },
        Temperature::Hot, 1.1, 7000.0, 1.3,
    );
    proxima_b.id = CelestialBodyId(fixed_uuid!("20000000-0000-0000-0000-000000000004"));
    alpha_cen.add_body(proxima_b);

    let mut proxima_c = CelestialBody::new(
        alpha_cen.id, "Proxima c".to_string(), BodyType::Moon,
        Atmosphere::Trace, Temperature::Cold, 0.5, 2000.0, 0.3,
    );
    proxima_c.id = CelestialBodyId(fixed_uuid!("20000000-0000-0000-0000-000000000005"));
    alpha_cen.add_body(proxima_c);

    state.galaxy.add_system(alpha_cen);

    // ── Sirius (unexplored) ────────────────────────────────────────────────
    let mut sirius = StarSystem::new(galaxy_id, "Sirius".to_string(), 3003);
    sirius.id = outpost_core::domain::StarSystemId(fixed_uuid!("10000000-0000-0000-0000-000000000003"));

    let mut sirius_a1 = CelestialBody::new(
        sirius.id, "Sirius A-I".to_string(), BodyType::TerrestrialPlanet,
        Atmosphere::Toxic { composition: std::collections::HashMap::new() },
        Temperature::Hot, 0.9, 5500.0, 0.8,
    );
    sirius_a1.id = CelestialBodyId(fixed_uuid!("20000000-0000-0000-0000-000000000006"));
    sirius.add_body(sirius_a1);

    state.galaxy.add_system(sirius);

    tracing::info!("Dev data seeded: 3 systems, 6 bodies, 2 sites (stable UUIDs)");
}
