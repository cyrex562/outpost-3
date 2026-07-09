//! The top-level authorable record envelope.
//!
//! [`ComponentRecord`] is the single type the authoring layer produces and the
//! engine ingests: a discriminated union over `component_type`. It is the root
//! of the exported JSON Schema, so the LLM authoring target and the Python
//! validation contract both speak in terms of it.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ir::action::{Action, Skill};
use crate::ir::components::{Procedure, StatusEffect, Table, Tag, Trait};
use crate::ir::effect::Trigger;
use crate::ir::entity::{Creature, Item};

/// A single authorable content record, tagged by `component_type`.
///
/// Internally tagged: e.g. a trait serializes as
/// `{"component_type": "trait", "id": "...", "name": "...", ...}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "component_type", rename_all = "snake_case")]
pub enum ComponentRecord {
    Trait(Trait),
    Tag(Tag),
    StatusEffect(StatusEffect),
    Trigger(Trigger),
    Procedure(Procedure),
    Table(Table),
    Action(Action),
    Skill(Skill),
    Creature(Creature),
    Item(Item),
}

impl ComponentRecord {
    /// The `component_type` discriminator string for this record.
    pub fn component_type(&self) -> &'static str {
        match self {
            ComponentRecord::Trait(_) => "trait",
            ComponentRecord::Tag(_) => "tag",
            ComponentRecord::StatusEffect(_) => "status_effect",
            ComponentRecord::Trigger(_) => "trigger",
            ComponentRecord::Procedure(_) => "procedure",
            ComponentRecord::Table(_) => "table",
            ComponentRecord::Action(_) => "action",
            ComponentRecord::Skill(_) => "skill",
            ComponentRecord::Creature(_) => "creature",
            ComponentRecord::Item(_) => "item",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn trait_record_carries_component_type_tag() {
        let value = json!({
            "component_type": "trait",
            "id": "tough", "name": "Tough", "category": "edge"
        });
        let record: ComponentRecord = serde_json::from_value(value).unwrap();
        assert_eq!(record.component_type(), "trait");
        assert!(matches!(record, ComponentRecord::Trait(_)));
        // Serialization materializes defaults (like Pydantic's model_dump), and
        // re-parsing is value-preserving.
        let serialized = serde_json::to_value(&record).unwrap();
        assert_eq!(serialized["component_type"], json!("trait"));
        let reparsed: ComponentRecord = serde_json::from_value(serialized).unwrap();
        assert_eq!(reparsed, record);
    }

    #[test]
    fn status_effect_record_round_trips() {
        let record = ComponentRecord::StatusEffect(StatusEffect {
            id: "poisoned".into(),
            name: "Poisoned".into(),
            description: String::new(),
            default_duration_ticks: 3,
            tags: vec![],
            provides_tags: vec![],
            icon: None,
            stacking: Default::default(),
            modifiers: vec![],
            triggers: vec![],
        });
        let v = serde_json::to_value(&record).unwrap();
        assert_eq!(v["component_type"], json!("status_effect"));
        let back: ComponentRecord = serde_json::from_value(v).unwrap();
        assert_eq!(back, record);
    }
}
