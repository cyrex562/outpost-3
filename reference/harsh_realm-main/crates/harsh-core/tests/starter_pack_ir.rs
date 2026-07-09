//! Validates the shipped xwn-core starter IR pack: it compiles cleanly and
//! drives combat end-to-end (the shard-lance lacerates and the wound drains hp).
//!
//! This pins the canonical example so it can't silently rot — if the starter
//! content stops compiling or stops firing, this fails.

use std::path::PathBuf;

use harsh_core::character::Character;
use harsh_core::combat::creation::create_combat;
use harsh_core::combat_runtime::AwarenessResult;
use harsh_core::creature::CreatureData;
use harsh_core::db::WorldDatabase;
use harsh_core::db_schema::SCHEMA_SQL;
use harsh_core::runtime_content::{compile_pack_ir, DemoCombatRunner, RuntimeContentStore};
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Repo content root (the packs root), relative to this crate.
fn packs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

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

#[test]
fn xwn_core_starter_ir_compiles_and_drives_combat() {
    // 1. The shipped starter content compiles with no diagnostics.
    let (records, diagnostics) = compile_pack_ir(&packs_root(), &["xwn-core".to_string()]);
    assert!(diagnostics.is_empty(), "starter pack diagnostics: {diagnostics:?}");
    assert!(!records.is_empty(), "xwn-core ir/ should compile some records");

    let store = RuntimeContentStore::from_records(records);
    assert!(store.get_item("weapon.shard_lance").is_some(), "shard lance present");
    assert!(store.get_trait("shard_laceration").is_some(), "trait present");
    assert!(store.get_status_effect("lacerated").is_some(), "status present");
    assert!(store.get_trait("caustic_bite").is_some(), "intrinsic creature trait present");
    assert!(store.get_status_effect("corroded").is_some(), "intrinsic status present");
    let crawler = store
        .get_creature("ash_crawler")
        .cloned()
        .expect("ash crawler present");

    // 2. A PC wielding the shard lance lacerates the crawler, which then bleeds.
    let mut pc = Character::new("Reaver", "warrior");
    pc.id = "pc".into();
    pc.hp = 16;
    pc.max_hp = 16;
    pc.ac = 15;
    pc.attack_bonus = 6;
    pc.equipment = vec![serde_json::from_value(serde_json::json!({
        "item_id": "weapon.shard_lance", "name": "Shard Lance", "type": "weapon", "damage": "1d6"
    }))
    .unwrap()];

    let creature: CreatureData = {
        let (data, _warn) = harsh_core::runtime_content::ir_creature_to_data(&crawler);
        data
    };
    // The intrinsic trait survives the IR → engine adapter.
    assert_eq!(creature.traits, vec!["caustic_bite"]);
    // HR-761: the IR damage-model fields are carried (no longer dropped).
    assert_eq!(creature.defenses.get("dr"), Some(&1));
    assert_eq!(creature.pools.len(), 1);
    assert_eq!(creature.pools[0].id, "carapace");

    let db = seeded_db();
    let mut rng = StdRng::seed_from_u64(11);
    let state = create_combat(&pc, &[creature], AwarenessResult::MutualAwareness, &mut rng, "", &[], 1.0);
    // ...and lands on the crawler combatant so its bite can fire in combat.
    let crawler_combatant = state.combatants.iter().find(|c| !c.is_player).unwrap();
    assert_eq!(crawler_combatant.traits, vec!["caustic_bite"]);
    // Pools/defenses flow IR → adapter → combatant.
    assert_eq!(crawler_combatant.defenses.get("dr"), Some(&1));
    assert_eq!(crawler_combatant.pools.iter().filter(|p| p.id == "carapace").count(), 1);
    let mut runner = DemoCombatRunner::new(state, 11);
    let outcome = runner.run_to_completion(&db, &store, 40);

    assert!(outcome.errors.is_empty(), "runtime errors: {:?}", outcome.errors);
    let lacerated = outcome
        .transcript
        .iter()
        .any(|t| t.triggers.iter().any(|m| m.to_lowercase().contains("lacerat")));
    assert!(lacerated, "shard laceration should fire: {:#?}", outcome.transcript);
    assert_eq!(outcome.winner, "player");
}
