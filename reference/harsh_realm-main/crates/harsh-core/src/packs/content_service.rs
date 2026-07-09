//! Override-aware content reads for the active world.
//!
//! Ported from `src/harsh_realm/packs/content_service.py`. Async DB reads become
//! synchronous (`rusqlite`); methods return `Result` because the override lookup
//! touches the database.

use crate::repositories::world_pack::WorldPackRepository;
use crate::runtime::{JsonObject, JsonValue};

use super::registry::{parse_qualified_id, PackRegistry};

/// Override-aware content reads for the active world.
pub struct ContentService<'a> {
    registry: &'a PackRegistry,
    repo: WorldPackRepository<'a>,
}

impl<'a> ContentService<'a> {
    /// Create a content service from an active pack registry and repository.
    pub fn new(registry: &'a PackRegistry, repo: WorldPackRepository<'a>) -> Self {
        ContentService { registry, repo }
    }

    /// Return a content record, preferring a world override when present.
    pub fn get(&self, qualified_id: &str) -> Result<Option<JsonObject>, String> {
        let Some((pack_id, category, slug)) = parse_qualified_id(qualified_id) else {
            return Ok(None);
        };
        let pack_record = self.registry.get_record(qualified_id);
        let override_record = self.repo.get_override(&pack_id, qualified_id)?;
        match override_record {
            None => Ok(pack_record),
            Some(override_record) => Ok(Some(merge_override(
                pack_record,
                override_record,
                &pack_id,
                &category,
                &slug,
                qualified_id,
            ))),
        }
    }

    /// Return all records in a category with overrides applied.
    pub fn list_records(&self, category: &str) -> Result<Vec<JsonObject>, String> {
        let mut records: Vec<JsonObject> = Vec::new();
        for record in self.registry.list_records(category, None) {
            let qualified_id = record.get("_qualified_id").and_then(|v| v.as_str());
            let pack_id = record.get("_pack_id").and_then(|v| v.as_str());
            let slug = record.get("_slug").and_then(|v| v.as_str());
            let (Some(qualified_id), Some(pack_id), Some(slug)) = (qualified_id, pack_id, slug)
            else {
                records.push(record);
                continue;
            };
            let qualified_id = qualified_id.to_string();
            let pack_id = pack_id.to_string();
            let slug = slug.to_string();
            match self.repo.get_override(&pack_id, &qualified_id)? {
                None => records.push(record),
                Some(override_record) => records.push(merge_override(
                    Some(record),
                    override_record,
                    &pack_id,
                    category,
                    &slug,
                    &qualified_id,
                )),
            }
        }
        Ok(records)
    }

    /// Return whether a qualified record has a world override.
    pub fn has_override(&self, qualified_id: &str) -> Result<bool, String> {
        let Some((pack_id, _category, _slug)) = parse_qualified_id(qualified_id) else {
            return Ok(false);
        };
        Ok(self.repo.get_override(&pack_id, qualified_id)?.is_some())
    }
}

fn merge_override(
    pack_record: Option<JsonObject>,
    override_record: JsonObject,
    pack_id: &str,
    category: &str,
    slug: &str,
    qualified_id: &str,
) -> JsonObject {
    let mut record = pack_record.unwrap_or_default();
    for (key, value) in override_record {
        record.insert(key, value);
    }
    record.insert("_pack_id".into(), JsonValue::String(pack_id.to_string()));
    record.insert("_category".into(), JsonValue::String(category.to_string()));
    record.insert("_slug".into(), JsonValue::String(slug.to_string()));
    record.insert(
        "_qualified_id".into(),
        JsonValue::String(qualified_id.to_string()),
    );
    record.insert("_overridden".into(), JsonValue::Bool(true));
    record
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::packs::loader::{Pack, PackRecords};
    use crate::packs::manifest::PackManifest;
    use std::collections::BTreeMap;

    fn registry_with_record() -> PackRegistry {
        let mut records: PackRecords = PackRecords::new();
        let mut cat: BTreeMap<String, JsonObject> = BTreeMap::new();
        let mut rec = JsonObject::new();
        rec.insert("name".into(), serde_json::json!("Original"));
        rec.insert("power".into(), serde_json::json!(1));
        rec.insert("_pack_id".into(), serde_json::json!("xwn"));
        rec.insert("_category".into(), serde_json::json!("skills"));
        rec.insert("_slug".into(), serde_json::json!("stab"));
        rec.insert("_qualified_id".into(), serde_json::json!("xwn:skills.stab"));
        cat.insert("stab".to_string(), rec);
        records.insert("skills".to_string(), cat);
        let pack = Pack {
            manifest: PackManifest {
                id: "xwn".into(),
                version: "1.0.0".into(),
                name: "xwn".into(),
                description: String::new(),
                authors: vec![],
                depends: vec![],
                conflicts: vec![],
                provides: vec![],
            },
            root_path: std::path::PathBuf::from("/tmp"),
            records,
        };
        PackRegistry::new(vec![pack]).unwrap()
    }

    #[test]
    fn get_returns_pack_record_without_override() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let registry = registry_with_record();
        let svc = ContentService::new(&registry, WorldPackRepository::new(&db));
        let rec = svc.get("xwn:skills.stab").unwrap().unwrap();
        assert_eq!(rec.get("name").unwrap().as_str(), Some("Original"));
        assert!(rec.get("_overridden").is_none());
        assert!(svc.get("bad-id").unwrap().is_none());
    }

    #[test]
    fn get_merges_override() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let registry = registry_with_record();
        let svc = ContentService::new(&registry, WorldPackRepository::new(&db));
        let mut override_record = JsonObject::new();
        override_record.insert("name".into(), serde_json::json!("Tweaked"));
        WorldPackRepository::new(&db)
            .set_override("xwn", "xwn:skills.stab", &override_record)
            .unwrap();

        assert!(svc.has_override("xwn:skills.stab").unwrap());
        let rec = svc.get("xwn:skills.stab").unwrap().unwrap();
        assert_eq!(rec.get("name").unwrap().as_str(), Some("Tweaked"));
        assert_eq!(rec.get("power").unwrap().as_i64(), Some(1)); // preserved
        assert_eq!(rec.get("_overridden").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn list_records_applies_overrides() {
        let db = WorldDatabase::open_in_memory().unwrap();
        let registry = registry_with_record();
        let svc = ContentService::new(&registry, WorldPackRepository::new(&db));
        let mut override_record = JsonObject::new();
        override_record.insert("name".into(), serde_json::json!("Tweaked"));
        WorldPackRepository::new(&db)
            .set_override("xwn", "xwn:skills.stab", &override_record)
            .unwrap();
        let records = svc.list_records("skills").unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get("name").unwrap().as_str(), Some("Tweaked"));
    }
}
