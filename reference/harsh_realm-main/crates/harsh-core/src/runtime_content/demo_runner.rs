//! [`DemoCombatRunner`] — exercise IR-driven content in an isolated combat.
//!
//! The content demo harness builds an ephemeral in-memory world, loads the
//! authored records into a [`RuntimeContentStore`], and runs an auto-resolved
//! combat to completion, capturing a turn-by-turn transcript. Each turn an actor
//! attacks; the runner emits a `combat.attack` event and feeds it through the
//! [`TriggerRuntime`], so IR weapon/status triggers fire exactly as they would in
//! the live game — letting you see a new monster/weapon/effect in action without
//! starting a real game.

use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Serialize;
use serde_json::{Map, Value};

use crate::combat::action_resolver::resolve_action;
use crate::combat::resolvers::{resolve_attack, AttackParams};
use crate::combat_runtime::{Combatant, CombatState};
use crate::db::WorldDatabase;
use crate::events::GameEvent;
use crate::status_effects::repository::StatusEffectRepository;
use crate::status_effects::service::{StatusEffectService, WorldClock};

use super::store::RuntimeContentStore;
use super::eval_context::{EntitySnapshot, EvalContextBuilder};
use super::trigger_runtime::{EntityResolver, TriggerRuntime};

const CASCADE_DEPTH: usize = 8;

/// Fixed clock for the demo (a single tick; over-time expiry is deferred).
struct DemoClock(i32);
impl WorldClock for DemoClock {
    fn tick(&self) -> i32 {
        self.0
    }
}

/// One resolved attack in the demo transcript.
#[derive(Debug, Clone, Serialize)]
pub struct DemoTurn {
    /// Round number (1-based).
    pub round: i32,
    /// Acting combatant display name.
    pub attacker: String,
    /// Target combatant display name.
    pub target: String,
    /// Whether the attack hit.
    pub hit: bool,
    /// Damage dealt by the attack (0 on a miss).
    pub damage: i32,
    /// Attack narration text.
    pub narration: String,
    /// Annotations from IR triggers that fired this turn (log/event messages).
    pub triggers: Vec<String>,
    /// Acting combatant hp after the turn (poison etc. may reduce it).
    pub attacker_hp: i32,
    /// Target combatant hp after the turn.
    pub target_hp: i32,
}

/// Final state of one combatant when the demo ends.
#[derive(Debug, Clone, Serialize)]
pub struct DemoCombatantState {
    /// Entity id.
    pub entity_id: String,
    /// Display name.
    pub name: String,
    /// Whether this is the player.
    pub is_player: bool,
    /// Remaining hp.
    pub hp: i32,
    /// Max hp.
    pub max_hp: i32,
    /// Alive flag.
    pub alive: bool,
    /// Active status effect ids at the end.
    pub statuses: Vec<String>,
}

/// The complete result of a demo combat.
#[derive(Debug, Clone, Serialize)]
pub struct DemoOutcome {
    /// Turn-by-turn transcript.
    pub transcript: Vec<DemoTurn>,
    /// Rounds elapsed.
    pub rounds: i32,
    /// `"player"`, `"enemies"`, or `"draw"`.
    pub winner: String,
    /// Final combatant snapshots.
    pub combatants: Vec<DemoCombatantState>,
    /// Non-fatal errors encountered while running triggers.
    pub errors: Vec<String>,
}

/// Drives an auto-resolved combat, firing IR triggers each turn.
pub struct DemoCombatRunner {
    state: CombatState,
    rng: StdRng,
    tick: i64,
}

impl DemoCombatRunner {
    /// Construct a runner over a prepared [`CombatState`], seeded for determinism.
    pub fn new(state: CombatState, seed: u64) -> Self {
        Self {
            state,
            rng: StdRng::seed_from_u64(seed),
            tick: 0,
        }
    }

    /// Run the combat to completion (or `max_rounds`), returning the transcript.
    pub fn run_to_completion(
        &mut self,
        db: &WorldDatabase,
        store: &RuntimeContentStore,
        max_rounds: i32,
    ) -> DemoOutcome {
        let mut transcript = Vec::new();
        let mut errors = Vec::new();
        let mut round = 1;

        while round <= max_rounds && !self.is_over() {
            let order = self.state.initiative_order.clone();
            for actor_id in order {
                if self.is_over() {
                    break;
                }
                let actor = match self.combatant(&actor_id) {
                    Some(c) if c.alive => c.clone(),
                    _ => continue,
                };
                let target = match self.pick_target(actor.is_player) {
                    Some(t) => t.clone(),
                    None => continue,
                };

                let status = StatusEffectService::new(
                    StatusEffectRepository::new(db),
                    store,
                    Some(DemoClock(self.tick as i32)),
                );
                let runtime = TriggerRuntime::new(store, &status, db, self.tick, CASCADE_DEPTH);

                // Prefer an authored action (HR-767) if the actor has one in the
                // store; otherwise fall back to the legacy weapon attack.
                let action = actor
                    .actions
                    .first()
                    .and_then(|id| store.get_action(id))
                    .cloned();

                let (hit, damage, narration, trigger_notes) = if let Some(action) = action {
                    let hp_before = self.hp_of(&target.entity_id);
                    let builder = EvalContextBuilder::new(store, &status);
                    let self_snap = EntitySnapshot::from_combatant(&actor);
                    let target_snap = EntitySnapshot::from_combatant(&target);
                    match builder.build(&self_snap, Some(&target_snap), "action.resolve", &Map::new())
                    {
                        Ok(ctx) => match resolve_action(
                            &action,
                            actor.attack_bonus as i64,
                            &target,
                            &ctx,
                            &mut self.rng,
                        ) {
                            Ok(out) => {
                                let applied = runtime.apply_intents(self, &out.intents);
                                errors.extend(applied.errors);
                                let hp_after = self.hp_of(&target.entity_id);
                                let hit = !matches!(out.outcome_key.as_str(), "miss" | "fumble");
                                let notes: Vec<String> =
                                    applied.emitted.iter().map(describe_event).collect();
                                (
                                    hit,
                                    (hp_before - hp_after).max(0),
                                    format!(
                                        "{} performs {} ({})",
                                        actor.display_name, action.id, out.outcome_key
                                    ),
                                    notes,
                                )
                            }
                            Err(e) => {
                                errors.push(format!("action {}: {e}", action.id));
                                continue;
                            }
                        },
                        Err(e) => {
                            errors.push(format!("action context: {e}"));
                            continue;
                        }
                    }
                } else {
                    let result =
                        resolve_attack(&actor, &target, AttackParams::default(), &mut self.rng);
                    let damage = result.damage.as_ref().map(|d| d.total).unwrap_or(0);
                    if result.hit && damage > 0 {
                        self.apply_resource_delta(&target.entity_id, "hp", -(damage as i64));
                    }
                    // Emit the combat.attack event and run IR triggers on it.
                    let event =
                        attack_event(self.tick, &actor.entity_id, &target.entity_id, result.hit, damage);
                    let outcome = runtime.run(self, &[event]);
                    errors.extend(outcome.errors);
                    let notes: Vec<String> = outcome.emitted.iter().map(describe_event).collect();
                    (result.hit, damage, result.narration, notes)
                };

                let attacker_hp = self.hp_of(&actor.entity_id);
                let target_hp = self.hp_of(&target.entity_id);
                transcript.push(DemoTurn {
                    round,
                    attacker: actor.display_name.clone(),
                    target: target.display_name.clone(),
                    hit,
                    damage,
                    narration,
                    triggers: trigger_notes,
                    attacker_hp,
                    target_hp,
                });
            }
            round += 1;
        }

        let rounds = (round - 1).max(0);
        let winner = self.winner().to_string();
        let combatants = self.snapshot(db, store);
        DemoOutcome {
            transcript,
            rounds,
            winner,
            combatants,
            errors,
        }
    }

    fn is_over(&self) -> bool {
        !self.any_alive(true) || !self.any_alive(false)
    }

    fn any_alive(&self, is_player: bool) -> bool {
        self.state
            .combatants
            .iter()
            .any(|c| c.is_player == is_player && c.alive)
    }

    fn pick_target(&self, attacker_is_player: bool) -> Option<&Combatant> {
        self.state
            .combatants
            .iter()
            .find(|c| c.is_player != attacker_is_player && c.alive)
    }

    /// Inherent combatant lookup (the resolver `snapshot` builds on this).
    fn combatant(&self, entity_id: &str) -> Option<&Combatant> {
        self.state
            .combatants
            .iter()
            .find(|c| c.entity_id == entity_id)
    }

    fn hp_of(&self, entity_id: &str) -> i32 {
        self.combatant(entity_id).map(|c| c.hp).unwrap_or(0)
    }

    fn winner(&self) -> &'static str {
        let players = self.any_alive(true);
        let enemies = self.any_alive(false);
        match (players, enemies) {
            (true, false) => "player",
            (false, true) => "enemies",
            _ => "draw",
        }
    }

    fn snapshot(&self, db: &WorldDatabase, store: &RuntimeContentStore) -> Vec<DemoCombatantState> {
        let status = StatusEffectService::new(
            StatusEffectRepository::new(db),
            store,
            Some(DemoClock(self.tick as i32)),
        );
        self.state
            .combatants
            .iter()
            .map(|c| {
                let statuses = status
                    .list_for_entity(&c.entity_id)
                    .map(|v| v.into_iter().map(|a| a.effect_id).collect())
                    .unwrap_or_default();
                DemoCombatantState {
                    entity_id: c.entity_id.clone(),
                    name: c.display_name.clone(),
                    is_player: c.is_player,
                    hp: c.hp,
                    max_hp: c.max_hp,
                    alive: c.alive,
                    statuses,
                }
            })
            .collect()
    }
}

impl EntityResolver for DemoCombatRunner {
    fn snapshot(&self, entity_id: &str) -> Option<EntitySnapshot> {
        self.combatant(entity_id).map(EntitySnapshot::from_combatant)
    }

    fn apply_resource_delta(&mut self, entity_id: &str, resource: &str, delta: i64) {
        let Some(c) = self
            .state
            .combatants
            .iter_mut()
            .find(|c| c.entity_id == entity_id)
        else {
            return;
        };
        if resource == "hp" {
            c.hp = ((c.hp as i64 + delta).max(0)).min(c.max_hp as i64) as i32;
            if c.hp == 0 {
                c.alive = false;
            }
        } else if let Some(pool) = c.pools.iter_mut().find(|p| p.id == resource) {
            pool.current += delta;
        }
    }
}

fn attack_event(tick: i64, attacker: &str, target: &str, hit: bool, damage: i32) -> GameEvent {
    let mut data = Map::new();
    data.insert("attacker_id".into(), Value::String(attacker.to_string()));
    data.insert("target_id".into(), Value::String(target.to_string()));
    data.insert("hit".into(), Value::Bool(hit));
    data.insert("damage".into(), Value::from(damage));
    GameEvent::new(tick, "combat.attack", data).with_source("demo")
}

fn describe_event(event: &GameEvent) -> String {
    if event.event_type == "gm.narrate" {
        if let Some(msg) = event.data.get("message").and_then(Value::as_str) {
            return msg.to_string();
        }
    }
    event.event_type.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::Character;
    use crate::combat::creation::create_combat;
    use crate::combat_runtime::AwarenessResult;
    use crate::creature::CreatureData;
    use crate::db_schema::SCHEMA_SQL;
    use crate::ir::ComponentRecord;
    use serde_json::json;

    fn seeded_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn store() -> RuntimeContentStore {
        let records: Vec<ComponentRecord> = vec![
            json!({"component_type": "item", "id": "envenomed_dagger", "name": "Envenomed Dagger",
                   "damage": "1d4", "tags": ["weapon"], "grants_traits": ["envenom_on_hit"]}),
            json!({"component_type": "trait", "id": "envenom_on_hit", "name": "Envenom on Hit",
                   "category": "weapon_quality",
                   "triggers": [{"id": "envenom_strike", "on": "combat.attack",
                                 "when": "event.hit == true",
                                 "do": [{"kind": "apply_status",
                                         "params": {"entity_id": "target", "status_id": "poisoned",
                                                    "duration_ticks": 3}},
                                        {"kind": "log",
                                         "params": {"message": "envenom: target is poisoned"}}]}]}),
            json!({"component_type": "status_effect", "id": "poisoned", "name": "Poisoned",
                   "default_duration_ticks": 3, "provides_tags": ["poisoned"], "stacking": "replace",
                   "triggers": [{"id": "poison_tick", "on": "combat.attack",
                                 "when": "has_tag(self, 'poisoned')",
                                 "do": [{"kind": "change_resource",
                                         "params": {"resource": "hp", "delta": -2}},
                                        {"kind": "log", "params": {"message": "poison bites"}}]}]}),
            // An authored action a creature performs on its turn (HR-767).
            json!({"component_type": "action", "id": "rend", "kind": "contest",
                   "resolution": {"tn_source": {"defense": "ac"},
                                  "roll_spec": {"mechanic": "xwn_d20_attack"}},
                   "outcome": [{"when": "success", "do": [
                       {"kind": "emit_damage",
                        "params": {"entity_id": "target",
                                   "packet": {"amount": 3, "types": ["physical"]}}},
                       {"kind": "log", "params": {"message": "rend tears flesh"}}]}]}),
        ]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        RuntimeContentStore::from_records(records)
    }

    fn pc_with_dagger() -> Character {
        let mut pc = Character::new("Adept", "warrior");
        pc.id = "pc".into();
        pc.hp = 12;
        pc.max_hp = 12;
        pc.ac = 14;
        pc.attack_bonus = 4;
        pc.equipment = vec![serde_json::from_value(json!({
            "item_id": "envenomed_dagger", "name": "Envenomed Dagger",
            "type": "weapon", "damage": "1d4"
        }))
        .unwrap()];
        pc
    }

    fn hound() -> CreatureData {
        serde_json::from_value(json!({
            "id": "scrip_hound", "name": "Scrip-Hound", "hd": 2, "hp_per_hd": 4,
            "ac": 11, "attack_bonus": 1, "damage": "1d4", "behavior": "aggressive"
        }))
        .unwrap()
    }

    #[test]
    fn demo_combat_applies_poison_and_produces_transcript() {
        let db = seeded_db();
        let store = store();
        let mut rng = StdRng::seed_from_u64(7);
        let state = create_combat(&pc_with_dagger(), &[hound()], AwarenessResult::MutualAwareness, &mut rng, "", &[], 1.0);
        let mut runner = DemoCombatRunner::new(state, 7);
        let outcome = runner.run_to_completion(&db, &store, 30);

        assert!(!outcome.transcript.is_empty());
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        // The envenom annotation appears somewhere in the transcript.
        let saw_envenom = outcome
            .transcript
            .iter()
            .any(|t| t.triggers.iter().any(|m| m.contains("poisoned")));
        assert!(saw_envenom, "expected envenom annotation: {:#?}", outcome.transcript);
        // The hound ends up dead (poison + hits) — player should win in 30 rounds.
        assert_eq!(outcome.winner, "player");
        let hound = outcome
            .combatants
            .iter()
            .find(|c| !c.is_player)
            .expect("hound present");
        assert!(!hound.alive);
    }

    fn brute() -> CreatureData {
        serde_json::from_value(json!({
            "id": "brute", "name": "Brute", "hd": 3, "hp_per_hd": 4, "ac": 12,
            "attack_bonus": 6, "damage": "1d4", "behavior": "aggressive",
            "actions": ["rend"]
        }))
        .unwrap()
    }

    #[test]
    fn creature_performs_authored_action_on_its_turn() {
        let db = seeded_db();
        let store = store();
        // A sturdy, easy-to-hit PC so the brute survives to act and lands `rend`.
        let mut pc = pc_with_dagger();
        pc.ac = 8;
        pc.hp = 30;
        pc.max_hp = 30;
        let mut rng = StdRng::seed_from_u64(5);
        let state = create_combat(&pc, &[brute()], AwarenessResult::MutualAwareness, &mut rng, "", &[], 1.0);
        let mut runner = DemoCombatRunner::new(state, 5);
        let outcome = runner.run_to_completion(&db, &store, 12);
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);

        // The brute performs its authored `rend` action (not a legacy attack).
        let performed_rend = outcome.transcript.iter().any(|t| {
            t.attacker == "Brute"
                && (t.narration.contains("performs rend")
                    || t.triggers.iter().any(|m| m.contains("rend tears")))
        });
        assert!(performed_rend, "brute should perform rend: {:#?}", outcome.transcript);

        // And `rend` dealt damage to the PC via the IR damage pipeline.
        let pc_state = outcome.combatants.iter().find(|c| c.is_player).unwrap();
        assert!(pc_state.hp < 30, "rend should have damaged the PC");
    }

    #[test]
    fn poison_persists_as_active_status_during_combat() {
        // Sanity: a poisoned status sticks to the hound after the first hit.
        let db = seeded_db();
        let store = store();
        let mut rng = StdRng::seed_from_u64(1);
        let state = create_combat(&pc_with_dagger(), &[hound()], AwarenessResult::PlayerSurprise, &mut rng, "", &[], 1.0);
        let mut runner = DemoCombatRunner::new(state, 1);
        let outcome = runner.run_to_completion(&db, &store, 1);
        // After one round the hound has been hit at least once; transcript exists.
        assert!(!outcome.transcript.is_empty());
    }
}
