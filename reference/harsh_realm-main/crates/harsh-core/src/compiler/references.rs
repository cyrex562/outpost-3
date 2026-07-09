//! Cross-reference resolution (grammar spec §7.3).
//!
//! After all records compile, references are checked against the compiled-record
//! ids plus host-supplied [`Catalogs`]. Per the B1 decision:
//! - a missing **authored-content** reference (a status/action/… that should be a
//!   record) → `SyntaxError` (a broken internal reference);
//! - a missing **engine-primitive** reference (an event / resource) →
//!   `UnimplementedPrimitive` (the author may be prototyping ahead).
//!
//! Each catalog category is **opt-in**: if the host leaves a category empty, that
//! reference class is not checked (avoids false positives when a catalog wasn't
//! supplied). References to records compiled in this unit always resolve.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::compiler::diagnostics::Diagnostic;
use crate::compiler::validate::triggers_of;
use crate::dsl::parse_condition;
use crate::ir::condition::{Expr, PathRoot};
use crate::ir::effect::Effect;
use crate::ir::ComponentRecord;

/// Registered vocabularies the host supplies from the loaded pack set. Empty
/// categories disable checking for that reference class.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Catalogs {
    #[serde(default)]
    pub events: BTreeSet<String>,
    #[serde(default)]
    pub resources: BTreeSet<String>,
    #[serde(default)]
    pub statuses: BTreeSet<String>,
    #[serde(default)]
    pub actions: BTreeSet<String>,
    /// Per-event payload field names (R5-W4). When an event appears here, a
    /// trigger's `when:` may only reference `event.<field>`s in this set.
    #[serde(default)]
    pub event_fields: BTreeMap<String, BTreeSet<String>>,
    /// Already-compiled action bodies (id → record) the host supplies as
    /// archetypes for `base:` merge — e.g. actions already in the world IR store.
    #[serde(default)]
    pub action_bases: BTreeMap<String, serde_json::Value>,
}

/// Resolve references across a compiled record set, returning diagnostics.
pub fn resolve_references(records: &[ComponentRecord], catalogs: &Catalogs) -> Vec<Diagnostic> {
    // Records compiled in this unit always resolve, unioned with the catalogs.
    let compiled_statuses = ids_of(records, |r| matches!(r, ComponentRecord::StatusEffect(_)));
    let compiled_actions = ids_of(records, |r| matches!(r, ComponentRecord::Action(_)));
    let known_statuses = union(&catalogs.statuses, &compiled_statuses);
    let known_actions = union(&catalogs.actions, &compiled_actions);

    let mut diags = Vec::new();
    for record in records {
        check_record(
            record,
            catalogs,
            &known_statuses,
            &known_actions,
            &mut diags,
        );
    }
    diags
}

fn check_record(
    record: &ComponentRecord,
    catalogs: &Catalogs,
    known_statuses: &BTreeSet<String>,
    known_actions: &BTreeSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    // Event references: every trigger's `on`.
    if !catalogs.events.is_empty() {
        for t in triggers_of(record) {
            if !catalogs.events.contains(&t.on) {
                diags.push(Diagnostic::unimplemented(format!(
                    "trigger on event {:?} — no such event is registered",
                    t.on
                )));
            }
        }
    }

    // Effect-borne references.
    for effect in effects_of(record) {
        match effect.kind.as_str() {
            "emit_event" if !catalogs.events.is_empty() => {
                if let Some(ev) = effect.params.get("event_type").and_then(|v| v.as_str()) {
                    if !catalogs.events.contains(ev) {
                        diags.push(Diagnostic::unimplemented(format!(
                            "emit_event {ev:?} — no such event is registered"
                        )));
                    }
                }
            }
            "change_resource" if !catalogs.resources.is_empty() => {
                if let Some(res) = effect.params.get("resource").and_then(|v| v.as_str()) {
                    if !catalogs.resources.contains(res) {
                        diags.push(Diagnostic::unimplemented(format!(
                            "resource {res:?} — no such resource is registered"
                        )));
                    }
                }
            }
            "apply_status" | "remove_status" if !catalogs.statuses.is_empty() => {
                if let Some(s) = effect.params.get("status_id").and_then(|v| v.as_str()) {
                    if !known_statuses.contains(s) {
                        diags.push(Diagnostic::syntax(format!(
                            "status {s:?} — no such status_effect record"
                        )));
                    }
                }
            }
            _ => {}
        }
    }

    // Entity → action references.
    if !catalogs.actions.is_empty() {
        let action_refs: Vec<&String> = match record {
            ComponentRecord::Creature(c) => c.actions.iter().collect(),
            ComponentRecord::Skill(s) => s
                .grants
                .iter()
                .chain(s.governs.iter().map(|g| &g.action))
                .collect(),
            _ => Vec::new(),
        };
        for a in action_refs {
            if !known_actions.contains(a) {
                diags.push(Diagnostic::syntax(format!(
                    "action {a:?} — no such action record"
                )));
            }
        }
    }

    // Event-payload field references (R5-W4): a trigger's `when:` may only read
    // `event.<field>`s the triggering event actually carries. Opt-in per event.
    for trigger in triggers_of(record) {
        let Some(allowed) = catalogs.event_fields.get(&trigger.on) else {
            continue;
        };
        // A malformed `when:` is already reported by validate.rs; skip it here.
        let Ok(expr) = parse_condition(&trigger.when) else {
            continue;
        };
        let mut referenced = BTreeSet::new();
        collect_event_fields(&expr, &mut referenced);
        for field in referenced {
            if !allowed.contains(&field) {
                diags.push(Diagnostic::syntax(format!(
                    "condition reads event.{field}, but event {:?} carries no such field",
                    trigger.on
                )));
            }
        }
    }
}

/// Collect the first segment of every `event.<...>` path in a condition.
fn collect_event_fields(expr: &Expr, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Path {
            root: PathRoot::Event,
            parts,
        } => {
            if let Some(first) = parts.first() {
                out.insert(first.clone());
            }
        }
        Expr::Path { .. } | Expr::Literal { .. } => {}
        Expr::BinaryOp { left, right, .. } => {
            collect_event_fields(left, out);
            collect_event_fields(right, out);
        }
        Expr::UnaryOp { operand, .. } => collect_event_fields(operand, out),
        Expr::FuncCall { args, .. } => {
            for arg in args {
                collect_event_fields(arg, out);
            }
        }
        Expr::ListLiteral { items } => {
            for item in items {
                collect_event_fields(item, out);
            }
        }
    }
}

/// Effects a record carries (trigger `do:` lists + action outcome `do:` lists).
fn effects_of(record: &ComponentRecord) -> Vec<&Effect> {
    let mut out: Vec<&Effect> = triggers_of(record)
        .iter()
        .flat_map(|t| t.do_.iter())
        .collect();
    if let ComponentRecord::Action(a) = record {
        for branch in &a.outcome {
            out.extend(branch.do_.iter());
        }
    }
    out
}

fn ids_of(
    records: &[ComponentRecord],
    pred: impl Fn(&ComponentRecord) -> bool,
) -> BTreeSet<String> {
    records
        .iter()
        .filter(|r| pred(r))
        .filter_map(record_id)
        .collect()
}

fn record_id(record: &ComponentRecord) -> Option<String> {
    match record {
        ComponentRecord::StatusEffect(s) => Some(s.id.clone()),
        ComponentRecord::Action(a) => Some(a.id.clone()),
        _ => None,
    }
}

fn union(a: &BTreeSet<String>, b: &BTreeSet<String>) -> BTreeSet<String> {
    a.union(b).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rec(v: serde_json::Value) -> ComponentRecord {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn unregistered_event_is_unimplemented() {
        let r = rec(json!({"component_type": "trigger", "id": "t", "on": "world.tick", "do": []}));
        let catalogs = Catalogs {
            events: ["combat.hit".to_string()].into_iter().collect(),
            ..Default::default()
        };
        let diags = resolve_references(&[r], &catalogs);
        assert_eq!(diags.len(), 1);
        assert!(!diags[0].is_syntax()); // UnimplementedPrimitive
    }

    fn attack_event_fields() -> Catalogs {
        let mut event_fields = BTreeMap::new();
        event_fields.insert(
            "combat.attack".to_string(),
            ["damage".to_string(), "hit".to_string()]
                .into_iter()
                .collect(),
        );
        Catalogs {
            event_fields,
            ..Default::default()
        }
    }

    #[test]
    fn unknown_event_field_is_a_syntax_error() {
        let r = rec(json!({
            "component_type": "trigger", "id": "t", "on": "combat.attack",
            "when": "event.damage > 0 and event.bogus", "do": []
        }));
        let diags = resolve_references(&[r], &attack_event_fields());
        assert_eq!(diags.len(), 1);
        assert!(diags[0].is_syntax());
        assert!(diags[0].message.contains("bogus"));
    }

    #[test]
    fn known_event_fields_pass() {
        let r = rec(json!({
            "component_type": "trigger", "id": "t", "on": "combat.attack",
            "when": "event.hit and event.damage > 2", "do": []
        }));
        assert!(resolve_references(&[r], &attack_event_fields()).is_empty());
    }

    #[test]
    fn event_not_in_catalog_skips_field_check() {
        // world.tick has no event_fields entry → its event.* reads aren't checked.
        let r = rec(json!({
            "component_type": "trigger", "id": "t", "on": "world.tick",
            "when": "event.anything_at_all", "do": []
        }));
        assert!(resolve_references(&[r], &attack_event_fields()).is_empty());
    }

    #[test]
    fn missing_status_reference_is_syntax_error() {
        // An effect applies 'bleeding', which is neither compiled nor in the catalog.
        let r = rec(json!({
            "component_type": "trigger", "id": "t", "on": "combat.hit",
            "do": [ { "kind": "apply_status", "params": { "status_id": "bleeding" } } ]
        }));
        let catalogs = Catalogs {
            statuses: ["poisoned".to_string()].into_iter().collect(),
            ..Default::default()
        };
        let diags = resolve_references(&[r], &catalogs);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].is_syntax());
    }

    #[test]
    fn compiled_status_resolves_a_reference() {
        let status =
            rec(json!({"component_type": "status_effect", "id": "bleeding", "name": "Bleeding"}));
        let trigger = rec(json!({
            "component_type": "trigger", "id": "t", "on": "combat.hit",
            "do": [ { "kind": "apply_status", "params": { "status_id": "bleeding" } } ]
        }));
        // statuses catalog non-empty (so the class is checked); bleeding resolves
        // via the compiled status_effect.
        let catalogs = Catalogs {
            statuses: ["other".to_string()].into_iter().collect(),
            ..Default::default()
        };
        assert!(resolve_references(&[status, trigger], &catalogs).is_empty());
    }

    #[test]
    fn empty_catalog_disables_the_check() {
        let r =
            rec(json!({"component_type": "trigger", "id": "t", "on": "anything.at.all", "do": []}));
        // No events catalog → event references not checked.
        assert!(resolve_references(&[r], &Catalogs::default()).is_empty());
    }

    #[test]
    fn unknown_action_reference_is_syntax_error() {
        let c = rec(json!({
            "component_type": "creature", "id": "hound", "name": "Hound", "hd": 1,
            "ac": 12, "attack_bonus": 1, "damage": "1d6", "actions": ["bite"]
        }));
        let catalogs = Catalogs {
            actions: ["claw".to_string()].into_iter().collect(),
            ..Default::default()
        };
        let diags = resolve_references(&[c], &catalogs);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].is_syntax());
    }
}
