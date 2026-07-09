//! The form-2 effect verb registry (grammar spec §5).
//!
//! Authors write effects verb-first, e.g. `{ apply: bleeding, to: target, for: 3 }`.
//! The registry lowers each to a raw IR [`Effect`] `{kind, params}` by mapping the
//! verb key's value (`_self`) and the named-arg keys (`to`, `for`, …) onto the
//! effect's params. The grammar never changes when verbs are added — only the
//! registry grows. An unknown verb is an `UnimplementedPrimitive`, not a grammar
//! error (the author may be prototyping ahead of the engine).
//!
//! Optional args with no value are *omitted* from params (not defaulted to a
//! literal), so the runtime fills role-resolved defaults like the acting entity.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::compiler::diagnostics::Diagnostic;
use crate::ir::effect::Effect;

/// A value transform applied while binding an arg.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transform {
    /// Negate an integer (so `damage … amount: 2` becomes `delta: -2`).
    Negate,
}

/// How one form-2 argument binds to an effect param.
#[derive(Debug, Clone)]
pub struct VerbArg {
    /// The IR param key this arg populates.
    pub role: &'static str,
    pub required: bool,
    pub default: Option<Value>,
    pub transform: Option<Transform>,
}

impl VerbArg {
    fn new(role: &'static str, required: bool) -> Self {
        Self {
            role,
            required,
            default: None,
            transform: None,
        }
    }
    fn with_default(role: &'static str, default: Value) -> Self {
        Self {
            role,
            required: false,
            default: Some(default),
            transform: None,
        }
    }
    fn negated(role: &'static str) -> Self {
        Self {
            role,
            required: true,
            default: None,
            transform: Some(Transform::Negate),
        }
    }
}

/// A verb: the IR `kind` it lowers to, the binding for its `_self` value, and its
/// named args (keyed by the YAML arg name).
#[derive(Debug, Clone)]
pub struct VerbSpec {
    pub kind: &'static str,
    pub self_arg: Option<VerbArg>,
    pub args: BTreeMap<&'static str, VerbArg>,
}

/// The effect verb registry.
#[derive(Debug, Clone)]
pub struct VerbRegistry {
    verbs: BTreeMap<&'static str, VerbSpec>,
}

impl VerbRegistry {
    /// The built-in XWN verb set (grammar guide §4 / spec §5).
    pub fn builtin() -> Self {
        let mut verbs = BTreeMap::new();
        let mut add = |name: &'static str, spec: VerbSpec| {
            verbs.insert(name, spec);
        };

        add(
            "apply",
            spec(
                "apply_status",
                Some(VerbArg::new("status_id", true)),
                &[
                    ("to", VerbArg::new("entity_id", false)),
                    ("for", VerbArg::new("duration_ticks", false)),
                    ("source", VerbArg::new("source", false)),
                ],
            ),
        );
        add(
            "remove",
            spec(
                "remove_status",
                Some(VerbArg::new("status_id", true)),
                &[("from", VerbArg::new("entity_id", false))],
            ),
        );
        add(
            "damage",
            spec(
                "change_resource",
                Some(VerbArg::new("entity_id", true)),
                &[
                    ("amount", VerbArg::negated("delta")),
                    (
                        "resource",
                        VerbArg::with_default("resource", Value::from("hp")),
                    ),
                ],
            ),
        );
        add(
            "heal",
            spec(
                "change_resource",
                Some(VerbArg::new("entity_id", true)),
                &[
                    ("amount", VerbArg::new("delta", true)),
                    (
                        "resource",
                        VerbArg::with_default("resource", Value::from("hp")),
                    ),
                ],
            ),
        );
        add(
            "change",
            spec(
                "change_resource",
                Some(VerbArg::new("resource", true)),
                &[
                    ("by", VerbArg::new("delta", true)),
                    ("on", VerbArg::new("entity_id", false)),
                ],
            ),
        );
        add(
            "add_modifier",
            spec(
                "apply_modifier",
                Some(VerbArg::new("modifier", true)),
                &[
                    ("to", VerbArg::new("entity_id", false)),
                    ("source", VerbArg::new("source_id", false)),
                ],
            ),
        );
        add(
            "drop_modifier",
            spec(
                "remove_modifier",
                Some(VerbArg::new("source_id", true)),
                &[("from", VerbArg::new("entity_id", false))],
            ),
        );
        add(
            "emit",
            spec(
                "emit_event",
                Some(VerbArg::new("event_type", true)),
                &[("with", VerbArg::new("payload", false))],
            ),
        );
        add("log", spec("log", Some(VerbArg::new("message", true)), &[]));
        add(
            "emit_damage",
            spec(
                "emit_damage",
                Some(VerbArg::new("packet", true)),
                &[("to", VerbArg::new("entity_id", false))],
            ),
        );

        Self { verbs }
    }

    /// Whether a verb name is registered.
    pub fn knows(&self, verb: &str) -> bool {
        self.verbs.contains_key(verb)
    }

    /// Lower one form-2 effect map to a raw IR [`Effect`].
    pub fn lower(&self, form: &Map<String, Value>) -> Result<Effect, Diagnostic> {
        let verb_keys: Vec<&String> = form
            .keys()
            .filter(|k| self.verbs.contains_key(k.as_str()))
            .collect();
        let verb = match verb_keys.as_slice() {
            [] => {
                let keys: Vec<&str> = form.keys().map(String::as_str).collect();
                return Err(Diagnostic::unimplemented(format!(
                    "unknown effect verb (no registered verb among keys {keys:?})"
                )));
            }
            [v] => v.as_str(),
            _ => {
                return Err(Diagnostic::syntax(format!(
                    "ambiguous effect: more than one verb key {verb_keys:?}"
                )))
            }
        };
        let spec = &self.verbs[verb];
        let mut params = Map::new();

        if let Some(self_arg) = &spec.self_arg {
            let value = bind(&form[verb], self_arg.transform)?;
            params.insert(self_arg.role.to_string(), value);
        }

        // Bind provided named args (rejecting unknown ones).
        for (key, value) in form {
            if key == verb {
                continue;
            }
            match spec.args.get(key.as_str()) {
                Some(arg) => {
                    params.insert(arg.role.to_string(), bind(value, arg.transform)?);
                }
                None => {
                    return Err(Diagnostic::syntax(format!(
                        "verb '{verb}' has no argument '{key}'"
                    )))
                }
            }
        }

        // Defaults + required checks for absent args.
        for (key, arg) in &spec.args {
            if form.contains_key(*key) {
                continue;
            }
            if let Some(default) = &arg.default {
                params.insert(arg.role.to_string(), default.clone());
            } else if arg.required {
                return Err(Diagnostic::syntax(format!(
                    "verb '{verb}' requires argument '{key}'"
                )));
            }
        }

        Ok(Effect {
            kind: spec.kind.to_string(),
            params,
        })
    }
}

fn spec(
    kind: &'static str,
    self_arg: Option<VerbArg>,
    args: &[(&'static str, VerbArg)],
) -> VerbSpec {
    VerbSpec {
        kind,
        self_arg,
        args: args.iter().cloned().collect(),
    }
}

fn bind(value: &Value, transform: Option<Transform>) -> Result<Value, Diagnostic> {
    match transform {
        Some(Transform::Negate) => match value.as_i64() {
            Some(n) => Ok(Value::from(-n)),
            None => Err(Diagnostic::syntax(format!(
                "expected an integer to negate, got {value}"
            ))),
        },
        None => Ok(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn form(v: Value) -> Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn apply_lowers_with_named_args() {
        let reg = VerbRegistry::builtin();
        let e = reg
            .lower(&form(
                json!({ "apply": "bleeding", "to": "target", "for": 3 }),
            ))
            .unwrap();
        assert_eq!(e.kind, "apply_status");
        assert_eq!(e.params["status_id"], json!("bleeding"));
        assert_eq!(e.params["entity_id"], json!("target"));
        assert_eq!(e.params["duration_ticks"], json!(3));
    }

    #[test]
    fn damage_negates_amount_and_defaults_resource() {
        let reg = VerbRegistry::builtin();
        let e = reg
            .lower(&form(json!({ "damage": "self", "amount": 2 })))
            .unwrap();
        assert_eq!(e.kind, "change_resource");
        assert_eq!(e.params["entity_id"], json!("self"));
        assert_eq!(e.params["delta"], json!(-2)); // negated
        assert_eq!(e.params["resource"], json!("hp")); // default
    }

    #[test]
    fn optional_args_are_omitted_not_defaulted() {
        let reg = VerbRegistry::builtin();
        // No `to:` → entity_id omitted (runtime fills the acting entity).
        let e = reg.lower(&form(json!({ "apply": "poisoned" }))).unwrap();
        assert!(!e.params.contains_key("entity_id"));
        assert_eq!(e.params["status_id"], json!("poisoned"));
    }

    #[test]
    fn emit_damage_carries_packet_object() {
        let reg = VerbRegistry::builtin();
        let e = reg
            .lower(&form(
                json!({ "emit_damage": { "amount": 3, "types": ["fire"] }, "to": "target" }),
            ))
            .unwrap();
        assert_eq!(e.kind, "emit_damage");
        assert_eq!(e.params["packet"]["amount"], json!(3));
        assert_eq!(e.params["entity_id"], json!("target"));
    }

    #[test]
    fn unknown_verb_is_unimplemented() {
        let reg = VerbRegistry::builtin();
        let d = reg.lower(&form(json!({ "teleport": "moon" }))).unwrap_err();
        assert!(!d.is_syntax()); // UnimplementedPrimitive
    }

    #[test]
    fn unknown_arg_and_missing_required_are_syntax_errors() {
        let reg = VerbRegistry::builtin();
        // Unknown arg `where`.
        assert!(reg
            .lower(&form(json!({ "apply": "x", "where": "y" })))
            .unwrap_err()
            .is_syntax());
        // `damage` requires `amount`.
        assert!(reg
            .lower(&form(json!({ "damage": "self" })))
            .unwrap_err()
            .is_syntax());
    }
}
