//! World lifecycle + character REST handlers.

use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use harsh_core::dice::DiceRoller;
use harsh_core::character_creation::CharacterDraftSubmission;
use harsh_core::public_api::{CreateWorldRequest, LoadWorldRequest};

use crate::chargen::{build_character_options, compute_auto_pick};
use crate::state::AppState;
use crate::worldsvc::{character_json, map_json, quests_json, status_effects_json};

/// `{ "error": ..., "message": ... }` body with a status code.
fn err(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": "RequestError", "message": message }))).into_response()
}

// --- world lifecycle -------------------------------------------------------

pub async fn create_world(
    State(state): State<AppState>,
    Json(req): Json<CreateWorldRequest>,
) -> Response {
    match state.session.create_world(req).await {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

pub async fn load_world(
    State(state): State<AppState>,
    Json(req): Json<LoadWorldRequest>,
) -> Response {
    match state.session.load_world(req.file).await {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

pub async fn current_world(State(state): State<AppState>) -> Response {
    match state.session.current_world().await {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

pub async fn delete_world(
    State(state): State<AppState>,
    Path(file): Path<String>,
) -> Response {
    match state.session.delete_world(file).await {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

pub async fn current_map(State(state): State<AppState>) -> Response {
    match state.session.read(map_json).await {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::CONFLICT, &e),
    }
}

// --- character -------------------------------------------------------------

pub async fn character_options() -> Response {
    let body = tokio::task::spawn_blocking(build_character_options)
        .await
        .unwrap_or_else(|_| json!({}));
    Json(body).into_response()
}

pub async fn roll_attributes() -> Response {
    let scores = tokio::task::spawn_blocking(|| {
        DiceRoller::new(rand::thread_rng()).roll_attribute_set()
    })
    .await
    .unwrap_or_default();
    Json(json!({ "scores": scores })).into_response()
}

#[derive(Debug, Deserialize)]
pub struct AutoPickRequest {
    pub character_class: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, i32>,
}

pub async fn auto_pick_skills(Json(req): Json<AutoPickRequest>) -> Response {
    let skills = tokio::task::spawn_blocking(move || {
        compute_auto_pick(&req.character_class, &req.attributes)
    })
    .await
    .unwrap_or_default();
    Json(json!({ "skills": skills })).into_response()
}

pub async fn create_character(
    State(state): State<AppState>,
    Json(submission): Json<CharacterDraftSubmission>,
) -> Response {
    match state.session.create_character(submission).await {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::BAD_REQUEST, &e),
    }
}

pub async fn get_character(State(state): State<AppState>) -> Response {
    match state.session.read(character_json).await {
        Ok(Value::Null) => err(StatusCode::NOT_FOUND, "no character in the current world"),
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::CONFLICT, &e),
    }
}

// --- UI layout -------------------------------------------------------------

pub async fn get_ui_layout(State(state): State<AppState>) -> Response {
    let result = state
        .session
        .read(|db| {
            let layout = db
                .get_meta("ui_layout")
                .ok()
                .flatten()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or_else(|| json!({}));
            Ok(layout)
        })
        .await;
    Json(result.unwrap_or_else(|_| json!({}))).into_response()
}

pub async fn set_ui_layout(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    // Fire-and-forget from the frontend; persist to the active world if present.
    let _ = state
        .session
        .read(move |db| {
            let serialized = serde_json::to_string(&body).unwrap_or_else(|_| "{}".into());
            db.set_meta("ui_layout", &serialized)?;
            Ok(json!({ "ok": true }))
        })
        .await;
    Json(json!({ "ok": true })).into_response()
}

pub async fn character_status_effects(
    State(state): State<AppState>,
    Path(entity_id): Path<String>,
) -> Response {
    let result = state
        .session
        .read(move |db| status_effects_json(db, &entity_id))
        .await;
    // Status effects are non-critical UI; fall back to an empty list on error.
    Json(result.unwrap_or_else(|_| json!([]))).into_response()
}

/// `GET /api/character/:entity_id/quests` — active + completed quest instances.
pub async fn character_quests(
    State(state): State<AppState>,
    Path(entity_id): Path<String>,
) -> Response {
    let result = state
        .session
        .read(move |db| quests_json(db, &entity_id))
        .await;
    Json(result.unwrap_or_else(|_| json!({ "active": [], "completed": [] }))).into_response()
}
