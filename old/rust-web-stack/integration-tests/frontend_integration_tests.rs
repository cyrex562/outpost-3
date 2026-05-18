#[cfg(feature = "server")]
#[actix_web::test]
async fn test_starmap_returns_success() {
    use actix_web::{test, web, App};
    use outpost3::http::handlers;
    
    // Setup Tera
    let tera = tera::Tera::new("templates/**/*.html").unwrap();
    
    // Setup DB (Using a temporary file to ensure pool connections share the same DB)
    // In a real scenario, we'd use a tempfile crate or similar
    let db_path = format!("/tmp/test_db_{}.sqlite", uuid::Uuid::new_v4());
    let pool = outpost3::db::pool::create_db_pool(&db_path);
    
    // Run migrations
    outpost3::db::migrations::run_migrations(&pool).expect("Failed to run migrations");
    
    // Seed Data
    {
        let conn = pool.get().expect("Failed to get conn");
        conn.execute(
            "INSERT INTO star_systems (id, name, spectral_class, distance_from_sol, discovery_level) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params!["system_1", "Alpha Test", "G2V", 4.3, "Scanned"],
        ).expect("Failed to insert system");
    }

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(tera))
            .route("/map", web::get().to(handlers::view_starmap))
    ).await;

    let req = test::TestRequest::get().uri("/map").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    
    // Assertions for new UI elements
    assert!(body_str.contains("Stellar Cartography Database"));
    assert!(body_str.contains("Alpha Test"));
    assert!(body_str.contains("G2V")); // Validation of data present
    assert!(body_str.contains("data-table")); // Validation of CSS class
    
    // Cleanup
    std::fs::remove_file(db_path).unwrap_or(());
}

#[cfg(feature = "server")]
#[actix_web::test]
async fn test_colony_dashboard_returns_success() {
    use actix_web::{test, web, App};
    use outpost3::http::handlers;

    let tera = tera::Tera::new("templates/**/*.html").unwrap();
    let db_path = format!("/tmp/test_db_colony_{}.sqlite", uuid::Uuid::new_v4());
    let pool = outpost3::db::pool::create_db_pool(&db_path);
    
    outpost3::db::migrations::run_migrations(&pool).expect("Failed to run migrations");

    // Seed Colony
    {
        let conn = pool.get().expect("Failed to get conn");
        conn.execute(
            "INSERT INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
             rusqlite::params![1, 1, "Test Colony", "2026-01-01T00:00:00Z", 100, 80.0, 0.0],
        ).expect("Failed to seed colony");
        
        // Seed Building
        conn.execute(
             "INSERT INTO buildings (building_id, colony_id, building_type, state, workers_assigned, level) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
             rusqlite::params![1, 1, r#"{"Mine":{"resource_type":"IronOre","output_rate":10}}"#, r#""Operational""#, 5, 1],
        ).expect("Failed to seed building");
    }

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(tera))
            .route("/colony/{id}", web::get().to(handlers::view_colony))
    ).await;

    let req = test::TestRequest::get().uri("/colony/1").to_request();
    let resp = test::call_service(&app, req).await;
    
    assert!(resp.status().is_success());
    
    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).unwrap();
    
    // Assertions
    assert!(body_str.contains("Test Colony"));
    assert!(body_str.contains("Infrastructure")); // Tab check
    assert!(body_str.contains("Resource Stockpile")); // Tab check
    assert!(body_str.contains("Mine (IronOre)")); // Building check
    
    std::fs::remove_file(db_path).unwrap_or(());
}
