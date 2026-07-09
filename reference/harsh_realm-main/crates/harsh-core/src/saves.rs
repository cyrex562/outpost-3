//! Saving throw resolution engine.
//!
//! Ported from `src/harsh_realm/engine/saves.py`. The target/modifier
//! arithmetic lives in `resolvers::saves`; this rolls the d20 and gathers the
//! inputs from the character.

use rand::Rng;

use crate::character::Character;
use crate::engine_results::SaveResult;
use crate::resolvers::saves::{resolve_save as resolve_arithmetic, save_stat_key};

/// Default luck-save target (the character model has no luck-save field).
const DEFAULT_LUCK_SAVE: i32 = 15;

/// Roll a saving throw for the character.
///
/// Rolls d20 + relevant stat modifier + save bonuses vs. the base target
/// (plus `difficulty_modifier`). `modifier_total` folds into the bonus.
pub fn resolve_save<R: Rng + ?Sized>(
    character: &Character,
    save_type: &str,
    difficulty_modifier: i32,
    modifier_total: i32,
    rng: &mut R,
) -> SaveResult {
    let base_target = match save_type {
        "physical" => character.physical_save,
        "evasion" => character.evasion_save,
        "mental" => character.mental_save,
        _ => DEFAULT_LUCK_SAVE, // luck (and unknown)
    };

    let stat_key = save_stat_key(save_type);
    let stat_mod = if stat_key.is_empty() {
        0
    } else {
        character.attr_mods.get(stat_key).copied().unwrap_or(0)
    };

    let bonus = character.save_bonuses.get_by_name(save_type).unwrap_or(0);

    let roll = rng.gen_range(1..=20);
    let outcome = resolve_arithmetic(
        roll as i64,
        stat_mod as i64,
        (bonus + modifier_total) as i64,
        base_target as i64,
        difficulty_modifier as i64,
    );

    SaveResult {
        save_type: save_type.to_string(),
        roll: outcome.roll as i32,
        modifier: outcome.modifier as i32,
        total: outcome.total as i32,
        target: outcome.target as i32,
        passed: outcome.passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn hero() -> Character {
        let mut c = Character::new("Hero", "warrior");
        c.physical_save = 14;
        c.attr_mods.insert("con".into(), 2);
        c
    }

    #[test]
    fn physical_save_uses_con_and_target() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = resolve_save(&hero(), "physical", 0, 0, &mut rng);
        assert_eq!(r.save_type, "physical");
        assert!((1..=20).contains(&r.roll));
        // modifier = con mod (2) + bonus (0)
        assert_eq!(r.modifier, 2);
        assert_eq!(r.target, 14);
        assert_eq!(r.total, r.roll + 2);
        assert_eq!(r.passed, r.total >= 14);
    }

    #[test]
    fn luck_save_defaults_to_fifteen_no_stat() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = resolve_save(&hero(), "luck", 0, 0, &mut rng);
        assert_eq!(r.target, 15);
        assert_eq!(r.modifier, 0);
    }

    #[test]
    fn evasion_save_uses_evasion_target_and_dex() {
        let mut c = Character::new("Elf", "expert");
        c.evasion_save = 13;
        c.attr_mods.insert("dex".into(), 1);
        let mut rng = StdRng::seed_from_u64(4);
        let r = resolve_save(&c, "evasion", 0, 0, &mut rng);
        assert_eq!(r.save_type, "evasion");
        assert_eq!(r.target, 13);
        // modifier = dex mod (1) + bonus (0) + modifier_total (0)
        assert_eq!(r.modifier, 1);
        assert_eq!(r.total, r.roll + 1);
        assert_eq!(r.passed, r.total >= 13);
    }

    #[test]
    fn mental_save_uses_mental_target_and_wis() {
        let mut c = Character::new("Sage", "expert");
        c.mental_save = 12;
        c.attr_mods.insert("wis".into(), 2);
        let mut rng = StdRng::seed_from_u64(9);
        let r = resolve_save(&c, "mental", 0, 0, &mut rng);
        assert_eq!(r.save_type, "mental");
        assert_eq!(r.target, 12);
        assert_eq!(r.modifier, 2);
        assert_eq!(r.total, r.roll + 2);
    }

    #[test]
    fn modifier_total_adds_to_save_roll() {
        // modifier_total is added alongside the save bonus; changing + to -
        // would give an incorrect total. Force a deterministic roll and verify.
        let mut c = Character::new("Hero", "warrior");
        c.physical_save = 15;
        c.attr_mods.insert("con".into(), 0);
        let mut rng = StdRng::seed_from_u64(7);
        let r = resolve_save(&c, "physical", 0, 3, &mut rng);
        // modifier = stat_mod (0) + bonus (0) + modifier_total (3) = 3
        assert_eq!(r.modifier, 3);
        assert_eq!(r.total, r.roll + 3);
    }
}
