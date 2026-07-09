//! Content authoring endpoints (`/api/content/*`).
//!
//! Compile/reference/template are world-independent. Store/records/counts target
//! the active world's IR record store.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use harsh_core::compiler::{compile, Catalogs, VerbRegistry, RECORD_TYPES};
use harsh_core::content::loader::load_yaml;
use harsh_core::content::{IRRecordRepository, MIGRATABLE_CATEGORIES};

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct YamlBody {
    #[serde(default)]
    pub yaml: String,
}

fn err(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": "ContentError", "message": message }))).into_response()
}

/// Compile a YAML source string into an IR report (`{syntax_errors, unimplemented, records}`).
fn compile_report(yaml: &str) -> Value {
    let docs = match load_yaml(yaml) {
        Ok(value) => vec![value],
        Err(e) => {
            return json!({
                "syntax_errors": [{
                    "class": "syntax",
                    "message": format!("YAML parse error: {e}"),
                    "location": null,
                }],
                "unimplemented": [],
                "records": [],
            });
        }
    };
    let report = compile(&docs, &VerbRegistry::builtin(), &Catalogs::default());
    serde_json::to_value(&report).unwrap_or_else(|_| {
        json!({ "syntax_errors": [], "unimplemented": [], "records": [] })
    })
}

pub async fn compile_content(Json(body): Json<YamlBody>) -> Response {
    let report = tokio::task::spawn_blocking(move || compile_report(&body.yaml))
        .await
        .unwrap_or_else(|_| json!({}));
    Json(report).into_response()
}

pub async fn content_reference() -> Json<Value> {
    Json(json!({ "markdown": reference_markdown() }))
}

fn reference_markdown() -> String {
    let mut md = String::from(
        "# Content Key Reference\n\nAuthor game content as YAML records. \
         Each top-level key names a record type. Supported record types:\n\n",
    );
    for record_type in RECORD_TYPES {
        md.push_str(&format!("- `{record_type}`\n"));
    }
    md.push_str(
        "\n## Notes\n\n- `id` and `name` are required on every record.\n\
         - Dice expressions use `NdM` form (e.g. `1d6`, `2d8`).\n",
    );
    md
}

pub async fn content_template() -> Json<Value> {
    Json(json!({ "yaml": CONTENT_TEMPLATE }))
}

const CONTENT_TEMPLATE: &str = "creature:\n  id: example_beast\n  name: Example Beast\n  \
hd: 1\n  ac: 12\n  attack_bonus: 1\n  damage: 1d6\n  description: A placeholder creature.\n";

pub async fn legacy_categories() -> Json<Value> {
    let categories: Vec<Value> = MIGRATABLE_CATEGORIES
        .iter()
        .map(|c| json!({ "category": c, "count": 0 }))
        .collect();
    Json(json!({ "categories": categories }))
}

pub async fn store_content(State(state): State<AppState>, Json(body): Json<YamlBody>) -> Response {
    let result = state
        .session
        .read(move |db| {
            let docs = load_yaml(&body.yaml).map(|v| vec![v]).unwrap_or_default();
            let report = compile(&docs, &VerbRegistry::builtin(), &Catalogs::default());
            let report_json = serde_json::to_value(&report).unwrap_or_else(|_| json!({}));
            let stored = if report.syntax_errors.is_empty() && !report.records.is_empty() {
                let records: Vec<harsh_core::runtime::JsonObject> = report
                    .records
                    .iter()
                    .filter_map(|r| {
                        serde_json::to_value(r).ok().and_then(|v| v.as_object().cloned())
                    })
                    .collect();
                IRRecordRepository::new(db)
                    .upsert_many(&records, "content-studio")
                    .unwrap_or(0)
            } else {
                0
            };
            Ok(json!({ "report": report_json, "stored": stored }))
        })
        .await;
    match result {
        Ok(body) => {
            // Make the freshly-stored IR live in the running controller without a
            // world reload (best-effort; ignore if no world is loaded).
            let _ = state.session.refresh_content().await;
            Json(body).into_response()
        }
        Err(e) => err(StatusCode::CONFLICT, &e),
    }
}

pub async fn list_records(State(state): State<AppState>) -> Response {
    let result = state
        .session
        .read(|db| {
            let repo = IRRecordRepository::new(db);
            let records = repo.list_records(None).unwrap_or_default();
            let counts = repo.counts().unwrap_or_default();
            Ok(json!({ "records": records, "counts": counts }))
        })
        .await;
    // No world loaded → empty browser rather than an error.
    Json(result.unwrap_or_else(|_| json!({ "records": [], "counts": {} }))).into_response()
}

pub async fn content_counts(State(state): State<AppState>) -> Response {
    let result = state
        .session
        .read(|db| Ok(json!(IRRecordRepository::new(db).counts().unwrap_or_default())))
        .await;
    Json(result.unwrap_or_else(|_| json!({}))).into_response()
}

pub async fn record_yaml(
    State(state): State<AppState>,
    Path((component_type, id)): Path<(String, String)>,
) -> Response {
    let result = state
        .session
        .read(move |db| {
            match IRRecordRepository::new(db).get(&component_type, &id)? {
                Some(record) => {
                    let yaml = serde_yaml::to_string(&record).map_err(|e| e.to_string())?;
                    Ok(json!({ "yaml": yaml }))
                }
                None => Err(format!("record not found: {component_type}/{id}")),
            }
        })
        .await;
    match result {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::NOT_FOUND, &e),
    }
}

pub async fn delete_record(
    State(state): State<AppState>,
    Path((component_type, id)): Path<(String, String)>,
) -> Response {
    let result = state
        .session
        .read(move |db| {
            IRRecordRepository::new(db).delete(&component_type, &id)?;
            Ok(json!({ "ok": true }))
        })
        .await;
    match result {
        Ok(body) => Json(body).into_response(),
        Err(e) => err(StatusCode::CONFLICT, &e),
    }
}
