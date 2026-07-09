//! Adjustable difficulty: per-world grade tables, resolved profile, and metadata.
//!
//! Seven independent dimensions (HR-107). Each dimension has 7 grades, 0..=6,
//! where **grade 3 is Normal** (identity — ×1.0 or +0). Grade 0 is always the
//! easier end, grade 6 the harder end, regardless of the underlying direction.
//! All tables are explicit constants — no interpolation — to keep them transparent
//! and unit-testable.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Grade→value tables (7 entries each, index == grade)
// ---------------------------------------------------------------------------

/// Enemy max-HP multiplier per grade.
pub const ENEMY_HP_GRADES: [f64; 7] = [0.5, 0.7, 0.85, 1.0, 1.2, 1.5, 2.0];
/// Player max-HP multiplier per grade.
pub const PLAYER_HP_GRADES: [f64; 7] = [1.6, 1.4, 1.2, 1.0, 0.9, 0.8, 0.7];
/// XP multiplier per grade.
pub const XP_GRADES: [f64; 7] = [1.5, 1.3, 1.15, 1.0, 0.9, 0.8, 0.7];
/// Player-to-hit additive modifier per grade.
pub const PLAYER_TO_HIT_GRADES: [i32; 7] = [3, 2, 1, 0, -1, -2, -3];
/// Enemy-to-hit additive modifier per grade.
pub const ENEMY_TO_HIT_GRADES: [i32; 7] = [-3, -2, -1, 0, 1, 2, 3];
/// Loot-amount multiplier per grade.
pub const LOOT_AMOUNT_GRADES: [f64; 7] = [2.0, 1.6, 1.3, 1.0, 0.85, 0.7, 0.5];
/// Loot-probability multiplier per grade.
pub const LOOT_PROBABILITY_GRADES: [f64; 7] = [1.6, 1.4, 1.2, 1.0, 0.85, 0.7, 0.5];

/// The grade that corresponds to Normal (identity) for every dimension.
pub const NORMAL_GRADE: i32 = 3;
/// Total number of grades per dimension.
pub const GRADE_COUNT: i32 = 7;

// ---------------------------------------------------------------------------
// Resolved profile
// ---------------------------------------------------------------------------

/// All difficulty scalars resolved from the stored grade map.
///
/// `Default` returns the all-Normal (identity) profile: every multiplier is
/// 1.0 and every modifier is 0. Any code path without a database handle can
/// use `DifficultyProfile::default()` and behave as vanilla XWN.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyProfile {
    /// Multiplier applied to enemy max HP at spawn.
    pub enemy_hp_mult: f64,
    /// Multiplier applied to the player's base max HP.
    pub player_hp_mult: f64,
    /// Multiplier applied to XP earned per encounter.
    pub xp_mult: f64,
    /// Additive modifier on player attack rolls vs enemies.
    pub player_to_hit_mod: i32,
    /// Additive modifier on enemy attack rolls vs the player.
    pub enemy_to_hit_mod: i32,
    /// Multiplier on loot item count and gold totals.
    pub loot_amount_mult: f64,
    /// Multiplier on loot drop probability (scales the "nothing" weight).
    pub loot_probability_mult: f64,
}

impl Default for DifficultyProfile {
    fn default() -> Self {
        DifficultyProfile {
            enemy_hp_mult: 1.0,
            player_hp_mult: 1.0,
            xp_mult: 1.0,
            player_to_hit_mod: 0,
            enemy_to_hit_mod: 0,
            loot_amount_mult: 1.0,
            loot_probability_mult: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Resolution
// ---------------------------------------------------------------------------

/// Clamp a grade to the valid range 0..=6.
fn clamp_grade(grade: i32) -> usize {
    grade.clamp(0, GRADE_COUNT - 1) as usize
}

/// Map stored grades (key → grade 0..=6) to a [`DifficultyProfile`].
///
/// Missing keys fall back to `NORMAL_GRADE` (3). Out-of-range grades are
/// clamped to 0..=6.
pub fn resolve(grades: &BTreeMap<String, i32>) -> DifficultyProfile {
    let g = |key: &str| -> usize { clamp_grade(grades.get(key).copied().unwrap_or(NORMAL_GRADE)) };
    DifficultyProfile {
        enemy_hp_mult: ENEMY_HP_GRADES[g("enemy_hp")],
        player_hp_mult: PLAYER_HP_GRADES[g("player_hp")],
        xp_mult: XP_GRADES[g("xp")],
        player_to_hit_mod: PLAYER_TO_HIT_GRADES[g("player_to_hit")],
        enemy_to_hit_mod: ENEMY_TO_HIT_GRADES[g("enemy_to_hit")],
        loot_amount_mult: LOOT_AMOUNT_GRADES[g("loot_amount")],
        loot_probability_mult: LOOT_PROBABILITY_GRADES[g("loot_probability")],
    }
}

// ---------------------------------------------------------------------------
// Dimension metadata
// ---------------------------------------------------------------------------

/// Static metadata describing a single difficulty dimension.
#[derive(Debug, Clone)]
pub struct DimensionMeta {
    /// Stable DB key.
    pub key: &'static str,
    /// Human-readable label.
    pub label: &'static str,
    /// Tooltip / description.
    pub description: &'static str,
    /// Label for grade 0 (easiest end).
    pub easy_label: &'static str,
    /// Label for grade 6 (hardest end).
    pub hard_label: &'static str,
}

/// Ordered list of all dimension metadata (insertion order = UI order).
pub fn all_dimension_meta() -> Vec<DimensionMeta> {
    vec![
        DimensionMeta {
            key: "enemy_hp",
            label: "Enemy HP",
            description: "Tougher enemies survive longer.",
            easy_label: "Weakest",
            hard_label: "Toughest",
        },
        DimensionMeta {
            key: "player_hp",
            label: "Player HP",
            description: "Increases or reduces your base max HP.",
            easy_label: "Heartiest",
            hard_label: "Frailest",
        },
        DimensionMeta {
            key: "xp",
            label: "XP Rate",
            description: "Faster or slower leveling.",
            easy_label: "Fast",
            hard_label: "Slow",
        },
        DimensionMeta {
            key: "player_to_hit",
            label: "Player Accuracy",
            description: "Your attack rolls hit more or less easily.",
            easy_label: "Deadly",
            hard_label: "Fumbling",
        },
        DimensionMeta {
            key: "enemy_to_hit",
            label: "Enemy Accuracy",
            description: "Enemy attacks hit more or less easily.",
            easy_label: "Inaccurate",
            hard_label: "Deadly",
        },
        DimensionMeta {
            key: "loot_amount",
            label: "Loot Amount",
            description: "Quantity of items and gold from defeated enemies.",
            easy_label: "Plentiful",
            hard_label: "Scarce",
        },
        DimensionMeta {
            key: "loot_probability",
            label: "Loot Chance",
            description: "Probability of enemies dropping loot.",
            easy_label: "Generous",
            hard_label: "Miserly",
        },
    ]
}

/// Return the ordered list of all dimension keys.
pub fn dimension_keys() -> Vec<&'static str> {
    all_dimension_meta().into_iter().map(|m| m.key).collect()
}

// ---------------------------------------------------------------------------
// Effect display formatting
// ---------------------------------------------------------------------------

fn format_mult(v: f64) -> String {
    // Show up to 4 significant decimal places, stripping trailing zeros.
    let raw = format!("{v:.4}");
    let trimmed = raw.trim_end_matches('0').trim_end_matches('.');
    format!("×{trimmed}")
}

fn format_mod(v: i32) -> String {
    match v.cmp(&0) {
        std::cmp::Ordering::Greater => format!("+{v}"),
        std::cmp::Ordering::Less => format!("{v}"),
        std::cmp::Ordering::Equal => "0".to_string(),
    }
}

/// Human-readable display of the resolved value at `grade` for `key`.
///
/// Multiplier dimensions: "×1.5", "×0.85", etc.
/// Modifier dimensions: "+2", "-3", "0".
pub fn current_effect(key: &str, grade: i32) -> String {
    let g = clamp_grade(grade);
    match key {
        "enemy_hp" => format_mult(ENEMY_HP_GRADES[g]),
        "player_hp" => format_mult(PLAYER_HP_GRADES[g]),
        "xp" => format_mult(XP_GRADES[g]),
        "player_to_hit" => format_mod(PLAYER_TO_HIT_GRADES[g]),
        "enemy_to_hit" => format_mod(ENEMY_TO_HIT_GRADES[g]),
        "loot_amount" => format_mult(LOOT_AMOUNT_GRADES[g]),
        "loot_probability" => format_mult(LOOT_PROBABILITY_GRADES[g]),
        _ => "?".to_string(),
    }
}

// ---------------------------------------------------------------------------
// API response types (shared with harsh-web)
// ---------------------------------------------------------------------------

/// A single difficulty dimension as returned by `GET /api/difficulty`.
///
/// Self-describing so the frontend renders generically without hardcoding keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultyDimension {
    /// Stable DB key, e.g. `"enemy_hp"`.
    pub key: String,
    /// Human-readable label, e.g. `"Enemy HP"`.
    pub label: String,
    /// Tooltip description.
    pub description: String,
    /// Current stored grade (0..=6).
    pub grade: i32,
    /// Total number of grades (always 7).
    pub grade_count: i32,
    /// Label for grade 0.
    pub easy_label: String,
    /// Label for grade 6.
    pub hard_label: String,
    /// Grade index of Normal identity (always 3).
    pub normal_grade: i32,
    /// Human-readable resolved value at `grade`, e.g. `"×1.5"` or `"+2"`.
    pub current_effect: String,
}

impl DifficultyDimension {
    /// Build from static metadata + current grade.
    pub fn from_meta(meta: &DimensionMeta, grade: i32) -> Self {
        DifficultyDimension {
            key: meta.key.to_string(),
            label: meta.label.to_string(),
            description: meta.description.to_string(),
            grade,
            grade_count: GRADE_COUNT,
            easy_label: meta.easy_label.to_string(),
            hard_label: meta.hard_label.to_string(),
            normal_grade: NORMAL_GRADE,
            current_effect: current_effect(meta.key, grade),
        }
    }
}

/// The full `GET /api/difficulty` response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DifficultySettingsResponse {
    /// All dimensions, in UI order.
    pub dimensions: Vec<DifficultyDimension>,
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- grade tables -------------------------------------------------------

    #[test]
    fn grade_tables_have_seven_entries() {
        assert_eq!(ENEMY_HP_GRADES.len(), 7);
        assert_eq!(PLAYER_HP_GRADES.len(), 7);
        assert_eq!(XP_GRADES.len(), 7);
        assert_eq!(PLAYER_TO_HIT_GRADES.len(), 7);
        assert_eq!(ENEMY_TO_HIT_GRADES.len(), 7);
        assert_eq!(LOOT_AMOUNT_GRADES.len(), 7);
        assert_eq!(LOOT_PROBABILITY_GRADES.len(), 7);
    }

    #[test]
    fn grade_3_is_identity_for_all_dimensions() {
        assert_eq!(ENEMY_HP_GRADES[3], 1.0);
        assert_eq!(PLAYER_HP_GRADES[3], 1.0);
        assert_eq!(XP_GRADES[3], 1.0);
        assert_eq!(PLAYER_TO_HIT_GRADES[3], 0);
        assert_eq!(ENEMY_TO_HIT_GRADES[3], 0);
        assert_eq!(LOOT_AMOUNT_GRADES[3], 1.0);
        assert_eq!(LOOT_PROBABILITY_GRADES[3], 1.0);
    }

    #[test]
    fn enemy_hp_grade_table_exact_values() {
        assert_eq!(ENEMY_HP_GRADES[0], 0.5);
        assert_eq!(ENEMY_HP_GRADES[3], 1.0);
        assert_eq!(ENEMY_HP_GRADES[6], 2.0);
    }

    #[test]
    fn player_hp_grade_table_exact_values() {
        assert_eq!(PLAYER_HP_GRADES[0], 1.6);
        assert_eq!(PLAYER_HP_GRADES[3], 1.0);
        assert_eq!(PLAYER_HP_GRADES[6], 0.7);
    }

    #[test]
    fn xp_grade_table_exact_values() {
        assert_eq!(XP_GRADES[0], 1.5);
        assert_eq!(XP_GRADES[3], 1.0);
        assert_eq!(XP_GRADES[6], 0.7);
    }

    #[test]
    fn player_to_hit_grade_table_exact_values() {
        assert_eq!(PLAYER_TO_HIT_GRADES[0], 3);
        assert_eq!(PLAYER_TO_HIT_GRADES[3], 0);
        assert_eq!(PLAYER_TO_HIT_GRADES[6], -3);
    }

    #[test]
    fn enemy_to_hit_grade_table_exact_values() {
        assert_eq!(ENEMY_TO_HIT_GRADES[0], -3);
        assert_eq!(ENEMY_TO_HIT_GRADES[3], 0);
        assert_eq!(ENEMY_TO_HIT_GRADES[6], 3);
    }

    #[test]
    fn loot_amount_grade_table_exact_values() {
        assert_eq!(LOOT_AMOUNT_GRADES[0], 2.0);
        assert_eq!(LOOT_AMOUNT_GRADES[3], 1.0);
        assert_eq!(LOOT_AMOUNT_GRADES[6], 0.5);
    }

    #[test]
    fn loot_probability_grade_table_exact_values() {
        assert_eq!(LOOT_PROBABILITY_GRADES[0], 1.6);
        assert_eq!(LOOT_PROBABILITY_GRADES[3], 1.0);
        assert_eq!(LOOT_PROBABILITY_GRADES[6], 0.5);
    }

    // ---- resolve ------------------------------------------------------------

    #[test]
    fn default_profile_is_identity() {
        let profile = DifficultyProfile::default();
        assert_eq!(profile.enemy_hp_mult, 1.0);
        assert_eq!(profile.player_hp_mult, 1.0);
        assert_eq!(profile.xp_mult, 1.0);
        assert_eq!(profile.player_to_hit_mod, 0);
        assert_eq!(profile.enemy_to_hit_mod, 0);
        assert_eq!(profile.loot_amount_mult, 1.0);
        assert_eq!(profile.loot_probability_mult, 1.0);
    }

    #[test]
    fn resolve_empty_grades_returns_identity() {
        let grades = BTreeMap::new();
        let profile = resolve(&grades);
        assert_eq!(profile, DifficultyProfile::default());
    }

    #[test]
    fn resolve_grade_3_returns_identity() {
        let mut grades = BTreeMap::new();
        for key in dimension_keys() {
            grades.insert(key.to_string(), 3);
        }
        let profile = resolve(&grades);
        assert_eq!(profile, DifficultyProfile::default());
    }

    #[test]
    fn resolve_grade_0_returns_easy_values() {
        let mut grades = BTreeMap::new();
        for key in dimension_keys() {
            grades.insert(key.to_string(), 0);
        }
        let profile = resolve(&grades);
        assert_eq!(profile.enemy_hp_mult, 0.5);
        assert_eq!(profile.player_hp_mult, 1.6);
        assert_eq!(profile.xp_mult, 1.5);
        assert_eq!(profile.player_to_hit_mod, 3);
        assert_eq!(profile.enemy_to_hit_mod, -3);
        assert_eq!(profile.loot_amount_mult, 2.0);
        assert_eq!(profile.loot_probability_mult, 1.6);
    }

    #[test]
    fn resolve_grade_6_returns_hard_values() {
        let mut grades = BTreeMap::new();
        for key in dimension_keys() {
            grades.insert(key.to_string(), 6);
        }
        let profile = resolve(&grades);
        assert_eq!(profile.enemy_hp_mult, 2.0);
        assert_eq!(profile.player_hp_mult, 0.7);
        assert_eq!(profile.xp_mult, 0.7);
        assert_eq!(profile.player_to_hit_mod, -3);
        assert_eq!(profile.enemy_to_hit_mod, 3);
        assert_eq!(profile.loot_amount_mult, 0.5);
        assert_eq!(profile.loot_probability_mult, 0.5);
    }

    #[test]
    fn resolve_out_of_range_grades_clamp() {
        let mut grades_low = BTreeMap::new();
        let mut grades_high = BTreeMap::new();
        for key in dimension_keys() {
            grades_low.insert(key.to_string(), -99);
            grades_high.insert(key.to_string(), 999);
        }
        let profile_low = resolve(&grades_low);
        let profile_high = resolve(&grades_high);
        // Clamped to grade 0 (easy end).
        assert_eq!(profile_low.enemy_hp_mult, ENEMY_HP_GRADES[0]);
        // Clamped to grade 6 (hard end).
        assert_eq!(profile_high.enemy_hp_mult, ENEMY_HP_GRADES[6]);
    }

    // ---- metadata -----------------------------------------------------------

    #[test]
    fn dimension_keys_has_seven_entries() {
        assert_eq!(dimension_keys().len(), 7);
    }

    #[test]
    fn all_dimension_meta_keys_are_unique() {
        let keys = dimension_keys();
        let mut seen = std::collections::HashSet::new();
        for k in &keys {
            assert!(seen.insert(k), "duplicate key: {k}");
        }
    }

    // ---- current_effect formatting ------------------------------------------

    #[test]
    fn effect_grade_3_is_identity_display() {
        assert_eq!(current_effect("enemy_hp", 3), "×1");
        assert_eq!(current_effect("player_hp", 3), "×1");
        assert_eq!(current_effect("xp", 3), "×1");
        assert_eq!(current_effect("player_to_hit", 3), "0");
        assert_eq!(current_effect("enemy_to_hit", 3), "0");
        assert_eq!(current_effect("loot_amount", 3), "×1");
        assert_eq!(current_effect("loot_probability", 3), "×1");
    }

    #[test]
    fn effect_modifier_positive_has_plus_sign() {
        assert_eq!(current_effect("player_to_hit", 0), "+3");
        assert_eq!(current_effect("enemy_to_hit", 6), "+3");
    }

    #[test]
    fn effect_modifier_negative_has_minus_sign() {
        assert_eq!(current_effect("player_to_hit", 6), "-3");
        assert_eq!(current_effect("enemy_to_hit", 0), "-3");
    }

    #[test]
    fn effect_mult_strips_trailing_zeros() {
        // 2.0 → "×2", 0.5 → "×0.5", 0.85 → "×0.85"
        assert_eq!(current_effect("enemy_hp", 6), "×2");
        assert_eq!(current_effect("enemy_hp", 0), "×0.5");
        assert_eq!(current_effect("enemy_hp", 2), "×0.85");
    }
}
