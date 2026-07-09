//! Schema models for declarative procedures.
//!
//! Ported from `src/harsh_realm/procedures/schema.py`. Pydantic `model_validator`
//! hooks become explicit `validate` methods; use [`Procedure::from_record`] to
//! deserialize-and-validate the way Python's `model_validate` does.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::runtime::{JsonObject, JsonValue};

/// Alias matching the Python `ProcedureValue = JsonValue`.
pub type ProcedureValue = JsonValue;

/// Declared type of a procedure input parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProcedureInputType {
    /// Accepts a string value.
    String,
    /// Accepts an integer value (not a boolean).
    Integer,
    /// Accepts a boolean value.
    Boolean,
    /// Accepts any value.
    #[default]
    Any,
}

/// The kind of executable step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcedureStepKind {
    /// Roll on a table.
    Roll,
    /// Invoke a compute callable.
    Compute,
    /// Run a nested procedure.
    Procedure,
    /// Render a template string.
    Format,
}

/// A named input parameter accepted by a procedure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcedureInput {
    /// Parameter name.
    pub name: String,
    /// Declared type.
    #[serde(default, rename = "type")]
    pub r#type: ProcedureInputType,
    /// Whether the parameter is required when no default is given.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Default value (`str | int | bool | None` in Python).
    #[serde(default)]
    pub default: Option<JsonValue>,
}

fn default_true() -> bool {
    true
}

fn default_count() -> i32 {
    1
}

/// One executable step in a declarative procedure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcedureStep {
    /// Step kind.
    pub kind: ProcedureStepKind,
    /// Output variable name.
    pub assign: String,
    /// Table id (roll steps).
    #[serde(default)]
    pub table: Option<String>,
    /// Compute function id (compute steps).
    #[serde(default)]
    pub function: Option<String>,
    /// Nested procedure id (procedure steps).
    #[serde(default)]
    pub procedure: Option<String>,
    /// Template string (format steps).
    #[serde(default)]
    pub template: Option<String>,
    /// Step parameter templates.
    #[serde(default)]
    pub params: BTreeMap<String, String>,
    /// Number of times to run the step.
    #[serde(default = "default_count")]
    pub count: i32,
}

impl ProcedureStep {
    /// Validate that the step's target field matches its kind.
    ///
    /// Mirrors the Python `step_has_exactly_one_target` model validator.
    pub fn validate(&self) -> Result<(), String> {
        if self.count < 1 {
            return Err("Procedure step count must be >= 1.".to_string());
        }
        let required_field = match self.kind {
            ProcedureStepKind::Roll => "table",
            ProcedureStepKind::Compute => "function",
            ProcedureStepKind::Procedure => "procedure",
            ProcedureStepKind::Format => "template",
        };
        let populated: Vec<&str> = [
            ("table", self.table.is_some()),
            ("function", self.function.is_some()),
            ("procedure", self.procedure.is_some()),
            ("template", self.template.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, present)| present.then_some(name))
        .collect();
        if populated != [required_field] {
            return Err(format!(
                "Procedure step kind {:?} requires exactly {required_field:?} \
                 and no other target fields.",
                self.kind
            ));
        }
        Ok(())
    }
}

/// Optional explicit output field mapping for a procedure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcedureOutput {
    /// Output name -> source variable path.
    pub fields: BTreeMap<String, String>,
}

/// A multi-step declarative procedure record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Procedure {
    /// Stable content id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Optional description.
    #[serde(default)]
    pub description: String,
    /// Declared inputs.
    #[serde(default)]
    pub inputs: Vec<ProcedureInput>,
    /// Ordered steps.
    pub steps: Vec<ProcedureStep>,
    /// Optional output mapping.
    #[serde(default)]
    pub output: Option<ProcedureOutput>,
    /// Free-form tags.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Procedure {
    /// Deserialize a content record and validate it, like `model_validate`.
    pub fn from_record(record: JsonObject) -> Result<Procedure, String> {
        let procedure: Procedure =
            serde_json::from_value(JsonValue::Object(record)).map_err(|e| e.to_string())?;
        procedure.validate()?;
        Ok(procedure)
    }

    /// Validate the procedure and each of its steps.
    pub fn validate(&self) -> Result<(), String> {
        for step in &self.steps {
            step.validate()?;
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for step in &self.steps {
            if !seen.insert(step.assign.as_str()) {
                return Err("Procedure step assign names must be unique.".to_string());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(json: serde_json::Value) -> JsonObject {
        match json {
            serde_json::Value::Object(map) => map,
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn roll_step_requires_table_only() {
        let step: ProcedureStep = serde_json::from_value(serde_json::json!({
            "kind": "roll", "assign": "x", "table": "t",
        }))
        .unwrap();
        assert!(step.validate().is_ok());

        let bad: ProcedureStep = serde_json::from_value(serde_json::json!({
            "kind": "roll", "assign": "x", "table": "t", "function": "f",
        }))
        .unwrap();
        assert!(bad.validate().is_err());

        let missing: ProcedureStep = serde_json::from_value(serde_json::json!({
            "kind": "compute", "assign": "x",
        }))
        .unwrap();
        assert!(missing.validate().is_err());
    }

    #[test]
    fn procedure_rejects_duplicate_assigns() {
        let dup = Procedure::from_record(record(serde_json::json!({
            "id": "p", "name": "P",
            "steps": [
                {"kind": "format", "assign": "a", "template": "x"},
                {"kind": "format", "assign": "a", "template": "y"},
            ],
        })));
        assert!(dup.is_err());

        let ok = Procedure::from_record(record(serde_json::json!({
            "id": "p", "name": "P",
            "steps": [
                {"kind": "format", "assign": "a", "template": "x"},
                {"kind": "format", "assign": "b", "template": "y"},
            ],
        })));
        assert!(ok.is_ok());
    }

    #[test]
    fn input_defaults() {
        let inp: ProcedureInput =
            serde_json::from_value(serde_json::json!({"name": "n"})).unwrap();
        assert_eq!(inp.r#type, ProcedureInputType::Any);
        assert!(inp.required);
        assert!(inp.default.is_none());
    }
}
