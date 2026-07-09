//! YAML document loading for content authoring.
//!
//! Ported from `src/harsh_realm/content/loader.py`.
//!
//! `serde_yaml` (0.9) uses YAML 1.2 and does not booleanise `on/off/yes/no`,
//! so no custom resolver stripping is required on the Rust side.

use std::path::Path;

use serde_json::Value;

/// Parse a single YAML document into a JSON value.
pub fn load_yaml(text: &str) -> Result<Value, String> {
    serde_yaml::from_str(text).map_err(|e| e.to_string())
}

/// Load every YAML document under `root` recursively (multi-doc supported).
///
/// Files are visited in sorted path order so derived IDs and diagnostics are
/// deterministic. Returns all non-null documents as JSON values.
pub fn load_documents(root: &Path) -> Result<Vec<Value>, String> {
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    // Collect recursively.
    let mut all_paths: Vec<std::path::PathBuf> = Vec::new();
    collect_yaml_paths(root, &mut all_paths).map_err(|e| e.to_string())?;
    drop(paths);
    all_paths.sort();

    let mut documents: Vec<Value> = Vec::new();
    for path in &all_paths {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        // serde_yaml::Deserializer supports multi-doc streams.
        for doc in serde_yaml::Deserializer::from_str(&text) {
            let v: Value = serde::de::Deserialize::deserialize(doc).map_err(|e| e.to_string())?;
            if !v.is_null() {
                documents.push(v);
            }
        }
    }
    Ok(documents)
}

fn collect_yaml_paths(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_yaml_paths(&path, out)?;
        } else if let Some(ext) = path.extension() {
            if ext == "yaml" || ext == "yml" {
                out.push(path);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_yaml_simple() {
        let v = load_yaml("id: foo\nname: bar").unwrap();
        assert_eq!(v["id"], "foo");
        assert_eq!(v["name"], "bar");
    }

    #[test]
    fn load_yaml_on_key_is_string() {
        // serde_yaml 0.9 (YAML 1.2) must NOT parse `on` as boolean true.
        let v = load_yaml("on: push").unwrap();
        assert_eq!(v["on"], "push");
    }

    #[test]
    fn load_yaml_bool_true_still_works() {
        let v = load_yaml("flag: true").unwrap();
        assert_eq!(v["flag"], true);
    }

    #[test]
    fn load_yaml_invalid_returns_err() {
        assert!(load_yaml(": invalid: yaml: [unclosed").is_err());
    }
}
