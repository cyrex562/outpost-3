/**
 * TypeScript interfaces for all API response shapes.
 * Used throughout the frontend for type-safe API interactions.
 */

// ---------------------------------------------------------------------------
// JSON-shaped data (for editor records and request bodies without a fixed schema)
// ---------------------------------------------------------------------------

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface JsonObject {
  [key: string]: JsonValue;
}

/**
 * A loosely-typed editor content record (always has an id, plus arbitrary
 * fields). The index signature is intentionally ``unknown`` (not the recursive
 * ``JsonValue``) so the type stays shallow for Vue's reactivity type machinery.
 */
export interface EditorRecord {
  id: string;
  [key: string]: unknown;
}

/** A per-world random table editor record. */
export interface RandomTable {
  id: string;
  category: string;
  name: string;
  entries: JsonObject[];
  tags: string[];
  source?: string;
}

/** An Oracle plot thread editor record. */
export interface OracleThread {
  id: string;
  title: string;
  type: string;
  status: string;
  progress: number;
  target?: number;
}

/** An Oracle NPC editor record. */
export interface OracleNpc {
  id: string;
  name: string;
  status?: string;
  notes?: string;
}

// ---------------------------------------------------------------------------
// Admin config types (M4)
// ---------------------------------------------------------------------------

export interface SkillMapping {
  verb: string
  skill: string
  attribute: string
  base_difficulty: number
  opposed: boolean
  description: string
  notes: string
}

export interface DifficultyTarget {
  name: string
  target: number
  description: string
}

export interface DispositionOutcome {
  outcome_key: string
  delta: number
  description: string
}

export interface EncounterWeight {
  faction_disposition: string
  encounter_tag: string
  weight_modifier: number
}

export interface FactionAssetStat {
  asset_type: string
  category: string
  min_attribute: number
  cost: number
  upkeep: number
  max_hp: number
  attack_stat: string
  counter_stat: string
  attack_roll: string
  special: string
  description: string
}

export interface PackDependency {
  id: string
  version: string
}

export interface PackManifest {
  id: string
  version: string
  name: string
  description: string
  authors: string[]
  depends: PackDependency[]
  conflicts: string[]
  provides: string[]
}

export interface WorldPackSummary {
  pack_id: string
  version: string
  load_order: number
}

// ---------------------------------------------------------------------------
// World state types (M4.5 editor)
// ---------------------------------------------------------------------------

export interface CellData {
  q: number
  r: number
  terrain: string
  features: string[]
  explored: boolean
  description: string | null
  faction_id: string | null
  data: Record<string, unknown>
}

export interface Character {
  id: string
  entity_type: string
  name: string
  location_q: number
  location_r: number
  alive: boolean
  data: CharacterData
}

export interface ActiveStatusEffect {
  id: number
  entity_id: string
  effect_id: string
  applied_at_tick: number
  expires_at_tick: number | null
  source: string | null
  data: Record<string, unknown>
  name?: string
  description?: string
  icon?: string
}

export interface CharacterData {
  id: string
  name: string
  character_class: string
  level: number
  xp: number
  xp_next: number
  attributes: Record<string, number>
  attr_mods: Record<string, number>
  skills: Record<string, number>
  hp: number
  max_hp: number
  ac: number
  attack_bonus: number
  physical_save: number
  evasion_save: number
  mental_save: number
  equipment: ItemData[]
  class_abilities: Record<string, unknown>
  position_q: number
  position_r: number
  personality?: unknown
}

export interface RecalcResult {
  attr_mods: Record<string, number>
  max_hp: number
  ac: number
  attack_bonus: number
  melee_attack: number
  ranged_attack: number
  physical_save: number
  evasion_save: number
  mental_save: number
}

export interface ItemData {
  name: string
  type: string
  price?: number
  weapon_damage?: string
  ac_bonus?: number
  heal?: number
  description?: string
  [key: string]: unknown
}

export interface Faction {
  id: string
  name: string
  hp: number
  max_hp: number
  force: number
  cunning: number
  wealth: number
  xp: number
  home_q: number | null
  home_r: number | null
  goals: string[]
  tags: string[]
  data: Record<string, unknown>
  assets?: FactionAssetInstance[]
  relations?: FactionRelation[]
}

export interface FactionAssetInstance {
  id: string
  faction_id?: string
  asset_type: string
  category: string
  hp: number
  max_hp: number
  location_q: number | null
  location_r: number | null
  data: Record<string, unknown>
}

export interface FactionRelation {
  faction_a: string
  faction_b: string
  disposition: string
}

export interface Dungeon {
  id: string
  name: string
  location_q: number | null
  location_r: number | null
  rooms: DungeonRoom[]
  connections: DungeonConnection[]
  data: Record<string, unknown>
}

/** Compact dungeon entry returned by the dungeon list endpoint. */
export interface DungeonSummary {
  id: string
  name: string
  location_q: number | null
  location_r: number | null
  room_count: number
}

export interface DungeonRoom {
  id: string
  name: string
  type: string
  description: string
  data: Record<string, unknown>
}

export interface DungeonConnection {
  from: string
  to: string
  direction: string
  passage: string
  notes: string
}

// ---------------------------------------------------------------------------
// World management types
// ---------------------------------------------------------------------------

export interface WorldListItem {
  name: string
  file: string
  last_modified: string
  /** File size in bytes; not currently populated by the backend list endpoint. */
  size?: number | undefined
}

export interface WorldMeta {
  [key: string]: string
}

export interface YAMLFileEntry {
  path: string
  size: number
}

// ---------------------------------------------------------------------------
// Game event types (WebSocket)
// ---------------------------------------------------------------------------

export interface GameEvent {
  tick: number
  event_type: string
  data: Record<string, unknown>
  source: string
  id: string
  timestamp: string
}

export interface WebSocketMessage {
  type: string
  data: GameEvent | Record<string, unknown>
}

// ---------------------------------------------------------------------------
// Native (Tauri) core — shapes returned by Rust IPC commands. These mirror the
// `harsh-core` Rust structs (and the Python engine results they were ported
// from), so the same data flows whether a roll is computed natively or in Python.
// ---------------------------------------------------------------------------

export interface DiceResult {
  rolls: number[]
  total: number
  modifier: number
  final: number
}

// ---------------------------------------------------------------------------
// Content authoring (R5-C compiler)
// ---------------------------------------------------------------------------

/** One compiler diagnostic in a bucket. */
export interface ContentDiagnostic {
  class: string
  message: string
  location?: string | null
}

/** The two-bucket compile report returned by POST /api/content/compile. */
export interface ContentCompileReport {
  syntax_errors: ContentDiagnostic[]
  unimplemented: ContentDiagnostic[]
  records: JsonObject[]
}

/** A migratable legacy pack category with its record count. */
export interface LegacyCategory {
  category: string
  count: number
}

/** Response from POST /api/content/migrate. */
export interface MigrateResponse {
  yaml: string
  needs: string[]
  unmet: string[]
  notes: string[]
  count: number
}

/** Response from POST /api/content/store. */
export interface StoreResponse {
  report: ContentCompileReport
  stored: number
}

/** One resolved attack in a content demo combat transcript. */
export interface DemoTurn {
  round: number
  attacker: string
  target: string
  hit: boolean
  damage: number
  narration: string
  triggers: string[]
  attacker_hp: number
  target_hp: number
}

/** Final state of one combatant when a demo combat ends. */
export interface DemoCombatantState {
  entity_id: string
  name: string
  is_player: boolean
  hp: number
  max_hp: number
  alive: boolean
  statuses: string[]
}

/** The combat outcome returned inside a demo response. */
export interface DemoOutcome {
  transcript: DemoTurn[]
  rounds: number
  winner: string
  combatants: DemoCombatantState[]
  errors: string[]
}

/** Response from POST /api/demo/combat. */
export interface DemoCombatResponse {
  compile: ContentCompileReport
  outcome: DemoOutcome | null
  error: string | null
}

export interface RuntimeConfig {
  admin_mode: boolean
  run_mode: string
  version: string
}

// ---------------------------------------------------------------------------
// Combat action bar types (HR-110)
// ---------------------------------------------------------------------------

/** A single button in the `combat.actions` payload. */
export interface CombatActionButton {
  command: string;
  label: string;
  /** "primary" | "danger" | "default" */
  style: string;
}

/** Payload of the `combat.actions` event. */
export interface CombatActionsPayload {
  phase: string;
  actions: CombatActionButton[];
}

/** Payload of the `exploration.actions` event (persistent tile-action bar). */
export interface ExplorationActionsPayload {
  actions: CombatActionButton[];
}

// ---------------------------------------------------------------------------
// Encounter / battle-grid types (HR-755)
// ---------------------------------------------------------------------------

export interface PositionsCell {
  q: number;
  r: number;
  terrain: string;
  features: string[];
}

export interface PositionsEntity {
  entity_id: string;
  kind: "pc" | "monster";
  name: string;
  q: number;
  r: number;
  hp: number;
  max_hp: number;
  alive: boolean;
}

/** A loot pile on the encounter battle grid (HR-787). */
export interface PositionsLoot {
  q: number;
  r: number;
  /** Representative value (top item value or currency) for the rarity tier. */
  value: number;
  item_count: number;
}

export interface CombatPositionsData {
  width: number;
  height: number;
  grid_type: string;
  cells: PositionsCell[];
  entities: PositionsEntity[];
  loot: PositionsLoot[];
}

// --- Character creation (HR-400) ---

export type AttributeMethod = "roll" | "standard_array" | "point_buy"

export interface RecommendedSkill {
  skill: string
  level: number
}

export interface ClassOption {
  id: string
  name: string
  description: string
  hit_die: number
  skill_points_start: number
  abilities: string[]
  recommended_skills: RecommendedSkill[]
}

export interface AutoPickSkillsResult {
  skills: Record<string, number>
  summary: string
}

export interface SkillRequirement {
  type: string
  message?: string | null
  classes: string[]
  attribute?: string | null
  min?: number | null
  trait_id?: string | null
}

export interface SkillOption {
  id: string
  name: string
  description: string
  default_attribute: string
  can_use_untrained: boolean
  combat_skill: boolean
  available_milestone: number
  category: string
  subcategory?: string | null
  requirements: SkillRequirement[]
}

export interface KitItemOption {
  name: string
  type: string
  damage?: string
  ac?: number
  ac_bonus?: number
  enc?: number
  quantity?: number
}

export interface EquipmentKitOption {
  id: string
  name: string
  /** Serialized via the model's `class` alias. */
  class: string
  description: string
  items: KitItemOption[]
}

export interface PointBuyConfig {
  budget: number
  min_score: number
  max_score: number
  costs: Record<number, number>
}

export interface CharacterCreationOptions {
  attribute_order: string[]
  methods: AttributeMethod[]
  standard_array: number[]
  point_buy: PointBuyConfig
  classes: ClassOption[]
  skills: SkillOption[]
  kits: EquipmentKitOption[]
  current_milestone: number
}

export interface RolledAttributes {
  scores: number[]
}

export interface CharacterDraftSubmission {
  name: string
  character_class: string
  attribute_method: AttributeMethod
  attributes: Record<string, number>
  skills: Record<string, number>
  kit_id: string
  rolled_scores?: number[]
}

export interface CharacterCreatedResult {
  ok: boolean
  character_id: string
  name: string
}

/** One adjustable-difficulty dimension (HR-107), self-describing for the UI. */
export interface DifficultyDimension {
  key: string
  label: string
  description: string
  grade: number
  grade_count: number
  easy_label: string
  hard_label: string
  normal_grade: number
  current_effect: string
}

/** Response body of `GET /api/difficulty`. */
export interface DifficultySettingsResponse {
  dimensions: DifficultyDimension[]
}
