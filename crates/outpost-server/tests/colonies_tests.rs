// Integration tests for colonies list endpoint (Task 2.4)

use actix_web::{test, web, App};
use outpost_core::domain::GameState;
use outpost_core::content::ContentLoader;
use outpost_server::http::routes;
use outpost_server::services::SimulationService;

/// Helper: Create a test GameState with building definitions loaded
fn create_test_game_state() -> GameState {
    let mut game_state = GameState::new("Test Galaxy".into(), 12345);
    
    // Load building definitions (relative to crates/outpost-server/)
    let mut content_loader = ContentLoader::new();
    if let Ok(yaml_content) = std::fs::read_to_string("../../content/buildings/basic_buildings.yaml") {
        content_loader.load_buildings(&yaml_content).expect("Load buildings");
    }
    if let Ok(yaml_content) = std::fs::read_to_string("../../content/resources/basic_resources.yaml") {
        content_loader.load_resources(&yaml_content).expect("Load resources");
    }
    
    game_state.building_definitions = content_loader.all_buildings().clone();
    game_state.resource_definitions = content_loader.all_resources().clone();
    game_state
}

#[actix_web::test]
async fn test_colonies_list_renders_with_no_sites() {
    let game_state = create_test_game_state();
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("templates/**/*.html")
            .expect("Failed to initialize Tera")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/colonies")
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Debug: Print response body if not 200
    let status = resp.status();
    if status != 200 {
        let body = test::read_body(resp).await;
        let body_str = std::str::from_utf8(&body).expect("Body should be valid UTF-8");
        eprintln!("Response body: {}", body_str);
        panic!("Expected 200, got {}", status);
    }
    
    assert_eq!(status, 200, "Should return 200 OK");
    
    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).expect("Body should be valid UTF-8");
    
    // Verify page title
    assert!(body_str.contains("Colonies"), "Should contain 'Colonies' in title");
    
    // Verify empty state message
    assert!(body_str.contains("No colonies founded yet"), "Should show empty state message");
}

#[actix_web::test]
async fn test_colonies_list_shows_summary_stats() {
    let game_state = create_test_game_state();
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("templates/**/*.html")
            .expect("Failed to initialize Tera")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/colonies")
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    assert_eq!(resp.status(), 200);
    
    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).expect("Body should be valid UTF-8");
    
    // Verify summary stats section exists
    assert!(body_str.contains("Total Sites"), "Should have 'Total Sites' stat");
    assert!(body_str.contains("Total Population"), "Should have 'Total Population' stat");
    assert!(body_str.contains("Average Morale"), "Should have 'Average Morale' stat");
    assert!(body_str.contains("Total Buildings"), "Should have 'Total Buildings' stat");
}

#[actix_web::test]
async fn test_colonies_list_has_table_headers() {
    let game_state = create_test_game_state();
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("templates/**/*.html")
            .expect("Failed to initialize Tera")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/colonies")
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    assert_eq!(resp.status(), 200);
    
    let body = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body).expect("Body should be valid UTF-8");
    
    // Verify table has correct headers
    assert!(body_str.contains("Name"), "Table should have 'Name' column");
    assert!(body_str.contains("Type"), "Table should have 'Type' column");
    assert!(body_str.contains("Location"), "Table should have 'Location' column");
    assert!(body_str.contains("Population"), "Table should have 'Population' column");
    assert!(body_str.contains("Morale"), "Table should have 'Morale' column");
    assert!(body_str.contains("Power"), "Table should have 'Power' column");
    assert!(body_str.contains("Buildings"), "Table should have 'Buildings' column");
    assert!(body_str.contains("Status"), "Table should have 'Status' column");
}

#[actix_web::test]
async fn test_colonies_route_is_registered() {
    let game_state = create_test_game_state();
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("templates/**/*.html")
            .expect("Failed to initialize Tera")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri("/colonies")
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Should not get 404 - route should be registered
    assert_ne!(resp.status(), 404, "Route /colonies should be registered");
    assert_eq!(resp.status(), 200, "Route /colonies should return 200 OK");
}
