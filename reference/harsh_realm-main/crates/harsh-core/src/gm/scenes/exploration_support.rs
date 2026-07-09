//! Shared support constants for the exploration scene.
//!
//! Ported from `src/harsh_realm/gm/scenes/exploration_support.py`.

/// Return the blocked-movement message for an impassable terrain.
pub fn blocked_message(terrain: &str) -> &'static str {
    match terrain {
        "mountains" => {
            "A sheer wall of mountains blocks your path. You cannot proceed that way."
        }
        "water" => "Dark water stretches before you. Without a boat, passage is impossible.",
        _ => "The terrain ahead is impassable. You cannot proceed that way.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_and_default_messages() {
        assert!(blocked_message("mountains").contains("mountains"));
        assert!(blocked_message("water").contains("water"));
        assert_eq!(
            blocked_message("lava"),
            "The terrain ahead is impassable. You cannot proceed that way."
        );
    }
}
