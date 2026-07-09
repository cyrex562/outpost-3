//! Adapt an authored [`ir::Item`](crate::ir::Item) into the engine's runtime
//! [`ItemData`](crate::item::ItemData).
//!
//! The item sibling of [`creature_adapter`](super::creature_adapter). The IR type
//! uses `i64` and carries `grants_traits` (consumed directly by the trigger store,
//! not the legacy item model); the engine type uses `i32` and the legacy
//! `enc_per_20`/`cost_per_20` fields. Conversion clamps out-of-range integers with
//! a warning (never panics) and reports the dropped/unmapped fields so gaps are
//! explicit rather than silent.

use crate::item::{ConsumableEffect, ItemData, SaveBonusProfile, SaveType};
use crate::ir::{Item, ItemEffect};

fn clamp_i64(value: i64, field: &str, warnings: &mut Vec<String>) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| {
        let clamped = if value > i32::MAX as i64 {
            i32::MAX
        } else {
            i32::MIN
        };
        warnings.push(format!(
            "item field '{field}' value {value} out of i32 range; clamped to {clamped}"
        ));
        clamped
    })
}

fn clamp_opt(value: Option<i64>, field: &str, warnings: &mut Vec<String>) -> Option<i32> {
    value.map(|v| clamp_i64(v, field, warnings))
}

fn save_type_from(name: &str) -> Option<SaveType> {
    match name {
        "physical" => Some(SaveType::Physical),
        "evasion" => Some(SaveType::Evasion),
        "mental" => Some(SaveType::Mental),
        "luck" => Some(SaveType::Luck),
        _ => None,
    }
}

/// Map an IR consumable effect to the engine [`ConsumableEffect`]. A `save_bonus`
/// with an unknown save type is dropped with a warning.
fn map_effect(
    effect: &ItemEffect,
    item_id: &str,
    warnings: &mut Vec<String>,
) -> Option<ConsumableEffect> {
    match effect {
        ItemEffect::Food { days } => Some(ConsumableEffect::Food {
            days: clamp_i64(*days, "effect.days", warnings),
        }),
        ItemEffect::Heal { dice } => Some(ConsumableEffect::Heal { dice: dice.clone() }),
        ItemEffect::SaveBonus {
            save_type,
            bonus,
            duration,
        } => match save_type_from(save_type) {
            Some(st) => Some(ConsumableEffect::SaveBonus {
                save_type: st,
                bonus: clamp_i64(*bonus, "effect.bonus", warnings),
                duration: clamp_i64(*duration, "effect.duration", warnings),
            }),
            None => {
                warnings.push(format!(
                    "item '{item_id}': unknown save_type '{save_type}' in effect; dropped"
                ));
                None
            }
        },
    }
}

/// Convert an IR item to engine [`ItemData`], returning any conversion warnings
/// (clamps, dropped/unmapped fields).
pub fn ir_item_to_data(i: &Item) -> (ItemData, Vec<String>) {
    let mut warnings = Vec::new();

    // `grants_traits` is served directly from the IR store (it grants a trait
    // whose modifiers/triggers fire in combat); the legacy ItemData has no slot
    // for it, so note rather than silently drop.
    if !i.grants_traits.is_empty() {
        warnings.push(format!(
            "item '{}': grants_traits is served from the IR store, not the legacy ItemData \
             (carried by the trigger runtime, not this conversion)",
            i.id
        ));
    }

    let effect = i
        .effect
        .as_ref()
        .and_then(|e| map_effect(e, &i.id, &mut warnings));

    let data = ItemData {
        id: i.id.clone(),
        name: i.name.clone(),
        enc: i.enc,
        cost: clamp_i64(i.cost, "cost", &mut warnings),
        tags: i.tags.clone(),
        damage: i.damage.clone(),
        damage_type: i.damage_type.clone(),
        attribute: i.attribute.clone(),
        shock_damage: clamp_i64(i.shock_damage, "shock_damage", &mut warnings),
        shock_ac_threshold: clamp_i64(i.shock_ac_threshold, "shock_ac_threshold", &mut warnings),
        range_band: i.range_band.clone(),
        ammo_type: i.ammo_type.clone(),
        ac: clamp_opt(i.ac, "ac", &mut warnings),
        ac_bonus: clamp_opt(i.ac_bonus, "ac_bonus", &mut warnings),
        // Legacy bulk-pricing fields have no IR equivalent.
        enc_per_20: None,
        cost_per_20: None,
        weapon_tags: i.weapon_tags.clone(),
        effect,
        power_charges: clamp_opt(i.power_charges, "power_charges", &mut warnings),
        save_bonuses: SaveBonusProfile {
            physical: clamp_i64(i.save_bonuses.physical, "save_bonuses.physical", &mut warnings),
            evasion: clamp_i64(i.save_bonuses.evasion, "save_bonuses.evasion", &mut warnings),
            mental: clamp_i64(i.save_bonuses.mental, "save_bonuses.mental", &mut warnings),
            luck: clamp_i64(i.save_bonuses.luck, "save_bonuses.luck", &mut warnings),
        },
    };
    (data, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ir(value: serde_json::Value) -> Item {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn maps_a_weapon() {
        let (data, warnings) = ir_item_to_data(&ir(json!({
            "id": "weapon.reaver_knife", "name": "Reaver's Knife", "enc": 1, "cost": 15,
            "tags": ["weapon", "melee"], "damage": "1d6", "damage_type": "melee",
            "attribute": "DEX", "shock_damage": 1, "shock_ac_threshold": 15,
            "weapon_tags": ["light"]
        })));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(data.id, "weapon.reaver_knife");
        assert_eq!(data.cost, 15);
        assert_eq!(data.damage.as_deref(), Some("1d6"));
        assert_eq!(data.shock_damage, 1);
        assert_eq!(data.weapon_tags, vec!["light"]);
        assert!(data.is_weapon());
    }

    #[test]
    fn maps_a_heal_consumable() {
        let (data, _) = ir_item_to_data(&ir(json!({
            "id": "med_kit", "name": "Med Kit",
            "effect": { "type": "heal", "dice": "1d6+1" }
        })));
        match data.effect.unwrap() {
            ConsumableEffect::Heal { dice } => assert_eq!(dice, "1d6+1"),
            other => panic!("expected heal, got {other:?}"),
        }
    }

    #[test]
    fn maps_save_bonus_effect_and_profile() {
        let (data, warnings) = ir_item_to_data(&ir(json!({
            "id": "ward_charm", "name": "Ward Charm",
            "save_bonuses": { "mental": 1 },
            "effect": { "type": "save_bonus", "save_type": "mental", "bonus": 2, "duration": 6 }
        })));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(data.save_bonuses.mental, 1);
        match data.effect.unwrap() {
            ConsumableEffect::SaveBonus { save_type, bonus, duration } => {
                assert_eq!(save_type, SaveType::Mental);
                assert_eq!(bonus, 2);
                assert_eq!(duration, 6);
            }
            other => panic!("expected save_bonus, got {other:?}"),
        }
    }

    #[test]
    fn unknown_save_type_drops_effect_with_warning() {
        let (data, warnings) = ir_item_to_data(&ir(json!({
            "id": "broken", "name": "Broken",
            "effect": { "type": "save_bonus", "save_type": "wisdom", "bonus": 1, "duration": 1 }
        })));
        assert!(data.effect.is_none());
        assert!(warnings.iter().any(|w| w.contains("unknown save_type")), "{warnings:?}");
    }

    #[test]
    fn grants_traits_is_noted_not_silently_dropped() {
        let (_data, warnings) = ir_item_to_data(&ir(json!({
            "id": "weapon.shard_lance", "name": "Shard Lance", "damage": "1d6",
            "grants_traits": ["shard_laceration"]
        })));
        assert!(warnings.iter().any(|w| w.contains("grants_traits")), "{warnings:?}");
    }

    #[test]
    fn clamps_out_of_range_cost() {
        let big = i32::MAX as i64 + 1;
        let (data, warnings) = ir_item_to_data(&ir(json!({
            "id": "pricey", "name": "Pricey", "cost": big
        })));
        assert_eq!(data.cost, i32::MAX);
        assert!(warnings.iter().any(|w| w.contains("'cost'")), "{warnings:?}");
    }
}
