//! `ClientEvent` — the canonical discriminated union of all events the backend
//! forwards to the frontend over the WebSocket.
//!
//! HR-794: makes Rust payload structs the single source of truth for the
//! frontend's TypeScript types. `generate_ts_bindings()` assembles the
//! generated `events.gen.ts` from `ts_rs::TS::decl()` on every payload type.
//! The `cargo xtask gen-types` command calls the `export-types` binary which
//! writes the file. The `committed_events_gen_matches_export` test is the
//! drift gate: it fails CI whenever the committed file diverges from the
//! current Rust types, instructing the developer to re-run `gen-types`.

use serde::{Deserialize, Serialize};
use ts_rs::{Config, TS};

use crate::payloads::notices_combat::{
    CharacterCreatedNotice, CharacterDeathNotice, CharacterHpChangedNotice, CharacterSnapshot,
    CombatActionButton, CombatActionsNotice, CombatAttackNotice, CombatEnemyDefeatedNotice,
    CombatFledNotice, CombatPlayerHitNotice, CombatPositionsNotice, CombatSaveNotice,
    CombatStartNotice, EnemyCombatantState, ExplorationActionsNotice, NarrationNotice,
    PositionsCell, PositionsEntity, PositionsLoot, SceneChangeNotice, ShoppingPurchaseNotice,
    ShoppingSaleNotice,
};
use crate::payloads::notices_world::{
    ActionSkillCheckNotice, CellPreview, CharacterDeathFinalNotice, CharacterExpertRerollNotice,
    CharacterLevelUpNotice, CharacterRespawnNotice, CharacterXpGainedNotice,
    DungeonEnterRoomNotice, ExplorationEnterHexNotice, ExplorationRevealedNotice,
    GmSuggestionsNotice, InventoryAmmoConsumedNotice, InventoryItemGivenNotice,
    InventoryItemLostNotice, LootSourceRevealedNotice, OracleChaosChangedNotice, OracleFateCheckNotice,
    OracleRandomEventNotice, OracleSceneCheckNotice, SocialDispositionChangeNotice,
    SocialHealerNotice, TownMapNotice, TownMoveNotice, TravelEndNotice, TravelInterruptedNotice,
    TravelJourneyNotice, TravelStepNotice,
};
use crate::payloads::quests::{
    QuestAcceptedNotice, QuestCompletedNotice, QuestFailedNotice, QuestProgressUpdatedNotice,
};
use crate::payloads::requests::ExplorationMoved;
use crate::payloads::transport::{
    ActiveStatusEffectPayload, StatusAppliedNotice, StatusExpiredNotice, StatusRemovedNotice,
};

/// Discriminated union of the client-facing events the backend forwards over
/// the WebSocket as `game_event` frames. The serde representation mirrors that
/// wire format:
///   `{ "event_type": "<type>", "data": { ...payload... } }`
///
/// This covers all of `CLIENT_FACING_EVENT_TYPES` **except `gm.narrate`**:
/// `event_to_frames` in `harsh-web`'s `wsmsg.rs` special-cases `gm.narrate`
/// into a `{ "type": "narration", "text": … }` frame, so it is never delivered
/// as a `game_event` and can never appear as a `ClientEvent` payload. It stays
/// in `CLIENT_FACING_EVENT_TYPES` (the frontend still handles the narration
/// frame) — it is simply not a `game_event` payload type.
///
/// This enum is the Rust-side source of truth for `events.gen.ts`.
/// Use `generate_ts_bindings()` (or `cargo xtask gen-types`) to regenerate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "event_type", content = "data")]
pub enum ClientEvent {
    #[serde(rename = "action.skill_check")]
    ActionSkillCheck(ActionSkillCheckNotice),

    #[serde(rename = "character.created")]
    CharacterCreated(CharacterCreatedNotice),

    #[serde(rename = "character.death")]
    CharacterDeath(CharacterDeathNotice),

    #[serde(rename = "character.death_final")]
    CharacterDeathFinal(CharacterDeathFinalNotice),

    #[serde(rename = "character.expert_reroll")]
    CharacterExpertReroll(CharacterExpertRerollNotice),

    #[serde(rename = "character.hp_changed")]
    CharacterHpChanged(CharacterHpChangedNotice),

    #[serde(rename = "character.level_up")]
    CharacterLevelUp(CharacterLevelUpNotice),

    #[serde(rename = "character.respawn")]
    CharacterRespawn(CharacterRespawnNotice),

    #[serde(rename = "character.xp_gained")]
    CharacterXpGained(CharacterXpGainedNotice),

    #[serde(rename = "combat.actions")]
    CombatActions(CombatActionsNotice),

    #[serde(rename = "combat.attack")]
    CombatAttack(CombatAttackNotice),

    #[serde(rename = "combat.enemy_defeated")]
    CombatEnemyDefeated(CombatEnemyDefeatedNotice),

    #[serde(rename = "combat.fled")]
    CombatFled(CombatFledNotice),

    #[serde(rename = "combat.player_hit")]
    CombatPlayerHit(CombatPlayerHitNotice),

    #[serde(rename = "combat.positions")]
    CombatPositions(CombatPositionsNotice),

    #[serde(rename = "combat.save")]
    CombatSave(CombatSaveNotice),

    #[serde(rename = "combat.start")]
    CombatStart(CombatStartNotice),

    #[serde(rename = "dungeon.enter_room")]
    DungeonEnterRoom(DungeonEnterRoomNotice),

    #[serde(rename = "exploration.actions")]
    ExplorationActions(ExplorationActionsNotice),

    #[serde(rename = "exploration.enter_hex")]
    ExplorationEnterHex(ExplorationEnterHexNotice),

    #[serde(rename = "exploration.moved")]
    ExplorationMoved(ExplorationMoved),

    #[serde(rename = "exploration.revealed")]
    ExplorationRevealed(ExplorationRevealedNotice),

    #[serde(rename = "gm.scene_change")]
    GmSceneChange(SceneChangeNotice),

    #[serde(rename = "gm.suggestions")]
    GmSuggestions(GmSuggestionsNotice),

    #[serde(rename = "inventory.ammo_consumed")]
    InventoryAmmoConsumed(InventoryAmmoConsumedNotice),

    #[serde(rename = "inventory.item_given")]
    InventoryItemGiven(InventoryItemGivenNotice),
    #[serde(rename = "loot.source_revealed")]
    LootSourceRevealed(LootSourceRevealedNotice),

    #[serde(rename = "inventory.item_lost")]
    InventoryItemLost(InventoryItemLostNotice),

    #[serde(rename = "oracle.chaos_changed")]
    OracleChaosChanged(OracleChaosChangedNotice),

    #[serde(rename = "oracle.fate_check")]
    OracleFateCheck(OracleFateCheckNotice),

    #[serde(rename = "oracle.random_event")]
    OracleRandomEvent(OracleRandomEventNotice),

    #[serde(rename = "oracle.scene_check")]
    OracleSceneCheck(OracleSceneCheckNotice),

    #[serde(rename = "quest.accepted")]
    QuestAccepted(QuestAcceptedNotice),
    #[serde(rename = "quest.completed")]
    QuestCompleted(QuestCompletedNotice),
    #[serde(rename = "quest.failed")]
    QuestFailed(QuestFailedNotice),
    #[serde(rename = "quest.progress_updated")]
    QuestProgressUpdated(QuestProgressUpdatedNotice),

    #[serde(rename = "shopping.purchase")]
    ShoppingPurchase(ShoppingPurchaseNotice),

    #[serde(rename = "shopping.sale")]
    ShoppingSale(ShoppingSaleNotice),

    #[serde(rename = "social.disposition_change")]
    SocialDispositionChange(SocialDispositionChangeNotice),

    #[serde(rename = "social.healer")]
    SocialHealer(SocialHealerNotice),

    #[serde(rename = "status.applied")]
    StatusApplied(StatusAppliedNotice),

    #[serde(rename = "status.expired")]
    StatusExpired(StatusExpiredNotice),

    #[serde(rename = "status.removed")]
    StatusRemoved(StatusRemovedNotice),

    #[serde(rename = "town.map")]
    TownMap(TownMapNotice),

    #[serde(rename = "town.move")]
    TownMove(TownMoveNotice),

    #[serde(rename = "travel.blocked")]
    TravelBlocked(TravelEndNotice),

    #[serde(rename = "travel.cancelled")]
    TravelCancelled(TravelEndNotice),

    #[serde(rename = "travel.completed")]
    TravelCompleted(TravelEndNotice),

    #[serde(rename = "travel.interrupted")]
    TravelInterrupted(TravelInterruptedNotice),

    #[serde(rename = "travel.resumed")]
    TravelResumed(TravelJourneyNotice),

    #[serde(rename = "travel.started")]
    TravelStarted(TravelJourneyNotice),

    #[serde(rename = "travel.step")]
    TravelStep(TravelStepNotice),
}

/// Assemble the single-file TypeScript bindings for all client-facing event
/// payload types. Every type is emitted in dependency order so the resulting
/// file is self-contained (no imports needed).
///
/// This function is the source of truth for `frontend/src/types/events.gen.ts`.
/// The `committed_events_gen_matches_export` test calls it and diffs the result
/// against the committed file; `cargo xtask gen-types` → `export-types` binary
/// also calls it.
/// Helper: call `T::decl(&cfg)` and prefix with `export ` so the resulting
/// file exports every type (enabling `import type { … } from "./events.gen"`
/// in the spec fixture and in future consumer code).
///
/// ts-rs `decl()` returns strings starting with `type` or `enum`; we
/// prepend `export ` unconditionally — both forms are valid exports.
fn exported<T: TS>(cfg: &Config) -> String {
    let d = T::decl(cfg);
    // d starts with "type Foo = …" or "enum Foo { … }"
    format!("export {d}")
}

pub fn generate_ts_bindings() -> String {
    let cfg = Config::new();

    let mut parts: Vec<String> = vec![
        "// @generated — do not edit by hand.".into(),
        "// Re-generate with: cargo xtask gen-types".into(),
        "// Source of truth: crates/harsh-core/src/payloads/".into(),
        String::new(),
    ];

    // -----------------------------------------------------------------------
    // Group A — leaf types: no references to other custom types in this file
    // (all JsonObject/JsonValue fields are replaced by #[ts(type = "...")])
    // -----------------------------------------------------------------------

    // From notices_combat.rs
    parts.push(exported::<CharacterDeathNotice>(&cfg));
    parts.push(exported::<NarrationNotice>(&cfg));
    parts.push(exported::<CharacterHpChangedNotice>(&cfg));
    parts.push(exported::<ShoppingPurchaseNotice>(&cfg));
    parts.push(exported::<ShoppingSaleNotice>(&cfg));
    parts.push(exported::<CharacterCreatedNotice>(&cfg));
    parts.push(exported::<SceneChangeNotice>(&cfg));
    parts.push(exported::<CombatAttackNotice>(&cfg));
    parts.push(exported::<CombatEnemyDefeatedNotice>(&cfg));
    parts.push(exported::<CombatFledNotice>(&cfg));
    parts.push(exported::<CombatPlayerHitNotice>(&cfg));
    parts.push(exported::<CombatSaveNotice>(&cfg));
    parts.push(exported::<CombatActionButton>(&cfg));
    parts.push(exported::<EnemyCombatantState>(&cfg));
    parts.push(exported::<PositionsCell>(&cfg));
    parts.push(exported::<PositionsEntity>(&cfg));
    parts.push(exported::<PositionsLoot>(&cfg));

    // From notices_world.rs
    parts.push(exported::<OracleSceneCheckNotice>(&cfg));
    parts.push(exported::<QuestAcceptedNotice>(&cfg));
    parts.push(exported::<QuestCompletedNotice>(&cfg));
    parts.push(exported::<QuestFailedNotice>(&cfg));
    parts.push(exported::<QuestProgressUpdatedNotice>(&cfg));
    parts.push(exported::<OracleRandomEventNotice>(&cfg));
    parts.push(exported::<OracleChaosChangedNotice>(&cfg));
    parts.push(exported::<SocialDispositionChangeNotice>(&cfg));
    parts.push(exported::<CharacterExpertRerollNotice>(&cfg));
    parts.push(exported::<TownMoveNotice>(&cfg));
    parts.push(exported::<TownMapNotice>(&cfg));
    parts.push(exported::<SocialHealerNotice>(&cfg));
    parts.push(exported::<CharacterRespawnNotice>(&cfg));
    parts.push(exported::<CharacterDeathFinalNotice>(&cfg));
    parts.push(exported::<ActionSkillCheckNotice>(&cfg));
    parts.push(exported::<TravelJourneyNotice>(&cfg));
    parts.push(exported::<TravelInterruptedNotice>(&cfg));
    parts.push(exported::<TravelEndNotice>(&cfg));

    // New structs (HR-794) — previously emitted as inline JsonObject
    parts.push(exported::<CharacterLevelUpNotice>(&cfg));
    parts.push(exported::<CharacterXpGainedNotice>(&cfg));
    parts.push(exported::<DungeonEnterRoomNotice>(&cfg));
    parts.push(exported::<ExplorationEnterHexNotice>(&cfg));
    parts.push(exported::<GmSuggestionsNotice>(&cfg));
    parts.push(exported::<InventoryAmmoConsumedNotice>(&cfg));
    parts.push(exported::<InventoryItemGivenNotice>(&cfg));
    parts.push(exported::<LootSourceRevealedNotice>(&cfg));
    parts.push(exported::<InventoryItemLostNotice>(&cfg));
    parts.push(exported::<OracleFateCheckNotice>(&cfg));

    // From transport.rs (status event leaves)
    parts.push(exported::<StatusRemovedNotice>(&cfg));

    // Types whose JsonObject/JsonValue fields are overridden → treated as leaves
    parts.push(exported::<ActiveStatusEffectPayload>(&cfg));
    parts.push(exported::<CellPreview>(&cfg));
    parts.push(exported::<CharacterSnapshot>(&cfg));

    // -----------------------------------------------------------------------
    // Group B — types that reference Group A custom types
    // -----------------------------------------------------------------------

    parts.push(exported::<CombatStartNotice>(&cfg));        // → EnemyCombatantState
    parts.push(exported::<CombatActionsNotice>(&cfg));      // → CombatActionButton
    parts.push(exported::<ExplorationActionsNotice>(&cfg)); // → CombatActionButton
    parts.push(exported::<CombatPositionsNotice>(&cfg));    // → PositionsCell, PositionsEntity
    parts.push(exported::<StatusAppliedNotice>(&cfg));      // → ActiveStatusEffectPayload
    parts.push(exported::<StatusExpiredNotice>(&cfg));      // → ActiveStatusEffectPayload
    parts.push(exported::<TravelStepNotice>(&cfg));         // → CellPreview
    parts.push(exported::<ExplorationRevealedNotice>(&cfg)); // → CellPreview
    parts.push(exported::<ExplorationMoved>(&cfg));         // → CharacterSnapshot, CellPreview

    // -----------------------------------------------------------------------
    // Group C — top-level discriminated union (depends on everything above)
    // -----------------------------------------------------------------------
    parts.push(exported::<ClientEvent>(&cfg));

    // Join with a blank line between declarations, trailing newline.
    let mut out = parts.join("\n\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Round-trip fixtures (serde tag+content shape verification)
    // -----------------------------------------------------------------------

    #[test]
    fn client_event_round_trip_simple() {
        let ev = ClientEvent::CharacterHpChanged(CharacterHpChangedNotice { hp: 50, max_hp: 100 });
        let json = serde_json::to_string(&ev).expect("serializes");
        let back: ClientEvent = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(ev, back);

        // Verify the wire format has the correct tag+content shape.
        let val: serde_json::Value = serde_json::from_str(&json).expect("parses");
        assert_eq!(val["event_type"], "character.hp_changed");
        assert_eq!(val["data"]["hp"], 50);
        assert_eq!(val["data"]["max_hp"], 100);
    }

    #[test]
    fn client_event_round_trip_combat_attack() {
        let ev = ClientEvent::CombatAttack(CombatAttackNotice {
            attacker: "PC".into(),
            target: "Goblin".into(),
            attacker_id: "pc-1".into(),
            target_id: "mob-1".into(),
            weapon: "short sword".into(),
            roll: 15,
            modifier: 2,
            total: 17,
            target_ac: 13,
            hit: true,
            damage: Some(6),
            shock: 0,
            critical: false,
            target_hp_remaining: Some(4),
            target_max_hp: Some(10),
        });
        let json = serde_json::to_string(&ev).expect("serializes");
        let back: ClientEvent = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(ev, back);

        let val: serde_json::Value = serde_json::from_str(&json).expect("parses");
        assert_eq!(val["event_type"], "combat.attack");
        assert_eq!(val["data"]["attacker"], "PC");
        assert!(val["data"]["hit"].as_bool().unwrap_or(false));
    }

    #[test]
    fn client_event_round_trip_status_applied() {
        let effect = ActiveStatusEffectPayload {
            id: 1,
            entity_id: "e1".into(),
            effect_id: "dread".into(),
            applied_at_tick: 10,
            expires_at_tick: Some(20),
            source: Some("ruins".into()),
            data: serde_json::Map::new(),
        };
        let ev = ClientEvent::StatusApplied(StatusAppliedNotice {
            active_effect: effect,
        });
        let json = serde_json::to_string(&ev).expect("serializes");
        let back: ClientEvent = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(ev, back);

        let val: serde_json::Value = serde_json::from_str(&json).expect("parses");
        assert_eq!(val["event_type"], "status.applied");
        assert_eq!(val["data"]["active_effect"]["effect_id"], "dread");
    }

    #[test]
    fn client_event_round_trip_travel_shared_struct() {
        // TravelEndNotice is shared by travel.blocked / travel.cancelled / travel.completed.
        let end = TravelEndNotice { target_q: 5, target_r: 3 };
        let blocked = ClientEvent::TravelBlocked(end.clone());
        let cancelled = ClientEvent::TravelCancelled(end.clone());
        let completed = ClientEvent::TravelCompleted(end.clone());

        for ev in [blocked, cancelled, completed] {
            let json = serde_json::to_string(&ev).expect("serializes");
            let back: ClientEvent = serde_json::from_str(&json).expect("deserializes");
            assert_eq!(ev, back);
        }
        // Verify tag values
        let j_b = serde_json::to_string(&ClientEvent::TravelBlocked(end.clone())).unwrap();
        assert!(j_b.contains("\"travel.blocked\""));
        let j_c = serde_json::to_string(&ClientEvent::TravelCancelled(end.clone())).unwrap();
        assert!(j_c.contains("\"travel.cancelled\""));
    }

    // -----------------------------------------------------------------------
    // Drift gate
    // -----------------------------------------------------------------------

    /// The committed `events.gen.ts` must equal the current export.
    ///
    /// If this test fails, regenerate with `cargo xtask gen-types` and commit.
    #[test]
    fn committed_events_gen_matches_export() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../frontend/src/types/events.gen.ts"
        );
        let committed = std::fs::read_to_string(path).expect(
            "frontend/src/types/events.gen.ts must exist; \
             run `cargo xtask gen-types` to generate it",
        );
        assert_eq!(
            committed,
            generate_ts_bindings(),
            "events.gen.ts is out of date — run `cargo xtask gen-types` and commit the result"
        );
    }
}
