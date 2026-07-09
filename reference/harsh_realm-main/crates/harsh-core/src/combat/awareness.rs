//! Pre-combat encounter awareness checks.
//!
//! Ported from `src/harsh_realm/engine/combat/awareness.py`. The roll math lives
//! in [`crate::resolvers::scene::resolve_awareness`]; this adds terrain skill
//! selection, the unavoidable-creature special case, and narration.

use rand::Rng;

use crate::character::Character;
use crate::combat_runtime::{AwarenessCheckResult, AwarenessResult};
use crate::creature::CreatureData;
use crate::resolvers::scene::resolve_awareness;

/// 2d6 base difficulty for the enemy to detect the player.
const ENEMY_DETECTION_BASE: i64 = 8;
/// Minimum enemy difficulty after sneak adjustment.
const ENEMY_DETECTION_FLOOR: i64 = 2;

fn is_wilderness(terrain: &str) -> bool {
    matches!(
        terrain,
        "forest" | "plains" | "hills" | "wasteland" | "desert" | "swamp" | "mountains"
    )
}

fn result_from_key(key: &str) -> AwarenessResult {
    match key {
        "player_surprise" => AwarenessResult::PlayerSurprise,
        "enemy_surprise" => AwarenessResult::EnemySurprise,
        _ => AwarenessResult::MutualAwareness,
    }
}

/// Perform an awareness check before combat begins.
pub fn awareness_check<R: Rng>(
    character: &Character,
    creature: &CreatureData,
    terrain: &str,
    rng: &mut R,
) -> AwarenessCheckResult {
    if creature.unavoidable {
        return AwarenessCheckResult {
            result: AwarenessResult::EnemySurprise,
            player_roll: 2,
            player_total: 2,
            player_difficulty: creature.awareness_difficulty,
            player_succeeded: false,
            enemy_roll: 12,
            enemy_difficulty: 2,
            enemy_succeeded: true,
            skill_used: "notice".to_string(),
            narration: creature.description_seen.clone(),
        };
    }

    let skill_used = if is_wilderness(&terrain.to_lowercase()) {
        "survive"
    } else {
        "notice"
    };
    let skill_level = character.skills.get(skill_used).copied().unwrap_or(-1);
    let wis_mod = character.attr_mods.get("wis").copied().unwrap_or(0);
    let sneak_level = character.skills.get("sneak").copied().unwrap_or(-1);

    let player_roll = rng.gen_range(1..=6) + rng.gen_range(1..=6);
    let enemy_roll = rng.gen_range(1..=6) + rng.gen_range(1..=6);
    let outcome = resolve_awareness(
        player_roll as i64,
        skill_level as i64,
        wis_mod as i64,
        creature.awareness_difficulty as i64,
        enemy_roll as i64,
        sneak_level as i64,
        ENEMY_DETECTION_BASE,
        ENEMY_DETECTION_FLOOR,
    );
    let result = result_from_key(&outcome.result);

    let narration = match result {
        AwarenessResult::PlayerSurprise => format!(
            "You spot {} before it notices you. You have the initiative!",
            creature.description_short
        ),
        AwarenessResult::EnemySurprise => {
            format!("{} It attacks before you can react!", creature.description_seen)
        }
        AwarenessResult::MutualAwareness => {
            format!("{} Both sides are ready to fight.", creature.description_seen)
        }
    };

    AwarenessCheckResult {
        result,
        player_roll,
        player_total: outcome.player_total as i32,
        player_difficulty: creature.awareness_difficulty,
        player_succeeded: outcome.player_succeeded,
        enemy_roll,
        enemy_difficulty: outcome.enemy_difficulty as i32,
        enemy_succeeded: outcome.enemy_succeeded,
        skill_used: skill_used.to_string(),
        narration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    fn creature(unavoidable: bool) -> CreatureData {
        serde_json::from_value(serde_json::json!({
            "id": "wolf", "name": "Wolf", "hd": 1, "ac": 12, "attack_bonus": 1,
            "damage": "1d6", "awareness_difficulty": 8, "unavoidable": unavoidable,
            "description_seen": "A wolf snarls.", "description_short": "a wolf",
        }))
        .unwrap()
    }

    #[test]
    fn unavoidable_creature_always_surprises() {
        let mut rng = SmallRng::seed_from_u64(1);
        let result = awareness_check(&Character::default(), &creature(true), "forest", &mut rng);
        assert_eq!(result.result, AwarenessResult::EnemySurprise);
        assert!(result.enemy_succeeded);
        assert!(!result.player_succeeded);
    }

    #[test]
    fn terrain_selects_skill() {
        let mut rng = SmallRng::seed_from_u64(2);
        let wild = awareness_check(&Character::default(), &creature(false), "forest", &mut rng);
        assert_eq!(wild.skill_used, "survive");
        let mut rng2 = SmallRng::seed_from_u64(2);
        let urban = awareness_check(&Character::default(), &creature(false), "ruins", &mut rng2);
        assert_eq!(urban.skill_used, "notice");
    }
}
