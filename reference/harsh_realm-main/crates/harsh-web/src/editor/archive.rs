//! ZIP + multipart endpoints: world export, table-bundle download/upload, and
//! CSV table import.

use std::io::{Cursor, Read, Write};

use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde_json::{json, Map, Value};
use zip::write::SimpleFileOptions;

use harsh_core::editor::transfer::is_exportable;

use super::files::safe_join;
use super::transfer::import_rows;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/worlds/:file/export", axum::routing::get(export_world))
        .route(
            "/api/admin/yaml-tables-zip",
            axum::routing::get(download_tables_zip).post(upload_tables_zip),
        )
        .route("/api/admin/import/:table/csv", axum::routing::post(import_csv))
}

// --- world export ----------------------------------------------------------

async fn export_world(State(s): State<AppState>, Path(file): Path<String>) -> Response {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return (StatusCode::BAD_REQUEST, format!("invalid world file: {file}")).into_response();
    }
    let path = s.config.worlds_dir.join(&file);
    let bytes = match tokio::fs::read(&path).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::NOT_FOUND, "world not found").into_response(),
    };
    let zipped = match zip_single(&file, &bytes) {
        Ok(z) => z,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    };
    let download = format!("{}_export.zip", file.trim_end_matches(".db"));
    zip_response(zipped, &download)
}

// --- table bundle ----------------------------------------------------------

async fn download_tables_zip(State(s): State<AppState>) -> Response {
    let root = s.config.packs_root.clone();
    let result = tokio::task::spawn_blocking(move || zip_tables(&root)).await;
    match result {
        Ok(Ok(bytes)) => zip_response(bytes, "tables.zip"),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn upload_tables_zip(State(s): State<AppState>, mut multipart: Multipart) -> Response {
    let root = s.config.packs_root.clone();
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
    match tokio::task::spawn_blocking(move || extract_yaml_zip(&root, &bytes)).await {
        Ok(Ok((written, errors))) => Json(json!({ "written": written, "errors": errors })).into_response(),
        Ok(Err(e)) => (StatusCode::BAD_REQUEST, e).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// --- csv import ------------------------------------------------------------

async fn import_csv(
    State(s): State<AppState>,
    Path(table): Path<String>,
    mut multipart: Multipart,
) -> Response {
    if !is_exportable(&table) {
        return (StatusCode::BAD_REQUEST, format!("table not importable: {table}")).into_response();
    }
    let mut text: Option<String> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            text = field.text().await.ok();
            break;
        }
    }
    let Some(text) = text else {
        return (StatusCode::BAD_REQUEST, "missing 'file' field").into_response();
    };
    let rows = parse_csv(&text);
    let result = s
        .session
        .read(move |db| {
            let (imported, errors) = import_rows(db, &table, &rows)?;
            Ok(json!({ "imported": imported, "errors": errors }))
        })
        .await;
    super::respond(result)
}

// --- zip helpers -----------------------------------------------------------

fn zip_single(name: &str, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, opts).map_err(|e| e.to_string())?;
    zip.write_all(bytes).map_err(|e| e.to_string())?;
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Zip every `tables/*.yaml` file under the content root, keyed by relative path.
fn zip_tables(root: &std::path::Path) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (abs, rel) in crate::editor::walk_yaml_files(root) {
        if !rel.contains("tables/") {
            continue;
        }
        if let Ok(bytes) = std::fs::read(&abs) {
            zip.start_file(&rel, opts).map_err(|e| e.to_string())?;
            zip.write_all(&bytes).map_err(|e| e.to_string())?;
        }
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

/// Extract `*.yaml`/`*.yml` entries from a zip into the content root. Returns
/// `(written_paths, errors)`. Non-YAML and unsafe paths are skipped.
fn extract_yaml_zip(root: &std::path::Path, bytes: &[u8]) -> Result<(Vec<String>, Vec<String>), String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    let mut errors = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if !(name.ends_with(".yaml") || name.ends_with(".yml")) {
            continue;
        }
        let abs = match safe_join(root, &name) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("{name}: {e}"));
                continue;
            }
        };
        let mut content = Vec::new();
        if let Err(e) = entry.read_to_end(&mut content) {
            errors.push(format!("{name}: {e}"));
            continue;
        }
        if let Some(parent) = abs.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&abs, &content) {
            Ok(_) => written.push(name),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    Ok((written, errors))
}

fn zip_response(bytes: Vec<u8>, filename: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        bytes,
    )
        .into_response()
}

// --- csv parsing -----------------------------------------------------------

/// Parse CSV text into rows keyed by the header row. Handles quoted fields with
/// embedded commas, quotes (`""`), and newlines.
pub(crate) fn parse_csv(text: &str) -> Vec<Map<String, Value>> {
    let records = split_csv_records(text);
    let mut iter = records.into_iter();
    let Some(header) = iter.next() else {
        return Vec::new();
    };
    iter.map(|fields| {
        let mut row = Map::new();
        for (i, col) in header.iter().enumerate() {
            let value = fields.get(i).cloned().unwrap_or_default();
            row.insert(col.clone(), Value::String(value));
        }
        row
    })
    .collect()
}

/// Split CSV text into records of fields, honouring RFC-4180 quoting.
fn split_csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut field));
            }
            '\r' if !in_quotes => {}
            '\n' if !in_quotes => {
                fields.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut fields));
            }
            other => field.push(other),
        }
    }
    if !field.is_empty() || !fields.is_empty() {
        fields.push(field);
        records.push(fields);
    }
    records.retain(|r| !(r.len() == 1 && r[0].is_empty()));
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_basic() {
        let rows = parse_csv("name,target\neasy,6\nhard,12\n");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], json!("easy"));
        assert_eq!(rows[1]["target"], json!("12"));
    }

    #[test]
    fn parse_csv_quoted_fields() {
        let rows = parse_csv("name,note\n\"a,b\",\"line1\nline2\"\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], json!("a,b"));
        assert_eq!(rows[0]["note"], json!("line1\nline2"));
    }

    #[test]
    fn parse_csv_escaped_quote() {
        let rows = parse_csv("name\n\"say \"\"hi\"\"\"\n");
        assert_eq!(rows[0]["name"], json!("say \"hi\""));
    }

    #[test]
    fn zip_roundtrip() {
        let bytes = zip_single("a.yaml", b"id: x").unwrap();
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut entry = archive.by_index(0).unwrap();
        assert_eq!(entry.name(), "a.yaml");
        let mut content = String::new();
        entry.read_to_string(&mut content).unwrap();
        assert_eq!(content, "id: x");
    }
}
