//! Command parser — converts raw player text into [`ParsedCommand`] values.
//!
//! Ported from `src/harsh_realm/parser/parser.py` and
//! `src/harsh_realm/parser/commands.py`.

use crate::command::ParsedCommand;

/// Resolve a raw verb token to its canonical verb.
fn verb_alias(token: &str) -> Option<&'static str> {
    let canonical = match token {
        "go" | "walk" | "head" | "move" => "move",
        "travel" => "travel",
        "goto" => "travel",
        "resume" => "resume",
        "cancel" => "cancel",
        "look" | "l" | "x" | "inspect" => "look",
        "examine" => "examine",
        "talk" | "speak" => "talk",
        "explore" => "explore",
        "enter" => "enter",
        "wait" => "wait",
        // HR-798: `rest` must map to its own verb so it reaches `handle_rest`
        // (which heals). It was aliased to `wait`, so `rest` only ever ran
        // `handle_wait` ("Time passes...") and never restored HP — the command
        // path never reached the (unit-tested-in-isolation) `handle_rest`.
        "rest" => "rest",
        "attack" | "fight" | "kill" | "hit" => "attack",
        "flee" | "run" | "escape" => "flee",
        "use" => "use",
        "yes" | "y" => "yes",
        "no" => "no",
        "help" | "?" | "h" => "help",
        "status" | "stat" | "stats" | "char" | "character" => "status",
        "inventory" | "inv" | "i" | "items" => "inventory",
        "quests" | "quest" | "journal" | "log" => "quests",
        "search" => "search",
        "convince" | "persuade" => "convince",
        "intimidate" | "threaten" => "intimidate",
        "deceive" | "lie" => "deceive",
        "bribe" => "bribe",
        "connect" => "connect",
        "ask" => "ask",
        "leave" | "goodbye" => "leave",
        "shop" => "shop",
        "buy" => "buy",
        "sell" => "sell",
        "list" => "list",
        "oracle" => "oracle",
        "add" => "add",
        "resolve" => "resolve",
        "create" => "create",
        "advance" | "charge" => "advance",
        "sneak" => "sneak",
        "heal" => "heal",
        "harvest" => "harvest",
        "withdraw" | "retreat" => "withdraw",
        "take" | "pickup" | "grab" => "take",
        "remove" => "remove",
        "admin" => "admin",
        "save" => "save",
        "quit" | "exit" | "q" => "quit",
        _ => return None,
    };
    Some(canonical)
}

/// Resolve a direction token to its canonical direction.
fn direction_alias(token: &str) -> Option<&'static str> {
    let canonical = match token {
        "n" | "north" => "north",
        "e" | "east" => "east",
        "w" | "west" => "west",
        "s" | "south" => "south",
        "ne" | "northeast" => "northeast",
        "nw" | "northwest" => "northwest",
        "se" | "southeast" => "southeast",
        "sw" | "southwest" => "southwest",
        _ => return None,
    };
    Some(canonical)
}

/// Parses raw player input into [`ParsedCommand`] values.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommandParser;

impl CommandParser {
    /// Parse raw player input into a [`ParsedCommand`].
    pub fn parse(&self, text: &str) -> ParsedCommand {
        let raw = text.to_string();
        let lowered = text.trim().to_lowercase();
        let tokens: Vec<String> = lowered.split_whitespace().map(str::to_string).collect();

        let Some(first) = tokens.first() else {
            return ParsedCommand::new(raw, "unknown", Vec::new(), None);
        };
        let rest: Vec<String> = tokens[1..].to_vec();

        // Special admin slash commands.
        if first == "/gm" || first == "/admin" {
            return ParsedCommand::new(raw, "admin", rest, None);
        }

        // Verb alias.
        if let Some(verb) = verb_alias(first) {
            if verb == "move" {
                let direction = rest.first().and_then(|t| direction_alias(t)).map(str::to_string);
                let remaining = if rest.is_empty() {
                    Vec::new()
                } else {
                    rest[1..].to_vec()
                };
                return ParsedCommand::new(raw, verb, remaining, direction);
            }
            return ParsedCommand::new(raw, verb, rest, None);
        }

        // Bare direction (e.g. "n", "north").
        if let Some(direction) = direction_alias(first) {
            return ParsedCommand::new(raw, "move", rest, Some(direction.to_string()));
        }

        ParsedCommand::new(raw, "unknown", rest, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_unknown() {
        let cmd = CommandParser.parse("   ");
        assert_eq!(cmd.verb, "unknown");
        assert!(cmd.args.is_empty());
        assert!(cmd.direction.is_none());
    }

    #[test]
    fn verb_aliases_resolve() {
        assert_eq!(CommandParser.parse("fight goblin").verb, "attack");
        let cmd = CommandParser.parse("fight goblin");
        assert_eq!(cmd.args, vec!["goblin".to_string()]);
        assert_eq!(CommandParser.parse("inv").verb, "inventory");
        assert_eq!(CommandParser.parse("charge").verb, "advance");
    }

    #[test]
    fn rest_is_its_own_verb_not_wait() {
        // HR-798: `rest` was aliased to `wait`, so it ran handle_wait and never healed.
        assert_eq!(CommandParser.parse("rest").verb, "rest");
        assert_eq!(CommandParser.parse("rest until healed").verb, "rest");
        assert_eq!(CommandParser.parse("wait").verb, "wait");
    }

    #[test]
    fn move_verb_extracts_direction() {
        let cmd = CommandParser.parse("go n");
        assert_eq!(cmd.verb, "move");
        assert_eq!(cmd.direction.as_deref(), Some("north"));
        assert!(cmd.args.is_empty());

        let extra = CommandParser.parse("go ne quickly");
        assert_eq!(extra.direction.as_deref(), Some("northeast"));
        assert_eq!(extra.args, vec!["quickly".to_string()]);
    }

    #[test]
    fn bare_direction_is_move() {
        let cmd = CommandParser.parse("north");
        assert_eq!(cmd.verb, "move");
        assert_eq!(cmd.direction.as_deref(), Some("north"));
    }

    #[test]
    fn slash_admin_command() {
        let cmd = CommandParser.parse("/gm spawn wolf");
        assert_eq!(cmd.verb, "admin");
        assert_eq!(cmd.args, vec!["spawn".to_string(), "wolf".to_string()]);
    }

    #[test]
    fn unknown_verb_preserves_args() {
        let cmd = CommandParser.parse("frobnicate the thing");
        assert_eq!(cmd.verb, "unknown");
        assert_eq!(cmd.args, vec!["the".to_string(), "thing".to_string()]);
    }
}
