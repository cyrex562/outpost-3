//! Flee resolution and destination.
//!
//! Ported from `src/harsh_realm/engine/combat/flee.py`. The flee skill-check math
//! is in [`crate::resolvers::scene::resolve_flee_check`].

use rand::Rng;

use crate::character::Character;
use crate::combat_runtime::FleeResult;
use crate::grid::{Grid, GridCoord, SquareGrid};
use crate::resolvers::scene::resolve_flee_check;
use crate::runtime::{InventoryItemRecord, JsonValue};

use super::narration::load_narration;

const FLEE_CONSEQUENCES: [&str; 3] = ["parting_blow", "dropped_item", "wrong_direction"];

/// Attempt to flee from combat. Always succeeds; cleanliness varies.
///
/// `enemy_flee_difficulties` carries each living enemy's flee difficulty.
pub fn resolve_flee<R: Rng>(
    character: &Character,
    enemy_flee_difficulties: &[i32],
    current_q: i32,
    current_r: i32,
    messy: bool,
    grid: Option<&dyn Grid>,
    rng: &mut R,
) -> FleeResult {
    let difficulty = enemy_flee_difficulties.iter().copied().max().unwrap_or(6);

    let exert_level = character.skills.get("exert").copied().unwrap_or(-1);
    let sneak_level = character.skills.get("sneak").copied().unwrap_or(-1);
    let str_mod = character.attr_mods.get("str").copied().unwrap_or(0);
    let dex_mod = character.attr_mods.get("dex").copied().unwrap_or(0);
    let exert_mod = exert_level + str_mod;
    let sneak_mod = sneak_level + dex_mod;

    let roll = rng.gen_range(1..=6) + rng.gen_range(1..=6);
    let outcome = resolve_flee_check(
        roll as i64,
        exert_mod as i64,
        sneak_mod as i64,
        difficulty as i64,
        messy,
    );
    let clean = outcome.clean;
    let total = outcome.total as i32;

    let square_grid = SquareGrid;
    let grid: &dyn Grid = grid.unwrap_or(&square_grid);
    let dest = grid.random_neighbor(GridCoord::new(current_q, current_r), rng);

    let mut consequence: Option<String> = None;
    let mut damage_taken = 0;
    let mut item_lost: Option<String> = None;

    if !clean {
        let chosen = FLEE_CONSEQUENCES[rng.gen_range(0..FLEE_CONSEQUENCES.len())];
        consequence = Some(chosen.to_string());
        if chosen == "parting_blow" {
            damage_taken = rng.gen_range(1..=4);
        } else if chosen == "dropped_item" {
            let droppable: Vec<InventoryItemRecord> = character
                .equipment
                .iter()
                .filter_map(|raw| {
                    serde_json::from_value::<InventoryItemRecord>(JsonValue::Object(raw.clone()))
                        .ok()
                })
                .filter(|item| item.r#type != "nothing")
                .collect();
            if !droppable.is_empty() {
                let dropped = &droppable[rng.gen_range(0..droppable.len())];
                item_lost = Some(if dropped.name.is_empty() {
                    "an item".to_string()
                } else {
                    dropped.name.clone()
                });
            }
        }
    }

    let flee_group = load_narration().flee;
    let options = if clean {
        flee_group.clean
    } else {
        flee_group.messy
    };
    let mut narration = if options.is_empty() {
        "You flee.".to_string()
    } else {
        options[rng.gen_range(0..options.len())].clone()
    };

    if damage_taken > 0 {
        narration.push_str(&format!(
            " You take {damage_taken} damage from a parting blow as you escape."
        ));
    }
    if let Some(item) = &item_lost {
        narration.push_str(&format!(" You drop your {item} in the panic."));
    }

    FleeResult {
        success: true,
        clean,
        skill_check_roll: roll,
        skill_check_total: total,
        skill_check_difficulty: difficulty,
        consequence,
        destination_q: dest.q,
        destination_r: dest.r,
        damage_taken,
        item_lost,
        narration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn flee_always_succeeds_and_moves() {
        let mut rng = SmallRng::seed_from_u64(2);
        let result = resolve_flee(&Character::default(), &[8], 5, 5, false, None, &mut rng);
        assert!(result.success);
        // Destination is a neighbor of (5, 5).
        assert!((result.destination_q - 5).abs() <= 1);
        assert!((result.destination_r - 5).abs() <= 1);
        assert!(!(result.destination_q == 5 && result.destination_r == 5));
    }

    #[test]
    fn messy_flee_is_never_clean_and_has_consequence() {
        let mut rng = SmallRng::seed_from_u64(4);
        let result = resolve_flee(&Character::default(), &[10], 0, 0, true, None, &mut rng);
        assert!(!result.clean);
        assert!(result.consequence.is_some());
    }

    #[test]
    fn default_difficulty_when_no_enemies() {
        let mut rng = SmallRng::seed_from_u64(1);
        let result = resolve_flee(&Character::default(), &[], 0, 0, false, None, &mut rng);
        assert_eq!(result.skill_check_difficulty, 6);
    }
}
