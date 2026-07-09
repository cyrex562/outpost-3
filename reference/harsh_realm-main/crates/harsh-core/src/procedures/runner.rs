//! Runtime execution for declarative procedures.
//!
//! Ported from `src/harsh_realm/procedures/runner.py`. The Python runner renders
//! step parameters and `format` templates with `str.format(**variables)`; the
//! only template syntax used by any pack is `{name}` and dotted/indexed access
//! such as `{x.field}` / `{x[key]}`, which this renderer supports. Python format
//! specs (`{x:>5}`) are not used and not implemented.

use std::collections::BTreeMap;

use rand::Rng;

use crate::packs::content_service::ContentService;
use crate::runtime::{JsonObject, JsonValue};
use crate::table_engine::{TableEngine, TableError};

use super::compute_registry::ComputeRegistry;
use super::schema::{Procedure, ProcedureInput, ProcedureInputType, ProcedureStepKind, ProcedureValue};

const MAX_RECURSION_DEPTH: u32 = 10;

/// Errors raised by procedure execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcedureError {
    /// Inputs failed validation (Python `ProcedureValidationError`).
    Validation(String),
    /// Execution failed (Python `ProcedureExecutionError`).
    Execution(String),
}

impl std::fmt::Display for ProcedureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcedureError::Validation(m) => write!(f, "{m}"),
            ProcedureError::Execution(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for ProcedureError {}

/// Execute pack-defined procedures against typed inputs.
pub struct ProcedureRunner<'a, TR: Rng, R: Rng> {
    content: &'a ContentService<'a>,
    tables: &'a mut TableEngine<'a, TR>,
    compute_registry: &'a ComputeRegistry,
    rng: R,
}

impl<'a, TR: Rng, R: Rng> ProcedureRunner<'a, TR, R> {
    /// Construct a runner over content, a table engine, a compute registry, and an RNG.
    pub fn new(
        content: &'a ContentService<'a>,
        tables: &'a mut TableEngine<'a, TR>,
        compute_registry: &'a ComputeRegistry,
        rng: R,
    ) -> Self {
        ProcedureRunner {
            content,
            tables,
            compute_registry,
            rng,
        }
    }

    /// Execute a procedure by qualified content id.
    pub fn run(
        &mut self,
        procedure_id: &str,
        inputs: BTreeMap<String, ProcedureValue>,
    ) -> Result<BTreeMap<String, ProcedureValue>, ProcedureError> {
        self.run_inner(procedure_id, inputs, 0)
    }

    fn run_inner(
        &mut self,
        procedure_id: &str,
        inputs: BTreeMap<String, ProcedureValue>,
        recursion_depth: u32,
    ) -> Result<BTreeMap<String, ProcedureValue>, ProcedureError> {
        if recursion_depth > MAX_RECURSION_DEPTH {
            return Err(ProcedureError::Execution(format!(
                "Procedure recursion depth exceeded {MAX_RECURSION_DEPTH}."
            )));
        }

        let record = self
            .content
            .get(procedure_id)
            .map_err(ProcedureError::Execution)?
            .ok_or_else(|| {
                ProcedureError::Execution(format!("Procedure not found: {procedure_id}"))
            })?;
        let procedure =
            Procedure::from_record(record).map_err(ProcedureError::Execution)?;

        let validated_inputs = self.validate_inputs(&procedure.inputs, inputs)?;
        let mut variables: BTreeMap<String, ProcedureValue> = validated_inputs.clone();
        let input_names: Vec<String> = validated_inputs.keys().cloned().collect();

        for step in &procedure.steps {
            let mut results: Vec<ProcedureValue> = Vec::new();
            for _ in 0..step.count {
                let resolved_params = self.resolve_params(&step.params, &variables)?;
                let value = match step.kind {
                    ProcedureStepKind::Roll => {
                        let table = step.table.as_ref().ok_or_else(|| {
                            ProcedureError::Execution(format!(
                                "Roll step {:?} is missing a table.",
                                step.assign
                            ))
                        })?;
                        self.roll(table, &resolved_params)?
                    }
                    ProcedureStepKind::Compute => {
                        let function = step.function.as_ref().ok_or_else(|| {
                            ProcedureError::Execution(format!(
                                "Compute step {:?} is missing a function.",
                                step.assign
                            ))
                        })?;
                        self.compute_registry
                            .invoke(function, &resolved_params)
                            .map_err(|_| {
                                ProcedureError::Execution(format!(
                                    "Compute function failed: {function}"
                                ))
                            })?
                    }
                    ProcedureStepKind::Procedure => {
                        let sub = step.procedure.as_ref().ok_or_else(|| {
                            ProcedureError::Execution(format!(
                                "Procedure step {:?} is missing a procedure.",
                                step.assign
                            ))
                        })?;
                        let sub_params: BTreeMap<String, ProcedureValue> = resolved_params
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        let sub_result = self.run_inner(sub, sub_params, recursion_depth + 1)?;
                        JsonValue::Object(sub_result.into_iter().collect())
                    }
                    ProcedureStepKind::Format => {
                        let template = step.template.as_ref().ok_or_else(|| {
                            ProcedureError::Execution(format!(
                                "Format step {:?} is missing a template.",
                                step.assign
                            ))
                        })?;
                        JsonValue::String(self.render_template(template, &variables)?)
                    }
                };
                results.push(value);
            }

            let assigned = if step.count > 1 {
                JsonValue::Array(results)
            } else {
                results.into_iter().next().unwrap_or(JsonValue::Null)
            };
            variables.insert(step.assign.clone(), assigned);
        }

        if let Some(output) = &procedure.output {
            let mut out: BTreeMap<String, ProcedureValue> = BTreeMap::new();
            for (output_name, variable_name) in &output.fields {
                out.insert(
                    output_name.clone(),
                    self.resolve_variable_path(variable_name, &variables)?,
                );
            }
            return Ok(out);
        }

        Ok(variables
            .into_iter()
            .filter(|(name, _)| !input_names.contains(name))
            .collect())
    }

    fn validate_inputs(
        &self,
        definitions: &[ProcedureInput],
        inputs: BTreeMap<String, ProcedureValue>,
    ) -> Result<BTreeMap<String, ProcedureValue>, ProcedureError> {
        let mut validated: BTreeMap<String, ProcedureValue> = BTreeMap::new();
        for definition in definitions {
            let value = if let Some(value) = inputs.get(&definition.name) {
                value.clone()
            } else if let Some(default) = &definition.default {
                if default.is_null() {
                    if definition.required {
                        return Err(ProcedureError::Validation(format!(
                            "Missing required procedure input: {}",
                            definition.name
                        )));
                    }
                    JsonValue::Null
                } else {
                    default.clone()
                }
            } else if definition.required {
                return Err(ProcedureError::Validation(format!(
                    "Missing required procedure input: {}",
                    definition.name
                )));
            } else {
                JsonValue::Null
            };

            if !value_matches_type(&value, definition.r#type) {
                return Err(ProcedureError::Validation(format!(
                    "Procedure input {:?} must be {:?}.",
                    definition.name, definition.r#type
                )));
            }
            validated.insert(definition.name.clone(), value);
        }
        Ok(validated)
    }

    fn resolve_params(
        &self,
        params: &BTreeMap<String, String>,
        variables: &BTreeMap<String, ProcedureValue>,
    ) -> Result<JsonObject, ProcedureError> {
        let mut resolved = JsonObject::new();
        for (name, template) in params {
            resolved.insert(
                name.clone(),
                JsonValue::String(self.render_template(template, variables)?),
            );
        }
        Ok(resolved)
    }

    fn render_template(
        &self,
        template: &str,
        variables: &BTreeMap<String, ProcedureValue>,
    ) -> Result<String, ProcedureError> {
        let mut out = String::new();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        out.push('{');
                        continue;
                    }
                    let mut expr = String::new();
                    for inner in chars.by_ref() {
                        if inner == '}' {
                            break;
                        }
                        expr.push(inner);
                    }
                    out.push_str(&self.resolve_template_expr(&expr, variables)?);
                }
                '}' => {
                    if chars.peek() == Some(&'}') {
                        chars.next();
                    }
                    out.push('}');
                }
                other => out.push(other),
            }
        }
        Ok(out)
    }

    fn resolve_template_expr(
        &self,
        expr: &str,
        variables: &BTreeMap<String, ProcedureValue>,
    ) -> Result<String, ProcedureError> {
        let root = expr
            .split(['.', '['])
            .next()
            .unwrap_or(expr)
            .trim()
            .to_string();
        let mut current = variables.get(&root).ok_or_else(|| {
            ProcedureError::Execution(format!("Missing template variable: {root}"))
        })?;
        // Walk dotted / indexed segments after the root.
        let rest = &expr[root.len()..];
        for segment in split_path_segments(rest) {
            current = match current {
                JsonValue::Object(map) => map.get(&segment).ok_or_else(|| {
                    ProcedureError::Execution(format!("Missing template variable: {expr}"))
                })?,
                _ => {
                    return Err(ProcedureError::Execution(format!(
                        "Missing template variable: {expr}"
                    )))
                }
            };
        }
        Ok(value_to_text(current))
    }

    fn roll(
        &mut self,
        table_id: &str,
        params: &JsonObject,
    ) -> Result<ProcedureValue, ProcedureError> {
        let engine_id = table_id_for_engine(table_id);
        match self.tables.roll_on(&engine_id, Some(params)) {
            Ok(result) => {
                if result.result_type == "text" {
                    Ok(JsonValue::String(result.raw_text.unwrap_or_default()))
                } else {
                    Ok(JsonValue::Object(result.result))
                }
            }
            Err(TableError::NotFound(_)) => self.roll_content_table(table_id, params),
            Err(e) => Err(ProcedureError::Execution(e.to_string())),
        }
    }

    fn roll_content_table(
        &mut self,
        table_id: &str,
        params: &JsonObject,
    ) -> Result<ProcedureValue, ProcedureError> {
        let record = self
            .content
            .get(table_id)
            .map_err(ProcedureError::Execution)?
            .ok_or_else(|| {
                ProcedureError::Execution(format!("Table not found: {table_id}"))
            })?;
        let entries_value = record.get("entries");
        let entries = match entries_value {
            Some(JsonValue::Array(list)) if !list.is_empty() => list,
            _ => {
                return Err(ProcedureError::Execution(format!(
                    "Table has no entries: {table_id}"
                )))
            }
        };

        let mut weights: Vec<f64> = Vec::new();
        let mut results: Vec<JsonValue> = Vec::new();
        for entry_value in entries {
            let JsonValue::Object(entry) = entry_value else {
                return Err(ProcedureError::Execution(format!(
                    "Table entry must be a mapping: {table_id}"
                )));
            };
            let result = entry.get("result").ok_or_else(|| {
                ProcedureError::Execution(format!("Table entry is missing result: {table_id}"))
            })?;
            let weight = match entry.get("weight") {
                None => 1.0,
                Some(JsonValue::Number(n)) => n.as_f64().unwrap_or(1.0),
                Some(_) => {
                    return Err(ProcedureError::Execution(format!(
                        "Table entry weight must be numeric: {table_id}"
                    )))
                }
            };
            weights.push(weight);
            results.push(result.clone());
        }

        let total: f64 = weights.iter().sum();
        let r = if total > 0.0 {
            self.rng.gen_range(0.0..total)
        } else {
            0.0
        };
        let index = crate::tables::weighted_select(&weights, r);
        let chosen = &results[index];
        if let JsonValue::Object(map) = chosen {
            if let Some(JsonValue::String(subtable)) = map.get("table") {
                let subtable = subtable.clone();
                return self.roll_content_table(&subtable, params);
            }
        }
        Ok(chosen.clone())
    }

    fn resolve_variable_path(
        &self,
        variable_path: &str,
        variables: &BTreeMap<String, ProcedureValue>,
    ) -> Result<ProcedureValue, ProcedureError> {
        let segments: Vec<&str> = variable_path.split('.').collect();
        let mut current = variables.get(segments[0]).ok_or_else(|| {
            ProcedureError::Execution(format!("Missing output variable: {}", segments[0]))
        })?;
        for segment in &segments[1..] {
            current = match current {
                JsonValue::Object(map) => map.get(*segment).ok_or_else(|| {
                    ProcedureError::Execution(format!(
                        "Missing output variable path: {variable_path}"
                    ))
                })?,
                _ => {
                    return Err(ProcedureError::Execution(format!(
                        "Missing output variable path: {variable_path}"
                    )))
                }
            };
        }
        Ok(current.clone())
    }
}

/// Adapt qualified table content ids to legacy TableEngine table ids.
fn table_id_for_engine(table_id: &str) -> String {
    if let Some(rest) = table_id.strip_prefix("xwn-core:tables.") {
        rest.rsplit('.').next().unwrap_or(rest).to_string()
    } else {
        table_id.to_string()
    }
}

fn value_matches_type(value: &JsonValue, expected: ProcedureInputType) -> bool {
    match expected {
        ProcedureInputType::Any => true,
        ProcedureInputType::String => value.is_string(),
        ProcedureInputType::Integer => value.is_i64() || value.is_u64(),
        ProcedureInputType::Boolean => value.is_boolean(),
    }
}

/// Render a JSON value as Python `str()` would.
fn value_to_text(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Bool(true) => "True".to_string(),
        JsonValue::Bool(false) => "False".to_string(),
        JsonValue::Null => "None".to_string(),
        other => other.to_string(),
    }
}

/// Split the path after a root identifier into `.field` / `[key]` segments.
fn split_path_segments(rest: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    for c in rest.chars() {
        match c {
            '.' | '[' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
            ']' => {
                if !current.is_empty() {
                    segments.push(std::mem::take(&mut current));
                }
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::packs::loader::{Pack, PackRecords};
    use crate::packs::manifest::PackManifest;
    use crate::packs::registry::PackRegistry;
    use crate::repositories::world_pack::WorldPackRepository;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    fn pack_with(records: Vec<(&str, &str, serde_json::Value)>) -> Pack {
        let mut pack_records: PackRecords = PackRecords::new();
        for (category, slug, body) in records {
            let mut rec: JsonObject = match body {
                serde_json::Value::Object(m) => m,
                _ => panic!("record must be object"),
            };
            rec.insert(
                "_qualified_id".into(),
                serde_json::json!(format!("xwn-core:{category}.{slug}")),
            );
            rec.insert("_pack_id".into(), serde_json::json!("xwn-core"));
            rec.insert("_category".into(), serde_json::json!(category));
            rec.insert("_slug".into(), serde_json::json!(slug));
            pack_records
                .entry(category.to_string())
                .or_default()
                .insert(slug.to_string(), rec);
        }
        Pack {
            manifest: PackManifest {
                id: "xwn-core".into(),
                version: "1.0.0".into(),
                name: "xwn-core".into(),
                description: String::new(),
                authors: vec![],
                depends: vec![],
                conflicts: vec![],
                provides: vec![],
            },
            root_path: std::path::PathBuf::from("/tmp"),
            records: pack_records,
        }
    }

    #[test]
    fn runs_content_table_rolls_and_format_step() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let registry = PackRegistry::new(vec![pack_with(vec![
            (
                "procedures",
                "greet",
                serde_json::json!({
                    "id": "xwn-core:procedures.greet",
                    "name": "Greet",
                    "steps": [
                        {"kind": "roll", "assign": "word", "table": "xwn-core:tables.words"},
                        {"kind": "format", "assign": "line", "template": "say {word}!"},
                    ],
                    "output": {"fields": {"line": "line"}},
                }),
            ),
            (
                "tables",
                "words",
                serde_json::json!({
                    "id": "xwn-core:tables.words",
                    "name": "Words",
                    "entries": [{"result": "hi", "weight": 1}],
                }),
            ),
        ])])
        .unwrap();
        let content = ContentService::new(&registry, WorldPackRepository::new(&db));
        let mut tables = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let compute = ComputeRegistry::new();
        let mut runner =
            ProcedureRunner::new(&content, &mut tables, &compute, SmallRng::seed_from_u64(2));

        let out = runner.run("xwn-core:procedures.greet", BTreeMap::new()).unwrap();
        assert_eq!(out.get("line").unwrap().as_str(), Some("say hi!"));
    }

    #[test]
    fn missing_required_input_errors() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let registry = PackRegistry::new(vec![pack_with(vec![(
            "procedures",
            "need",
            serde_json::json!({
                "id": "xwn-core:procedures.need",
                "name": "Need",
                "inputs": [{"name": "who", "type": "string", "required": true}],
                "steps": [{"kind": "format", "assign": "x", "template": "{who}"}],
            }),
        )])])
        .unwrap();
        let content = ContentService::new(&registry, WorldPackRepository::new(&db));
        let mut tables = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let compute = ComputeRegistry::new();
        let mut runner =
            ProcedureRunner::new(&content, &mut tables, &compute, SmallRng::seed_from_u64(2));
        let err = runner.run("xwn-core:procedures.need", BTreeMap::new()).unwrap_err();
        assert!(matches!(err, ProcedureError::Validation(_)));
    }

    #[test]
    fn compute_step_and_output_path() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let registry = PackRegistry::new(vec![pack_with(vec![(
            "procedures",
            "calc",
            serde_json::json!({
                "id": "xwn-core:procedures.calc",
                "name": "Calc",
                "inputs": [{"name": "n", "type": "string", "required": true}],
                "steps": [
                    {"kind": "compute", "assign": "wrapped",
                     "function": "xwn-core:wrap", "params": {"v": "{n}"}},
                ],
                "output": {"fields": {"inner": "wrapped.value"}},
            }),
        )])])
        .unwrap();
        let content = ContentService::new(&registry, WorldPackRepository::new(&db));
        let mut tables = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let mut compute = ComputeRegistry::new();
        compute
            .register(
                "xwn-core:wrap",
                Box::new(|params| {
                    let v = params.get("v").and_then(|x| x.as_str()).unwrap_or("");
                    Ok(serde_json::json!({"value": v}))
                }),
            )
            .unwrap();
        let mut runner =
            ProcedureRunner::new(&content, &mut tables, &compute, SmallRng::seed_from_u64(2));
        let mut inputs = BTreeMap::new();
        inputs.insert("n".to_string(), serde_json::json!("deep"));
        let out = runner.run("xwn-core:procedures.calc", inputs).unwrap();
        assert_eq!(out.get("inner").unwrap().as_str(), Some("deep"));
    }

    #[test]
    fn missing_template_variable_errors() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let registry = PackRegistry::new(vec![pack_with(vec![(
            "procedures",
            "bad",
            serde_json::json!({
                "id": "xwn-core:procedures.bad",
                "name": "Bad",
                "steps": [{"kind": "format", "assign": "x", "template": "{ghost}"}],
            }),
        )])])
        .unwrap();
        let content = ContentService::new(&registry, WorldPackRepository::new(&db));
        let mut tables = TableEngine::new(&db, SmallRng::seed_from_u64(1));
        let compute = ComputeRegistry::new();
        let mut runner =
            ProcedureRunner::new(&content, &mut tables, &compute, SmallRng::seed_from_u64(2));
        assert!(runner.run("xwn-core:procedures.bad", BTreeMap::new()).is_err());
    }
}
