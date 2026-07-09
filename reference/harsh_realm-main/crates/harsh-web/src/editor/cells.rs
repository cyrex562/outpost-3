//! Cell editor endpoints — list, single-cell PATCH, and bulk update.

use axum::extract::{Path, Query, State};
use axum::response::Response;
use axum::routing::{post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use harsh_core::editor::EditorCellRepository;
use harsh_core::editor_api::cells::{BulkCellUpdateBody, CellUpdateBody};

use super::respond;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/cells", axum::routing::get(list_cells))
        .route("/api/admin/cells/bulk-update", post(bulk_update))
        .route("/api/admin/cells/:q/:r", put(update_cell))
}

#[derive(Debug, Deserialize)]
pub struct CellListQuery {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

async fn list_cells(State(s): State<AppState>, Query(q): Query<CellListQuery>) -> Response {
    let limit = q.limit.unwrap_or(2000).clamp(1, 100_000);
    let offset = q.offset.unwrap_or(0).max(0);
    respond(
        s.session
            .read(move |db| {
                let cells = EditorCellRepository::new(db).list_cells(limit, offset)?;
                serde_json::to_value(cells).map_err(|e| e.to_string())
            })
            .await,
    )
}

async fn update_cell(
    State(s): State<AppState>,
    Path((q, r)): Path<(i32, i32)>,
    Json(body): Json<CellUpdateBody>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                EditorCellRepository::new(db).update_cell(q, r, &body)?;
                Ok(json!({ "ok": true }))
            })
            .await,
    )
}

async fn bulk_update(State(s): State<AppState>, Json(body): Json<BulkCellUpdateBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let updated = EditorCellRepository::new(db).bulk_update_cells(&body)?;
                Ok(json!({ "updated": updated }))
            })
            .await,
    )
}
