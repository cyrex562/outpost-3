//! Pack migration discovery and metadata.
//!
//! Ported from `src/harsh_realm/packs/migrations.py`.
//!
//! This module handles the pure parts: discovering migration files on disk and
//! computing the pending migration chain. Execution of SQL schema migrations
//! and Python data migrations is B9 (web host async layer).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::packs::loader::Pack;
use crate::packs::version::parse_version;

/// Pattern: v{from}_to_v{to}  (e.g. v0.1.0_to_v0.2.0)
fn parse_migration_stem(stem: &str) -> Option<(String, String)> {
    // Strip leading "v"
    let stem = stem.strip_prefix('v').unwrap_or(stem);
    let marker = "_to_v";
    let pos = stem.find(marker)?;
    let from = stem[..pos].to_string();
    let to = stem[pos + marker.len()..].to_string();
    // Validate both look like semver
    parse_version(&from).ok()?;
    parse_version(&to).ok()?;
    Some((from, to))
}

/// A discovered pack migration file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Migration {
    pub pack_id: String,
    pub from_version: String,
    pub to_version: String,
    /// `"schema"` (SQL) or `"data"` (Python callable — execution is B9).
    pub kind: String,
    pub path: PathBuf,
}

impl Migration {
    fn sort_key(&self) -> ((u32, u32, u32), u8) {
        let ver = parse_version(&self.to_version).unwrap_or((0, 0, 0));
        let kind_ord: u8 = if self.kind == "schema" { 0 } else { 1 };
        (ver, kind_ord)
    }
}

/// Discover all migration files in a pack's `migrations/` directory.
pub fn discover_migrations(pack: &Pack) -> Vec<Migration> {
    let root = pack.root_path.join("migrations");
    let mut all: Vec<Migration> = Vec::new();
    for kind in &["schema", "data"] {
        let kind_root = root.join(kind);
        if !kind_root.exists() {
            continue;
        }
        let expected_suffix = if *kind == "schema" { ".sql" } else { ".py" };
        let mut entries: Vec<PathBuf> = std::fs::read_dir(&kind_root)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        entries.sort();
        for path in entries {
            let suffix = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if format!(".{suffix}") != expected_suffix {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if let Some((from, to)) = parse_migration_stem(stem) {
                all.push(Migration {
                    pack_id: pack.manifest.id.clone(),
                    from_version: from,
                    to_version: to,
                    kind: kind.to_string(),
                    path: path.clone(),
                });
            }
        }
    }
    all.sort_by_key(|m| m.sort_key());
    all
}

/// List migrations needed to bring a world from `current_version` to the
/// installed pack version.
pub fn get_pending_migrations(pack: &Pack, current_version: &str) -> Vec<Migration> {
    let cur = match parse_version(current_version) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let target = match parse_version(&pack.manifest.version) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    if cur >= target {
        return vec![];
    }

    let migrations = discover_migrations(pack);
    let mut pending: Vec<Migration> = Vec::new();
    let mut cursor = current_version.to_string();

    loop {
        let cursor_ver = match parse_version(&cursor) {
            Ok(v) => v,
            Err(_) => break,
        };
        if cursor_ver >= target {
            break;
        }
        // Find all migrations from cursor that don't overshoot the target.
        let next_steps: Vec<&Migration> = migrations
            .iter()
            .filter(|m| {
                m.from_version == cursor
                    && parse_version(&m.to_version).map(|v| v <= target).unwrap_or(false)
            })
            .collect();
        if next_steps.is_empty() {
            break;
        }
        // Advance to the smallest next version.
        let next_ver = next_steps
            .iter()
            .map(|m| m.to_version.clone())
            .min_by_key(|v| parse_version(v).unwrap_or((u32::MAX, u32::MAX, u32::MAX)))
            .unwrap();
        let step: Vec<Migration> = next_steps
            .into_iter()
            .filter(|m| m.to_version == next_ver)
            .cloned()
            .collect();
        pending.extend(step);
        cursor = next_ver;
    }
    pending
}

/// Read a schema migration file as a SQL script string.
pub fn read_schema_migration(migration: &Migration) -> Result<String, String> {
    if migration.kind != "schema" {
        return Err(format!(
            "Migration {} is not a schema migration",
            migration.path.display()
        ));
    }
    std::fs::read_to_string(&migration.path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::packs::loader::Pack;

    fn make_pack(tmp: &std::path::Path, version: &str) -> Pack {
        use crate::packs::manifest::PackManifest;
        Pack {
            root_path: tmp.to_path_buf(),
            manifest: PackManifest {
                id: "test-pack".into(),
                name: "Test".into(),
                version: version.into(),
                description: String::new(),
                authors: vec![],
                depends: vec![],
                conflicts: vec![],
                provides: vec![],
            },
            records: Default::default(),
        }
    }

    fn with_tmp<F: FnOnce(&std::path::Path)>(f: F) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let tmp = std::env::temp_dir()
            .join(format!("harsh_test_{}_{}", std::process::id(), id));
        std::fs::create_dir_all(&tmp).unwrap();
        f(&tmp);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn write_migration(tmp: &std::path::Path, kind: &str, stem: &str) {
        let dir = tmp.join("migrations").join(kind);
        fs::create_dir_all(&dir).unwrap();
        let ext = if kind == "schema" { "sql" } else { "py" };
        fs::write(dir.join(format!("{stem}.{ext}")), b"-- migration").unwrap();
    }

    #[test]
    fn parse_migration_stem_valid() {
        let r = parse_migration_stem("v0.1.0_to_v0.2.0");
        assert_eq!(r, Some(("0.1.0".into(), "0.2.0".into())));
    }

    #[test]
    fn parse_migration_stem_invalid() {
        assert!(parse_migration_stem("not_a_migration").is_none());
    }

    #[test]
    fn discover_migrations_empty_pack() {
        with_tmp(|tmp| {
            let pack = make_pack(tmp, "1.0.0");
            assert!(discover_migrations(&pack).is_empty());
        });
    }

    #[test]
    fn discover_migrations_finds_files() {
        with_tmp(|tmp| {
            write_migration(tmp, "schema", "v0.1.0_to_v0.2.0");
            write_migration(tmp, "data", "v0.2.0_to_v0.3.0");
            let pack = make_pack(tmp, "0.3.0");
            let m = discover_migrations(&pack);
            assert_eq!(m.len(), 2);
            assert_eq!(m[0].kind, "schema");
            assert_eq!(m[1].kind, "data");
        });
    }

    #[test]
    fn get_pending_already_at_version() {
        with_tmp(|tmp| {
            write_migration(tmp, "schema", "v0.1.0_to_v0.2.0");
            let pack = make_pack(tmp, "0.2.0");
            assert!(get_pending_migrations(&pack, "0.2.0").is_empty());
        });
    }

    #[test]
    fn get_pending_returns_chain() {
        with_tmp(|tmp| {
            write_migration(tmp, "schema", "v0.1.0_to_v0.2.0");
            write_migration(tmp, "schema", "v0.2.0_to_v0.3.0");
            let pack = make_pack(tmp, "0.3.0");
            let pending = get_pending_migrations(&pack, "0.1.0");
            assert_eq!(pending.len(), 2);
            assert_eq!(pending[0].from_version, "0.1.0");
            assert_eq!(pending[1].from_version, "0.2.0");
        });
    }
}
