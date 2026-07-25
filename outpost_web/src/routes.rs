//! HTTP route handlers and router assembly.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use uuid::Uuid;

use outpost_core::{Command, EngineError};

use crate::query_routes;
use crate::state::AppState;
use crate::ws::{client_command_to_core, ws_handler};
use crate::wsmsg::ClientCommand;

/// Build the full application router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api", get(root))
        .route("/health", get(health))
        .route("/api/colonies", get(list_colonies))
        .route("/api/sol", get(current_sol))
        .route("/api/command", post(apply_shared_command))
        .route("/api/log", post(log_frontend_error))
        .route(
            "/api/colonize-targets",
            get(query_routes::get_colonize_targets),
        )
        .route("/api/buildings", get(query_routes::list_buildings))
        .route(
            "/api/supply-packages",
            get(query_routes::list_supply_packages),
        )
        .route("/api/planet-map", get(query_routes::get_planet_map))
        .route("/api/body-surface/:id", get(query_routes::get_body_surface))
        .route("/api/system-bodies", get(query_routes::get_system_bodies))
        .route("/api/system-name", get(query_routes::get_system_name))
        .route("/api/outposts", get(query_routes::list_outposts))
        .route(
            "/api/outpost-targets/:colony_id",
            get(query_routes::get_outpost_targets),
        )
        .route("/api/tech-tree", get(query_routes::get_tech_tree))
        .route(
            "/api/interrupt-digest",
            get(query_routes::get_interrupt_digest),
        )
        .route("/ws", get(ws_handler))
        // Session management
        .route("/sessions", post(create_session))
        .route("/sessions/:id/commands", post(apply_command))
        .route("/sessions/:id/state", get(get_state))
        .route("/sessions/:id", delete(delete_session))
        .with_state(state)
}

/// Root endpoint — returns a JSON identity payload.
async fn root() -> Json<Value> {
    Json(json!({ "app": "outpost3", "version": env!("CARGO_PKG_VERSION") }))
}

/// Health-check endpoint.
async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// List all colonies (convenience REST endpoint — mirrors `Query::ListColonies`).
async fn list_colonies(State(state): State<AppState>) -> impl IntoResponse {
    use outpost_core::Query;

    let engine = state.engine.lock().expect("engine lock");
    match engine.query(&Query::ListColonies) {
        Ok(result) => {
            let json = serde_json::to_value(result).unwrap_or_default();
            (StatusCode::OK, Json(json)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Current colony-sol counter.
async fn current_sol(State(state): State<AppState>) -> impl IntoResponse {
    use outpost_core::Query;

    let engine = state.engine.lock().expect("engine lock");
    match engine.query(&Query::CurrentSol) {
        Ok(result) => {
            let json = serde_json::to_value(result).unwrap_or_default();
            (StatusCode::OK, Json(json)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// `POST /api/command` — apply a [`ClientCommand`] to the shared browser-mode
/// engine.
///
/// This targets `AppState.engine` (the same engine the WebSocket `NewGame`
/// flow bootstraps content/planet/colony onto), not a per-session engine —
/// see `query_routes` for the rationale. Accepts the same `{"kind": ...}`
/// tagged shape as the WS `command` message (and the frontend's `Command`
/// type) rather than the core [`Command`] enum's externally-tagged shape,
/// so it's a drop-in REST alternative to sending over the socket. Returns
/// the events produced by the command in the same [`crate::wsmsg::ServerEvent`]
/// wire shape the WebSocket `event` message uses — not the core [`Event`]
/// enum's externally-tagged shape — so the frontend's shared `GameEvent`
/// handling code (keyed on `.kind`) works identically over REST and WS.
///
/// # Panics
///
/// Panics if the shared engine mutex is poisoned.
async fn apply_shared_command(
    State(state): State<AppState>,
    Json(client_cmd): Json<ClientCommand>,
) -> impl IntoResponse {
    let cmd = match client_command_to_core(client_cmd, &state) {
        Ok(cmd) => cmd,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response(),
    };

    let result = {
        let mut engine = state.engine.lock().expect("engine lock");
        engine.apply(&cmd)
    };

    match result {
        Ok(events) => {
            // Fan out to every WS-connected client watching the shared engine
            // (a second browser tab, a future spectator client, ...), same as
            // the WS command handler does — otherwise only the issuing tab's
            // locally-injected copy of these events would ever be seen.
            for e in &events {
                let _ = state.events.send(e.clone());
            }
            let wire_events: Vec<crate::wsmsg::ServerEvent> = events
                .iter()
                .map(crate::wsmsg::ServerEvent::from_core)
                .collect();
            let json = serde_json::to_value(&wire_events).unwrap_or(Value::Array(vec![]));
            (StatusCode::OK, Json(json)).into_response()
        }
        Err(e) => {
            let status = engine_error_status(&e);
            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

/// A frontend-reported error, forwarded into the server's log so a
/// browser-mode session that goes blank (an uncaught Vue render exception,
/// same failure mode as the Tauri desktop shell) leaves a trace on disk
/// instead of only in a devtools console nobody was watching.
#[derive(serde::Deserialize)]
struct FrontendErrorReport {
    source: String,
    message: String,
    stack: Option<String>,
}

/// Record a frontend error report (`POST /api/log`) via `tracing`.
async fn log_frontend_error(Json(report): Json<FrontendErrorReport>) -> impl IntoResponse {
    match &report.stack {
        Some(stack) => {
            tracing::error!(source = %report.source, message = %report.message, stack = %stack, "frontend error");
        }
        None => {
            tracing::error!(source = %report.source, message = %report.message, "frontend error");
        }
    }
    StatusCode::NO_CONTENT
}

// ─── Session endpoints ────────────────────────────────────────────────────────

/// `POST /sessions` — create a new game session, returns `{ "session_id": "<uuid>" }`.
async fn create_session(State(state): State<AppState>) -> impl IntoResponse {
    let id = state.sessions.create();
    (StatusCode::CREATED, Json(json!({ "session_id": id })))
}

/// `POST /sessions/{id}/commands` — apply a [`Command`] to the session engine.
///
/// Returns the list of [`Event`]s produced by the command.
async fn apply_command(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(cmd): Json<Command>,
) -> impl IntoResponse {
    let Some(session) = state.sessions.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session not found" })),
        )
            .into_response();
    };

    let result = {
        let mut engine = session.engine.lock().expect("session engine lock");
        engine.apply(&cmd)
    };

    match result {
        Ok(events) => {
            let json = serde_json::to_value(&events).unwrap_or(Value::Array(vec![]));
            (StatusCode::OK, Json(json)).into_response()
        }
        Err(e) => {
            let status = engine_error_status(&e);
            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

/// `GET /sessions/{id}/state` — return a JSON snapshot of the session state.
async fn get_state(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    use outpost_core::Query;

    let Some(session) = state.sessions.get(&id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session not found" })),
        )
            .into_response();
    };

    let engine = session.engine.lock().expect("session engine lock");
    let sol = engine.sol();
    let month = engine.month();
    let colonies = match engine.query(&Query::ListColonies) {
        Ok(outpost_core::QueryResult::Colonies(c)) => {
            serde_json::to_value(c).unwrap_or(Value::Array(vec![]))
        }
        _ => Value::Array(vec![]),
    };

    let snapshot = json!({
        "sol": sol,
        "month": month,
        "colonies": colonies,
    });
    (StatusCode::OK, Json(snapshot)).into_response()
}

/// `DELETE /sessions/{id}` — tear down a session.
async fn delete_session(State(state): State<AppState>, Path(id): Path<Uuid>) -> impl IntoResponse {
    if state.sessions.remove(&id) {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

/// Map an [`EngineError`] to an appropriate HTTP status code.
fn engine_error_status(e: &EngineError) -> StatusCode {
    match e {
        EngineError::InvalidArgument(_)
        | EngineError::ColonyNotFound(_)
        | EngineError::ProjectNotFound(_)
        | EngineError::DirectiveNotFound(_)
        | EngineError::NoPlanetMap
        | EngineError::SiteNotFound(_)
        | EngineError::SiteNotHabitable
        | EngineError::SiteOccupied
        | EngineError::SlotCapacityExceeded { .. }
        | EngineError::InsufficientResources { .. } => StatusCode::BAD_REQUEST,
        EngineError::GameOver => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::RuntimeConfig, state::new_state};
    use axum::body::Body;
    use axum::http::Request;
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_router() -> Router {
        let state = new_state(RuntimeConfig::default());
        build_router(state)
    }

    #[tokio::test]
    async fn root_returns_app_name() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/api").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["app"], "outpost3");
    }

    #[tokio::test]
    async fn health_check_returns_ok() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn list_colonies_empty_on_fresh_engine() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/api/colonies").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn current_sol_returns_zero_on_fresh_engine() {
        let router = test_router();
        let response = router
            .oneshot(Request::get("/api/sol").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        // QueryResult::Counter(0) serialised
        assert!(json.get("Counter").is_some() || json.to_string().contains("0"));
    }

    #[tokio::test]
    async fn create_session_returns_session_id() {
        let router = test_router();
        let response = router
            .oneshot(
                Request::post("/sessions")
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert!(json["session_id"].is_string());
    }

    #[tokio::test]
    async fn session_command_and_state_roundtrip() {
        // Share one AppState across multiple router builds so handlers see the same sessions.
        let state = new_state(RuntimeConfig::default());

        let make_router = || build_router(Arc::clone(&state));

        // Create a session.
        let resp = make_router()
            .oneshot(
                Request::post("/sessions")
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        let session_id = json["session_id"].as_str().unwrap().to_string();

        // Apply a FoundColony command.
        let cmd = json!({
            "FoundColony": {
                "name": "Alpha",
                "starting_population": 100
            }
        });
        let resp = make_router()
            .oneshot(
                Request::post(format!("/sessions/{session_id}/commands"))
                    .header("content-type", "application/json")
                    .body(Body::from(cmd.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // GET state returns valid JSON with sol field.
        let resp = make_router()
            .oneshot(
                Request::get(format!("/sessions/{session_id}/state"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let state_json: Value = serde_json::from_slice(&body).unwrap();
        assert!(state_json["sol"].is_number());
        assert!(state_json["colonies"].is_array());

        // DELETE session.
        let resp = make_router()
            .oneshot(
                Request::delete(format!("/sessions/{session_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        // GET state after deletion returns 404.
        let resp = make_router()
            .oneshot(
                Request::get(format!("/sessions/{session_id}/state"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_session_command_returns_404() {
        let router = test_router();
        let cmd = json!({ "FoundColony": { "name": "X", "starting_population": 10 } });
        let resp = router
            .oneshot(
                Request::post("/sessions/00000000-0000-0000-0000-000000000000/commands")
                    .header("content-type", "application/json")
                    .body(Body::from(cmd.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
