//! Narrate when a player first crosses a low-health threshold.
//!
//! Ported from `src/harsh_realm/engine/low_health_narration.py`. Each player is
//! warned at most once per controller session; the warned-set uses interior
//! mutability so the narrator can be registered as a (shared) domain handler.

use std::cell::RefCell;
use std::collections::HashSet;

use crate::domain_events::DomainEventDispatcher;
use crate::events::GameEvent;
use crate::payloads::notices_combat::NarrationNotice;
use crate::runtime::{JsonObject, JsonValue};

const DEFAULT_THRESHOLD: f64 = 0.25;

/// Emit one low-health narration per player per controller session.
pub struct LowHealthNarrator {
    threshold: f64,
    warned: RefCell<HashSet<String>>,
}

struct LowHealthTarget {
    target_id: String,
    is_player: bool,
    hp: i64,
    max_hp: i64,
    name: String,
    alive: bool,
}

impl Default for LowHealthNarrator {
    fn default() -> Self {
        LowHealthNarrator::new(DEFAULT_THRESHOLD)
    }
}

impl LowHealthNarrator {
    /// Create a narrator with an explicit low-health threshold fraction.
    pub fn new(threshold_fraction: f64) -> Self {
        LowHealthNarrator {
            threshold: threshold_fraction,
            warned: RefCell::new(HashSet::new()),
        }
    }

    /// Handle a post-damage event, returning a narration when appropriate.
    pub fn handle(&self, event: &GameEvent) -> Vec<GameEvent> {
        let Some(target) = extract_target(&event.data) else {
            return vec![];
        };
        if self.warned.borrow().contains(&target.target_id) {
            return vec![];
        }
        if !target.is_player || !target.alive || target.max_hp <= 0 {
            return vec![];
        }
        let threshold = (target.max_hp as f64 * self.threshold) as i64;
        if target.hp > threshold {
            return vec![];
        }
        self.warned.borrow_mut().insert(target.target_id.clone());
        let notice = NarrationNotice {
            text: format!("{} is gravely wounded.", target.name),
        };
        vec![GameEvent::new(event.tick, "gm.narrate", event_data(&notice))
            .with_source("low_health_narrator")]
    }
}

fn event_data(notice: &NarrationNotice) -> JsonObject {
    match serde_json::to_value(notice) {
        Ok(JsonValue::Object(map)) => map,
        _ => JsonObject::new(),
    }
}

fn extract_target(data: &JsonObject) -> Option<LowHealthTarget> {
    if data.contains_key("target_hp") {
        return extract_generic_target(data);
    }
    if data.contains_key("player_hp") {
        return extract_player_hit_target(data);
    }
    None
}

fn extract_generic_target(data: &JsonObject) -> Option<LowHealthTarget> {
    let target_id = string_value(data.get("target_id"))?;
    let hp = int_value(data.get("target_hp"))?;
    let max_hp = int_value(data.get("target_max_hp"))?;
    Some(LowHealthTarget {
        is_player: data.get("target_is_player") == Some(&JsonValue::Bool(true)),
        hp,
        max_hp,
        name: string_value(data.get("target_name")).unwrap_or_else(|| target_id.clone()),
        alive: data
            .get("target_alive")
            .map_or(true, |v| v == &JsonValue::Bool(true)),
        target_id,
    })
}

fn extract_player_hit_target(data: &JsonObject) -> Option<LowHealthTarget> {
    let hp = int_value(data.get("player_hp"))?;
    let max_hp = int_value(data.get("player_max_hp"))?;
    let target_id = string_value(data.get("player_id")).unwrap_or_else(|| "player".to_string());
    Some(LowHealthTarget {
        is_player: true,
        hp,
        max_hp,
        name: string_value(data.get("player_name")).unwrap_or_else(|| target_id.clone()),
        alive: data
            .get("player_alive")
            .map_or(true, |v| v == &JsonValue::Bool(true)),
        target_id,
    })
}

/// Return an int only for actual integer JSON values (not booleans).
fn int_value(value: Option<&JsonValue>) -> Option<i64> {
    match value {
        Some(JsonValue::Number(n)) => n.as_i64(),
        _ => None,
    }
}

/// Return a string only for actual string JSON values.
fn string_value(value: Option<&JsonValue>) -> Option<String> {
    match value {
        Some(JsonValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// Register the low-health narrator with the domain dispatcher.
///
/// The narrator is borrowed for the dispatcher's lifetime; the caller owns it.
pub fn register_low_health_narrator<'h>(
    dispatcher: &mut DomainEventDispatcher<'h>,
    narrator: &'h LowHealthNarrator,
) {
    dispatcher.subscribe("combat.player_hit", Box::new(|event| narrator.handle(event)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit_event(hp: i64, max_hp: i64, alive: bool) -> GameEvent {
        let mut data = JsonObject::new();
        data.insert("player_hp".into(), serde_json::json!(hp));
        data.insert("player_max_hp".into(), serde_json::json!(max_hp));
        data.insert("player_name".into(), serde_json::json!("Hero"));
        data.insert("player_alive".into(), serde_json::json!(alive));
        GameEvent::new(0, "combat.player_hit", data)
    }

    #[test]
    fn warns_once_when_below_threshold() {
        let narrator = LowHealthNarrator::default();
        // 2 / 10 = 0.2 <= 0.25 threshold -> warn.
        let out = narrator.handle(&hit_event(2, 10, true));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].event_type, "gm.narrate");
        assert_eq!(
            out[0].data.get("text").unwrap().as_str(),
            Some("Hero is gravely wounded.")
        );
        // Second time: already warned -> silent.
        assert!(narrator.handle(&hit_event(1, 10, true)).is_empty());
    }

    #[test]
    fn no_warning_above_threshold_or_dead() {
        let narrator = LowHealthNarrator::default();
        assert!(narrator.handle(&hit_event(8, 10, true)).is_empty());
        assert!(narrator.handle(&hit_event(0, 10, false)).is_empty());
    }

    #[test]
    fn generic_target_shape_requires_player_flag() {
        let narrator = LowHealthNarrator::default();
        let mut data = JsonObject::new();
        data.insert("target_id".into(), serde_json::json!("hero"));
        data.insert("target_hp".into(), serde_json::json!(1));
        data.insert("target_max_hp".into(), serde_json::json!(10));
        data.insert("target_is_player".into(), serde_json::json!(true));
        data.insert("target_name".into(), serde_json::json!("Hero"));
        let event = GameEvent::new(3, "combat.attack", data);
        let out = narrator.handle(&event);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tick, 3);
    }

    #[test]
    fn non_player_target_is_ignored() {
        let narrator = LowHealthNarrator::default();
        let mut data = JsonObject::new();
        data.insert("target_id".into(), serde_json::json!("goblin"));
        data.insert("target_hp".into(), serde_json::json!(1));
        data.insert("target_max_hp".into(), serde_json::json!(10));
        // No target_is_player -> treated as non-player.
        let event = GameEvent::new(0, "combat.attack", data);
        assert!(narrator.handle(&event).is_empty());
    }
}
