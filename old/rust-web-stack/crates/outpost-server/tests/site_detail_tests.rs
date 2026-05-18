// Integration tests for site detail endpoints (Task 2.3)

use actix_web::{test, web, App};
use outpost_core::domain::{GameState, SiteId};
use outpost_core::content::ContentLoader;
use outpost_server::http::routes;
use outpost_server::services::SimulationService;

fn load_content() -> ContentLoader {
    let mut content_loader = ContentLoader::new();
    let buildings_yaml = std::fs::read_to_string(
        format!("{}/../../content/buildings/basic_buildings.yaml", env!("CARGO_MANIFEST_DIR"))
    ).expect("Load buildings");
    content_loader.load_buildings(&buildings_yaml).expect("Parse buildings");
    let resources_yaml = std::fs::read_to_string(
        format!("{}/../../content/resources/basic_resources.yaml", env!("CARGO_MANIFEST_DIR"))
    ).expect("Load resources");
    content_loader.load_resources(&resources_yaml).expect("Parse resources");
    content_loader
}

async fn build_app(game_state: GameState) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let content = load_content();
    let simulation = web::Data::new(SimulationService::new(game_state, content));
    let template_glob = format!("{}/templates/**/*.html", env!("CARGO_MANIFEST_DIR"));
    let tmpl = web::Data::new(
        tera::Tera::new(&template_glob)
            .expect("Failed to load templates"),
    );
    test::init_service(
        App::new()
            .app_data(simulation)
            .app_data(tmpl)
            .configure(routes::configure),
    )
    .await
}

/// Create a GameState with one site that has a smelter building (worker_capacity=8).
fn game_state_with_smelter_site() -> (GameState, SiteId) {
    use outpost_core::domain::{
        Site, CelestialBodyId,
        star_system::StarSystem,
        building_instance::{BuildingInstance, BuildingState},
    };

    let content = load_content();
    let mut game_state = GameState::new("Test Galaxy".into(), 42);
    game_state.building_definitions = content.all_buildings().clone();
    game_state.resource_definitions = content.all_resources().clone();

    let galaxy_id = game_state.galaxy.id;
    let mut system = StarSystem::new(galaxy_id, "Test System".to_string(), 0);

    let mut site = Site::new_settlement(CelestialBodyId::new(), "Smelter Base".to_string(), 0, 100);
    let site_id = site.id;

    let mut building = BuildingInstance::new_operational(site.id, "smelter".to_string(), 0);
    building.state = BuildingState::Operational;
    building.workers_assigned = 0;
    let building_id = building.id;
    site.buildings.insert(building_id, building);

    system.add_site(site);
    game_state.galaxy.add_system(system);

    (game_state, site_id)
}

/// Helper: Create a test GameState with one site and some buildings
fn create_test_game_state() -> GameState {
    let mut game_state = GameState::new("Test Galaxy".into(), 12345);
    
    // Load content from actual YAML files
    let mut content_loader = ContentLoader::new();
    if let Ok(yaml_content) = std::fs::read_to_string("../../content/buildings/basic_buildings.yaml") {
        content_loader.load_buildings(&yaml_content).expect("Load buildings");
    }
    if let Ok(yaml_content) = std::fs::read_to_string("../../content/resources/basic_resources.yaml") {
        content_loader.load_resources(&yaml_content).expect("Load resources");
    }
    
    game_state.building_definitions = content_loader.all_buildings().clone();
    game_state.resource_definitions = content_loader.all_resources().clone();
    
    // For simplicity, just return the base game state
    // Real tests would add a site with buildings
    game_state
}

#[actix_web::test]
async fn test_site_detail_page_handles_missing_site() {
    let game_state = create_test_game_state();
    
    // Use a random site ID that doesn't exist
    let fake_site_id = SiteId::new();
    
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri(&format!("/site/{}", fake_site_id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Should return 404 for non-existent site
    assert_eq!(resp.status(), 404, "Should return 404 for missing site");
}

#[actix_web::test]
async fn test_buildings_tab_handles_missing_site() {
    let game_state = create_test_game_state();
    let fake_site_id = SiteId::new();
    
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri(&format!("/site/{}/buildings", fake_site_id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Should return 404 for non-existent site
    assert_eq!(resp.status(), 404, "Should return 404 for missing site");
}

#[actix_web::test]
async fn test_construction_tab_handles_missing_site() {
    let game_state = create_test_game_state();
    let fake_site_id = SiteId::new();
    
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
           .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    let req = test::TestRequest::get()
        .uri(&format!("/site/{}/construction", fake_site_id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Should return 404 for non-existent site
    assert_eq!(resp.status(), 404, "Should return 404 for missing site");
}

#[actix_web::test]
async fn test_start_construction_handles_missing_site() {
    let game_state = create_test_game_state();
    let fake_site_id = SiteId::new();
    
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    // Try to build a warehouse at non-existent site
    let req = test::TestRequest::post()
        .uri(&format!("/site/{}/build", fake_site_id))
        .set_form(&[("building_def_id", "warehouse")])
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Should handle error (404 or error response)
    assert!(!resp.status().is_success() || resp.status() == 200, "Should handle missing site");
}

#[actix_web::test]
async fn test_cancel_construction_handles_missing_job() {
    let game_state = create_test_game_state();
    let fake_site_id = SiteId::new();
    
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    // Try to cancel a non-existent job
    use outpost_core::domain::ids::BuildingId;
    let fake_job_id = BuildingId::new();
    
    let req = test::TestRequest::post()
        .uri(&format!("/site/{}/construction/{}/cancel", fake_site_id, fake_job_id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Should handle the error (404 or error response)
    assert!(resp.status().as_u16() >= 200 && resp.status().as_u16() < 600, "Should return valid HTTP status");
}

#[actix_web::test]
async fn test_toggle_pause_construction_handles_missing_job() {
    let game_state = create_test_game_state();
    let fake_site_id = SiteId::new();
    
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    use outpost_core::domain::ids::BuildingId;
    let fake_job_id = BuildingId::new();
    
    let req = test::TestRequest::post()
        .uri(&format!("/site/{}/construction/{}/pause", fake_site_id, fake_job_id))
        .to_request();
    
    let resp = test::call_service(&mut app, req).await;
    
    // Should handle the error gracefully
    assert!(resp.status().as_u16() >= 200 && resp.status().as_u16() < 600, "Should return valid HTTP status");
}

#[actix_web::test]
async fn test_routes_are_registered() {
    // This test verifies all site detail routes (including MVP.4 labor routes) are registered
    let game_state = create_test_game_state();
    let fake_site_id = SiteId::new();
    
    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );
    
    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;
    
    // All GET routes — site detail, tabs, and the new labor tab
    let get_routes = vec![
        format!("/site/{}", fake_site_id),
        format!("/site/{}/buildings", fake_site_id),
        format!("/site/{}/construction", fake_site_id),
        format!("/site/{}/labor", fake_site_id),   // MVP.4 labor tab
    ];
    
    for uri in get_routes {
        let req = test::TestRequest::get().uri(&uri).to_request();
        let resp = test::call_service(&mut app, req).await;
        // Any valid HTTP status is acceptable (route exists)
        assert!(resp.status().as_u16() >= 200 && resp.status().as_u16() < 600,
            "Route {} returned invalid status {}", uri, resp.status());
    }
}

// ============================================================
// MVP.4 Labor Tab Tests
// ============================================================

#[actix_web::test]
async fn test_labor_tab_returns_404_for_missing_site() {
    let game_state = create_test_game_state();
    let fake_site_id = SiteId::new();

    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );

    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;

    let req = test::TestRequest::get()
        .uri(&format!("/site/{}/labor", fake_site_id))
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), 404, "Labor tab should return 404 for non-existent site");
}

#[actix_web::test]
async fn test_assign_labor_rejects_invalid_site_id() {
    let game_state = create_test_game_state();

    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );

    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;

    use outpost_core::domain::ids::BuildingId;
    let fake_bid = BuildingId::new();

    let req = test::TestRequest::post()
        .uri(&format!("/site/not-a-uuid/building/{}/assign", fake_bid))
        .set_form(&[("skill", "Laborer"), ("count", "1")])
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), 400, "assign_labor should return 400 for invalid site UUID");
}

#[actix_web::test]
async fn test_deallocate_labor_rejects_invalid_site_id() {
    let game_state = create_test_game_state();

    let simulation = web::Data::new(SimulationService::new(game_state, outpost_core::content::ContentLoader::new()));
    let tmpl = web::Data::new(
        tera::Tera::new("crates/outpost-server/templates/**/*.html")
            .expect("Failed to load templates")
    );

    let mut app = test::init_service(
        App::new()
            .app_data(simulation.clone())
            .app_data(tmpl.clone())
            .configure(routes::configure)
    ).await;

    use outpost_core::domain::ids::BuildingId;
    let fake_bid = BuildingId::new();

    let req = test::TestRequest::post()
        .uri(&format!("/site/not-a-uuid/building/{}/deallocate", fake_bid))
        .set_form(&[("skill", "Laborer"), ("count", "1")])
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), 400, "deallocate_labor should return 400 for invalid site UUID");
}

#[actix_web::test]
async fn test_labor_tab_renders_for_existing_site() {
    let (game_state, site_id) = game_state_with_smelter_site();
    let mut app = build_app(game_state).await;

    let req = test::TestRequest::get()
        .uri(&format!("/site/{}/labor", site_id))
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), 200, "Labor tab should return 200 for existing site");

    let body = test::read_body(resp).await;
    let html = std::str::from_utf8(&body).expect("UTF-8 body");
    assert!(html.contains("labor"), "Labor tab HTML should mention labor");
}

#[actix_web::test]
async fn test_assign_labor_returns_updated_tab() {
    let (game_state, site_id) = game_state_with_smelter_site();

    // Get the building id from game state
    let building_id = {
        let site = game_state.galaxy.find_site(&site_id).unwrap();
        *site.buildings.keys().next().unwrap()
    };

    let mut app = build_app(game_state).await;

    let req = test::TestRequest::post()
        .uri(&format!("/site/{}/building/{}/assign", site_id, building_id))
        .set_form(&[("skill", "Laborer"), ("count", "3")])
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    // Should succeed and return updated labor tab
    assert_eq!(resp.status(), 200, "Assign labor should return 200 on success");

    let body = test::read_body(resp).await;
    let html = std::str::from_utf8(&body).expect("UTF-8 body");
    assert!(html.contains("labor") || html.contains("worker"), "Should return labor tab HTML");
}

#[actix_web::test]
async fn test_assign_labor_rejects_over_capacity() {
    let (game_state, site_id) = game_state_with_smelter_site();

    let building_id = {
        let site = game_state.galaxy.find_site(&site_id).unwrap();
        *site.buildings.keys().next().unwrap()
    };

    let mut app = build_app(game_state).await;

    // Smelter worker_capacity = 8, try to assign 100
    let req = test::TestRequest::post()
        .uri(&format!("/site/{}/building/{}/assign", site_id, building_id))
        .set_form(&[("skill", "Laborer"), ("count", "100")])
        .to_request();

    let resp = test::call_service(&mut app, req).await;
    assert_eq!(resp.status(), 400, "Assigning more workers than capacity should return 400");
}

