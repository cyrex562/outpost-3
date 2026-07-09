//! Typed runtime record models shared across scene handlers and payloads.
//!
//! Ported from `src/harsh_realm/models/runtime.py`. The Python legacy
//! dict-access shims (`__getitem__`/`get`/`to_dict`) are dropped — Rust callers
//! use fields directly and `serde_json` for JSON-safe conversion.

use serde::{Deserialize, Serialize};

/// JSON value, mirroring Python's recursive `JsonValue` alias.
pub type JsonValue = serde_json::Value;
/// JSON object map, mirroring Python's `JsonObject = dict[str, JsonValue]`.
pub type JsonObject = serde_json::Map<String, serde_json::Value>;

fn d_unknown() -> String {
    "Unknown".to_string()
}
fn d_misc() -> String {
    "misc".to_string()
}
fn d_one() -> i32 {
    1
}
fn d_zero_value() -> JsonValue {
    JsonValue::from(0)
}

/// NPC payload used by social, exploration, and town scenes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNpcRecord {
    /// Entity id, when this NPC is persisted.
    #[serde(default)]
    pub entity_id: Option<String>,
    /// Display name.
    #[serde(default = "d_unknown")]
    pub name: String,
    /// Occupation/role.
    #[serde(default)]
    pub occupation: String,
    /// Personality trait words.
    #[serde(default)]
    pub personality_traits: Vec<String>,
    /// Motivation phrase.
    #[serde(default)]
    pub motivation: String,
    /// Appearance description.
    #[serde(default)]
    pub appearance: String,
    /// Greeting line.
    #[serde(default)]
    pub greeting: String,
    /// Owning faction id.
    #[serde(default)]
    pub faction_id: Option<String>,
    /// Disposition (an int or a string label).
    #[serde(default = "d_zero_value")]
    pub disposition: JsonValue,
    /// UNE personality payload.
    #[serde(default)]
    pub une_personality: Option<JsonObject>,
    /// Building id where the NPC is found.
    #[serde(default)]
    pub building_id: Option<String>,
    /// Establishment type at the building.
    #[serde(default)]
    pub establishment_type: Option<String>,
    /// Establishment name at the building.
    #[serde(default)]
    pub establishment_name: Option<String>,
}

/// One shop inventory listing used by the shopping scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ShopItemRecord {
    /// Item name.
    pub name: String,
    /// Item category/type.
    pub r#type: String,
    /// Sale price.
    #[serde(default)]
    pub price: i32,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Canonical item id.
    #[serde(default)]
    pub item_id: Option<String>,
    /// Weapon damage expression.
    #[serde(default)]
    pub weapon_damage: Option<String>,
    /// Shock damage.
    #[serde(default)]
    pub shock_damage: Option<i32>,
    /// Shock AC threshold.
    #[serde(default)]
    pub shock_ac_threshold: Option<i32>,
    /// AC bonus.
    #[serde(default)]
    pub ac_bonus: Option<i32>,
    /// Heal amount (an int or dice string).
    #[serde(default)]
    pub heal: Option<JsonValue>,
}

/// One inventory item carried by a character. Mutable; preserves unknown keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InventoryItemRecord {
    /// Item name.
    pub name: String,
    /// Broad type (default `"misc"`).
    #[serde(default = "d_misc")]
    pub r#type: String,
    /// Specific item type, if any.
    #[serde(default)]
    pub item_type: Option<String>,
    /// Stack quantity (default 1).
    #[serde(default = "d_one")]
    pub quantity: i32,
    /// Intrinsic value.
    #[serde(default)]
    pub value: i32,
    /// Price.
    #[serde(default)]
    pub price: i32,
    /// Description.
    #[serde(default)]
    pub description: String,
    /// Canonical item id.
    #[serde(default)]
    pub item_id: Option<String>,
    /// Weapon damage expression.
    #[serde(default)]
    pub weapon_damage: Option<String>,
    /// Damage expression.
    #[serde(default)]
    pub damage: Option<String>,
    /// Shock damage.
    #[serde(default)]
    pub shock_damage: Option<i32>,
    /// Shock AC threshold.
    #[serde(default)]
    pub shock_ac_threshold: Option<i32>,
    /// Encumbrance.
    #[serde(default)]
    pub enc: Option<f64>,
    /// Armor class.
    #[serde(default)]
    pub ac: Option<i32>,
    /// AC bonus.
    #[serde(default)]
    pub ac_bonus: Option<i32>,
    /// Arbitrary structured data.
    #[serde(default)]
    pub data: JsonObject,
    /// Unknown extra keys preserved across round-trips (Pydantic `extra="allow"`).
    #[serde(flatten)]
    pub extra: JsonObject,
}

impl InventoryItemRecord {
    /// Coin amount represented by this item: `data["amount"]` or `value`.
    pub fn currency_amount(&self) -> i64 {
        self.data
            .get("amount")
            .and_then(|v| v.as_i64())
            .unwrap_or(self.value as i64)
    }
}

/// Current weather conditions in a region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeatherState {
    /// clear, overcast, rain, storm, snow, fog.
    pub condition: String,
    /// freezing, cold, mild, warm, hot.
    pub temperature: String,
    /// Human-readable description.
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_npc_defaults() {
        let npc: SceneNpcRecord = serde_json::from_str("{}").unwrap();
        assert_eq!(npc.name, "Unknown");
        assert_eq!(npc.disposition, JsonValue::from(0));
        assert!(npc.personality_traits.is_empty());
    }

    #[test]
    fn inventory_currency_amount_prefers_data_amount() {
        let mut rec: InventoryItemRecord =
            serde_json::from_str(r#"{"name":"gold","value":5}"#).unwrap();
        assert_eq!(rec.currency_amount(), 5);
        rec.data.insert("amount".into(), JsonValue::from(42));
        assert_eq!(rec.currency_amount(), 42);
    }

    #[test]
    fn inventory_preserves_unknown_keys() {
        let rec: InventoryItemRecord =
            serde_json::from_str(r#"{"name":"x","custom":7}"#).unwrap();
        assert_eq!(rec.extra.get("custom"), Some(&JsonValue::from(7)));
    }
}
