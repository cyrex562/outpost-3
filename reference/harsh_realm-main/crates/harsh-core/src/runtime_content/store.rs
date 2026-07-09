//! [`RuntimeContentStore`] — a typed, id-keyed projection of compiled IR records.
//!
//! Built from a `Vec<ComponentRecord>` (the compiler's output) or read back from
//! the per-world `ir_records` table. It serves typed lookups to the trigger
//! runtime and implements [`ContentLookup`] so the status-effect service can load
//! status definitions straight from authored content.

use std::collections::HashMap;

use serde_json::Value;

use crate::content::ir_store::IRRecordRepository;
use crate::ir::{
    Action, ComponentRecord, Creature, Item, Procedure, Skill, StatusEffect, Table, Tag, Trait,
    Trigger,
};
use crate::runtime::JsonObject;
use crate::status_effects::service::ContentLookup;

/// In-memory, read-optimized projection of compiled IR content.
#[derive(Debug, Clone, Default)]
pub struct RuntimeContentStore {
    traits: HashMap<String, Trait>,
    tags: HashMap<String, Tag>,
    statuses: HashMap<String, StatusEffect>,
    items: HashMap<String, Item>,
    creatures: HashMap<String, Creature>,
    actions: HashMap<String, Action>,
    skills: HashMap<String, Skill>,
    procedures: HashMap<String, Procedure>,
    tables: HashMap<String, Table>,
    /// Standalone `trigger` records (global subscriptions) — parsed but not yet
    /// fired this increment (sourcing is per owning record).
    standalone_triggers: HashMap<String, Trigger>,
}

impl RuntimeContentStore {
    /// Project a set of compiled records into typed lookups. Later records with a
    /// duplicate `(type, id)` win (last-wins), matching the ir_records upsert.
    pub fn from_records(records: Vec<ComponentRecord>) -> Self {
        let mut s = Self::default();
        for record in records {
            match record {
                ComponentRecord::Trait(t) => {
                    s.traits.insert(t.id.clone(), t);
                }
                ComponentRecord::Tag(t) => {
                    s.tags.insert(t.id.clone(), t);
                }
                ComponentRecord::StatusEffect(se) => {
                    s.statuses.insert(se.id.clone(), se);
                }
                ComponentRecord::Trigger(tr) => {
                    s.standalone_triggers.insert(tr.id.clone(), tr);
                }
                ComponentRecord::Procedure(p) => {
                    s.procedures.insert(p.id.clone(), p);
                }
                ComponentRecord::Table(t) => {
                    s.tables.insert(t.id.clone(), t);
                }
                ComponentRecord::Action(a) => {
                    s.actions.insert(a.id.clone(), a);
                }
                ComponentRecord::Skill(sk) => {
                    s.skills.insert(sk.id.clone(), sk);
                }
                ComponentRecord::Creature(c) => {
                    s.creatures.insert(c.id.clone(), c);
                }
                ComponentRecord::Item(i) => {
                    s.items.insert(i.id.clone(), i);
                }
            }
        }
        s
    }

    /// Load every record from the per-world `ir_records` table. A row that fails
    /// to deserialize into a [`ComponentRecord`] is collected into the returned
    /// error list (with its type/id) and skipped — load never aborts.
    pub fn from_repository(repo: &IRRecordRepository<'_>) -> Result<(Self, Vec<String>), String> {
        let rows = repo.list_records(None)?;
        let mut records = Vec::with_capacity(rows.len());
        let mut errors = Vec::new();
        for row in rows {
            match serde_json::from_value::<ComponentRecord>(Value::Object(row.clone())) {
                Ok(record) => records.push(record),
                Err(e) => {
                    let ct = row
                        .get("component_type")
                        .and_then(Value::as_str)
                        .unwrap_or("<unknown>");
                    let id = row.get("id").and_then(Value::as_str).unwrap_or("<unknown>");
                    errors.push(format!("skipped ir_record {ct}/{id}: {e}"));
                }
            }
        }
        Ok((Self::from_records(records), errors))
    }

    /// Look up a status effect by id.
    pub fn get_status_effect(&self, id: &str) -> Option<&StatusEffect> {
        self.statuses.get(id)
    }

    /// Look up an item by id.
    pub fn get_item(&self, id: &str) -> Option<&Item> {
        self.items.get(id)
    }

    /// All IR items in the store (for folding into the live item catalog).
    pub fn items(&self) -> impl Iterator<Item = &Item> + '_ {
        self.items.values()
    }

    /// Look up a creature by id.
    pub fn get_creature(&self, id: &str) -> Option<&Creature> {
        self.creatures.get(id)
    }

    /// All IR creatures in the store (for folding into the live creature catalog).
    pub fn creatures(&self) -> impl Iterator<Item = &Creature> + '_ {
        self.creatures.values()
    }

    /// Look up a trait by id.
    pub fn get_trait(&self, id: &str) -> Option<&Trait> {
        self.traits.get(id)
    }

    /// Look up an action by id.
    pub fn get_action(&self, id: &str) -> Option<&Action> {
        self.actions.get(id)
    }

    /// Standalone (global) `trigger` records subscribing to `event_type`, sorted
    /// by id for deterministic firing order. These fire by event type alone —
    /// they are not carried by the acting entity (rooms/objects/world reactions).
    pub fn standalone_triggers_on(&self, event_type: &str) -> Vec<&Trigger> {
        let mut out: Vec<&Trigger> = self
            .standalone_triggers
            .values()
            .filter(|t| t.on == event_type)
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// `true` when no records are loaded (used to guard the runtime hook so
    /// existing un-populated worlds are untouched).
    pub fn is_empty(&self) -> bool {
        self.traits.is_empty()
            && self.tags.is_empty()
            && self.statuses.is_empty()
            && self.items.is_empty()
            && self.creatures.is_empty()
            && self.actions.is_empty()
            && self.skills.is_empty()
            && self.procedures.is_empty()
            && self.tables.is_empty()
            && self.standalone_triggers.is_empty()
    }
}

/// Serve status-effect content to the status service. Implemented on the
/// reference so the store keeps a single owner while the service borrows it.
impl ContentLookup for &RuntimeContentStore {
    fn get(&self, id: &str) -> Option<JsonObject> {
        let effect = self.statuses.get(id)?;
        match serde_json::to_value(effect) {
            Ok(Value::Object(map)) => Some(map),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use serde_json::json;

    fn records_from(values: Vec<Value>) -> Vec<ComponentRecord> {
        values
            .into_iter()
            .map(|v| serde_json::from_value(v).unwrap())
            .collect()
    }

    fn sample() -> Vec<ComponentRecord> {
        records_from(vec![
            json!({"component_type": "creature", "id": "scrip_hound", "name": "Scrip-Hound",
                   "hd": 2, "ac": 14, "attack_bonus": 3, "damage": "1d8"}),
            json!({"component_type": "item", "id": "envenomed_dagger", "name": "Envenomed Dagger",
                   "damage": "1d4", "grants_traits": ["envenom_on_hit"]}),
            json!({"component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
                   "default_duration_ticks": 3, "provides_tags": ["poisoned"], "stacking": "replace"}),
        ])
    }

    #[test]
    fn from_records_indexes_by_type() {
        let store = RuntimeContentStore::from_records(sample());
        assert!(!store.is_empty());
        assert_eq!(store.get_creature("scrip_hound").unwrap().hd, 2);
        assert_eq!(
            store.get_item("envenomed_dagger").unwrap().grants_traits,
            vec!["envenom_on_hit"]
        );
        assert_eq!(store.get_status_effect("poisoned").unwrap().name, "Poisoned");
        assert!(store.get_creature("missing").is_none());
    }

    #[test]
    fn empty_store_is_empty() {
        assert!(RuntimeContentStore::default().is_empty());
    }

    #[test]
    fn content_lookup_returns_status_raw_json() {
        let store = RuntimeContentStore::from_records(sample());
        let store_ref = &store;
        let raw = ContentLookup::get(&store_ref, "poisoned").expect("poisoned present");
        assert_eq!(raw["id"], json!("poisoned"));
        assert_eq!(raw["provides_tags"], json!(["poisoned"]));
        // Round-trips into the status service's own schema.
        let parsed: crate::status_effects::schema::StatusEffect =
            serde_json::from_value(Value::Object(raw)).unwrap();
        assert_eq!(parsed.provides_tags, vec!["poisoned".to_string()]);
        assert!(ContentLookup::get(&store_ref, "not_a_status").is_none());
    }

    fn seeded_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    #[test]
    fn from_repository_loads_and_collects_parse_errors() {
        let db = seeded_db();
        let repo = IRRecordRepository::new(&db);
        // A valid creature plus a malformed one (missing required `damage`).
        let good: JsonObject = serde_json::from_value(json!({
            "component_type": "creature", "id": "scrip_hound", "name": "Scrip-Hound",
            "hd": 2, "ac": 14, "attack_bonus": 3, "damage": "1d8"
        }))
        .unwrap();
        let bad: JsonObject = serde_json::from_value(json!({
            "component_type": "creature", "id": "broken", "name": "Broken"
        }))
        .unwrap();
        repo.upsert_many(&[good, bad], "authored").unwrap();

        let (store, errors) = RuntimeContentStore::from_repository(&repo).unwrap();
        assert!(store.get_creature("scrip_hound").is_some());
        assert!(store.get_creature("broken").is_none());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("creature/broken"), "{:?}", errors);
    }
}
