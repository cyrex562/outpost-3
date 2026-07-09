//! Item registry — loads all canonical items from `items/*.yaml`.
//!
//! Ported from `src/harsh_realm/engine/item_registry.py`.

use std::collections::BTreeMap;
use std::path::Path;

use crate::item::{ItemCatalog, ItemData};

/// Loads items from `data_dir/items/*.yaml` and provides lookup by id.
#[derive(Debug, Clone, Default)]
pub struct ItemRegistry {
    items: BTreeMap<String, ItemData>,
}

impl ItemRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        ItemRegistry::default()
    }

    /// Load items from all YAML files in `data_dir/items/`.
    ///
    /// Returns an error on a duplicate item id across files.
    pub fn load(&mut self, data_dir: &Path) -> Result<(), String> {
        let items_dir = data_dir.join("items");
        let Ok(read_dir) = std::fs::read_dir(&items_dir) else {
            return Ok(()); // missing dir: nothing to load (matches Python warning+return)
        };
        self.items.clear();
        let mut paths: Vec<_> = read_dir
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
            .collect();
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let catalog: ItemCatalog = serde_yaml::from_str(&text).map_err(|e| e.to_string())?;
            for item in catalog.0 {
                if self.items.contains_key(&item.id) {
                    return Err(format!(
                        "Duplicate item ID {:?} in {:?} (already defined)",
                        item.id,
                        path.file_name().unwrap_or_default()
                    ));
                }
                self.items.insert(item.id.clone(), item);
            }
        }
        Ok(())
    }

    /// Merge additional items into the registry, last-wins on id. Used to fold the
    /// world's IR-authored items into the legacy file catalog so there is one item
    /// catalog at runtime (mirrors `CreatureRegistry::extend`).
    pub fn extend(&mut self, items: impl IntoIterator<Item = ItemData>) {
        for item in items {
            self.items.insert(item.id.clone(), item);
        }
    }

    /// Return the item with the given id, if present.
    pub fn get(&self, item_id: &str) -> Option<&ItemData> {
        self.items.get(item_id)
    }

    /// Return the item with the given id, or an error.
    pub fn get_or_raise(&self, item_id: &str) -> Result<&ItemData, String> {
        self.items
            .get(item_id)
            .ok_or_else(|| format!("Unknown item ID: {item_id:?}"))
    }

    /// All loaded items.
    pub fn all_items(&self) -> Vec<&ItemData> {
        self.items.values().collect()
    }

    /// All items carrying `tag`.
    pub fn items_by_tag(&self, tag: &str) -> Vec<&ItemData> {
        self.items
            .values()
            .filter(|i| i.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// All items whose id starts with the given category prefix.
    pub fn items_by_category(&self, category: &str) -> Vec<&ItemData> {
        self.items
            .values()
            .filter(|i| i.category() == category)
            .collect()
    }

    /// Number of loaded items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Whether an item id is registered.
    pub fn contains(&self, item_id: &str) -> bool {
        self.items.contains_key(item_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_queries() {
        let dir = std::env::temp_dir().join(format!("hr_items_{}", std::process::id()));
        let items = dir.join("items");
        std::fs::create_dir_all(&items).unwrap();
        std::fs::write(
            items.join("weapons.yaml"),
            "- id: weapon.sword\n  name: Sword\n  tags: [martial]\n- id: armor.leather\n  name: Leather\n",
        )
        .unwrap();
        let mut reg = ItemRegistry::new();
        reg.load(&dir).unwrap();
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.get("weapon.sword").unwrap().name, "Sword");
        assert!(reg.contains("armor.leather"));
        assert_eq!(reg.items_by_tag("martial").len(), 1);
        assert_eq!(reg.items_by_category("weapon").len(), 1);
        assert!(reg.get_or_raise("nope").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extend_merges_last_wins() {
        let mut reg = ItemRegistry::new();
        let base: ItemData =
            serde_json::from_value(serde_json::json!({"id": "weapon.knife", "name": "Knife"}))
                .unwrap();
        reg.extend(vec![base]);
        // An IR override of the same id plus a new IR-only item.
        let ir_knife: ItemData = serde_json::from_value(
            serde_json::json!({"id": "weapon.knife", "name": "Pretech Knife"}),
        )
        .unwrap();
        let ir_new: ItemData =
            serde_json::from_value(serde_json::json!({"id": "weapon.shard_lance", "name": "Shard Lance"}))
                .unwrap();
        reg.extend(vec![ir_knife, ir_new]);
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.get("weapon.knife").unwrap().name, "Pretech Knife"); // IR overrode
        assert!(reg.contains("weapon.shard_lance"));
    }

    #[test]
    fn duplicate_id_errors() {
        let dir = std::env::temp_dir().join(format!("hr_items_dup_{}", std::process::id()));
        let items = dir.join("items");
        std::fs::create_dir_all(&items).unwrap();
        std::fs::write(items.join("a.yaml"), "- id: weapon.x\n  name: A\n").unwrap();
        std::fs::write(items.join("b.yaml"), "- id: weapon.x\n  name: B\n").unwrap();
        let mut reg = ItemRegistry::new();
        assert!(reg.load(&dir).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
