//! System-agnostic class advancement, loaded from content.
//!
//! Ported from `src/harsh_realm/engine/class_progression.py`. The progression
//! (hit die, per-level attack bonus, skill points per level) is data authored in
//! `classes.yaml`. Unlike the Python module-level cache, this loads on demand;
//! a cache can be reintroduced once the content service is ported.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::packs::paths::default_content_dir;

/// One class's advancement table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassProgression {
    /// Hit die size.
    pub hit_die: i32,
    /// Skill points granted per level.
    pub skill_points_per_level: i32,
    /// Attack bonus indexed by (level - 1).
    pub attack_bonus_by_level: Vec<i32>,
}

impl ClassProgression {
    /// Attack bonus at `level` (clamped to the authored table).
    pub fn attack_bonus(&self, level: i32) -> i32 {
        if self.attack_bonus_by_level.is_empty() {
            return 0;
        }
        let max_index = self.attack_bonus_by_level.len() as i32 - 1;
        let index = (level - 1).clamp(0, max_index) as usize;
        self.attack_bonus_by_level[index]
    }
}

/// Fallback for an unknown class (legacy default: ceil(level/2), d6).
pub fn default_progression() -> ClassProgression {
    ClassProgression {
        hit_die: 6,
        skill_points_per_level: 1,
        attack_bonus_by_level: vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5],
    }
}

fn d_six() -> i32 {
    6
}
fn d_one() -> i32 {
    1
}

#[derive(Debug, Deserialize)]
struct ClassEntry {
    id: String,
    #[serde(default = "d_six")]
    hit_die: i32,
    #[serde(default = "d_one")]
    skill_points_per_level: i32,
    #[serde(default)]
    attack_bonus_by_level: Vec<i32>,
}

#[derive(Debug, Deserialize, Default)]
struct ClassesDocument {
    #[serde(default)]
    classes: Vec<ClassEntry>,
}

/// Load the class progression table from `classes.yaml` under the content dir.
/// Returns an empty map when the file is missing or unparseable.
pub fn load_progressions() -> BTreeMap<String, ClassProgression> {
    let path = default_content_dir().join("classes.yaml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Ok(doc) = serde_yaml::from_str::<ClassesDocument>(&text) else {
        return BTreeMap::new();
    };
    doc.classes
        .into_iter()
        .map(|e| {
            (
                e.id.to_lowercase(),
                ClassProgression {
                    hit_die: e.hit_die,
                    skill_points_per_level: e.skill_points_per_level,
                    attack_bonus_by_level: e.attack_bonus_by_level,
                },
            )
        })
        .collect()
}

/// Return the progression for a class, or the default for unknown classes.
pub fn get_progression(character_class: &str) -> ClassProgression {
    load_progressions()
        .remove(&character_class.to_lowercase())
        .unwrap_or_else(default_progression)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_bonus_clamps() {
        let p = default_progression();
        assert_eq!(p.attack_bonus(1), 1);
        assert_eq!(p.attack_bonus(10), 5);
        assert_eq!(p.attack_bonus(99), 5); // clamps to last
        assert_eq!(p.attack_bonus(0), 1); // clamps to first
    }

    #[test]
    fn empty_table_attack_bonus_is_zero() {
        let p = ClassProgression {
            hit_die: 6,
            skill_points_per_level: 1,
            attack_bonus_by_level: vec![],
        };
        assert_eq!(p.attack_bonus(3), 0);
    }

    #[test]
    fn unknown_class_returns_default() {
        // No content dir in tests -> empty map -> default.
        let p = get_progression("nonexistent-class-xyz");
        assert_eq!(p, default_progression());
    }
}
