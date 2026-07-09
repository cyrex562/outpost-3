//! R5 vertical slice: `unarmed_strike` + `parry` end-to-end.
//!
//! Drives an `action` IR record through the full R5-P resolution pipeline —
//! contest → outcome selection → `emit_damage` → damage pipeline → pool delta —
//! and shows a `parry` reaction turning a hit into a miss. This is the integration
//! proof that the primitives (R5-P) and the action IR (R5-A) compose; it stands in
//! for what the host event loop (R5-W) will orchestrate.

use std::collections::BTreeMap;

use harsh_core::ir::{Action, Effect, TnSource};
use harsh_core::resolution::{
    apply_damage, apply_reaction, outcome_key, resolve_contest, ContestRequest, DamagePacket, Pool,
    ReactionEffect, RollMechanic, Roller,
};
use serde_json::json;

/// A minimal materialized attacker (what the host resolves from skills/attrs).
struct Attacker {
    skill_level: i64,
    attr_mod: i64,
    attack_bonus: i64,
}

/// A minimal materialized target.
struct Target {
    ac: i64,
    hp: Pool,
}

fn unarmed_strike() -> Action {
    serde_json::from_value(json!({
        "id": "unarmed_strike",
        "base": "melee_attack",
        "resolution": {
            "tn_source": { "defense": "ac" },
            "roll_spec": { "mechanic": "xwn_d20_attack", "skill": "punch", "attribute": "STR" }
        },
        "targeting": { "shape": "single", "range_band": "melee" },
        "outcome": [
            { "when": "success", "do": [
                { "kind": "emit_damage",
                  "params": { "packet": { "amount": 3, "types": ["physical", "blunt"] }, "to": "target" } }
            ] }
        ],
        "tags": ["attack", "melee", "unarmed"]
    }))
    .expect("unarmed_strike is a valid action record")
}

/// Drive an action's attack contest against a target, returning the resolution.
/// `rolled` is the host's d20 (dice stay out of the core).
fn resolve_attack(
    action: &Action,
    attacker: &Attacker,
    target: &Target,
    rolled: i64,
) -> harsh_core::resolution::ResolutionResult {
    let res = action
        .resolution
        .as_ref()
        .expect("contest action has resolution");
    // The host resolves the tn_source: a `defense` source reads the target's value.
    let tn = match &res.tn_source {
        TnSource::Defense(name) if name == "ac" => target.ac,
        other => panic!("test only handles defense:ac, got {other:?}"),
    };
    let mechanic = res.roll_spec.mechanic.unwrap_or(RollMechanic::XwnD20Attack);
    let modifier = attacker.attack_bonus + attacker.skill_level + attacker.attr_mod;
    resolve_contest(&ContestRequest {
        mechanic,
        roller: Roller::Actor,
        rolled,
        modifier,
        tn,
    })
}

/// Find the outcome branch matching a result and pull its first `emit_damage`
/// packet (the host would run every effect; the slice checks the damage path).
fn damage_packet_for(action: &Action, key: &str) -> Option<DamagePacket> {
    let branch = action.outcome.iter().find(|b| b.when == key)?;
    branch.do_.iter().find_map(|e: &Effect| {
        if e.kind == "emit_damage" {
            serde_json::from_value(e.params.get("packet")?.clone()).ok()
        } else {
            None
        }
    })
}

#[test]
fn unarmed_strike_hits_and_deals_damage() {
    let action = unarmed_strike();
    let attacker = Attacker {
        skill_level: 1,
        attr_mod: 1,
        attack_bonus: 2,
    }; // +4 total
    let target = Target {
        ac: 13,
        hp: Pool {
            id: "hp".into(),
            tags: vec!["hp".into()],
            current: 8,
            max: 8,
            can_go_negative: false,
        },
    };

    // Roll a 10: 10 + 4 = 14 ≥ 13 → hit.
    let result = resolve_attack(&action, &attacker, &target, 10);
    assert!(result.success);
    assert_eq!(outcome_key(&result), "success");

    // The success branch emits a 3-damage physical packet.
    let packet = damage_packet_for(&action, outcome_key(&result)).expect("success emits damage");
    assert_eq!(packet.amount, 3);

    // Push it through the damage pipeline against the target's hp pool.
    let app = apply_damage(&packet, &BTreeMap::new(), &[], &[target.hp]);
    assert_eq!(app.final_amount, 3);
    assert_eq!(app.deltas[0].pool_id, "hp");
    assert_eq!(app.deltas[0].delta, -3); // hp 8 → 5
    assert!(app.depleted.is_empty());
}

#[test]
fn unarmed_strike_misses_on_low_roll() {
    let action = unarmed_strike();
    let attacker = Attacker {
        skill_level: 0,
        attr_mod: 0,
        attack_bonus: 1,
    };
    let target = Target {
        ac: 15,
        hp: Pool {
            id: "hp".into(),
            tags: vec![],
            current: 8,
            max: 8,
            can_go_negative: false,
        },
    };
    // Roll a 5: 5 + 1 = 6 < 15 → miss; no damage branch.
    let result = resolve_attack(&action, &attacker, &target, 5);
    assert!(!result.success);
    assert_eq!(outcome_key(&result), "miss");
    assert!(damage_packet_for(&action, outcome_key(&result)).is_none());
}

#[test]
fn parry_reaction_turns_a_hit_into_a_miss() {
    let action = unarmed_strike();
    let attacker = Attacker {
        skill_level: 1,
        attr_mod: 1,
        attack_bonus: 2,
    };
    let target = Target {
        ac: 13,
        hp: Pool {
            id: "hp".into(),
            tags: vec!["hp".into()],
            current: 8,
            max: 8,
            can_go_negative: false,
        },
    };
    let hit = resolve_attack(&action, &attacker, &target, 10); // 14 ≥ 13 → hit
    assert!(hit.success);

    // A successful parry raises the effective TN to 16 (the defender's parry total).
    let parried = apply_reaction(&hit, RollMechanic::XwnD20Attack, ReactionEffect::SetTn(16));
    assert!(!parried.success);
    assert_eq!(outcome_key(&parried), "miss");
    // No damage now flows.
    assert!(damage_packet_for(&action, outcome_key(&parried)).is_none());
}
