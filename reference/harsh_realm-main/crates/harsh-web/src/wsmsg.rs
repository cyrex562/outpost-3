//! WebSocket message translation between game events and frontend frames.

use serde::Deserialize;
use serde_json::json;

use harsh_core::events::GameEvent;

/// Inbound message from the client. The frontend sends `{type:"command", text}`.
#[derive(Debug, Deserialize)]
pub struct ClientMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub text: String,
}

/// Convert a game event into the websocket frame(s) the frontend expects.
///
/// `gm.narrate` events become `{type:"narration", text}`; every other event
/// becomes `{type:"game_event", event:{...}}`. The frontend suppresses
/// `gm.narrate` as a game_event and renders the narration text instead.
pub fn event_to_frames(event: &GameEvent) -> Vec<String> {
    if event.event_type == "gm.narrate" {
        let text = event
            .data
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return vec![json!({ "type": "narration", "text": text }).to_string()];
    }
    let frame = json!({
        "type": "game_event",
        "event": {
            "id": event.id,
            "tick": event.tick,
            "event_type": event.event_type,
            "data": event.data,
            "source": event.source,
            "timestamp": event.timestamp,
        }
    });
    vec![frame.to_string()]
}

/// Build an `{type:"error", message}` frame.
pub fn error_frame(message: &str) -> String {
    json!({ "type": "error", "message": message }).to_string()
}
