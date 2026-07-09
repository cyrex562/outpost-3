//! World + character service helpers — pure functions over a `&WorldDatabase`.
//!
//! These run on the session-actor thread (which owns the sync `WorldDatabase`).
//! They mirror the flows the deleted Python routes performed (world creation,
//! character placement, map projection), reusing `harsh-core` engine functions.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::thread_rng;
use serde_json::{json, Value};

use harsh_core::admin::service::AdminService;
use harsh_core::cell::TerrainRegistry;
use harsh_core::character_build::{
    build_character, class_hit_die, load_kits, load_skill_definitions, load_valid_skills,
    validate_submission, CURRENT_MILESTONE,
};
use harsh_core::character_creation::CharacterDraftSubmission;
use harsh_core::db::WorldDatabase;
use harsh_core::dice::DiceRoller;
use harsh_core::events::EventBus;
use harsh_core::generators::world_gen::WorldGenerator;
use harsh_core::gm::{GMController, Narrator};
use harsh_core::grid::SquareGrid;
use harsh_core::content::IRRecordRepository;
use harsh_core::packs::discover_packs;
use harsh_core::public_api::CreateWorldRequest;
use harsh_core::repositories::cell::CellRepository;
use harsh_core::repositories::entity::EntityRepository;
use harsh_core::repositories::world_pack::{PackBinding, WorldPackRepository};
use harsh_core::runtime_content::compile_pack_ir;
use harsh_core::status_effects::StatusEffectRepository;

/// Paths the actor needs to assemble engine objects.
#[derive(Debug, Clone)]
pub struct ActorPaths {
    /// Directory holding world `.db` files.
    pub worlds_dir: PathBuf,
    /// Built-in content directory (terrain, narrator templates, world-gen data).
    pub content_dir: PathBuf,
    /// Packs root (parent of `<pack-id>/` dirs) for resolving bound-pack content.
    pub packs_root: PathBuf,
}

/// Load a populated terrain registry from the content directory.
fn load_registry(paths: &ActorPaths) -> Result<TerrainRegistry, String> {
    let mut registry = TerrainRegistry::new();
    registry.load_all(&paths.content_dir)?;
    Ok(registry)
}

/// Build a ready-to-use GM controller bound to `db` (registry + narrator loaded,
/// state restored from `gm_state`).
pub fn build_controller<'db>(
    db: &'db WorldDatabase,
    paths: &ActorPaths,
) -> Result<GMController<'db>, String> {
    let registry = load_registry(paths)?;
    let narrator = Narrator::new(registry.clone(), &paths.content_dir);
    let mut controller = GMController::new(db, EventBus::default(), registry, narrator);
    controller.initialize();
    Ok(controller)
}

/// Slugify a world name into a filesystem-safe stem.
fn slugify(name: &str) -> String {
    let slug: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "world".to_string()
    } else {
        slug
    }
}

/// Monotonic-ish unique suffix for world filenames.
fn unique_suffix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{millis}")
}

/// Create + generate a new world, returning the open db and a `{file, name}` body.
pub fn create_world(
    paths: &ActorPaths,
    req: &CreateWorldRequest,
) -> Result<(WorldDatabase, Value), String> {
    req.validate()?;
    std::fs::create_dir_all(&paths.worlds_dir).map_err(|e| e.to_string())?;
    let file = format!("{}-{}.db", slugify(&req.name), unique_suffix());
    let path = paths.worlds_dir.join(&file);

    let db = WorldDatabase::create(&path, &req.name, None)?;
    // Seed editable config tables (skill mappings, difficulty targets, etc.).
    AdminService::new(&db).seed_all_from_yaml()?;
    // Bind the chosen packs and compile their IR content so authored IR records
    // drive this world from creation.
    bind_and_compile_packs(&db, paths, req);
    // Generate and persist the grid region.
    let registry = load_registry(paths)?;
    let generator = WorldGenerator::new(&db, registry, Some(paths.content_dir.clone()));
    let summary = generator.generate_region(req.width, req.height, req.seed.map(|s| s as u64))?;
    db.set_meta("width", &summary.width.to_string())?;
    db.set_meta("height", &summary.height.to_string())?;

    Ok((db, json!({ "file": file, "name": req.name })))
}

/// The default pack bound to a world when the request names none.
const DEFAULT_PACK: &str = "xwn-core";

/// Persist the world's bound pack list and compile those packs' IR content into
/// the world's `ir_records`. Best-effort: pack issues are logged, never fatal to
/// world creation (a world with no IR content simply runs without IR triggers).
fn bind_and_compile_packs(db: &WorldDatabase, paths: &ActorPaths, req: &CreateWorldRequest) {
    let pack_ids: Vec<String> = req
        .pack_ids
        .clone()
        .filter(|ids| !ids.is_empty())
        .unwrap_or_else(|| vec![DEFAULT_PACK.to_string()]);

    // Resolve versions from discovery (best-effort; default when unknown).
    let versions: std::collections::HashMap<String, String> =
        discover_packs(&paths.packs_root)
            .unwrap_or_default()
            .into_iter()
            .map(|m| (m.id, m.version))
            .collect();
    let bindings: Vec<PackBinding> = pack_ids
        .iter()
        .enumerate()
        .map(|(i, id)| PackBinding {
            pack_id: id.clone(),
            version: versions.get(id).cloned().unwrap_or_else(|| "0.0.0".to_string()),
            load_order: i as i32,
        })
        .collect();
    if let Err(e) = WorldPackRepository::new(db).set_pack_list(&bindings) {
        eprintln!("create_world: failed to bind packs: {e}");
    }

    let (records, diagnostics) = compile_pack_ir(&paths.packs_root, &pack_ids);
    for d in &diagnostics {
        eprintln!("create_world: {d}");
    }
    if !records.is_empty() {
        let objects: Vec<harsh_core::runtime::JsonObject> = records
            .iter()
            .filter_map(|r| serde_json::to_value(r).ok().and_then(|v| v.as_object().cloned()))
            .collect();
        if let Err(e) = IRRecordRepository::new(db).upsert_many(&objects, "pack") {
            eprintln!("create_world: failed to store pack IR records: {e}");
        }
    }
}

/// Open an existing world, returning the open db and a `{file, name}` body.
pub fn open_world(paths: &ActorPaths, file: &str) -> Result<(WorldDatabase, Value), String> {
    // Reject path traversal — `file` is a bare filename in the worlds dir.
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err(format!("Invalid world file: {file}"));
    }
    let path = paths.worlds_dir.join(file);
    let db = WorldDatabase::open(&path)?;
    let name = db.get_meta("name")?.unwrap_or_else(|| file.to_string());
    Ok((db, json!({ "file": file, "name": name })))
}

/// `{file, name}` describing the currently open world.
pub fn current_world_json(db: &WorldDatabase) -> Value {
    let name = db.get_meta("name").ok().flatten().unwrap_or_default();
    json!({ "file": world_filename(db), "name": name })
}

/// Filename (with extension) of the world a database is backed by.
pub fn world_filename(db: &WorldDatabase) -> String {
    db.filepath()
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Delete a world file that is not currently open. Returns `{ ok: true }`.
pub fn delete_world(paths: &ActorPaths, file: &str) -> Result<Value, String> {
    if file.contains('/') || file.contains('\\') || file.contains("..") {
        return Err(format!("Invalid world file: {file}"));
    }
    let path = paths.worlds_dir.join(file);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(json!({ "ok": true }))
}

/// Build, place, and persist a new character from a form submission. Mirrors the
/// chat scene's `persist_character`: roll HP, find the starting hex, reveal it.
pub fn create_and_place_character(
    db: &WorldDatabase,
    submission: &CharacterDraftSubmission,
) -> Result<Value, String> {
    let valid_skills = load_valid_skills();
    let kits = load_kits();
    let skill_defs = load_skill_definitions();
    validate_submission(submission, &valid_skills, &kits, Some(&skill_defs), CURRENT_MILESTONE)?;

    let hit_die = class_hit_die(&submission.character_class);
    let hp_roll = DiceRoller::new(thread_rng())
        .roll(1, hit_die as u32, 0)
        .map_err(|e| e.to_string())?
        .total;
    let mut character = build_character(submission, &valid_skills, &kits, hp_roll)?;

    let cell_repo = CellRepository::new(db);
    let (start_q, start_r) = cell_repo.find_starting_hex()?;
    character.position_q = start_q as i32;
    character.position_r = start_r as i32;
    cell_repo.mark_explored(start_q, start_r, 1)?;
    cell_repo.reveal_neighbors(start_q, start_r, &SquareGrid::default())?;

    EntityRepository::new(db).create_character(&character)?;
    Ok(json!({ "id": character.id, "name": character.name }))
}

/// Load the current character merged with its live location columns, or `null`.
pub fn character_json(db: &WorldDatabase) -> Result<Value, String> {
    let (character, record) = EntityRepository::new(db).load_character_with_record()?;
    let Some(character) = character else {
        return Ok(Value::Null);
    };
    let mut obj = match serde_json::to_value(&character).map_err(|e| e.to_string())? {
        Value::Object(map) => map,
        _ => return Err("character did not serialize to an object".into()),
    };
    if let Some(record) = record {
        obj.insert("location_q".into(), json!(record.location_q));
        obj.insert("location_r".into(), json!(record.location_r));
    }
    Ok(Value::Object(obj))
}

/// Active status effects for an entity, as a JSON array.
pub fn status_effects_json(db: &WorldDatabase, entity_id: &str) -> Result<Value, String> {
    let effects = StatusEffectRepository::new(db).list_for_entity(entity_id)?;
    serde_json::to_value(effects).map_err(|e| e.to_string())
}

/// Active + completed quest instances for an entity (HR-19).
pub fn quests_json(db: &WorldDatabase, entity_id: &str) -> Result<Value, String> {
    let repo = harsh_core::quest::QuestRepository::new(db);
    let active = repo.list_active(entity_id)?;
    let completed = repo.list_completed(entity_id)?;
    Ok(serde_json::json!({ "active": active, "completed": completed }))
}

/// Project all world cells into the frontend map response shape.
pub fn map_json(db: &WorldDatabase) -> Result<Value, String> {
    let meta_width: Option<i64> = db.get_meta("width")?.and_then(|s| s.parse().ok());
    let meta_height: Option<i64> = db.get_meta("height")?.and_then(|s| s.parse().ok());

    let rows = db.fetch_all(
        "SELECT q, r, terrain, features, explored, description, faction_id, data FROM cells",
        &[],
    )?;
    let mut cells = Vec::with_capacity(rows.len());
    let (mut max_q, mut max_r) = (0i64, 0i64);
    for row in &rows {
        let q = row.get("q").and_then(Value::as_i64).unwrap_or(0);
        let r = row.get("r").and_then(Value::as_i64).unwrap_or(0);
        max_q = max_q.max(q);
        max_r = max_r.max(r);
        let features = parse_json_column(row.get("features"), json!([]));
        let data = parse_json_column(row.get("data"), json!({}));
        let explored = row.get("explored").and_then(Value::as_i64).unwrap_or(0);
        cells.push(json!({
            "q": q,
            "r": r,
            "terrain": row.get("terrain").and_then(Value::as_str).unwrap_or(""),
            "features": features,
            "explored": explored,
            "description": row.get("description").cloned().unwrap_or(Value::Null),
            "faction_id": row.get("faction_id").cloned().unwrap_or(Value::Null),
            "data": data,
        }));
    }

    let width = meta_width.unwrap_or(max_q + 1);
    let height = meta_height.unwrap_or(max_r + 1);
    Ok(json!({
        "width": width,
        "height": height,
        "grid_type": "square",
        "cells": cells,
    }))
}

/// Parse a TEXT column holding JSON, falling back to `default` on null/parse error.
fn parse_json_column(value: Option<&Value>, default: Value) -> Value {
    value
        .and_then(Value::as_str)
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(default)
}
