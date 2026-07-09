//! HTTP route handlers + router assembly.
//!
//! Stage 0: foundational read-only endpoints + static frontend serving. Stage 1
//! adds world lifecycle, character endpoints, and the `/ws` game loop.

use axum::extract::State;
use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::services::ServeDir;

use harsh_core::api::status::RuntimeConfigInfo;
use harsh_core::db::WorldDatabase;
use harsh_core::packs::discover_packs;

use crate::admin;
use crate::content;
use crate::demo;
use crate::difficulty_routes;
use crate::editor;
use crate::gameplay;
use crate::state::AppState;
use crate::ws::ws_handler;

/// Build the full application router.
pub fn build_router(state: AppState) -> Router {
    let dist = state.config.frontend_dist.clone();

    Router::new()
        .route("/api", get(root))
        .route("/health", get(health))
        .route("/api/config", get(runtime_config))
        .route("/api/packs", get(list_packs))
        // World lifecycle.
        .route("/api/worlds", get(list_worlds).post(gameplay::create_world))
        .route("/api/worlds/load", post(gameplay::load_world))
        .route("/api/worlds/current", get(gameplay::current_world))
        .route("/api/worlds/current/map", get(gameplay::current_map))
        .route("/api/admin/worlds", get(list_worlds))
        .route("/api/admin/worlds/:file", delete(gameplay::delete_world))
        // Character.
        .route("/api/character", get(gameplay::get_character))
        .route("/api/character/options", get(gameplay::character_options))
        .route("/api/character/roll", post(gameplay::roll_attributes))
        .route("/api/character/auto-pick-skills", post(gameplay::auto_pick_skills))
        .route("/api/character/create", post(gameplay::create_character))
        .route(
            "/api/character/:entity_id/status_effects",
            get(gameplay::character_status_effects),
        )
        .route(
            "/api/character/:entity_id/quests",
            get(gameplay::character_quests),
        )
        // UI layout persistence.
        .route("/api/ui/layout", get(gameplay::get_ui_layout).post(gameplay::set_ui_layout))
        // Content authoring.
        .route("/api/content/compile", post(content::compile_content))
        .route("/api/content/store", post(content::store_content))
        .route("/api/content/records", get(content::list_records))
        .route("/api/content/counts", get(content::content_counts))
        .route(
            "/api/content/records/:component_type/:id/yaml",
            get(content::record_yaml),
        )
        .route(
            "/api/content/records/:component_type/:id",
            delete(content::delete_record),
        )
        .route("/api/content/reference", get(content::content_reference))
        .route("/api/content/template", get(content::content_template))
        .route("/api/content/legacy/categories", get(content::legacy_categories))
        // Content demo harness (isolated, off-session).
        .route("/api/demo/combat", post(demo::demo_combat))
        .route("/api/demo/events", post(demo::demo_events))
        // Game-loop websocket.
        .route("/ws", get(ws_handler))
        // Admin config CRUD.
        .merge(admin::routes())
        // Difficulty settings (player-facing, no admin_mode gate).
        .merge(difficulty_routes::routes())
        // Editor world-state CRUD + GM tools.
        .merge(editor::routes())
        // Hashed asset bundles are served from disk as real files.
        .nest_service("/assets", ServeDir::new(dist.join("assets")))
        // Every other path returns index.html so the history-mode Vue router can
        // resolve client-side routes (/admin, /content) on hard refresh.
        .fallback(spa_index)
        .with_state(state)
}

/// Serve `index.html` for SPA routes (200), or 404 for unknown API paths.
async fn spa_index(State(state): State<AppState>, uri: Uri) -> Response {
    let path = uri.path();
    if path.starts_with("/api") || path.starts_with("/ws") {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }
    let index = state.config.frontend_dist.join("index.html");
    match tokio::fs::read(&index).await {
        Ok(bytes) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            bytes,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "frontend not built").into_response(),
    }
}

async fn root() -> Json<Value> {
    Json(json!({
        "service": "harsh_realm",
        "status": "running",
        "docs": "/docs",
        "health": "/health",
        "api": "/api/worlds",
        "websocket": "/ws",
    }))
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "healthy", "service": "harsh_realm" }))
}

async fn runtime_config(State(state): State<AppState>) -> Json<RuntimeConfigInfo> {
    Json(RuntimeConfigInfo {
        admin_mode: state.config.admin_mode,
        run_mode: state.config.run_mode.clone(),
        version: state.config.version.clone(),
    })
}

async fn list_worlds(State(state): State<AppState>) -> Json<Value> {
    let dir = state.config.worlds_dir.clone();
    let worlds = tokio::task::spawn_blocking(move || WorldDatabase::list_worlds(&dir))
        .await
        .unwrap_or_default();
    Json(serde_json::to_value(worlds).unwrap_or_else(|_| json!([])))
}

async fn list_packs(State(state): State<AppState>) -> Json<Value> {
    let root = state.config.packs_root.clone();
    let packs = tokio::task::spawn_blocking(move || discover_packs(&root).unwrap_or_default())
        .await
        .unwrap_or_default();
    Json(serde_json::to_value(packs).unwrap_or_else(|_| json!([])))
}
