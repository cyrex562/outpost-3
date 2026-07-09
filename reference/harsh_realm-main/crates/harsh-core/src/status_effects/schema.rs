//! Pydantic-equivalent schema for status effect content records.
//!
//! Ported from `src/harsh_realm/status_effects/schema.py`.

use serde::{Deserialize, Serialize};

/// How a re-application interacts with an existing instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Stacking {
    /// Overwrite the existing instance's duration.
    #[default]
    Replace,
    /// Sum the durations of overlapping instances.
    Extend,
    /// Allow multiple parallel instances.
    Stack,
}

/// A status effect content record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusEffect {
    /// Stable content id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: String,
    /// Default duration in ticks; 0 means permanent until explicitly removed.
    #[serde(default)]
    pub default_duration_ticks: i32,
    /// Free-form contextual tags.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Tags granted to the entity while this effect is active.
    #[serde(default)]
    pub provides_tags: Vec<String>,
    /// Optional UI icon reference.
    #[serde(default)]
    pub icon: Option<String>,
    /// Re-application stacking policy.
    #[serde(default)]
    pub stacking: Stacking,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_stacking_parse() {
        let effect: StatusEffect =
            serde_json::from_str(r#"{"id":"poison","name":"Poison","stacking":"stack"}"#).unwrap();
        assert_eq!(effect.stacking, Stacking::Stack);
        assert_eq!(effect.default_duration_ticks, 0);
        assert!(effect.tags.is_empty());

        let plain: StatusEffect =
            serde_json::from_str(r#"{"id":"blessed","name":"Blessed"}"#).unwrap();
        assert_eq!(plain.stacking, Stacking::Replace);
    }
}
