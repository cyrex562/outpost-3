//! [`resolve_action`] — drive an authored [`Action`] through the R5 resolution
//! pipeline and return the effects it fires as [`Intent`]s.
//!
//! This generalizes the `tests/vertical_slice.rs` proof into reusable engine code:
//! materialize the contest (mechanic + actor modifier + target number), roll it,
//! pick the matching `outcome` branch, and lower that branch's effects to intents.
//! It reuses [`dispatch`] for the effect list, so compute effects (`roll_dice`,
//! `{ref}`/`{expr}`) and entity-role resolution work exactly as in triggers.
//!
//! The host (combat turn loop) materializes the actor's modifier and supplies the
//! [`EvalContext`] (so `self`/`target` roles resolve); applying the returned
//! intents — including routing `emit_damage` through the damage pipeline — is the
//! caller's job, identical to how trigger intents are applied. Wiring this into the
//! live enemy turn is tracked separately (HR-767); this module is the resolver it
//! will call.

use rand::Rng;

use crate::combat_runtime::Combatant;
use crate::dispatch::dispatch;
use crate::dsl::EvalContext;
use crate::intent::Intent;
use crate::ir::action::{Action, ActionKind, DifficultyRef, TnSource};
use crate::ir::effect::Trigger;
use crate::resolution::contest::{outcome_key, resolve_contest, ContestRequest, Roller};
use crate::resolution::mechanic::RollMechanic;
use crate::resolution::result::ResolutionResult;

/// The result of resolving an action: the contest outcome plus the intents its
/// matching outcome branch produced (to be applied by the caller).
#[derive(Debug, Clone)]
pub struct ActionOutcome {
    /// The contest result (`None` for an `apply` action, which has no contest).
    pub result: Option<ResolutionResult>,
    /// The outcome branch key that fired (`success`/`miss`/`crit`/`fumble`, or
    /// `success` for an `apply` action).
    pub outcome_key: String,
    /// Intents produced by the fired branch's effects, in order.
    pub intents: Vec<Intent>,
}

/// Resolve `action` performed against `target`, rolling the contest with `rng`.
///
/// `actor_modifier` is the host-summed roll modifier (attack bonus + skill +
/// attribute + gathered modifiers). `ctx` must place the actor at `self` and the
/// target at `target` so the branch's effects resolve their entity roles.
pub fn resolve_action<R: Rng>(
    action: &Action,
    actor_modifier: i64,
    target: &Combatant,
    ctx: &EvalContext,
    rng: &mut R,
) -> Result<ActionOutcome, String> {
    // `apply` actions (buffs, self-effects) have no contest — run the success branch.
    if action.kind == ActionKind::Apply || action.resolution.is_none() {
        let intents = run_branch(action, "success", ctx, rng)?;
        return Ok(ActionOutcome {
            result: None,
            outcome_key: "success".to_string(),
            intents,
        });
    }

    let resolution = action.resolution.as_ref().expect("checked above");
    let mechanic = resolution
        .roll_spec
        .mechanic
        .unwrap_or(RollMechanic::XwnD20Attack);
    let tn = resolve_tn(&resolution.tn_source, target)?;
    let rolled = roll_die(mechanic, rng);

    let result = resolve_contest(&ContestRequest {
        mechanic,
        roller: Roller::Actor,
        rolled,
        modifier: actor_modifier,
        tn,
    });
    let key = outcome_key(&result).to_string();
    let intents = run_branch(action, &key, ctx, rng)?;

    Ok(ActionOutcome {
        result: Some(result),
        outcome_key: key,
        intents,
    })
}

/// Roll the raw die value for a mechanic (`d20`, or `2d6` summed).
fn roll_die<R: Rng>(mechanic: RollMechanic, rng: &mut R) -> i64 {
    match mechanic {
        RollMechanic::Xwn2d6 => rng.gen_range(1..=6) + rng.gen_range(1..=6),
        RollMechanic::XwnD20Attack | RollMechanic::XwnD20Save => rng.gen_range(1..=20),
    }
}

/// Resolve a contest's target number from its source. `defense` reads the
/// target's named defense (falling back to the legacy `ac`); `static`/`difficulty`
/// values are used directly. Opposed/effect-DC and difficulty-by-key need extra
/// context and are reported as errors (collected, never panicked).
fn resolve_tn(tn_source: &TnSource, target: &Combatant) -> Result<i64, String> {
    match tn_source {
        TnSource::Static(n) => Ok(*n),
        TnSource::Defense(name) => Ok(target
            .defenses
            .get(name)
            .copied()
            .unwrap_or(target.ac as i64)),
        TnSource::Difficulty(DifficultyRef::Value(n)) => Ok(*n),
        TnSource::EffectDc(n) => Ok(*n),
        TnSource::Difficulty(DifficultyRef::Key(k)) => {
            Err(format!("tn_source difficulty key '{k}' needs a difficulty table (deferred)"))
        }
        TnSource::Opposed(expr) => {
            Err(format!("tn_source opposed '{expr}' needs an opposed roll (deferred)"))
        }
    }
}

/// Lower the effects of the outcome branch matching `key` (if any) to intents,
/// reusing [`dispatch`] so compute effects and entity roles behave as in triggers.
fn run_branch<R: Rng>(
    action: &Action,
    key: &str,
    ctx: &EvalContext,
    rng: &mut R,
) -> Result<Vec<Intent>, String> {
    let Some(branch) = action.outcome.iter().find(|b| b.when == key) else {
        return Ok(Vec::new()); // no branch for this outcome (e.g. a miss with no effects)
    };
    // Wrap the branch's effects in an always-firing trigger so `dispatch` runs
    // them through the same compute/lowering path as real triggers.
    let trigger = Trigger {
        id: format!("action.{key}"),
        on: "action.resolve".to_string(),
        when: "true".to_string(),
        do_: branch.do_.clone(),
        description: String::new(),
    };
    dispatch(&[trigger], "action.resolve", ctx, rng).map_err(|e| e.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::{EntityView, EvalContext};
    use rand::rngs::SmallRng;
    use rand::SeedableRng;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn unarmed_strike() -> Action {
        serde_json::from_value(json!({
            "id": "unarmed_strike",
            "resolution": {
                "tn_source": { "defense": "ac" },
                "roll_spec": { "mechanic": "xwn_d20_attack", "skill": "punch", "attribute": "STR" }
            },
            "targeting": { "shape": "single", "range_band": "melee" },
            "outcome": [
                { "when": "success", "do": [
                    { "kind": "emit_damage",
                      "params": { "entity_id": "target",
                                  "packet": { "amount": 3, "types": ["physical"] } } }
                ] },
                { "when": "crit", "do": [
                    { "kind": "roll_dice", "params": { "dice": "1d6", "bind": "bonus" } },
                    { "kind": "emit_damage",
                      "params": { "entity_id": "target",
                                  "packet": { "amount": 3, "types": ["physical"] } } }
                ] }
            ],
            "tags": ["attack", "melee"]
        }))
        .unwrap()
    }

    fn target(ac: i32) -> Combatant {
        serde_json::from_value(json!({
            "entity_id": "npc", "name": "npc", "display_name": "npc", "is_player": false,
            "initiative": 0, "hp": 8, "max_hp": 8, "ac": ac, "attack_bonus": 0,
            "damage_expr": "1d4", "attack_description": "claws", "behavior": "aggressive",
            "num_attacks": 1
        }))
        .unwrap()
    }

    fn ctx() -> EvalContext {
        let mut entities = BTreeMap::new();
        entities.insert("pc".to_string(), EntityView { id: "pc".into(), ..Default::default() });
        entities.insert("npc".to_string(), EntityView { id: "npc".into(), ..Default::default() });
        EvalContext {
            entities,
            self_id: "pc".into(),
            target_id: Some("npc".into()),
            ..Default::default()
        }
    }

    #[test]
    fn hit_fires_success_branch_emit_damage() {
        let action = unarmed_strike();
        let target = target(10);
        // +9 modifier vs ac 10 guarantees a hit on any non-fumble roll.
        let mut rng = SmallRng::seed_from_u64(2);
        let out = resolve_action(&action, 9, &target, &ctx(), &mut rng).unwrap();
        assert!(out.outcome_key == "success" || out.outcome_key == "crit");
        let dmg = out.intents.iter().any(|i| matches!(i, Intent::EmitDamage { entity_id, .. } if entity_id == "npc"));
        assert!(dmg, "success/crit should emit damage to the target: {:?}", out.intents);
    }

    #[test]
    fn miss_fires_no_effects() {
        let action = unarmed_strike();
        let target = target(20);
        // −5 modifier vs ac 20 can never reach it (max d20+(-5)=15 < 20), barring nat 20.
        for seed in 0..20 {
            let mut rng = SmallRng::seed_from_u64(seed);
            let out = resolve_action(&action, -5, &target, &ctx(), &mut rng).unwrap();
            if out.outcome_key == "miss" {
                assert!(out.intents.is_empty(), "a miss has no branch effects");
                return;
            }
        }
        panic!("expected at least one miss across seeds");
    }

    #[test]
    fn static_tn_resolves_without_a_defense() {
        let mut action = unarmed_strike();
        // Swap to a static TN of 5 so any decent roll succeeds.
        action.resolution.as_mut().unwrap().tn_source = TnSource::Static(5);
        let mut rng = SmallRng::seed_from_u64(1);
        let out = resolve_action(&action, 5, &target(99), &ctx(), &mut rng).unwrap();
        // ac 99 is ignored; static TN 5 is easily beaten.
        assert!(out.result.unwrap().success);
    }

    #[test]
    fn apply_action_runs_without_a_contest() {
        let action: Action = serde_json::from_value(json!({
            "id": "brace",
            "kind": "apply",
            "outcome": [
                { "when": "success", "do": [
                    { "kind": "apply_status", "params": { "status_id": "braced", "duration_ticks": 2 } }
                ] }
            ]
        }))
        .unwrap();
        let mut rng = SmallRng::seed_from_u64(1);
        let out = resolve_action(&action, 0, &target(10), &ctx(), &mut rng).unwrap();
        assert!(out.result.is_none());
        assert_eq!(out.outcome_key, "success");
        assert!(out
            .intents
            .iter()
            .any(|i| matches!(i, Intent::ApplyStatus { status_id, .. } if status_id == "braced")));
    }

    #[test]
    fn defense_tn_reads_named_defense() {
        let mut target = target(10);
        target.defenses.insert("ac".into(), 14);
        // modifier +0; with a static-feeling guaranteed miss we just check the TN
        // wiring: a roll that beats 10 but not 14 should miss.
        // Force the contest by rolling: seed chosen so d20 lands 12 (12 < 14, ≥ 10).
        // Instead of pinning RNG, assert the resolved TN via a deterministic helper.
        assert_eq!(resolve_tn(&TnSource::Defense("ac".into()), &target).unwrap(), 14);
        target.defenses.clear();
        assert_eq!(resolve_tn(&TnSource::Defense("ac".into()), &target).unwrap(), 10);
    }

    #[test]
    fn deferred_tn_sources_error_cleanly() {
        let t = target(10);
        assert!(resolve_tn(&TnSource::Opposed("dex".into()), &t).is_err());
        assert!(resolve_tn(&TnSource::Difficulty(DifficultyRef::Key("hard".into())), &t).is_err());
    }
}
