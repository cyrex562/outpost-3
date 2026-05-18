// Integration tests for event log endpoints (Task 6.4)

use actix_web::{test, web, App};
use outpost_core::content::{ContentLoader, EventCategory, EventSeverity};
use outpost_core::domain::GameState;
use outpost_server::http::routes;
use outpost_server::services::SimulationService;

fn load_content() -> ContentLoader {
    let mut loader = ContentLoader::new();
    let buildings_yaml = std::fs::read_to_string(
        format!("{}/../../content/buildings/basic_buildings.yaml", env!("CARGO_MANIFEST_DIR"))
    ).expect("Load buildings");
    loader.load_buildings(&buildings_yaml).expect("Parse buildings");

    let resources_yaml = std::fs::read_to_string(
        format!("{}/../../content/resources/basic_resources.yaml", env!("CARGO_MANIFEST_DIR"))
    ).expect("Load resources");
    loader.load_resources(&resources_yaml).expect("Parse resources");

    let events_yaml = std::fs::read_to_string(
        format!("{}/../../content/events/narrative_events.yaml", env!("CARGO_MANIFEST_DIR"))
    ).expect("Load events");
    loader.load_events(&events_yaml).expect("Parse events");

    loader
}

async fn build_app(state: GameState) -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    let content = load_content();
    let simulation = web::Data::new(SimulationService::new(state, content));
    let template_glob = format!("{}/templates/**/*.html", env!("CARGO_MANIFEST_DIR"));
    let tmpl = web::Data::new(tera::Tera::new(&template_glob).expect("Load templates"));
    test::init_service(
        App::new()
            .app_data(simulation)
            .app_data(tmpl)
            .configure(routes::configure),
    )
    .await
}

fn empty_state() -> GameState {
    let mut state = GameState::new("Test Galaxy".into(), 42);
    let content = load_content();
    state.building_definitions = content.all_buildings().clone();
    state.resource_definitions = content.all_resources().clone();
    state.event_definitions   = content.all_events().clone();
    state
}

fn state_with_events() -> GameState {
    use outpost_core::domain::SiteId;
    let mut state = empty_state();

    // Push some test events into the log
    let site_id = SiteId::new();
    state.event_log.push(
        1, site_id, "equipment_failure".into(), "Critical Equipment Failure".into(),
        EventCategory::Disaster, EventSeverity::Critical, true,
    );
    state.event_log.push(
        2, site_id, "mineral_deposit_discovered".into(), "Rich Mineral Deposit".into(),
        EventCategory::Discovery, EventSeverity::Info, false,
    );
    state.event_log.push(
        3, site_id, "morale_crisis".into(), "Unrest Among Colonists".into(),
        EventCategory::Social, EventSeverity::Warning, false,
    );
    state
}

// ── GET /events ──

#[actix_web::test]
async fn test_event_log_page_returns_200() {
    let app = build_app(empty_state()).await;
    let req = test::TestRequest::get().uri("/events").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_event_log_page_empty_state() {
    let app = build_app(empty_state()).await;
    let req = test::TestRequest::get().uri("/events").to_request();
    let body = test::call_and_read_body(&app, req).await;
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Event Log"), "Page must have title");
    assert!(html.contains("No events yet") || html.contains("0 event"), "Should show empty state");
}

#[actix_web::test]
async fn test_event_log_page_shows_entries() {
    let app = build_app(state_with_events()).await;
    let req = test::TestRequest::get().uri("/events").to_request();
    let body = test::call_and_read_body(&app, req).await;
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Critical Equipment Failure"), "Should show event title");
    assert!(html.contains("Rich Mineral Deposit"),       "Should show discovery event");
    assert!(html.contains("Unrest Among Colonists"),     "Should show social event");
}

#[actix_web::test]
async fn test_event_log_page_category_filter() {
    let app = build_app(state_with_events()).await;
    let req = test::TestRequest::get().uri("/events?category=disaster").to_request();
    let body = test::call_and_read_body(&app, req).await;
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Critical Equipment Failure"), "Disaster should be shown");
    assert!(!html.contains("Rich Mineral Deposit"),      "Discovery should be filtered out");
}

#[actix_web::test]
async fn test_event_log_page_severity_filter() {
    let app = build_app(state_with_events()).await;
    let req = test::TestRequest::get().uri("/events?severity=critical").to_request();
    let body = test::call_and_read_body(&app, req).await;
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("Critical Equipment Failure"), "Critical should be shown");
    // warning and info should be filtered
    assert!(!html.contains("Rich Mineral Deposit"), "Info event should be filtered");
}

// ── GET /api/events/ticker ──

#[actix_web::test]
async fn test_event_ticker_returns_200() {
    let app = build_app(empty_state()).await;
    let req = test::TestRequest::get().uri("/api/events/ticker").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_event_ticker_shows_recent_events() {
    let app = build_app(state_with_events()).await;
    let req = test::TestRequest::get().uri("/api/events/ticker").to_request();
    let body = test::call_and_read_body(&app, req).await;
    let html = String::from_utf8_lossy(&body);
    // ticker should contain at least one event title
    assert!(
        html.contains("Critical Equipment Failure")
            || html.contains("Rich Mineral Deposit")
            || html.contains("Unrest Among Colonists"),
        "Ticker must show at least one recent event"
    );
}

// ── GET /api/events/badges ──

#[actix_web::test]
async fn test_event_badges_returns_200() {
    let app = build_app(empty_state()).await;
    let req = test::TestRequest::get().uri("/api/events/badges").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
}

#[actix_web::test]
async fn test_event_badges_shows_critical_count() {
    let app = build_app(state_with_events()).await;
    let req = test::TestRequest::get().uri("/api/events/badges").to_request();
    let body = test::call_and_read_body(&app, req).await;
    let html = String::from_utf8_lossy(&body);
    // 1 critical event pushed → badge should show 1
    assert!(html.contains('1'), "Badge should show critical count of 1");
}

// ── Route registration ──

#[actix_web::test]
async fn test_event_routes_registered() {
    let app = build_app(empty_state()).await;

    let routes = [
        "/events",
        "/api/events/ticker",
        "/api/events/badges",
    ];

    for route in &routes {
        let req = test::TestRequest::get().uri(route).to_request();
        let resp = test::call_service(&app, req).await;
        assert!(
            resp.status().is_success(),
            "Route {} should return 2xx, got {}",
            route,
            resp.status()
        );
    }
}
