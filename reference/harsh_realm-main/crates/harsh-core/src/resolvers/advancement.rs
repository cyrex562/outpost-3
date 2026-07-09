//! XP / level advancement — pure core of `harsh_realm.engine.advancement`.
//!
//! The HP-gain die roll stays with the host; this ports the attack-bonus
//! progression, the default XP table, hit dice, and the deterministic level-up
//! arithmetic.

/// Default XP-needed-to-reach-level table (WWN/SWN), matching `DEFAULT_XP_TABLE`.
pub const DEFAULT_XP_TABLE: &[(u32, i64)] = &[
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
];

/// XP required to reach `level` per the default table, if defined.
pub fn xp_for_level(level: u32) -> Option<i64> {
    DEFAULT_XP_TABLE
        .iter()
        .find(|(l, _)| *l == level)
        .map(|(_, xp)| *xp)
}

/// Hit-die sides for a class (`HD_BY_CLASS`); unknown classes default to 6.
pub fn hit_die_for_class(character_class: &str) -> u32 {
    match character_class {
        "warrior" => 8,
        "expert" => 6,
        "adventurer" => 6,
        _ => 6,
    }
}

/// Attack bonus for a class at a level, matching `_attack_bonus_for_class`.
///
/// Warrior: `level`. Expert: `ceil(level/2)`. Adventurer: `ceil(level*2/3)`.
/// Unknown: `ceil(level/2)`.
pub fn attack_bonus_for_class(character_class: &str, level: i64) -> i64 {
    fn ceil_div(a: i64, b: i64) -> i64 {
        // Matches Python math.ceil(a/b) for the positive levels used here.
        (a + b - 1).div_euclid(b)
    }
    match character_class {
        "warrior" => level,
        "expert" => ceil_div(level, 2),
        "adventurer" => ceil_div(level * 2, 3),
        _ => ceil_div(level, 2),
    }
}

/// Skill points granted on level up: experts get 2, everyone else 1.
pub fn skill_points_for_class(character_class: &str) -> i64 {
    if character_class == "expert" {
        2
    } else {
        1
    }
}

/// Saving throws improve on even levels.
pub fn saves_improved(new_level: i64) -> bool {
    new_level % 2 == 0
}

/// HP gained on level up: `max(1, hp_roll + con_mod)`.
pub fn hp_gained(hp_roll: i64, con_mod: i64) -> i64 {
    (hp_roll + con_mod).max(1)
}

/// Whether a character can level up: enough XP and below the host-supplied cap.
pub fn can_level_up(xp: i64, xp_next: i64, level: i64, level_cap: i64) -> bool {
    xp >= xp_next && level < level_cap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_bonus_progressions() {
        assert_eq!(attack_bonus_for_class("warrior", 5), 5);
        // Expert: ceil(level/2)
        assert_eq!(attack_bonus_for_class("expert", 1), 1);
        assert_eq!(attack_bonus_for_class("expert", 2), 1);
        assert_eq!(attack_bonus_for_class("expert", 3), 2);
        // Adventurer: ceil(level*2/3)
        assert_eq!(attack_bonus_for_class("adventurer", 1), 1);
        assert_eq!(attack_bonus_for_class("adventurer", 3), 2);
        assert_eq!(attack_bonus_for_class("adventurer", 4), 3);
        // Unknown defaults to expert progression.
        assert_eq!(attack_bonus_for_class("mage", 4), 2);
    }

    #[test]
    fn xp_table_and_hit_dice() {
        assert_eq!(xp_for_level(1), Some(0));
        assert_eq!(xp_for_level(5), Some(12000));
        assert_eq!(xp_for_level(11), None);
        assert_eq!(hit_die_for_class("warrior"), 8);
        assert_eq!(hit_die_for_class("expert"), 6);
        assert_eq!(hit_die_for_class("druid"), 6);
    }

    #[test]
    fn level_up_helpers() {
        assert_eq!(skill_points_for_class("expert"), 2);
        assert_eq!(skill_points_for_class("warrior"), 1);
        assert!(saves_improved(4));
        assert!(!saves_improved(3));
        assert_eq!(hp_gained(1, -3), 1); // floor at 1
        assert_eq!(hp_gained(5, 2), 7);
        assert!(can_level_up(1500, 1500, 1, 10));
        assert!(!can_level_up(1500, 1500, 10, 10)); // at cap
    }
}
