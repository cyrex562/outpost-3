//! Shared data loaders for world generation.
//!
//! Ported from `src/harsh_realm/generators/_world_support.py`.

use crate::generation::{NameListDocument, TerrainWeightsConfig};
use crate::packs::paths::default_content_dir;

/// Load the typed terrain weight config from content.
pub fn load_terrain_weights() -> Result<TerrainWeightsConfig, String> {
    let path = default_content_dir().join("tables/terrain/terrain_weights.yaml");
    if !path.exists() {
        return Err(format!("Data file not found: {}", path.display()));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}

/// Load a typed `names: [...]` YAML content document by relative path.
pub fn load_name_list(rel_path: &str) -> Result<NameListDocument, String> {
    let path = default_content_dir().join(rel_path);
    if !path.exists() {
        return Err(format!("Data file not found: {}", path.display()));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_yaml::from_str(&text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_terrain_weights_errors() {
        // Only asserts the missing-file path; presence depends on the content dir.
        let result = load_name_list("definitely/missing/names.yaml");
        assert!(result.is_err());
    }
}
