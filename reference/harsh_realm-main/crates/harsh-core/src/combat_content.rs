//! Authored combat narration content loaded from YAML; treat as immutable.
//!
//! Ported from `src/harsh_realm/models/combat_content.py`.

use serde::{Deserialize, Serialize};

/// Narration variants for one attack category.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AttackNarrationGroup {
    /// Melee variants.
    #[serde(default)]
    pub melee: Vec<String>,
    /// Bite variants.
    #[serde(default)]
    pub bite: Vec<String>,
    /// Ranged variants.
    #[serde(default)]
    pub ranged: Vec<String>,
    /// Punch variants.
    #[serde(default)]
    pub punch: Vec<String>,
}

/// Narration variants for flee outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FleeNarrationGroup {
    /// Clean getaway variants.
    #[serde(default)]
    pub clean: Vec<String>,
    /// Messy getaway variants.
    #[serde(default)]
    pub messy: Vec<String>,
}

/// Combat narration YAML document.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CombatNarrationDocument {
    /// Hit narration.
    #[serde(default)]
    pub hit: AttackNarrationGroup,
    /// Miss narration.
    #[serde(default)]
    pub miss: AttackNarrationGroup,
    /// Flee narration.
    #[serde(default)]
    pub flee: FleeNarrationGroup,
    /// Natural-1 variants.
    #[serde(default)]
    pub natural_1: Vec<String>,
    /// Natural-20 variants.
    #[serde(default)]
    pub natural_20: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document_defaults() {
        let doc: CombatNarrationDocument = serde_json::from_str("{}").unwrap();
        assert!(doc.hit.melee.is_empty());
        assert!(doc.natural_20.is_empty());
    }
}
