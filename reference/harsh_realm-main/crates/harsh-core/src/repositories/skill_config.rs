//! Repository helpers for skill-check configuration tables.
//!
//! Ported from `src/harsh_realm/engine/skill_config_repository.py`.

use std::collections::BTreeMap;

use crate::admin::config::{DispositionOutcome, SkillMapping};
use crate::db::{Row, WorldDatabase};

/// Centralizes read access to skill and disposition config tables.
pub struct SkillConfigRepository<'a> {
    db: &'a WorldDatabase,
}

fn rs(row: &Row, key: &str) -> String {
    row.get(key).and_then(|v| v.as_str()).unwrap_or_default().to_string()
}
fn ri(row: &Row, key: &str) -> i64 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0)
}

impl<'a> SkillConfigRepository<'a> {
    /// Create a repository over a world database.
    pub fn new(db: &'a WorldDatabase) -> Self {
        SkillConfigRepository { db }
    }

    /// Return all verb-to-skill mappings keyed by verb.
    pub fn load_skill_mappings(&self) -> Result<BTreeMap<String, SkillMapping>, String> {
        let rows = self.db.fetch_all("SELECT * FROM skill_mappings", &[])?;
        let mut out = BTreeMap::new();
        for row in &rows {
            let verb = rs(row, "verb");
            out.insert(
                verb.clone(),
                SkillMapping {
                    verb,
                    skill: rs(row, "skill"),
                    attribute: rs(row, "attribute"),
                    base_difficulty: ri(row, "base_difficulty") as i32,
                    opposed: ri(row, "opposed") != 0,
                    description: rs(row, "description"),
                    notes: rs(row, "notes"),
                },
            );
        }
        Ok(out)
    }

    /// Return all disposition outcomes keyed by outcome key.
    pub fn load_disposition_outcomes(&self) -> Result<BTreeMap<String, DispositionOutcome>, String> {
        let rows = self.db.fetch_all("SELECT * FROM disposition_outcomes", &[])?;
        let mut out = BTreeMap::new();
        for row in &rows {
            let key = rs(row, "outcome_key");
            out.insert(
                key.clone(),
                DispositionOutcome {
                    outcome_key: key,
                    delta: ri(row, "delta") as i32,
                    description: rs(row, "description"),
                },
            );
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_config_tables() {
        let db = WorldDatabase::open_in_memory().unwrap();
        db.execute(
            "INSERT INTO skill_mappings (verb, skill, attribute, base_difficulty, opposed) VALUES ('convince','Talk','cha',8,1)",
            &[],
        )
        .unwrap();
        db.execute(
            "INSERT INTO disposition_outcomes (outcome_key, delta, description) VALUES ('solid_success', 1, 'good')",
            &[],
        )
        .unwrap();
        let repo = SkillConfigRepository::new(&db);
        let mappings = repo.load_skill_mappings().unwrap();
        assert_eq!(mappings["convince"].skill, "Talk");
        assert!(mappings["convince"].opposed);
        let outcomes = repo.load_disposition_outcomes().unwrap();
        assert_eq!(outcomes["solid_success"].delta, 1);
    }
}
