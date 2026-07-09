//! Filesystem loader for directory-based rules packs.
//!
//! Ported from `src/harsh_realm/packs/loader.py`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};

use super::exceptions::PackError;
use super::manifest::PackManifest;

/// Records keyed by category then slug.
pub type PackRecords = BTreeMap<String, BTreeMap<String, JsonObject>>;

/// A loaded pack: manifest metadata plus content access.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    /// Parsed manifest.
    pub manifest: PackManifest,
    /// Resolved pack root directory.
    pub root_path: PathBuf,
    /// Loaded content records.
    #[serde(default)]
    pub records: PackRecords,
}

impl Pack {
    /// Return a content record by category and slug.
    pub fn get_record(&self, category: &str, slug: &str) -> Option<JsonObject> {
        self.records.get(category)?.get(slug).cloned()
    }

    /// Return all records in a category sorted by slug.
    pub fn list_records(&self, category: &str) -> Vec<(String, JsonObject)> {
        match self.records.get(category) {
            Some(map) => map
                .iter()
                .map(|(slug, record)| (slug.clone(), record.clone()))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Return all content categories provided by this pack.
    pub fn list_categories(&self) -> Vec<String> {
        self.records.keys().cloned().collect()
    }
}

/// Load a directory-based pack from disk.
pub fn load_pack(path: &Path) -> Result<Pack, PackError> {
    let root_path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    let manifest = load_manifest(&root_path)?;
    let records = load_content(&root_path, &manifest.id)?;
    Ok(Pack {
        manifest,
        root_path,
        records,
    })
}

/// Load and validate a pack manifest from a pack root directory.
pub fn load_manifest(root_path: &Path) -> Result<PackManifest, PackError> {
    let manifest_path = root_path.join("pack.yaml");
    if !manifest_path.exists() {
        return Err(PackError::Load(format!(
            "Pack manifest not found: {}",
            manifest_path.display()
        )));
    }
    let document = load_yaml_file(&manifest_path)?;
    let JsonValue::Object(map) = document else {
        return Err(PackError::Load(format!(
            "Pack manifest must be a mapping: {}",
            manifest_path.display()
        )));
    };
    PackManifest::from_document(map).map_err(|e| {
        PackError::Load(format!(
            "Invalid pack manifest {}: {e}",
            manifest_path.display()
        ))
    })
}

fn load_content(root_path: &Path, pack_id: &str) -> Result<PackRecords, PackError> {
    let content_path = root_path.join("content");
    let mut records: PackRecords = BTreeMap::new();
    if !content_path.exists() {
        return Ok(records);
    }
    if !content_path.is_dir() {
        return Err(PackError::Load(format!(
            "Pack content path must be a directory: {}",
            content_path.display()
        )));
    }
    let mut yaml_paths: Vec<PathBuf> = Vec::new();
    collect_yaml_files(&content_path, &mut yaml_paths)?;
    yaml_paths.sort();
    for yaml_path in &yaml_paths {
        load_content_file(&content_path, yaml_path, pack_id, &mut records)?;
    }
    Ok(records)
}

fn collect_yaml_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), PackError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| PackError::Load(format!("Could not read pack dir {}: {e}", dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| PackError::Load(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            out.push(path);
        }
    }
    Ok(())
}

fn load_content_file(
    content_root: &Path,
    yaml_path: &Path,
    pack_id: &str,
    records: &mut PackRecords,
) -> Result<(), PackError> {
    let document = load_yaml_file(yaml_path)?;
    let category = category_for_path(content_root, yaml_path);
    let stem = yaml_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();

    // Mapping with a "records" list.
    if let JsonValue::Object(ref map) = document {
        if let Some(JsonValue::Array(raw_records)) = map.get("records") {
            for (index, raw_record) in raw_records.iter().enumerate() {
                let record = as_record(raw_record, yaml_path)?;
                let slug = slug_from_record(&record, &format!("{stem}-{}", index + 1));
                add_record(records, pack_id, &category, &slug, record, yaml_path)?;
            }
            return Ok(());
        }
    }

    // Top-level list.
    if let JsonValue::Array(raw_records) = &document {
        for (index, raw_record) in raw_records.iter().enumerate() {
            let record = as_record(raw_record, yaml_path)?;
            let slug = slug_from_record(&record, &format!("{stem}-{}", index + 1));
            add_record(records, pack_id, &category, &slug, record, yaml_path)?;
        }
        return Ok(());
    }

    // Single record.
    let record = as_record(&document, yaml_path)?;
    add_record(records, pack_id, &category, &stem, record, yaml_path)
}

fn load_yaml_file(path: &Path) -> Result<JsonValue, PackError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| PackError::Load(format!("Could not read pack file {}: {e}", path.display())))?;
    let document: JsonValue = serde_yaml::from_str(&text)
        .map_err(|e| PackError::Load(format!("Invalid YAML in pack file {}: {e}", path.display())))?;
    if document.is_null() {
        return Ok(JsonValue::Object(JsonObject::new()));
    }
    Ok(document)
}

fn category_for_path(content_root: &Path, yaml_path: &Path) -> String {
    let relative = yaml_path.strip_prefix(content_root).unwrap_or(yaml_path);
    match relative.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join("."),
        _ => yaml_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string(),
    }
}

fn as_record(raw_record: &JsonValue, path: &Path) -> Result<JsonObject, PackError> {
    match raw_record {
        JsonValue::Object(map) => Ok(map.clone()),
        _ => Err(PackError::Load(format!(
            "Pack content record must be a mapping: {}",
            path.display()
        ))),
    }
}

fn slug_from_record(record: &JsonObject, fallback: &str) -> String {
    if let Some(JsonValue::String(slug)) = record.get("slug") {
        if !slug.is_empty() {
            return slug.clone();
        }
    }
    if let Some(JsonValue::String(record_id)) = record.get("id") {
        if !record_id.is_empty() {
            let after_colon = record_id.rsplit(':').next().unwrap_or(record_id);
            return after_colon.rsplit('.').next().unwrap_or(after_colon).to_string();
        }
    }
    fallback.to_string()
}

fn add_record(
    records: &mut PackRecords,
    pack_id: &str,
    category: &str,
    slug: &str,
    record: JsonObject,
    source_path: &Path,
) -> Result<(), PackError> {
    let category_records = records.entry(category.to_string()).or_default();
    if category_records.contains_key(slug) {
        return Err(PackError::Load(format!(
            "Duplicate record {category}.{slug} in pack {pack_id}: {}",
            source_path.display()
        )));
    }
    let mut enriched = record;
    enriched.insert("_pack_id".into(), JsonValue::String(pack_id.to_string()));
    enriched.insert("_category".into(), JsonValue::String(category.to_string()));
    enriched.insert("_slug".into(), JsonValue::String(slug.to_string()));
    enriched.insert(
        "_qualified_id".into(),
        JsonValue::String(format!("{pack_id}:{category}.{slug}")),
    );
    category_records.insert(slug.to_string(), enriched);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        std::fs::write(path, content).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("hr-pack-test-{name}"));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn loads_manifest_and_categorised_records() {
        let root = temp_dir("loader-basic");
        write(
            &root.join("pack.yaml"),
            "id: xwn-core\nversion: 1.0.0\nname: XWN Core\n",
        );
        let content = root.join("content");
        std::fs::create_dir_all(content.join("tables")).unwrap();
        write(
            &content.join("skills.yaml"),
            "- id: stab\n  name: Stab\n- id: shoot\n  name: Shoot\n",
        );
        write(
            &content.join("tables").join("loot.yaml"),
            "id: loot\nname: Loot Table\n",
        );

        let pack = load_pack(&root).unwrap();
        assert_eq!(pack.manifest.id, "xwn-core");
        // Top-level dir file -> category == file stem.
        let skills = pack.list_records("skills");
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].0, "shoot"); // sorted by slug
        assert_eq!(
            skills[0].1.get("_qualified_id").unwrap().as_str().unwrap(),
            "xwn-core:skills.shoot"
        );
        // Nested dir -> category == dir parts joined by '.'.
        let loot = pack.get_record("tables", "loot").unwrap();
        assert_eq!(loot.get("_category").unwrap().as_str().unwrap(), "tables");
    }

    #[test]
    fn duplicate_slug_is_an_error() {
        let root = temp_dir("loader-dup");
        write(
            &root.join("pack.yaml"),
            "id: dup\nversion: 1.0.0\nname: Dup\n",
        );
        let content = root.join("content");
        std::fs::create_dir_all(&content).unwrap();
        write(
            &content.join("things.yaml"),
            "- id: a\n- id: a\n",
        );
        assert!(load_pack(&root).is_err());
    }

    #[test]
    fn missing_manifest_is_an_error() {
        let root = temp_dir("loader-nomanifest");
        assert!(load_pack(&root).is_err());
    }
}
