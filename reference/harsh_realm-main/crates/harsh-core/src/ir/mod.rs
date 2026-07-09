//! The Harsh Realm intermediate representation (IR).
//!
//! One versioned vocabulary of declarative components — the single source of
//! truth shared by the Rust engine, the Python (Pydantic) validation layer, and
//! the LLM authoring target. Each type derives `serde` (wire format) and
//! `schemars::JsonSchema` (exported schema). Field names, defaults, and tag
//! strings deliberately match the existing Python content schemas so authored
//! records round-trip across the boundary unchanged.

pub mod action;
pub mod components;
pub mod condition;
pub mod effect;
pub mod entity;
pub mod record;

pub use action::{
    Action, ActionKind, ActionPrerequisite, ActionResolution, DifficultyRef, OutcomeBranch,
    RollSpec, Skill, SkillGoverns, TnSource,
};
pub use components::{
    Modifier, ModifierCondition, ModifierPredicate, ModifierStacking, Prerequisite,
    PrerequisiteKind, Procedure, ProcedureInput, ProcedureInputType, ProcedureOutput,
    ProcedureStep, ProcedureStepKind, StatusEffect, StatusStacking, Table, Tag, Trait, TraitCost,
};
pub use condition::{BinaryOperator, Expr, LiteralKind, PathRoot, UnaryOperator};
pub use effect::{Effect, EffectKind, Trigger};
pub use entity::{Creature, Harvestable, Item, ItemEffect, SaveBonusProfile};
pub use record::ComponentRecord;

/// Semantic version of the IR vocabulary. Bump on any breaking schema change.
///
/// Records and the exported schema carry this so the authoring layer and
/// migrations can detect mismatches.
pub const SCHEMA_VERSION: &str = "0.2.0";

/// Build the JSON Schema for the top-level [`ComponentRecord`] envelope.
///
/// This is the canonical schema: what the LLM authoring prompt targets. Exported
/// by the `export-schema` binary and pinned in the repo-root
/// `schema/harsh-ir.schema.json` (a CI drift check keeps them in sync).
pub fn component_record_schema() -> schemars::schema::RootSchema {
    schemars::schema_for!(ComponentRecord)
}

/// The pinned schema serialized as pretty JSON (with a trailing newline).
pub fn component_record_schema_json() -> String {
    let schema = component_record_schema();
    let mut s = serde_json::to_string_pretty(&schema).expect("schema serializes");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_has_component_type_discriminator() {
        let json = component_record_schema_json();
        assert!(
            json.contains("component_type"),
            "schema tags on component_type"
        );
        // All record kinds should appear as discriminator values.
        for kind in [
            "trait",
            "tag",
            "status_effect",
            "trigger",
            "procedure",
            "table",
            "action",
            "skill",
            "creature",
            "item",
        ] {
            assert!(json.contains(kind), "schema mentions {kind}");
        }
    }

    #[test]
    fn schema_json_is_stable_across_calls() {
        // Determinism is required for the drift check to be meaningful.
        assert_eq!(
            component_record_schema_json(),
            component_record_schema_json()
        );
    }

    #[test]
    fn committed_schema_matches_export() {
        // Drift gate: the committed schema must equal the current export. If this
        // fails, regenerate with `cargo run --bin export-schema` and commit.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../schema/harsh-ir.schema.json");
        let committed = std::fs::read_to_string(path)
            .expect("schema/harsh-ir.schema.json must exist at the repo root (run export-schema)");
        assert_eq!(
            committed,
            component_record_schema_json(),
            "IR schema drift: run `cargo run --bin export-schema` and commit the result",
        );
    }
}
