//! End-to-end vertical slice: authored YAML compiles to IR and *drives* combat.
//!
//! This pins the first-increment promise — "make IR content drive the live game"
//! — across the whole spine: the compiler turns an envenomed-weapon pack into IR
//! records, the runtime content store + trigger runtime fire those records during
//! an isolated combat, and the authored `poisoned` status actually changes a
//! creature's state. If any link breaks (compiler, store, trigger sourcing,
//! dispatch, intent application), this test fails.

use harsh_core::combat::creation::create_combat;
use harsh_core::combat_runtime::AwarenessResult;
use harsh_core::character::Character;
use harsh_core::compiler::{compile, Catalogs, VerbRegistry};
use harsh_core::creature::CreatureData;
use harsh_core::db::WorldDatabase;
use harsh_core::db_schema::SCHEMA_SQL;
use harsh_core::runtime_content::{DemoCombatRunner, RuntimeContentStore};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::de::Deserialize;
use serde_json::Value;

/// The authored content: an envenomed dagger that grants a trait whose on-hit
/// trigger applies a `poisoned` status, which in turn drains hp when the poisoned
/// creature acts. Multi-document YAML — one record per document.
const SLICE_YAML: &str = r#"
item:
  id: envenomed_dagger
  name: Envenomed Dagger
  enc: 1
  cost: 25
  tags: [weapon, melee]
  damage: 1d4
  attribute: DEX
  weapon_tags: [light]
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
          params: { message: "envenom: the strike leaves the target poisoned" }
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
          params: { message: "poison bites the poisoned creature" }
"#;

fn parse_documents(yaml: &str) -> Vec<Value> {
    serde_yaml::Deserializer::from_str(yaml)
        .map(|doc| Value::deserialize(doc).expect("yaml document parses"))
        .filter(|v| !v.is_null())
        .collect()
}

fn seeded_db() -> WorldDatabase {
    let db = WorldDatabase::open_in_memory().expect("in-memory db");
    for stmt in SCHEMA_SQL.split(';') {
        let s = stmt.trim();
        if !s.is_empty() {
            let _ = db.execute(s, &[]);
        }
    }
    db
}

fn pc_with_dagger() -> Character {
    let mut pc = Character::new("Adept", "warrior");
    pc.id = "pc".into();
    pc.hp = 14;
    pc.max_hp = 14;
    pc.ac = 15;
    pc.attack_bonus = 5;
    pc.equipment = vec![serde_json::from_value(serde_json::json!({
        "item_id": "envenomed_dagger",
        "name": "Envenomed Dagger",
        "type": "weapon",
        "damage": "1d4"
    }))
    .unwrap()];
    pc
}

fn scrip_hound() -> CreatureData {
    serde_json::from_value(serde_json::json!({
        "id": "scrip_hound", "name": "Scrip-Hound", "hd": 2, "hp_per_hd": 4,
        "ac": 11, "attack_bonus": 1, "damage": "1d4", "behavior": "aggressive"
    }))
    .unwrap()
}

#[test]
fn authored_envenomed_weapon_compiles_and_drives_combat() {
    // 1. Compile the authored pack.
    let docs = parse_documents(SLICE_YAML);
    assert_eq!(docs.len(), 3, "three records authored");
    let report = compile(&docs, &VerbRegistry::builtin(), &Catalogs::default());
    assert!(
        report.syntax_errors.is_empty(),
        "syntax errors: {:?}",
        report.syntax_errors
    );
    assert!(
        report.unimplemented.is_empty(),
        "unimplemented: {:?}",
        report.unimplemented
    );
    assert_eq!(report.records.len(), 3, "all three records compiled");

    // 2. Project compiled IR into the runtime content store.
    let store = RuntimeContentStore::from_records(report.records);
    assert!(store.get_item("envenomed_dagger").is_some());
    assert!(store.get_trait("envenom_on_hit").is_some());
    assert!(store.get_status_effect("poisoned").is_some());

    // 3. Run an isolated combat: PC with the dagger vs a Scrip-Hound.
    let db = seeded_db();
    let mut rng = StdRng::seed_from_u64(42);
    let state = create_combat(
        &pc_with_dagger(),
        &[scrip_hound()],
        AwarenessResult::MutualAwareness,
        &mut rng,
        "",
        &[],
        1.0,
    );
    let mut runner = DemoCombatRunner::new(state, 42);
    let outcome = runner.run_to_completion(&db, &store, 40);

    // 4. The IR actually drove the game.
    assert!(outcome.errors.is_empty(), "runtime errors: {:?}", outcome.errors);
    assert!(!outcome.transcript.is_empty(), "combat produced turns");

    // (a) The envenom trigger fired — the target was poisoned at least once.
    let envenom_fired = outcome
        .transcript
        .iter()
        .any(|t| t.triggers.iter().any(|m| m.contains("poisoned")));
    assert!(envenom_fired, "envenom trigger should fire: {:#?}", outcome.transcript);

    // (b) The poison tick fired — its annotation appears.
    let poison_ticked = outcome
        .transcript
        .iter()
        .any(|t| t.triggers.iter().any(|m| m.contains("poison bites")));
    assert!(poison_ticked, "poison tick should fire: {:#?}", outcome.transcript);

    // (c) The hound died (hits + poison), and the player won.
    assert_eq!(outcome.winner, "player");
    let hound = outcome
        .combatants
        .iter()
        .find(|c| !c.is_player)
        .expect("hound present");
    assert!(!hound.alive, "the hound should be defeated");
}
