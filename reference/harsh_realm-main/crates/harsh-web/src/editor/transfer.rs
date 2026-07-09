//! Per-table import/export endpoints (JSON + CSV) and bulk export/import.
//!
//! Backed by [`harsh_core::editor::transfer`]'s whitelist of exportable tables.
//! Table names are always validated against that whitelist before being
//! interpolated into SQL; row values are bound as parameters, and unknown
//! columns are dropped against the live table schema.

use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use rusqlite::ToSql;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use harsh_core::db::WorldDatabase;
use harsh_core::editor::transfer::{export_query, is_exportable, EXPORT_TABLES};

use super::respond;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/export/:table", axum::routing::get(export_table))
        .route("/api/admin/import/:table", axum::routing::post(import_table))
        .route("/api/admin/export-all", axum::routing::get(export_all))
        .route("/api/admin/import-all", axum::routing::post(import_all))
}

// --- export ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    #[serde(default)]
    pub format: Option<String>,
}

async fn export_table(
    State(s): State<AppState>,
    Path(table): Path<String>,
    Query(q): Query<ExportQuery>,
) -> Response {
    let Some(query) = export_query(&table) else {
        return (StatusCode::BAD_REQUEST, format!("table not exportable: {table}")).into_response();
    };
    let rows = s.session.read(move |db| {
        let rows = db.fetch_all(query, &[])?;
        Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
    });
    let rows = rows.await;
    let want_csv = q.format.as_deref() == Some("csv");
    match rows {
        Ok(Value::Array(items)) if want_csv => {
            let body = rows_to_csv(&items);
            (
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{table}.csv\""),
                    ),
                ],
                body,
            )
                .into_response()
        }
        other => respond(other),
    }
}

async fn export_all(State(s): State<AppState>) -> Response {
    // One short session read per table (rather than one closure over all ~22), so
    // the actor thread is released between tables and gameplay isn't blocked for
    // the whole dump. Trade-off: the dump is a per-table snapshot, not one global
    // snapshot — acceptable for an editor export.
    let mut out = Map::new();
    for (table, query) in EXPORT_TABLES {
        let query = *query;
        match s
            .session
            .read(move |db| {
                let rows = db.fetch_all(query, &[])?;
                Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
            })
            .await
        {
            Ok(rows) => {
                out.insert((*table).to_string(), rows);
            }
            Err(e) => return respond(Err(format!("export {table}: {e}"))),
        }
    }
    respond(Ok(Value::Object(out)))
}

// --- import ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ImportBody {
    #[serde(default)]
    pub rows: Vec<Map<String, Value>>,
}

async fn import_table(
    State(s): State<AppState>,
    Path(table): Path<String>,
    Json(body): Json<ImportBody>,
) -> Response {
    if !is_exportable(&table) {
        return (StatusCode::BAD_REQUEST, format!("table not importable: {table}")).into_response();
    }
    respond(
        s.session
            .read(move |db| {
                let (imported, errors) = import_rows(db, &table, &body.rows)?;
                Ok(json!({ "imported": imported, "errors": errors }))
            })
            .await,
    )
}

#[derive(Debug, Deserialize)]
pub struct ImportAllBody {
    #[serde(default)]
    pub tables: Map<String, Value>,
}

async fn import_all(State(s): State<AppState>, Json(body): Json<ImportAllBody>) -> Response {
    // One short session read per table (each `import_rows` is already best-effort
    // with per-table error collection — there's no cross-table transaction to
    // preserve), so a large import doesn't hold the actor for the whole load.
    let mut summary = Map::new();
    for (table, rows_value) in body.tables {
        if !is_exportable(&table) {
            summary.insert(table, json!({ "imported": 0, "errors": ["unknown table"] }));
            continue;
        }
        let rows: Vec<Map<String, Value>> = rows_value
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_object().cloned()).collect())
            .unwrap_or_default();
        let table_for_db = table.clone();
        match s
            .session
            .read(move |db| {
                let (imported, errors) = import_rows(db, &table_for_db, &rows)?;
                Ok(json!({ "imported": imported, "errors": errors }))
            })
            .await
        {
            Ok(v) => {
                summary.insert(table, v);
            }
            Err(e) => return respond(Err(format!("import {table}: {e}"))),
        }
    }
    respond(Ok(Value::Object(summary)))
}

/// Insert (replace) rows into a whitelisted table, dropping keys that are not
/// real columns. Returns `(count_imported, errors)`.
pub(crate) fn import_rows(
    db: &WorldDatabase,
    table: &str,
    rows: &[Map<String, Value>],
) -> Result<(usize, Vec<String>), String> {
    let valid_cols = table_columns(db, table)?;
    let mut imported = 0usize;
    let mut errors: Vec<String> = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let cols: Vec<&String> = row.keys().filter(|k| valid_cols.contains(*k)).collect();
        if cols.is_empty() {
            errors.push(format!("row {idx}: no recognised columns"));
            continue;
        }
        let placeholders = vec!["?"; cols.len()].join(", ");
        let col_list = cols.iter().map(|c| format!("\"{c}\"")).collect::<Vec<_>>().join(", ");
        let sql = format!("INSERT OR REPLACE INTO \"{table}\" ({col_list}) VALUES ({placeholders})");
        let boxed: Vec<Box<dyn ToSql>> = cols.iter().map(|c| json_to_sql(&row[*c])).collect();
        let refs: Vec<&dyn ToSql> = boxed.iter().map(|b| b.as_ref()).collect();
        match db.execute(&sql, &refs) {
            Ok(_) => imported += 1,
            Err(e) => errors.push(format!("row {idx}: {e}")),
        }
    }
    Ok((imported, errors))
}

/// Real column names of a whitelisted table (via `PRAGMA table_info`).
fn table_columns(db: &WorldDatabase, table: &str) -> Result<std::collections::HashSet<String>, String> {
    // `table` is whitelisted by the caller, so interpolation is safe; PRAGMA
    // cannot bind the table name as a parameter.
    let rows = db.fetch_all(&format!("PRAGMA table_info(\"{table}\")"), &[])?;
    Ok(rows
        .iter()
        .filter_map(|r| r.get("name").and_then(Value::as_str).map(str::to_string))
        .collect())
}

/// Convert a JSON value into a boxed SQL parameter. Containers are stored as
/// their serialised JSON string (these columns are TEXT).
pub(crate) fn json_to_sql(value: &Value) -> Box<dyn ToSql> {
    match value {
        Value::Null => Box::new(Option::<String>::None),
        Value::Bool(b) => Box::new(*b as i64),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Box::new(i)
            } else {
                Box::new(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => Box::new(s.clone()),
        other => Box::new(other.to_string()),
    }
}

// --- csv -------------------------------------------------------------------

/// Render rows to CSV. Columns are the union of keys in first-seen order.
fn rows_to_csv(rows: &[Value]) -> String {
    let mut columns: Vec<String> = Vec::new();
    for row in rows {
        if let Some(obj) = row.as_object() {
            for key in obj.keys() {
                if !columns.iter().any(|c| c == key) {
                    columns.push(key.clone());
                }
            }
        }
    }
    let mut out = String::new();
    out.push_str(&columns.iter().map(|c| csv_field(c)).collect::<Vec<_>>().join(","));
    out.push('\n');
    for row in rows {
        let obj = row.as_object();
        let line: Vec<String> = columns
            .iter()
            .map(|col| {
                let cell = obj.and_then(|o| o.get(col)).unwrap_or(&Value::Null);
                csv_field(&match cell {
                    Value::Null => String::new(),
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
            })
            .collect();
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

/// Quote a CSV field when it contains a comma, quote, or newline.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
