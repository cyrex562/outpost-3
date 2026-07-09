//! Transport/websocket messages and status payloads. Ported from
//! `payloads/transport.py`. Literal `type` discriminators become string fields
//! with a fixed default.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::runtime::{JsonObject, JsonValue};

/// Typed entity row with parsed JSON payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityRecord {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Location column.
    #[serde(default)]
    pub location_q: Option<i32>,
    /// Location row.
    #[serde(default)]
    pub location_r: Option<i32>,
    /// Parsed entity payload.
    pub entity_data: JsonObject,
    /// Entity type.
    #[serde(default)]
    pub entity_type: Option<String>,
    /// Alive flag (bool or int in legacy rows).
    #[serde(default)]
    pub alive: Option<JsonValue>,
}

/// Serialized event body for websocket game-event messages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSocketEventBody {
    /// Event id.
    pub id: String,
    /// Tick.
    pub tick: i64,
    /// Event type.
    pub event_type: String,
    /// Payload.
    pub data: JsonObject,
    /// Source.
    pub source: String,
    /// Timestamp.
    pub timestamp: String,
}

fn d_narration() -> String {
    "narration".to_string()
}
fn d_error() -> String {
    "error".to_string()
}
fn d_player_input() -> String {
    "player_input".to_string()
}
fn d_game_event() -> String {
    "game_event".to_string()
}

/// Narration websocket/menu message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NarrationMessage {
    /// Message type discriminator (`"narration"`).
    #[serde(rename = "type", default = "d_narration")]
    pub message_type: String,
    /// Text.
    pub text: String,
}

/// Error websocket message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorMessage {
    /// Message type discriminator (`"error"`).
    #[serde(rename = "type", default = "d_error")]
    pub message_type: String,
    /// Message.
    pub message: String,
}

/// Echoed player-input websocket message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInputMessage {
    /// Message type discriminator (`"player_input"`).
    #[serde(rename = "type", default = "d_player_input")]
    pub message_type: String,
    /// Text.
    pub text: String,
}

/// Websocket envelope for a published game event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameEventMessage {
    /// Message type discriminator (`"game_event"`).
    #[serde(rename = "type", default = "d_game_event")]
    pub message_type: String,
    /// Event body.
    pub event: WebSocketEventBody,
}

/// Typed live-update payload for editor mutations on the active world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorLiveUpdate {
    /// Resource type.
    pub resource_type: String,
    /// Action.
    pub action: String,
    /// World name.
    pub world: String,
    /// Resource id.
    #[serde(default)]
    pub resource_id: Option<String>,
    /// Column.
    #[serde(default)]
    pub q: Option<i32>,
    /// Row.
    #[serde(default)]
    pub r: Option<i32>,
    /// Count.
    #[serde(default)]
    pub count: Option<i32>,
}

/// Typed live-update payload for runtime-relevant admin config changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminConfigUpdated {
    /// Config type.
    pub config_type: String,
    /// Operation.
    pub operation: String,
    /// World name.
    pub world: String,
    /// Key.
    #[serde(default)]
    pub key: Option<String>,
    /// Scope.
    #[serde(default)]
    pub scope: Option<String>,
}

/// Request payload for applying a status effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusApplyRequested {
    /// Entity id.
    pub entity_id: String,
    /// Effect id.
    pub effect_id: String,
    /// Duration in ticks.
    #[serde(default)]
    pub duration_ticks: Option<i32>,
    /// Source.
    #[serde(default)]
    pub source: Option<String>,
    /// Data payload.
    #[serde(default)]
    pub data: JsonObject,
}

/// Serialized active status effect payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ActiveStatusEffectPayload {
    /// Row id.
    // ts-rs maps i64 → `bigint`, but serde_json serializes i64 as a plain JSON
    // number; the wire value is a number, so pin the TS type accordingly.
    #[ts(type = "number")]
    pub id: i64,
    /// Entity id.
    pub entity_id: String,
    /// Effect id.
    pub effect_id: String,
    /// Applied-at tick.
    #[ts(type = "number")]
    pub applied_at_tick: i64,
    /// Expires-at tick.
    #[ts(type = "number | null")]
    pub expires_at_tick: Option<i64>,
    /// Source.
    pub source: Option<String>,
    /// Arbitrary effect data.
    #[serde(default)]
    #[ts(type = "Record<string, unknown>")]
    pub data: JsonObject,
}

/// Terminal payload emitted after a status effect is applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct StatusAppliedNotice {
    /// The active effect.
    pub active_effect: ActiveStatusEffectPayload,
}

/// Request payload for removing a status effect from an entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusRemoveRequested {
    /// Entity id.
    pub entity_id: String,
    /// Effect id.
    pub effect_id: String,
}

/// Terminal payload emitted after status effects are removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct StatusRemovedNotice {
    /// Entity id.
    pub entity_id: String,
    /// Effect id.
    pub effect_id: String,
    /// Number removed.
    pub removed_count: i32,
}

/// Terminal payload emitted after a status effect expires.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct StatusExpiredNotice {
    /// The active effect.
    pub active_effect: ActiveStatusEffectPayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narration_message_type_default() {
        let m: NarrationMessage = serde_json::from_str(r#"{"text":"hi"}"#).unwrap();
        assert_eq!(m.message_type, "narration");
        let json = serde_json::to_value(&m).unwrap();
        assert_eq!(json["type"], JsonValue::from("narration"));
    }

    #[test]
    fn entity_record_alive_accepts_int_or_bool() {
        let r: EntityRecord =
            serde_json::from_str(r#"{"id":"e","name":"n","entity_data":{},"alive":1}"#).unwrap();
        assert_eq!(r.alive, Some(JsonValue::from(1)));
    }
}
