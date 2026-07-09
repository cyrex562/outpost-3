//! The stateful game-loop websocket (`/ws`).
//!
//! On connect the client receives the current scene's initial narration. Each
//! `{type:"command", text}` is forwarded to the session actor; the resulting
//! game events are broadcast and streamed back as narration/game_event frames.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use tokio::sync::broadcast::error::RecvError;

use crate::state::AppState;
use crate::wsmsg::{event_to_frames, ClientMessage};

/// Upgrade an HTTP request to the game websocket.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut events = state.session.events.subscribe();

    // Send the current scene's opening narration to this client only.
    if let Ok(initial) = state.session.initial_narration().await {
        for event in &initial {
            for frame in event_to_frames(event) {
                if socket.send(Message::Text(frame)).await.is_err() {
                    return;
                }
            }
        }
    }

    loop {
        tokio::select! {
            inbound = socket.recv() => {
                match inbound {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(msg) = serde_json::from_str::<ClientMessage>(&text) {
                            if msg.message_type == "command" {
                                // Events broadcast by the actor arrive via the
                                // `events` branch below; ignore the ack here.
                                let _ = state.session.player_input(msg.text).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {} // ping/pong/binary: ignore
                    Some(Err(_)) => break,
                }
            }
            broadcast = events.recv() => {
                match broadcast {
                    Ok(event) => {
                        for frame in event_to_frames(&event) {
                            if socket.send(Message::Text(frame)).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => {} // dropped frames under load; continue
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }
}
