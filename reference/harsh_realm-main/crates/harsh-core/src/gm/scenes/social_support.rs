//! Support helpers for the social scene.
//!
//! Ported from `src/harsh_realm/gm/scenes/social_support.py`.

/// Map a disposition score (clamped to -3..=3) to a mood label.
pub fn disposition_label(score: i32) -> &'static str {
    match score.clamp(-3, 3) {
        -3 => "Hostile",
        -2 => "Unsteady",
        -1 => "Guarded",
        0 => "Indifferent",
        1 => "Sociable",
        2 => "Friendly",
        3 => "Helpful",
        _ => "Indifferent",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_clamping() {
        assert_eq!(disposition_label(-3), "Hostile");
        assert_eq!(disposition_label(0), "Indifferent");
        assert_eq!(disposition_label(3), "Helpful");
        // Out of range clamps to the nearest band.
        assert_eq!(disposition_label(-9), "Hostile");
        assert_eq!(disposition_label(9), "Helpful");
    }
}
