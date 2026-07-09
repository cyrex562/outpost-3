//! Admin config endpoints (`/api/admin/*`).
//!
//! Thin wrappers over `harsh-core`'s `AdminService` + GM-state repository,
//! scoped to the active world. Editable config tables (skill mappings,
//! difficulty targets, disposition outcomes, encounter weights, faction asset
//! stats, XP progression, random tables, items, creature templates), world meta,
//! and the oracle chaos factor.
//!
//! Note: cross-world editing via `?world=` and the heavier editor/import-export
//! surfaces are not yet ported; these operate on the loaded world.

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::{json, Value};

use harsh_core::admin::service::AdminService;
use harsh_core::admin_api::{
    DifficultyTargetUpdateRequest, DispositionOutcomeUpdateRequest, EncounterWeightUpdateRequest,
    RandomTableUpdateRequest, SkillMappingUpdateRequest, XpProgressionUpdateRequest,
};
use harsh_core::repositories::gm_state::GMStateRepository;

use crate::state::AppState;

const CHAOS_KEY: &str = "oracle_chaos_factor";

/// Admin config routes, merged into the main router before `with_state`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/skill-mappings", get(list_skill_mappings))
        .route("/api/admin/skill-mappings/reset-all", post(reset_all_skill_mappings))
        .route("/api/admin/skill-mappings/:verb/reset", post(reset_skill_mapping))
        .route(
            "/api/admin/skill-mappings/:verb",
            put(set_skill_mapping).delete(delete_skill_mapping),
        )
        .route("/api/admin/difficulty-targets", get(list_difficulty_targets))
        .route("/api/admin/difficulty-targets/:name/reset", post(reset_difficulty_target))
        .route("/api/admin/difficulty-targets/:name", put(set_difficulty_target))
        .route("/api/admin/disposition-outcomes", get(list_disposition_outcomes))
        .route("/api/admin/disposition-outcomes/:key", put(set_disposition_outcome))
        .route("/api/admin/encounter-weights", get(list_encounter_weights))
        .route("/api/admin/encounter-weights/:disposition/:tag", put(set_encounter_weight))
        .route("/api/admin/faction-assets", get(list_faction_assets))
        .route("/api/admin/xp-progression", get(list_xp_progression))
        .route("/api/admin/xp-progression/reset-all", post(reset_all_xp_progression))
        .route("/api/admin/xp-progression/:level/reset", post(reset_xp_progression))
        .route("/api/admin/xp-progression/:level", put(set_xp_progression))
        .route("/api/admin/random-tables", get(list_random_tables))
        .route("/api/admin/random-tables/reset-all", post(reset_all_random_tables))
        .route("/api/admin/random-tables/:id/reset", post(reset_random_table))
        .route(
            "/api/admin/random-tables/:id",
            put(set_random_table).delete(delete_random_table),
        )
        .route("/api/admin/items-data", get(list_items_data))
        .route("/api/admin/items-data/reset-all", post(reset_all_items_data))
        .route("/api/admin/items-data/:id/reset", post(reset_item_data))
        .route(
            "/api/admin/items-data/:id",
            put(set_item_data).delete(delete_item_data),
        )
        .route("/api/admin/creature-templates", get(list_creature_templates))
        .route(
            "/api/admin/creature-templates/reset-all",
            post(reset_all_creature_templates),
        )
        .route(
            "/api/admin/creature-templates/:id/reset",
            post(reset_creature_template),
        )
        .route(
            "/api/admin/creature-templates/:id",
            put(set_creature_template).delete(delete_creature_template),
        )
        .route("/api/admin/export-config", post(export_config))
        .route("/api/admin/world-meta", get(get_world_meta))
        .route("/api/admin/world-meta/:key", put(set_world_meta))
        .route("/api/admin/oracle-state", get(get_oracle_state).put(set_oracle_state))
}

fn respond(result: Result<Value, String>) -> Response {
    crate::response::respond_with(result, "AdminError")
}

/// Run an AdminService closure against the active world and JSON-encode the result.
async fn with_admin<F>(state: &AppState, f: F) -> Response
where
    F: FnOnce(&AdminService) -> Result<Value, String> + Send + 'static,
{
    let result = state
        .session
        .read(move |db| f(&AdminService::new(db)))
        .await;
    respond(result)
}

// --- skill mappings --------------------------------------------------------

pub async fn list_skill_mappings(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_skill_mappings()?))).await
}

pub async fn set_skill_mapping(
    State(s): State<AppState>,
    Path(verb): Path<String>,
    Json(b): Json<SkillMappingUpdateRequest>,
) -> Response {
    with_admin(&s, move |a| {
        Ok(json!(a.set_skill_mapping(
            &verb,
            &b.skill,
            &b.attribute,
            b.base_difficulty,
            b.opposed,
            &b.description,
            &b.notes,
        )?))
    })
    .await
}

pub async fn delete_skill_mapping(State(s): State<AppState>, Path(verb): Path<String>) -> Response {
    with_admin(&s, move |a| Ok(json!({ "ok": a.delete_skill_mapping(&verb)? }))).await
}

pub async fn reset_skill_mapping(State(s): State<AppState>, Path(verb): Path<String>) -> Response {
    with_admin(&s, move |a| Ok(json!(a.reset_skill_mapping(&verb)?))).await
}

pub async fn reset_all_skill_mappings(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.reset_all_skill_mappings()?))).await
}

// --- difficulty targets ----------------------------------------------------

pub async fn list_difficulty_targets(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_difficulty_targets()?))).await
}

pub async fn set_difficulty_target(
    State(s): State<AppState>,
    Path(name): Path<String>,
    Json(b): Json<DifficultyTargetUpdateRequest>,
) -> Response {
    with_admin(&s, move |a| {
        Ok(json!(a.set_difficulty_target(&name, b.target, &b.description)?))
    })
    .await
}

pub async fn reset_difficulty_target(State(s): State<AppState>, Path(name): Path<String>) -> Response {
    with_admin(&s, move |a| Ok(json!(a.reset_difficulty_target(&name)?))).await
}

// --- disposition outcomes --------------------------------------------------

pub async fn list_disposition_outcomes(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_disposition_outcomes()?))).await
}

pub async fn set_disposition_outcome(
    State(s): State<AppState>,
    Path(key): Path<String>,
    Json(b): Json<DispositionOutcomeUpdateRequest>,
) -> Response {
    with_admin(&s, move |a| {
        Ok(json!(a.set_disposition_outcome(&key, b.delta, &b.description)?))
    })
    .await
}

// --- encounter weights -----------------------------------------------------

pub async fn list_encounter_weights(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_encounter_weights()?))).await
}

pub async fn set_encounter_weight(
    State(s): State<AppState>,
    Path((disposition, tag)): Path<(String, String)>,
    Json(b): Json<EncounterWeightUpdateRequest>,
) -> Response {
    with_admin(&s, move |a| {
        Ok(json!(a.set_encounter_weight(&disposition, &tag, b.weight_modifier)?))
    })
    .await
}

// --- faction asset stats ---------------------------------------------------

pub async fn list_faction_assets(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_faction_asset_stats()?))).await
}

// --- xp progression --------------------------------------------------------

pub async fn list_xp_progression(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_xp_progression()?))).await
}

pub async fn set_xp_progression(
    State(s): State<AppState>,
    Path(level): Path<i32>,
    Json(b): Json<XpProgressionUpdateRequest>,
) -> Response {
    with_admin(&s, move |a| Ok(json!(a.set_xp_progression(level, b.xp_needed)?))).await
}

pub async fn reset_xp_progression(State(s): State<AppState>, Path(level): Path<i32>) -> Response {
    with_admin(&s, move |a| Ok(json!(a.reset_xp_progression(level)?))).await
}

pub async fn reset_all_xp_progression(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.reset_all_xp_progression()?))).await
}

// --- random tables ---------------------------------------------------------

pub async fn list_random_tables(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_random_tables()?))).await
}

pub async fn set_random_table(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<RandomTableUpdateRequest>,
) -> Response {
    with_admin(&s, move |a| {
        Ok(json!(a.set_random_table(
            &id,
            &b.category,
            &b.name,
            &b.entries,
            &b.tags,
            &b.source,
        )?))
    })
    .await
}

pub async fn delete_random_table(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    with_admin(&s, move |a| Ok(json!({ "ok": a.delete_random_table(&id)? }))).await
}

/// Reset a per-world random table to its pack default by removing the override
/// row; pack content then applies at runtime.
pub async fn reset_random_table(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    delete_override(&s, "random_tables", "id", id).await
}

pub async fn reset_all_random_tables(State(s): State<AppState>) -> Response {
    clear_overrides(&s, "random_tables").await
}

// --- items + creature templates --------------------------------------------

pub async fn list_items_data(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_items()?))).await
}

pub async fn set_item_data(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    upsert_named_json(&s, "items", id, body).await
}

pub async fn delete_item_data(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    with_admin(&s, move |a| Ok(json!({ "ok": a.delete_item(&id)? }))).await
}

pub async fn reset_item_data(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    delete_override(&s, "items", "id", id).await
}

pub async fn reset_all_items_data(State(s): State<AppState>) -> Response {
    clear_overrides(&s, "items").await
}

pub async fn list_creature_templates(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.list_creature_templates()?))).await
}

pub async fn set_creature_template(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> Response {
    upsert_named_json(&s, "creature_templates", id, body).await
}

pub async fn delete_creature_template(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    with_admin(&s, move |a| Ok(json!({ "ok": a.delete_creature_template(&id)? }))).await
}

pub async fn reset_creature_template(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    delete_override(&s, "creature_templates", "id", id).await
}

pub async fn reset_all_creature_templates(State(s): State<AppState>) -> Response {
    clear_overrides(&s, "creature_templates").await
}

/// Upsert a `(id, name, data, source)` editor record. `name` is lifted out of
/// the body; every other key (except `id`/`source`) is stored in the `data` JSON
/// column. `source` is stamped `"world"` to mark it a per-world override.
async fn upsert_named_json(s: &AppState, table: &'static str, id: String, body: Value) -> Response {
    let result = s
        .session
        .read(move |db| {
            let obj = body.as_object().cloned().unwrap_or_default();
            let name = obj.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let mut data = obj.clone();
            data.remove("id");
            data.remove("name");
            data.remove("source");
            let data_json = serde_json::to_string(&data).unwrap_or_else(|_| "{}".into());
            db.execute(
                &format!(
                    "INSERT OR REPLACE INTO {table} (id, name, data, source) VALUES (?, ?, ?, 'world')"
                ),
                &[&id, &name, &data_json],
            )?;
            Ok(json!({ "ok": true, "id": id }))
        })
        .await;
    respond(result)
}

/// Delete one override row (revert to pack default).
async fn delete_override(s: &AppState, table: &'static str, key: &'static str, id: String) -> Response {
    let result = s
        .session
        .read(move |db| {
            let n = db.execute(&format!("DELETE FROM {table} WHERE {key} = ?"), &[&id])?;
            Ok(json!({ "ok": true, "removed": n }))
        })
        .await;
    respond(result)
}

/// Delete all per-world override rows from a table, reverting to pack defaults.
///
/// Only rows explicitly stamped `source = 'world'` (the marker the override write
/// paths set) are removed. Content-authored, imported, or pack-seeded rows — which
/// may carry a NULL/empty or pack-id `source` — are preserved, so a reset never
/// wipes non-override content.
async fn clear_overrides(s: &AppState, table: &'static str) -> Response {
    let result = s
        .session
        .read(move |db| {
            let n = db.execute(
                &format!("DELETE FROM {table} WHERE source = 'world'"),
                &[],
            )?;
            Ok(json!({ "ok": true, "removed": n }))
        })
        .await;
    respond(result)
}

// --- export config ---------------------------------------------------------

pub async fn export_config(State(s): State<AppState>) -> Response {
    with_admin(&s, |a| Ok(json!(a.export_world_config()?))).await
}

// --- world meta ------------------------------------------------------------

pub async fn get_world_meta(State(s): State<AppState>) -> Response {
    let result = s
        .session
        .read(|db| {
            let rows = db.fetch_all("SELECT key, value FROM world_meta", &[])?;
            let mut map = serde_json::Map::new();
            for row in rows {
                if let (Some(k), Some(v)) = (
                    row.get("key").and_then(Value::as_str),
                    row.get("value").and_then(Value::as_str),
                ) {
                    map.insert(k.to_string(), json!(v));
                }
            }
            Ok(Value::Object(map))
        })
        .await;
    respond(result)
}

#[derive(Debug, serde::Deserialize)]
pub struct MetaValue {
    pub value: String,
}

pub async fn set_world_meta(
    State(s): State<AppState>,
    Path(key): Path<String>,
    Json(b): Json<MetaValue>,
) -> Response {
    let result = s
        .session
        .read(move |db| {
            db.set_meta(&key, &b.value)?;
            Ok(json!({ "ok": true }))
        })
        .await;
    respond(result)
}

// --- oracle state (chaos factor) -------------------------------------------

pub async fn get_oracle_state(State(s): State<AppState>) -> Response {
    let result = s
        .session
        .read(|db| {
            let chaos = GMStateRepository::new(db).get_int(CHAOS_KEY, 5)?;
            Ok(json!({ "chaos_factor": chaos }))
        })
        .await;
    respond(result)
}

#[derive(Debug, serde::Deserialize)]
pub struct OracleStateUpdate {
    #[serde(default)]
    pub chaos_factor: Option<i64>,
}

pub async fn set_oracle_state(
    State(s): State<AppState>,
    Json(b): Json<OracleStateUpdate>,
) -> Response {
    let result = s
        .session
        .read(move |db| {
            let repo = GMStateRepository::new(db);
            if let Some(chaos) = b.chaos_factor {
                repo.set_value(CHAOS_KEY, &chaos.to_string())?;
            }
            Ok(json!({ "chaos_factor": repo.get_int(CHAOS_KEY, 5)? }))
        })
        .await;
    respond(result)
}
