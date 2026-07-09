//! Shared constants and helpers for the town scene.
//!
//! Ported from `src/harsh_realm/gm/scenes/_town_support.py`.

use crate::runtime::{JsonObject, JsonValue};
use crate::settlement::SettlementSummary;

/// Whether a town terrain type blocks movement.
pub fn is_impassable_town(terrain: &str) -> bool {
    terrain == "building"
}

/// Normalize a legacy settlement dict into a full [`SettlementSummary`].
///
/// Missing optional fields are defaulted (description empty, `settlement_id`
/// = `"scene"`, empty establishment/resident lists) before validation.
pub fn normalize_settlement_summary(mut payload: JsonObject) -> Result<SettlementSummary, String> {
    payload
        .entry("description".to_string())
        .or_insert_with(|| JsonValue::from(""));
    payload
        .entry("settlement_id".to_string())
        .or_insert_with(|| JsonValue::from("scene"));
    payload
        .entry("establishments".to_string())
        .or_insert_with(|| JsonValue::Array(Vec::new()));
    payload
        .entry("resident_npc_ids".to_string())
        .or_insert_with(|| JsonValue::Array(Vec::new()));
    serde_json::from_value(JsonValue::Object(payload)).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impassable_town_terrain() {
        assert!(is_impassable_town("building"));
        assert!(!is_impassable_town("road"));
    }

    #[test]
    fn normalize_fills_defaults() {
        let mut payload = JsonObject::new();
        payload.insert("name".into(), JsonValue::from("Riverbend"));
        payload.insert("size".into(), JsonValue::from("village"));
        let summary = normalize_settlement_summary(payload).unwrap();
        assert_eq!(summary.name, "Riverbend");
        assert_eq!(summary.settlement_id, "scene");
        assert!(summary.establishments.is_empty());
        assert!(summary.resident_npc_ids.is_empty());
    }

    #[test]
    fn normalize_preserves_existing_id() {
        let mut payload = JsonObject::new();
        payload.insert("name".into(), JsonValue::from("Riverbend"));
        payload.insert("size".into(), JsonValue::from("town"));
        payload.insert("settlement_id".into(), JsonValue::from("ent-1"));
        let summary = normalize_settlement_summary(payload).unwrap();
        assert_eq!(summary.settlement_id, "ent-1");
    }
}
