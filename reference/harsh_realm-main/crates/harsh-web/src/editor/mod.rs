//! Editor/world-state CRUD endpoints (`/api/admin/*` editor surface + `/api/gm/*`).
//!
//! These are thin Axum wrappers over `harsh-core`'s editor repositories
//! (`EditorCellRepository`, `FactionRepository`, `DungeonRepository`,
//! `OracleRepository`, `EntityRepository`) plus a little raw-SQL assembly, all
//! scoped to the **active world**. Like [`crate::admin`], they ignore the
//! optional `?world=` query parameter: the session actor owns a single loaded
//! world, and the frontend already drives `?world=` to that same world. Editing
//! an unloaded world would require a second live connection and is out of scope
//! for the single-actor model.
//!
//! All grid coordinates are square-grid `q`/`r` (no hex). Mutations run inside
//! the session actor's serialized closure, so concurrent writes are safe.

use std::path::{Path, PathBuf};

use axum::response::Response;
use rusqlite::ToSql;
use serde_json::Value;

use harsh_core::db::WorldDatabase;

use crate::state::AppState;

pub mod archive;
pub mod cells;
pub mod characters;
pub mod dungeons;
pub mod factions;
pub mod files;
pub mod gm;
pub mod oracle;
pub mod transfer;

/// Convert a `Result<Value, String>` into a JSON response, mapping errors to a
/// 409 with `{ error, message }` (matching the config-admin convention).
pub(crate) fn respond(result: Result<Value, String>) -> Response {
    crate::response::respond_with(result, "EditorError")
}

/// Recursively collect every `*.yaml`/`*.yml` file under `root`, returning
/// `(absolute_path, relative_path)` pairs with forward-slash relative paths,
/// sorted by relative path. The shared content-tree walker for the editor's
/// file listing, table-status, and archive endpoints.
pub(crate) fn walk_yaml_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "yaml" && ext != "yml" {
                continue;
            }
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((path, rel));
        }
    }
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// Apply a partial `UPDATE <table> SET <sets> WHERE <key_col> = ?`.
///
/// `sets` are the `"col = ?"` fragments and `params` their bound values in the
/// same order; `key_val` is appended as the final parameter. No-op (returns
/// `false`) when `sets` is empty, so callers can build the SET list conditionally.
/// Shared by the editor's character/entity/oracle write paths.
pub(crate) fn apply_partial_update<S: AsRef<str>>(
    db: &WorldDatabase,
    table: &str,
    sets: &[S],
    params: Vec<Box<dyn ToSql>>,
    key_col: &str,
    key_val: &str,
) -> Result<bool, String> {
    if sets.is_empty() {
        return Ok(false);
    }
    let joined = sets.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(", ");
    let sql = format!("UPDATE {table} SET {joined} WHERE {key_col} = ?");
    let mut refs: Vec<&dyn ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let key_ref: &dyn ToSql = &key_val;
    refs.push(key_ref);
    db.execute(&sql, &refs)?;
    Ok(true)
}

/// Merge all editor + GM routes into the main router.
pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .merge(cells::routes())
        .merge(characters::routes())
        .merge(factions::routes())
        .merge(dungeons::routes())
        .merge(oracle::routes())
        .merge(gm::routes())
        .merge(transfer::routes())
        .merge(files::routes())
        .merge(archive::routes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use harsh_core::db_schema::SCHEMA_SQL;
    use harsh_core::repositories::entity::EntityRepository;
    use harsh_core::runtime::JsonObject;

    fn seeded_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    #[test]
    fn partial_update_applies_sets_and_keys_on_the_id() {
        let db = seeded_db();
        EntityRepository::new(&db)
            .create_entity("e1", "npc", "Old", 0, 0, &JsonObject::new())
            .unwrap();

        let params: Vec<Box<dyn ToSql>> = vec![Box::new("New".to_string())];
        let ran = apply_partial_update(&db, "entities", &["name = ?"], params, "id", "e1").unwrap();
        assert!(ran);
        let row = db
            .fetch_one("SELECT name FROM entities WHERE id = ?", &[&"e1"])
            .unwrap()
            .unwrap();
        assert_eq!(row.get("name").and_then(Value::as_str), Some("New"));
    }

    #[test]
    fn partial_update_is_a_noop_for_empty_sets() {
        let db = seeded_db();
        let empty: Vec<Box<dyn ToSql>> = vec![];
        let ran =
            apply_partial_update(&db, "entities", &[] as &[&str], empty, "id", "e1").unwrap();
        assert!(!ran, "no SET fragments → no update");
    }

    #[test]
    fn walk_yaml_files_recurses_filters_and_sorts() {
        let dir = std::env::temp_dir().join(format!("hr_walk_{}", std::process::id()));
        let tables = dir.join("tables");
        std::fs::create_dir_all(&tables).unwrap();
        std::fs::write(dir.join("a.yaml"), "x: 1").unwrap();
        std::fs::write(tables.join("b.yml"), "y: 2").unwrap();
        std::fs::write(dir.join("ignore.txt"), "nope").unwrap();

        let found = walk_yaml_files(&dir);
        let rels: Vec<&str> = found.iter().map(|(_, r)| r.as_str()).collect();
        // Recurses into subdirs, excludes non-yaml, forward-slash + sorted rels.
        assert_eq!(rels, vec!["a.yaml", "tables/b.yml"]);
        // The absolute path actually exists and reads.
        assert!(found[0].0.exists());

        std::fs::remove_dir_all(&dir).ok();
    }
}
