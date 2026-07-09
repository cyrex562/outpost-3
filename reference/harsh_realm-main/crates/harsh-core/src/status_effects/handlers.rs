//! Domain event handlers for status effect lifecycle requests.
//!
//! Ported from `src/harsh_realm/status_effects/handlers.py`.

use crate::domain_events::DomainEventDispatcher;
use crate::events::GameEvent;
use crate::payloads::transport::{
    ActiveStatusEffectPayload, StatusAppliedNotice, StatusApplyRequested, StatusExpiredNotice,
    StatusRemoveRequested, StatusRemovedNotice,
};
use crate::runtime::JsonObject;

use super::models::ActiveStatusEffect;
use super::service::{ContentLookup, StatusEffectService, WorldClock};

const SOURCE: &str = "status_effects";

/// Serialize a payload struct to a flat event-data object.
fn event_data<T: serde::Serialize>(payload: &T) -> JsonObject {
    match serde_json::to_value(payload) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

fn to_payload(effect: &ActiveStatusEffect) -> ActiveStatusEffectPayload {
    ActiveStatusEffectPayload {
        id: effect.id,
        entity_id: effect.entity_id.clone(),
        effect_id: effect.effect_id.clone(),
        applied_at_tick: effect.applied_at_tick as i64,
        expires_at_tick: effect.expires_at_tick.map(|v| v as i64),
        source: effect.source.clone(),
        data: effect.data.clone(),
    }
}

/// Handle the `status.apply_requested` event, returning `status.applied`.
/// Deserialize an event's payload, logging (not silently swallowing) on failure.
/// Mirrors `gm::event_handlers::parse_payload` (HR-783/HR-789) so a payload
/// mismatch leaves a diagnostic trace instead of an invisible no-op.
fn parse_payload<T: serde::de::DeserializeOwned>(event: &GameEvent) -> Option<T> {
    match serde_json::from_value::<T>(serde_json::Value::Object(event.data.clone())) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!(
                "status_effect_handler: '{}' failed to deserialize {}: {e}",
                event.event_type,
                std::any::type_name::<T>()
            );
            None
        }
    }
}

pub fn handle_apply_requested<C: ContentLookup, K: WorldClock>(
    service: &StatusEffectService<'_, C, K>,
    event: &GameEvent,
) -> Vec<GameEvent> {
    let Some(payload) = parse_payload::<StatusApplyRequested>(event) else {
        return vec![];
    };
    let active = match service.apply(
        &payload.entity_id,
        &payload.effect_id,
        payload.duration_ticks,
        payload.source.as_deref(),
        Some(payload.data.clone()),
    ) {
        Ok(active) => active,
        Err(_) => return vec![],
    };
    let notice = StatusAppliedNotice {
        active_effect: to_payload(&active),
    };
    vec![GameEvent::new(event.tick, "status.applied", event_data(&notice)).with_source(SOURCE)]
}

/// Handle the `status.remove_requested` event, returning `status.removed`.
pub fn handle_remove_requested<C: ContentLookup, K: WorldClock>(
    service: &StatusEffectService<'_, C, K>,
    event: &GameEvent,
) -> Vec<GameEvent> {
    let Some(payload) = parse_payload::<StatusRemoveRequested>(event) else {
        return vec![];
    };
    let removed_count = match service.remove(&payload.entity_id, &payload.effect_id) {
        Ok(count) => count as i32,
        Err(_) => return vec![],
    };
    let notice = StatusRemovedNotice {
        entity_id: payload.entity_id,
        effect_id: payload.effect_id,
        removed_count,
    };
    vec![GameEvent::new(event.tick, "status.removed", event_data(&notice)).with_source(SOURCE)]
}

/// Handle the `world.tick_advanced` event, expiring due effects.
pub fn handle_tick_advanced<C: ContentLookup, K: WorldClock>(
    service: &StatusEffectService<'_, C, K>,
    event: &GameEvent,
) -> Vec<GameEvent> {
    let current_tick = match current_tick_from_event(event) {
        Ok(tick) => tick,
        Err(_) => return vec![],
    };
    let expired = match service.expire_due(current_tick) {
        Ok(effects) => effects,
        Err(_) => return vec![],
    };
    expired
        .iter()
        .map(|effect| {
            let notice = StatusExpiredNotice {
                active_effect: to_payload(effect),
            };
            GameEvent::new(event.tick, "status.expired", event_data(&notice)).with_source(SOURCE)
        })
        .collect()
}

fn current_tick_from_event(event: &GameEvent) -> Result<i32, String> {
    let value = event
        .data
        .get("current_tick")
        .or_else(|| event.data.get("tick"))
        .and_then(|v| {
            // Reject JSON booleans, which serde_json reports as integers via as_i64.
            if v.is_boolean() {
                None
            } else {
                v.as_i64()
            }
        });
    match value {
        Some(tick) => Ok(tick as i32),
        None => Ok(event.tick as i32),
    }
}

/// Register status effect domain handlers against a dispatcher.
///
/// The service is borrowed for as long as the dispatcher lives.
pub fn register_status_effect_handlers<'h, C: ContentLookup + 'h, K: WorldClock + 'h>(
    dispatcher: &mut DomainEventDispatcher<'h>,
    service: &'h StatusEffectService<'h, C, K>,
) {
    dispatcher.subscribe(
        "status.apply_requested",
        Box::new(move |event| handle_apply_requested(service, event)),
    );
    dispatcher.subscribe(
        "status.remove_requested",
        Box::new(move |event| handle_remove_requested(service, event)),
    );
    dispatcher.subscribe(
        "world.tick_advanced",
        Box::new(move |event| handle_tick_advanced(service, event)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::status_effects::repository::StatusEffectRepository;
    use std::collections::HashMap;

    #[test]
    fn parse_payload_returns_some_for_valid_and_none_for_malformed() {
        // Valid StatusApplyRequested payload round-trips.
        let mut good = JsonObject::new();
        good.insert("entity_id".into(), serde_json::json!("pc"));
        good.insert("effect_id".into(), serde_json::json!("dread"));
        let ev = GameEvent::new(0, "status.apply_requested", good);
        let parsed: Option<StatusApplyRequested> = parse_payload(&ev);
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().effect_id, "dread");

        // Malformed payload returns None without panicking (logs to stderr).
        let mut bad = JsonObject::new();
        bad.insert("not_a_field".into(), serde_json::json!(1));
        let ev = GameEvent::new(0, "status.apply_requested", bad);
        let parsed: Option<StatusApplyRequested> = parse_payload(&ev);
        assert!(parsed.is_none());
    }

    struct MapContent(HashMap<String, JsonObject>);
    impl ContentLookup for MapContent {
        fn get(&self, id: &str) -> Option<JsonObject> {
            self.0.get(id).cloned()
        }
    }
    struct Clock(i32);
    impl WorldClock for Clock {
        fn tick(&self) -> i32 {
            self.0
        }
    }

    fn content() -> MapContent {
        let mut m = HashMap::new();
        m.insert(
            "poison".to_string(),
            serde_json::from_value(serde_json::json!({
                "id": "poison",
                "name": "Poison",
                "default_duration_ticks": 3,
                "stacking": "replace",
            }))
            .unwrap(),
        );
        MapContent(m)
    }

    #[test]
    fn apply_then_expire_via_handlers() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let svc = StatusEffectService::new(
            StatusEffectRepository::new(&db),
            content(),
            Some(Clock(0)),
        );

        let req = StatusApplyRequested {
            entity_id: "hero".into(),
            effect_id: "poison".into(),
            duration_ticks: None,
            source: Some("trap".into()),
            data: JsonObject::new(),
        };
        let apply_event = GameEvent::new(0, "status.apply_requested", event_data(&req));
        let out = handle_apply_requested(&svc, &apply_event);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].event_type, "status.applied");
        assert_eq!(out[0].source, "status_effects");

        let mut tick_data = JsonObject::new();
        tick_data.insert("current_tick".into(), serde_json::json!(10));
        let tick_event = GameEvent::new(10, "world.tick_advanced", tick_data);
        let expired = handle_tick_advanced(&svc, &tick_event);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].event_type, "status.expired");
    }

    #[test]
    fn remove_via_handler_reports_count() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let svc = StatusEffectService::new(
            StatusEffectRepository::new(&db),
            content(),
            Some(Clock(0)),
        );
        svc.apply("hero", "poison", None, None, None).unwrap();

        let req = StatusRemoveRequested {
            entity_id: "hero".into(),
            effect_id: "poison".into(),
        };
        let event = GameEvent::new(0, "status.remove_requested", event_data(&req));
        let out = handle_remove_requested(&svc, &event);
        assert_eq!(out.len(), 1);
        let notice: StatusRemovedNotice =
            serde_json::from_value(serde_json::Value::Object(out[0].data.clone())).unwrap();
        assert_eq!(notice.removed_count, 1);
    }

    #[test]
    fn boolean_tick_is_rejected() {
        let mut data = JsonObject::new();
        data.insert("current_tick".into(), serde_json::json!(true));
        // Falls back to event.tick (5) rather than treating `true` as 1.
        let event = GameEvent::new(5, "world.tick_advanced", data);
        assert_eq!(current_tick_from_event(&event).unwrap(), 5);
    }
}
