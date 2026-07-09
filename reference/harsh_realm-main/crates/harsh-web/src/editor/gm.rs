//! GM tools (`/api/gm/*`) — direct, admin-driven mutations to the active world:
//! teleport, spawn, give item, and set HP/gold/XP/attribute. These write to the
//! authoritative stores (`entities`, `character_state`, `entity_resources`) so
//! both the editor view and the live game reflect the change.

use axum::extract::State;
use axum::response::Response;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use harsh_core::db::WorldDatabase;
use harsh_core::repositories::entity::EntityRepository;
use harsh_core::repositories::resources::{ResourceInstance, GOLD_RESOURCE_ID, HP_RESOURCE_ID};

use super::respond;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/gm/teleport", post(teleport))
        .route("/api/gm/spawn-entity", post(spawn_entity))
        .route("/api/gm/give-item", post(give_item))
        .route("/api/gm/set-hp", post(set_hp))
        .route("/api/gm/set-gold", post(set_gold))
        .route("/api/gm/set-xp", post(set_xp))
        .route("/api/gm/set-attr", post(set_attr))
}

/// Entity display name, or the id if the entity is gone. A lookup error is
/// logged (not swallowed) before falling back to the id.
fn entity_name(db: &WorldDatabase, id: &str) -> String {
    match db.fetch_one("SELECT name FROM entities WHERE id = ?", &[&id]) {
        Ok(row) => row
            .and_then(|row| row.get("name").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_else(|| id.to_string()),
        Err(e) => {
            eprintln!("entity_name: lookup failed for {id}: {e}");
            id.to_string()
        }
    }
}

/// Whether the entity has a `character_state` row.
fn has_character_state(db: &WorldDatabase, id: &str) -> Result<bool, String> {
    Ok(db
        .fetch_one("SELECT entity_id FROM character_state WHERE entity_id = ?", &[&id])?
        .is_some())
}

// --- teleport --------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct TeleportBody {
    pub entity_id: String,
    pub q: i64,
    pub r: i64,
}

async fn teleport(State(s): State<AppState>, Json(b): Json<TeleportBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                EntityRepository::new(db).update_entity_position(&b.entity_id, b.q, b.r, None)?;
                if has_character_state(db, &b.entity_id)? {
                    db.execute(
                        "UPDATE character_state SET position_q = ?, position_r = ? WHERE entity_id = ?",
                        &[&b.q, &b.r, &b.entity_id],
                    )?;
                }
                Ok(json!({ "name": entity_name(db, &b.entity_id), "q": b.q, "r": b.r }))
            })
            .await,
    )
}

// --- spawn -----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SpawnBody {
    pub entity_type: String,
    pub name: String,
    pub q: i64,
    pub r: i64,
    #[serde(default)]
    pub data: Map<String, Value>,
}

async fn spawn_entity(State(s): State<AppState>, Json(b): Json<SpawnBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let id = uuid::Uuid::new_v4().to_string();
                EntityRepository::new(db).create_entity(&id, &b.entity_type, &b.name, b.q, b.r, &b.data)?;
                Ok(json!({ "id": id, "name": b.name, "entity_type": b.entity_type, "q": b.q, "r": b.r }))
            })
            .await,
    )
}

// --- give item -------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct GiveItemBody {
    pub entity_id: String,
    pub item: Value,
}

async fn give_item(State(s): State<AppState>, Json(b): Json<GiveItemBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let (col, table, key) = if has_character_state(db, &b.entity_id)? {
                    ("equipment_json", "character_state", "entity_id")
                } else {
                    ("data", "entities", "id")
                };
                // Read current equipment list (from the *_json column, or the
                // `equipment` field of the entities.data blob).
                let row = db
                    .fetch_one(&format!("SELECT {col} FROM {table} WHERE {key} = ?"), &[&b.entity_id])?
                    .ok_or_else(|| format!("entity not found: {}", b.entity_id))?;
                let raw = row.get(col).and_then(Value::as_str).unwrap_or("");
                let mut equipment = if table == "character_state" {
                    serde_json::from_str::<Vec<Value>>(raw).unwrap_or_default()
                } else {
                    serde_json::from_str::<Value>(raw)
                        .ok()
                        .and_then(|v| v.get("equipment").and_then(Value::as_array).cloned())
                        .unwrap_or_default()
                };
                equipment.push(b.item.clone());
                let count = equipment.len();
                if table == "character_state" {
                    let serialized = serde_json::to_string(&equipment).map_err(|e| e.to_string())?;
                    db.execute(
                        "UPDATE character_state SET equipment_json = ? WHERE entity_id = ?",
                        &[&serialized, &b.entity_id],
                    )?;
                } else {
                    let mut blob: Map<String, Value> =
                        serde_json::from_str(raw).unwrap_or_default();
                    blob.insert("equipment".into(), Value::Array(equipment));
                    let serialized = serde_json::to_string(&blob).map_err(|e| e.to_string())?;
                    db.execute("UPDATE entities SET data = ? WHERE id = ?", &[&serialized, &b.entity_id])?;
                }
                Ok(json!({ "name": entity_name(db, &b.entity_id), "equipment_count": count }))
            })
            .await,
    )
}

// --- set hp ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SetHpBody {
    pub entity_id: String,
    pub hp: i64,
}

async fn set_hp(State(s): State<AppState>, Json(b): Json<SetHpBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                if has_character_state(db, &b.entity_id)? {
                    db.execute(
                        "UPDATE character_state SET hp = ? WHERE entity_id = ?",
                        &[&b.hp, &b.entity_id],
                    )?;
                }
                set_resource(db, &b.entity_id, HP_RESOURCE_ID, b.hp as i32)?;
                Ok(json!({ "name": entity_name(db, &b.entity_id), "hp": b.hp }))
            })
            .await,
    )
}

// --- set gold --------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SetGoldBody {
    pub entity_id: String,
    pub gold: i64,
}

async fn set_gold(State(s): State<AppState>, Json(b): Json<SetGoldBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                set_resource(db, &b.entity_id, GOLD_RESOURCE_ID, b.gold as i32)?;
                // Mirror into the character's class_abilities blob so the character
                // payload (which reads gold from there) stays consistent.
                if has_character_state(db, &b.entity_id)? {
                    patch_class_ability_int(db, &b.entity_id, "gold", b.gold)?;
                }
                Ok(json!({ "name": entity_name(db, &b.entity_id), "gold": b.gold }))
            })
            .await,
    )
}

// --- set xp ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SetXpBody {
    pub entity_id: String,
    pub xp: i64,
}

async fn set_xp(State(s): State<AppState>, Json(b): Json<SetXpBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                db.execute(
                    "UPDATE character_state SET xp = ? WHERE entity_id = ?",
                    &[&b.xp, &b.entity_id],
                )?;
                Ok(json!({ "name": entity_name(db, &b.entity_id), "xp": b.xp }))
            })
            .await,
    )
}

// --- set attribute ---------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SetAttrBody {
    pub entity_id: String,
    pub attribute: String,
    pub value: i64,
}

async fn set_attr(State(s): State<AppState>, Json(b): Json<SetAttrBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let row = db
                    .fetch_one(
                        "SELECT attributes_json FROM character_state WHERE entity_id = ?",
                        &[&b.entity_id],
                    )?
                    .ok_or_else(|| format!("character not found: {}", b.entity_id))?;
                let mut attrs: Map<String, Value> = row
                    .get("attributes_json")
                    .and_then(Value::as_str)
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                attrs.insert(b.attribute.clone(), json!(b.value));
                let serialized = serde_json::to_string(&attrs).map_err(|e| e.to_string())?;
                db.execute(
                    "UPDATE character_state SET attributes_json = ? WHERE entity_id = ?",
                    &[&serialized, &b.entity_id],
                )?;
                Ok(json!({ "name": entity_name(db, &b.entity_id), "attribute": b.attribute, "value": b.value }))
            })
            .await,
    )
}

// --- helpers ---------------------------------------------------------------

/// Set a resource's `current` to `value`, preserving (or seeding) its max.
fn set_resource(db: &WorldDatabase, entity_id: &str, resource_id: &str, value: i32) -> Result<(), String> {
    let repo = harsh_core::repositories::resources::ResourceRepository::new(db);
    let existing = repo.get(entity_id, resource_id)?;
    let max = existing.as_ref().and_then(|r| r.max).or(Some(value));
    let last_regen_tick = existing.as_ref().map(|r| r.last_regen_tick).unwrap_or(0);
    repo.set(&ResourceInstance {
        entity_id: entity_id.to_string(),
        resource_id: resource_id.to_string(),
        current: value,
        max,
        last_regen_tick,
    })
}

/// Patch one integer key inside `character_state.class_abilities_json`.
fn patch_class_ability_int(db: &WorldDatabase, entity_id: &str, key: &str, value: i64) -> Result<(), String> {
    let row = db
        .fetch_one("SELECT class_abilities_json FROM character_state WHERE entity_id = ?", &[&entity_id])?
        .ok_or_else(|| format!("character not found: {entity_id}"))?;
    let mut abilities: Map<String, Value> = row
        .get("class_abilities_json")
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    abilities.insert(key.to_string(), json!(value));
    let serialized = serde_json::to_string(&abilities).map_err(|e| e.to_string())?;
    db.execute(
        "UPDATE character_state SET class_abilities_json = ? WHERE entity_id = ?",
        &[&serialized, &entity_id],
    )?;
    Ok(())
}
