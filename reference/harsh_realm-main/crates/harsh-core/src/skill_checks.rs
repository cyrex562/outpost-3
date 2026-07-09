//! Unified skill check resolution engine.
//!
//! Ported from `src/harsh_realm/engine/skill_checks.py`. Reads `skill_mappings`,
//! `difficulty_targets`, and `disposition_outcomes` from the DB and resolves a
//! verb into a [`SkillCheckResult`], with special handling for `intimidate`
//! (always costs disposition) and `deceive` (graduated failure). The pure roll
//! math lives in [`crate::resolvers::skill_check`].

use std::collections::BTreeMap;

use rand::Rng;

use crate::admin::config::{DispositionOutcome, SkillMapping};
use crate::character::Character;
use crate::db::WorldDatabase;
use crate::engine_results::SkillCheckResult;
use crate::repositories::skill_config::SkillConfigRepository;
use crate::resolvers::skill_check::{classify_margin, resolve_skill_check};

/// Resolves skill checks using DB-stored config tables.
pub struct SkillCheckResolver<'a> {
    config: SkillConfigRepository<'a>,
    mappings_cache: Option<BTreeMap<String, SkillMapping>>,
    outcomes_cache: Option<BTreeMap<String, DispositionOutcome>>,
}

impl<'a> SkillCheckResolver<'a> {
    /// Create a resolver over a world database with populated config tables.
    pub fn new(db: &'a WorldDatabase) -> Self {
        SkillCheckResolver {
            config: SkillConfigRepository::new(db),
            mappings_cache: None,
            outcomes_cache: None,
        }
    }

    /// Clear cached config data (call after admin changes).
    pub fn invalidate_cache(&mut self) {
        self.mappings_cache = None;
        self.outcomes_cache = None;
    }

    fn load_mappings(&mut self) -> Result<&BTreeMap<String, SkillMapping>, String> {
        if self.mappings_cache.is_none() {
            self.mappings_cache = Some(self.config.load_skill_mappings()?);
        }
        Ok(self.mappings_cache.as_ref().expect("just populated"))
    }

    fn load_outcomes(&mut self) -> Result<&BTreeMap<String, DispositionOutcome>, String> {
        if self.outcomes_cache.is_none() {
            self.outcomes_cache = Some(self.config.load_disposition_outcomes()?);
        }
        Ok(self.outcomes_cache.as_ref().expect("just populated"))
    }

    /// Get the skill mapping for a verb.
    pub fn get_mapping(&mut self, verb: &str) -> Result<Option<SkillMapping>, String> {
        Ok(self.load_mappings()?.get(verb).cloned())
    }

    /// Resolve a skill check for a verb.
    pub fn resolve<R: Rng>(
        &mut self,
        verb: &str,
        character: &Character,
        difficulty_override: Option<i32>,
        npc_wis_mod: i32,
        modifier_total: i32,
        rng: &mut R,
    ) -> Result<SkillCheckResult, String> {
        let mapping = self
            .load_mappings()?
            .get(verb)
            .cloned()
            .unwrap_or_else(|| default_mapping(verb));

        let mut difficulty = difficulty_override.unwrap_or(mapping.base_difficulty);
        if mapping.opposed && npc_wis_mod != 0 {
            difficulty += npc_wis_mod;
        }

        let skill_level = character
            .skills
            .get(&mapping.skill.to_lowercase())
            .copied()
            .unwrap_or(-1);
        let attr_mod = character
            .attr_mods
            .get(&mapping.attribute.to_lowercase())
            .copied()
            .unwrap_or(0);

        let roll = rng.gen_range(1..=6) + rng.gen_range(1..=6);
        let outcome = resolve_skill_check(
            roll as i64,
            skill_level as i64,
            (attr_mod + modifier_total) as i64,
            difficulty as i64,
        );

        let outcomes = self.load_outcomes()?.clone();
        let disposition_delta = compute_disposition_delta(
            verb,
            outcome.margin as i32,
            outcome.success,
            &outcome.outcome_key,
            &outcomes,
        );

        let narration = build_narration(
            verb,
            &mapping,
            roll,
            outcome.total as i32,
            difficulty,
            outcome.margin as i32,
            outcome.success,
        );

        Ok(SkillCheckResult {
            verb: verb.to_string(),
            skill: mapping.skill,
            attribute: mapping.attribute,
            roll,
            modifier: outcome.modifier as i32,
            total: outcome.total as i32,
            difficulty,
            margin: outcome.margin as i32,
            success: outcome.success,
            outcome_key: outcome.outcome_key,
            disposition_delta,
            narration,
        })
    }

    /// Resolve a check and, if an Expert class fails, perform a reroll.
    ///
    /// Returns `(original, reroll)`; `reroll` is `None` when not applicable
    /// (non-Expert, original succeeded, or reroll already used).
    pub fn resolve_with_reroll<R: Rng>(
        &mut self,
        verb: &str,
        character: &Character,
        difficulty_override: Option<i32>,
        rng: &mut R,
    ) -> Result<(SkillCheckResult, Option<SkillCheckResult>), String> {
        let result = self.resolve(verb, character, difficulty_override, 0, 0, rng)?;
        if result.success {
            return Ok((result, None));
        }
        if character.character_class != "expert" {
            return Ok((result, None));
        }
        let reroll_available = character
            .class_abilities
            .get("expert_reroll_available")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if !reroll_available {
            return Ok((result, None));
        }
        let reroll = self.resolve(verb, character, difficulty_override, 0, 0, rng)?;
        Ok((result, Some(reroll)))
    }
}

/// Classify a skill check margin into an outcome band (re-exported for parity).
pub fn classify_margin_i32(margin: i32) -> &'static str {
    classify_margin(margin as i64)
}

fn default_mapping(verb: &str) -> SkillMapping {
    SkillMapping {
        verb: verb.to_string(),
        skill: "Talk".to_string(),
        attribute: "CHA".to_string(),
        base_difficulty: 8,
        opposed: false,
        description: String::new(),
        notes: String::new(),
    }
}

fn compute_disposition_delta(
    verb: &str,
    margin: i32,
    success: bool,
    outcome_key: &str,
    outcomes: &BTreeMap<String, DispositionOutcome>,
) -> i32 {
    // Intimidate: always costs disposition on success (overrides band).
    if verb == "intimidate" && success {
        return outcomes.get("intimidate_success").map_or(-1, |o| o.delta);
    }
    // Deceive: graduated failure.
    if verb == "deceive" && !success {
        return deceive_failure_delta(margin, outcomes);
    }
    // Standard: use outcome band.
    outcomes.get(outcome_key).map_or(0, |o| o.delta)
}

fn deceive_failure_delta(margin: i32, outcomes: &BTreeMap<String, DispositionOutcome>) -> i32 {
    let abs_margin = margin.abs();
    if abs_margin >= 4 {
        outcomes.get("deceive_caught").map_or(-3, |o| o.delta)
    } else if abs_margin >= 2 {
        -2
    } else {
        -1
    }
}

fn build_narration(
    verb: &str,
    mapping: &SkillMapping,
    roll: i32,
    total: i32,
    difficulty: i32,
    margin: i32,
    success: bool,
) -> String {
    let outcome_word = if success { "Success" } else { "Failure" };
    let margin_str = if margin >= 0 {
        format!("+{margin}")
    } else {
        margin.to_string()
    };
    format!(
        "{} check ({}/{}): rolled {} + {} = {} vs {}. **{}** (margin {}).",
        capitalize(verb),
        mapping.skill,
        mapping.attribute,
        roll,
        total - roll,
        total,
        difficulty,
        outcome_word,
        margin_str
    )
}

/// Capitalize like Python `str.capitalize`: first char upper, rest lower.
fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn seed_world() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        // Seed one skill mapping and a couple of disposition outcomes.
        db.execute(
            "INSERT INTO skill_mappings (verb, skill, attribute, base_difficulty, opposed) \
             VALUES (?, ?, ?, ?, ?)",
            &[&"convince", &"Talk", &"cha", &8_i64, &0_i64],
        )
        .unwrap();
        db.execute(
            "INSERT INTO disposition_outcomes (outcome_key, delta, description) VALUES (?, ?, ?)",
            &[&"solid_success", &1_i64, &""],
        )
        .unwrap();
        db.execute(
            "INSERT INTO disposition_outcomes (outcome_key, delta, description) VALUES (?, ?, ?)",
            &[&"intimidate_success", &-2_i64, &""],
        )
        .unwrap();
        db
    }

    fn character() -> Character {
        let mut c = Character::default();
        c.attr_mods.insert("cha".into(), 1);
        c.attr_mods.insert("str".into(), 2);
        c.skills.insert("talk".into(), 1);
        c
    }

    #[test]
    fn resolves_known_verb_with_mapping() {
        let db = seed_world();
        let mut resolver = SkillCheckResolver::new(&db);
        let mut rng = SmallRng::seed_from_u64(3);
        let result = resolver
            .resolve("convince", &character(), None, 0, 0, &mut rng)
            .unwrap();
        assert_eq!(result.skill, "Talk");
        assert_eq!(result.attribute, "cha");
        // modifier = skill_level(1) + attr_mod(1) = 2
        assert_eq!(result.modifier, 2);
        assert_eq!(result.total, result.roll + 2);
        assert!(result.narration.starts_with("Convince check (Talk/cha)"));
    }

    #[test]
    fn unknown_verb_uses_defaults() {
        let db = seed_world();
        let mut resolver = SkillCheckResolver::new(&db);
        let mut rng = SmallRng::seed_from_u64(1);
        let result = resolver
            .resolve("ponder", &character(), None, 0, 0, &mut rng)
            .unwrap();
        assert_eq!(result.skill, "Talk");
        assert_eq!(result.attribute, "CHA");
        assert_eq!(result.difficulty, 8);
    }

    #[test]
    fn intimidate_success_costs_disposition() {
        let outcomes = {
            let mut m = BTreeMap::new();
            m.insert(
                "intimidate_success".to_string(),
                DispositionOutcome {
                    outcome_key: "intimidate_success".into(),
                    delta: -2,
                    description: String::new(),
                },
            );
            m
        };
        assert_eq!(
            compute_disposition_delta("intimidate", 5, true, "exceptional_success", &outcomes),
            -2
        );
        // No table entry -> default -1.
        assert_eq!(
            compute_disposition_delta("intimidate", 5, true, "x", &BTreeMap::new()),
            -1
        );
    }

    #[test]
    fn deceive_failure_is_graduated() {
        let empty = BTreeMap::new();
        assert_eq!(compute_disposition_delta("deceive", -1, false, "failure", &empty), -1);
        assert_eq!(compute_disposition_delta("deceive", -2, false, "failure", &empty), -2);
        assert_eq!(compute_disposition_delta("deceive", -4, false, "failure", &empty), -3);
    }

    #[test]
    fn capitalize_matches_python() {
        assert_eq!(capitalize("intimidate"), "Intimidate");
        assert_eq!(capitalize("CONVINCE"), "Convince");
        assert_eq!(capitalize(""), "");
    }
}
