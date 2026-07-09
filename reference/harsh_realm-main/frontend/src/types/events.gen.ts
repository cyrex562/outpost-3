// @generated — do not edit by hand.

// Re-generate with: cargo xtask gen-types

// Source of truth: crates/harsh-core/src/payloads/



export type CharacterDeathNotice = { 
/**
 * Position column.
 */
position_q: number, 
/**
 * Position row.
 */
position_r: number, };

export type NarrationNotice = { 
/**
 * Narration text.
 */
text: string, };

export type CharacterHpChangedNotice = { 
/**
 * HP.
 */
hp: number, 
/**
 * Max HP.
 */
max_hp: number, };

export type ShoppingPurchaseNotice = { 
/**
 * Item name.
 */
item: string, 
/**
 * Price.
 */
price: number, 
/**
 * Gold remaining.
 */
gold_remaining: number, };

export type ShoppingSaleNotice = { 
/**
 * Item name.
 */
item: string, 
/**
 * Price.
 */
price: number, 
/**
 * Gold total.
 */
gold_total: number, };

export type CharacterCreatedNotice = { 
/**
 * Character id.
 */
character_id: string, };

export type SceneChangeNotice = { 
/**
 * Target scene.
 */
to: string, };

export type CombatAttackNotice = { 
/**
 * Attacker name.
 */
attacker: string, 
/**
 * Target name.
 */
target: string, 
/**
 * Attacker id.
 */
attacker_id: string, 
/**
 * Target id.
 */
target_id: string, 
/**
 * Weapon name.
 */
weapon: string, 
/**
 * d20 roll.
 */
roll: number, 
/**
 * Modifier.
 */
modifier: number, 
/**
 * Total.
 */
total: number, 
/**
 * Target AC.
 */
target_ac: number, 
/**
 * Whether it hit.
 */
hit: boolean, 
/**
 * Damage dealt.
 */
damage: number | null, 
/**
 * Shock damage.
 */
shock: number, 
/**
 * Critical hit.
 */
critical: boolean, 
/**
 * Target's remaining HP after this attack (None when the target is the player).
 */
target_hp_remaining: number | null, 
/**
 * Target's max HP (None when the target is the player).
 */
target_max_hp: number | null, };

export type CombatEnemyDefeatedNotice = { 
/**
 * Entity id.
 */
entity_id: string, 
/**
 * Name.
 */
name: string, };

export type CombatFledNotice = { 
/**
 * Clean getaway.
 */
clean: boolean, 
/**
 * Consequence.
 */
consequence: string | null, 
/**
 * Destination column.
 */
destination_q: number, 
/**
 * Destination row.
 */
destination_r: number, };

export type CombatPlayerHitNotice = { 
/**
 * Attacker.
 */
attacker: string, 
/**
 * Damage.
 */
damage: number, 
/**
 * Player HP.
 */
player_hp: number, 
/**
 * Player max HP.
 */
player_max_hp: number, 
/**
 * Player id.
 */
player_id: string | null, 
/**
 * Player name.
 */
player_name: string | null, 
/**
 * Player alive.
 */
player_alive: boolean, };

export type CombatSaveNotice = { 
/**
 * Character display name.
 */
character: string, 
/**
 * Save category (`"physical"`, `"evasion"`, `"mental"`, or `"luck"`).
 */
save_type: string, 
/**
 * d20 roll.
 */
roll: number, 
/**
 * Modifier applied (stat modifier + bonuses).
 */
modifier: number, 
/**
 * Roll + modifier.
 */
total: number, 
/**
 * Target number (character's base save + difficulty modifier from trap).
 */
target: number, 
/**
 * Whether the save passed (`total >= target`).
 */
passed: boolean, };

export type CombatActionButton = { 
/**
 * Raw command string sent to the backend.
 */
command: string, 
/**
 * Human-readable button label.
 */
label: string, 
/**
 * Frontend styling hint: "primary" | "danger" | "default".
 */
style: string, };

export type EnemyCombatantState = { 
/**
 * Entity ID.
 */
entity_id: string, 
/**
 * Display name.
 */
name: string, 
/**
 * Current HP.
 */
hp: number, 
/**
 * Max HP.
 */
max_hp: number, };

export type PositionsCell = { 
/**
 * Column (0-based).
 */
q: number, 
/**
 * Row (0-based).
 */
r: number, 
/**
 * Terrain id (copied from the encounter world-cell).
 */
terrain: string, 
/**
 * Feature ids stamped onto this cell (empty for most cells; at most one for
 * corner cells that carry a world-cell feature).
 */
features: Array<string>, };

export type PositionsEntity = { 
/**
 * Entity id matching `combat.start` / `combat.attack` so the frontend can
 * correlate HP updates.
 */
entity_id: string, 
/**
 * `"pc"` for the player, `"monster"` for enemies.
 */
kind: string, 
/**
 * Display name.
 */
name: string, 
/**
 * Battle-grid column.
 */
q: number, 
/**
 * Battle-grid row.
 */
r: number, 
/**
 * Current HP.
 */
hp: number, 
/**
 * Max HP.
 */
max_hp: number, 
/**
 * Whether the entity is alive.
 */
alive: boolean, };

export type PositionsLoot = { 
/**
 * Battle-grid column of the drop.
 */
q: number, 
/**
 * Battle-grid row of the drop.
 */
r: number, 
/**
 * Representative value (highest single item value or currency) — the
 * frontend maps this to a rarity tier for the indicator.
 */
value: number, 
/**
 * Number of distinct item stacks (+ currency) in the pile.
 */
item_count: number, };

export type OracleSceneCheckNotice = { 
/**
 * Roll.
 */
roll: number, 
/**
 * Chaos factor.
 */
chaos_factor: number, 
/**
 * Modification.
 */
modification: string, };

export type QuestAcceptedNotice = { quest_id: string, quest_name: string, description: string, };

export type QuestCompletedNotice = { quest_id: string, quest_name: string, reward_gold: number, reward_xp: number, reward_items: Array<string>, };

export type QuestFailedNotice = { quest_id: string, quest_name: string, };

export type QuestProgressUpdatedNotice = { quest_id: string, objective_key: string, value: number, progress: Record<string, unknown>, };

export type OracleRandomEventNotice = { 
/**
 * Focus.
 */
focus: string, 
/**
 * Action.
 */
action: string, 
/**
 * Subject.
 */
subject: string, };

export type OracleChaosChangedNotice = { 
/**
 * Old value.
 */
old_value: number, 
/**
 * New value.
 */
new_value: number, 
/**
 * Chaos factor.
 */
chaos_factor: number, };

export type SocialDispositionChangeNotice = { 
/**
 * Entity id.
 */
entity_id: string, 
/**
 * NPC display name (HR-778: required by frontend `social.disposition_change` handler).
 */
npc: string, 
/**
 * Old score.
 */
old_score: number, 
/**
 * New score.
 */
new_score: number, 
/**
 * Mood label for old_score, e.g. "indifferent".
 */
old_mood: string, 
/**
 * Mood label for new_score, e.g. "helpful".
 */
new_mood: string, 
/**
 * Reason.
 */
reason: string, };

export type CharacterExpertRerollNotice = { 
/**
 * Verb.
 */
verb: string, 
/**
 * Original total.
 */
original_total: number, 
/**
 * Reroll total.
 */
reroll_total: number, 
/**
 * Which result was used.
 */
used: string, };

export type TownMoveNotice = { 
/**
 * Direction.
 */
direction: string, 
/**
 * Player column.
 */
player_q: number, 
/**
 * Player row.
 */
player_r: number, 
/**
 * Terrain.
 */
terrain: string, };

export type TownMapNotice = { 
/**
 * Cells (arbitrary cell objects).
 */
cells: Record<string, unknown>[], 
/**
 * Buildings (arbitrary building objects).
 */
buildings: Record<string, unknown>[], 
/**
 * Player column.
 */
player_q: number, 
/**
 * Player row.
 */
player_r: number, 
/**
 * Settlement name.
 */
settlement_name: string, };

export type SocialHealerNotice = { 
/**
 * HP restored.
 */
hp_restored: number, 
/**
 * Cost.
 */
cost: number, 
/**
 * NPC name.
 */
npc_name: string, };

export type CharacterRespawnNotice = { 
/**
 * Name.
 */
name: string, 
/**
 * Settlement column.
 */
settlement_q: number, 
/**
 * Settlement row.
 */
settlement_r: number, 
/**
 * New HP.
 */
new_hp: number, 
/**
 * XP lost.
 */
xp_lost: number, 
/**
 * Item lost.
 */
item_lost: string | null, };

export type CharacterDeathFinalNotice = { 
/**
 * Name.
 */
name: string, };

export type ActionSkillCheckNotice = { 
/**
 * Verb.
 */
verb: string, 
/**
 * Skill.
 */
skill: string, 
/**
 * Attribute.
 */
attribute: string, 
/**
 * Roll.
 */
roll: number, 
/**
 * Total.
 */
total: number, 
/**
 * Difficulty.
 */
difficulty: number, 
/**
 * Margin.
 */
margin: number, 
/**
 * Success.
 */
success: boolean, 
/**
 * Outcome band.
 */
outcome_key: string, 
/**
 * Disposition delta.
 */
disposition_delta: number, 
/**
 * NPC entity id.
 */
npc_entity_id: string | null, };

export type TravelJourneyNotice = { 
/**
 * Destination column.
 */
target_q: number, 
/**
 * Destination row.
 */
target_r: number, 
/**
 * Cells remaining to the destination at journey start.
 */
steps_remaining: number, };

export type TravelInterruptedNotice = { 
/**
 * Destination column (frontend shows the resume banner for this target).
 */
target_q: number, 
/**
 * Destination row.
 */
target_r: number, 
/**
 * Why the journey stopped (e.g. `"encounter"`).
 */
reason: string, };

export type TravelEndNotice = { 
/**
 * Destination column.
 */
target_q: number, 
/**
 * Destination row.
 */
target_r: number, };

export type CharacterLevelUpNotice = { 
/**
 * New character level.
 */
new_level: number, 
/**
 * New max HP after the level-up roll.
 */
new_max_hp: number, 
/**
 * HP gained from the level-up roll.
 */
hp_gained: number, };

export type CharacterXpGainedNotice = { 
/**
 * XP awarded this combat.
 */
xp_gained: number, 
/**
 * Character's new XP total.
 */
new_total: number, 
/**
 * XP needed to reach the next level.
 */
xp_next: number, };

export type DungeonEnterRoomNotice = { 
/**
 * Dungeon id.
 */
dungeon_id: string, 
/**
 * Room id.
 */
room_id: string, 
/**
 * Room display name.
 */
room_name: string, 
/**
 * Room type (e.g. `"corridor"`, `"chamber"`).
 */
room_type: string, 
/**
 * Direction entered from.
 */
direction: string, 
/**
 * Whether this is the first time the room was entered.
 */
first_visit: boolean, };

export type ExplorationEnterHexNotice = { 
/**
 * Entity id of the entering character (used by trigger `self_id` conditions).
 */
self_id: string, 
/**
 * Terrain id of the entered cell.
 */
terrain: string, 
/**
 * Feature ids on the entered cell.
 */
features: Array<string>, 
/**
 * Column of the entered cell.
 */
q: number, 
/**
 * Row of the entered cell.
 */
r: number, 
/**
 * Whether this is the first time the cell was entered.
 */
first_visit: boolean, };

export type GmSuggestionsNotice = { 
/**
 * Ordered list of command suggestions to show in the UI.
 */
commands: Array<string>, };

export type InventoryAmmoConsumedNotice = { 
/**
 * Character id.
 */
character_id: string, 
/**
 * Ammo type / item id.
 */
ammo_type: string, 
/**
 * Quantity remaining after consumption.
 */
remaining: number, };

export type InventoryItemGivenNotice = { 
/**
 * Character id.
 */
character_id: string, 
/**
 * The item payload (arbitrary item data object).
 */
item: Record<string, unknown>, };

export type LootSourceRevealedNotice = { 
/**
 * Cell column of the source.
 */
q: number, 
/**
 * Cell row of the source.
 */
r: number, 
/**
 * Stable source id within the cell.
 */
id: string, 
/**
 * `"ground"` / `"container"` / `"corpse"`.
 */
kind: string, 
/**
 * Display name, e.g. `"weathered crate"`.
 */
name: string, 
/**
 * Remaining item payloads in the source.
 */
items: Record<string, unknown>[], 
/**
 * Remaining currency in the source.
 */
gold: number, };

export type InventoryItemLostNotice = { 
/**
 * Character id.
 */
character_id: string, 
/**
 * Name of the lost item.
 */
item_name: string, };

export type OracleFateCheckNotice = { 
/**
 * The yes/no question asked.
 */
question: string, 
/**
 * Likelihood label (e.g. `"likely"`, `"50/50"`).
 */
likelihood: string, 
/**
 * Current chaos factor at the time of the check.
 */
chaos_factor: number, 
/**
 * The d100 roll result.
 */
roll: number, };

export type StatusRemovedNotice = { 
/**
 * Entity id.
 */
entity_id: string, 
/**
 * Effect id.
 */
effect_id: string, 
/**
 * Number removed.
 */
removed_count: number, };

export type ActiveStatusEffectPayload = { 
/**
 * Row id.
 */
id: number, 
/**
 * Entity id.
 */
entity_id: string, 
/**
 * Effect id.
 */
effect_id: string, 
/**
 * Applied-at tick.
 */
applied_at_tick: number, 
/**
 * Expires-at tick.
 */
expires_at_tick: number | null, 
/**
 * Source.
 */
source: string | null, 
/**
 * Arbitrary effect data.
 */
data: Record<string, unknown>, };

export type CellPreview = { 
/**
 * Column.
 */
q: number, 
/**
 * Row.
 */
r: number, 
/**
 * Terrain id.
 */
terrain: string, 
/**
 * Feature ids.
 */
features: Array<string>, 
/**
 * Whether explored (bool or int in legacy payloads — unknown in TS).
 */
explored: unknown, };

export type CharacterSnapshot = { 
/**
 * Id.
 */
id: string, 
/**
 * Name.
 */
name: string, 
/**
 * Class.
 */
character_class: string, 
/**
 * Level.
 */
level: number, 
/**
 * XP.
 */
xp: number, 
/**
 * XP to next.
 */
xp_next: number, 
/**
 * Attribute scores.
 */
attributes: { [key in string]: number }, 
/**
 * Attribute modifiers.
 */
attr_mods: { [key in string]: number }, 
/**
 * Skills.
 */
skills: { [key in string]: number }, 
/**
 * Current HP.
 */
hp: number, 
/**
 * Max HP.
 */
max_hp: number, 
/**
 * Armour class.
 */
ac: number, 
/**
 * Attack bonus.
 */
attack_bonus: number, 
/**
 * Physical save.
 */
physical_save: number, 
/**
 * Evasion save.
 */
evasion_save: number, 
/**
 * Mental save.
 */
mental_save: number, 
/**
 * Equipment (arbitrary item objects).
 */
equipment: Record<string, unknown>[], 
/**
 * Class abilities (arbitrary key-value map).
 */
class_abilities: Record<string, unknown>, 
/**
 * Position column.
 */
position_q: number, 
/**
 * Position row.
 */
position_r: number, };

export type CombatStartNotice = { 
/**
 * Awareness outcome.
 */
awareness: string, 
/**
 * Enemy display names (kept for backwards compatibility).
 */
enemies: Array<string>, 
/**
 * Full enemy states with HP for the combat roster panel.
 */
enemy_states: Array<EnemyCombatantState>, };

export type CombatActionsNotice = { 
/**
 * Combat phase: "active" | "last_stand" | "prompt" | "over".
 */
phase: string, 
/**
 * Ordered list of action buttons to render.
 */
actions: Array<CombatActionButton>, };

export type ExplorationActionsNotice = { 
/**
 * Ordered list of tile-context action buttons to render.
 */
actions: Array<CombatActionButton>, };

export type CombatPositionsNotice = { 
/**
 * Grid width in cells (always 9).
 */
width: number, 
/**
 * Grid height in cells (always 9).
 */
height: number, 
/**
 * Grid topology identifier (always `"square"`).
 */
grid_type: string, 
/**
 * All 81 cells in row-major order (r outer, q inner).
 */
cells: Array<PositionsCell>, 
/**
 * PC and all alive monsters.
 */
entities: Array<PositionsEntity>, 
/**
 * Loot piles dropped by defeated enemies (HR-787); empty until a kill.
 */
loot: Array<PositionsLoot>, };

export type StatusAppliedNotice = { 
/**
 * The active effect.
 */
active_effect: ActiveStatusEffectPayload, };

export type StatusExpiredNotice = { 
/**
 * The active effect.
 */
active_effect: ActiveStatusEffectPayload, };

export type TravelStepNotice = { 
/**
 * Character id.
 */
character_id: string, 
/**
 * From column.
 */
from_q: number, 
/**
 * From row.
 */
from_r: number, 
/**
 * To column.
 */
to_q: number, 
/**
 * To row.
 */
to_r: number, 
/**
 * Direction stepped.
 */
direction: string, 
/**
 * Whether the entered cell was previously unexplored.
 */
first_visit: boolean, 
/**
 * The cell stepped into.
 */
target_cell: CellPreview, };

export type ExplorationRevealedNotice = { 
/**
 * The revealed cell (carries `explored: 2` so the client renders it as "seen").
 */
cell: CellPreview, };

export type ExplorationMoved = { 
/**
 * Character id.
 */
character_id: string, 
/**
 * Character snapshot.
 */
character_data: CharacterSnapshot, 
/**
 * From column.
 */
from_q: number, 
/**
 * From row.
 */
from_r: number, 
/**
 * To column.
 */
to_q: number, 
/**
 * To row.
 */
to_r: number, 
/**
 * Direction.
 */
direction: string, 
/**
 * First visit.
 */
first_visit: boolean, 
/**
 * Target cell.
 */
target_cell: CellPreview, 
/**
 * Adjacent cells.
 */
adjacent_cells: Array<CellPreview>, 
/**
 * Optional weather data for the entered hex.
 */
weather: Record<string, unknown> | null, };

export type ClientEvent = { "event_type": "action.skill_check", "data": ActionSkillCheckNotice } | { "event_type": "character.created", "data": CharacterCreatedNotice } | { "event_type": "character.death", "data": CharacterDeathNotice } | { "event_type": "character.death_final", "data": CharacterDeathFinalNotice } | { "event_type": "character.expert_reroll", "data": CharacterExpertRerollNotice } | { "event_type": "character.hp_changed", "data": CharacterHpChangedNotice } | { "event_type": "character.level_up", "data": CharacterLevelUpNotice } | { "event_type": "character.respawn", "data": CharacterRespawnNotice } | { "event_type": "character.xp_gained", "data": CharacterXpGainedNotice } | { "event_type": "combat.actions", "data": CombatActionsNotice } | { "event_type": "combat.attack", "data": CombatAttackNotice } | { "event_type": "combat.enemy_defeated", "data": CombatEnemyDefeatedNotice } | { "event_type": "combat.fled", "data": CombatFledNotice } | { "event_type": "combat.player_hit", "data": CombatPlayerHitNotice } | { "event_type": "combat.positions", "data": CombatPositionsNotice } | { "event_type": "combat.save", "data": CombatSaveNotice } | { "event_type": "combat.start", "data": CombatStartNotice } | { "event_type": "dungeon.enter_room", "data": DungeonEnterRoomNotice } | { "event_type": "exploration.actions", "data": ExplorationActionsNotice } | { "event_type": "exploration.enter_hex", "data": ExplorationEnterHexNotice } | { "event_type": "exploration.moved", "data": ExplorationMoved } | { "event_type": "exploration.revealed", "data": ExplorationRevealedNotice } | { "event_type": "gm.scene_change", "data": SceneChangeNotice } | { "event_type": "gm.suggestions", "data": GmSuggestionsNotice } | { "event_type": "inventory.ammo_consumed", "data": InventoryAmmoConsumedNotice } | { "event_type": "inventory.item_given", "data": InventoryItemGivenNotice } | { "event_type": "loot.source_revealed", "data": LootSourceRevealedNotice } | { "event_type": "inventory.item_lost", "data": InventoryItemLostNotice } | { "event_type": "oracle.chaos_changed", "data": OracleChaosChangedNotice } | { "event_type": "oracle.fate_check", "data": OracleFateCheckNotice } | { "event_type": "oracle.random_event", "data": OracleRandomEventNotice } | { "event_type": "oracle.scene_check", "data": OracleSceneCheckNotice } | { "event_type": "quest.accepted", "data": QuestAcceptedNotice } | { "event_type": "quest.completed", "data": QuestCompletedNotice } | { "event_type": "quest.failed", "data": QuestFailedNotice } | { "event_type": "quest.progress_updated", "data": QuestProgressUpdatedNotice } | { "event_type": "shopping.purchase", "data": ShoppingPurchaseNotice } | { "event_type": "shopping.sale", "data": ShoppingSaleNotice } | { "event_type": "social.disposition_change", "data": SocialDispositionChangeNotice } | { "event_type": "social.healer", "data": SocialHealerNotice } | { "event_type": "status.applied", "data": StatusAppliedNotice } | { "event_type": "status.expired", "data": StatusExpiredNotice } | { "event_type": "status.removed", "data": StatusRemovedNotice } | { "event_type": "town.map", "data": TownMapNotice } | { "event_type": "town.move", "data": TownMoveNotice } | { "event_type": "travel.blocked", "data": TravelEndNotice } | { "event_type": "travel.cancelled", "data": TravelEndNotice } | { "event_type": "travel.completed", "data": TravelEndNotice } | { "event_type": "travel.interrupted", "data": TravelInterruptedNotice } | { "event_type": "travel.resumed", "data": TravelJourneyNotice } | { "event_type": "travel.started", "data": TravelJourneyNotice } | { "event_type": "travel.step", "data": TravelStepNotice };
