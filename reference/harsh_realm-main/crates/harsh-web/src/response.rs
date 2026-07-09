//! Shared HTTP response helper for the admin/editor JSON endpoints.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

/// JSON-encode `result`, mapping an `Err` to a 409 `{ error, message }` body.
///
/// `error_kind` is the error label (e.g. `"AdminError"` / `"EditorError"`), the
/// only thing that differed between the admin and editor `respond` helpers.
pub(crate) fn respond_with(result: Result<Value, String>, error_kind: &'static str) -> Response {
    match result {
        Ok(body) => Json(body).into_response(),
        Err(e) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": error_kind, "message": e })),
        )
            .into_response(),
    }
}
