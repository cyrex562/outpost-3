//! Trigger matching and effect → intent lowering.
//!
//! Given an incoming event and an [`EvalContext`], [`dispatch`] selects the
//! triggers that subscribe to the event and whose `when` condition holds, then
//! lowers each fired effect into a typed [`Intent`]. The engine returns the
//! intents; it never applies them (the host does — Python now, Rust later).
//!
//! This is the R1 port of `triggers/dispatcher.py` (matching) plus the
//! effect→intent mapping (R1-05). Effect params follow simple conventions
//! documented on [`lower_effect`]; `entity_id` defaults to the acting entity.

use rand::Rng;
use serde_json::{Map, Value};

use crate::dsl::{evaluate, evaluate_to_bool, parse_condition, EvalContext, EvalError};
use crate::intent::Intent;
use crate::ir::effect::{Effect, Trigger};

/// Select the triggers that fire for `event_type` under `ctx` and lower their
/// effects to intents, in trigger order then effect order.
///
/// A trigger fires when `trigger.on == event_type` and its `when` condition
/// evaluates to `true`. A parse or evaluation error on any matching trigger is
/// returned (fail-closed) rather than silently dropped.
pub fn dispatch<R: Rng + ?Sized>(
    triggers: &[Trigger],
    event_type: &str,
    ctx: &EvalContext,
    rng: &mut R,
) -> Result<Vec<Intent>, EvalError> {
    let mut intents = Vec::new();
    for trigger in triggers {
        if trigger.on != event_type {
            continue;
        }
        let condition =
            parse_condition(&trigger.when).map_err(|e| EvalError(format!("when: {e}")))?;
        if !evaluate_to_bool(&condition, ctx)? {
            continue;
        }
        // Per-trigger compute scope: `roll_dice`/`run_procedure` bind results here,
        // and later effects in the same `do` list reference them by name via
        // `{ "ref": "damage" }` or `{ "expr": "local.damage + 1" }`.
        let mut local = ctx.local.clone();
        for effect in &trigger.do_ {
            match effect.kind.as_str() {
                "roll_dice" => run_roll_dice(effect, &mut local, rng)?,
                "run_procedure" => {
                    return Err(EvalError(
                        "effect 'run_procedure' needs a procedure-runner context not yet wired \
                         into the trigger runtime (deferred)"
                            .to_string(),
                    ))
                }
                _ => intents.push(lower_effect(effect, ctx, &local)?),
            }
        }
    }
    Ok(intents)
}

/// Roll a `roll_dice` compute effect and bind its total into `local`.
///
/// Params: `dice` (an `XdY[+Z]` expression) and `bind` (the local name to store
/// the integer total under). Randomness comes from `rng` so the result is
/// deterministic given a seeded source.
fn run_roll_dice<R: Rng + ?Sized>(
    effect: &Effect,
    local: &mut Map<String, Value>,
    rng: &mut R,
) -> Result<(), EvalError> {
    let dice = req_str(&effect.params, "dice", "roll_dice")?;
    let bind = req_str(&effect.params, "bind", "roll_dice")?;
    let (num, sides, flat) = crate::damage::parse_damage_expr(&dice)
        .map_err(|e| EvalError(format!("roll_dice: {e}")))?;
    if num < 1 || sides < 2 {
        return Err(EvalError(format!(
            "roll_dice: invalid dice expression {dice:?}"
        )));
    }
    let mut total: i64 = flat as i64;
    for _ in 0..num {
        total += rng.gen_range(1..=sides) as i64;
    }
    local.insert(bind, Value::from(total));
    Ok(())
}

/// Lower a single declarative effect into a typed [`Intent`].
///
/// Param conventions (all read from `effect.params`):
/// - `entity_id` (string, optional) — target entity. The role tokens `self` and
///   `target` resolve against the context; any other value is a literal id; absent
///   defaults to `self` (R5-W3).
/// - `change_resource`: `resource` (string), `delta` (int).
/// - `apply_status`: `status_id` (string), `duration_ticks` (int, optional),
///   `source` (string, optional).
/// - `remove_status`: `status_id` (string).
/// - `apply_modifier`: `modifier` (object, an IR `Modifier`), `source_id`
///   (string, optional — defaults to the empty string).
/// - `remove_modifier`: `source_id` (string).
/// - `emit_event`: `event_type` (string), `payload` (object, optional).
/// - `log`: `message` (string).
///
/// Numeric params (`delta`, coords, …) may be literals or references to a
/// computed value: `{ "ref": "name" }` reads `local.name`, `{ "expr": "…" }`
/// evaluates a DSL expression (with `local` merged into the context). `roll_dice`
/// and `run_procedure` are handled in [`dispatch`] (they bind into `local`), not
/// here, so `lower_effect` only ever sees intent-producing verbs.
pub fn lower_effect(
    effect: &Effect,
    ctx: &EvalContext,
    local: &Map<String, Value>,
) -> Result<Intent, EvalError> {
    let p = &effect.params;
    match effect.kind.as_str() {
        "change_resource" => Ok(Intent::ChangeResource {
            entity_id: entity_id(p, ctx)?,
            resource: req_str(p, "resource", &effect.kind)?,
            delta: resolve_i64(p, "delta", &effect.kind, ctx, local)?,
        }),
        "apply_status" => Ok(Intent::ApplyStatus {
            entity_id: entity_id(p, ctx)?,
            status_id: req_str(p, "status_id", &effect.kind)?,
            duration_ticks: opt_u32(p, "duration_ticks"),
            source: opt_str(p, "source"),
        }),
        "remove_status" => Ok(Intent::RemoveStatus {
            entity_id: entity_id(p, ctx)?,
            status_id: req_str(p, "status_id", &effect.kind)?,
        }),
        "apply_modifier" => {
            let modifier = p
                .get("modifier")
                .ok_or_else(|| missing("apply_modifier", "modifier"))?;
            let modifier = serde_json::from_value(modifier.clone())
                .map_err(|e| EvalError(format!("apply_modifier.modifier invalid: {e}")))?;
            Ok(Intent::ApplyModifier {
                entity_id: entity_id(p, ctx)?,
                modifier,
                source_id: opt_str(p, "source_id").unwrap_or_default(),
            })
        }
        "remove_modifier" => Ok(Intent::RemoveModifier {
            entity_id: entity_id(p, ctx)?,
            source_id: req_str(p, "source_id", &effect.kind)?,
        }),
        "emit_event" => Ok(Intent::EmitEvent {
            event_type: req_str(p, "event_type", &effect.kind)?,
            payload: match p.get("payload") {
                Some(Value::Object(o)) => o.clone(),
                None => Map::new(),
                Some(other) => {
                    return Err(EvalError(format!(
                        "emit_event.payload must be an object, got {other}"
                    )))
                }
            },
        }),
        "log" => Ok(Intent::Log {
            message: req_str(p, "message", &effect.kind)?,
        }),
        "emit_damage" => {
            let packet = p
                .get("packet")
                .ok_or_else(|| missing("emit_damage", "packet"))?;
            let packet = serde_json::from_value(packet.clone())
                .map_err(|e| EvalError(format!("emit_damage.packet invalid: {e}")))?;
            Ok(Intent::EmitDamage {
                entity_id: entity_id(p, ctx)?,
                packet,
            })
        }
        "spawn_entity" => Ok(Intent::SpawnEntity {
            template_id: req_str(p, "template_id", &effect.kind)?,
            at_q: resolve_i64(p, "at_q", &effect.kind, ctx, local)?,
            at_r: resolve_i64(p, "at_r", &effect.kind, ctx, local)?,
        }),
        "reposition" => Ok(Intent::Reposition {
            entity_id: entity_id(p, ctx)?,
            to_q: resolve_i64(p, "to_q", &effect.kind, ctx, local)?,
            to_r: resolve_i64(p, "to_r", &effect.kind, ctx, local)?,
        }),
        // Resolution control-flow — these mutate an in-flight resolution and are
        // handled by the resolver, not lowered to intents (per the R5 decision).
        "run_action" | "set_event" => Err(EvalError(format!(
            "effect verb '{}' is resolution control-flow, not a lowerable intent",
            effect.kind
        ))),
        other => Err(EvalError(format!("Unknown effect verb: {other}"))),
    }
}

/// Resolve an effect's target entity, honouring the role tokens `self` and
/// `target` (R5-W3). An absent `entity_id` defaults to the `self` role; any other
/// string is treated as a literal entity id. `target` errors when no target is in
/// the context (fail-closed) so a mis-targeted effect never silently hits self.
fn entity_id(params: &Map<String, Value>, ctx: &EvalContext) -> Result<String, EvalError> {
    let role = match params.get("entity_id") {
        Some(Value::String(s)) => s.as_str(),
        _ => "self",
    };
    match role {
        "self" => Ok(ctx.self_id.clone()),
        "target" => ctx.target_id.clone().ok_or_else(|| {
            EvalError("effect targets 'target' but no target entity is in context".into())
        }),
        literal => Ok(literal.to_string()),
    }
}

fn req_str(params: &Map<String, Value>, key: &str, kind: &str) -> Result<String, EvalError> {
    match params.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(missing(kind, key)),
    }
}

/// Read an integer param, resolving `{ "ref": "name" }` / `{ "expr": "…" }`
/// against the compute `local` scope (and `ctx` for `expr`) before coercing.
fn resolve_i64(
    params: &Map<String, Value>,
    key: &str,
    kind: &str,
    ctx: &EvalContext,
    local: &Map<String, Value>,
) -> Result<i64, EvalError> {
    let raw = params.get(key).ok_or_else(|| missing(kind, key))?;
    let value = resolve_value(raw, ctx, local)?;
    value.as_i64().ok_or_else(|| {
        EvalError(format!(
            "effect '{kind}' param '{key}' must be an integer, got {value}"
        ))
    })
}

/// Resolve a param value: a `{ "ref": "name" }` object reads `local.name`, a
/// `{ "expr": "…" }` object evaluates a DSL expression with `local` merged into
/// the context, and anything else is returned as the literal it is.
fn resolve_value(
    raw: &Value,
    ctx: &EvalContext,
    local: &Map<String, Value>,
) -> Result<Value, EvalError> {
    let Value::Object(obj) = raw else {
        return Ok(raw.clone());
    };
    if let Some(name) = obj.get("ref") {
        let name = name
            .as_str()
            .ok_or_else(|| EvalError("`ref` must be a string".into()))?;
        return local
            .get(name)
            .cloned()
            .ok_or_else(|| EvalError(format!("unbound local ref '{name}'")));
    }
    if let Some(src) = obj.get("expr") {
        let src = src
            .as_str()
            .ok_or_else(|| EvalError("`expr` must be a string".into()))?;
        let expr = parse_condition(src).map_err(|e| EvalError(format!("expr: {e}")))?;
        let mut merged = ctx.clone();
        for (k, v) in local {
            merged.local.insert(k.clone(), v.clone());
        }
        return evaluate(&expr, &merged);
    }
    Ok(raw.clone())
}

fn opt_str(params: &Map<String, Value>, key: &str) -> Option<String> {
    match params.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn opt_u32(params: &Map<String, Value>, key: &str) -> Option<u32> {
    params.get(key).and_then(Value::as_u64).map(|n| n as u32)
}

fn missing(kind: &str, key: &str) -> EvalError {
    EvalError(format!("effect '{kind}' requires param '{key}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::EntityView;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn ctx() -> EvalContext {
        let mut entities = BTreeMap::new();
        entities.insert(
            "pc".to_string(),
            EntityView {
                id: "pc".into(),
                tags: vec!["undead".into()],
                ..Default::default()
            },
        );
        EvalContext {
            entities,
            self_id: "pc".into(),
            target_id: Some("npc".into()),
            ..Default::default()
        }
    }

    fn effect(kind: &str, params: Value) -> Effect {
        Effect {
            kind: kind.into(),
            params: params.as_object().unwrap().clone(),
        }
    }

    fn trigger(on: &str, when: &str, effects: Vec<Effect>) -> Trigger {
        Trigger {
            id: "t".into(),
            on: on.into(),
            when: when.into(),
            do_: effects,
            description: String::new(),
        }
    }

    /// Lower a single effect with the default ctx and an empty compute scope.
    fn lower(e: &Effect) -> Result<Intent, EvalError> {
        lower_effect(e, &ctx(), &Map::new())
    }

    /// Seeded RNG for deterministic dice in tests.
    fn rng() -> rand::rngs::SmallRng {
        use rand::SeedableRng;
        rand::rngs::SmallRng::seed_from_u64(7)
    }

    #[test]
    fn lowers_change_resource_with_default_entity() {
        let e = effect("change_resource", json!({"resource": "hp", "delta": -3}));
        let intent = lower(&e).unwrap();
        assert_eq!(
            intent,
            Intent::ChangeResource {
                entity_id: "pc".into(),
                resource: "hp".into(),
                delta: -3,
            }
        );
    }

    #[test]
    fn dispatch_matches_event_and_condition() {
        let triggers = vec![
            trigger(
                "combat.hit",
                "has_tag(entity, 'undead')",
                vec![effect("log", json!({"message": "smite"}))],
            ),
            // Wrong event type — must not fire.
            trigger(
                "combat.miss",
                "true",
                vec![effect("log", json!({"message": "no"}))],
            ),
            // Condition false — must not fire.
            trigger(
                "combat.hit",
                "has_tag(entity, 'angel')",
                vec![effect("log", json!({"message": "nope"}))],
            ),
        ];
        let intents = dispatch(&triggers, "combat.hit", &ctx(), &mut rng()).unwrap();
        assert_eq!(
            intents,
            vec![Intent::Log {
                message: "smite".into()
            }]
        );
    }

    #[test]
    fn explicit_entity_id_overrides_default() {
        let e = effect(
            "apply_status",
            json!({"entity_id": "npc", "status_id": "poisoned", "duration_ticks": 3}),
        );
        let intent = lower(&e).unwrap();
        assert_eq!(
            intent,
            Intent::ApplyStatus {
                entity_id: "npc".into(),
                status_id: "poisoned".into(),
                duration_ticks: Some(3),
                source: None,
            }
        );
    }

    #[test]
    fn unknown_verbs_error_explicitly() {
        // Compute verbs are handled in `dispatch`, never reach `lower_effect`;
        // an unknown verb here still errors rather than silently no-ops.
        assert!(lower(&effect("bogus", json!({}))).is_err());
    }

    #[test]
    fn missing_required_param_errors() {
        assert!(lower(&effect("change_resource", json!({"resource": "hp"}))).is_err());
    }

    #[test]
    fn roll_dice_binds_result_for_a_later_effect() {
        // A trigger rolls dice, then spends the result via `{ "ref": "dmg" }`.
        let t = trigger(
            "combat.hit",
            "true",
            vec![
                effect("roll_dice", json!({"dice": "2d6+1", "bind": "dmg"})),
                effect(
                    "change_resource",
                    json!({"entity_id": "target", "resource": "hp",
                           "delta": {"expr": "0 - local.dmg"}}),
                ),
            ],
        );
        let intents = dispatch(&[t], "combat.hit", &ctx(), &mut rng()).unwrap();
        assert_eq!(intents.len(), 1, "roll_dice itself produces no intent");
        match &intents[0] {
            Intent::ChangeResource { entity_id, resource, delta } => {
                assert_eq!(entity_id, "npc");
                assert_eq!(resource, "hp");
                // 2d6+1 ∈ [3, 13], spent as negative hp.
                assert!((-13..=-3).contains(delta), "delta was {delta}");
            }
            other => panic!("expected ChangeResource, got {other:?}"),
        }
    }

    #[test]
    fn ref_resolves_bound_value_directly() {
        let t = trigger(
            "combat.hit",
            "true",
            vec![
                effect("roll_dice", json!({"dice": "1d6+10", "bind": "amt"})),
                effect(
                    "change_resource",
                    json!({"resource": "hp", "delta": {"ref": "amt"}}),
                ),
            ],
        );
        let intents = dispatch(&[t], "combat.hit", &ctx(), &mut rng()).unwrap();
        // `{ref}` pulls the bound roll (1d6+10 ∈ [11, 16]) verbatim.
        match &intents[0] {
            Intent::ChangeResource { delta, .. } => {
                assert!((11..=16).contains(delta), "delta was {delta}")
            }
            other => panic!("expected ChangeResource, got {other:?}"),
        }
    }

    #[test]
    fn run_procedure_is_an_explicit_deferral() {
        let t = trigger(
            "combat.hit",
            "true",
            vec![effect("run_procedure", json!({"procedure": "x"}))],
        );
        let err = dispatch(&[t], "combat.hit", &ctx(), &mut rng()).unwrap_err();
        assert!(err.0.contains("run_procedure"), "{}", err.0);
    }

    #[test]
    fn unbound_ref_errors() {
        let t = trigger(
            "combat.hit",
            "true",
            vec![effect(
                "change_resource",
                json!({"resource": "hp", "delta": {"ref": "missing"}}),
            )],
        );
        assert!(dispatch(&[t], "combat.hit", &ctx(), &mut rng()).is_err());
    }

    #[test]
    fn lowers_emit_damage_to_resolved_target_role() {
        let e = effect(
            "emit_damage",
            json!({"entity_id": "target", "packet": {"amount": 6, "types": ["physical"]}}),
        );
        match lower(&e).unwrap() {
            Intent::EmitDamage { entity_id, packet } => {
                // `target` role resolves to the context's target entity.
                assert_eq!(entity_id, "npc");
                assert_eq!(packet.amount, 6);
            }
            other => panic!("expected EmitDamage, got {other:?}"),
        }
    }

    #[test]
    fn entity_roles_resolve_self_target_and_literal() {
        // self → ctx.self_id; absent → self.
        let self_role = lower(&effect("log", json!({"message": "x"})));
        assert!(self_role.is_ok());
        let dmg = |id: &str| {
            effect(
                "change_resource",
                json!({"entity_id": id, "resource": "hp", "delta": -1}),
            )
        };
        // self
        match lower(&dmg("self")).unwrap() {
            Intent::ChangeResource { entity_id, .. } => assert_eq!(entity_id, "pc"),
            o => panic!("{o:?}"),
        }
        // target
        match lower(&dmg("target")).unwrap() {
            Intent::ChangeResource { entity_id, .. } => assert_eq!(entity_id, "npc"),
            o => panic!("{o:?}"),
        }
        // literal id passes through
        match lower(&dmg("boss_42")).unwrap() {
            Intent::ChangeResource { entity_id, .. } => assert_eq!(entity_id, "boss_42"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn target_role_without_target_is_an_error() {
        let no_target = EvalContext {
            self_id: "pc".into(),
            ..Default::default()
        };
        let e = effect(
            "apply_status",
            json!({"entity_id": "target", "status_id": "x"}),
        );
        assert!(lower_effect(&e, &no_target, &Map::new()).is_err());
    }

    #[test]
    fn control_flow_verbs_are_not_intents() {
        // run_action / set_event are resolution control-flow, not lowerable intents.
        assert!(lower(&effect("run_action", json!({}))).is_err());
        assert!(lower(&effect("set_event", json!({}))).is_err());
    }
}
