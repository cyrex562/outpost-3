//! Integration: creating a world binds its packs and compiles their IR content
//! into `ir_records`, so authored pack IR drives the world from creation.

use std::fs;
use std::path::PathBuf;

use harsh_core::content::IRRecordRepository;
use harsh_core::public_api::CreateWorldRequest;
use harsh_core::repositories::world_pack::WorldPackRepository;
use harsh_web::worldsvc::{create_world, open_world, ActorPaths};

/// The real repo content dir (for terrain/world-gen), relative to this crate.
fn repo_content_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
        .join("xwn-core")
        .join("content")
}

fn temp_root(tag: &str) -> PathBuf {
    let mut base = std::env::temp_dir();
    base.push(format!("harsh_pack_ir_world_{tag}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

/// Write a fixture pack carrying an envenomed-dagger IR set under content/ir/.
fn write_fixture_pack(packs_root: &std::path::Path) {
    let ir = packs_root.join("demo-pack").join("content").join("ir");
    fs::create_dir_all(&ir).unwrap();
    fs::write(
        ir.join("envenom.yaml"),
        r#"
item:
  id: envenomed_dagger
  name: Envenomed Dagger
  damage: 1d4
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
---
status_effect:
  id: poisoned
  name: Poisoned
  provides_tags: [poisoned]
  stacking: replace
"#,
    )
    .unwrap();
}

#[test]
fn creating_a_world_compiles_bound_pack_ir_into_ir_records() {
    let root = temp_root("ok");
    let worlds_dir = root.join("worlds");
    let packs_root = root.join("packs");
    fs::create_dir_all(&worlds_dir).unwrap();
    fs::create_dir_all(&packs_root).unwrap();
    write_fixture_pack(&packs_root);

    let paths = ActorPaths {
        worlds_dir: worlds_dir.clone(),
        content_dir: repo_content_dir(),
        packs_root,
    };
    let req = CreateWorldRequest {
        name: "Pack IR World".into(),
        width: 6,
        height: 6,
        seed: Some(7),
        pack_ids: Some(vec!["demo-pack".into()]),
    };

    let (db, info) = create_world(&paths, &req).expect("world created");
    let file = info["file"].as_str().expect("file name").to_string();
    drop(db);

    // Re-open the created world and inspect its persisted state.
    let (db, _) = open_world(&paths, &file).expect("world opens");

    // The pack binding was persisted.
    let bindings = WorldPackRepository::new(&db).get_pack_list().unwrap();
    assert!(
        bindings.iter().any(|b| b.pack_id == "demo-pack"),
        "demo-pack should be bound: {bindings:?}"
    );

    // The pack's IR content compiled into ir_records.
    let repo = IRRecordRepository::new(&db);
    let counts = repo.counts().unwrap();
    assert_eq!(counts.get("item").copied().unwrap_or(0), 1, "{counts:?}");
    assert_eq!(counts.get("trait").copied().unwrap_or(0), 1, "{counts:?}");
    assert_eq!(
        counts.get("status_effect").copied().unwrap_or(0),
        1,
        "{counts:?}"
    );
    assert!(repo.get("item", "envenomed_dagger").unwrap().is_some());

    drop(db);
    let _ = fs::remove_dir_all(&root);
}
