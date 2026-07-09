//! Filesystem-backed admin endpoints: editor schemas, YAML pack files, table
//! status, and world clone.
//!
//! YAML paths are confined to the content root; `..`, absolute paths, and drive
//! prefixes are rejected. These endpoints are independent of the active world
//! (they read the on-disk content pack), except world clone which copies a
//! world `.db` file.

use std::path::{Path as FsPath, PathBuf};

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/editor-schemas", axum::routing::get(editor_schemas))
        .route("/api/admin/yaml-files", axum::routing::get(list_yaml_files))
        .route("/api/admin/yaml-table-status", axum::routing::get(yaml_table_status))
        .route(
            "/api/admin/yaml-files/*path",
            axum::routing::get(get_yaml).put(put_yaml).post(upload_yaml).delete(delete_yaml),
        )
        .route("/api/admin/worlds/:file/clone", axum::routing::post(clone_world))
}

// --- editor schemas --------------------------------------------------------

async fn editor_schemas(State(s): State<AppState>) -> Response {
    let dir = s.config.packs_root.join("schemas").join("editors");
    let schemas = tokio::task::spawn_blocking(move || read_schema_dir(&dir))
        .await
        .unwrap_or_default();
    Json(Value::Array(schemas)).into_response()
}

fn read_schema_dir(dir: &FsPath) -> Vec<Value> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(value) = serde_yaml::from_str::<Value>(&text) {
                out.push(value);
            }
        }
    }
    out
}

// --- yaml file path safety -------------------------------------------------

/// Resolve a request path under `root`, rejecting traversal/absolute paths.
pub(crate) fn safe_join(root: &FsPath, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.trim_start_matches('/');
    if rel.is_empty() {
        return Err("empty path".into());
    }
    for part in rel.split(['/', '\\']) {
        if part == ".." || part.contains(':') {
            return Err(format!("illegal path: {rel}"));
        }
    }
    Ok(root.join(rel))
}

// --- list / status ---------------------------------------------------------

async fn list_yaml_files(State(s): State<AppState>) -> Response {
    let root = s.config.packs_root.clone();
    let files = tokio::task::spawn_blocking(move || walk_yaml(&root))
        .await
        .unwrap_or_default();
    Json(Value::Array(files)).into_response()
}

/// Walk the content root for `*.yaml`/`*.yml`, returning `[{path, size}]` with
/// forward-slash relative paths.
fn walk_yaml(root: &FsPath) -> Vec<Value> {
    super::walk_yaml_files(root)
        .into_iter()
        .map(|(abs, rel)| {
            let size = std::fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
            json!({ "path": rel, "size": size })
        })
        .collect()
}

async fn yaml_table_status(State(s): State<AppState>) -> Response {
    let root = s.config.packs_root.clone();
    let status = tokio::task::spawn_blocking(move || table_status(&root))
        .await
        .unwrap_or_default();
    Json(Value::Array(status)).into_response()
}

/// Summarise YAML files that live under a `tables/` directory: entry count and
/// whether a TODO marker is present. Single pass over the shared walker — each
/// matching file is read once.
fn table_status(root: &FsPath) -> Vec<Value> {
    let mut out = Vec::new();
    for (abs, rel) in super::walk_yaml_files(root) {
        if !rel.contains("tables/") {
            continue;
        }
        let text = std::fs::read_to_string(&abs).unwrap_or_default();
        let size = text.len() as u64;
        let has_todo = text.to_uppercase().contains("TODO");
        let entry_count = serde_yaml::from_str::<Value>(&text)
            .ok()
            .map(|v| count_entries(&v))
            .unwrap_or(0);
        out.push(json!({
            "path": rel,
            "size": size,
            "entry_count": entry_count,
            "has_todo": has_todo,
        }));
    }
    out
}

/// Count rows in a table document (`entries`/`table`/`rows` array, else 0).
fn count_entries(value: &Value) -> usize {
    for key in ["entries", "table", "rows", "items"] {
        if let Some(arr) = value.get(key).and_then(Value::as_array) {
            return arr.len();
        }
    }
    0
}

// --- get / put / delete ----------------------------------------------------

async fn get_yaml(State(s): State<AppState>, Path(path): Path<String>) -> Response {
    let root = s.config.packs_root.clone();
    // `<path>/content` -> JSON `{content}`; bare `<path>` -> raw download.
    if let Some(rel) = path.strip_suffix("/content") {
        let rel = rel.to_string();
        return match safe_join(&root, &rel) {
            Ok(abs) => {
                let content = tokio::fs::read_to_string(&abs).await.unwrap_or_default();
                Json(json!({ "path": rel, "content": content })).into_response()
            }
            Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
        };
    }
    match safe_join(&root, &path) {
        Ok(abs) => match tokio::fs::read(&abs).await {
            Ok(bytes) => {
                let name = abs.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file.yaml".into());
                (
                    [
                        (header::CONTENT_TYPE, "application/x-yaml".to_string()),
                        (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\"")),
                    ],
                    bytes,
                )
                    .into_response()
            }
            Err(_) => (StatusCode::NOT_FOUND, "file not found").into_response(),
        },
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct YamlContentBody {
    #[serde(default)]
    pub content: String,
}

async fn put_yaml(
    State(s): State<AppState>,
    Path(path): Path<String>,
    Json(body): Json<YamlContentBody>,
) -> Response {
    let root = s.config.packs_root.clone();
    let rel = path.strip_suffix("/content").unwrap_or(&path).to_string();
    let abs = match safe_join(&root, &rel) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    if let Some(parent) = abs.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }
    match tokio::fs::write(&abs, body.content.as_bytes()).await {
        Ok(_) => Json(json!({ "ok": true, "path": rel })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// Multipart upload of a single YAML file to `<path>` (form field `file`).
async fn upload_yaml(
    State(s): State<AppState>,
    Path(path): Path<String>,
    mut multipart: axum::extract::Multipart,
) -> Response {
    let root = s.config.packs_root.clone();
    let abs = match safe_join(&root, &path) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    let mut bytes: Option<Vec<u8>> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            bytes = field.bytes().await.ok().map(|b| b.to_vec());
            break;
        }
    }
    let Some(bytes) = bytes else {
        return (StatusCode::BAD_REQUEST, "missing 'file' field").into_response();
    };
    if let Some(parent) = abs.parent() {
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    }
    match tokio::fs::write(&abs, &bytes).await {
        Ok(_) => Json(json!({ "ok": true, "path": path })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_yaml(State(s): State<AppState>, Path(path): Path<String>) -> Response {
    let root = s.config.packs_root.clone();
    let abs = match safe_join(&root, &path) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };
    match tokio::fs::remove_file(&abs).await {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

// --- world clone -----------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CloneQuery {
    pub new_name: String,
}

async fn clone_world(
    State(s): State<AppState>,
    Path(file): Path<String>,
    Query(q): Query<CloneQuery>,
) -> Response {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return (StatusCode::BAD_REQUEST, format!("invalid world file: {file}")).into_response();
    }
    let worlds_dir = s.config.worlds_dir.clone();
    let result = tokio::task::spawn_blocking(move || clone_world_file(&worlds_dir, &file, &q.new_name))
        .await
        .unwrap_or_else(|e| Err(e.to_string()));
    match result {
        Ok(body) => Json(body).into_response(),
        Err(e) => (StatusCode::CONFLICT, Json(json!({ "error": "CloneError", "message": e }))).into_response(),
    }
}

fn clone_world_file(worlds_dir: &FsPath, file: &str, new_name: &str) -> Result<Value, String> {
    let src = worlds_dir.join(file);
    if !src.exists() {
        return Err(format!("world not found: {file}"));
    }
    let slug: String = new_name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-');
    let stem = if slug.is_empty() { "world" } else { slug };
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dest_file = format!("{stem}-{millis}.db");
    let dest = worlds_dir.join(&dest_file);
    std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;
    // Rename the copy's world-name metadata (best-effort; the clone itself
    // succeeded — log rather than swallow if the rename can't be applied).
    match harsh_core::db::WorldDatabase::open(&dest) {
        Ok(db) => {
            if let Err(e) = db.set_meta("name", new_name) {
                eprintln!("clone_world_file: failed to set name on {dest_file}: {e}");
            }
        }
        Err(e) => eprintln!("clone_world_file: could not open copy {dest_file} to rename: {e}"),
    }
    Ok(json!({ "file": dest_file, "name": new_name }))
}
