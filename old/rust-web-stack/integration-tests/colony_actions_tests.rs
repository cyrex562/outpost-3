#[cfg(feature = "server")]
use actix_web::{test, App, web};
use outpost3::http::routes;
use outpost3::db::DbPool;
use outpost3::events::EventStore;
use std::sync::Arc;
use rusqlite::Connection;

#[actix_web::test]
async fn test_allocate_labor_success() {
    let db_path = format!("/tmp/test_db_action_alloc_{}.sqlite", uuid::Uuid::new_v4());
    let pool = outpost3::db::pool::create_db_pool(&db_path);
    outpost3::db::migrations::run_migrations(&pool).expect("Failed to run migrations");
    
    let event_store = web::Data::new(EventStore::new(pool.clone()));

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level) VALUES (1, 1, 'Test Colony', '2023-01-01T00:00:00Z', 100, 100.0, 0.0)",
        [],
    ).unwrap();
    
    let housing_type = r#"{"Housing":{"capacity":100,"comfort_level":5}}"#;
    let operational_state = r#""Operational""#;
    
    conn.execute(
        "INSERT INTO buildings (building_id, colony_id, building_type, state, workers_assigned, level) VALUES (1, 1, ?1, ?2, 0, 1)",
        rusqlite::params![housing_type, operational_state],
    ).unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(event_store.clone())
            .configure(routes::configure)
    ).await;

    let req = test::TestRequest::post()
        .uri("/colony/1/building/1/allocate_labor")
        .set_form(&[("workers", "10")])
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_redirection());
    
    let workers: u32 = conn.query_row(
        "SELECT workers_assigned FROM buildings WHERE building_id = 1",
        [],
        |row| row.get(0)
    ).unwrap();

    assert_eq!(workers, 10);
    
    std::fs::remove_file(db_path).unwrap_or(());
}

#[actix_web::test]
async fn test_deallocate_labor_success() {
    let db_path = format!("/tmp/test_db_action_dealloc_{}.sqlite", uuid::Uuid::new_v4());
    let pool = outpost3::db::pool::create_db_pool(&db_path);
    outpost3::db::migrations::run_migrations(&pool).expect("Failed to run migrations");
    
    let event_store = web::Data::new(EventStore::new(pool.clone()));

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level) VALUES (1, 1, 'Test Colony', '2023-01-01T00:00:00Z', 100, 100.0, 0.0)",
        [],
    ).unwrap();
    
    let housing_type = r#"{"Housing":{"capacity":100,"comfort_level":5}}"#;
    let operational_state = r#""Operational""#;
    
    conn.execute(
        "INSERT INTO buildings (building_id, colony_id, building_type, state, workers_assigned, level) VALUES (1, 1, ?1, ?2, 20, 1)",
        rusqlite::params![housing_type, operational_state],
    ).unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(event_store.clone())
            .configure(routes::configure)
    ).await;

    let req = test::TestRequest::post()
        .uri("/colony/1/building/1/deallocate_labor")
        .set_form(&[("workers", "5")])
        .to_request();

    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_redirection());

    let workers: u32 = conn.query_row(
        "SELECT workers_assigned FROM buildings WHERE building_id = 1",
        [],
        |row| row.get(0)
    ).unwrap();

    assert_eq!(workers, 15);
    
    std::fs::remove_file(db_path).unwrap_or(());
}

#[actix_web::test]
async fn test_toggle_building_success() {
    let db_path = format!("/tmp/test_db_action_toggle_{}.sqlite", uuid::Uuid::new_v4());
    let pool = outpost3::db::pool::create_db_pool(&db_path);
    outpost3::db::migrations::run_migrations(&pool).expect("Failed to run migrations");
    
    let event_store = web::Data::new(EventStore::new(pool.clone()));

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level) VALUES (1, 1, 'Test Colony', '2023-01-01T00:00:00Z', 100, 100.0, 0.0)",
        [],
    ).unwrap();
    
    let housing_type = r#"{"Housing":{"capacity":100,"comfort_level":5}}"#;
    let operational_state = r#""Operational""#;
    
    conn.execute(
        "INSERT INTO buildings (building_id, colony_id, building_type, state, workers_assigned, level) VALUES (1, 1, ?1, ?2, 0, 1)",
        rusqlite::params![housing_type, operational_state],
    ).unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(event_store.clone())
            .configure(routes::configure)
    ).await;

    // Toggle to Shutdown
    let req = test::TestRequest::post()
        .uri("/colony/1/building/1/toggle")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_redirection());

    let state: String = conn.query_row(
        "SELECT state FROM buildings WHERE building_id = 1",
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(state, r#""Shutdown""#);

    // Toggle back to Operational
    let req2 = test::TestRequest::post()
        .uri("/colony/1/building/1/toggle")
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert!(resp2.status().is_redirection());

    let state2: String = conn.query_row(
        "SELECT state FROM buildings WHERE building_id = 1",
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(state2, r#""Operational""#);
    
    std::fs::remove_file(db_path).unwrap_or(());
}

#[actix_web::test]
async fn test_repair_building_success() {
    let db_path = format!("/tmp/test_db_action_repair_{}.sqlite", uuid::Uuid::new_v4());
    let pool = outpost3::db::pool::create_db_pool(&db_path);
    outpost3::db::migrations::run_migrations(&pool).expect("Failed to run migrations");
    
    let event_store = web::Data::new(EventStore::new(pool.clone()));

    let conn = pool.get().unwrap();
    conn.execute(
        "INSERT INTO colonies (colony_id, planet_id, name, founded_at, population, morale, pollution_level) VALUES (1, 1, 'Test Colony', '2023-01-01T00:00:00Z', 100, 100.0, 0.0)",
        [],
    ).unwrap();
    
    // Seed resources for repair (Cost is Severity * 10 Credits + Severity/10 + 1 Steel)
    // Severity 50 -> 500 Credits, 6 Steel
    conn.execute(
        "INSERT INTO resource_stockpiles (colony_id, resource_type, quantity) VALUES (1, 'Credits', 1000), (1, 'Steel', 100)",
        [],
    ).unwrap();
    
    let housing_type = r#"{"Housing":{"capacity":100,"comfort_level":5}}"#;
    let damaged_state = r#"{"Damaged":{"severity":50}}"#;
    
    conn.execute(
        "INSERT INTO buildings (building_id, colony_id, building_type, state, workers_assigned, level) VALUES (1, 1, ?1, ?2, 0, 1)",
        rusqlite::params![housing_type, damaged_state],
    ).unwrap();

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(event_store.clone())
            .configure(routes::configure)
    ).await;

    let req = test::TestRequest::post()
        .uri("/colony/1/building/1/repair")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_redirection());

    let state: String = conn.query_row(
        "SELECT state FROM buildings WHERE building_id = 1",
        [],
        |row| row.get(0)
    ).unwrap();
    assert_eq!(state, r#""Operational""#);
    
    // Verify resource consumption
    let credits: i64 = conn.query_row(
        "SELECT quantity FROM resource_stockpiles WHERE colony_id = 1 AND resource_type = 'Credits'",
        [],
        |row| row.get(0)
    ).unwrap();
    
    assert!(credits < 1000); // 1000 - 500 = 500
    
    std::fs::remove_file(db_path).unwrap_or(());
}
