//! Domain event handlers for GM-owned subsystems.
//!
//! Ported from:
//!  - `src/harsh_realm/gm/gm_state_event_handlers.py`
//!  - `src/harsh_realm/gm/combat_event_handlers.py`
//!  - `src/harsh_realm/gm/shopping_event_handlers.py`
//!  - `src/harsh_realm/gm/social_event_handlers.py`

use rand::thread_rng;
use serde_json::Value as JsonValue;

use crate::advancement::AdvancementSystem;
use crate::character::Character;
use crate::db::WorldDatabase;
use crate::domain_events::DomainEventDispatcher;
use crate::events::GameEvent;
use crate::gm::scenes::disposition_label;
use crate::payloads::notices_combat::{
    CharacterHpChangedNotice, ShoppingPurchaseNotice, ShoppingSaleNotice,
};
use crate::payloads::notices_world::{
    CharacterLevelUpNotice, CharacterRespawnNotice, CharacterXpGainedNotice,
    InventoryAmmoConsumedNotice, InventoryItemGivenNotice, InventoryItemLostNotice,
    SocialDispositionChangeNotice, SocialHealerNotice,
};
use crate::payloads::requests::{
    CharacterRespawnRequested, CharacterUpdateRequested, CombatConsumeAmmoRequested,
    CombatFleeRequested, CombatHarvestRequested, CombatTakeDamageRequested,
    CombatUpdateCharacterRequested, CombatUseItemRequested, CombatVictoryRequested,
    ShoppingPurchaseRequested, ShoppingSaleRequested, SocialDispositionUpdateRequested,
    SocialHealerRequested, StatePersistRequest,
};
use crate::repositories::cell::CellRepository;
use crate::repositories::entity::EntityRepository;
use crate::repositories::gm_state::GMStateRepository;
use crate::runtime::{InventoryItemRecord, JsonObject};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn narrate_event(tick: i64, text: impl Into<String>) -> GameEvent {
    let mut data = JsonObject::new();
    data.insert("text".into(), JsonValue::String(text.into()));
    GameEvent::new(tick, "gm.narrate", data).with_source("gm")
}

fn payload_event<T: serde::Serialize>(tick: i64, event_type: &str, payload: &T) -> GameEvent {
    let data: JsonObject =
        serde_json::from_value(serde_json::to_value(payload).unwrap_or_default())
            .unwrap_or_default();
    GameEvent::new(tick, event_type, data)
}

fn char_from_snapshot(snapshot: &crate::payloads::notices_combat::CharacterSnapshot) -> Character {
    let json = serde_json::to_value(snapshot).unwrap_or_default();
    serde_json::from_value(json).unwrap_or_default()
}

/// Deserialize an event's payload, logging (not swallowing silently) on failure.
/// Returns None so the caller can no-op safely, but leaves a diagnostic trace.
fn parse_payload<T: serde::de::DeserializeOwned>(event: &GameEvent) -> Option<T> {
    match serde_json::from_value::<T>(serde_json::Value::Object(event.data.clone())) {
        Ok(v) => Some(v),
        Err(e) => {
            eprintln!(
                "event_handler: '{}' failed to deserialize {}: {e}",
                event.event_type,
                std::any::type_name::<T>()
            );
            None
        }
    }
}

// ---------------------------------------------------------------------------
// GM State event handler
// ---------------------------------------------------------------------------

fn handle_state_persist_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let req = match parse_payload::<StatePersistRequest>(event) {
        Some(r) => r,
        None => return vec![],
    };
    let repo = GMStateRepository::new(db);
    let _ = repo.set_value(&req.key, &req.value);
    vec![]
}

// ---------------------------------------------------------------------------
// Combat event handlers
// ---------------------------------------------------------------------------

fn handle_victory_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CombatVictoryRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };

    let mut character = char_from_snapshot(&data.character_data);
    let mut events: Vec<GameEvent> = Vec::new();

    if data.xp_gained > 0 {
        character.xp += data.xp_gained;
        let xp_notice = CharacterXpGainedNotice {
            xp_gained: data.xp_gained,
            new_total: character.xp,
            xp_next: character.xp_next,
        };
        events.push(
            payload_event(event.tick, "character.xp_gained", &xp_notice).with_source("combat"),
        );
    }

    if character.xp >= character.xp_next && character.level < 10 {
        let mut adv = AdvancementSystem::new(Some(db));
        let mut rng = thread_rng();
        let level_result = adv.apply_level_up(&mut character, &mut rng);
        let lvl_notice = CharacterLevelUpNotice {
            new_level: level_result.new_level,
            new_max_hp: level_result.new_max_hp,
            hp_gained: level_result.hp_gained,
        };
        events.push(
            payload_event(event.tick, "character.level_up", &lvl_notice).with_source("combat"),
        );
    }

    for item_data in &data.items_gained {
        character.equipment.push(item_data.clone());
        let item_notice = InventoryItemGivenNotice {
            character_id: data.character_id.clone(),
            item: item_data.clone(),
        };
        events.push(
            payload_event(event.tick, "inventory.item_given", &item_notice)
                .with_source("combat"),
        );
    }

    if data.currency_gained > 0 {
        let mut coin = JsonObject::new();
        coin.insert(
            "name".into(),
            JsonValue::String(format!("{} coin", data.currency_gained)),
        );
        coin.insert("type".into(), JsonValue::String("currency".into()));
        coin.insert(
            "value".into(),
            JsonValue::Number(data.currency_gained.into()),
        );
        let mut amount_inner = JsonObject::new();
        amount_inner.insert(
            "amount".into(),
            JsonValue::Number(data.currency_gained.into()),
        );
        coin.insert("data".into(), JsonValue::Object(amount_inner));
        character.equipment.push(coin.clone());
        let coin_notice = InventoryItemGivenNotice {
            character_id: data.character_id.clone(),
            item: coin,
        };
        events.push(
            payload_event(event.tick, "inventory.item_given", &coin_notice).with_source("combat"),
        );
    }

    // HR-786: leave item drops on a searchable corpse at the player's cell
    // (a low-DC search reveals it; gold already auto-collected above).
    if !data.corpse_items.is_empty() {
        let coord = crate::grid::GridCoord {
            q: character.position_q,
            r: character.position_r,
        };
        let cell_repo = CellRepository::new(db);
        if let Ok(Some(cell)) = cell_repo.fetch_cell(coord) {
            let mut cell_data = cell.data.clone();
            let corpse = crate::loot_source::LootSource {
                id: format!("corpse_{}", event.tick),
                kind: crate::loot_source::KIND_CORPSE.to_string(),
                name: "fallen foe's corpse".to_string(),
                contents: data.corpse_items.clone(),
                gold: 0,
                difficulty: 4,
                searched: false,
                empty: false,
            };
            crate::loot_source::push_source(&mut cell_data, corpse);
            let _ = cell_repo.save_cell_data(coord.q as i64, coord.r as i64, &cell_data);
        }
    }

    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, true, false);
    events
}

fn handle_flee_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CombatFleeRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let mut character = char_from_snapshot(&data.character_data);
    let mut events: Vec<GameEvent> = Vec::new();

    if data.damage_taken > 0 {
        character.hp = (character.hp - data.damage_taken).max(0);
        let notice = CharacterHpChangedNotice {
            hp: character.hp,
            max_hp: character.max_hp,
        };
        events.push(payload_event(event.tick, "character.hp_changed", &notice).with_source("combat"));
    }

    if let Some(ref lost_item) = data.item_lost {
        let mut new_equip = Vec::new();
        let mut removed = false;
        for item in &character.equipment {
            if !removed && item.get("name").and_then(|v| v.as_str()) == Some(lost_item) {
                removed = true;
                let lost_notice = InventoryItemLostNotice {
                    character_id: data.character_id.clone(),
                    item_name: lost_item.clone(),
                };
                events.push(
                    payload_event(event.tick, "inventory.item_lost", &lost_notice)
                        .with_source("combat"),
                );
            } else {
                new_equip.push(item.clone());
            }
        }
        character.equipment = new_equip;
    }

    character.position_q = data.destination_q;
    character.position_r = data.destination_r;

    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, true, true, false);
    events
}

fn handle_consume_ammo_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CombatConsumeAmmoRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let mut character = char_from_snapshot(&data.character_data);

    let mut new_equip: Vec<JsonObject> = Vec::new();
    let mut found = false;
    let mut remaining = 0i32;
    for raw in &character.equipment {
        if let Ok(mut item) = serde_json::from_value::<InventoryItemRecord>(
            JsonValue::Object(raw.clone()),
        ) {
            if !found
                && (item.item_id.as_deref() == Some(&data.ammo_type)
                    || item.name.to_lowercase() == data.ammo_type.to_lowercase())
            {
                found = true;
                item.quantity -= 1;
                if item.quantity > 0 {
                    remaining = item.quantity as i32;
                    if let Ok(JsonValue::Object(obj)) = serde_json::to_value(&item) {
                        new_equip.push(obj);
                    }
                }
            } else {
                new_equip.push(raw.clone());
            }
        } else {
            new_equip.push(raw.clone());
        }
    }
    character.equipment = new_equip;

    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, false, false);

    let ammo_notice = InventoryAmmoConsumedNotice {
        character_id: data.character_id.clone(),
        ammo_type: data.ammo_type.clone(),
        remaining,
    };
    vec![payload_event(event.tick, "inventory.ammo_consumed", &ammo_notice).with_source("combat")]
}

fn handle_harvest_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CombatHarvestRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let mut character = char_from_snapshot(&data.character_data);

    if !data.success {
        return vec![];
    }

    let mut item_data = JsonObject::new();
    item_data.insert("name".into(), JsonValue::String(data.material.clone()));
    item_data.insert("type".into(), JsonValue::String("material".into()));
    item_data.insert(
        "description".into(),
        JsonValue::String(format!("Harvested {}", data.material)),
    );
    character.equipment.push(item_data.clone());

    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, false, false);

    let harvest_notice = InventoryItemGivenNotice {
        character_id: data.character_id.clone(),
        item: item_data,
    };
    vec![payload_event(event.tick, "inventory.item_given", &harvest_notice).with_source("combat")]
}

fn handle_use_item_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CombatUseItemRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let mut character = char_from_snapshot(&data.character_data);

    let mut new_equip: Vec<JsonObject> = Vec::new();
    let mut removed = false;
    for item in &character.equipment {
        if !removed && item.get("name").and_then(|v| v.as_str()) == Some(&data.item_name) {
            removed = true;
        } else {
            new_equip.push(item.clone());
        }
    }
    character.equipment = new_equip;

    if data.hp_restored > 0 {
        character.hp = (character.hp + data.hp_restored).min(character.max_hp);
    }

    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, true, false);

    let hp_notice = CharacterHpChangedNotice {
        hp: character.hp,
        max_hp: character.max_hp,
    };
    let lost_notice = InventoryItemLostNotice {
        character_id: data.character_id.clone(),
        item_name: data.item_name.clone(),
    };

    vec![
        payload_event(event.tick, "character.hp_changed", &hp_notice).with_source("combat"),
        payload_event(event.tick, "inventory.item_lost", &lost_notice).with_source("combat"),
    ]
}

fn handle_take_damage_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CombatTakeDamageRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let mut character = char_from_snapshot(&data.character_data);
    character.hp = (character.hp - data.damage).max(0);

    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, true, false);

    let notice = CharacterHpChangedNotice {
        hp: character.hp,
        max_hp: character.max_hp,
    };
    vec![payload_event(event.tick, "character.hp_changed", &notice).with_source("combat")]
}

fn handle_update_character_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CombatUpdateCharacterRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let character = char_from_snapshot(&data.character_data);
    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, true, false);

    let notice = CharacterHpChangedNotice {
        hp: character.hp,
        max_hp: character.max_hp,
    };
    vec![payload_event(event.tick, "character.hp_changed", &notice).with_source("combat")]
}

fn handle_respawn_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CharacterRespawnRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let mut character = char_from_snapshot(&data.character_data);

    character.hp = data.new_hp.max(0).min(character.max_hp);

    if data.xp_loss > 0 {
        character.xp = (character.xp - data.xp_loss).max(0);
    }

    if let Some(ref lost_name) = data.lost_item {
        let mut new_equip: Vec<JsonObject> = Vec::new();
        let mut removed = false;
        let mut lost_item_data: Option<JsonObject> = None;
        for item in &character.equipment {
            if !removed && item.get("name").and_then(|v| v.as_str()) == Some(lost_name) {
                removed = true;
                lost_item_data = Some(item.clone());
            } else {
                new_equip.push(item.clone());
            }
        }
        character.equipment = new_equip;

        if let Some(item_data) = lost_item_data {
            let cell_repo = CellRepository::new(db);
            let _ = cell_repo.save_death_markers(data.death_q as i64, data.death_r as i64, vec![item_data]);
        }
    }

    character.position_q = data.settlement_q;
    character.position_r = data.settlement_r;

    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, true, true, false);

    let respawn_notice = CharacterRespawnNotice {
        name: character.name.clone(),
        settlement_q: data.settlement_q,
        settlement_r: data.settlement_r,
        new_hp: character.hp,
        xp_lost: data.xp_loss,
        item_lost: data.lost_item.clone(),
    };
    let hp_notice = CharacterHpChangedNotice {
        hp: character.hp,
        max_hp: character.max_hp,
    };
    vec![
        payload_event(event.tick, "character.respawn", &respawn_notice).with_source("respawn"),
        payload_event(event.tick, "character.hp_changed", &hp_notice).with_source("respawn"),
    ]
}

// ---------------------------------------------------------------------------
// Shopping event handlers
// ---------------------------------------------------------------------------

fn handle_purchase_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<ShoppingPurchaseRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let mut character = char_from_snapshot(&data.character_data);
    // HR-790: actually add the purchased item to inventory. The snapshot is taken
    // before the buy, and nothing else persists the item — without this push the
    // player pays gold and receives nothing.
    character.equipment.push(data.item.clone());
    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, false, false);

    let item_name = data
        .item
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let notice = ShoppingPurchaseNotice {
        item: item_name,
        price: data.price,
        gold_remaining: data.gold_remaining,
    };
    vec![payload_event(event.tick, "shopping.purchase", &notice).with_source("shopping")]
}

fn handle_sale_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<ShoppingSaleRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let character = char_from_snapshot(&data.character_data);
    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, false, false);

    let notice = ShoppingSaleNotice {
        item: data.item_name.clone(),
        price: data.price,
        gold_total: data.gold_total,
    };
    vec![payload_event(event.tick, "shopping.sale", &notice).with_source("shopping")]
}

// ---------------------------------------------------------------------------
// Social event handlers
// ---------------------------------------------------------------------------

fn handle_disposition_update_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<SocialDispositionUpdateRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };

    let old_disposition = db
        .fetch_one(
            "SELECT disposition FROM npc_state WHERE entity_id = ?",
            &[&data.entity_id],
        )
        .ok()
        .flatten()
        .and_then(|row| row.get("disposition").and_then(|v| v.as_i64()))
        .unwrap_or(0) as i32;

    let repo = EntityRepository::new(db);
    let _ = repo.update_npc_disposition(&data.entity_id, data.new_disposition as i64);

    // HR-778: resolve NPC display name for the enriched notice.
    // Prefer the name carried in the request (no DB hit); fall back to an
    // entity-table lookup; use the entity_id as a last resort.
    let npc_display_name = data
        .npc_name
        .filter(|s| !s.is_empty())
        .or_else(|| {
            repo.load_entity_record(&data.entity_id)
                .ok()
                .flatten()
                .map(|rec| rec.name)
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| data.entity_id.clone());

    let notice = SocialDispositionChangeNotice {
        entity_id: data.entity_id.clone(),
        npc: npc_display_name,
        old_score: old_disposition,
        new_score: data.new_disposition,
        old_mood: disposition_label(old_disposition).to_string(),
        new_mood: disposition_label(data.new_disposition).to_string(),
        reason: "interaction".to_string(),
    };
    vec![payload_event(event.tick, "social.disposition_change", &notice).with_source("social")]
}

fn handle_healer_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<SocialHealerRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let character = char_from_snapshot(&data.character_data);
    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, true, false);

    let notice = SocialHealerNotice {
        hp_restored: data.hp_restored,
        cost: data.gold_spent,
        npc_name: data.npc_name.clone(),
    };
    vec![payload_event(event.tick, "social.healer", &notice).with_source("social")]
}

fn handle_character_update_requested(event: &GameEvent, db: &WorldDatabase) -> Vec<GameEvent> {
    let data = match parse_payload::<CharacterUpdateRequested>(event) {
        Some(d) => d,
        None => return vec![],
    };
    let character = char_from_snapshot(&data.character_data);
    let repo = EntityRepository::new(db);
    let _ = repo.save_character(&character, false, false, false);
    vec![]
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all GM domain event handlers on `dispatcher`.
pub fn register_gm_event_handlers<'h>(
    dispatcher: &mut DomainEventDispatcher<'h>,
    db: &'h WorldDatabase,
) {
    // GM state
    dispatcher.subscribe(
        "gm.state_persist_requested",
        Box::new(move |ev| handle_state_persist_requested(ev, db)),
    );

    // Combat
    dispatcher.subscribe(
        "combat.victory_requested",
        Box::new(move |ev| handle_victory_requested(ev, db)),
    );
    dispatcher.subscribe(
        "combat.flee_requested",
        Box::new(move |ev| handle_flee_requested(ev, db)),
    );
    dispatcher.subscribe(
        "combat.consume_ammo_requested",
        Box::new(move |ev| handle_consume_ammo_requested(ev, db)),
    );
    dispatcher.subscribe(
        "combat.harvest_requested",
        Box::new(move |ev| handle_harvest_requested(ev, db)),
    );
    dispatcher.subscribe(
        "combat.use_item_requested",
        Box::new(move |ev| handle_use_item_requested(ev, db)),
    );
    dispatcher.subscribe(
        "combat.take_damage_requested",
        Box::new(move |ev| handle_take_damage_requested(ev, db)),
    );
    dispatcher.subscribe(
        "combat.update_character_requested",
        Box::new(move |ev| handle_update_character_requested(ev, db)),
    );
    dispatcher.subscribe(
        "character.respawn_requested",
        Box::new(move |ev| handle_respawn_requested(ev, db)),
    );

    // Shopping
    dispatcher.subscribe(
        "shopping.purchase_requested",
        Box::new(move |ev| handle_purchase_requested(ev, db)),
    );
    dispatcher.subscribe(
        "shopping.sale_requested",
        Box::new(move |ev| handle_sale_requested(ev, db)),
    );

    // Social
    dispatcher.subscribe(
        "social.disposition_update_requested",
        Box::new(move |ev| handle_disposition_update_requested(ev, db)),
    );
    dispatcher.subscribe(
        "social.healer_requested",
        Box::new(move |ev| handle_healer_requested(ev, db)),
    );
    dispatcher.subscribe(
        "social.update_character_requested",
        Box::new(move |ev| handle_character_update_requested(ev, db)),
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::WorldDatabase;
    use crate::db_schema::SCHEMA_SQL;
    use crate::payloads::notices_combat::CharacterSnapshot;
    use std::collections::BTreeMap;

    fn open_db() -> WorldDatabase {
        let db = WorldDatabase::open_in_memory().unwrap();
        for stmt in SCHEMA_SQL.split(';') {
            let s = stmt.trim();
            if !s.is_empty() {
                let _ = db.execute(s, &[]);
            }
        }
        db
    }

    fn snapshot() -> CharacterSnapshot {
        CharacterSnapshot {
            id: "char_1".to_string(),
            name: "Hero".to_string(),
            character_class: "warrior".to_string(),
            level: 1,
            xp: 0,
            xp_next: 1500,
            attributes: BTreeMap::new(),
            attr_mods: BTreeMap::new(),
            skills: BTreeMap::new(),
            hp: 10,
            max_hp: 10,
            ac: 12,
            attack_bonus: 1,
            physical_save: 15,
            evasion_save: 15,
            mental_save: 15,
            equipment: vec![],
            class_abilities: serde_json::Map::new(),
            position_q: 0,
            position_r: 0,
        }
    }

    fn mk_event(tick: i64, event_type: &str, payload: impl serde::Serialize) -> GameEvent {
        let data: JsonObject =
            serde_json::from_value(serde_json::to_value(&payload).unwrap()).unwrap();
        GameEvent::new(tick, event_type, data)
    }

    #[test]
    fn victory_leaves_item_drops_on_a_searchable_corpse() {
        // HR-786: combat item drops go onto a searchable corpse on the player's
        // cell (unsearched, low DC); gold still auto-collects; items are NOT
        // added straight to inventory.
        let db = open_db();
        db.execute(
            "INSERT OR REPLACE INTO cells (q, r, terrain, features, explored, data) \
             VALUES (0, 0, 'plains', '[]', 1, '{}')",
            &[],
        )
        .unwrap();

        let relic: JsonObject = serde_json::from_value(
            serde_json::json!({ "name": "Pretech Relic", "type": "relic", "value": 150 }),
        )
        .unwrap();
        let req = CombatVictoryRequested {
            character_id: "char_1".to_string(),
            character_data: snapshot(),
            xp_gained: 0,
            items_gained: vec![],
            currency_gained: 25,
            corpse_items: vec![relic],
            harvestable: None,
        };
        let events = handle_victory_requested(&mk_event(5, "combat.victory_requested", &req), &db);

        // Gold auto-collects → a coin inventory.item_given is emitted.
        assert!(
            events.iter().any(|e| e.event_type == "inventory.item_given"),
            "gold still auto-collects to inventory"
        );

        // The relic waits on an unsearched corpse (DC 4) on the cell.
        let cell = crate::repositories::cell::CellRepository::new(&db)
            .fetch_cell(crate::grid::GridCoord { q: 0, r: 0 })
            .unwrap()
            .unwrap();
        let sources = crate::loot_source::read_sources(&cell.data);
        let corpse = sources
            .iter()
            .find(|s| s.kind == crate::loot_source::KIND_CORPSE)
            .expect("a corpse loot source is left on the cell");
        assert!(!corpse.searched, "the corpse must be searched to reveal");
        assert_eq!(corpse.difficulty, 4, "corpses use a low search DC");
        assert_eq!(corpse.contents.len(), 1);
        assert_eq!(
            corpse.contents[0].get("name").and_then(|v| v.as_str()),
            Some("Pretech Relic")
        );
    }

    #[test]
    fn state_persist_handler_writes_key() {
        let db = open_db();
        let req = StatePersistRequest {
            key: "scene".to_string(),
            value: "exploration".to_string(),
        };
        let event = mk_event(0, "gm.state_persist_requested", &req);
        let result = handle_state_persist_requested(&event, &db);
        assert!(result.is_empty());
        let repo = GMStateRepository::new(&db);
        assert_eq!(repo.get_value("scene").unwrap(), Some("exploration".to_string()));
    }

    #[test]
    fn take_damage_handler_reduces_hp() {
        let db = open_db();
        let req = CombatTakeDamageRequested {
            character_id: "char_1".to_string(),
            character_data: snapshot(),
            damage: 4,
            source: "combat".to_string(),
        };
        let event = mk_event(1, "combat.take_damage_requested", &req);
        let events = handle_take_damage_requested(&event, &db);
        assert!(!events.is_empty());
        assert_eq!(events[0].event_type, "character.hp_changed");
    }

    /// HR-773 end-to-end: the conformed dungeon-trap payload, when run through
    /// `handle_take_damage_requested`, must actually apply the rolled damage AND
    /// persist it. Persist a character at hp=10, feed a real
    /// `CombatTakeDamageRequested { damage: 3, source: "pit" }` (the exact shape
    /// the fixed `trigger_traps` now emits), reload from the DB, and assert hp
    /// dropped to 7. Guards against the handler silently regressing to a no-op.
    #[test]
    fn hr773_conformed_trap_payload_applies_and_persists_damage() {
        let db = open_db();

        // Persist a living character with hp 10 / max_hp 10.
        let mut character = Character::new("Trapped Hero", "warrior");
        character.id = "char_1".to_string();
        character.hp = 10;
        character.max_hp = 10;
        let repo = EntityRepository::new(&db);
        repo.create_character(&character).unwrap();

        // Build the same payload the fixed dungeon emit produces, with a
        // deterministic flat damage of 3.
        let mut snap = snapshot();
        snap.hp = 10;
        snap.max_hp = 10;
        let req = CombatTakeDamageRequested {
            character_id: "char_1".to_string(),
            character_data: snap,
            damage: 3,
            source: "pit".to_string(),
        };
        let event = mk_event(1, "combat.take_damage_requested", &req);

        let events = handle_take_damage_requested(&event, &db);
        assert_eq!(events[0].event_type, "character.hp_changed");

        // Reload from the DB and assert the damage was actually applied + saved.
        let reloaded = repo.load_first_character().unwrap().unwrap();
        assert_eq!(reloaded.hp, 7, "trap damage must reduce persisted hp from 10 to 7");
    }

    #[test]
    fn harvest_success_gives_item() {
        let db = open_db();
        let req = CombatHarvestRequested {
            character_id: "char_1".to_string(),
            character_data: snapshot(),
            material: "wolf_pelt".to_string(),
            success: true,
            narration: "You harvest the pelt.".to_string(),
        };
        let event = mk_event(1, "combat.harvest_requested", &req);
        let events = handle_harvest_requested(&event, &db);
        assert!(!events.is_empty());
        assert_eq!(events[0].event_type, "inventory.item_given");
    }

    #[test]
    fn harvest_failure_emits_nothing() {
        let db = open_db();
        let req = CombatHarvestRequested {
            character_id: "char_1".to_string(),
            character_data: snapshot(),
            material: "wolf_pelt".to_string(),
            success: false,
            narration: "You botch the harvest.".to_string(),
        };
        let event = mk_event(1, "combat.harvest_requested", &req);
        let events = handle_harvest_requested(&event, &db);
        assert!(events.is_empty());
    }

    /// HR-773 regression: the old `trigger_traps` emit used fields `damage_expr`,
    /// `hazard_id`, `hazard_type` instead of `damage` + `character_data`.
    /// `serde_json::from_value::<CombatTakeDamageRequested>` returned `Err`, so
    /// `handle_take_damage_requested` returned `vec![]` — damage was silently lost.
    #[test]
    fn hr773_old_dungeon_trap_payload_makes_handler_a_no_op() {
        let db = open_db();
        // Reproduce the old hand-built payload from trigger_traps pre-fix.
        let mut data = JsonObject::new();
        data.insert(
            "character_id".to_string(),
            JsonValue::String("char_1".to_string()),
        );
        data.insert(
            "damage_expr".to_string(),
            JsonValue::String("1d6".to_string()),
        );
        data.insert(
            "hazard_id".to_string(),
            JsonValue::String("room_trap:0".to_string()),
        );
        data.insert(
            "hazard_type".to_string(),
            JsonValue::String("pit".to_string()),
        );
        let old_event = GameEvent::new(1, "combat.take_damage_requested", data);
        let result = handle_take_damage_requested(&old_event, &db);
        assert!(
            result.is_empty(),
            "HR-773: old dungeon trap payload causes handler to return vec![] (damage silently lost)"
        );
    }

    /// HR-778 regression: `handle_disposition_update_requested` must emit a
    /// `social.disposition_change` payload that includes the NPC display name
    /// (`npc`), `old_mood`, and `new_mood` fields — the exact keys the frontend
    /// `_websocketHandlers.ts` reads.  Without the fix these fields are absent,
    /// so the chat line renders as "NPC: →  (±N)" instead of the NPC's name and
    /// the human-readable mood labels.
    ///
    /// The test proves *fail-without / pass-with*: before the fix the `npc`,
    /// `old_mood`, and `new_mood` keys did not exist in the emitted JSON; after
    /// the fix they contain the correct non-empty strings.
    #[test]
    fn hr778_disposition_change_notice_includes_npc_name_and_mood_labels() {
        let db = open_db();
        let repo = EntityRepository::new(&db);

        // Persist a living NPC entity with a known display name and an initial
        // disposition of 0 (indifferent).
        let entity_id = "npc_tavernkeep_01";
        let npc_name = "Greta the Tavernkeep";
        let mut entity_data = JsonObject::new();
        entity_data.insert("disposition".into(), JsonValue::from(0i64));
        repo.create_entity(entity_id, "npc", npc_name, 5, 5, &entity_data)
            .expect("insert NPC entity");

        // Seed the npc_state row so the old disposition lookup succeeds.
        // (create_entity calls sync_typed_state which inserts into npc_state when
        // entity_type == "npc"; but disposition may not be seeded — so insert/
        // replace directly to be safe.)
        let _ = db.execute(
            "INSERT OR REPLACE INTO npc_state (entity_id, disposition) VALUES (?, ?)",
            &[&entity_id, &0i64],
        );

        // Build a `social.disposition_update_requested` event that carries the NPC
        // name — score goes 0 (indifferent) → 3 (helpful), crossing a mood boundary.
        let req = SocialDispositionUpdateRequested {
            entity_id: entity_id.to_string(),
            new_disposition: 3,
            npc_name: Some(npc_name.to_string()),
        };
        let event = mk_event(1, "social.disposition_update_requested", &req);

        let events = handle_disposition_update_requested(&event, &db);

        assert_eq!(events.len(), 1, "exactly one event emitted");
        let emitted = &events[0];
        assert_eq!(emitted.event_type, "social.disposition_change");

        // Deserialize the payload and assert all three enriched fields.
        let notice: SocialDispositionChangeNotice =
            serde_json::from_value(serde_json::Value::Object(emitted.data.clone()))
                .expect("payload must deserialize into SocialDispositionChangeNotice");

        assert_eq!(
            notice.npc, npc_name,
            "HR-778: npc field must be the NPC display name, not the entity id or 'NPC'"
        );
        assert_eq!(
            notice.old_score, 0,
            "old_score must reflect the persisted disposition"
        );
        assert_eq!(
            notice.new_score, 3,
            "new_score must match the requested value"
        );
        // HR-785: mood labels must match the social-scene narration casing
        // (Title Case, from social_support::disposition_label).
        assert_eq!(
            notice.old_mood, "Indifferent",
            "HR-778: old_mood must be the disposition label for score 0"
        );
        assert_eq!(
            notice.new_mood, "Helpful",
            "HR-778: new_mood must be the disposition label for score 3"
        );
    }

    /// HR-778 complementary: when `npc_name` is absent from the request (legacy /
    /// third-party callers), the handler must fall back to the entity-table lookup
    /// and still produce a non-empty `npc` field.
    #[test]
    fn hr778_disposition_change_notice_falls_back_to_entity_name_lookup() {
        let db = open_db();
        let repo = EntityRepository::new(&db);

        let entity_id = "npc_blacksmith_02";
        let npc_name = "Brond the Blacksmith";
        let mut entity_data = JsonObject::new();
        entity_data.insert("disposition".into(), JsonValue::from(0i64));
        repo.create_entity(entity_id, "npc", npc_name, 3, 3, &entity_data)
            .expect("insert NPC entity");

        let _ = db.execute(
            "INSERT OR REPLACE INTO npc_state (entity_id, disposition) VALUES (?, ?)",
            &[&entity_id, &0i64],
        );

        // No `npc_name` in the request — simulates a caller that doesn't pass it.
        let req = SocialDispositionUpdateRequested {
            entity_id: entity_id.to_string(),
            new_disposition: 1,
            npc_name: None,
        };
        let event = mk_event(2, "social.disposition_update_requested", &req);
        let events = handle_disposition_update_requested(&event, &db);

        assert_eq!(events.len(), 1);
        let notice: SocialDispositionChangeNotice =
            serde_json::from_value(serde_json::Value::Object(events[0].data.clone()))
                .expect("payload deserializes");

        assert_eq!(
            notice.npc, npc_name,
            "HR-778: fallback to entity name lookup must yield the stored display name"
        );
        assert_eq!(notice.old_mood, "Indifferent");
        assert_eq!(notice.new_mood, "Sociable");
    }

    #[test]
    fn register_handlers_compiles() {
        let db = open_db();
        let mut dispatcher = DomainEventDispatcher::default();
        register_gm_event_handlers(&mut dispatcher, &db);
    }

    // -------------------------------------------------------------------------
    // HR-783: parse_payload regression tests
    // -------------------------------------------------------------------------

    /// A well-formed payload round-trips to Some(expected) via parse_payload.
    #[test]
    fn parse_payload_returns_some_for_valid() {
        let req = StatePersistRequest {
            key: "scene".to_string(),
            value: "exploration".to_string(),
        };
        let event = mk_event(0, "gm.state_persist_requested", &req);
        let result: Option<StatePersistRequest> = parse_payload(&event);
        assert!(result.is_some(), "valid payload must deserialize to Some");
        let parsed = result.unwrap();
        assert_eq!(parsed.key, "scene");
        assert_eq!(parsed.value, "exploration");
    }

    /// A malformed payload (wrong/missing fields) returns None without panicking.
    /// (The end-to-end handler no-op is covered by
    /// `take_damage_handler_is_safe_no_op_on_malformed_event`.)
    #[test]
    fn parse_payload_returns_none_and_does_not_panic_for_malformed() {
        // Build an event with a completely wrong payload shape.
        let mut bad_data = JsonObject::new();
        bad_data.insert("not_a_real_field".into(), serde_json::Value::Bool(true));
        let event = GameEvent::new(0, "combat.take_damage_requested", bad_data);

        // parse_payload must return None (not panic, not swallow silently).
        let result: Option<CombatTakeDamageRequested> = parse_payload(&event);
        assert!(
            result.is_none(),
            "malformed payload must return None, not panic"
        );
    }

    /// HR-783 bonus: handle_take_damage_requested returns vec![] (not a panic)
    /// when its payload is malformed — confirming the swap preserved no-op behaviour.
    #[test]
    fn take_damage_handler_is_safe_no_op_on_malformed_event() {
        let db = open_db();
        let mut bad_data = JsonObject::new();
        bad_data.insert("damage_expr".into(), serde_json::Value::String("1d6".into()));
        bad_data.insert("hazard_type".into(), serde_json::Value::String("pit".into()));
        let event = GameEvent::new(0, "combat.take_damage_requested", bad_data);
        let result = handle_take_damage_requested(&event, &db);
        assert!(
            result.is_empty(),
            "malformed event must produce empty vec[], not panic"
        );
    }

    #[test]
    fn hr790_purchase_adds_item_to_inventory() {
        let db = open_db();
        // Persist a buyer with an empty pack.
        let mut character = Character::new("Buyer", "warrior");
        character.id = "char_1".to_string();
        assert!(character.equipment.is_empty());
        let repo = EntityRepository::new(&db);
        repo.create_character(&character).unwrap();

        // The purchase request carries the item in `data.item` (the snapshot's
        // equipment is empty — taken before the buy).
        let item: JsonObject = serde_json::from_value(serde_json::json!({
            "name": "Iron Dagger", "type": "weapon", "value": 10
        }))
        .unwrap();
        let req = ShoppingPurchaseRequested {
            character_id: "char_1".to_string(),
            character_data: snapshot(),
            item,
            price: 10,
            gold_remaining: 90,
        };
        let event = mk_event(1, "shopping.purchase_requested", &req);
        let _ = handle_purchase_requested(&event, &db);

        // Regression: buying must persist the item into the character's inventory.
        // Before the fix the item was only read for its name, never added.
        let loaded = repo.load_first_character().unwrap().expect("character present");
        assert_eq!(loaded.equipment.len(), 1, "buying must add the item to inventory");
        assert_eq!(
            loaded.equipment[0].get("name").and_then(|v| v.as_str()),
            Some("Iron Dagger"),
            "the purchased item must round-trip into inventory"
        );
    }
}
