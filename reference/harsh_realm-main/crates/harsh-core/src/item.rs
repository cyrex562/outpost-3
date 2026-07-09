//! Canonical item data model.
//!
//! Ported from `src/harsh_realm/models/item.py`. Pure value types describing
//! authored items (weapons, armor, ammo, consumables, pretech) plus the save
//! bonus profile and the tagged consumable-effect union.

use serde::{Deserialize, Serialize};

/// The four saving-throw categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SaveType {
    /// Physical save (CON-based).
    Physical,
    /// Evasion save (DEX-based).
    Evasion,
    /// Mental save (WIS-based).
    Mental,
    /// Luck save (no attribute).
    Luck,
}

/// Save bonuses granted by an item or temporary effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SaveBonusProfile {
    /// Bonus to physical saves.
    #[serde(default)]
    pub physical: i32,
    /// Bonus to evasion saves.
    #[serde(default)]
    pub evasion: i32,
    /// Bonus to mental saves.
    #[serde(default)]
    pub mental: i32,
    /// Bonus to luck saves.
    #[serde(default)]
    pub luck: i32,
}

impl SaveBonusProfile {
    /// Bonus for a save category.
    pub fn get(&self, save_type: SaveType) -> i32 {
        match save_type {
            SaveType::Physical => self.physical,
            SaveType::Evasion => self.evasion,
            SaveType::Mental => self.mental,
            SaveType::Luck => self.luck,
        }
    }

    /// Bonus for a save category named by string, or `None` if unknown.
    pub fn get_by_name(&self, name: &str) -> Option<i32> {
        match name {
            "physical" => Some(self.physical),
            "evasion" => Some(self.evasion),
            "mental" => Some(self.mental),
            "luck" => Some(self.luck),
            _ => None,
        }
    }
}

/// A consumable's effect — a tagged union discriminated by `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConsumableEffect {
    /// Provides food for a number of days.
    Food {
        /// Days of food provided (>= 1).
        days: i32,
    },
    /// Restores hit points by a dice expression.
    Heal {
        /// Healing dice expression (e.g. `"1d6"`).
        dice: String,
    },
    /// Grants a temporary saving-throw bonus.
    SaveBonus {
        /// Which save is buffed.
        save_type: SaveType,
        /// Bonus magnitude.
        bonus: i32,
        /// Duration in rounds/ticks (>= 1).
        duration: i32,
    },
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

/// A canonical item loaded from authored item catalogs.
///
/// All items share `id`/`name`/`enc`/`cost`/`tags`; category-specific fields
/// are optional. The category is derived from the id prefix
/// (e.g. `weapon.short_sword` -> `weapon`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemData {
    /// Canonical id, e.g. `weapon.short_sword`.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Encumbrance (may be fractional).
    #[serde(default)]
    pub enc: f64,
    /// Cost in the base currency.
    #[serde(default)]
    pub cost: i32,
    /// Free-form tags.
    #[serde(default)]
    pub tags: Vec<String>,

    // Weapon-specific
    /// Damage dice expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage: Option<String>,
    /// `"melee"` or `"ranged"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_type: Option<String>,
    /// Governing attribute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// Shock damage on a miss.
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub shock_damage: i32,
    /// AC threshold under which shock applies.
    #[serde(default, skip_serializing_if = "is_zero_i32")]
    pub shock_ac_threshold: i32,
    /// Range band identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_band: Option<String>,
    /// Ammo type consumed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ammo_type: Option<String>,

    // Armor-specific
    /// Base armor class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac: Option<i32>,
    /// AC bonus (e.g. shields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ac_bonus: Option<i32>,

    // Ammo-specific
    /// Encumbrance per 20 units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enc_per_20: Option<i32>,
    /// Cost per 20 units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_per_20: Option<i32>,
    /// Weapon tags the ammo applies to.
    #[serde(default)]
    pub weapon_tags: Vec<String>,

    // Consumable-specific
    /// Consumable effect, when this is a consumable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<ConsumableEffect>,

    // Pretech-specific
    /// Remaining power charges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_charges: Option<i32>,

    /// Save bonuses granted while equipped/held.
    #[serde(default)]
    pub save_bonuses: SaveBonusProfile,
}

impl ItemData {
    /// Category derived from the id prefix (`weapon.x` -> `weapon`), or
    /// `"unknown"` when the id has no `.` separator.
    pub fn category(&self) -> &str {
        match self.id.split_once('.') {
            Some((prefix, _)) => prefix,
            None => "unknown",
        }
    }

    /// Whether this is a weapon.
    pub fn is_weapon(&self) -> bool {
        self.category() == "weapon"
    }

    /// Whether this is armor.
    pub fn is_armor(&self) -> bool {
        self.category() == "armor"
    }

    /// Whether this is ammunition.
    pub fn is_ammo(&self) -> bool {
        self.category() == "ammo"
    }

    /// Whether this is a consumable.
    pub fn is_consumable(&self) -> bool {
        self.category() == "consumable"
    }

    /// Whether this is a melee weapon.
    pub fn is_melee(&self) -> bool {
        self.damage_type.as_deref() == Some("melee")
    }

    /// Whether this is a ranged weapon.
    pub fn is_ranged(&self) -> bool {
        self.damage_type.as_deref() == Some("ranged")
    }
}

/// One authored item catalog document — a flat list of items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemCatalog(pub Vec<ItemData>);

impl ItemCatalog {
    /// Catalog entries as a slice.
    pub fn items(&self) -> &[ItemData] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> ItemData {
        serde_json::from_value(serde_json::json!({"id": id, "name": id})).unwrap()
    }

    #[test]
    fn save_bonus_profile_lookup() {
        let p = SaveBonusProfile {
            physical: 1,
            evasion: 2,
            mental: 0,
            luck: -1,
        };
        assert_eq!(p.get(SaveType::Evasion), 2);
        assert_eq!(p.get(SaveType::Luck), -1);
        assert_eq!(p.get_by_name("mental"), Some(0));
        assert_eq!(p.get_by_name("nope"), None);
    }

    #[test]
    fn save_bonus_profile_defaults_to_zero() {
        let p: SaveBonusProfile = serde_json::from_str("{}").unwrap();
        assert_eq!(p, SaveBonusProfile::default());
        assert_eq!(p.get(SaveType::Physical), 0);
    }

    #[test]
    fn category_and_kind_predicates() {
        assert_eq!(item("weapon.short_sword").category(), "weapon");
        assert!(item("weapon.short_sword").is_weapon());
        assert!(item("armor.leather").is_armor());
        assert!(item("ammo.arrow").is_ammo());
        assert!(item("consumable.ration").is_consumable());
        assert_eq!(item("torch").category(), "unknown");
        assert!(!item("torch").is_weapon());
    }

    #[test]
    fn melee_vs_ranged() {
        let mut sword = item("weapon.short_sword");
        sword.damage_type = Some("melee".to_string());
        assert!(sword.is_melee());
        assert!(!sword.is_ranged());

        let mut bow = item("weapon.bow");
        bow.damage_type = Some("ranged".to_string());
        assert!(bow.is_ranged());
        assert!(!bow.is_melee());
    }

    #[test]
    fn minimal_item_fills_defaults() {
        let it = item("weapon.club");
        assert_eq!(it.enc, 0.0);
        assert_eq!(it.cost, 0);
        assert!(it.tags.is_empty());
        assert_eq!(it.shock_damage, 0);
        assert!(it.effect.is_none());
        assert_eq!(it.save_bonuses, SaveBonusProfile::default());
    }

    #[test]
    fn consumable_effect_tags_round_trip() {
        for (json, expected) in [
            (
                serde_json::json!({"type": "food", "days": 3}),
                ConsumableEffect::Food { days: 3 },
            ),
            (
                serde_json::json!({"type": "heal", "dice": "1d6"}),
                ConsumableEffect::Heal {
                    dice: "1d6".to_string(),
                },
            ),
            (
                serde_json::json!({"type": "save_bonus", "save_type": "mental", "bonus": 2, "duration": 4}),
                ConsumableEffect::SaveBonus {
                    save_type: SaveType::Mental,
                    bonus: 2,
                    duration: 4,
                },
            ),
        ] {
            let parsed: ConsumableEffect = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(parsed, expected);
            // tag survives a serialize round-trip
            let reser = serde_json::to_value(&parsed).unwrap();
            assert_eq!(reser["type"], json["type"]);
        }
    }

    #[test]
    fn item_with_consumable_effect_parses() {
        let it: ItemData = serde_json::from_value(serde_json::json!({
            "id": "consumable.healing_draught",
            "name": "Healing Draught",
            "effect": {"type": "heal", "dice": "2d4"},
        }))
        .unwrap();
        assert!(it.is_consumable());
        assert_eq!(
            it.effect,
            Some(ConsumableEffect::Heal {
                dice: "2d4".to_string()
            })
        );
    }

    #[test]
    fn catalog_round_trips_as_array() {
        let cat = ItemCatalog(vec![item("weapon.club"), item("armor.leather")]);
        let json = serde_json::to_string(&cat).unwrap();
        assert!(json.starts_with('['), "transparent newtype serializes as array: {json}");
        let back: ItemCatalog = serde_json::from_str(&json).unwrap();
        assert_eq!(back.items().len(), 2);
        assert_eq!(back, cat);
    }
}
