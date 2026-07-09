//! Item use system for Harsh Realm.
//!
//! Ported from `src/harsh_realm/engine/items.py`.

use rand::Rng;
use serde_json::Value;

use crate::character::Character;
use crate::engine_results::ItemUseResult;
use crate::healing::HealingSystem;
use crate::runtime::{InventoryItemRecord, JsonObject};

fn as_record(raw: &JsonObject) -> Option<InventoryItemRecord> {
    serde_json::from_value(Value::Object(raw.clone())).ok()
}

/// Handles item usage for the player character.
#[derive(Debug, Clone, Default)]
pub struct ItemSystem;

impl ItemSystem {
    /// Find an item by name (exact, then prefix, then substring), returning its
    /// equipment index and the parsed record.
    pub fn find_item_with_index(
        &self,
        character: &Character,
        item_name: &str,
    ) -> Option<(usize, InventoryItemRecord)> {
        let needle = item_name.trim().to_lowercase();
        let records: Vec<(usize, InventoryItemRecord)> = character
            .equipment
            .iter()
            .enumerate()
            .filter_map(|(i, raw)| as_record(raw).map(|r| (i, r)))
            .collect();

        // Exact, then prefix, then substring.
        for (i, r) in &records {
            if r.name.to_lowercase() == needle {
                return Some((*i, r.clone()));
            }
        }
        for (i, r) in &records {
            if r.name.to_lowercase().starts_with(&needle) {
                return Some((*i, r.clone()));
            }
        }
        for (i, r) in &records {
            if r.name.to_lowercase().contains(&needle) {
                return Some((*i, r.clone()));
            }
        }
        None
    }

    /// Find a matching item (case-insensitive partial match).
    pub fn find_item(
        &self,
        character: &Character,
        item_name: &str,
    ) -> Option<InventoryItemRecord> {
        self.find_item_with_index(character, item_name)
            .map(|(_, r)| r)
    }

    /// Find and use an item, applying its effect and removing consumables.
    pub fn use_item<R: Rng + ?Sized>(
        &self,
        character: &mut Character,
        item_name: &str,
        rng: &mut R,
    ) -> ItemUseResult {
        let Some((index, item)) = self.find_item_with_index(character, item_name) else {
            return ItemUseResult {
                success: false,
                item_name: item_name.to_string(),
                effect: String::new(),
                narration: format!("You don't have a '{item_name}' in your inventory."),
                error: Some(format!("Item not found: {item_name}")),
            };
        };

        let actual_name = if item.name.is_empty() {
            item_name.to_string()
        } else {
            item.name.clone()
        };
        let item_type = if !item.r#type.is_empty() {
            item.r#type.clone()
        } else {
            item.item_type.clone().unwrap_or_else(|| "misc".to_string())
        };

        // Consumables with a healing expression.
        if item.data.contains_key("healing") {
            let heal_result = HealingSystem.use_healing_item(character, &item, rng);
            character.equipment.remove(index);
            return ItemUseResult {
                success: true,
                item_name: actual_name,
                effect: format!("healed {} HP", heal_result.hp_restored),
                narration: heal_result.narration,
                error: None,
            };
        }

        // Generic/misc items can't be "used" meaningfully.
        if matches!(item_type.as_str(), "junk" | "misc" | "material" | "pretech") {
            return ItemUseResult {
                success: false,
                item_name: actual_name.clone(),
                effect: String::new(),
                narration: format!("You can't use the {actual_name} that way."),
                error: Some("Item is not usable in this context".into()),
            };
        }

        ItemUseResult {
            success: false,
            item_name: actual_name.clone(),
            effect: String::new(),
            narration: format!("You're not sure how to use the {actual_name} here."),
            error: Some("Item not usable in context".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn hero_with_items() -> Character {
        let mut c = Character::new("Hero", "warrior");
        c.max_hp = 20;
        c.hp = 5;
        c.equipment = vec![
            serde_json::from_str(r#"{"name":"Healing Potion","data":{"healing":"2d6"}}"#).unwrap(),
            serde_json::from_str(r#"{"name":"Rusty Junk","type":"junk"}"#).unwrap(),
        ];
        c
    }

    #[test]
    fn use_healing_item_removes_it() {
        let mut c = hero_with_items();
        let mut rng = StdRng::seed_from_u64(1);
        let r = ItemSystem.use_item(&mut c, "healing", &mut rng);
        assert!(r.success);
        assert!(c.hp > 5);
        assert_eq!(c.equipment.len(), 1); // potion consumed
    }

    #[test]
    fn junk_is_not_usable() {
        let mut c = hero_with_items();
        let mut rng = StdRng::seed_from_u64(1);
        let r = ItemSystem.use_item(&mut c, "rusty", &mut rng);
        assert!(!r.success);
        assert!(r.error.is_some());
    }

    #[test]
    fn missing_item_reports_error() {
        let mut c = hero_with_items();
        let mut rng = StdRng::seed_from_u64(1);
        let r = ItemSystem.use_item(&mut c, "sword", &mut rng);
        assert!(!r.success);
        assert_eq!(r.error.as_deref(), Some("Item not found: sword"));
    }
}
