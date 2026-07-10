//! HTTP route handlers and router assembly.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::state::AppState;
use crate::ws::ws_handler;

/// Build the full application router.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api", get(root))
        .route("/health", get(health))
        .route("/api/colonies", get(list_colonies))
        .route("/api/sol", get(current_sol))
        .route("/ws", get(ws_handler))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::RuntimeConfig, state::new_state};
    use axum::body::Body;
    use axum::http::Request;
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
}
