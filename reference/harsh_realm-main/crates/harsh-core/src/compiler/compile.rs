//! Whole-document compilation: form-2 YAML records → validated IR records.
//!
//! The host parses YAML to JSON and passes the documents here (so `harsh-core`
//! needs no YAML dependency; the deterministic *logic* stays in Rust). Each
//! document is a single-key map whose key is the record type
//! (`creature:`, `trait:`, …). The compiler lowers form-2 effects in every `do:`
//! list via the verb registry, stamps `component_type`, and validates by
//! deserializing into [`ComponentRecord`]. All diagnostics are collected; the
//! compile fails atomically if any are present (grammar spec §7).
//!
//! Covers top-level records, form-2 lowering, structural + semantic validation
//! (dice/conditions), and cross-reference resolution against opt-in catalogs.
//! Inline-record lifting (§4) is the remaining sugar (reference-by-ID is the
//! primary form).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::compiler::diagnostics::Diagnostic;
use crate::compiler::merge::resolve_action_base;
use crate::compiler::references::{resolve_references, Catalogs};
use crate::compiler::verbs::VerbRegistry;
use crate::ir::ComponentRecord;

/// The known top-level record types (the `ComponentRecord` variants).
pub const RECORD_TYPES: &[&str] = &[
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
];

/// The outcome of a compile: the two diagnostic buckets and, only when both are
/// empty, the emitted records.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CompileReport {
    pub syntax_errors: Vec<Diagnostic>,
    pub unimplemented: Vec<Diagnostic>,
    pub records: Vec<ComponentRecord>,
}

impl CompileReport {
    /// True if either diagnostic bucket is non-empty (no records were committed).
    pub fn failed(&self) -> bool {
        !self.syntax_errors.is_empty() || !self.unimplemented.is_empty()
    }

    fn push(&mut self, diag: Diagnostic) {
        if diag.is_syntax() {
            self.syntax_errors.push(diag);
        } else {
            self.unimplemented.push(diag);
        }
    }
}

/// Compile a set of documents (each a parsed YAML record) into IR records.
///
/// Collects every diagnostic across all documents; on any diagnostic, `records`
/// is left empty (nothing is committed).
pub fn compile(documents: &[Value], registry: &VerbRegistry, catalogs: &Catalogs) -> CompileReport {
    let mut report = CompileReport::default();

    // Phase 1 — parse each document and lower its form-2 effects.
    let mut parsed: Vec<(String, String, Map<String, Value>)> = Vec::new();
    for (i, doc) in documents.iter().enumerate() {
        let loc = format!("document[{i}]");
        match parse_and_lower(doc, registry) {
            Ok((record_type, body)) => parsed.push((loc, record_type, body)),
            Err(diags) => {
                for d in diags {
                    report.push(d.at(loc.clone()));
                }
            }
        }
    }

    // Phase 2 — build the action `base:` index: host-supplied archetypes first,
    // overlaid by in-unit action bodies (a unit's own archetype wins).
    let mut base_index: BTreeMap<String, Value> = BTreeMap::new();
    for (id, value) in &catalogs.action_bases {
        let mut value = value.clone();
        if let Some(object) = value.as_object_mut() {
            object.remove("component_type");
        }
        base_index.insert(id.clone(), value);
    }
    for (_loc, record_type, body) in &parsed {
        if record_type == "action" {
            if let Some(id) = body.get("id").and_then(Value::as_str) {
                base_index.insert(id.to_string(), Value::Object(body.clone()));
            }
        }
    }

    // Phase 3 — merge action archetypes, then finalize (deserialize + validate).
    let mut records = Vec::new();
    for (loc, record_type, body) in &parsed {
        let merged = if record_type == "action" && body.contains_key("base") {
            match resolve_action_base(body, &base_index, &mut Vec::new()) {
                Ok(merged) => merged,
                Err(d) => {
                    report.push(d.at(loc.clone()));
                    continue;
                }
            }
        } else {
            body.clone()
        };
        match finalize(record_type, merged) {
            Ok(record) => records.push(record),
            Err(diags) => {
                for d in diags {
                    report.push(d.at(loc.clone()));
                }
            }
        }
    }

    // Phase 4 — cross-reference resolution, only when everything else is clean.
    if !report.failed() {
        let ref_diags = resolve_references(&records, catalogs);
        if ref_diags.is_empty() {
            report.records = records;
        } else {
            for d in ref_diags {
                report.push(d);
            }
        }
    }
    report
}

/// Parse one document into `(record_type, body)`, lowering its `do:` effects.
fn parse_and_lower(
    doc: &Value,
    registry: &VerbRegistry,
) -> Result<(String, Map<String, Value>), Vec<Diagnostic>> {
    let obj = doc
        .as_object()
        .ok_or_else(|| vec![Diagnostic::syntax("a content document must be a mapping")])?;
    if obj.len() != 1 {
        return Err(vec![Diagnostic::syntax(format!(
            "a content document must have exactly one top-level record-type key, found {}",
            obj.len()
        ))]);
    }
    let (record_type, body) = obj.iter().next().expect("len == 1");
    if !RECORD_TYPES.contains(&record_type.as_str()) {
        return Err(vec![Diagnostic::syntax(format!(
            "unknown record type '{record_type}'"
        ))]);
    }
    let inner = body.as_object().ok_or_else(|| {
        vec![Diagnostic::syntax(format!(
            "'{record_type}' body must be a mapping"
        ))]
    })?;

    let mut diags = Vec::new();
    let mut wrapper = Value::Object(inner.clone());
    lower_do_lists(&mut wrapper, registry, &mut diags);
    if !diags.is_empty() {
        return Err(diags);
    }
    let lowered = wrapper.as_object().expect("still an object").clone();
    Ok((record_type.clone(), lowered))
}

/// Stamp the discriminator, deserialize into the IR, and validate semantics.
fn finalize(
    record_type: &str,
    mut inner: Map<String, Value>,
) -> Result<ComponentRecord, Vec<Diagnostic>> {
    inner.insert(
        "component_type".to_string(),
        Value::String(record_type.to_string()),
    );
    let record = serde_json::from_value::<ComponentRecord>(Value::Object(inner))
        .map_err(|e| vec![Diagnostic::syntax(format!("invalid {record_type}: {e}"))])?;

    let semantic = crate::compiler::validate::validate_semantics(&record);
    if !semantic.is_empty() {
        return Err(semantic);
    }
    Ok(record)
}

/// Recursively replace each element of every `do:` array with its lowered IR
/// effect (form-2 `{verb, args…}` → `{kind, params}`).
fn lower_do_lists(value: &mut Value, registry: &VerbRegistry, diags: &mut Vec<Diagnostic>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if key == "do" {
                    if let Value::Array(items) = child {
                        for item in items.iter_mut() {
                            lower_effect_item(item, registry, diags);
                        }
                        continue;
                    }
                }
                lower_do_lists(child, registry, diags);
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                lower_do_lists(item, registry, diags);
            }
        }
        _ => {}
    }
}

/// Lower one `do:` element in place.
///
/// Idempotent: an already-lowered raw effect (`{kind, params}`) passes through
/// unchanged, so recompiling stored IR (e.g. an edited record from the IR store)
/// works. Its structure is validated later when the record is deserialized.
fn lower_effect_item(item: &mut Value, registry: &VerbRegistry, diags: &mut Vec<Diagnostic>) {
    let Some(form) = item.as_object() else {
        diags.push(Diagnostic::syntax(
            "a `do:` entry must be a keyed verb mapping",
        ));
        return;
    };
    if is_raw_effect(form) {
        return;
    }
    match registry.lower(form) {
        Ok(effect) => {
            *item = serde_json::to_value(&effect).expect("effect serializes");
        }
        Err(d) => diags.push(d),
    }
}

/// True for an already-lowered effect: a `kind` key, and only `kind`/`params`.
/// No form-2 verb is named `kind`/`params`, so this is unambiguous.
fn is_raw_effect(form: &Map<String, Value>) -> bool {
    form.get("kind").map(Value::is_string).unwrap_or(false)
        && form.keys().all(|k| k == "kind" || k == "params")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reg() -> VerbRegistry {
        VerbRegistry::builtin()
    }

    #[test]
    fn compiles_a_status_effect_with_form2_trigger() {
        let doc = json!({
            "status_effect": {
                "id": "bleeding",
                "name": "Bleeding",
                "default_duration_ticks": 3,
                "stacking": "stack",
                "triggers": [
                    { "id": "tick", "on": "world.tick", "do": [
                        { "damage": "self", "amount": 1 }
                    ] }
                ]
            }
        });
        let report = compile(&[doc], &reg(), &Catalogs::default());
        assert!(!report.failed(), "diagnostics: {:?}", report.syntax_errors);
        assert_eq!(report.records.len(), 1);
        // The form-2 effect was lowered to a change_resource intent shape.
        match &report.records[0] {
            ComponentRecord::StatusEffect(s) => {
                let do_ = &s.triggers[0].do_;
                assert_eq!(do_[0].kind, "change_resource");
                assert_eq!(do_[0].params["delta"], json!(-1));
            }
            other => panic!("expected status_effect, got {other:?}"),
        }
    }

    #[test]
    fn raw_effects_pass_through_lowering() {
        // A trigger do-list that already holds a raw {kind, params} effect (as
        // stored IR would) recompiles cleanly — lowering is idempotent.
        let doc = json!({
            "status_effect": {
                "id": "bleeding", "name": "Bleeding", "stacking": "stack",
                "triggers": [ { "id": "t", "on": "world.tick", "do": [
                    { "kind": "change_resource",
                      "params": { "entity_id": "self", "resource": "hp", "delta": -1 } }
                ] } ]
            }
        });
        let report = compile(&[doc], &reg(), &Catalogs::default());
        assert!(!report.failed(), "{:?}", report.syntax_errors);
        match &report.records[0] {
            ComponentRecord::StatusEffect(s) => {
                assert_eq!(s.triggers[0].do_[0].kind, "change_resource");
                assert_eq!(s.triggers[0].do_[0].params["delta"], json!(-1));
            }
            other => panic!("expected status_effect, got {other:?}"),
        }
    }

    #[test]
    fn compiles_an_action_with_outcome_effects() {
        let doc = json!({
            "action": {
                "id": "unarmed_strike",
                "resolution": { "tn_source": { "defense": "ac" },
                                "roll_spec": { "skill": "punch", "attribute": "STR" } },
                "outcome": [
                    { "when": "success", "do": [ { "log": "thud" } ] }
                ]
            }
        });
        let report = compile(&[doc], &reg(), &Catalogs::default());
        assert!(!report.failed(), "{:?}", report.syntax_errors);
        match &report.records[0] {
            ComponentRecord::Action(a) => {
                assert_eq!(a.outcome[0].do_[0].kind, "log");
                assert_eq!(a.outcome[0].do_[0].params["message"], json!("thud"));
            }
            other => panic!("expected action, got {other:?}"),
        }
    }

    #[test]
    fn thin_action_inherits_from_in_unit_base() {
        // A thin L2 action with only roll_spec inherits the archetype's resolution.
        let archetype = json!({
            "action": {
                "id": "melee_attack",
                "kind": "contest",
                "resolution": {
                    "roller": "actor",
                    "tn_source": { "defense": "ac" },
                    "roll_spec": { "mechanic": "xwn_d20_attack", "attribute": "STR" }
                },
                "targeting": { "shape": "single", "range_band": "melee" }
            }
        });
        let thin = json!({
            "action": {
                "id": "bite",
                "base": "melee_attack",
                "resolution": { "roll_spec": { "skill": "punch" } }
            }
        });
        let report = compile(&[archetype, thin], &reg(), &Catalogs::default());
        assert!(!report.failed(), "{:?}", report.syntax_errors);
        let bite = report
            .records
            .iter()
            .find_map(|r| match r {
                ComponentRecord::Action(a) if a.id == "bite" => Some(a),
                _ => None,
            })
            .expect("bite compiled");
        let resolution = bite.resolution.as_ref().expect("inherited resolution");
        // tn_source + targeting inherited; roll_spec deep-merged (mechanic kept).
        assert_eq!(resolution.roll_spec.skill.as_deref(), Some("punch"));
        assert!(resolution.roll_spec.mechanic.is_some());
        assert!(bite.targeting.is_some());
    }

    #[test]
    fn thin_action_inherits_from_host_supplied_base() {
        // The archetype lives in catalogs.action_bases (e.g. the IR store), not the unit.
        let bases: BTreeMap<String, Value> = [(
            "melee_attack".to_string(),
            json!({
                "component_type": "action",
                "id": "melee_attack",
                "kind": "contest",
                "resolution": { "tn_source": { "defense": "ac" },
                                "roll_spec": { "attribute": "STR" } }
            }),
        )]
        .into_iter()
        .collect();
        let catalogs = Catalogs { action_bases: bases, ..Default::default() };
        let thin = json!({ "action": { "id": "claw", "base": "melee_attack" } });
        let report = compile(&[thin], &reg(), &catalogs);
        assert!(!report.failed(), "{:?}", report.syntax_errors);
        assert_eq!(report.records.len(), 1);
    }

    #[test]
    fn unresolved_base_is_a_syntax_error() {
        let thin = json!({ "action": { "id": "x", "base": "nope" } });
        let report = compile(&[thin], &reg(), &Catalogs::default());
        assert_eq!(report.syntax_errors.len(), 1);
        assert!(report.syntax_errors[0].message.contains("no such action"));
    }

    #[test]
    fn unknown_verb_fails_with_no_records() {
        let doc = json!({
            "trigger": { "id": "t", "on": "combat.hit", "do": [ { "teleport": "moon" } ] }
        });
        let report = compile(&[doc], &reg(), &Catalogs::default());
        assert!(report.failed());
        assert_eq!(report.unimplemented.len(), 1);
        assert!(report.records.is_empty()); // nothing committed
    }

    #[test]
    fn unknown_record_type_is_a_syntax_error() {
        let report = compile(
            &[json!({ "spell": { "id": "x" } })],
            &reg(),
            &Catalogs::default(),
        );
        assert_eq!(report.syntax_errors.len(), 1);
        assert!(report.syntax_errors[0]
            .message
            .contains("unknown record type"));
    }

    #[test]
    fn missing_required_field_is_a_syntax_error() {
        // A trait with no `category` (required) fails structural validation.
        let report = compile(
            &[json!({ "trait": { "id": "x", "name": "X" } })],
            &reg(),
            &Catalogs::default(),
        );
        assert_eq!(report.syntax_errors.len(), 1);
        assert!(report.records.is_empty());
    }

    #[test]
    fn all_diagnostics_collected_before_failing() {
        let docs = vec![
            json!({ "spell": { "id": "a" } }), // unknown type
            json!({ "trigger": { "id": "t", "on": "x", "do": [ { "zap": 1 } ] } }), // unknown verb
        ];
        let report = compile(&docs, &reg(), &Catalogs::default());
        assert_eq!(report.syntax_errors.len(), 1);
        assert_eq!(report.unimplemented.len(), 1);
        assert!(report.records.is_empty());
    }
}
