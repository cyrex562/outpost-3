//! Entity-component vocabulary IR — mirrors the Python content schemas in
//! `harsh_realm.modifiers`, `traits`, `tags`, `status_effects`, and
//! `procedures`. Field names and defaults match the Pydantic models so authored
//! records round-trip unchanged.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::ir::effect::Trigger;

// ---------------------------------------------------------------------------
// Modifiers (modifiers/schema.py)
// ---------------------------------------------------------------------------

/// How a modifier stacks with others targeting the same value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModifierStacking {
    #[default]
    Additive,
    Multiplicative,
    Replace,
    Max,
    Min,
}

/// A flat condition predicate gating a modifier (Phase 2 vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModifierPredicate {
    Always,
    EntityHasTag,
    TargetHasTag,
    EntityHasTrait,
}

/// A modifier condition: a predicate plus an optional argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModifierCondition {
    pub predicate: ModifierPredicate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arg: Option<String>,
}

impl Default for ModifierCondition {
    fn default() -> Self {
        Self {
            predicate: ModifierPredicate::Always,
            arg: None,
        }
    }
}

/// A single modifier contribution from some source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Modifier {
    /// Namespaced target, e.g. `attribute.str`.
    pub target: String,
    /// Magnitude; sign indicates bonus or penalty.
    pub value: i64,
    #[serde(default)]
    pub stacking: ModifierStacking,
    /// For replace mode; higher priority wins.
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub condition: ModifierCondition,
    #[serde(default)]
    pub description: String,
}

// ---------------------------------------------------------------------------
// Traits (traits/schema.py)
// ---------------------------------------------------------------------------

/// A requirement that must be met before an entity can gain a trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PrerequisiteKind {
    Trait,
    AttributeMin,
    LevelMin,
    SkillMin,
}

/// A trait prerequisite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Prerequisite {
    pub kind: PrerequisiteKind,
    pub arg: String,
    #[serde(default)]
    pub value: i64,
}

/// How acquiring a trait costs a character in a source system.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TraitCost {
    #[serde(default)]
    pub points: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    #[serde(default)]
    pub description: String,
}

/// A trait, feature, edge, advantage, gift, feat, talent, or ability record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Trait {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// advantage, disadvantage, edge, hindrance, gift, feat, talent, etc.
    pub category: String,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
    #[serde(default)]
    pub provides_tags: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<Prerequisite>,
    /// Qualified trait IDs that cannot coexist with this trait.
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<TraitCost>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

// ---------------------------------------------------------------------------
// Tags
// ---------------------------------------------------------------------------

/// A tag definition. Tags are lightweight markers; `implies` lets one tag pull
/// in others for query purposes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tag {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub implies: Vec<String>,
}

// ---------------------------------------------------------------------------
// Status effects (status_effects/schema.py)
// ---------------------------------------------------------------------------

/// How a re-application interacts with an existing status instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StatusStacking {
    #[default]
    Replace,
    Extend,
    Stack,
}

/// A status effect content record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusEffect {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// 0 means permanent until explicitly removed.
    #[serde(default)]
    pub default_duration_ticks: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub provides_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub stacking: StatusStacking,
    #[serde(default)]
    pub modifiers: Vec<Modifier>,
    #[serde(default)]
    pub triggers: Vec<Trigger>,
}

// ---------------------------------------------------------------------------
// Procedures (procedures/schema.py)
// ---------------------------------------------------------------------------

/// The static type of a procedure input parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureInputType {
    String,
    Integer,
    Boolean,
    #[default]
    Any,
}

/// A named input parameter accepted by a procedure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureInput {
    pub name: String,
    #[serde(default)]
    pub r#type: ProcedureInputType,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

fn default_count() -> u32 {
    1
}

/// The kind of a procedure step (selects which target field is required).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProcedureStepKind {
    Roll,
    Compute,
    Procedure,
    Format,
}

/// One executable step in a declarative procedure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureStep {
    pub kind: ProcedureStepKind,
    /// Output variable name.
    pub assign: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub procedure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    #[serde(default = "default_count")]
    pub count: u32,
}

/// An explicit output field mapping for a procedure (`var.path` per output key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProcedureOutput {
    pub fields: BTreeMap<String, String>,
}

/// A declarative, multi-step generator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Procedure {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub inputs: Vec<ProcedureInput>,
    #[serde(default)]
    pub steps: Vec<ProcedureStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<ProcedureOutput>,
    #[serde(default)]
    pub tags: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------

/// A random table content record. Entries stay open (`serde_json::Value`) until
/// the table engine is ported in R4 and can formalize entry shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Table {
    pub id: String,
    pub category: String,
    pub name: String,
    #[serde(default)]
    pub entries: Vec<serde_json::Value>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn modifier_applies_python_defaults() {
        let m: Modifier =
            serde_json::from_value(json!({"target": "attribute.str", "value": 2})).unwrap();
        assert_eq!(m.stacking, ModifierStacking::Additive);
        assert_eq!(m.priority, 0);
        assert_eq!(m.condition.predicate, ModifierPredicate::Always);
        assert!(m.condition.arg.is_none());
    }

    #[test]
    fn trait_minimal_record_round_trips() {
        let value = json!({"id": "tough", "name": "Tough", "category": "edge"});
        let t: Trait = serde_json::from_value(value).unwrap();
        assert_eq!(t.id, "tough");
        assert!(t.modifiers.is_empty());
        assert!(t.cost.is_none());
        // Round-trip: empty collections are emitted, not dropped.
        let v = serde_json::to_value(&t).unwrap();
        assert_eq!(v["modifiers"], json!([]));
        assert!(v.get("cost").is_none(), "None cost is skipped");
    }

    #[test]
    fn status_effect_defaults_replace_stacking() {
        let s: StatusEffect =
            serde_json::from_value(json!({"id": "poisoned", "name": "Poisoned"})).unwrap();
        assert_eq!(s.stacking, StatusStacking::Replace);
        assert_eq!(s.default_duration_ticks, 0);
    }

    #[test]
    fn procedure_step_keeps_type_and_count_defaults() {
        let step: ProcedureStep = serde_json::from_value(json!({
            "kind": "roll", "assign": "result", "table": "encounters"
        }))
        .unwrap();
        assert_eq!(step.count, 1);
        assert_eq!(step.kind, ProcedureStepKind::Roll);
    }

    #[test]
    fn procedure_input_type_field_uses_type_key() {
        let inp: ProcedureInput =
            serde_json::from_value(json!({"name": "n", "type": "integer"})).unwrap();
        assert_eq!(inp.r#type, ProcedureInputType::Integer);
        assert!(inp.required);
        let v = serde_json::to_value(&inp).unwrap();
        assert_eq!(v["type"], json!("integer"));
    }
}
