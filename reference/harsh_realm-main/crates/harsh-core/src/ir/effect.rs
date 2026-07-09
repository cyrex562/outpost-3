//! Declarative effects and triggers — IR mirror of `harsh_realm.triggers`.
//!
//! An [`Effect`] is an open vocabulary of `{kind, params}` (matching the Python
//! `Effect` base model), with the known verbs documented in [`EffectKind`]. A
//! [`Trigger`] binds an event subscription to a condition and a list of effects.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A declarative effect: a verb (`kind`) plus an open map of parameters.
///
/// The set is intentionally open (a plain string) so packs can introduce new
/// verbs; [`EffectKind`] documents the verbs the core engine lowers to intents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Effect {
    pub kind: String,
    #[serde(default)]
    pub params: Map<String, Value>,
}

impl Effect {
    /// Convenience constructor.
    pub fn new(kind: impl Into<String>, params: Map<String, Value>) -> Self {
        Self {
            kind: kind.into(),
            params,
        }
    }
}

/// The effect verbs the core engine knows how to lower into intents.
///
/// This is documentation/validation surface; [`Effect::kind`] stays a string so
/// unknown (pack- or plugin-provided) verbs are still representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    ApplyModifier,
    RemoveModifier,
    ChangeResource,
    ApplyStatus,
    RemoveStatus,
    EmitEvent,
    RollDice,
    RunProcedure,
    Log,
}

impl EffectKind {
    /// The wire string for this verb (matches the Python `kind` literals).
    pub fn as_str(self) -> &'static str {
        match self {
            EffectKind::ApplyModifier => "apply_modifier",
            EffectKind::RemoveModifier => "remove_modifier",
            EffectKind::ChangeResource => "change_resource",
            EffectKind::ApplyStatus => "apply_status",
            EffectKind::RemoveStatus => "remove_status",
            EffectKind::EmitEvent => "emit_event",
            EffectKind::RollDice => "roll_dice",
            EffectKind::RunProcedure => "run_procedure",
            EffectKind::Log => "log",
        }
    }
}

fn default_when() -> String {
    "true".to_string()
}

/// A declarative trigger: an event subscription with a condition and effects.
///
/// Mirrors `triggers/schema.py`. `when` is the *string* form of a condition
/// expression (the R1 parser turns it into a [`super::condition::Expr`]); it
/// defaults to `"true"` (always fires).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Trigger {
    /// Unique within the owning record (trait/status/etc).
    pub id: String,
    /// Event type to subscribe to.
    pub on: String,
    /// Condition expression in string form. Default: `"true"`.
    #[serde(default = "default_when")]
    pub when: String,
    /// Effects to run when the trigger fires, in order.
    #[serde(default, rename = "do")]
    pub do_: Vec<Effect>,
    #[serde(default)]
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn trigger_defaults_when_and_do() {
        let value = json!({"id": "t1", "on": "combat.attack"});
        let trigger: Trigger = serde_json::from_value(value).unwrap();
        assert_eq!(trigger.when, "true");
        assert!(trigger.do_.is_empty());
        assert_eq!(trigger.description, "");
    }

    #[test]
    fn trigger_uses_do_key_on_wire() {
        let trigger = Trigger {
            id: "t1".into(),
            on: "combat.attack".into(),
            when: "true".into(),
            do_: vec![Effect::new("log", Map::new())],
            description: String::new(),
        };
        let v = serde_json::to_value(&trigger).unwrap();
        assert!(v.get("do").is_some(), "effects must serialize under `do`");
        assert!(v.get("do_").is_none());
    }

    #[test]
    fn effect_round_trips_with_params() {
        let value = json!({
            "kind": "change_resource",
            "params": {"resource": "hp", "delta": -3}
        });
        let effect: Effect = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(effect.kind, "change_resource");
        assert_eq!(effect.params["delta"], json!(-3));
        assert_eq!(serde_json::to_value(&effect).unwrap(), value);
    }

    #[test]
    fn effect_kind_wire_strings_match_python() {
        assert_eq!(EffectKind::ApplyStatus.as_str(), "apply_status");
        assert_eq!(
            serde_json::to_value(EffectKind::RollDice).unwrap(),
            json!("roll_dice")
        );
    }
}
