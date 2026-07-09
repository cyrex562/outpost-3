//! [`IntentApplier`] — apply the [`Intent`]s produced by a trigger to world state.
//!
//! This is the single place IR-driven mutations touch state. Each intent maps to
//! an existing service:
//!
//! - `ApplyStatus` / `RemoveStatus` → [`StatusEffectService`]
//! - `ApplyModifier` / `RemoveModifier` → [`TransientModifierRepository`]
//! - `ChangeResource` / `EmitDamage` → returned as `resource_deltas` (a
//!   combatant's hp is transient, owned by the combat state, not the DB; the
//!   orchestrator applies these)
//! - `EmitEvent` → a follow-on [`GameEvent`]; `Log` → a `gm.narrate` event
//! - `SpawnEntity` / `Reposition` → unsupported in the combat context (collected
//!   as an error; implemented in a later phase)
//!
//! Application is best-effort: a failing intent is recorded in `errors` and never
//! aborts the batch or panics (AGENTS.md "no silent failures").

use serde_json::{Map, Value};

use crate::db::WorldDatabase;
use crate::events::GameEvent;
use crate::intent::Intent;
use crate::modifiers::transient::TransientModifierRepository;
use crate::resolution::damage::DamagePacket;
use crate::status_effects::service::{ContentLookup, StatusEffectService, WorldClock};

/// The result of applying a batch of intents.
#[derive(Debug, Default)]
pub struct ApplyOutcome {
    /// Follow-on events to publish/cascade (from `emit_event` and `log`).
    pub events: Vec<GameEvent>,
    /// Per-intent errors; the batch continues past each.
    pub errors: Vec<String>,
    /// Pending `(entity_id, resource, delta)` changes the orchestrator applies to
    /// transient combat state (hp etc.).
    pub resource_deltas: Vec<(String, String, i64)>,
    /// Pending `(entity_id, packet)` damage the orchestrator routes through the
    /// IR damage pipeline against the target's live pools/mitigations.
    pub damage_packets: Vec<(String, DamagePacket)>,
}

/// Applies intents via the existing durable services.
pub struct IntentApplier<'a, C: ContentLookup, K: WorldClock> {
    db: &'a WorldDatabase,
    status: &'a StatusEffectService<'a, C, K>,
    tick: i64,
}

impl<'a, C: ContentLookup, K: WorldClock> IntentApplier<'a, C, K> {
    /// Construct an applier over the world db, status service, and current tick.
    pub fn new(db: &'a WorldDatabase, status: &'a StatusEffectService<'a, C, K>, tick: i64) -> Self {
        Self { db, status, tick }
    }

    /// Apply every intent best-effort, collecting events, deltas, and errors.
    pub fn apply(&self, intents: &[Intent]) -> ApplyOutcome {
        let mut out = ApplyOutcome::default();
        for intent in intents {
            self.apply_one(intent, &mut out);
        }
        out
    }

    fn apply_one(&self, intent: &Intent, out: &mut ApplyOutcome) {
        match intent {
            Intent::ChangeResource {
                entity_id,
                resource,
                delta,
            } => {
                out.resource_deltas
                    .push((entity_id.clone(), resource.clone(), *delta));
            }
            Intent::ApplyStatus {
                entity_id,
                status_id,
                duration_ticks,
                source,
            } => {
                let duration = duration_ticks.map(|u| u as i32);
                if let Err(e) =
                    self.status
                        .apply(entity_id, status_id, duration, source.as_deref(), None)
                {
                    out.errors
                        .push(format!("apply_status {status_id} on {entity_id}: {e}"));
                }
            }
            Intent::RemoveStatus {
                entity_id,
                status_id,
            } => {
                if let Err(e) = self.status.remove(entity_id, status_id) {
                    out.errors
                        .push(format!("remove_status {status_id} on {entity_id}: {e}"));
                }
            }
            Intent::ApplyModifier {
                entity_id,
                modifier,
                source_id,
            } => {
                let repo = TransientModifierRepository::new(self.db);
                let source = if source_id.is_empty() {
                    None
                } else {
                    Some(source_id.as_str())
                };
                if let Err(e) = repo.add(entity_id, modifier, self.tick, None, source) {
                    out.errors
                        .push(format!("apply_modifier on {entity_id}: {e}"));
                }
            }
            Intent::RemoveModifier {
                entity_id,
                source_id,
            } => {
                let repo = TransientModifierRepository::new(self.db);
                if let Err(e) = repo.remove_by_source(entity_id, source_id) {
                    out.errors
                        .push(format!("remove_modifier {source_id} on {entity_id}: {e}"));
                }
            }
            Intent::EmitEvent {
                event_type,
                payload,
            } => {
                out.events.push(
                    GameEvent::new(self.tick, event_type.clone(), payload.clone())
                        .with_source("ir"),
                );
            }
            Intent::Log { message } => {
                let mut data = Map::new();
                data.insert("message".into(), Value::String(message.clone()));
                out.events
                    .push(GameEvent::new(self.tick, "gm.narrate", data).with_source("ir"));
            }
            Intent::EmitDamage { entity_id, packet } => {
                // Routed through the IR damage pipeline by the orchestrator, which
                // has the target's live pools/mitigations (this applier is
                // state-free re: combatants).
                out.damage_packets
                    .push((entity_id.clone(), packet.clone()));
            }
            Intent::SpawnEntity { .. } | Intent::Reposition { .. } => {
                out.errors.push(format!(
                    "intent '{}' is unsupported in the combat demo context (deferred)",
                    intent.tag()
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use crate::ir::ComponentRecord;
    use crate::runtime_content::store::RuntimeContentStore;
    use crate::status_effects::repository::StatusEffectRepository;
    use serde_json::json;

    struct FixedClock(i32);
    impl WorldClock for FixedClock {
        fn tick(&self) -> i32 {
            self.0
        }
    }

    fn seeded_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn store() -> RuntimeContentStore {
        let records: Vec<ComponentRecord> = vec![json!({
            "component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
            "default_duration_ticks": 3, "provides_tags": ["poisoned"], "stacking": "replace"
        })]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        RuntimeContentStore::from_records(records)
    }

    #[test]
    fn change_resource_routes_to_resource_deltas() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let applier = IntentApplier::new(&db, &svc, 0);
        let out = applier.apply(&[Intent::ChangeResource {
            entity_id: "npc".into(),
            resource: "hp".into(),
            delta: -2,
        }]);
        assert_eq!(out.resource_deltas, vec![("npc".into(), "hp".into(), -2)]);
        assert!(out.errors.is_empty());
    }

    #[test]
    fn apply_status_intent_persists_effect() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(1)));
        let applier = IntentApplier::new(&db, &svc, 1);
        let out = applier.apply(&[Intent::ApplyStatus {
            entity_id: "npc".into(),
            status_id: "poisoned".into(),
            duration_ticks: Some(3),
            source: Some("envenom".into()),
        }]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        let active = svc.list_for_entity("npc").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].effect_id, "poisoned");
    }

    #[test]
    fn apply_and_remove_modifier_by_source() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let applier = IntentApplier::new(&db, &svc, 0);
        let modifier: crate::ir::Modifier = serde_json::from_value(json!({
            "target": "attribute.str", "value": -1
        }))
        .unwrap();
        applier.apply(&[Intent::ApplyModifier {
            entity_id: "npc".into(),
            modifier,
            source_id: "poisoned".into(),
        }]);
        let repo = TransientModifierRepository::new(&db);
        assert_eq!(repo.list_for_entity("npc", 0).unwrap().len(), 1);
        let out = applier.apply(&[Intent::RemoveModifier {
            entity_id: "npc".into(),
            source_id: "poisoned".into(),
        }]);
        assert!(out.errors.is_empty());
        assert_eq!(repo.list_for_entity("npc", 0).unwrap().len(), 0);
    }

    #[test]
    fn emit_event_and_log_become_events() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let applier = IntentApplier::new(&db, &svc, 5);
        let out = applier.apply(&[
            Intent::Log {
                message: "poison bites".into(),
            },
            Intent::EmitEvent {
                event_type: "combat.poison_tick".into(),
                payload: Map::new(),
            },
        ]);
        assert_eq!(out.events.len(), 2);
        assert_eq!(out.events[0].event_type, "gm.narrate");
        assert_eq!(out.events[0].data["message"], json!("poison bites"));
        assert_eq!(out.events[1].event_type, "combat.poison_tick");
    }

    #[test]
    fn unsupported_intents_collect_errors_not_panics() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let applier = IntentApplier::new(&db, &svc, 0);
        let out = applier.apply(&[Intent::SpawnEntity {
            template_id: "x".into(),
            at_q: 0,
            at_r: 0,
        }]);
        assert_eq!(out.errors.len(), 1);
        assert!(out.errors[0].contains("spawn_entity"));
    }

    #[test]
    fn one_failing_intent_does_not_abort_batch() {
        let db = seeded_db();
        let store = store();
        let svc =
            StatusEffectService::new(StatusEffectRepository::new(&db), &store, Some(FixedClock(0)));
        let applier = IntentApplier::new(&db, &svc, 0);
        let out = applier.apply(&[
            // Unknown status → error.
            Intent::ApplyStatus {
                entity_id: "npc".into(),
                status_id: "ghost".into(),
                duration_ticks: None,
                source: None,
            },
            // Still processed.
            Intent::ChangeResource {
                entity_id: "npc".into(),
                resource: "hp".into(),
                delta: -1,
            },
        ]);
        assert_eq!(out.errors.len(), 1);
        assert_eq!(out.resource_deltas.len(), 1);
    }
}
