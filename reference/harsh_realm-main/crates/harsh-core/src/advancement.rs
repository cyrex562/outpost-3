//! XP and level advancement system.
//!
//! Ported from `src/harsh_realm/engine/advancement.py`. The level-up math is
//! shared with [`crate::resolvers::advancement`]; class hit dice and attack
//! progression come from [`crate::class_progression`]. Python rolled the HP die
//! via its RNG; here the roll is taken from a `rand::Rng`.

use std::collections::BTreeMap;

use rand::Rng;

use crate::character::Character;
use crate::class_progression::get_progression;
use crate::creature::CreatureData;
use crate::db::WorldDatabase;
use crate::engine_results::{LevelUpResult, XPAwardResult};
use crate::resolvers::advancement::{can_level_up, hp_gained, saves_improved};

/// XWN level cap (host-owned config; the engine takes it as a parameter).
const LEVEL_CAP: i64 = 10;

/// Default XP table per WWN/SWN — used if the DB table is empty.
fn default_xp_table() -> BTreeMap<i32, i32> {
    BTreeMap::from([
        (1, 0),
        (2, 1500),
        (3, 3000),
        (4, 6000),
        (5, 12000),
        (6, 24000),
        (7, 48000),
        (8, 96000),
        (9, 192000),
        (10, 384000),
    ])
}

/// Handles XP awards and level advancement.
pub struct AdvancementSystem<'a> {
    db: Option<&'a WorldDatabase>,
    cached_xp_table: Option<BTreeMap<i32, i32>>,
}

impl<'a> AdvancementSystem<'a> {
    /// Create an advancement system, optionally backed by a world database.
    pub fn new(db: Option<&'a WorldDatabase>) -> Self {
        AdvancementSystem {
            db,
            cached_xp_table: None,
        }
    }

    fn get_xp_table(&mut self) -> BTreeMap<i32, i32> {
        if let Some(table) = &self.cached_xp_table {
            return table.clone();
        }
        let Some(db) = self.db else {
            return default_xp_table();
        };
        let rows = match db.fetch_all(
            "SELECT level, xp_needed FROM xp_progression ORDER BY level",
            &[],
        ) {
            Ok(rows) if !rows.is_empty() => rows,
            _ => return default_xp_table(),
        };
        let mut table = BTreeMap::new();
        for row in &rows {
            let level = row.get("level").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let xp = row.get("xp_needed").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            table.insert(level, xp);
        }
        self.cached_xp_table = Some(table.clone());
        table
    }

    /// Award XP for defeated enemies.
    pub fn award_combat_xp(
        &self,
        character: &Character,
        defeated_enemies: &[CreatureData],
    ) -> XPAwardResult {
        let xp_gained: i32 = defeated_enemies.iter().map(|e| e.xp_value).sum();
        let old_total = character.xp;
        let new_total = old_total + xp_gained;

        let level_up = new_total >= character.xp_next && character.level < 10;

        let mut narration = format!(
            "You gain {xp_gained} XP ({new_total}/{}).",
            character.xp_next
        );
        if level_up {
            narration.push_str(&format!(" You have reached level {}!", character.level + 1));
        }

        XPAwardResult {
            xp_gained,
            new_total,
            old_total,
            level_up,
            new_level: if level_up {
                character.level + 1
            } else {
                character.level
            },
            narration,
        }
    }

    /// Return whether the character has enough XP to level up.
    pub fn check_level_up(&self, character: &Character) -> bool {
        can_level_up(
            character.xp as i64,
            character.xp_next as i64,
            character.level as i64,
            LEVEL_CAP,
        )
    }

    /// Apply a level up to the character, mutating it and returning the result.
    pub fn apply_level_up<R: Rng>(
        &mut self,
        character: &mut Character,
        rng: &mut R,
    ) -> LevelUpResult {
        let new_level = character.level + 1;
        let progression = get_progression(&character.character_class);
        let hd = progression.hit_die;
        let con_mod = character.attr_mods.get("con").copied().unwrap_or(0);

        let hp_roll = rng.gen_range(1..=hd);
        let gained = hp_gained(hp_roll as i64, con_mod as i64) as i32;

        // Keep base_max_hp and effective max_hp consistent with the current
        // difficulty multiplier.  If base_max_hp is unset (legacy character
        // where the field defaulted to 0), initialise it from the current
        // max_hp (implying mult = 1.0) before accumulating the level gain.
        if character.base_max_hp == 0 {
            character.base_max_hp = character.max_hp;
        }
        let current_mult = if character.base_max_hp > 0 {
            character.max_hp as f64 / character.base_max_hp as f64
        } else {
            1.0
        };
        let new_base_max_hp = character.base_max_hp + gained;
        let new_max_hp = (new_base_max_hp as f64 * current_mult).round().max(1.0) as i32;
        let effective_gained = new_max_hp - character.max_hp;
        let new_hp = character.hp + effective_gained;

        let new_attack_bonus = progression.attack_bonus(new_level);
        let skill_points = progression.skill_points_per_level;

        let xp_table = self.get_xp_table();
        let new_xp_next = xp_table
            .get(&(new_level + 1))
            .copied()
            .unwrap_or(character.xp_next * 2);

        let improved = saves_improved(new_level as i64);

        character.level = new_level;
        character.base_max_hp = new_base_max_hp;
        character.max_hp = new_max_hp;
        character.hp = new_hp.min(new_max_hp);
        character.attack_bonus = new_attack_bonus;
        character.xp_next = new_xp_next;
        character.unspent_skill_points += skill_points;

        if improved {
            character.physical_save = (character.physical_save - 1).max(2);
            character.evasion_save = (character.evasion_save - 1).max(2);
            character.mental_save = (character.mental_save - 1).max(2);
        }

        let mut narration = format!(
            "Level up! You are now level {new_level}. \
             You gain {gained} HP (max HP now {new_max_hp}). \
             Attack bonus: {new_attack_bonus:+}.",
        );
        if improved {
            narration.push_str(&format!(
                " Saving throws improve! Physical: {}, Evasion: {}, Mental: {}.",
                character.physical_save, character.evasion_save, character.mental_save
            ));
        }

        LevelUpResult {
            new_level,
            hp_gained: gained,
            new_max_hp,
            skill_points,
            new_attack_bonus,
            saves_improved: improved,
            narration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn leveled_character() -> Character {
        let mut c = Character::default();
        c.level = 1;
        c.xp = 1500;
        c.xp_next = 1500;
        c.max_hp = 8;
        c.hp = 8;
        c.character_class = "warrior".to_string();
        c.attr_mods.insert("con".into(), 1);
        c
    }

    #[test]
    fn award_combat_xp_flags_level_up() {
        let sys = AdvancementSystem::new(None);
        let mut c = Character::default();
        c.level = 1;
        c.xp = 0;
        c.xp_next = 100;
        let enemy: CreatureData = serde_json::from_value(serde_json::json!({
            "id": "wolf", "name": "Wolf", "hd": 1, "ac": 12,
            "attack_bonus": 1, "damage": "1d6", "xp_value": 150,
        }))
        .unwrap();
        let result = sys.award_combat_xp(&c, &[enemy]);
        assert_eq!(result.xp_gained, 150);
        assert_eq!(result.new_total, 150);
        assert!(result.level_up);
        assert_eq!(result.new_level, 2);
    }

    #[test]
    fn check_level_up_respects_cap() {
        let sys = AdvancementSystem::new(None);
        let mut c = Character::default();
        c.level = 10;
        c.xp = 999_999;
        c.xp_next = 1;
        assert!(!sys.check_level_up(&c));
    }

    #[test]
    fn apply_level_up_increases_level_and_hp() {
        let mut sys = AdvancementSystem::new(None);
        let mut c = leveled_character();
        let mut rng = SmallRng::seed_from_u64(5);
        let before_max = c.max_hp;
        let result = sys.apply_level_up(&mut c, &mut rng);
        assert_eq!(c.level, 2);
        assert_eq!(result.new_level, 2);
        assert!(c.max_hp > before_max);
        // Warrior attack bonus at level 2 from default progression.
        assert_eq!(c.attack_bonus, result.new_attack_bonus);
        // xp_next pulled from default table (level 3 => 3000).
        assert_eq!(c.xp_next, 3000);
        assert!(result.narration.contains("Level up!"));
    }

    #[test]
    fn even_level_improves_saves() {
        let mut sys = AdvancementSystem::new(None);
        let mut c = leveled_character();
        let mut rng = SmallRng::seed_from_u64(9);
        let phys_before = c.physical_save;
        let result = sys.apply_level_up(&mut c, &mut rng);
        // Level 2 is even -> saves improve by 1.
        assert!(result.saves_improved);
        assert_eq!(c.physical_save, phys_before - 1);
    }
}
