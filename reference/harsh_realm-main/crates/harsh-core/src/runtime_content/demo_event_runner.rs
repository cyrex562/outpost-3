//! [`DemoEventRunner`] — exercise non-combat IR by feeding events to entities.
//!
//! The combat demo ([`DemoCombatRunner`](super::DemoCombatRunner)) auto-resolves a
//! fight; this is its non-combat sibling. You hand it a few entities (their traits,
//! statuses, hp, pools, defenses) and a sequence of events — `exploration.enter_hex`,
//! `time.tick`, an `emit_event` echo, anything — and it runs each through the
//! [`TriggerRuntime`] exactly as live play would, capturing what fired (narration,
//! errors) and the entities' final state. This lets a content author see an
//! exploration/world/over-time effect work without starting a real game.
//!
//! It is scene-agnostic: each entity is its own `self`, so room/object/world
//! reactions (global triggers) and player reactions both surface. Like the rest of
//! the runtime it is fail-safe — errors are collected, never panicked.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::db::WorldDatabase;
use crate::events::GameEvent;
use crate::resolution::damage::Pool;
use crate::status_effects::repository::StatusEffectRepository;
use crate::status_effects::service::{StatusEffectService, WorldClock};

use super::eval_context::{mitigations_from_defenses, pools_with_hp, EntitySnapshot};
use super::store::RuntimeContentStore;
use super::trigger_runtime::{EntityResolver, TriggerRuntime};

const CASCADE_DEPTH: usize = 8;

/// Fixed clock view for one demo step.
struct DemoClock(i32);
impl WorldClock for DemoClock {
    fn tick(&self) -> i32 {
        self.0
    }
}

/// A demo entity's live state (the runner mutates `hp`/`pools` as effects apply).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemoEntity {
    /// Entity id (matches the `self_id` in events you feed).
    pub id: String,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
    #[serde(default)]
    pub ac: i32,
    #[serde(default)]
    pub is_player: bool,
    /// Intrinsic trait ids this entity carries (source its triggers).
    #[serde(default)]
    pub traits: Vec<String>,
    /// Extra typed pools beyond the derived `hp` pool.
    #[serde(default)]
    pub pools: Vec<Pool>,
    /// Named defenses (`dr`/`armor` mitigate IR damage).
    #[serde(default)]
    pub defenses: BTreeMap<String, i64>,
}

impl DemoEntity {
    fn snapshot(&self) -> EntitySnapshot {
        let mut scalar_fields = Map::new();
        scalar_fields.insert("hp".into(), Value::from(self.hp));
        scalar_fields.insert("max_hp".into(), Value::from(self.max_hp));
        scalar_fields.insert("ac".into(), Value::from(self.ac));
        scalar_fields.insert("is_player".into(), Value::from(self.is_player));
        EntitySnapshot {
            entity_id: self.id.clone(),
            equipped_item_ids: Vec::new(),
            intrinsic_trait_ids: self.traits.clone(),
            scalar_fields,
            pools: pools_with_hp(self.hp, self.max_hp, &self.pools),
            mitigations: mitigations_from_defenses(&self.defenses),
        }
    }
}

/// An [`EntityResolver`] over a fixed list of [`DemoEntity`] state.
struct DemoEventResolver {
    entities: Vec<DemoEntity>,
}

impl EntityResolver for DemoEventResolver {
    fn snapshot(&self, entity_id: &str) -> Option<EntitySnapshot> {
        self.entities
            .iter()
            .find(|e| e.id == entity_id)
            .map(DemoEntity::snapshot)
    }

    fn apply_resource_delta(&mut self, entity_id: &str, resource: &str, delta: i64) {
        let Some(e) = self.entities.iter_mut().find(|e| e.id == entity_id) else {
            return;
        };
        if resource == "hp" {
            e.hp = ((e.hp as i64 + delta).max(0)).min(e.max_hp.max(0) as i64) as i32;
        } else if let Some(pool) = e.pools.iter_mut().find(|p| p.id == resource) {
            pool.current += delta;
        }
    }
}

/// What one event produced when run through the IR runtime.
#[derive(Debug, Clone, Serialize)]
pub struct DemoEventStep {
    /// The event type that was fired.
    pub event_type: String,
    /// World tick at which it fired.
    pub tick: i64,
    /// `gm.narrate`/emitted messages produced by triggers this step.
    pub messages: Vec<String>,
    /// Non-fatal errors collected this step.
    pub errors: Vec<String>,
}

/// One entity's state at the end of the run.
#[derive(Debug, Clone, Serialize)]
pub struct DemoEntityState {
    pub id: String,
    pub hp: i32,
    pub max_hp: i32,
    pub pools: Vec<Pool>,
    /// Active status effect ids at the end.
    pub statuses: Vec<String>,
}

/// The complete result of a non-combat event demo.
#[derive(Debug, Clone, Serialize)]
pub struct DemoEventOutcome {
    pub steps: Vec<DemoEventStep>,
    pub entities: Vec<DemoEntityState>,
    /// All non-fatal errors across every step (also per-step in `steps`).
    pub errors: Vec<String>,
}

/// Runs a sequence of events against a set of entities, firing IR triggers.
pub struct DemoEventRunner {
    resolver: DemoEventResolver,
    tick: i64,
    seed: u64,
}

impl DemoEventRunner {
    /// Construct a runner over the given entities, with a deterministic dice seed.
    pub fn new(entities: Vec<DemoEntity>, seed: u64) -> Self {
        Self {
            resolver: DemoEventResolver { entities },
            tick: 0,
            seed,
        }
    }

    /// Run each event in order. The clock advances one tick per event (so
    /// `time.tick` over-time effects accrue), and statuses whose duration elapsed
    /// expire after each step — mirroring the controller's turn model.
    pub fn run(
        &mut self,
        db: &WorldDatabase,
        store: &RuntimeContentStore,
        events: &[GameEvent],
    ) -> DemoEventOutcome {
        let mut steps = Vec::new();
        let mut all_errors = Vec::new();

        for event in events {
            self.tick += 1;
            let tick = self.tick;
            let status = StatusEffectService::new(
                StatusEffectRepository::new(db),
                store,
                Some(DemoClock(tick as i32)),
            );
            let runtime =
                TriggerRuntime::with_seed(store, &status, db, tick, CASCADE_DEPTH, self.seed ^ tick as u64);
            let out = runtime.run(&mut self.resolver, &[event.clone()]);

            // Expire statuses whose duration elapsed (drives over-time decay).
            if let Err(e) = StatusEffectRepository::new(db).expire_due(tick as i32) {
                all_errors.push(format!("expire_due: {e}"));
            }

            let messages: Vec<String> = out
                .emitted
                .iter()
                .map(message_of)
                .collect();
            all_errors.extend(out.errors.clone());
            steps.push(DemoEventStep {
                event_type: event.event_type.clone(),
                tick,
                messages,
                errors: out.errors,
            });
        }

        let entities = self
            .resolver
            .entities
            .iter()
            .map(|e| DemoEntityState {
                id: e.id.clone(),
                hp: e.hp,
                max_hp: e.max_hp,
                pools: e.pools.clone(),
                statuses: StatusEffectRepository::new(db)
                    .list_for_entity(&e.id)
                    .map(|v| v.into_iter().map(|a| a.effect_id).collect())
                    .unwrap_or_default(),
            })
            .collect();

        DemoEventOutcome {
            steps,
            entities,
            errors: all_errors,
        }
    }
}

/// Pull the human-facing text out of an emitted event (`gm.narrate` carries
/// `message`/`text`; otherwise fall back to the event type).
fn message_of(event: &GameEvent) -> String {
    event
        .data
        .get("message")
        .or_else(|| event.data.get("text"))
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| event.event_type.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
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
            // A carried trait: entering a ruins hex applies `dread`.
            json!({"component_type": "trait", "id": "ruin_dread", "name": "Ruin Dread",
                   "category": "regional",
                   "triggers": [{"id": "on_ruins", "on": "exploration.enter_hex",
                                 "when": "event.terrain == 'ruins'",
                                 "do": [{"kind": "apply_status",
                                         "params": {"status_id": "dread", "duration_ticks": 3}},
                                        {"kind": "log", "params": {"message": "dread settles"}}]}]}),
            // dread saps 1 hp on every tick while active.
            json!({"component_type": "status_effect", "id": "dread", "name": "Dread",
                   "default_duration_ticks": 3, "provides_tags": ["dread"], "stacking": "replace",
                   "triggers": [{"id": "gnaw", "on": "time.tick",
                                 "when": "has_tag(self, 'dread')",
                                 "do": [{"kind": "change_resource",
                                         "params": {"resource": "hp", "delta": -1}}]}]}),
            // A global/world trigger: any enter_hex logs a watch line.
            json!({"component_type": "trigger", "id": "world_watch", "on": "exploration.enter_hex",
                   "when": "true",
                   "do": [{"kind": "log", "params": {"message": "the world watches"}}]}),
        ]
        .into_iter()
        .map(|v| serde_json::from_value(v).unwrap())
        .collect();
        RuntimeContentStore::from_records(records)
    }

    fn enter_hex(terrain: &str) -> GameEvent {
        let mut d = Map::new();
        d.insert("self_id".into(), Value::String("pc".into()));
        d.insert("terrain".into(), Value::String(terrain.into()));
        GameEvent::new(0, "exploration.enter_hex", d)
    }

    fn tick_event() -> GameEvent {
        let mut d = Map::new();
        d.insert("self_id".into(), Value::String("pc".into()));
        GameEvent::new(0, "time.tick", d)
    }

    fn pc() -> DemoEntity {
        DemoEntity {
            id: "pc".into(),
            hp: 10,
            max_hp: 10,
            ac: 12,
            is_player: true,
            traits: vec!["ruin_dread".into()],
            pools: vec![],
            defenses: BTreeMap::new(),
        }
    }

    #[test]
    fn enter_ruins_then_ticks_apply_and_decay() {
        let db = seeded_db();
        let store = store();
        let mut runner = DemoEventRunner::new(vec![pc()], 1);
        // Enter ruins (applies dread + world watch + carried log), then two ticks.
        let out = runner.run(&db, &store, &[enter_hex("ruins"), tick_event(), tick_event()]);
        assert!(out.errors.is_empty(), "{:?}", out.errors);

        // Step 1: both the carried trait's log and the global watch fired.
        let step1 = &out.steps[0];
        assert!(step1.messages.iter().any(|m| m.contains("dread settles")));
        assert!(step1.messages.iter().any(|m| m.contains("world watches")));

        // dread applied at tick 1 (expires at 4); two ticks at 2 and 3 sap 1 hp each.
        let pc = out.entities.iter().find(|e| e.id == "pc").unwrap();
        assert_eq!(pc.hp, 8, "two dread ticks");
        assert!(pc.statuses.contains(&"dread".to_string()));
    }

    #[test]
    fn global_fires_for_entity_without_the_carried_trait() {
        let db = seeded_db();
        let store = store();
        let mut bare = pc();
        bare.traits.clear(); // no ruin_dread
        let mut runner = DemoEventRunner::new(vec![bare], 1);
        let out = runner.run(&db, &store, &[enter_hex("plains")]);
        // No dread (not ruins, no carried trait), but the global watch still fires.
        let step = &out.steps[0];
        assert!(step.messages.iter().any(|m| m.contains("world watches")));
        let pc = out.entities.iter().find(|e| e.id == "pc").unwrap();
        assert!(pc.statuses.is_empty());
        assert_eq!(pc.hp, 10);
    }
}
