//! Typed player-command value types.
//!
//! Ported from `src/harsh_realm/models/parser.py`. Holds the parsed result of a
//! single line of player input. The command *parser* itself is ported
//! separately; this is the value type it produces and that scene handlers
//! consume.

use serde::{Deserialize, Serialize};

/// The result of parsing a single line of player input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// The original, untrimmed input line.
    pub raw: String,
    /// The resolved canonical verb (e.g. `"move"`, `"look"`).
    pub verb: String,
    /// Remaining whitespace-split arguments after the verb.
    pub args: Vec<String>,
    /// A movement direction, when the command resolved to one.
    pub direction: Option<String>,
}

impl ParsedCommand {
    /// Create a parsed command.
    pub fn new(
        raw: impl Into<String>,
        verb: impl Into<String>,
        args: Vec<String>,
        direction: Option<String>,
    ) -> Self {
        ParsedCommand {
            raw: raw.into(),
            verb: verb.into(),
            args,
            direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_fields() {
        let cmd = ParsedCommand::new("go north", "move", vec![], Some("north".to_string()));
        assert_eq!(cmd.raw, "go north");
        assert_eq!(cmd.verb, "move");
        assert!(cmd.args.is_empty());
        assert_eq!(cmd.direction.as_deref(), Some("north"));
    }

    #[test]
    fn equality_is_structural() {
        let a = ParsedCommand::new("attack orc", "attack", vec!["orc".to_string()], None);
        let b = ParsedCommand::new("attack orc", "attack", vec!["orc".to_string()], None);
        let c = ParsedCommand::new("attack orc", "attack", vec!["goblin".to_string()], None);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn round_trips_through_json() {
        let cmd = ParsedCommand::new(
            "buy 3 torches",
            "buy",
            vec!["3".to_string(), "torches".to_string()],
            None,
        );
        let json = serde_json::to_string(&cmd).unwrap();
        assert_eq!(serde_json::from_str::<ParsedCommand>(&json).unwrap(), cmd);
    }

    #[test]
    fn direction_absent_serializes_as_null() {
        let cmd = ParsedCommand::new("look", "look", vec![], None);
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"direction\":null"), "got {json}");
    }
}
