//! Gameplay command request payloads. Ported from `payloads/requests.py`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::payloads::notices_combat::CharacterSnapshot;
use crate::payloads::notices_world::CellPreview;
use crate::runtime::JsonObject;

/// Internal request payload for `gm_state` persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatePersistRequest {
    /// Key.
    pub key: String,
    /// Value.
    pub value: String,
}

/// Payload for the `exploration.moved` frontend notice.
///
/// HR-792: renamed from `ExplorationMoveRequested`. This is a client notice, not a
/// persistence-request — the `*Requested` name was the HR-791 footgun (a struct whose
/// name implies a `_requested` event, which `resolve_domain_events` filters out). The
/// emitted `event_type` is `"exploration.moved"`; position is persisted directly in
/// `ExplorationScene::handle_move`, not via a domain handler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ExplorationMoved {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// From column.
    pub from_q: i32,
    /// From row.
    pub from_r: i32,
    /// To column.
    pub to_q: i32,
    /// To row.
    pub to_r: i32,
    /// Direction.
    pub direction: String,
    /// First visit.
    #[serde(default)]
    pub first_visit: bool,
    /// Target cell.
    pub target_cell: CellPreview,
    /// Adjacent cells.
    pub adjacent_cells: Vec<CellPreview>,
    /// Optional weather data for the entered hex.
    #[serde(default)]
    #[ts(type = "Record<string, unknown> | null")]
    pub weather: Option<JsonObject>,
}

// HR-793: `ExplorationRestRequested`, `ExplorationTakeRequested`, and
// `ExplorationSearchRequested` were removed. They ended in `_requested`, were
// filtered by `resolve_domain_events`, and had no subscriber — their structured
// results never reached the client. Rest now relies on `character.hp_changed`;
// search/take emit `inventory.item_given` per item (see `ExplorationScene`).

/// Payload for `combat.victory_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatVictoryRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// XP gained.
    pub xp_gained: i32,
    /// Items gained (auto-collected to inventory).
    #[serde(default)]
    pub items_gained: Vec<JsonObject>,
    /// Currency gained (auto-collected to inventory).
    #[serde(default)]
    pub currency_gained: i32,
    /// HR-786: item drops left on a searchable corpse at the player's cell
    /// instead of auto-collected (gold still auto-collects via `currency_gained`).
    #[serde(default)]
    pub corpse_items: Vec<JsonObject>,
    /// Harvestable payload.
    #[serde(default)]
    pub harvestable: Option<JsonObject>,
}

/// Payload for `combat.flee_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatFleeRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// Damage taken.
    #[serde(default)]
    pub damage_taken: i32,
    /// Item lost.
    #[serde(default)]
    pub item_lost: Option<String>,
    /// Destination column.
    pub destination_q: i32,
    /// Destination row.
    pub destination_r: i32,
}

/// Payload for `combat.consume_ammo_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatConsumeAmmoRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// Ammo type.
    pub ammo_type: String,
}

/// Payload for `combat.harvest_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatHarvestRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// Material.
    pub material: String,
    /// Success.
    pub success: bool,
    /// Narration.
    pub narration: String,
}

/// Payload for `combat.use_item_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatUseItemRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// Item name.
    pub item_name: String,
    /// HP restored.
    #[serde(default)]
    pub hp_restored: i32,
    /// Narration.
    #[serde(default)]
    pub narration: String,
}

/// Payload for `combat.take_damage_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatTakeDamageRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// Damage.
    pub damage: i32,
    /// Source.
    pub source: String,
}

/// Payload for `combat.update_character_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CombatUpdateCharacterRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
}

/// Payload for `character.respawn_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterRespawnRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// New HP.
    pub new_hp: i32,
    /// XP loss.
    pub xp_loss: i32,
    /// Lost item.
    #[serde(default)]
    pub lost_item: Option<String>,
    /// Settlement column.
    pub settlement_q: i32,
    /// Settlement row.
    pub settlement_r: i32,
    /// Death column.
    pub death_q: i32,
    /// Death row.
    pub death_r: i32,
}

/// Payload for `shopping.purchase_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingPurchaseRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// Item payload.
    pub item: JsonObject,
    /// Price.
    pub price: i32,
    /// Gold remaining.
    pub gold_remaining: i32,
}

/// Payload for `shopping.sale_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingSaleRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// Item name.
    pub item_name: String,
    /// Price.
    pub price: i32,
    /// Gold total.
    pub gold_total: i32,
}

// HR-792: the `gm.*` editor request payloads (GMTeleportRequested, GMSpawnRequested,
// GMGiveItemRequested, GMSetHPRequested, GMSetGoldRequested, GMSetXPRequested,
// GMSetAttrRequested) were removed — they were never emitted or deserialized anywhere.
// The admin/editor GM tools write directly to the DB via `editor/gm.rs`, not through
// the event bus. Restore from git history if a future event-driven GM path needs them.

/// Payload for `social.disposition_update_requested`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialDispositionUpdateRequested {
    /// Entity id.
    pub entity_id: String,
    /// New disposition.
    pub new_disposition: i32,
    /// NPC display name, if known at emit time (HR-778: lets the handler avoid a DB
    /// lookup when the caller already has the name in scope).
    #[serde(default)]
    pub npc_name: Option<String>,
}

/// Payload for `social.healer_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialHealerRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
    /// HP restored.
    pub hp_restored: i32,
    /// Gold spent.
    pub gold_spent: i32,
    /// NPC name.
    pub npc_name: String,
}

/// Payload for `character.update_requested`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterUpdateRequested {
    /// Character id.
    pub character_id: String,
    /// Character snapshot.
    pub character_data: CharacterSnapshot,
}
