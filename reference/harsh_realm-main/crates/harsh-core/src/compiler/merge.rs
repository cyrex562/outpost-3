//! Action `base:` archetype merge (R5-A3).
//!
//! A concrete action may set `base: <archetype>` and supply only the fields it
//! overrides; the compiler fills the rest from the archetype. Resolution is a
//! recursive deep merge (objects merge key-by-key; scalars/arrays replace), so a
//! thin `roll_spec: { skill: punch }` inherits the archetype's `tn_source` and
//! keeps the archetype's `roll_spec.mechanic`.
//!
//! Bases are looked up in an index built from the in-unit action documents plus
//! host-supplied `action_bases` (e.g. already-compiled archetypes from the IR
//! store). Missing bases and cycles are diagnostics, not panics.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::compiler::diagnostics::Diagnostic;

/// Recursively merge `over` onto `base`: object keys merge, everything else is
/// replaced by `over`.
pub fn deep_merge(base: &Value, over: &Value) -> Value {
    match (base, over) {
        (Value::Object(base_map), Value::Object(over_map)) => {
            let mut merged = base_map.clone();
            for (key, over_value) in over_map {
                let merged_value = match merged.get(key) {
                    Some(base_value) => deep_merge(base_value, over_value),
                    None => over_value.clone(),
                };
                merged.insert(key.clone(), merged_value);
            }
            Value::Object(merged)
        }
        _ => over.clone(),
    }
}

/// Resolve an action body's `base:` chain into a fully-merged body.
///
/// `index` maps action id → its (pre-merge) body. Returns the merged body, or a
/// diagnostic for a missing base or a cycle.
pub fn resolve_action_base(
    body: &Map<String, Value>,
    index: &BTreeMap<String, Value>,
    seen: &mut Vec<String>,
) -> Result<Map<String, Value>, Diagnostic> {
    let base_id = match body.get("base").and_then(Value::as_str) {
        Some(id) => id.to_string(),
        None => return Ok(body.clone()),
    };

    if seen.contains(&base_id) {
        return Err(Diagnostic::syntax(format!(
            "action base cycle through {base_id:?}"
        )));
    }
    let base_value = index.get(&base_id).ok_or_else(|| {
        Diagnostic::syntax(format!("action base {base_id:?} — no such action"))
    })?;
    let base_body = base_value.as_object().ok_or_else(|| {
        Diagnostic::syntax(format!("action base {base_id:?} is not a mapping"))
    })?;

    seen.push(base_id);
    let resolved_base = resolve_action_base(base_body, index, seen)?;
    seen.pop();

    // Concrete fields override the (recursively-resolved) base.
    let merged = deep_merge(&Value::Object(resolved_base), &Value::Object(body.clone()));
    Ok(merged.as_object().expect("merge of objects is an object").clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deep_merge_overrides_scalars_and_merges_objects() {
        let base = json!({"a": 1, "nested": {"x": 1, "y": 2}, "arr": [1, 2]});
        let over = json!({"a": 9, "nested": {"y": 20, "z": 3}, "arr": [9]});
        let merged = deep_merge(&base, &over);
        assert_eq!(
            merged,
            json!({"a": 9, "nested": {"x": 1, "y": 20, "z": 3}, "arr": [9]})
        );
    }

    fn index(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn resolve_inherits_tn_source_and_keeps_base_mechanic() {
        let idx = index(&[(
            "melee_attack",
            json!({
                "id": "melee_attack",
                "resolution": {
                    "roller": "actor",
                    "tn_source": {"defense": "ac"},
                    "roll_spec": {"mechanic": "xwn_d20_attack", "attribute": "STR"}
                }
            }),
        )]);
        let thin = json!({"id": "bite", "base": "melee_attack",
                          "resolution": {"roll_spec": {"skill": "punch"}}});
        let merged =
            resolve_action_base(thin.as_object().unwrap(), &idx, &mut Vec::new()).unwrap();
        let res = &merged["resolution"];
        assert_eq!(merged["id"], json!("bite"));
        assert_eq!(res["tn_source"], json!({"defense": "ac"})); // inherited
        assert_eq!(res["roller"], json!("actor")); // inherited
        // roll_spec deep-merges: base mechanic kept, concrete skill added.
        assert_eq!(res["roll_spec"]["mechanic"], json!("xwn_d20_attack"));
        assert_eq!(res["roll_spec"]["skill"], json!("punch"));
    }

    #[test]
    fn missing_base_is_a_diagnostic() {
        let thin = json!({"id": "x", "base": "nope"});
        let err = resolve_action_base(thin.as_object().unwrap(), &index(&[]), &mut Vec::new())
            .unwrap_err();
        assert!(err.is_syntax());
        assert!(err.message.contains("no such action"));
    }

    #[test]
    fn cycle_is_a_diagnostic() {
        let idx = index(&[
            ("a", json!({"id": "a", "base": "b"})),
            ("b", json!({"id": "b", "base": "a"})),
        ]);
        let err = resolve_action_base(
            idx["a"].as_object().unwrap(),
            &idx,
            &mut Vec::new(),
        )
        .unwrap_err();
        assert!(err.message.contains("cycle"));
    }

    #[test]
    fn two_level_chain_resolves() {
        let idx = index(&[
            ("arch", json!({"id": "arch", "kind": "contest", "x": 1})),
            ("mid", json!({"id": "mid", "base": "arch", "y": 2})),
        ]);
        let leaf = json!({"id": "leaf", "base": "mid", "z": 3});
        let merged =
            resolve_action_base(leaf.as_object().unwrap(), &idx, &mut Vec::new()).unwrap();
        assert_eq!(merged["kind"], json!("contest")); // from arch
        assert_eq!(merged["x"], json!(1));
        assert_eq!(merged["y"], json!(2)); // from mid
        assert_eq!(merged["z"], json!(3)); // own
        assert_eq!(merged["id"], json!("leaf"));
    }
}
