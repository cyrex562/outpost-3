//! Attack, damage, and shock resolution.
//!
//! Ported from `src/harsh_realm/engine/combat/resolvers.py`. The roll math is in
//! [`crate::resolvers::combat`]; this adds dice rolling, damage-on-hit chaining,
//! and narration.

use rand::Rng;

use crate::combat_runtime::Combatant;
use crate::damage::parse_damage_expr;
use crate::engine_results::{AttackResult, DamageResult};
use crate::item::ItemData;
use crate::resolvers::combat::{damage_total, resolve_hit, resolve_shock as resolve_shock_core};

use super::narration::{build_attack_narration, load_narration};

/// Optional attack parameters (mirrors the Python keyword args).
#[derive(Debug, Clone, Copy, Default)]
pub struct AttackParams {
    /// Whether the attacker is a Warrior (bonus damage).
    pub is_warrior: bool,
    /// Warrior level for bonus damage.
    pub warrior_level: i32,
    /// Combat skill level modifier.
    pub skill_level: i32,
    /// Attribute modifier (STR melee / DEX ranged).
    pub attr_mod: i32,
    /// Extra attack modifier.
    pub attack_modifier: i32,
    /// Extra damage modifier.
    pub damage_modifier: i32,
}

/// The avoidance value an attack must beat to hit `target`, drawn from the
/// target's named `defenses` and selected by the attacker's range band: a ranged
/// attack is opposed by `evasion` (falling back to `ac`), a melee attack by `ac`.
/// When no named defense is authored, the legacy `ac` field is used, so existing
/// content is unchanged.
pub fn avoidance_defense(target: &Combatant, attacker_range_band: &str) -> i64 {
    let legacy_ac = target.ac as i64;
    if attacker_range_band == "melee" {
        target.defenses.get("ac").copied().unwrap_or(legacy_ac)
    } else {
        target
            .defenses
            .get("evasion")
            .or_else(|| target.defenses.get("ac"))
            .copied()
            .unwrap_or(legacy_ac)
    }
}

/// Resolve one attack from attacker against target.
pub fn resolve_attack<R: Rng>(
    attacker: &Combatant,
    target: &Combatant,
    params: AttackParams,
    rng: &mut R,
) -> AttackResult {
    let roll = rng.gen_range(1..=20);
    let target_defense = avoidance_defense(target, &attacker.range_band);
    let outcome = resolve_hit(
        roll as i64,
        attacker.attack_bonus as i64,
        params.skill_level as i64,
        (params.attr_mod + params.attack_modifier) as i64,
        target_defense,
    );

    let damage_result = if outcome.hit {
        Some(resolve_damage(
            &attacker.damage_expr,
            params.attr_mod,
            params.is_warrior,
            params.warrior_level,
            params.damage_modifier,
            rng,
        ))
    } else {
        None
    };

    let doc = load_narration();
    let narration = build_attack_narration(
        &doc,
        attacker,
        target,
        roll,
        outcome.total as i32,
        outcome.hit,
        outcome.natural_1,
        outcome.natural_20,
        damage_result.as_ref(),
        rng,
    );

    AttackResult {
        roll,
        total: outcome.total as i32,
        target_ac: target_defense as i32,
        hit: outcome.hit,
        natural_1: outcome.natural_1,
        natural_20: outcome.natural_20,
        damage: damage_result,
        attacker_name: attacker.display_name.clone(),
        target_name: target.display_name.clone(),
        narration,
    }
}

/// Roll and resolve damage from a dice expression.
pub fn resolve_damage<R: Rng>(
    damage_expr: &str,
    attr_mod: i32,
    is_warrior: bool,
    level: i32,
    damage_modifier: i32,
    rng: &mut R,
) -> DamageResult {
    let (num_dice, sides, flat_mod) = parse_damage_expr(damage_expr).unwrap_or((0, 0, 0));
    let dice_sum: i32 = if sides > 0 {
        (0..num_dice).map(|_| rng.gen_range(1..=sides)).sum()
    } else {
        0
    };
    let outcome = damage_total(
        dice_sum as i64,
        flat_mod as i64,
        (attr_mod + damage_modifier) as i64,
        is_warrior,
        level as i64,
    );
    DamageResult {
        roll: outcome.roll as i32,
        modifier: outcome.modifier as i32,
        total: outcome.total as i32,
        weapon_expr: damage_expr.to_string(),
    }
}

/// Calculate shock damage dealt on a melee miss.
pub fn resolve_shock(str_modifier: i32, weapon: &ItemData, target_ac: i32) -> i32 {
    resolve_shock_core(
        str_modifier as i64,
        weapon.shock_damage as i64,
        weapon.shock_ac_threshold as i64,
        target_ac as i64,
    ) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn combatant(name: &str, ac: i32, damage_expr: &str) -> Combatant {
        serde_json::from_value(serde_json::json!({
            "entity_id": name, "name": name, "display_name": name,
            "is_player": name == "Hero", "initiative": 0, "hp": 5, "max_hp": 5,
            "ac": ac, "attack_bonus": 5, "damage_expr": damage_expr,
            "attack_description": "attacks", "behavior": "aggressive", "num_attacks": 1,
        }))
        .unwrap()
    }

    #[test]
    fn attack_hits_and_rolls_damage() {
        let hero = combatant("Hero", 12, "1d6");
        let wolf = combatant("Wolf", 5, "1d4");
        // Find a seed where the d20 hits (low AC 5, +5 attack bonus).
        let mut hit_seen = false;
        for seed in 0..30 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let result = resolve_attack(&hero, &wolf, AttackParams::default(), &mut rng);
            if result.hit {
                assert!(result.damage.is_some());
                assert!(result.damage.unwrap().total >= 1);
                hit_seen = true;
                break;
            }
        }
        assert!(hit_seen);
    }

    #[test]
    fn shock_applies_only_within_threshold() {
        let weapon: ItemData = serde_json::from_value(serde_json::json!({
            "id": "spear", "name": "Spear", "type": "weapon",
            "shock_damage": 2, "shock_ac_threshold": 15,
        }))
        .unwrap();
        assert_eq!(resolve_shock(1, &weapon, 12), 3); // 2 + 1
        assert_eq!(resolve_shock(1, &weapon, 16), 0); // above threshold
    }

    #[test]
    fn damage_floored_at_one() {
        let mut rng = SmallRng::seed_from_u64(1);
        let d = resolve_damage("1d4-10", 0, false, 1, 0, &mut rng);
        assert_eq!(d.total, 1);
    }

    #[test]
    fn avoidance_prefers_named_defenses_by_range() {
        let mut target = combatant("Ghoul", 12, "1d6");
        target.defenses.insert("ac".into(), 13);
        target.defenses.insert("evasion".into(), 16);
        // Melee contests the `ac` defense; ranged contests `evasion`.
        assert_eq!(avoidance_defense(&target, "melee"), 13);
        assert_eq!(avoidance_defense(&target, "ranged"), 16);

        // No named defenses → the legacy `ac` field (unchanged behavior).
        let bare = combatant("Rat", 11, "1d4");
        assert_eq!(avoidance_defense(&bare, "melee"), 11);
        assert_eq!(avoidance_defense(&bare, "ranged"), 11);

        // Ranged with only an `ac` defense falls back to it.
        let mut only_ac = combatant("Bot", 9, "1d4");
        only_ac.defenses.insert("ac".into(), 14);
        assert_eq!(avoidance_defense(&only_ac, "ranged"), 14);
    }

    #[test]
    fn ranged_attack_contests_evasion() {
        let mut archer = combatant("Hero", 12, "1d6");
        archer.range_band = "ranged".into();
        let mut target = combatant("Ghoul", 10, "1d6");
        target.defenses.insert("evasion".into(), 18);
        let mut rng = SmallRng::seed_from_u64(3);
        let result = resolve_attack(&archer, &target, AttackParams::default(), &mut rng);
        // The contested value is evasion, not the legacy ac (10).
        assert_eq!(result.target_ac, 18);
    }
}
