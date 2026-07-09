//! The intent protocol — the engine's only output channel for state change.
//!
//! `harsh-core` performs no I/O and mutates no durable state. Evaluating a
//! trigger produces a list of [`Intent`]s: typed, serializable descriptions of
//! *what should happen*. A host applies them:
//!
//! - **Now:** the Python persistence layer turns intents into SQLite writes.
//! - **Later:** a Rust persistence layer can apply them directly (rusqlite),
//!   without any change to the engine — the intent is the stable contract.
//!
//! Each [`Effect`](crate::ir::Effect) verb the engine understands lowers to one
//! intent variant (R1-05).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A typed, serializable description of a single state change to apply.
///
/// Internally tagged on `intent` so the wire form is self-describing, e.g.
/// `{"intent": "change_resource", "entity_id": "pc", "resource": "hp", "delta": -3}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "intent", rename_all = "snake_case")]
pub enum Intent {
    /// Apply a modifier to an entity (from `apply_modifier`).
    ApplyModifier {
        entity_id: String,
        modifier: crate::ir::Modifier,
        source_id: String,
    },
    /// Remove a previously-applied modifier (from `remove_modifier`).
    RemoveModifier {
        entity_id: String,
        source_id: String,
    },
    /// Change a numeric resource by a delta (from `change_resource`).
    ChangeResource {
        entity_id: String,
        resource: String,
        delta: i64,
    },
    /// Apply a status effect to an entity (from `apply_status`).
    ApplyStatus {
        entity_id: String,
        status_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ticks: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    /// Remove a status effect from an entity (from `remove_status`).
    RemoveStatus {
        entity_id: String,
        status_id: String,
    },
    /// Emit a new event onto the bus (from `emit_event`).
    EmitEvent {
        event_type: String,
        #[serde(default)]
        payload: Map<String, Value>,
    },
    /// Append a log line (from `log`).
    Log { message: String },
    /// Deal a damage packet to an entity (from `emit_damage`). The host runs the
    /// damage pipeline ([`crate::resolution::apply_damage`]) against the target's
    /// materialized pools/mitigations and applies the resulting pool deltas.
    EmitDamage {
        entity_id: String,
        packet: crate::resolution::damage::DamagePacket,
    },
    /// Spawn an entity from a template at a grid location (from `spawn_entity`).
    SpawnEntity {
        template_id: String,
        at_q: i64,
        at_r: i64,
    },
    /// Move an entity to a grid location (from `reposition`).
    Reposition {
        entity_id: String,
        to_q: i64,
        to_r: i64,
    },
}

impl Intent {
    /// The `intent` discriminator string for this variant.
    pub fn tag(&self) -> &'static str {
        match self {
            Intent::ApplyModifier { .. } => "apply_modifier",
            Intent::RemoveModifier { .. } => "remove_modifier",
            Intent::ChangeResource { .. } => "change_resource",
            Intent::ApplyStatus { .. } => "apply_status",
            Intent::RemoveStatus { .. } => "remove_status",
            Intent::EmitEvent { .. } => "emit_event",
            Intent::Log { .. } => "log",
            Intent::EmitDamage { .. } => "emit_damage",
            Intent::SpawnEntity { .. } => "spawn_entity",
            Intent::Reposition { .. } => "reposition",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn change_resource_intent_wire_shape() {
        let intent = Intent::ChangeResource {
            entity_id: "pc".into(),
            resource: "hp".into(),
            delta: -3,
        };
        assert_eq!(
            serde_json::to_value(&intent).unwrap(),
            json!({
                "intent": "change_resource",
                "entity_id": "pc",
                "resource": "hp",
                "delta": -3
            })
        );
        assert_eq!(intent.tag(), "change_resource");
    }

    #[test]
    fn apply_status_skips_optional_fields() {
        let intent = Intent::ApplyStatus {
            entity_id: "pc".into(),
            status_id: "poisoned".into(),
            duration_ticks: None,
            source: None,
        };
        let v = serde_json::to_value(&intent).unwrap();
        assert!(v.get("duration_ticks").is_none());
        assert!(v.get("source").is_none());
        // Round-trips.
        let back: Intent = serde_json::from_value(v).unwrap();
        assert_eq!(back, intent);
    }

    #[test]
    fn log_intent_round_trips() {
        let value = json!({"intent": "log", "message": "hello"});
        let intent: Intent = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&intent).unwrap(), value);
    }

    #[test]
    fn emit_damage_intent_round_trips() {
        let intent = Intent::EmitDamage {
            entity_id: "target".into(),
            packet: crate::resolution::damage::DamagePacket {
                amount: 6,
                types: vec!["physical".into()],
                tier: Default::default(),
            },
        };
        assert_eq!(intent.tag(), "emit_damage");
        let v = serde_json::to_value(&intent).unwrap();
        assert_eq!(v["intent"], json!("emit_damage"));
        assert_eq!(v["packet"]["amount"], json!(6));
        let back: Intent = serde_json::from_value(v).unwrap();
        assert_eq!(back, intent);
    }

    #[test]
    fn reposition_and_spawn_tags() {
        assert_eq!(
            Intent::Reposition {
                entity_id: "pc".into(),
                to_q: 1,
                to_r: 2
            }
            .tag(),
            "reposition"
        );
        assert_eq!(
            Intent::SpawnEntity {
                template_id: "scrip_hound".into(),
                at_q: 0,
                at_r: 0
            }
            .tag(),
            "spawn_entity"
        );
    }
}
