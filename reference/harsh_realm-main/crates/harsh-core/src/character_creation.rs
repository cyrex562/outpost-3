//! Character-creation content models.
//!
//! Ported from `src/harsh_realm/models/character_creation.py` and
//! `src/harsh_realm/models/character_submission.py`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::runtime::InventoryItemRecord;

// ---------------------------------------------------------------------------
// Equipment kits
// ---------------------------------------------------------------------------

/// One selectable starting-equipment kit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipmentKit {
    pub id: String,
    pub name: String,
    #[serde(rename = "class")]
    pub character_class: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub items: Vec<InventoryItemRecord>,
}

/// Collection of equipment kits loaded from YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EquipmentKitCatalog {
    #[serde(default)]
    pub kits: Vec<EquipmentKit>,
}

// ---------------------------------------------------------------------------
// Skill models
// ---------------------------------------------------------------------------

/// One declarative constraint for a skill to be takeable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillRequirement {
    #[serde(rename = "type")]
    pub requirement_type: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub attribute: Option<String>,
    #[serde(default)]
    pub min: Option<i32>,
    #[serde(default)]
    pub trait_id: Option<String>,
}

/// One class-recommended skill pick used by the auto-picker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecommendedSkill {
    pub skill: String,
    #[serde(default)]
    pub level: i32,
}

/// One skill definition loaded from `skills.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub default_attribute: String,
    #[serde(default)]
    pub alternate_attributes: Vec<String>,
    #[serde(default = "default_true")]
    pub can_use_untrained: bool,
    #[serde(default)]
    pub combat_skill: bool,
    #[serde(default = "default_milestone")]
    pub available_milestone: i32,
    #[serde(default = "default_general")]
    pub category: String,
    #[serde(default)]
    pub subcategory: Option<String>,
    #[serde(default)]
    pub requirements: Vec<SkillRequirement>,
}

fn default_true() -> bool { true }
fn default_milestone() -> i32 { 99 }
fn default_general() -> String { "General".to_string() }

/// Collection of skill definitions from `skills.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SkillCatalog {
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
}

// ---------------------------------------------------------------------------
// Submission models (forms-based character creation)
// ---------------------------------------------------------------------------

/// Attribute-generation method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeMethod {
    Roll,
    StandardArray,
    PointBuy,
}

/// A complete character-creation submission from the forms UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterDraftSubmission {
    pub name: String,
    pub character_class: String,
    pub attribute_method: AttributeMethod,
    pub attributes: BTreeMap<String, i32>,
    #[serde(default)]
    pub skills: BTreeMap<String, i32>,
    pub kit_id: String,
    #[serde(default)]
    pub rolled_scores: Option<Vec<i32>>,
}

/// One selectable class, summarised for the creation form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassOption {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub hit_die: i32,
    pub skill_points_start: i32,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default)]
    pub recommended_skills: Vec<RecommendedSkill>,
}

/// One selectable skill for the creation form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillOption {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_attribute: String,
    #[serde(default = "default_true")]
    pub can_use_untrained: bool,
    #[serde(default)]
    pub combat_skill: bool,
    #[serde(default = "default_milestone")]
    pub available_milestone: i32,
    #[serde(default = "default_general")]
    pub category: String,
    #[serde(default)]
    pub subcategory: Option<String>,
    #[serde(default)]
    pub requirements: Vec<SkillRequirement>,
}

/// Point-buy configuration for the creation form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointBuyConfig {
    pub budget: i32,
    pub min_score: i32,
    pub max_score: i32,
    pub costs: BTreeMap<i32, i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipment_kit_deserializes_class_alias() {
        let json = r#"{"id":"k1","name":"Scout Kit","class":"expert","items":[]}"#;
        let kit: EquipmentKit = serde_json::from_str(json).unwrap();
        assert_eq!(kit.character_class, "expert");
    }

    #[test]
    fn skill_definition_defaults() {
        let json = r#"{"id":"stab","name":"Stab","default_attribute":"str"}"#;
        let def: SkillDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.available_milestone, 99);
        assert!(def.can_use_untrained);
        assert_eq!(def.category, "General");
        assert!(def.requirements.is_empty());
    }

    #[test]
    fn skill_requirement_type_field() {
        let json = r#"{"type":"class","classes":["warrior"]}"#;
        let req: SkillRequirement = serde_json::from_str(json).unwrap();
        assert_eq!(req.requirement_type, "class");
        assert_eq!(req.classes, vec!["warrior"]);
    }

    #[test]
    fn attribute_method_roundtrip() {
        let m = AttributeMethod::StandardArray;
        let s = serde_json::to_string(&m).unwrap();
        assert_eq!(s, "\"standard_array\"");
        let back: AttributeMethod = serde_json::from_str(&s).unwrap();
        assert_eq!(back, m);
    }
}
