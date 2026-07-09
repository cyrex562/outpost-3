//! Content demo harness (`POST /api/demo/combat`).
//!
//! Compiles authored YAML and runs an isolated, auto-resolved combat so a content
//! author can see a new monster/weapon/effect in action without starting a real
//! game. Runs entirely off the session actor on its own ephemeral in-memory world
//! — it never touches the player's loaded world.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use harsh_core::character::Character;
use harsh_core::combat::creation::create_combat;
use harsh_core::combat_runtime::AwarenessResult;
use harsh_core::compiler::{compile, Catalogs, VerbRegistry};
use harsh_core::db::WorldDatabase;
use harsh_core::db_schema::SCHEMA_SQL;
use harsh_core::events::GameEvent;
use harsh_core::runtime::JsonObject;
use harsh_core::runtime_content::{
    ir_creature_to_data, DemoCombatRunner, DemoEntity, DemoEventRunner, RuntimeContentStore,
};

/// A minimal player template for the demo combatant.
#[derive(Debug, Deserialize)]
pub struct DemoPcTemplate {
    #[serde(default = "default_pc_name")]
    pub name: String,
    #[serde(default = "default_hp")]
    pub hp: i32,
    #[serde(default = "default_ac")]
    pub ac: i32,
    #[serde(default = "default_attack_bonus")]
    pub attack_bonus: i32,
    /// Authored weapon (an `item` id from the YAML) to equip on the PC.
    #[serde(default)]
    pub weapon_id: Option<String>,
}

impl Default for DemoPcTemplate {
    fn default() -> Self {
        Self {
            name: default_pc_name(),
            hp: default_hp(),
            ac: default_ac(),
            attack_bonus: default_attack_bonus(),
            weapon_id: None,
        }
    }
}

fn default_pc_name() -> String {
    "Adept".to_string()
}
fn default_hp() -> i32 {
    14
}
fn default_ac() -> i32 {
    15
}
fn default_attack_bonus() -> i32 {
    5
}
fn default_seed() -> u64 {
    1
}
fn default_max_rounds() -> i32 {
    40
}

/// Request body for `POST /api/demo/combat`.
#[derive(Debug, Deserialize)]
pub struct DemoCombatRequest {
    /// Authored content (multi-document YAML — one record per document).
    #[serde(default)]
    pub yaml: String,
    /// The authored `creature` id to field as the opponent.
    pub creature_id: String,
    #[serde(default)]
    pub pc: DemoPcTemplate,
    #[serde(default = "default_seed")]
    pub seed: u64,
    #[serde(default = "default_max_rounds")]
    pub max_rounds: i32,
}

/// Response body: the compile report plus the combat outcome.
#[derive(Debug, Serialize)]
pub struct DemoCombatResponse {
    pub compile: Value,
    pub outcome: Option<Value>,
    pub error: Option<String>,
}

fn err(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({ "error": "DemoError", "message": message })),
    )
        .into_response()
}

/// Parse a multi-document YAML string into a list of JSON documents.
fn parse_documents(yaml: &str) -> Result<Vec<Value>, String> {
    let mut docs = Vec::new();
    for doc in serde_yaml::Deserializer::from_str(yaml) {
        let value = Value::deserialize(doc).map_err(|e| e.to_string())?;
        if !value.is_null() {
            docs.push(value);
        }
    }
    Ok(docs)
}

/// Build a seeded in-memory world (schema only — no map/world generation).
fn ephemeral_db() -> Result<WorldDatabase, String> {
    let db = WorldDatabase::open_in_memory()?;
    for stmt in SCHEMA_SQL.split(';') {
        let s = stmt.trim();
        if !s.is_empty() {
            db.execute(s, &[])?;
        }
    }
    Ok(db)
}

fn build_pc(template: &DemoPcTemplate, store: &RuntimeContentStore) -> Character {
    let mut pc = Character::new(template.name.clone(), "warrior");
    pc.id = "demo_pc".into();
    pc.hp = template.hp;
    pc.max_hp = template.hp;
    pc.ac = template.ac;
    pc.attack_bonus = template.attack_bonus;
    if let Some(weapon_id) = &template.weapon_id {
        let damage = store
            .get_item(weapon_id)
            .and_then(|i| i.damage.clone())
            .unwrap_or_else(|| "1d4".to_string());
        if let Ok(item) = serde_json::from_value(json!({
            "item_id": weapon_id,
            "name": weapon_id,
            "type": "weapon",
            "damage": damage,
        })) {
            pc.equipment = vec![item];
        }
    }
    pc
}

/// Blocking demo runner (compile → ephemeral world → auto combat → transcript).
fn run_demo(req: DemoCombatRequest) -> Result<DemoCombatResponse, String> {
    let docs = parse_documents(&req.yaml).map_err(|e| format!("YAML parse error: {e}"))?;
    let report = compile(&docs, &VerbRegistry::builtin(), &Catalogs::default());
    let compile_json = serde_json::to_value(&report).unwrap_or_else(|_| json!({}));

    // Don't run if the content didn't compile cleanly.
    if !report.syntax_errors.is_empty() || !report.unimplemented.is_empty() {
        return Ok(DemoCombatResponse {
            compile: compile_json,
            outcome: None,
            error: Some("content did not compile cleanly; fix the diagnostics first".into()),
        });
    }

    let store = RuntimeContentStore::from_records(report.records);
    let creature_ir = match store.get_creature(&req.creature_id) {
        Some(c) => c,
        None => {
            return Ok(DemoCombatResponse {
                compile: compile_json,
                outcome: None,
                error: Some(format!(
                    "no compiled creature with id '{}' — author one or fix creature_id",
                    req.creature_id
                )),
            });
        }
    };
    let (creature, _warnings) = ir_creature_to_data(creature_ir);

    let db = ephemeral_db()?;
    let pc = build_pc(&req.pc, &store);
    let mut rng = rand::rngs::StdRng::seed_from_u64(req.seed);
    let state = create_combat(&pc, &[creature], AwarenessResult::MutualAwareness, &mut rng, "", &[], 1.0);
    let mut runner = DemoCombatRunner::new(state, req.seed);
    let outcome = runner.run_to_completion(&db, &store, req.max_rounds);

    Ok(DemoCombatResponse {
        compile: compile_json,
        outcome: Some(serde_json::to_value(&outcome).unwrap_or_else(|_| json!({}))),
        error: None,
    })
}

/// `POST /api/demo/combat` — compile content and run an isolated demo combat.
pub async fn demo_combat(Json(req): Json<DemoCombatRequest>) -> Response {
    match tokio::task::spawn_blocking(move || run_demo(req)).await {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(message)) => err(StatusCode::BAD_REQUEST, &message),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "demo task panicked"),
    }
}

// ---------------------------------------------------------------------------
// Non-combat event demo (`POST /api/demo/events`)
// ---------------------------------------------------------------------------

/// One event to fire at a demo entity. `payload` must carry `self_id` (the acting
/// entity) so the runtime sources that entity's triggers; e.g.
/// `{ "event_type": "exploration.enter_hex", "payload": { "self_id": "pc", "terrain": "ruins" } }`.
#[derive(Debug, Deserialize)]
pub struct DemoEventEntry {
    pub event_type: String,
    #[serde(default)]
    pub payload: JsonObject,
}

/// Request body for `POST /api/demo/events`.
#[derive(Debug, Deserialize)]
pub struct DemoEventsRequest {
    /// Authored content (multi-document YAML).
    #[serde(default)]
    pub yaml: String,
    /// The entities to run events against (their traits/statuses/pools/defenses).
    #[serde(default)]
    pub entities: Vec<DemoEntity>,
    /// Events to fire in order.
    #[serde(default)]
    pub events: Vec<DemoEventEntry>,
    #[serde(default = "default_seed")]
    pub seed: u64,
}

/// Response body: the compile report plus the event-demo outcome.
#[derive(Debug, Serialize)]
pub struct DemoEventsResponse {
    pub compile: Value,
    pub outcome: Option<Value>,
    pub error: Option<String>,
}

/// Blocking non-combat demo (compile → ephemeral world → run events → transcript).
fn run_event_demo(req: DemoEventsRequest) -> Result<DemoEventsResponse, String> {
    let docs = parse_documents(&req.yaml).map_err(|e| format!("YAML parse error: {e}"))?;
    let report = compile(&docs, &VerbRegistry::builtin(), &Catalogs::default());
    let compile_json = serde_json::to_value(&report).unwrap_or_else(|_| json!({}));

    if !report.syntax_errors.is_empty() || !report.unimplemented.is_empty() {
        return Ok(DemoEventsResponse {
            compile: compile_json,
            outcome: None,
            error: Some("content did not compile cleanly; fix the diagnostics first".into()),
        });
    }
    if req.events.is_empty() {
        return Ok(DemoEventsResponse {
            compile: compile_json,
            outcome: None,
            error: Some("no events to run — supply at least one event".into()),
        });
    }

    let store = RuntimeContentStore::from_records(report.records);
    let db = ephemeral_db()?;
    let events: Vec<GameEvent> = req
        .events
        .into_iter()
        .map(|e| GameEvent::new(0, &e.event_type, e.payload))
        .collect();

    let mut runner = DemoEventRunner::new(req.entities, req.seed);
    let outcome = runner.run(&db, &store, &events);

    Ok(DemoEventsResponse {
        compile: compile_json,
        outcome: Some(serde_json::to_value(&outcome).unwrap_or_else(|_| json!({}))),
        error: None,
    })
}

/// `POST /api/demo/events` — compile content and run an isolated non-combat demo.
pub async fn demo_events(Json(req): Json<DemoEventsRequest>) -> Response {
    match tokio::task::spawn_blocking(move || run_event_demo(req)).await {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(message)) => err(StatusCode::BAD_REQUEST, &message),
        Err(_) => err(StatusCode::INTERNAL_SERVER_ERROR, "demo task panicked"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLICE_YAML: &str = r#"
item:
  id: envenomed_dagger
  name: Envenomed Dagger
  damage: 1d4
  tags: [weapon, melee]
  grants_traits: [envenom_on_hit]
---
trait:
  id: envenom_on_hit
  name: Envenom on Hit
  category: weapon_quality
  triggers:
    - id: envenom_strike
      on: combat.attack
      when: "event.hit == true"
      do:
        - kind: apply_status
          params: { entity_id: target, status_id: poisoned, duration_ticks: 3 }
        - kind: log
          params: { message: "envenom: target is poisoned" }
---
status_effect:
  id: poisoned
  name: Poisoned
  default_duration_ticks: 3
  provides_tags: [poisoned]
  stacking: replace
  triggers:
    - id: poison_tick
      on: combat.attack
      when: "has_tag(self, 'poisoned')"
      do:
        - kind: change_resource
          params: { resource: hp, delta: -2 }
        - kind: log
          params: { message: "poison bites" }
---
creature:
  id: scrip_hound
  name: Scrip-Hound
  hd: 2
  hp_per_hd: 4
  ac: 11
  attack_bonus: 1
  damage: 1d4
"#;

    #[test]
    fn run_demo_compiles_and_drives_poison() {
        let req = DemoCombatRequest {
            yaml: SLICE_YAML.to_string(),
            creature_id: "scrip_hound".into(),
            pc: DemoPcTemplate {
                weapon_id: Some("envenomed_dagger".into()),
                ..Default::default()
            },
            seed: 42,
            max_rounds: 40,
        };
        let resp = run_demo(req).expect("demo runs");
        assert!(resp.error.is_none(), "{:?}", resp.error);
        let outcome = resp.outcome.expect("outcome present");
        // The transcript mentions poison and the player wins.
        let text = serde_json::to_string(&outcome).unwrap();
        assert!(text.contains("poisoned") || text.contains("poison bites"), "{text}");
        assert_eq!(outcome["winner"], json!("player"));
    }

    #[test]
    fn run_demo_reports_compile_errors_without_running() {
        let req = DemoCombatRequest {
            yaml: "creature:\n  id: broken\n  name: Broken\n".into(), // missing required fields
            creature_id: "broken".into(),
            pc: DemoPcTemplate::default(),
            seed: 1,
            max_rounds: 5,
        };
        let resp = run_demo(req).expect("demo returns");
        assert!(resp.outcome.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn run_demo_errors_on_missing_creature() {
        let req = DemoCombatRequest {
            yaml: "item:\n  id: x\n  name: X\n".into(),
            creature_id: "ghost".into(),
            pc: DemoPcTemplate::default(),
            seed: 1,
            max_rounds: 5,
        };
        let resp = run_demo(req).expect("demo returns");
        assert!(resp.outcome.is_none());
        assert!(resp.error.unwrap().contains("ghost"));
    }

    const EVENT_YAML: &str = r#"
trait:
  id: ruin_dread
  name: Ruin Dread
  category: regional
  triggers:
    - id: on_ruins
      on: exploration.enter_hex
      when: "event.terrain == 'ruins'"
      do:
        - kind: apply_status
          params: { status_id: dread, duration_ticks: 4 }
        - kind: log
          params: { message: "dread settles in the ruins" }
---
status_effect:
  id: dread
  name: Dread
  default_duration_ticks: 4
  provides_tags: [dread]
  stacking: replace
"#;

    fn event(event_type: &str, terrain: &str) -> DemoEventEntry {
        let mut payload = JsonObject::new();
        payload.insert("self_id".into(), json!("pc"));
        payload.insert("terrain".into(), json!(terrain));
        DemoEventEntry {
            event_type: event_type.into(),
            payload,
        }
    }

    fn demo_pc() -> DemoEntity {
        serde_json::from_value(json!({
            "id": "pc", "hp": 10, "max_hp": 10, "ac": 12, "is_player": true,
            "traits": ["ruin_dread"]
        }))
        .unwrap()
    }

    #[test]
    fn run_event_demo_applies_status_on_enter_hex() {
        let req = DemoEventsRequest {
            yaml: EVENT_YAML.into(),
            entities: vec![demo_pc()],
            events: vec![event("exploration.enter_hex", "ruins")],
            seed: 1,
        };
        let resp = run_event_demo(req).expect("demo runs");
        assert!(resp.error.is_none(), "{:?}", resp.error);
        let outcome = resp.outcome.expect("outcome present");
        let text = serde_json::to_string(&outcome).unwrap();
        assert!(text.contains("dread settles"), "{text}");
        assert!(text.contains("dread"), "status applied: {text}");
    }

    #[test]
    fn run_event_demo_reports_compile_errors_without_running() {
        let req = DemoEventsRequest {
            yaml: "creature:\n  id: broken\n  name: Broken\n".into(),
            entities: vec![demo_pc()],
            events: vec![event("exploration.enter_hex", "ruins")],
            seed: 1,
        };
        let resp = run_event_demo(req).expect("demo returns");
        assert!(resp.outcome.is_none());
        assert!(resp.error.is_some());
    }

    #[test]
    fn run_event_demo_requires_events() {
        let req = DemoEventsRequest {
            yaml: EVENT_YAML.into(),
            entities: vec![demo_pc()],
            events: vec![],
            seed: 1,
        };
        let resp = run_event_demo(req).expect("demo returns");
        assert!(resp.outcome.is_none());
        assert!(resp.error.unwrap().contains("no events"));
    }
}
