//! Custom error types for Harsh Realm game logic.
//!
//! Ported from `src/harsh_realm/exceptions.py`. The Python `HarshRealmError`
//! class hierarchy becomes a single enum; the plugin-invocation variant keeps
//! its structured `code`/`message`/`data`.

use std::fmt;

use crate::runtime::JsonObject;

/// Base error type for game logic errors.
#[derive(Debug, Clone, PartialEq)]
pub enum HarshRealmError {
    /// An entity id doesn't exist in the world.
    EntityNotFound(String),
    /// A command is not valid in the current scene state.
    InvalidCommand(String),
    /// A structured character-creation submission is invalid.
    CharacterCreation(String),
    /// An operation requires a loaded world but none is active.
    WorldNotLoaded(String),
    /// An action requires resources the entity doesn't have.
    InsufficientResource(String),
    /// A world database has an unexpected schema.
    Schema(String),
    /// A plugin IPC message is malformed or violates the protocol.
    PluginProtocol(String),
    /// A plugin failed to spawn or complete the handshake.
    PluginStart(String),
    /// A plugin invocation or handshake exceeded its deadline.
    PluginTimeout(String),
    /// The plugin process exited unexpectedly with work pending.
    PluginCrashed(String),
    /// A plugin returned a structured error for an invocation.
    PluginInvocation {
        /// Stable error code.
        code: String,
        /// Human-readable message.
        message: String,
        /// Optional structured data.
        data: Option<JsonObject>,
    },
}

impl fmt::Display for HarshRealmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarshRealmError::EntityNotFound(m) => write!(f, "{m}"),
            HarshRealmError::InvalidCommand(m) => write!(f, "{m}"),
            HarshRealmError::CharacterCreation(m) => write!(f, "{m}"),
            HarshRealmError::WorldNotLoaded(m) => write!(f, "{m}"),
            HarshRealmError::InsufficientResource(m) => write!(f, "{m}"),
            HarshRealmError::Schema(m) => write!(f, "{m}"),
            HarshRealmError::PluginProtocol(m) => write!(f, "{m}"),
            HarshRealmError::PluginStart(m) => write!(f, "{m}"),
            HarshRealmError::PluginTimeout(m) => write!(f, "{m}"),
            HarshRealmError::PluginCrashed(m) => write!(f, "{m}"),
            HarshRealmError::PluginInvocation { code, message, .. } => {
                write!(f, "[{code}] {message}")
            }
        }
    }
}

impl std::error::Error for HarshRealmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_invocation_display_matches_python_format() {
        let e = HarshRealmError::PluginInvocation {
            code: "E_TIMEOUT".into(),
            message: "hook stalled".into(),
            data: None,
        };
        assert_eq!(e.to_string(), "[E_TIMEOUT] hook stalled");
    }

    #[test]
    fn simple_variants_display_message() {
        assert_eq!(
            HarshRealmError::EntityNotFound("no such entity: x".into()).to_string(),
            "no such entity: x"
        );
    }
}
