//! Typed payload models for gameplay events and transport/message envelopes.
//!
//! Ported from `src/harsh_realm/payloads/`. The Python `PayloadModel` base only
//! provided `model_dump`-style helpers; in Rust each payload is a plain serde
//! struct. `serde_json::to_value(&payload)` is the `as_event_data` equivalent.

pub use crate::runtime::{JsonObject, JsonValue};

pub mod client_event;
pub mod contexts;
pub mod notices_combat;
pub mod notices_world;
pub mod quests;
pub mod requests;
pub mod transport;
