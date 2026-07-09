//! Enemy AI decision engine for combat.
//!
//! Ported from `src/harsh_realm/engine/enemy_ai.py`. All behavior tags
//! (aggressive, territorial, defensive, ambush, pack) currently resolve to a
//! basic attack against the player; future milestones will differentiate them.

use crate::combat_runtime::{Combatant, CombatState};
use crate::engine_results::EnemyAction;

/// Choose an action for the given enemy combatant.
///
/// Returns an attack targeting the first player combatant, or `None` if no
/// player is present in the combat state (the Python version raised
/// `StopIteration` in that case).
pub fn choose_action(_combatant: &Combatant, combat_state: &CombatState) -> Option<EnemyAction> {
    let player = combat_state.combatants.iter().find(|c| c.is_player)?;
    Some(EnemyAction {
        action_type: "attack".to_string(),
        target_id: player.entity_id.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combatant(entity_id: &str, is_player: bool) -> Combatant {
        serde_json::from_value(serde_json::json!({
            "entity_id": entity_id,
            "name": entity_id,
            "display_name": entity_id,
            "is_player": is_player,
            "initiative": 0,
            "hp": 5,
            "max_hp": 5,
            "ac": 12,
            "attack_bonus": 1,
            "damage_expr": "1d6",
            "attack_description": "strikes",
            "behavior": "aggressive",
            "num_attacks": 1,
        }))
        .expect("combatant json")
    }

    fn state(combatants: Vec<Combatant>) -> CombatState {
        let order: Vec<String> = combatants.iter().map(|c| c.entity_id.clone()).collect();
        CombatState {
            combatants,
            initiative_order: order,
            ..serde_json::from_value(serde_json::json!({
                "combatants": [], "initiative_order": []
            }))
            .unwrap()
        }
    }

    #[test]
    fn targets_the_player() {
        let s = state(vec![combatant("goblin", false), combatant("hero", true)]);
        let action = choose_action(&s.combatants[0], &s).unwrap();
        assert_eq!(action.action_type, "attack");
        assert_eq!(action.target_id, "hero");
    }

    #[test]
    fn no_player_yields_none() {
        let s = state(vec![combatant("goblin", false)]);
        assert!(choose_action(&s.combatants[0], &s).is_none());
    }
}
