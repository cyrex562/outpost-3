//! Legacy → grammar migration.
//!
//! Ported from `src/harsh_realm/content/legacy.py`.
//!
//! Converts category-grouped pack YAML records (e.g. `creatures:`, `items:`)
//! into form-2 grammar documents (single record-type-keyed maps) consumed by
//! the compiler.

use std::collections::HashMap;

use serde_json::Value;

use crate::payloads::JsonObject;

/// Pack category → the grammar record-type key it migrates to.
fn category_to_record() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("creatures", "creature");
    m.insert("items", "item");
    m.insert("status_effects", "status_effect");
    m.insert("tags", "tag");
    m.insert("traits", "trait");
    m
}

/// Categories the migrator can handle.
pub const MIGRATABLE_CATEGORIES: &[&str] = &["creatures", "items", "status_effects", "tags", "traits"];

fn default_attack_for_skill(skill: &str) -> &'static str {
    match skill {
        "punch" => "strike",
        "stab" => "stab_attack",
        _ => "strike",
    }
}

/// The outcome of porting a batch of legacy records.
#[derive(Debug, Default, Clone)]
pub struct MigrationResult {
    pub documents: Vec<JsonObject>,
    /// Referenced IDs (e.g. actions) that now need their own records.
    pub needs: Vec<String>,
    pub notes: Vec<String>,
}

impl MigrationResult {
    /// Render the grammar documents as a multi-doc YAML string.
    pub fn to_yaml(&self) -> String {
        let chunks: Vec<String> = self
            .documents
            .iter()
            .map(|doc| {
                serde_yaml::to_string(doc)
                    .unwrap_or_default()
                    .trim_end()
                    .to_string()
            })
            .collect();
        if chunks.is_empty() {
            String::new()
        } else {
            chunks.join("\n---\n") + "\n"
        }
    }
}

fn strip_loader_fields(record: &JsonObject) -> JsonObject {
    record
        .iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn migrate_creature(legacy: &JsonObject) -> (JsonObject, Vec<String>, Vec<String>) {
    let mut needs: Vec<String> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut body = strip_loader_fields(legacy);
    let creature_id = body.get("id").and_then(|v| v.as_str()).unwrap_or("?").to_string();

    let innate = body.remove("innate_attacks").unwrap_or(Value::Null);
    let attack_skill = body.remove("attack_skill");

    let mut actions: Vec<String> = match &innate {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    };

    if actions.is_empty() {
        if let Some(Value::String(skill)) = &attack_skill {
            let default = default_attack_for_skill(skill);
            notes.push(format!(
                "creature '{creature_id}': attack_skill '{skill}' → action '{default}'."
            ));
            actions.push(default.to_string());
        }
    }

    if actions.is_empty() {
        notes.push(format!("creature '{creature_id}': no attack — add at least one action."));
    }

    needs.extend(actions.iter().cloned());
    body.insert("actions".into(), Value::Array(actions.into_iter().map(Value::String).collect()));

    let mut doc = JsonObject::new();
    doc.insert("creature".into(), Value::Object(body));
    (doc, needs, notes)
}

fn migrate_passthrough(record_key: &str, legacy: &JsonObject) -> (JsonObject, Vec<String>, Vec<String>) {
    let body = strip_loader_fields(legacy);
    let mut doc = JsonObject::new();
    doc.insert(record_key.into(), Value::Object(body));
    (doc, vec![], vec![])
}

fn flatten_legacy(category: &str, records: &[Value]) -> Vec<JsonObject> {
    let mut items: Vec<JsonObject> = Vec::new();
    for record in records {
        let record = match record.as_object() {
            Some(m) => m,
            None => continue,
        };
        if let Some(Value::Array(grouped)) = record.get(category) {
            for item in grouped {
                if let Some(obj) = item.as_object() {
                    items.push(obj.clone());
                }
            }
            continue;
        }
        if record.contains_key("id") {
            items.push(record.clone());
            continue;
        }
        let lists: Vec<&Vec<Value>> = record
            .values()
            .filter_map(|v| v.as_array())
            .collect();
        if lists.len() == 1 {
            for item in lists[0] {
                if let Some(obj) = item.as_object() {
                    items.push(obj.clone());
                }
            }
        } else {
            items.push(record.clone());
        }
    }
    items
}

/// Port a batch of legacy records from one pack category to grammar documents.
pub fn migrate_records(category: &str, records: &[Value]) -> MigrationResult {
    let map = category_to_record();
    let record_key = match map.get(category) {
        Some(k) => *k,
        None => {
            return MigrationResult {
                notes: vec![format!("No migration is defined for category '{category}' yet.")],
                ..Default::default()
            };
        }
    };

    let mut result = MigrationResult::default();
    for record in flatten_legacy(category, records) {
        let (document, needs, notes) = if record_key == "creature" {
            migrate_creature(&record)
        } else {
            migrate_passthrough(record_key, &record)
        };
        result.documents.push(document);
        result.needs.extend(needs);
        result.notes.extend(notes);
    }

    // De-duplicate needs, preserving order.
    let mut seen = std::collections::HashSet::new();
    result.needs.retain(|n| seen.insert(n.clone()));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrate_creature_with_innate_attacks() {
        let legacy: JsonObject = serde_json::from_value(json!({
            "id": "wolf",
            "hp": 8,
            "innate_attacks": ["bite"]
        })).unwrap();
        let (doc, needs, notes) = migrate_creature(&legacy);
        assert!(doc.contains_key("creature"));
        let body = doc["creature"].as_object().unwrap();
        assert_eq!(body["actions"], json!(["bite"]));
        assert!(needs.contains(&"bite".to_string()));
        assert!(notes.is_empty());
    }

    #[test]
    fn migrate_creature_with_attack_skill() {
        let legacy: JsonObject = serde_json::from_value(json!({
            "id": "goblin",
            "attack_skill": "stab"
        })).unwrap();
        let (doc, needs, notes) = migrate_creature(&legacy);
        let body = doc["creature"].as_object().unwrap();
        assert_eq!(body["actions"], json!(["stab_attack"]));
        assert!(needs.contains(&"stab_attack".to_string()));
        assert!(!notes.is_empty());
    }

    #[test]
    fn migrate_creature_no_attack() {
        let legacy: JsonObject = serde_json::from_value(json!({"id": "slug"})).unwrap();
        let (_, _, notes) = migrate_creature(&legacy);
        assert!(notes.iter().any(|n| n.contains("no attack")));
    }

    #[test]
    fn migrate_passthrough_item() {
        let legacy: JsonObject = serde_json::from_value(json!({"id": "sword", "damage": "1d8"})).unwrap();
        let (doc, needs, notes) = migrate_passthrough("item", &legacy);
        assert!(doc.contains_key("item"));
        assert!(needs.is_empty());
        assert!(notes.is_empty());
    }

    #[test]
    fn migrate_records_unknown_category() {
        let result = migrate_records("spells", &[]);
        assert!(!result.notes.is_empty());
        assert!(result.notes[0].contains("No migration"));
    }

    #[test]
    fn migrate_records_creatures_grouped() {
        let records = vec![json!({
            "creatures": [
                {"id": "rat", "innate_attacks": ["bite"]},
                {"id": "bat", "innate_attacks": ["claw"]}
            ]
        })];
        let result = migrate_records("creatures", &records);
        assert_eq!(result.documents.len(), 2);
        // Needs de-duplicated — bite and claw distinct.
        assert_eq!(result.needs.len(), 2);
    }

    #[test]
    fn migrate_records_deduplicates_needs() {
        let records = vec![
            json!({"creatures": [
                {"id": "wolf1", "innate_attacks": ["bite"]},
                {"id": "wolf2", "innate_attacks": ["bite"]}
            ]})
        ];
        let result = migrate_records("creatures", &records);
        // "bite" appears twice but needs should be deduplicated to 1.
        assert_eq!(result.needs, vec!["bite"]);
    }

    #[test]
    fn strip_loader_fields_removes_underscore_keys() {
        let record: JsonObject = serde_json::from_value(json!({
            "id": "x",
            "_pack_id": "xwn-core",
            "_qualified_id": "xwn-core/x"
        })).unwrap();
        let stripped = strip_loader_fields(&record);
        assert!(!stripped.contains_key("_pack_id"));
        assert!(stripped.contains_key("id"));
    }
}
