//! UNE (Universal NPC Emulator) personality and disposition system.
//!
//! Ported from `src/harsh_realm/engine/npc_personality.py`. Generates NPC
//! personalities, motivations, bearings, and disposition labels from
//! YAML-backed tables. RNG is injected so callers control determinism.

use std::collections::HashMap;
use std::path::PathBuf;

use rand::seq::SliceRandom;
use rand::RngCore;

use crate::npc::{UNEBearing, UNEBearingResult, UNEBearingTable, UNEMotivation, UNEPersonality, UNEScalarTable};
use crate::packs::paths::default_content_dir;

// HR-785: the duplicate lowercase `disposition_label` was removed. The single
// canonical implementation is `gm::scenes::social_support::disposition_label`
// (Title Case), used by the social scene narration and the disposition_change
// notice. This module's version had no production callers.

/// Apply a Mythic chaos factor to a base disposition score, clamped to -3..=3.
pub fn chaos_modified_disposition(base_score: i32, chaos_factor: i32) -> i32 {
    let modifier = if chaos_factor >= 7 {
        -1
    } else if chaos_factor <= 3 {
        1
    } else {
        0
    };
    (base_score + modifier).clamp(-3, 3)
}

/// Generates NPC personalities using the UNE system, caching loaded tables.
#[derive(Debug, Clone)]
pub struct UNEGenerator {
    data_dir: PathBuf,
    scalar_cache: HashMap<String, Vec<String>>,
    bearing_cache: Option<Vec<UNEBearingResult>>,
}

impl UNEGenerator {
    /// Create a generator rooted at `data_dir` (defaults to the content dir).
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        UNEGenerator {
            data_dir: data_dir.unwrap_or_else(default_content_dir),
            scalar_cache: HashMap::new(),
            bearing_cache: None,
        }
    }

    fn ensure_scalar(&mut self, table_name: &str) {
        if self.scalar_cache.contains_key(table_name) {
            return;
        }
        let path = self
            .data_dir
            .join("tables")
            .join("npc")
            .join(format!("{table_name}.yaml"));
        let values = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_yaml::from_str::<UNEScalarTable>(&t).ok())
            .map(|doc| doc.entries.into_iter().map(|e| e.result).collect())
            .unwrap_or_default();
        self.scalar_cache.insert(table_name.to_string(), values);
    }

    fn ensure_bearing(&mut self) {
        if self.bearing_cache.is_some() {
            return;
        }
        let path = self
            .data_dir
            .join("tables")
            .join("npc")
            .join("une_bearings.yaml");
        let values = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_yaml::from_str::<UNEBearingTable>(&t).ok())
            .map(|doc| doc.entries.into_iter().map(|e| e.result).collect())
            .unwrap_or_default();
        self.bearing_cache = Some(values);
    }

    fn pick(&mut self, table_name: &str, rng: &mut dyn RngCore) -> String {
        self.ensure_scalar(table_name);
        self.scalar_cache[table_name]
            .choose(rng)
            .cloned()
            .unwrap_or_default()
    }

    /// Generate a full UNE personality profile.
    pub fn generate_personality(
        &mut self,
        power_level: Option<&str>,
        rng: &mut dyn RngCore,
    ) -> UNEPersonality {
        let power_level = match power_level {
            Some(p) => p.to_string(),
            None => self.pick("une_power_level", rng),
        };
        let descriptor = self.pick("une_descriptors", rng);
        let motivation = self.generate_motivation(rng);
        let bearing = self.generate_bearing(5, rng);

        UNEPersonality {
            power_level,
            descriptor,
            motivation_verb: motivation.verb,
            motivation_noun: motivation.noun,
            bearing: bearing.bearing,
            bearing_focus: bearing.focus,
            base_disposition: 0,
        }
    }

    /// Generate a UNE motivation (verb + noun pair).
    pub fn generate_motivation(&mut self, rng: &mut dyn RngCore) -> UNEMotivation {
        let verb = self.pick("une_motivation_verbs", rng);
        let noun = self.pick("une_motivation_nouns", rng);
        UNEMotivation { verb, noun }
    }

    /// Generate a UNE bearing with an optional chaos modifier.
    pub fn generate_bearing(&mut self, chaos_factor: i32, rng: &mut dyn RngCore) -> UNEBearing {
        self.ensure_bearing();
        let items = self.bearing_cache.as_ref().expect("bearing cache populated");
        if items.is_empty() {
            return UNEBearing {
                bearing: String::new(),
                focus: String::new(),
            };
        }
        let pick1 = items.choose(rng).cloned();
        let pick2 = items.choose(rng).cloned();
        // High chaos picks the later (more extreme) roll; otherwise the first.
        let chosen = if chaos_factor >= 7 { pick2 } else { pick1 };
        match chosen {
            Some(r) => UNEBearing {
                bearing: r.bearing,
                focus: r.focus,
            },
            None => UNEBearing {
                bearing: String::new(),
                focus: String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaos_shifts_disposition() {
        assert_eq!(chaos_modified_disposition(0, 8), -1);
        assert_eq!(chaos_modified_disposition(0, 2), 1);
        assert_eq!(chaos_modified_disposition(0, 5), 0);
        assert_eq!(chaos_modified_disposition(3, 2), 3); // clamps
        assert_eq!(chaos_modified_disposition(-3, 8), -3);
    }

    #[test]
    fn empty_tables_yield_blanks() {
        use rand::SeedableRng;
        // No content dir in tests -> empty tables -> blank picks.
        let mut gen = UNEGenerator::new(Some(PathBuf::from("/nonexistent")));
        let mut rng = rand::rngs::StdRng::seed_from_u64(0);
        let p = gen.generate_personality(None, &mut rng);
        assert_eq!(p.power_level, "");
        assert_eq!(p.bearing, "");
    }
}
