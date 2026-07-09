//! Faction editor endpoints — faction CRUD plus per-faction asset management.

use axum::extract::{Path, State};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use harsh_core::editor_api::factions::FactionUpdateRequest;
use harsh_core::faction::{FactionAssetData, FactionAssetState, FactionData};
use harsh_core::repositories::faction::FactionRepository;

use super::respond;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/factions", get(list_factions).post(create_faction))
        .route(
            "/api/admin/factions/:id",
            get(get_faction).put(update_faction).delete(delete_faction),
        )
        .route("/api/admin/factions/:id/assets", post(add_asset))
        .route("/api/admin/factions/:id/assets/:asset_id", axum::routing::delete(delete_asset))
}

async fn list_factions(State(s): State<AppState>) -> Response {
    respond(
        s.session
            .read(|db| {
                let factions = FactionRepository::new(db).list_factions()?;
                serde_json::to_value(factions).map_err(|e| e.to_string())
            })
            .await,
    )
}

async fn get_faction(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let repo = FactionRepository::new(db);
                let Some(faction) = repo.get_faction(&id)? else {
                    return Err(format!("faction not found: {id}"));
                };
                let mut obj = match serde_json::to_value(&faction).map_err(|e| e.to_string())? {
                    Value::Object(map) => map,
                    _ => return Err("faction did not serialize to an object".into()),
                };
                let assets = repo.list_assets(Some(&id))?;
                obj.insert("assets".into(), serde_json::to_value(assets).map_err(|e| e.to_string())?);
                obj.insert("relations".into(), load_relations(db, &id)?);
                Ok(Value::Object(obj))
            })
            .await,
    )
}

/// Relations involving a faction, as `[{faction_a, faction_b, disposition}]`.
fn load_relations(db: &harsh_core::db::WorldDatabase, id: &str) -> Result<Value, String> {
    let rows = db.fetch_all(
        "SELECT faction_a, faction_b, disposition FROM faction_relations \
         WHERE faction_a = ? OR faction_b = ?",
        &[&id, &id],
    )?;
    let relations: Vec<Value> = rows
        .iter()
        .map(|row| {
            json!({
                "faction_a": row.get("faction_a").and_then(Value::as_str).unwrap_or(""),
                "faction_b": row.get("faction_b").and_then(Value::as_str).unwrap_or(""),
                "disposition": row.get("disposition").and_then(Value::as_str).unwrap_or("neutral"),
            })
        })
        .collect();
    Ok(Value::Array(relations))
}

#[derive(Debug, Deserialize)]
pub struct CreateFactionBody {
    #[serde(default)]
    pub name: Option<String>,
}

async fn create_faction(State(s): State<AppState>, Json(body): Json<CreateFactionBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let faction = FactionData {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: body.name.unwrap_or_else(|| "New Faction".to_string()),
                    hp: 7,
                    max_hp: 7,
                    force: 0,
                    cunning: 0,
                    wealth: 0,
                    xp: 0,
                    home_q: None,
                    home_r: None,
                    goals: Vec::new(),
                    tags: Vec::new(),
                    data: Default::default(),
                };
                FactionRepository::new(db).create_faction(&faction)?;
                serde_json::to_value(&faction).map_err(|e| e.to_string())
            })
            .await,
    )
}

async fn update_faction(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<FactionUpdateRequest>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                let updated = FactionRepository::new(db).update_faction(&id, &req)?;
                match updated {
                    Some(f) => serde_json::to_value(&f).map_err(|e| e.to_string()),
                    None => Err(format!("faction not found: {id}")),
                }
            })
            .await,
    )
}

async fn delete_faction(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    respond(
        s.session
            .read(move |db| {
                FactionRepository::new(db).delete_faction(&id)?;
                Ok(json!({ "ok": true }))
            })
            .await,
    )
}

#[derive(Debug, Deserialize)]
pub struct AddAssetBody {
    pub asset_type: String,
    #[serde(default)]
    pub category: String,
}

async fn add_asset(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<AddAssetBody>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                let repo = FactionRepository::new(db);
                if repo.get_faction(&id)?.is_none() {
                    return Err(format!("faction not found: {id}"));
                }
                // Seed HP from the asset-type stats when known; otherwise 1.
                let max_hp = repo
                    .get_asset_stats(&body.asset_type)?
                    .map(|stat| stat.max_hp)
                    .unwrap_or(1);
                let category = if body.category.is_empty() {
                    "force".to_string()
                } else {
                    body.category.clone()
                };
                let asset = FactionAssetData {
                    id: uuid::Uuid::new_v4().to_string(),
                    faction_id: id.clone(),
                    asset_type: body.asset_type.clone(),
                    category,
                    hp: max_hp,
                    max_hp,
                    location_q: None,
                    location_r: None,
                    data: Default::default(),
                };
                repo.create_asset(&asset, FactionAssetState::default())?;
                serde_json::to_value(&asset).map_err(|e| e.to_string())
            })
            .await,
    )
}

async fn delete_asset(
    State(s): State<AppState>,
    Path((_id, asset_id)): Path<(String, String)>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                FactionRepository::new(db).delete_asset(&asset_id)?;
                Ok(json!({ "ok": true }))
            })
            .await,
    )
}
