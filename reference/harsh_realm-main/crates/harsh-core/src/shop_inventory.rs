//! Shop inventory loader — reads per-building-type YAML tier definitions.
//!
//! Ported from `src/harsh_realm/engine/shop_inventory.py`. The Python module
//! cache is dropped (loads on demand); a cache can return with the content
//! service.

use std::path::Path;

use crate::generation::ShopCatalog;
use crate::item_registry::ItemRegistry;
use crate::packs::paths::default_content_dir;
use crate::shop::ShopItem;

fn load_shop_yaml(building_type: &str, data_dir: &Path) -> ShopCatalog {
    let path = data_dir.join("shops").join(format!("{building_type}.yaml"));
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_yaml::from_str::<ShopCatalog>(&t).ok())
        .unwrap_or_default()
}

/// Title-case a building type for display (`general_store` -> `General Store`).
fn titleize(building_type: &str) -> String {
    building_type
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Load shop inventory for a building type at a given tier.
///
/// Unknown items (not in the registry) are skipped.
pub fn load_shop_inventory(
    building_type: &str,
    tier: &str,
    registry: &ItemRegistry,
    data_dir: Option<&Path>,
) -> Vec<ShopItem> {
    let owned;
    let dir = match data_dir {
        Some(d) => d,
        None => {
            owned = default_content_dir();
            &owned
        }
    };
    let catalog = load_shop_yaml(building_type, dir);
    let Some(tier_data) = catalog.get_tier(tier) else {
        return vec![];
    };
    let mut items = Vec::new();
    for entry in &tier_data.items {
        if let Some(item_data) = registry.get(&entry.item_id) {
            items.push(ShopItem {
                item_data: item_data.clone(),
                quantity: entry.quantity,
                price: item_data.cost,
            });
        }
    }
    items
}

/// Display name for a shop at a given tier (falls back to a titleized type).
pub fn get_shop_name(building_type: &str, tier: &str, data_dir: Option<&Path>) -> String {
    let owned;
    let dir = match data_dir {
        Some(d) => d,
        None => {
            owned = default_content_dir();
            &owned
        }
    };
    let catalog = load_shop_yaml(building_type, dir);
    match catalog.get_tier(tier) {
        Some(t) => t.name.clone(),
        None => titleize(building_type),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titleize_building_types() {
        assert_eq!(titleize("general_store"), "General Store");
        assert_eq!(titleize("blacksmith"), "Blacksmith");
    }

    #[test]
    fn loads_inventory_resolving_registry() {
        let dir = std::env::temp_dir().join(format!("hr_shop_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("shops")).unwrap();
        std::fs::create_dir_all(dir.join("items")).unwrap();
        std::fs::write(
            dir.join("items").join("w.yaml"),
            "- id: weapon.club\n  name: Club\n  cost: 5\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("shops").join("blacksmith.yaml"),
            "small:\n  name: Village Forge\n  items:\n    - item_id: weapon.club\n      quantity: 2\n    - item_id: missing.item\n",
        )
        .unwrap();
        let mut reg = ItemRegistry::new();
        reg.load(&dir).unwrap();
        let inv = load_shop_inventory("blacksmith", "small", &reg, Some(&dir));
        assert_eq!(inv.len(), 1); // missing.item skipped
        assert_eq!(inv[0].price, 5);
        assert_eq!(inv[0].quantity, 2);
        assert_eq!(get_shop_name("blacksmith", "small", Some(&dir)), "Village Forge");
        assert_eq!(get_shop_name("blacksmith", "huge", Some(&dir)), "Blacksmith");
        std::fs::remove_dir_all(&dir).ok();
    }
}
