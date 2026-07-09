//! Oracle editor endpoints — plot threads and tracked NPCs. (The chaos-factor
//! oracle-state endpoint lives in [`crate::admin`].)

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use rusqlite::ToSql;
use serde::Deserialize;
use serde_json::{json, Value};

use harsh_core::db::WorldDatabase;
use harsh_core::repositories::oracle::OracleRepository;

use super::respond;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/threads", get(list_threads).post(create_thread))
        .route("/api/admin/threads/:id", axum::routing::put(update_thread).delete(delete_thread))
        .route("/api/admin/oracle-npcs", get(list_npcs).post(create_npc))
        .route(
            "/api/admin/oracle-npcs/:id",
            axum::routing::put(update_npc).delete(delete_npc),
        )
}

// --- threads ---------------------------------------------------------------

async fn list_threads(State(s): State<AppState>) -> Response {
    respond(
        s.session
            .read(|db| {
                let rows = db.fetch_all(
                    "SELECT id, type, title, status, progress FROM threads ORDER BY id",
                    &[],
                )?;
                let threads: Vec<Value> = rows
                    .iter()
                    .map(|row| {
                        json!({
                            "id": row.get("id").and_then(Value::as_str).unwrap_or(""),
                            "type": row.get("type").and_then(Value::as_str).unwrap_or("story"),
                            "title": row.get("title").and_then(Value::as_str).unwrap_or(""),
                            "status": row.get("status").and_then(Value::as_str).unwrap_or("active"),
                            "progress": row.get("progress").and_then(Value::as_i64).unwrap_or(0),
                        })
                    })
                    .collect();
                Ok(Value::Array(threads))
            })
            .await,
    )
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadBody {
    pub title: String,
    #[serde(default, rename = "type")]
    pub thread_type: Option<String>,
}

async fn create_thread(State(s): State<AppState>, Json(body): Json<CreateThreadBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let id = uuid::Uuid::new_v4().to_string();
                let thread_type = body.thread_type.unwrap_or_else(|| "story".to_string());
                OracleRepository::new(db).create_thread(&id, &thread_type, &body.title)?;
                Ok(json!({ "id": id }))
            })
            .await,
    )
}

#[derive(Debug, Deserialize)]
pub struct UpdateThreadBody {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, rename = "type")]
    pub thread_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub progress: Option<i64>,
}

async fn update_thread(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateThreadBody>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                let mut sets: Vec<&str> = Vec::new();
                let mut params: Vec<Box<dyn ToSql>> = Vec::new();
                if let Some(title) = &body.title {
                    sets.push("title = ?");
                    params.push(Box::new(title.clone()));
                }
                if let Some(t) = &body.thread_type {
                    sets.push("type = ?");
                    params.push(Box::new(t.clone()));
                }
                if let Some(status) = &body.status {
                    sets.push("status = ?");
                    params.push(Box::new(status.clone()));
                }
                if let Some(progress) = body.progress {
                    sets.push("progress = ?");
                    params.push(Box::new(progress));
                }
                run_update(db, "threads", &sets, params, &id)
            })
            .await,
    )
}

async fn delete_thread(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    respond(
        s.session
            .read(move |db| {
                db.execute("DELETE FROM threads WHERE id = ?", &[&id])?;
                Ok(json!({ "ok": true }))
            })
            .await,
    )
}

// --- oracle npcs -----------------------------------------------------------

async fn list_npcs(State(s): State<AppState>) -> Response {
    respond(
        s.session
            .read(|db| {
                let npcs = OracleRepository::new(db).list_oracle_npcs(None)?;
                serde_json::to_value(npcs).map_err(|e| e.to_string())
            })
            .await,
    )
}

#[derive(Debug, Deserialize)]
pub struct CreateNpcBody {
    pub name: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub entity_id: Option<String>,
}

async fn create_npc(State(s): State<AppState>, Json(body): Json<CreateNpcBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let id = uuid::Uuid::new_v4().to_string();
                OracleRepository::new(db).create_oracle_npc(
                    &id,
                    &body.name,
                    body.notes.as_deref().unwrap_or(""),
                    body.entity_id.as_deref(),
                )?;
                Ok(json!({ "id": id }))
            })
            .await,
    )
}

#[derive(Debug, Deserialize)]
pub struct UpdateNpcBody {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

async fn update_npc(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNpcBody>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                let mut sets: Vec<&str> = Vec::new();
                let mut params: Vec<Box<dyn ToSql>> = Vec::new();
                if let Some(name) = &body.name {
                    sets.push("name = ?");
                    params.push(Box::new(name.clone()));
                }
                if let Some(status) = &body.status {
                    sets.push("status = ?");
                    params.push(Box::new(status.clone()));
                }
                if let Some(notes) = &body.notes {
                    sets.push("notes = ?");
                    params.push(Box::new(notes.clone()));
                }
                run_update(db, "oracle_npcs", &sets, params, &id)
            })
            .await,
    )
}

async fn delete_npc(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    respond(
        s.session
            .read(move |db| {
                OracleRepository::new(db).delete_oracle_npc(&id)?;
                Ok(json!({ "ok": true }))
            })
            .await,
    )
}

/// Apply a partial `UPDATE <table> SET <sets> WHERE id = ?`. No-op if `sets` empty.
fn run_update(
    db: &WorldDatabase,
    table: &str,
    sets: &[&str],
    params: Vec<Box<dyn ToSql>>,
    id: &str,
) -> Result<Value, String> {
    super::apply_partial_update(db, table, sets, params, "id", id)?;
    Ok(json!({ "ok": true }))
}
