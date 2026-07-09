//! Pack discovery helpers.
//!
//! Ported from `src/harsh_realm/packs/discovery.py`.

use std::path::Path;

use super::exceptions::PackError;
use super::loader::load_manifest;
use super::manifest::PackManifest;

/// Return manifests of all packs in the packs root, sorted by id.
pub fn discover_packs(packs_root: &Path) -> Result<Vec<PackManifest>, PackError> {
    if !packs_root.exists() || !packs_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut candidates: Vec<_> = std::fs::read_dir(packs_root)
        .map_err(|e| PackError::Load(e.to_string()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    candidates.sort();

    let mut manifests: Vec<PackManifest> = Vec::new();
    for candidate in candidates {
        if candidate.is_dir() && candidate.join("pack.yaml").exists() {
            manifests.push(load_manifest(&candidate)?);
        }
    }
    manifests.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(manifests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("hr-pack-discovery-{name}"));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn discovers_and_sorts_packs() {
        let root = temp_dir("basic");
        for id in ["zeta", "alpha"] {
            let pack_dir = root.join(id);
            std::fs::create_dir_all(&pack_dir).unwrap();
            std::fs::write(
                pack_dir.join("pack.yaml"),
                format!("id: {id}\nversion: 1.0.0\nname: {id}\n"),
            )
            .unwrap();
        }
        let manifests = discover_packs(&root).unwrap();
        let ids: Vec<&str> = manifests.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "zeta"]);
    }

    #[test]
    fn missing_root_is_empty() {
        let missing = std::env::temp_dir().join("hr-pack-discovery-nope-xyz");
        let _ = std::fs::remove_dir_all(&missing);
        assert!(discover_packs(&missing).unwrap().is_empty());
    }
}
