//! Combat and character event notice payloads. Ported from
//! `payloads/notices_combat.py`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::runtime::JsonObject;

fn d1() -> i32 {
    1
}
fn d1500() -> i32 {
    1500
}
fn d10() -> i32 {
    10
}
fn d15() -> i32 {
    15
}
fn dtrue() -> bool {
    true
}

/// Serialized character payload used in persistence/request events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
pub struct CharacterSnapshot {
    /// Id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Class.
    pub character_class: String,
    /// Level.
    #[serde(default = "d1")]
    pub level: i32,
    /// XP.
    #[serde(default)]
    pub xp: i32,
    /// XP to next.
    #[serde(default = "d1500")]
    pub xp_next: i32,
    /// Attribute scores.
    pub attributes: BTreeMap<String, i32>,
    /// Attribute modifiers.
    pub attr_mods: BTreeMap<String, i32>,
    /// Skills.
    pub skills: BTreeMap<String, i32>,
    /// Current HP.
    #[serde(default)]
    pub hp: i32,
    /// Max HP.
    #[serde(default)]
    pub max_hp: i32,
    /// Armour class.
    #[serde(default = "d10")]
    pub ac: i32,
    /// Attack bonus.
    #[serde(default)]
    pub attack_bonus: i32,
    /// Physical save.
    #[serde(default = "d15")]
    pub physical_save: i32,
    /// Evasion save.
    #[serde(default = "d15")]
    pub evasion_save: i32,
    /// Mental save.
    #[serde(default = "d15")]
    pub mental_save: i32,
    /// Equipment (arbitrary item objects).
    #[ts(type = "Record<string, unknown>[]")]
    pub equipment: Vec<JsonObject>,
    /// Class abilities (arbitrary key-value map).
    #[ts(type = "Record<string, unknown>")]
    pub class_abilities: JsonObject,
    /// Position column.
    #[serde(default)]
    pub position_q: i32,
    /// Position row.
    #[serde(default)]
    pub position_r: i32,
}

/// Payload for `character.death` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterDeathNotice {
    /// Position column.
    pub position_q: i32,
    /// Position row.
    pub position_r: i32,
}

/// Payload for `gm.narrate` presentation events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct NarrationNotice {
    /// Narration text.
    pub text: String,
}

/// Per-enemy state sent with `combat.start` for the frontend roster panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct EnemyCombatantState {
    /// Entity ID.
    pub entity_id: String,
    /// Display name.
    pub name: String,
    /// Current HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
}

/// Payload for `combat.start` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatStartNotice {
    /// Awareness outcome.
    pub awareness: String,
    /// Enemy display names (kept for backwards compatibility).
    pub enemies: Vec<String>,
    /// Full enemy states with HP for the combat roster panel.
    pub enemy_states: Vec<EnemyCombatantState>,
}

/// Payload for `combat.attack` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatAttackNotice {
    /// Attacker name.
    pub attacker: String,
    /// Target name.
    pub target: String,
    /// Attacker id.
    #[serde(default)]
    pub attacker_id: String,
    /// Target id.
    #[serde(default)]
    pub target_id: String,
    /// Weapon name.
    pub weapon: String,
    /// d20 roll.
    pub roll: i32,
    /// Modifier.
    pub modifier: i32,
    /// Total.
    pub total: i32,
    /// Target AC.
    pub target_ac: i32,
    /// Whether it hit.
    pub hit: bool,
    /// Damage dealt.
    #[serde(default)]
    pub damage: Option<i32>,
    /// Shock damage.
    #[serde(default)]
    pub shock: i32,
    /// Critical hit.
    #[serde(default)]
    pub critical: bool,
    /// Target's remaining HP after this attack (None when the target is the player).
    #[serde(default)]
    pub target_hp_remaining: Option<i32>,
    /// Target's max HP (None when the target is the player).
    #[serde(default)]
    pub target_max_hp: Option<i32>,
}

/// Payload for `combat.enemy_defeated` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatEnemyDefeatedNotice {
    /// Entity id.
    pub entity_id: String,
    /// Name.
    pub name: String,
}

/// Payload for `combat.fled` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatFledNotice {
    /// Clean getaway.
    pub clean: bool,
    /// Consequence.
    #[serde(default)]
    pub consequence: Option<String>,
    /// Destination column.
    pub destination_q: i32,
    /// Destination row.
    pub destination_r: i32,
}

/// Payload for `combat.player_hit` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatPlayerHitNotice {
    /// Attacker.
    pub attacker: String,
    /// Damage.
    pub damage: i32,
    /// Player HP.
    pub player_hp: i32,
    /// Player max HP.
    pub player_max_hp: i32,
    /// Player id.
    #[serde(default)]
    pub player_id: Option<String>,
    /// Player name.
    #[serde(default)]
    pub player_name: Option<String>,
    /// Player alive.
    #[serde(default = "dtrue")]
    pub player_alive: bool,
}

/// Payload for `character.hp_changed` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterHpChangedNotice {
    /// HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
}

/// Payload for `shopping.purchase` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ShoppingPurchaseNotice {
    /// Item name.
    pub item: String,
    /// Price.
    pub price: i32,
    /// Gold remaining.
    pub gold_remaining: i32,
}

/// Payload for `shopping.sale` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ShoppingSaleNotice {
    /// Item name.
    pub item: String,
    /// Price.
    pub price: i32,
    /// Gold total.
    pub gold_total: i32,
}

/// Payload for `character.created` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CharacterCreatedNotice {
    /// Character id.
    pub character_id: String,
}

/// Payload for `gm.scene_change` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SceneChangeNotice {
    /// Target scene.
    pub to: String,
}

/// Payload for `combat.save` events emitted when a dungeon trap (or other
/// hazard) requires a saving throw and the result is resolved inline.
///
/// The frontend `combat.save` branch reads `character`, `save_type`, `roll`,
/// `modifier`, `total`, `target`, and `passed` from `ev.data`.
///
/// HR-774: replaces the orphan `action.save_requested` that had no consumer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatSaveNotice {
    /// Character display name.
    pub character: String,
    /// Save category (`"physical"`, `"evasion"`, `"mental"`, or `"luck"`).
    pub save_type: String,
    /// d20 roll.
    pub roll: i32,
    /// Modifier applied (stat modifier + bonuses).
    pub modifier: i32,
    /// Roll + modifier.
    pub total: i32,
    /// Target number (character's base save + difficulty modifier from trap).
    pub target: i32,
    /// Whether the save passed (`total >= target`).
    pub passed: bool,
}

// ---------------------------------------------------------------------------
// Combat action bar / combat.actions
// ---------------------------------------------------------------------------

/// A single button in the combat action bar emitted by `combat.actions`.
///
/// `command` is the raw text sent back to the backend (e.g. `"attack"`).
/// `label` is the human-readable button text.
/// `style` is a frontend styling hint: `"primary"` | `"danger"` | `"default"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatActionButton {
    /// Raw command string sent to the backend.
    pub command: String,
    /// Human-readable button label.
    pub label: String,
    /// Frontend styling hint: "primary" | "danger" | "default".
    pub style: String,
}

/// Payload for `combat.actions` events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatActionsNotice {
    /// Combat phase: "active" | "last_stand" | "prompt" | "over".
    pub phase: String,
    /// Ordered list of action buttons to render.
    pub actions: Vec<CombatActionButton>,
}

// ---------------------------------------------------------------------------
// Exploration action bar / exploration.actions
// ---------------------------------------------------------------------------

/// Payload for `exploration.actions` events — the persistent tile-action bar.
///
/// Reuses [`CombatActionButton`] (same `command`/`label`/`style` shape) so the
/// frontend renders it with the same `ActionBar` component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ExplorationActionsNotice {
    /// Ordered list of tile-context action buttons to render.
    pub actions: Vec<CombatActionButton>,
}

// ---------------------------------------------------------------------------
// Battle-grid / combat.positions
// ---------------------------------------------------------------------------

/// One cell in the battle grid emitted by `combat.positions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PositionsCell {
    /// Column (0-based).
    pub q: i32,
    /// Row (0-based).
    pub r: i32,
    /// Terrain id (copied from the encounter world-cell).
    pub terrain: String,
    /// Feature ids stamped onto this cell (empty for most cells; at most one for
    /// corner cells that carry a world-cell feature).
    pub features: Vec<String>,
}

/// One entity (PC or monster) in the `combat.positions` snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PositionsEntity {
    /// Entity id matching `combat.start` / `combat.attack` so the frontend can
    /// correlate HP updates.
    pub entity_id: String,
    /// `"pc"` for the player, `"monster"` for enemies.
    pub kind: String,
    /// Display name.
    pub name: String,
    /// Battle-grid column.
    pub q: i32,
    /// Battle-grid row.
    pub r: i32,
    /// Current HP.
    pub hp: i32,
    /// Max HP.
    pub max_hp: i32,
    /// Whether the entity is alive.
    pub alive: bool,
}

/// A loot pile on the battle grid, dropped by a defeated enemy (HR-787).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct PositionsLoot {
    /// Battle-grid column of the drop.
    pub q: i32,
    /// Battle-grid row of the drop.
    pub r: i32,
    /// Representative value (highest single item value or currency) — the
    /// frontend maps this to a rarity tier for the indicator.
    pub value: i32,
    /// Number of distinct item stacks (+ currency) in the pile.
    pub item_count: i32,
}

/// Full payload for `combat.positions` events.
///
/// Carries the 9×9 battle grid (81 cells) and the positions of all living
/// combatants. The frontend renders this as a top-down encounter view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct CombatPositionsNotice {
    /// Grid width in cells (always 9).
    pub width: i32,
    /// Grid height in cells (always 9).
    pub height: i32,
    /// Grid topology identifier (always `"square"`).
    pub grid_type: String,
    /// All 81 cells in row-major order (r outer, q inner).
    pub cells: Vec<PositionsCell>,
    /// PC and all alive monsters.
    pub entities: Vec<PositionsEntity>,
    /// Loot piles dropped by defeated enemies (HR-787); empty until a kill.
    #[serde(default)]
    pub loot: Vec<PositionsLoot>,
}
