//! Dungeon editor endpoints — list, get (with rooms/connections), create,
//! update, delete. All coordinates are square-grid `location_q`/`location_r`.

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use harsh_core::editor_api::dungeons::DungeonUpdateRequest;
use harsh_core::repositories::dungeon::DungeonRepository;

use super::respond;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/dungeons", get(list_dungeons).post(create_dungeon))
        .route(
            "/api/admin/dungeons/:id",
            get(get_dungeon).put(update_dungeon).delete(delete_dungeon),
        )
}

async fn list_dungeons(State(s): State<AppState>) -> Response {
    respond(
        s.session
            .read(|db| {
                let dungeons = DungeonRepository::new(db).list_dungeons()?;
                serde_json::to_value(dungeons).map_err(|e| e.to_string())
            })
            .await,
    )
}

async fn get_dungeon(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    respond(
        s.session
            .read(move |db| match DungeonRepository::new(db).get_dungeon(&id)? {
                Some(record) => serde_json::to_value(record).map_err(|e| e.to_string()),
                None => Err(format!("dungeon not found: {id}")),
            })
            .await,
    )
}

async fn create_dungeon(
    State(s): State<AppState>,
    Json(body): Json<DungeonUpdateRequest>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                let created = DungeonRepository::new(db).create_dungeon(&body)?;
                serde_json::to_value(created).map_err(|e| e.to_string())
            })
            .await,
    )
}

async fn update_dungeon(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<DungeonUpdateRequest>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                let ok = DungeonRepository::new(db).update_dungeon(&id, &body)?;
                if ok {
                    Ok(json!({ "ok": true }))
                } else {
                    Err(format!("dungeon not found: {id}"))
                }
            })
            .await,
    )
}

async fn delete_dungeon(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let ok = DungeonRepository::new(db).delete_dungeon(&id)?;
                Ok::<Value, String>(json!({ "ok": ok }))
            })
            .await,
    )
}
