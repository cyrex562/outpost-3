//! GM, oracle, movement, social and town event notice payloads. Ported from
//! `payloads/notices_world.py`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::runtime::{JsonObject, JsonValue};

fn d_false_value() -> JsonValue {
    JsonValue::Bool(false)
}

/// Payload for `gm.teleport` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMTeleportNotice {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// From column.
    #[serde(default)]
    pub from_q: Option<i32>,
    /// From row.
    #[serde(default)]
    pub from_r: Option<i32>,
    /// To column.
    pub to_q: i32,
    /// To row.
    pub to_r: i32,
}

/// Payload for `gm.spawn` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSpawnNotice {
    /// Entity id.
    pub entity_id: String,
    /// Entity type.
    pub entity_type: String,
    /// Name.
    pub name: String,
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
}

/// Payload for `gm.give_item` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMGiveItemNotice {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Item name.
    pub item_name: String,
}

/// Payload for `gm.set_hp` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetHPNotice {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Old HP.
    pub old_hp: i32,
    /// New HP.
    pub new_hp: i32,
}

/// Payload for `gm.set_gold` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetGoldNotice {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Old gold.
    pub old_gold: i32,
    /// New gold.
    pub new_gold: i32,
}

/// Payload for `gm.set_xp` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetXPNotice {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Old XP.
    pub old_xp: i32,
    /// New XP.
    pub new_xp: i32,
}

/// Payload for `gm.set_attr` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GMSetAttrNotice {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
    /// Attribute.
    pub attribute: String,
    /// Old value.
    pub old_value: i32,
    /// New value.
    pub new_value: i32,
}

/// Payload for `oracle.scene_check` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OracleSceneCheckNotice {
    /// Roll.
    pub roll: i32,
    /// Chaos factor.
    pub chaos_factor: i32,
    /// Modification.
    pub modification: String,
}

/// Payload for `oracle.random_event` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OracleRandomEventNotice {
    /// Focus.
    pub focus: String,
    /// Action.
    pub action: String,
    /// Subject.
    pub subject: String,
}

/// Payload for `oracle.chaos_changed` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OracleChaosChangedNotice {
    /// Old value.
    pub old_value: i32,
    /// New value.
    pub new_value: i32,
    /// Chaos factor.
    pub chaos_factor: i32,
}

/// Compact cell payload used in movement and search events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct CellPreview {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
    /// Terrain id.
    pub terrain: String,
    /// Feature ids.
    pub features: Vec<String>,
    /// Whether explored (bool or int in legacy payloads — unknown in TS).
    #[serde(default = "d_false_value")]
    #[ts(type = "unknown")]
    pub explored: JsonValue,
}

/// Simple coordinate payload used inside movement events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionNotice {
    /// Column.
    pub q: i32,
    /// Row.
    pub r: i32,
}

/// Payload for `travel.step` — one cell advanced along a multi-step journey (HR-408).
/// Mirrors the fields `_websocketHandlers.ts` reads (`to_q`/`to_r`/`target_cell`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct TravelStepNotice {
    /// Character id.
    pub character_id: String,
    /// From column.
    pub from_q: i32,
    /// From row.
    pub from_r: i32,
    /// To column.
    pub to_q: i32,
    /// To row.
    pub to_r: i32,
    /// Direction stepped.
    pub direction: String,
    /// Whether the entered cell was previously unexplored.
    #[serde(default)]
    pub first_visit: bool,
    /// The cell stepped into.
    pub target_cell: CellPreview,
}

/// Payload for `exploration.revealed` — a single cell revealed without entering it
/// (#108: walking into impassable terrain reveals it so the player can see it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct ExplorationRevealedNotice {
    /// The revealed cell (carries `explored: 2` so the client renders it as "seen").
    pub cell: CellPreview,
}

/// Payload for `travel.started` / `travel.resumed` (HR-408).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct TravelJourneyNotice {
    /// Destination column.
    pub target_q: i32,
    /// Destination row.
    pub target_r: i32,
    /// Cells remaining to the destination at journey start.
    #[serde(default)]
    pub steps_remaining: i32,
}

/// Payload for `travel.interrupted` — a journey halted before arrival (HR-408).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct TravelInterruptedNotice {
    /// Destination column (frontend shows the resume banner for this target).
    pub target_q: i32,
    /// Destination row.
    pub target_r: i32,
    /// Why the journey stopped (e.g. `"encounter"`).
    #[serde(default)]
    pub reason: String,
}

/// Payload for `travel.completed` / `travel.cancelled` / `travel.blocked` (HR-408).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct TravelEndNotice {
    /// Destination column.
    pub target_q: i32,
    /// Destination row.
    pub target_r: i32,
}

/// Payload for `social.disposition_change` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SocialDispositionChangeNotice {
    /// Entity id.
    pub entity_id: String,
    /// NPC display name (HR-778: required by frontend `social.disposition_change` handler).
    pub npc: String,
    /// Old score.
    pub old_score: i32,
    /// New score.
    pub new_score: i32,
    /// Mood label for old_score, e.g. "indifferent".
    pub old_mood: String,
    /// Mood label for new_score, e.g. "helpful".
    pub new_mood: String,
    /// Reason.
    pub reason: String,
}

/// Payload for `action.skill_check` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ActionSkillCheckNotice {
    /// Verb.
    pub verb: String,
    /// Skill.
    pub skill: String,
    /// Attribute.
    pub attribute: String,
    /// Roll.
    pub roll: i32,
    /// Total.
    pub total: i32,
    /// Difficulty.
    pub difficulty: i32,
    /// Margin.
    pub margin: i32,
    /// Success.
    pub success: bool,
    /// Outcome band.
    pub outcome_key: String,
    /// Disposition delta.
    pub disposition_delta: i32,
    /// NPC entity id.
    #[serde(default)]
    pub npc_entity_id: Option<String>,
}

/// Payload for `character.expert_reroll` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterExpertRerollNotice {
    /// Verb.
    pub verb: String,
    /// Original total.
    pub original_total: i32,
    /// Reroll total.
    pub reroll_total: i32,
    /// Which result was used.
    pub used: String,
}

/// Payload for `town.move` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct TownMoveNotice {
    /// Direction.
    pub direction: String,
    /// Player column.
    pub player_q: i32,
    /// Player row.
    pub player_r: i32,
    /// Terrain.
    pub terrain: String,
}

/// Payload for `town.map` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct TownMapNotice {
    /// Cells (arbitrary cell objects).
    #[ts(type = "Record<string, unknown>[]")]
    pub cells: Vec<JsonObject>,
    /// Buildings (arbitrary building objects).
    #[ts(type = "Record<string, unknown>[]")]
    pub buildings: Vec<JsonObject>,
    /// Player column.
    pub player_q: i32,
    /// Player row.
    pub player_r: i32,
    /// Settlement name.
    pub settlement_name: String,
}

/// Payload for `social.healer` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SocialHealerNotice {
    /// HP restored.
    pub hp_restored: i32,
    /// Cost.
    pub cost: i32,
    /// NPC name.
    pub npc_name: String,
}

/// Payload for `character.respawn` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterRespawnNotice {
    /// Name.
    pub name: String,
    /// Settlement column.
    pub settlement_q: i32,
    /// Settlement row.
    pub settlement_r: i32,
    /// New HP.
    pub new_hp: i32,
    /// XP lost.
    pub xp_lost: i32,
    /// Item lost.
    #[serde(default)]
    pub item_lost: Option<String>,
}

/// Payload for `character.death_final` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterDeathFinalNotice {
    /// Name.
    pub name: String,
}

// ---------------------------------------------------------------------------
// New payload structs for events that previously used inline JsonObject.
// HR-794: switched from hand-rolled JsonObject construction to typed structs.
// ---------------------------------------------------------------------------

/// Payload for `character.level_up` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterLevelUpNotice {
    /// New character level.
    pub new_level: i32,
    /// New max HP after the level-up roll.
    pub new_max_hp: i32,
    /// HP gained from the level-up roll.
    pub hp_gained: i32,
}

/// Payload for `character.xp_gained` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterXpGainedNotice {
    /// XP awarded this combat.
    pub xp_gained: i32,
    /// Character's new XP total.
    pub new_total: i32,
    /// XP needed to reach the next level.
    pub xp_next: i32,
}

/// Payload for `dungeon.enter_room` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct DungeonEnterRoomNotice {
    /// Dungeon id.
    pub dungeon_id: String,
    /// Room id.
    pub room_id: String,
    /// Room display name.
    pub room_name: String,
    /// Room type (e.g. `"corridor"`, `"chamber"`).
    pub room_type: String,
    /// Direction entered from.
    pub direction: String,
    /// Whether this is the first time the room was entered.
    pub first_visit: bool,
}

/// Payload for `exploration.enter_hex` events (suppressed on the frontend;
/// used by IR trigger conditions to detect hex entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExplorationEnterHexNotice {
    /// Entity id of the entering character (used by trigger `self_id` conditions).
    pub self_id: String,
    /// Terrain id of the entered cell.
    pub terrain: String,
    /// Feature ids on the entered cell.
    pub features: Vec<String>,
    /// Column of the entered cell.
    pub q: i32,
    /// Row of the entered cell.
    pub r: i32,
    /// Whether this is the first time the cell was entered.
    pub first_visit: bool,
}

/// Payload for `gm.suggestions` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct GmSuggestionsNotice {
    /// Ordered list of command suggestions to show in the UI.
    pub commands: Vec<String>,
}

/// Payload for `inventory.ammo_consumed` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct InventoryAmmoConsumedNotice {
    /// Character id.
    pub character_id: String,
    /// Ammo type / item id.
    pub ammo_type: String,
    /// Quantity remaining after consumption.
    pub remaining: i32,
}

/// Payload for `inventory.item_given` events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct InventoryItemGivenNotice {
    /// Character id.
    pub character_id: String,
    /// The item payload (arbitrary item data object).
    #[ts(type = "Record<string, unknown>")]
    pub item: JsonObject,
}

/// Payload for `loot.source_revealed` events (HR-786): a searchable loot source
/// (container/corpse/ground) revealed by a successful search, or re-emitted with
/// its remaining contents after a `take`. Empty `items` + zero `gold` means the
/// source is exhausted and the loot panel should drop it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct LootSourceRevealedNotice {
    /// Cell column of the source.
    pub q: i32,
    /// Cell row of the source.
    pub r: i32,
    /// Stable source id within the cell.
    pub id: String,
    /// `"ground"` / `"container"` / `"corpse"`.
    pub kind: String,
    /// Display name, e.g. `"weathered crate"`.
    pub name: String,
    /// Remaining item payloads in the source.
    #[ts(type = "Record<string, unknown>[]")]
    pub items: Vec<JsonObject>,
    /// Remaining currency in the source.
    pub gold: i32,
}

/// Payload for `inventory.item_lost` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct InventoryItemLostNotice {
    /// Character id.
    pub character_id: String,
    /// Name of the lost item.
    pub item_name: String,
}

/// Payload for `oracle.fate_check` events (suppressed on the frontend;
/// carries the raw oracle result for logging/display purposes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct OracleFateCheckNotice {
    /// The yes/no question asked.
    pub question: String,
    /// Likelihood label (e.g. `"likely"`, `"50/50"`).
    pub likelihood: String,
    /// Current chaos factor at the time of the check.
    pub chaos_factor: i32,
    /// The d100 roll result.
    pub roll: i32,
}
