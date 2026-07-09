//! Event bus for Harsh Realm.
//!
//! Ported from `src/harsh_realm/events.py` (the pure substrate). All game state
//! changes flow through the [`EventBus`]; handlers may return new events which
//! cascade up to a depth limit. The DB-backed `EventLogger` is ported with the
//! persistence layer.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::runtime::{JsonObject, JsonValue};

const MAX_CASCADE_DEPTH: usize = 10;

const PRESENTATION_EVENT_TYPES: [&str; 3] = ["gm.narrate", "gm.suggestions", "town.map"];

/// The engine's built-in, client-facing event types — every `event_type` the backend
/// forwards over the websocket and expects the frontend to either handle or explicitly
/// suppress. This is the canonical inventory that the `frontend_handles_all_client_facing_events`
/// coverage test checks against `_websocketHandlers.ts`.
///
/// HR-792 (Phase 0): hand-maintained. When you add a client-facing event, add it here AND
/// give it a handler (or suppress entry) in `_websocketHandlers.ts`, or the coverage test
/// fails. This is the seed of the HR-794 event registry, which will make the list
/// authoritative (generated from the emit sites) rather than hand-maintained.
///
/// Excluded by design: `*_requested` command-intent events (consumed server-side, filtered
/// by `resolve_domain_events`), `time.tick` (internal heartbeat consumed by `run_ir_triggers`,
/// never forwarded), and dynamic pack/IR-trigger-emitted events (covered by the frontend's
/// runtime unhandled-event warning instead of this static list).
pub const CLIENT_FACING_EVENT_TYPES: [&str; 53] = [
    "action.skill_check",
    "character.created",
    "character.death",
    "character.death_final",
    "character.expert_reroll",
    "character.hp_changed",
    "character.level_up",
    "character.respawn",
    "character.xp_gained",
    "combat.actions",
    "combat.attack",
    "combat.enemy_defeated",
    "combat.fled",
    "combat.player_hit",
    "combat.positions",
    "combat.save",
    "combat.start",
    "dungeon.enter_room",
    "exploration.actions",
    "exploration.enter_hex",
    "exploration.moved",
    "exploration.revealed",
    "gm.narrate",
    "gm.scene_change",
    "gm.suggestions",
    "inventory.ammo_consumed",
    "inventory.item_given",
    "inventory.item_lost",
    "loot.source_revealed",
    "oracle.chaos_changed",
    "oracle.fate_check",
    "oracle.random_event",
    "oracle.scene_check",
    "quest.accepted",
    "quest.completed",
    "quest.failed",
    "quest.progress_updated",
    "shopping.purchase",
    "shopping.sale",
    "social.disposition_change",
    "social.healer",
    "status.applied",
    "status.expired",
    "status.removed",
    "town.map",
    "town.move",
    "travel.blocked",
    "travel.cancelled",
    "travel.completed",
    "travel.interrupted",
    "travel.resumed",
    "travel.started",
    "travel.step",
];

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn default_source() -> String {
    "system".to_string()
}

/// An immutable game event flowing through the bus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEvent {
    /// World clock value at creation time.
    pub tick: i64,
    /// Dotted namespace string, e.g. `"combat.attack"`.
    pub event_type: String,
    /// JSON-serializable payload.
    pub data: JsonObject,
    /// Origin (`"player"`, `"system"`, `"gm"`, or a subsystem name).
    #[serde(default = "default_source")]
    pub source: String,
    /// Auto-generated UUID.
    #[serde(default = "new_id")]
    pub id: String,
    /// ISO-8601 UTC timestamp.
    #[serde(default = "now_iso")]
    pub timestamp: String,
}

impl GameEvent {
    /// Create an event with an auto-generated id and timestamp.
    pub fn new(tick: i64, event_type: impl Into<String>, data: JsonObject) -> Self {
        GameEvent {
            tick,
            event_type: event_type.into(),
            data,
            source: default_source(),
            id: new_id(),
            timestamp: now_iso(),
        }
    }

    /// Builder-style override of the event source.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }
}

/// Classification of a game event for logging and debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A requested command/intent (event type ends with `_requested`).
    CommandIntent,
    /// An authoritative state/outcome event.
    DomainResult,
    /// A presentation-only event.
    Presentation,
}

impl EventKind {
    /// The snake_case string form used in logs.
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::CommandIntent => "command_intent",
            EventKind::DomainResult => "domain_result",
            EventKind::Presentation => "presentation",
        }
    }
}

/// Classify a game event.
pub fn classify_event_kind(event: &GameEvent) -> EventKind {
    if event.event_type.ends_with("_requested") {
        EventKind::CommandIntent
    } else if PRESENTATION_EVENT_TYPES.contains(&event.event_type.as_str()) {
        EventKind::Presentation
    } else {
        EventKind::DomainResult
    }
}

/// Whether an event represents authoritative state/outcome data.
pub fn is_authoritative_event(event: &GameEvent) -> bool {
    classify_event_kind(event) == EventKind::DomainResult
}

/// Build structured log metadata for a game event.
pub fn describe_event_for_log(event: &GameEvent) -> JsonObject {
    let kind = classify_event_kind(event);
    let mut out = JsonObject::new();
    out.insert("event_id".into(), JsonValue::from(event.id.clone()));
    out.insert("tick".into(), JsonValue::from(event.tick));
    out.insert("event_type".into(), JsonValue::from(event.event_type.clone()));
    out.insert("source".into(), JsonValue::from(event.source.clone()));
    out.insert("event_kind".into(), JsonValue::from(kind.as_str()));
    out.insert(
        "authoritative".into(),
        JsonValue::from(is_authoritative_event(event)),
    );
    out.insert("timestamp".into(), JsonValue::from(event.timestamp.clone()));
    out.insert("payload".into(), JsonValue::Object(event.data.clone()));
    out
}

/// A handler receives an event and may return events to cascade.
pub type HandlerFn = Box<dyn Fn(&GameEvent) -> Vec<GameEvent>>;

/// In-process pub/sub event dispatcher with bounded cascade.
pub struct EventBus {
    handlers: HashMap<String, Vec<HandlerFn>>,
    wildcard_handlers: Vec<HandlerFn>,
    max_cascade_depth: usize,
}

impl Default for EventBus {
    fn default() -> Self {
        EventBus::new(MAX_CASCADE_DEPTH)
    }
}

impl EventBus {
    /// Create a bus with the given cascade depth limit.
    pub fn new(max_cascade_depth: usize) -> Self {
        EventBus {
            handlers: HashMap::new(),
            wildcard_handlers: Vec::new(),
            max_cascade_depth,
        }
    }

    /// Register a handler for a specific event type.
    ///
    /// (Unlike the Python bus, identical-handler de-duplication is not
    /// performed — Rust closures are not comparable.)
    pub fn subscribe(&mut self, event_type: impl Into<String>, handler: HandlerFn) {
        self.handlers.entry(event_type.into()).or_default().push(handler);
    }

    /// Register a wildcard handler that receives every published event.
    pub fn subscribe_all(&mut self, handler: HandlerFn) {
        self.wildcard_handlers.push(handler);
    }

    /// Dispatch an event and process cascades; returns all events published
    /// during this call (including cascades), in dispatch order.
    pub fn publish(&self, event: GameEvent) -> Vec<GameEvent> {
        let mut acc = Vec::new();
        self.dispatch(event, 0, &mut acc);
        acc
    }

    fn dispatch(&self, event: GameEvent, depth: usize, acc: &mut Vec<GameEvent>) {
        if depth > self.max_cascade_depth {
            return;
        }
        // Run handlers against the event before moving it into the accumulator.
        let mut children: Vec<GameEvent> = Vec::new();
        if let Some(hs) = self.handlers.get(&event.event_type) {
            for h in hs {
                children.extend(h(&event));
            }
        }
        for h in &self.wildcard_handlers {
            children.extend(h(&event));
        }
        acc.push(event);
        for child in children {
            self.dispatch(child, depth + 1, acc);
        }
    }

    /// Remove all subscriptions.
    pub fn clear(&mut self) {
        self.handlers.clear();
        self.wildcard_handlers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HR-792 / HR-795 104c backend↔frontend coverage gate.
    ///
    /// After the 104c cutover every client-facing event is either:
    ///   (a) handled by a reducer registered in the worldModel files, or
    ///   (b) listed in `suppressed.ts` as an intentionally no-op event.
    ///
    /// This test reads all worldModel source files and asserts that each
    /// `CLIENT_FACING_EVENT_TYPES` entry appears as a quoted string literal
    /// in at least one of them.  The old `_websocketHandlers.ts` is kept as a
    /// thin adapter (narration / error frame handling) but no longer carries
    /// the game_event dispatch strings.
    #[test]
    fn frontend_handles_all_client_facing_events() {
        let base = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../frontend/src/worldModel/"
        );
        let files = [
            "reducers.ts",
            "reducers-b.ts",
            "reducers-b-character.ts",
            "reducers-b-combat.ts",
            "reducers-b-misc.ts",
            "suppressed.ts",
        ];
        let mut combined = String::new();
        for f in &files {
            let path = format!("{}{}", base, f);
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("worldModel/{f} must exist (HR-795 104c coverage gate)"));
            combined.push_str(&content);
            combined.push('\n');
        }
        let missing: Vec<&str> = CLIENT_FACING_EVENT_TYPES
            .iter()
            .copied()
            .filter(|ev| !combined.contains(&format!("\"{ev}\"")))
            .collect();
        assert!(
            missing.is_empty(),
            "these client-facing event types are emitted by the backend but neither handled \
             (as a reducer in worldModel/reducers*.ts) nor suppressed \
             (in worldModel/suppressed.ts): {missing:?}"
        );
    }

    #[test]
    fn client_facing_list_has_no_requested_or_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for ev in CLIENT_FACING_EVENT_TYPES {
            assert!(
                !ev.ends_with("_requested"),
                "{ev} is a command-intent event; it is filtered out and never reaches the client"
            );
            assert!(seen.insert(ev), "duplicate entry in CLIENT_FACING_EVENT_TYPES: {ev}");
        }
    }

    #[test]
    fn classifies_event_kinds() {
        let req = GameEvent::new(0, "combat.attack_requested", JsonObject::new());
        assert_eq!(classify_event_kind(&req), EventKind::CommandIntent);
        let pres = GameEvent::new(0, "gm.narrate", JsonObject::new());
        assert_eq!(classify_event_kind(&pres), EventKind::Presentation);
        let dom = GameEvent::new(0, "combat.attack", JsonObject::new());
        assert_eq!(classify_event_kind(&dom), EventKind::DomainResult);
        assert!(is_authoritative_event(&dom));
        assert!(!is_authoritative_event(&pres));
    }

    #[test]
    fn publish_dispatches_and_cascades() {
        let mut bus = EventBus::default();
        bus.subscribe(
            "a",
            Box::new(|e| {
                if e.event_type == "a" {
                    vec![GameEvent::new(e.tick, "b", JsonObject::new())]
                } else {
                    vec![]
                }
            }),
        );
        let out = bus.publish(GameEvent::new(1, "a", JsonObject::new()));
        let types: Vec<&str> = out.iter().map(|e| e.event_type.as_str()).collect();
        assert_eq!(types, vec!["a", "b"]);
    }

    #[test]
    fn cascade_depth_is_bounded() {
        let mut bus = EventBus::new(3);
        // Each event spawns another of the same type -> would be infinite.
        bus.subscribe_all(Box::new(|e| {
            vec![GameEvent::new(e.tick, "loop", JsonObject::new())]
        }));
        let out = bus.publish(GameEvent::new(0, "loop", JsonObject::new()));
        // depth 0..=3 inclusive = 4 dispatched events before the limit stops it.
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn describe_includes_metadata() {
        let e = GameEvent::new(7, "combat.attack", JsonObject::new());
        let meta = describe_event_for_log(&e);
        assert_eq!(meta["tick"], JsonValue::from(7));
        assert_eq!(meta["event_kind"], JsonValue::from("domain_result"));
        assert_eq!(meta["authoritative"], JsonValue::from(true));
    }
}
