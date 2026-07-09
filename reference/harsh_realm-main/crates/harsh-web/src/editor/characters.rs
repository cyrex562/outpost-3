//! Character/entity editor endpoints.
//!
//! `entities` holds the generic row (id/name/type/location/alive/data); a player
//! character's derived stats live in the normalised `character_state` table. The
//! editor presents a single `data` blob, so GET assembles `character_state` (when
//! present) into `data`, and PUT writes the overlapping fields back to it.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::ToSql;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use harsh_core::character_recalc::recalculate_character;
use harsh_core::db::WorldDatabase;
use harsh_core::editor_api::characters::CharacterRecalcPreviewRequest;
use harsh_core::repositories::editor_entity::EditorEntityRepository;
use harsh_core::repositories::entity::EntityRepository;
use harsh_core::runtime::InventoryItemRecord;

use super::respond;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/admin/characters", get(list_characters).post(create_character))
        .route("/api/admin/characters/preview-recalc", post(preview_recalc))
        .route(
            "/api/admin/characters/:id",
            get(get_character).put(update_character).delete(delete_character),
        )
        .route("/api/admin/entities/by-location", get(entities_by_location))
}

// --- list ------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CharacterListQuery {
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub alive: Option<bool>,
}

async fn list_characters(
    State(s): State<AppState>,
    Query(q): Query<CharacterListQuery>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                let records = EditorEntityRepository::new(db)
                    .list(q.entity_type.as_deref(), q.alive)?;
                // Summary view: id/name/type/location/alive (no `data` blob).
                let list: Vec<Value> = records
                    .iter()
                    .map(|r| {
                        json!({
                            "id": r.id,
                            "name": r.name,
                            "entity_type": r.entity_type,
                            "location_q": r.location_q,
                            "location_r": r.location_r,
                            "alive": r.alive,
                        })
                    })
                    .collect();
                Ok(Value::Array(list))
            })
            .await,
    )
}

// --- get -------------------------------------------------------------------

async fn get_character(State(s): State<AppState>, Path(id): Path<String>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let record = EditorEntityRepository::new(db)
                    .load(&id)?
                    .ok_or_else(|| format!("entity not found: {id}"))?;
                serde_json::to_value(&record).map_err(|e| e.to_string())
            })
            .await,
    )
}

// --- create ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateCharacterBody {
    pub name: String,
    #[serde(default = "default_entity_type")]
    pub entity_type: String,
    #[serde(default)]
    pub location_q: i64,
    #[serde(default)]
    pub location_r: i64,
    #[serde(default)]
    pub data: Map<String, Value>,
}

fn default_entity_type() -> String {
    "character".to_string()
}

async fn create_character(State(s): State<AppState>, Json(body): Json<CreateCharacterBody>) -> Response {
    respond(
        s.session
            .read(move |db| {
                let id = uuid::Uuid::new_v4().to_string();
                EntityRepository::new(db).create_entity(
                    &id,
                    &body.entity_type,
                    &body.name,
                    body.location_q,
                    body.location_r,
                    &body.data,
                )?;
                Ok(json!({ "id": id, "name": body.name }))
            })
            .await,
    )
}

// --- update ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct UpdateCharacterBody {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub entity_type: Option<String>,
    #[serde(default)]
    pub location_q: Option<i64>,
    #[serde(default)]
    pub location_r: Option<i64>,
    #[serde(default)]
    pub alive: Option<bool>,
    #[serde(default)]
    pub data: Option<Map<String, Value>>,
}

async fn update_character(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCharacterBody>,
) -> Response {
    // 1. Resolve existence + whether the entity is a normalised character (which
    //    only persists the schema's known fields; a plain entity stores its whole
    //    `data` blob verbatim).
    let id_check = id.clone();
    let has_state = match s
        .session
        .read(move |db| {
            if db.fetch_one("SELECT id FROM entities WHERE id = ?", &[&id_check])?.is_none() {
                return Err(format!("entity not found: {id_check}"));
            }
            let has_state = db
                .fetch_one("SELECT entity_id FROM character_state WHERE entity_id = ?", &[&id_check])?
                .is_some();
            Ok(json!({ "has_state": has_state }))
        })
        .await
    {
        Ok(v) => v.get("has_state").and_then(Value::as_bool).unwrap_or(false),
        Err(e) => return respond(Err(e)),
    };

    // 2. Reject unrecognized `data` keys for normalised characters *before* any
    //    writes (they would otherwise be silently dropped), with a structured 422.
    if has_state {
        if let Some(data) = &body.data {
            let unknown = unrecognized_character_fields(data);
            if !unknown.is_empty() {
                return unrecognized_fields_response(unknown);
            }
        }
    }

    // 3. Apply the update.
    respond(
        s.session
            .read(move |db| {
                update_entity_columns(db, &id, &body)?;
                if let Some(data) = &body.data {
                    if has_state {
                        sync_character_state(db, &id, data, body.location_q, body.location_r)?;
                    } else {
                        let serialized = serde_json::to_string(data).map_err(|e| e.to_string())?;
                        db.execute("UPDATE entities SET data = ? WHERE id = ?", &[&serialized, &id])?;
                    }
                }
                Ok(json!({ "ok": true }))
            })
            .await,
    )
}

/// Recognized top-level keys in a character's editor `data` blob: the fields the
/// GET assembles and the PUT can persist. An incoming key not in this set is a
/// typo or an unimplemented term — reported, not silently dropped.
const KNOWN_CHARACTER_DATA_KEYS: &[&str] = &[
    "level", "xp", "xp_next", "hp", "max_hp", "ac", "attack_bonus",
    "physical_save", "evasion_save", "mental_save", "position_q", "position_r",
    "character_class", "attributes", "attr_mods", "skills", "equipment",
    "class_abilities", "personality",
];

/// One unrecognized field in an editor payload: where it was, a fragment of its
/// value, and the closest known field names (likely misspellings).
#[derive(Debug, Serialize)]
struct UnrecognizedField {
    /// Dotted path to the offending key, e.g. `data.xpp`.
    path: String,
    /// A short fragment of the offending value (truncated).
    fragment: String,
    /// Closest known field names, nearest first (a probable correct spelling).
    did_you_mean: Vec<String>,
}

/// Collect the `data` keys that aren't recognized character fields, each with a
/// value fragment and spelling suggestions.
fn unrecognized_character_fields(data: &Map<String, Value>) -> Vec<UnrecognizedField> {
    data.iter()
        .filter(|(k, _)| !KNOWN_CHARACTER_DATA_KEYS.contains(&k.as_str()))
        .map(|(k, v)| UnrecognizedField {
            path: format!("data.{k}"),
            fragment: value_fragment(v),
            did_you_mean: closest_known_keys(k),
        })
        .collect()
}

/// A short, single-line fragment of a JSON value for an error message.
fn value_fragment(v: &Value) -> String {
    const MAX: usize = 80;
    let s = v.to_string();
    if s.chars().count() > MAX {
        let head: String = s.chars().take(MAX).collect();
        format!("{head}…")
    } else {
        s
    }
}

/// Known keys closest to `key` by edit distance (≤ a small threshold), nearest
/// first, at most three — the "did you mean …" suggestions.
fn closest_known_keys(key: &str) -> Vec<String> {
    let mut scored: Vec<(usize, &str)> = KNOWN_CHARACTER_DATA_KEYS
        .iter()
        .map(|known| (levenshtein(key, known), *known))
        .collect();
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let threshold = (key.len() / 2).max(2);
    scored
        .into_iter()
        .filter(|(d, _)| *d <= threshold)
        .take(3)
        .map(|(_, k)| k.to_string())
        .collect()
}

/// Classic Levenshtein edit distance (two-row DP) over ASCII field names.
fn levenshtein(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, &ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

/// Build the structured 422 response for unrecognized editor fields.
fn unrecognized_fields_response(fields: Vec<UnrecognizedField>) -> Response {
    let body = json!({
        "error": "UnrecognizedFields",
        "message": format!(
            "{} unrecognized field(s) in character `data` — nothing was saved. \
             Fix the spelling, remove the field, or add it to the editor schema \
             (KNOWN_CHARACTER_DATA_KEYS).",
            fields.len()
        ),
        "unrecognized": fields,
        "known_fields": KNOWN_CHARACTER_DATA_KEYS,
    });
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}

/// Update the generic `entities` columns from the request.
fn update_entity_columns(db: &WorldDatabase, id: &str, body: &UpdateCharacterBody) -> Result<(), String> {
    let mut sets: Vec<&str> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    if let Some(name) = &body.name {
        sets.push("name = ?");
        params.push(Box::new(name.clone()));
    }
    if let Some(t) = &body.entity_type {
        sets.push("entity_type = ?");
        params.push(Box::new(t.clone()));
    }
    if let Some(q) = body.location_q {
        sets.push("location_q = ?");
        params.push(Box::new(q));
    }
    if let Some(r) = body.location_r {
        sets.push("location_r = ?");
        params.push(Box::new(r));
    }
    if let Some(alive) = body.alive {
        sets.push("alive = ?");
        params.push(Box::new(alive as i32));
    }
    super::apply_partial_update(db, "entities", &sets, params, "id", id)?;
    Ok(())
}

/// Write editor `data` fields back to the normalised `character_state` row.
fn sync_character_state(
    db: &WorldDatabase,
    id: &str,
    data: &Map<String, Value>,
    loc_q: Option<i64>,
    loc_r: Option<i64>,
) -> Result<(), String> {
    let mut sets: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    // Scalar int columns mirrored 1:1 from data keys.
    for key in [
        "level", "xp", "xp_next", "hp", "max_hp", "ac", "attack_bonus", "physical_save",
        "evasion_save", "mental_save",
    ] {
        if let Some(v) = data.get(key).and_then(Value::as_i64) {
            sets.push(format!("{key} = ?"));
            params.push(Box::new(v));
        }
    }
    if let Some(class) = data.get("character_class").and_then(Value::as_str) {
        sets.push("character_class = ?".into());
        params.push(Box::new(class.to_string()));
    }
    // JSON sub-objects serialised to their *_json columns.
    for (json_key, col) in [
        ("attributes", "attributes_json"),
        ("attr_mods", "attr_mods_json"),
        ("skills", "skills_json"),
        ("equipment", "equipment_json"),
        ("class_abilities", "class_abilities_json"),
    ] {
        if let Some(v) = data.get(json_key) {
            sets.push(format!("{col} = ?"));
            params.push(Box::new(serde_json::to_string(v).unwrap_or_default()));
        }
    }
    // Keep position in step with the entity location when it moves.
    if let Some(q) = loc_q {
        sets.push("position_q = ?".into());
        params.push(Box::new(q));
    }
    if let Some(r) = loc_r {
        sets.push("position_r = ?".into());
        params.push(Box::new(r));
    }
    super::apply_partial_update(db, "character_state", &sets, params, "entity_id", id)?;
    Ok(())
}

// --- delete ----------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    #[serde(default)]
    pub hard: bool,
}

async fn delete_character(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<DeleteQuery>,
) -> Response {
    respond(
        s.session
            .read(move |db| {
                if q.hard {
                    // Remove dependent rows first to satisfy FK references.
                    db.execute("DELETE FROM character_state WHERE entity_id = ?", &[&id])?;
                    db.execute("DELETE FROM npc_state WHERE entity_id = ?", &[&id])?;
                    db.execute("DELETE FROM entities WHERE id = ?", &[&id])?;
                } else {
                    db.execute("UPDATE entities SET alive = 0 WHERE id = ?", &[&id])?;
                }
                Ok(json!({ "ok": true }))
            })
            .await,
    )
}

// --- preview recalc --------------------------------------------------------

async fn preview_recalc(
    State(_s): State<AppState>,
    Json(req): Json<CharacterRecalcPreviewRequest>,
) -> Response {
    let result = tokio::task::spawn_blocking(move || {
        let attributes: BTreeMap<String, i32> = req.attributes.clone();
        let equipment: Vec<InventoryItemRecord> = req
            .equipment
            .iter()
            .filter_map(|obj| serde_json::from_value(Value::Object(obj.clone())).ok())
            .collect();
        recalculate_character(&req.character_class, req.level, &attributes, &equipment, None)
    })
    .await;
    match result {
        Ok(recalc) => Json(serde_json::to_value(recalc).unwrap_or_else(|_| json!({}))).into_response(),
        Err(_) => respond(Err("recalc task failed".into())),
    }
}

// --- entities by location --------------------------------------------------

async fn entities_by_location(State(s): State<AppState>) -> Response {
    respond(
        s.session
            .read(|db| {
                let rows = db.fetch_all(
                    "SELECT id, entity_type, name, location_q, location_r FROM entities \
                     WHERE alive = 1",
                    &[],
                )?;
                let mut grouped: Map<String, Value> = Map::new();
                for row in &rows {
                    let q = row.get("location_q").and_then(Value::as_i64);
                    let r = row.get("location_r").and_then(Value::as_i64);
                    let (Some(q), Some(r)) = (q, r) else { continue };
                    let key = format!("{q},{r}");
                    let entry = json!({
                        "id": row.get("id").and_then(Value::as_str).unwrap_or(""),
                        "entity_type": row.get("entity_type").and_then(Value::as_str).unwrap_or(""),
                        "name": row.get("name").and_then(Value::as_str).unwrap_or(""),
                        "q": q,
                        "r": r,
                    });
                    grouped
                        .entry(key)
                        .or_insert_with(|| Value::Array(Vec::new()))
                        .as_array_mut()
                        .expect("array")
                        .push(entry);
                }
                Ok(Value::Object(grouped))
            })
            .await,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_produce_no_unrecognized() {
        let data: Map<String, Value> = serde_json::from_value(json!({
            "hp": 9, "max_hp": 12, "attributes": {"str": 12}, "character_class": "warrior"
        }))
        .unwrap();
        assert!(unrecognized_character_fields(&data).is_empty());
    }

    #[test]
    fn unknown_keys_are_reported_with_path_and_suggestions() {
        let data: Map<String, Value> = serde_json::from_value(json!({
            "hp": 9, "xpp": 42, "attributs": {"str": 12}
        }))
        .unwrap();
        let fields = unrecognized_character_fields(&data);
        assert_eq!(fields.len(), 2);
        let by_path = |p: &str| fields.iter().find(|f| f.path == p).expect("field present");

        let xpp = by_path("data.xpp");
        assert_eq!(xpp.fragment, "42");
        assert!(xpp.did_you_mean.contains(&"xp".to_string()), "{:?}", xpp.did_you_mean);

        let attr = by_path("data.attributs");
        assert!(attr.did_you_mean.contains(&"attributes".to_string()), "{:?}", attr.did_you_mean);
    }

    #[test]
    fn wildly_different_key_gets_no_suggestions() {
        // Nothing within the edit-distance threshold.
        assert!(closest_known_keys("zzzzzzzzzz").is_empty());
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("xp", "xp"), 0);
        assert_eq!(levenshtein("xpp", "xp"), 1);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
    }

    #[test]
    fn value_fragment_truncates_long_values() {
        let long = "a".repeat(200);
        let frag = value_fragment(&Value::String(long));
        assert!(frag.chars().count() <= 81, "fragment should be capped");
        assert!(frag.ends_with("…"));
    }
}
