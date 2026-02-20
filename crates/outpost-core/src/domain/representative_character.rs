//! Representative Characters — named individuals representing colonist population.
//!
//! Rather than simulating every colonist, a small group of representative characters
//! (think "council members" or "key figures") gives the player a human face for the colony.
//! Each character has skills, morale, and optionally an assigned building role.

use crate::domain::ids::BuildingId;
use crate::domain::population::SkillCategory;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a representative character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CharacterId(pub Uuid);

impl CharacterId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for CharacterId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CharacterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A named representative colonist at a site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepresentativeCharacter {
    /// Unique ID.
    pub id: CharacterId,
    /// Display name.
    pub name: String,
    /// Age in years.
    pub age: u8,
    /// Primary skill/profession.
    pub primary_skill: SkillCategory,
    /// Current morale (0–100).
    pub morale: f32,
    /// Current health (0–100).
    pub health: f32,
    /// Building this character is assigned to (if any).
    pub assigned_building_id: Option<BuildingId>,
    /// Personality traits (flavour text).
    pub traits: Vec<String>,
}

impl RepresentativeCharacter {
    pub fn new(
        name: String,
        age: u8,
        primary_skill: SkillCategory,
        traits: Vec<String>,
    ) -> Self {
        Self {
            id: CharacterId::new(),
            name,
            age,
            primary_skill,
            morale: 75.0,
            health: 100.0,
            assigned_building_id: None,
            traits,
        }
    }

    /// Adjust morale, clamped to [0, 100].
    pub fn adjust_morale(&mut self, delta: f32) {
        self.morale = (self.morale + delta).clamp(0.0, 100.0);
    }

    /// Assign to a building.
    pub fn assign_to(&mut self, building_id: BuildingId) {
        self.assigned_building_id = Some(building_id);
    }

    /// Unassign from current building.
    pub fn unassign(&mut self) {
        self.assigned_building_id = None;
    }

    /// Whether this character is currently assigned to a building.
    pub fn is_assigned(&self) -> bool {
        self.assigned_building_id.is_some()
    }
}

// ---------------------------------------------------------------------------
// Procedural Character Generator
// ---------------------------------------------------------------------------

/// First names pool.
const FIRST_NAMES: &[&str] = &[
    "Alara", "Brix", "Cael", "Dara", "Emyr", "Faye", "Garen", "Hessa",
    "Iren", "Jyn", "Kael", "Lyra", "Maren", "Nyx", "Orin", "Pax",
    "Quinn", "Reva", "Soren", "Tira", "Ulric", "Vael", "Wren", "Xara",
    "Yael", "Zora",
];

/// Family names pool.
const FAMILY_NAMES: &[&str] = &[
    "Ashford", "Brennan", "Croft", "Drake", "Evander", "Farrow",
    "Gould", "Harmon", "Inara", "Janos", "Keller", "Larkin", "Marsh",
    "Nakamura", "Okafor", "Patel", "Quinn", "Reinhart", "Saito",
    "Torres", "Upton", "Vance", "Walsh", "Xu", "Yuen", "Zarek",
];

/// Character trait pools.
const POSITIVE_TRAITS: &[&str] = &[
    "Diligent", "Optimistic", "Resourceful", "Team player",
    "Quick learner", "Detail-oriented", "Calm under pressure",
];

const NEGATIVE_TRAITS: &[&str] = &[
    "Impatient", "Stubborn", "Overcautious", "Blunt",
    "Perfectionist", "Risk-averse",
];

/// Simple deterministic pseudo-random number generator (LCG).
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(1) }
    }

    fn next_u64(&mut self) -> u64 {
        // LCG parameters from Knuth
        self.state = self.state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }

    fn next_usize(&mut self, max: usize) -> usize {
        (self.next_u64() as usize) % max
    }

    fn next_u8(&mut self, min: u8, max: u8) -> u8 {
        min + (self.next_u64() as u8 % (max - min + 1))
    }

    fn next_f32(&mut self, min: f32, max: f32) -> f32 {
        let ratio = (self.next_u64() as f32) / (u64::MAX as f32);
        min + ratio * (max - min)
    }
}

/// Generate a set of representative characters for a new site.
///
/// `seed` is typically the site's UUID hash or the current tick.
/// Returns 5–10 named characters with varied skills.
pub fn generate_characters(seed: u64, count: usize) -> Vec<RepresentativeCharacter> {
    let mut rng = SimpleRng::new(seed);
    let count = count.clamp(5, 10);

    // Skill distribution across the group
    let skill_rotation = [
        SkillCategory::Engineer,
        SkillCategory::Laborer,
        SkillCategory::Operator,
        SkillCategory::Farmer,
        SkillCategory::Medic,
        SkillCategory::Scientist,
        SkillCategory::Laborer,
        SkillCategory::Operator,
        SkillCategory::Engineer,
        SkillCategory::Laborer,
    ];

    (0..count)
        .map(|i| {
            let first = FIRST_NAMES[rng.next_usize(FIRST_NAMES.len())];
            let last = FAMILY_NAMES[rng.next_usize(FAMILY_NAMES.len())];
            let name = format!("{} {}", first, last);
            let age = rng.next_u8(22, 58);
            let skill = skill_rotation[i % skill_rotation.len()];

            // Pick 1–2 traits
            let trait_count = rng.next_usize(2) + 1;
            let mut traits = Vec::new();
            if rng.next_u64() % 3 != 0 {
                traits.push(POSITIVE_TRAITS[rng.next_usize(POSITIVE_TRAITS.len())].to_string());
            } else {
                traits.push(NEGATIVE_TRAITS[rng.next_usize(NEGATIVE_TRAITS.len())].to_string());
            }
            if trait_count > 1 {
                traits.push(POSITIVE_TRAITS[rng.next_usize(POSITIVE_TRAITS.len())].to_string());
            }

            let mut character = RepresentativeCharacter::new(name, age, skill, traits);
            // Slight morale variation at start
            character.morale = rng.next_f32(65.0, 85.0);

            character
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_characters_count() {
        let chars = generate_characters(42, 7);
        assert_eq!(chars.len(), 7);
    }

    #[test]
    fn test_generate_characters_clamped() {
        let chars = generate_characters(42, 3); // below minimum
        assert_eq!(chars.len(), 5);
        let chars = generate_characters(42, 20); // above maximum
        assert_eq!(chars.len(), 10);
    }

    #[test]
    fn test_character_morale_in_range() {
        let chars = generate_characters(99, 8);
        for c in &chars {
            assert!(c.morale >= 0.0 && c.morale <= 100.0);
        }
    }

    #[test]
    fn test_character_not_assigned_by_default() {
        let chars = generate_characters(1, 5);
        for c in &chars {
            assert!(!c.is_assigned());
        }
    }

    #[test]
    fn test_assign_and_unassign_character() {
        let mut c = RepresentativeCharacter::new(
            "Test Person".to_string(),
            30,
            SkillCategory::Engineer,
            vec![],
        );
        assert!(!c.is_assigned());

        let building_id = BuildingId::new();
        c.assign_to(building_id);
        assert!(c.is_assigned());
        assert_eq!(c.assigned_building_id, Some(building_id));

        c.unassign();
        assert!(!c.is_assigned());
    }

    #[test]
    fn test_morale_clamped() {
        let mut c = RepresentativeCharacter::new(
            "Test".to_string(), 25, SkillCategory::Laborer, vec![],
        );
        c.adjust_morale(100.0);
        assert_eq!(c.morale, 100.0);
        c.adjust_morale(-200.0);
        assert_eq!(c.morale, 0.0);
    }
}
