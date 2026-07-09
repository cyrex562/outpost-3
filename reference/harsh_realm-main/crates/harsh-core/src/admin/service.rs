//! AdminService — CRUD + reset for all admin config tables.
//!
//! Ported from `src/harsh_realm/admin/service.py` and
//! `src/harsh_realm/admin/content_mixin.py`.

use std::path::PathBuf;

use serde_json::Value;

use crate::admin::base::NamedJsonRecord;
use crate::admin::config::{
    AdminConfigExport, DifficultyTarget, DispositionOutcome, EncounterWeight, FactionAssetStat,
    RandomTable, SkillMapping, XpProgression,
};
use crate::db::WorldDatabase;
use crate::packs::paths::default_content_dir;
use crate::runtime::JsonObject;

// ---------------------------------------------------------------------------
// Row extraction helpers
// ---------------------------------------------------------------------------

fn get_str(row: &JsonObject, key: &str) -> String {
    row.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn get_i32(row: &JsonObject, key: &str) -> i32 {
    row.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
}

fn get_bool(row: &JsonObject, key: &str) -> bool {
    match row.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

fn get_json_array(row: &JsonObject, key: &str) -> Vec<JsonObject> {
    row.get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

fn get_json_tags(row: &JsonObject, key: &str) -> Vec<String> {
    row.get(key)
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Row → model converters
// ---------------------------------------------------------------------------

fn skill_mapping_from_row(row: &JsonObject) -> SkillMapping {
    SkillMapping {
        verb: get_str(row, "verb"),
        skill: get_str(row, "skill"),
        attribute: get_str(row, "attribute"),
        base_difficulty: get_i32(row, "base_difficulty"),
        opposed: get_bool(row, "opposed"),
        description: get_str(row, "description"),
        notes: get_str(row, "notes"),
    }
}

fn difficulty_target_from_row(row: &JsonObject) -> DifficultyTarget {
    DifficultyTarget {
        name: get_str(row, "name"),
        target: get_i32(row, "target"),
        description: get_str(row, "description"),
    }
}

fn disposition_outcome_from_row(row: &JsonObject) -> DispositionOutcome {
    DispositionOutcome {
        outcome_key: get_str(row, "outcome_key"),
        delta: get_i32(row, "delta"),
        description: get_str(row, "description"),
    }
}

fn encounter_weight_from_row(row: &JsonObject) -> EncounterWeight {
    EncounterWeight {
        faction_disposition: get_str(row, "faction_disposition"),
        encounter_tag: get_str(row, "encounter_tag"),
        weight_modifier: get_i32(row, "weight_modifier"),
    }
}

fn faction_asset_stat_from_row(row: &JsonObject) -> FactionAssetStat {
    FactionAssetStat {
        asset_type: get_str(row, "asset_type"),
        category: get_str(row, "category"),
        min_attribute: get_i32(row, "min_attribute"),
        cost: get_i32(row, "cost"),
        upkeep: get_i32(row, "upkeep"),
        max_hp: {
            let v = get_i32(row, "max_hp");
            if v == 0 { 1 } else { v }
        },
        attack_stat: get_str(row, "attack_stat"),
        counter_stat: get_str(row, "counter_stat"),
        attack_roll: get_str(row, "attack_roll"),
        special: get_str(row, "special"),
        description: get_str(row, "description"),
    }
}

fn xp_progression_from_row(row: &JsonObject) -> XpProgression {
    XpProgression {
        level: get_i32(row, "level"),
        xp_needed: get_i32(row, "xp_needed"),
    }
}

fn random_table_from_row(row: &JsonObject) -> RandomTable {
    RandomTable {
        id: get_str(row, "id"),
        category: get_str(row, "category"),
        name: get_str(row, "name"),
        entries: get_json_array(row, "entries"),
        tags: get_json_tags(row, "tags"),
        source: get_str(row, "source"),
    }
}

fn named_json_from_row(row: &JsonObject) -> NamedJsonRecord {
    let id = get_str(row, "id");
    let name = get_str(row, "name");
    let source = get_str(row, "source");
    let extra: JsonObject = row
        .get("data")
        .and_then(|v| v.as_str())
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    NamedJsonRecord { id, name, source, extra }
}

// ---------------------------------------------------------------------------
// YAML loading helpers
// ---------------------------------------------------------------------------

fn data_dir() -> PathBuf {
    default_content_dir()
}

fn load_yaml_list<T: serde::de::DeserializeOwned>(rel_path: &str) -> Vec<T> {
    let path = data_dir().join(rel_path);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    serde_yaml::from_str::<Vec<T>>(&text).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// AdminService
// ---------------------------------------------------------------------------

/// CRUD and reset operations for all admin config tables.
pub struct AdminService<'db> {
    db: &'db WorldDatabase,
}

impl<'db> AdminService<'db> {
    /// Create a new AdminService backed by the given database.
    pub fn new(db: &'db WorldDatabase) -> Self {
        Self { db }
    }

    // -----------------------------------------------------------------------
    // Skill mappings
    // -----------------------------------------------------------------------

    pub fn list_skill_mappings(&self) -> Result<Vec<SkillMapping>, String> {
        let rows = self.db.fetch_all("SELECT * FROM skill_mappings ORDER BY verb", &[])?;
        Ok(rows.iter().map(skill_mapping_from_row).collect())
    }

    pub fn get_skill_mapping(&self, verb: &str) -> Result<Option<SkillMapping>, String> {
        let row = self
            .db
            .fetch_one("SELECT * FROM skill_mappings WHERE verb = ?", &[&verb])?;
        Ok(row.as_ref().map(skill_mapping_from_row))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set_skill_mapping(
        &self,
        verb: &str,
        skill: &str,
        attribute: &str,
        base_difficulty: i32,
        opposed: bool,
        description: &str,
        notes: &str,
    ) -> Result<SkillMapping, String> {
        self.db.execute(
            "INSERT OR REPLACE INTO skill_mappings \
             (verb, skill, attribute, base_difficulty, opposed, description, notes) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            &[
                &verb,
                &skill,
                &attribute,
                &base_difficulty,
                &(opposed as i32),
                &description,
                &notes,
            ],
        )?;
        Ok(SkillMapping {
            verb: verb.to_string(),
            skill: skill.to_string(),
            attribute: attribute.to_string(),
            base_difficulty,
            opposed,
            description: description.to_string(),
            notes: notes.to_string(),
        })
    }

    pub fn delete_skill_mapping(&self, verb: &str) -> Result<bool, String> {
        let existing = self
            .db
            .fetch_one("SELECT verb FROM skill_mappings WHERE verb = ?", &[&verb])?;
        if existing.is_none() {
            return Ok(false);
        }
        self.db
            .execute("DELETE FROM skill_mappings WHERE verb = ?", &[&verb])?;
        Ok(true)
    }

    pub fn reset_skill_mapping(&self, verb: &str) -> Result<Option<SkillMapping>, String> {
        let defaults: Vec<SkillMapping> = load_yaml_list("skill_mappings.yaml");
        for entry in &defaults {
            if entry.verb == verb {
                return Ok(Some(self.set_skill_mapping(
                    &entry.verb,
                    &entry.skill,
                    &entry.attribute,
                    entry.base_difficulty,
                    entry.opposed,
                    &entry.description,
                    &entry.notes,
                )?));
            }
        }
        Ok(None)
    }

    pub fn reset_all_skill_mappings(&self) -> Result<Vec<SkillMapping>, String> {
        self.db.execute("DELETE FROM skill_mappings", &[])?;
        self.seed_skill_mappings()
    }

    fn seed_skill_mappings(&self) -> Result<Vec<SkillMapping>, String> {
        let defaults: Vec<SkillMapping> = load_yaml_list("skill_mappings.yaml");
        let mut results = Vec::new();
        for entry in &defaults {
            results.push(self.set_skill_mapping(
                &entry.verb,
                &entry.skill,
                &entry.attribute,
                entry.base_difficulty,
                entry.opposed,
                &entry.description,
                &entry.notes,
            )?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Difficulty targets
    // -----------------------------------------------------------------------

    pub fn list_difficulty_targets(&self) -> Result<Vec<DifficultyTarget>, String> {
        let rows = self
            .db
            .fetch_all("SELECT * FROM difficulty_targets ORDER BY target", &[])?;
        Ok(rows.iter().map(difficulty_target_from_row).collect())
    }

    pub fn get_difficulty_target(&self, name: &str) -> Result<Option<DifficultyTarget>, String> {
        let row = self
            .db
            .fetch_one("SELECT * FROM difficulty_targets WHERE name = ?", &[&name])?;
        Ok(row.as_ref().map(difficulty_target_from_row))
    }

    pub fn set_difficulty_target(
        &self,
        name: &str,
        target: i32,
        description: &str,
    ) -> Result<DifficultyTarget, String> {
        let existing = self.get_difficulty_target(name)?;
        let desc = if description.is_empty() {
            existing.as_ref().map(|e| e.description.as_str()).unwrap_or("").to_string()
        } else {
            description.to_string()
        };
        self.db.execute(
            "INSERT OR REPLACE INTO difficulty_targets (name, target, description) VALUES (?, ?, ?)",
            &[&name, &target, &desc.as_str()],
        )?;
        Ok(DifficultyTarget { name: name.to_string(), target, description: desc })
    }

    pub fn reset_difficulty_target(&self, name: &str) -> Result<Option<DifficultyTarget>, String> {
        let defaults: Vec<DifficultyTarget> = load_yaml_list("difficulty_targets.yaml");
        for entry in &defaults {
            if entry.name == name {
                return Ok(Some(self.set_difficulty_target(
                    &entry.name,
                    entry.target,
                    &entry.description,
                )?));
            }
        }
        Ok(None)
    }

    fn seed_difficulty_targets(&self) -> Result<Vec<DifficultyTarget>, String> {
        let defaults: Vec<DifficultyTarget> = load_yaml_list("difficulty_targets.yaml");
        let mut results = Vec::new();
        for entry in &defaults {
            results.push(self.set_difficulty_target(&entry.name, entry.target, &entry.description)?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Disposition outcomes
    // -----------------------------------------------------------------------

    pub fn list_disposition_outcomes(&self) -> Result<Vec<DispositionOutcome>, String> {
        let rows = self
            .db
            .fetch_all("SELECT * FROM disposition_outcomes ORDER BY outcome_key", &[])?;
        Ok(rows.iter().map(disposition_outcome_from_row).collect())
    }

    pub fn get_disposition_outcome(
        &self,
        outcome_key: &str,
    ) -> Result<Option<DispositionOutcome>, String> {
        let row = self.db.fetch_one(
            "SELECT * FROM disposition_outcomes WHERE outcome_key = ?",
            &[&outcome_key],
        )?;
        Ok(row.as_ref().map(disposition_outcome_from_row))
    }

    pub fn set_disposition_outcome(
        &self,
        outcome_key: &str,
        delta: i32,
        description: &str,
    ) -> Result<DispositionOutcome, String> {
        let existing = self.get_disposition_outcome(outcome_key)?;
        let desc = if description.is_empty() {
            existing.as_ref().map(|e| e.description.as_str()).unwrap_or("").to_string()
        } else {
            description.to_string()
        };
        self.db.execute(
            "INSERT OR REPLACE INTO disposition_outcomes (outcome_key, delta, description) VALUES (?, ?, ?)",
            &[&outcome_key, &delta, &desc.as_str()],
        )?;
        Ok(DispositionOutcome { outcome_key: outcome_key.to_string(), delta, description: desc })
    }

    pub fn reset_disposition_outcome(
        &self,
        outcome_key: &str,
    ) -> Result<Option<DispositionOutcome>, String> {
        let defaults: Vec<DispositionOutcome> = load_yaml_list("disposition_outcomes.yaml");
        for entry in &defaults {
            if entry.outcome_key == outcome_key {
                return Ok(Some(self.set_disposition_outcome(
                    &entry.outcome_key,
                    entry.delta,
                    &entry.description,
                )?));
            }
        }
        Ok(None)
    }

    fn seed_disposition_outcomes(&self) -> Result<Vec<DispositionOutcome>, String> {
        let defaults: Vec<DispositionOutcome> = load_yaml_list("disposition_outcomes.yaml");
        let mut results = Vec::new();
        for entry in &defaults {
            results.push(self.set_disposition_outcome(
                &entry.outcome_key,
                entry.delta,
                &entry.description,
            )?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Encounter weights
    // -----------------------------------------------------------------------

    pub fn list_encounter_weights(&self) -> Result<Vec<EncounterWeight>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM encounter_weights ORDER BY faction_disposition, encounter_tag",
            &[],
        )?;
        Ok(rows.iter().map(encounter_weight_from_row).collect())
    }

    pub fn set_encounter_weight(
        &self,
        faction_disposition: &str,
        encounter_tag: &str,
        weight_modifier: i32,
    ) -> Result<EncounterWeight, String> {
        self.db.execute(
            "INSERT OR REPLACE INTO encounter_weights \
             (faction_disposition, encounter_tag, weight_modifier) VALUES (?, ?, ?)",
            &[&faction_disposition, &encounter_tag, &weight_modifier],
        )?;
        Ok(EncounterWeight {
            faction_disposition: faction_disposition.to_string(),
            encounter_tag: encounter_tag.to_string(),
            weight_modifier,
        })
    }

    pub fn reset_encounter_weights(
        &self,
        faction_disposition: Option<&str>,
    ) -> Result<Vec<EncounterWeight>, String> {
        if let Some(fd) = faction_disposition {
            self.db.execute(
                "DELETE FROM encounter_weights WHERE faction_disposition = ?",
                &[&fd],
            )?;
        } else {
            self.db.execute("DELETE FROM encounter_weights", &[])?;
        }
        self.seed_encounter_weights()
    }

    fn seed_encounter_weights(&self) -> Result<Vec<EncounterWeight>, String> {
        let defaults: Vec<EncounterWeight> = load_yaml_list("encounter_weights.yaml");
        let mut results = Vec::new();
        for entry in &defaults {
            results.push(self.set_encounter_weight(
                &entry.faction_disposition,
                &entry.encounter_tag,
                entry.weight_modifier,
            )?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Faction asset stats
    // -----------------------------------------------------------------------

    pub fn list_faction_asset_stats(&self) -> Result<Vec<FactionAssetStat>, String> {
        let rows = self.db.fetch_all(
            "SELECT * FROM faction_asset_stats ORDER BY category, asset_type",
            &[],
        )?;
        Ok(rows.iter().map(faction_asset_stat_from_row).collect())
    }

    pub fn get_faction_asset_stat(
        &self,
        asset_type: &str,
    ) -> Result<Option<FactionAssetStat>, String> {
        let row = self.db.fetch_one(
            "SELECT * FROM faction_asset_stats WHERE asset_type = ?",
            &[&asset_type],
        )?;
        Ok(row.as_ref().map(faction_asset_stat_from_row))
    }

    pub fn set_faction_asset_stat(&self, stat: &FactionAssetStat) -> Result<FactionAssetStat, String> {
        let existing = self.get_faction_asset_stat(&stat.asset_type)?;
        let merged = FactionAssetStat {
            asset_type: stat.asset_type.clone(),
            category: if stat.category.is_empty() {
                existing.as_ref().map(|e| e.category.as_str()).unwrap_or("").to_string()
            } else {
                stat.category.clone()
            },
            min_attribute: if stat.min_attribute < 0 {
                existing.as_ref().map(|e| e.min_attribute).unwrap_or(-1)
            } else {
                stat.min_attribute
            },
            cost: if stat.cost < 0 {
                existing.as_ref().map(|e| e.cost).unwrap_or(-1)
            } else {
                stat.cost
            },
            upkeep: stat.upkeep,
            max_hp: stat.max_hp,
            attack_stat: stat.attack_stat.clone(),
            counter_stat: stat.counter_stat.clone(),
            attack_roll: stat.attack_roll.clone(),
            special: stat.special.clone(),
            description: stat.description.clone(),
        };
        self.db.execute(
            "INSERT OR REPLACE INTO faction_asset_stats \
             (asset_type, category, min_attribute, cost, upkeep, max_hp, \
              attack_stat, counter_stat, attack_roll, special, description) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            &[
                &merged.asset_type.as_str(),
                &merged.category.as_str(),
                &merged.min_attribute,
                &merged.cost,
                &merged.upkeep,
                &merged.max_hp,
                &merged.attack_stat.as_str(),
                &merged.counter_stat.as_str(),
                &merged.attack_roll.as_str(),
                &merged.special.as_str(),
                &merged.description.as_str(),
            ],
        )?;
        Ok(merged)
    }

    pub fn reset_faction_asset_stat(
        &self,
        asset_type: &str,
    ) -> Result<Option<FactionAssetStat>, String> {
        let defaults: Vec<FactionAssetStat> = load_yaml_list("faction_assets.yaml");
        for entry in &defaults {
            if entry.asset_type == asset_type {
                return Ok(Some(self.set_faction_asset_stat(entry)?));
            }
        }
        Ok(None)
    }

    fn seed_faction_asset_stats(&self) -> Result<Vec<FactionAssetStat>, String> {
        let defaults: Vec<FactionAssetStat> = load_yaml_list("faction_assets.yaml");
        let mut results = Vec::new();
        for entry in &defaults {
            results.push(self.set_faction_asset_stat(entry)?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // XP progression
    // -----------------------------------------------------------------------

    pub fn list_xp_progression(&self) -> Result<Vec<XpProgression>, String> {
        let rows = self
            .db
            .fetch_all("SELECT * FROM xp_progression ORDER BY level", &[])?;
        Ok(rows.iter().map(xp_progression_from_row).collect())
    }

    pub fn set_xp_progression(&self, level: i32, xp_needed: i32) -> Result<XpProgression, String> {
        self.db.execute(
            "INSERT OR REPLACE INTO xp_progression (level, xp_needed) VALUES (?, ?)",
            &[&level, &xp_needed],
        )?;
        Ok(XpProgression { level, xp_needed })
    }

    pub fn reset_xp_progression(&self, level: i32) -> Result<Option<XpProgression>, String> {
        let defaults: Vec<XpProgression> = load_yaml_list("tables/xp_progression.yaml");
        for entry in &defaults {
            if entry.level == level {
                return Ok(Some(self.set_xp_progression(entry.level, entry.xp_needed)?));
            }
        }
        Ok(None)
    }

    pub fn reset_all_xp_progression(&self) -> Result<Vec<XpProgression>, String> {
        self.db.execute("DELETE FROM xp_progression", &[])?;
        self.seed_xp_progression()
    }

    fn seed_xp_progression(&self) -> Result<Vec<XpProgression>, String> {
        let defaults: Vec<XpProgression> = load_yaml_list("tables/xp_progression.yaml");
        let mut results = Vec::new();
        for entry in &defaults {
            results.push(self.set_xp_progression(entry.level, entry.xp_needed)?);
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Random tables
    // -----------------------------------------------------------------------

    pub fn list_random_tables(&self) -> Result<Vec<RandomTable>, String> {
        let rows = self
            .db
            .fetch_all("SELECT * FROM random_tables ORDER BY category, name", &[])?;
        Ok(rows.iter().map(random_table_from_row).collect())
    }

    pub fn get_random_table(&self, table_id: &str) -> Result<Option<RandomTable>, String> {
        let row = self
            .db
            .fetch_one("SELECT * FROM random_tables WHERE id = ?", &[&table_id])?;
        Ok(row.as_ref().map(random_table_from_row))
    }

    pub fn set_random_table(
        &self,
        table_id: &str,
        category: &str,
        name: &str,
        entries: &[JsonObject],
        tags: &[String],
        source: &str,
    ) -> Result<RandomTable, String> {
        let entries_json = serde_json::to_string(entries).unwrap_or_default();
        let tags_json = serde_json::to_string(tags).unwrap_or_default();
        self.db.execute(
            "INSERT OR REPLACE INTO random_tables (id, category, name, entries, tags, source) \
             VALUES (?, ?, ?, ?, ?, ?)",
            &[&table_id, &category, &name, &entries_json.as_str(), &tags_json.as_str(), &source],
        )?;
        Ok(RandomTable {
            id: table_id.to_string(),
            category: category.to_string(),
            name: name.to_string(),
            entries: entries.to_vec(),
            tags: tags.to_vec(),
            source: source.to_string(),
        })
    }

    pub fn delete_random_table(&self, table_id: &str) -> Result<bool, String> {
        let existing =
            self.db.fetch_one("SELECT id FROM random_tables WHERE id = ?", &[&table_id])?;
        if existing.is_none() {
            return Ok(false);
        }
        self.db.execute("DELETE FROM random_tables WHERE id = ?", &[&table_id])?;
        Ok(true)
    }

    // -----------------------------------------------------------------------
    // Named JSON records (items, creature_templates)
    // -----------------------------------------------------------------------

    pub fn list_items(&self) -> Result<Vec<NamedJsonRecord>, String> {
        let rows = self.db.fetch_all("SELECT * FROM items ORDER BY name", &[])?;
        Ok(rows.iter().map(named_json_from_row).collect())
    }

    pub fn get_item(&self, item_id: &str) -> Result<Option<NamedJsonRecord>, String> {
        let row = self.db.fetch_one("SELECT * FROM items WHERE id = ?", &[&item_id])?;
        Ok(row.as_ref().map(named_json_from_row))
    }

    pub fn delete_item(&self, item_id: &str) -> Result<bool, String> {
        let existing = self.db.fetch_one("SELECT id FROM items WHERE id = ?", &[&item_id])?;
        if existing.is_none() {
            return Ok(false);
        }
        self.db.execute("DELETE FROM items WHERE id = ?", &[&item_id])?;
        Ok(true)
    }

    pub fn list_creature_templates(&self) -> Result<Vec<NamedJsonRecord>, String> {
        let rows =
            self.db.fetch_all("SELECT * FROM creature_templates ORDER BY name", &[])?;
        Ok(rows.iter().map(named_json_from_row).collect())
    }

    pub fn get_creature_template(&self, creature_id: &str) -> Result<Option<NamedJsonRecord>, String> {
        let row = self.db.fetch_one(
            "SELECT * FROM creature_templates WHERE id = ?",
            &[&creature_id],
        )?;
        Ok(row.as_ref().map(named_json_from_row))
    }

    pub fn delete_creature_template(&self, creature_id: &str) -> Result<bool, String> {
        let existing = self.db.fetch_one(
            "SELECT id FROM creature_templates WHERE id = ?",
            &[&creature_id],
        )?;
        if existing.is_none() {
            return Ok(false);
        }
        self.db
            .execute("DELETE FROM creature_templates WHERE id = ?", &[&creature_id])?;
        Ok(true)
    }

    // -----------------------------------------------------------------------
    // Export / seed
    // -----------------------------------------------------------------------

    pub fn export_world_config(&self) -> Result<AdminConfigExport, String> {
        Ok(AdminConfigExport {
            skill_mappings: self.list_skill_mappings()?,
            difficulty_targets: self.list_difficulty_targets()?,
            disposition_outcomes: self.list_disposition_outcomes()?,
            encounter_weights: self.list_encounter_weights()?,
            faction_asset_stats: self.list_faction_asset_stats()?,
        })
    }

    /// Seed the `difficulty_settings` table to grade 3 (Normal) for every dimension.
    ///
    /// Uses `INSERT OR IGNORE` so existing per-world edits are preserved.
    pub fn seed_difficulty_settings(&self) -> Result<usize, String> {
        let keys = crate::difficulty::dimension_keys();
        let count = keys.len();
        for key in &keys {
            self.db.execute(
                "INSERT OR IGNORE INTO difficulty_settings (key, grade) VALUES (?, ?)",
                &[key, &(crate::difficulty::NORMAL_GRADE as i64)],
            )?;
        }
        Ok(count)
    }

    pub fn seed_all_from_yaml(&self) -> Result<std::collections::HashMap<String, usize>, String> {
        let mut counts = std::collections::HashMap::new();
        counts.insert("skill_mappings".to_string(), self.seed_skill_mappings()?.len());
        counts.insert("difficulty_settings".to_string(), self.seed_difficulty_settings()?);
        counts.insert("difficulty_targets".to_string(), self.seed_difficulty_targets()?.len());
        counts.insert(
            "disposition_outcomes".to_string(),
            self.seed_disposition_outcomes()?.len(),
        );
        counts.insert(
            "encounter_weights".to_string(),
            self.seed_encounter_weights()?.len(),
        );
        counts.insert(
            "faction_asset_stats".to_string(),
            self.seed_faction_asset_stats()?.len(),
        );
        counts.insert("xp_progression".to_string(), self.seed_xp_progression()?.len());
        Ok(counts)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_db() -> WorldDatabase {
        WorldDatabase::open_in_memory().expect("in-memory db")
    }

    #[test]
    fn set_and_get_skill_mapping() {
        let db = make_db();
        let svc = AdminService::new(&db);
        let sm = svc
            .set_skill_mapping("persuade", "persuasion", "CHA", 8, false, "desc", "")
            .unwrap();
        assert_eq!(sm.verb, "persuade");
        assert_eq!(sm.skill, "persuasion");
        let got = svc.get_skill_mapping("persuade").unwrap().unwrap();
        assert_eq!(got.attribute, "CHA");
        assert!(!got.opposed);
    }

    #[test]
    fn delete_skill_mapping() {
        let db = make_db();
        let svc = AdminService::new(&db);
        svc.set_skill_mapping("lie", "deception", "CHA", 9, false, "", "").unwrap();
        assert!(svc.delete_skill_mapping("lie").unwrap());
        assert!(!svc.delete_skill_mapping("lie").unwrap());
        assert!(svc.get_skill_mapping("lie").unwrap().is_none());
    }

    #[test]
    fn set_and_list_difficulty_targets() {
        let db = make_db();
        let svc = AdminService::new(&db);
        svc.set_difficulty_target("easy", 6, "Easy task").unwrap();
        svc.set_difficulty_target("hard", 12, "Hard task").unwrap();
        let list = svc.list_difficulty_targets().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "easy");
        assert_eq!(list[1].target, 12);
    }

    #[test]
    fn set_and_list_disposition_outcomes() {
        let db = make_db();
        let svc = AdminService::new(&db);
        svc.set_disposition_outcome("critical_success", 2, "").unwrap();
        svc.set_disposition_outcome("failure", -1, "").unwrap();
        let list = svc.list_disposition_outcomes().unwrap();
        assert_eq!(list.len(), 2);
        let crit = list.iter().find(|o| o.outcome_key == "critical_success").unwrap();
        assert_eq!(crit.delta, 2);
    }

    #[test]
    fn set_and_list_encounter_weights() {
        let db = make_db();
        let svc = AdminService::new(&db);
        svc.set_encounter_weight("friendly", "bandit", -2).unwrap();
        svc.set_encounter_weight("hostile", "guard", 3).unwrap();
        let list = svc.list_encounter_weights().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn export_world_config_empty() {
        let db = make_db();
        let svc = AdminService::new(&db);
        let export = svc.export_world_config().unwrap();
        assert!(export.skill_mappings.is_empty());
        assert!(export.difficulty_targets.is_empty());
        assert!(export.disposition_outcomes.is_empty());
    }

    #[test]
    fn xp_progression_set_and_list() {
        let db = make_db();
        let svc = AdminService::new(&db);
        svc.set_xp_progression(1, 0).unwrap();
        svc.set_xp_progression(2, 1500).unwrap();
        let list = svc.list_xp_progression().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].level, 1);
        assert_eq!(list[1].xp_needed, 1500);
    }

    #[test]
    fn random_table_crud() {
        let db = make_db();
        let svc = AdminService::new(&db);
        svc.set_random_table("t1", "encounters", "Test Table", &[], &[], "test.yaml")
            .unwrap();
        let got = svc.get_random_table("t1").unwrap().unwrap();
        assert_eq!(got.id, "t1");
        assert_eq!(got.category, "encounters");
        assert!(svc.delete_random_table("t1").unwrap());
        assert!(!svc.delete_random_table("t1").unwrap());
    }

    #[test]
    fn seed_difficulty_settings_seeds_all_seven_keys_at_grade_3() {
        let db = make_db();
        let svc = AdminService::new(&db);
        let n = svc.seed_difficulty_settings().unwrap();
        assert_eq!(n, 7);
        // Verify each key is stored at grade 3 (Normal).
        let rows = db
            .fetch_all("SELECT key, grade FROM difficulty_settings ORDER BY key", &[])
            .unwrap();
        assert_eq!(rows.len(), 7);
        for row in &rows {
            let grade = row.get("grade").and_then(|v| v.as_i64()).unwrap_or(-1);
            assert_eq!(grade, 3, "key {} not at grade 3", row.get("key").and_then(|v| v.as_str()).unwrap_or("?"));
        }
    }

    #[test]
    fn seed_difficulty_settings_is_idempotent() {
        let db = make_db();
        let svc = AdminService::new(&db);
        svc.seed_difficulty_settings().unwrap();
        // Second call: no changes because INSERT OR IGNORE.
        svc.seed_difficulty_settings().unwrap();
        let rows = db
            .fetch_all("SELECT key FROM difficulty_settings", &[])
            .unwrap();
        assert_eq!(rows.len(), 7, "duplicate rows after double seed");
    }
}
