/// Import / Export HTTP handlers.
///
/// GET  /admin/export/{table}      → JSON download
/// GET  /admin/export/{table}/csv  → CSV download
/// POST /admin/import/{table}      → JSON body upload (array of records)

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::db::{ExportTable, SharedRepository, json_to_csv};

#[derive(Deserialize)]
pub struct TablePath {
    pub table: String,
}

/// GET /admin/export/{table}
/// Returns all records as a JSON array.
pub async fn export_table(
    path: web::Path<TablePath>,
    repo: web::Data<SharedRepository>,
) -> HttpResponse {
    let table_name = &path.table;

    let table = match ExportTable::from_str(table_name) {
        Some(t) => t,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Unknown table: {}", table_name)
            }))
        }
    };

    match repo.export_json(table) {
        Ok(records) => {
            let filename = format!("{}.json", table_name);
            HttpResponse::Ok()
                .insert_header(("Content-Type", "application/json"))
                .insert_header((
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", filename),
                ))
                .json(records)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Export failed: {}", e)
        })),
    }
}

/// GET /admin/export/{table}/csv
/// Returns all records as a CSV file.
pub async fn export_table_csv(
    path: web::Path<TablePath>,
    repo: web::Data<SharedRepository>,
) -> HttpResponse {
    let table_name = &path.table;

    let table = match ExportTable::from_str(table_name) {
        Some(t) => t,
        None => {
            return HttpResponse::BadRequest().body(format!("Unknown table: {}", table_name))
        }
    };

    match repo.export_json(table) {
        Ok(records) => {
            let csv = json_to_csv(&records);
            let filename = format!("{}.csv", table_name);
            HttpResponse::Ok()
                .insert_header(("Content-Type", "text/csv"))
                .insert_header((
                    "Content-Disposition",
                    format!("attachment; filename=\"{}\"", filename),
                ))
                .body(csv)
        }
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("Export failed: {}", e))
        }
    }
}

/// POST /admin/import/{table}
/// Accepts a JSON array of records and imports them into the specified table.
///
/// Returns JSON: {"imported": N, "table": "..."}
pub async fn import_table(
    path: web::Path<TablePath>,
    body: web::Json<Vec<JsonValue>>,
    repo: web::Data<SharedRepository>,
) -> HttpResponse {
    let table_name = &path.table;

    let table = match ExportTable::from_str(table_name) {
        Some(t) => t,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Unknown table: {}", table_name)
            }))
        }
    };

    let records = body.into_inner();
    let record_count = records.len();

    match repo.import_json(table, records) {
        Ok(imported) => HttpResponse::Ok().json(serde_json::json!({
            "imported": imported,
            "skipped": record_count - imported,
            "table": table_name
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Import failed: {}", e)
        })),
    }
}
