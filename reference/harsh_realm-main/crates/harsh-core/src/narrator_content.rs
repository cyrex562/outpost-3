//! Typed authored narrator template documents.
//!
//! Ported from `src/harsh_realm/models/narrator_content.py`. Template maps are
//! keyed by terrain/feature/direction name (data-driven).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Terrain/feature/direction -> list of description variants.
pub type TerrainDescriptionMap = BTreeMap<String, Vec<String>>;
/// Feature name -> prefix string.
pub type FeaturePrefixMap = BTreeMap<String, String>;
/// Direction -> list of movement description variants.
pub type MovementDescriptionMap = BTreeMap<String, Vec<String>>;

/// Terrain description template document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TerrainDescriptionsDocument {
    /// Description variants keyed by terrain id.
    #[serde(default)]
    pub descriptions: TerrainDescriptionMap,
    /// Suffixes appended on first visit.
    #[serde(default)]
    pub first_visit_suffix: Vec<String>,
    /// Feature prefix strings.
    #[serde(default)]
    pub feature_prefix: FeaturePrefixMap,
}

/// Movement description template document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MovementDescriptionsDocument {
    /// Movement variants keyed by direction.
    #[serde(default)]
    pub movement: MovementDescriptionMap,
}

/// Adjacent-terrain hint template document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdjacentHintsDocument {
    /// Hint variants.
    #[serde(default)]
    pub hints: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_doc_parses_maps() {
        let doc: TerrainDescriptionsDocument = serde_json::from_str(
            r#"{"descriptions":{"plains":["flat"]},"feature_prefix":{"ruin":"a ruined"}}"#,
        )
        .unwrap();
        assert_eq!(doc.descriptions["plains"], vec!["flat".to_string()]);
        assert_eq!(doc.feature_prefix["ruin"], "a ruined");
    }
}
