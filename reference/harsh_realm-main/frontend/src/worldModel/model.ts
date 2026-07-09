/**
 * Renderer-agnostic client world model — HR-795, Layer 2.
 *
 * Pure TypeScript; zero Vue imports.  Pinia (and any future renderer) is a
 * thin subscriber that reads change notifications and projects them reactively.
 *
 * Typical usage:
 *   import { createWorldModel, apply, subscribe } from "./model";
 *   import "./reducers"; // side-effect: populates reducerRegistry
 *
 *   const model = createWorldModel();
 *   subscribe(model, (c) => { if (c.kind === "world") reRenderMap(); });
 *   apply(model, event);
 */
import type {
  ClientEvent,
  CombatActionButton,
  ActiveStatusEffectPayload,
} from "../types/events.gen";
import { emptyGrid, setCell, type Cell, type Grid } from "./grid";
import type { Coord } from "./grid";
import { SUPPRESSED_GAME_EVENTS } from "./suppressed";

export type { Coord, Grid };

// ---------------------------------------------------------------------------
// Re-export generated action-button type so reducers and subscribers share it
// ---------------------------------------------------------------------------

/** Action button shape (identical to the generated `CombatActionButton`). */
export type ActionButton = CombatActionButton;

// ---------------------------------------------------------------------------
// Status effect model
// ---------------------------------------------------------------------------

/**
 * Active status effect stored in CharacterModel.
 *
 * Mirrors `ActiveStatusEffectPayload` from events.gen.ts — no optional display
 * fields (name/description/icon) since those come from the REST `/api/character`
 * endpoint, not from events.
 */
export type StatusEffectModel = ActiveStatusEffectPayload;

// ---------------------------------------------------------------------------
// Equipment model
// ---------------------------------------------------------------------------

/**
 * Equipment snapshot held in CharacterModel.
 *
 * Mirrors `EquipmentItem` in `stores/game.ts` — all the fields that the REST
 * `/api/character` response carries (and that `InventoryPanel.vue` renders).
 * All fields beyond `name`/`type` are optional so the type is structurally
 * compatible with the store's `EquipmentItem`.
 */
export interface EquipmentModel {
  id?: string;
  item_id?: string;
  name: string;
  type: string;
  enc?: number;
  damage?: string;
  weapon_damage?: string;
  ac?: number;
  ac_bonus?: number;
  quantity?: number;
  shock_damage?: string;
  shock_ac_threshold?: number;
}

// ---------------------------------------------------------------------------
// Weather model
// ---------------------------------------------------------------------------

export interface WeatherModel {
  condition: string;
  temperature: string;
  description: string;
}

// ---------------------------------------------------------------------------
// CharacterModel
// ---------------------------------------------------------------------------

/**
 * Character state held in the world model.
 *
 * Mirrors the fields of `CharacterState` in `stores/game.ts` that are updated
 * by game events (HP, XP, gold, location, status effects).  Fields that only
 * change via REST refetch (name, class, level, equipment list) are present but
 * start at placeholder defaults; the `refetch-character` Change effect signals
 * 104c to overwrite them from `/api/character`.
 *
 * Chat messages, layout state, modals, and pendingInput live in Pinia only.
 */
export interface CharacterModel {
  id: string;
  name: string;
  characterClass: string;
  level: number;
  hp: number;
  maxHp: number;
  ac: number;
  xp: number;
  xpNext: number;
  gold: number;
  str: number;
  locationQ: number | null;
  locationR: number | null;
  terrain: string | null;
  features: string[];
  weather: WeatherModel | null;
  /** Derived display labels for active status effects (updated alongside statusEffects). */
  conditions: string[];
  statusEffects: StatusEffectModel[];
  equipment: EquipmentModel[];
}

// ---------------------------------------------------------------------------
// CombatEnemyModel
// ---------------------------------------------------------------------------

/**
 * One enemy in the typed combat roster (mirrors `CombatEnemy` in `stores/game.ts`).
 */
export interface CombatEnemyModel {
  entityId: string;
  name: string;
  hp: number;
  maxHp: number;
  defeated: boolean;
}

// ---------------------------------------------------------------------------
// CombatState
// ---------------------------------------------------------------------------

/** A loot pile on the encounter battle grid (HR-787). */
export interface EncounterLoot {
  q: number;
  r: number;
  /** Representative value → the view maps this to a rarity tier. */
  value: number;
  itemCount: number;
}

/** A revealed searchable loot source on a cell (HR-786), shown in the loot panel. */
export interface LootSourceView {
  q: number;
  r: number;
  id: string;
  kind: string;
  name: string;
  items: Array<Record<string, unknown>>;
  gold: number;
}

export interface CombatState {
  enemies: CombatEnemyModel[];
  /** Available action buttons (from `combat.actions`). */
  actions: ActionButton[];
  /** Current phase string (from `combat.actions`). */
  phase: string;
  /** Loot piles dropped by defeated enemies (from `combat.positions`). */
  loot: EncounterLoot[];
}

// ---------------------------------------------------------------------------
// WorldModel
// ---------------------------------------------------------------------------

/**
 * The canonical client world state.
 *
 * 104a: grid / position / scene data.
 * 104b: character stats, chaos, suggestions, combat roster, action bars,
 *       pending travel.
 */
export interface WorldModel {
  world: Grid;
  town: Grid;
  encounter: Grid;
  /**
   * Current player position (scene-dependent: world coord during
   * exploration/travel, town coord inside a settlement, approximate during
   * combat).  `null` before the first move event.
   */
  player: Coord | null;
  /** Active scene name as returned by `gm.scene_change`. */
  scene: string;

  // --- 104b additions -------------------------------------------------------

  /**
   * Typed character snapshot.  `null` until a `refetch-character` effect is
   * consumed by a 104c subscriber that calls `/api/character`.
   */
  character: CharacterModel | null;
  /** Mythic GME chaos factor (1–9); mirrors `gameStore.chaosFactor`. */
  chaos: number;
  /** Ordered command suggestions from `gm.suggestions`. */
  suggestions: string[];
  /** Typed combat roster + action bar (mirrors `gameStore.combatEnemies/Actions/Phase`). */
  combat: CombatState;
  /** Tile-context action buttons from `exploration.actions`. */
  explorationActions: ActionButton[];
  /** Interrupted travel destination awaiting resume/cancel (HR-408). */
  pendingTravel: Coord | null;
  /** Revealed searchable loot sources on the current cell (HR-786). */
  loot: LootSourceView[];

  // --- 104c additions -------------------------------------------------------

  /**
   * Raw building records from the current town (populated by `town.map`
   * events).  Projected to `townStore.buildings` by the 104c projection.
   */
  townBuildings: Record<string, unknown>[];
  /**
   * Current settlement name (populated by `town.map` events).
   * Projected to `townStore.settlementName` by the 104c projection.
   */
  townSettlementName: string;
}

export function createWorldModel(): WorldModel {
  return {
    world: emptyGrid("world"),
    town: emptyGrid("town"),
    encounter: emptyGrid("encounter"),
    player: null,
    scene: "exploration",
    character: null,
    chaos: 5,
    suggestions: [],
    combat: { enemies: [], actions: [], phase: "", loot: [] },
    explorationActions: [],
    pendingTravel: null,
    loot: [],
    townBuildings: [],
    townSettlementName: "",
  };
}

// ---------------------------------------------------------------------------
// Change notifications
// ---------------------------------------------------------------------------

/**
 * Discriminated-union change token emitted after a reducer mutates the model,
 * or as a side-effect instruction for a 104c subscriber.
 *
 * Region changes (`world`…`pendingTravel`): subscriber re-reads the named
 * region of the model.
 *
 * Effect changes (`message`, `refetch-character`, `refetch-map`): subscriber
 * executes the corresponding side-effect (add a chat line, call REST, etc.).
 * Nothing consumes these in 104b; 104c wires them.
 */
export type Change =
  // --- region changes (model state mutated) ---
  // `world`: `cells` present = incremental update of just those cells (a move);
  // `cells` absent = full rebuild from model.world (hydrate / initial load).
  | { kind: "world"; cells?: Cell[] }
  | { kind: "town" }
  | { kind: "encounter" }
  | { kind: "player" }
  | { kind: "scene" }
  | { kind: "character" }
  | { kind: "chaos" }
  | { kind: "suggestions" }
  | { kind: "combat" }
  | { kind: "explorationActions" }
  | { kind: "pendingTravel" }
  | { kind: "loot" }
  // --- effect changes (subscriber must act) ---
  | { kind: "message"; text: string; style: string }
  | { kind: "refetch-character" }
  | { kind: "refetch-map" }
  | { kind: "refetch-quests" }
  // Dismiss the character-creation attribute-roll modal (chat-creation flow).
  | { kind: "clear-attribute-roll" };

type Listener = (c: Change) => void;

/** Keyed by model instance so listeners are GC-ed with the model. */
const modelListeners = new WeakMap<WorldModel, Set<Listener>>();

/**
 * Subscribe to model changes.  Returns an unsubscribe function.
 */
export function subscribe(
  model: WorldModel,
  listener: Listener,
): () => void {
  let set = modelListeners.get(model);
  if (set === undefined) {
    set = new Set<Listener>();
    modelListeners.set(model, set);
  }
  // Capture as `const` so the closure below holds a non-optional reference.
  const listenerSet: Set<Listener> = set;
  listenerSet.add(listener);
  return (): void => {
    listenerSet.delete(listener);
  };
}

export function notify(model: WorldModel, change: Change): void {
  const set = modelListeners.get(model);
  if (set !== undefined) {
    for (const listener of set) {
      listener(change);
    }
  }
}

// ---------------------------------------------------------------------------
// Reducer registry
// ---------------------------------------------------------------------------

type ReducerFn = (model: WorldModel, data: unknown) => Change[];

/**
 * Populated by `reducers.ts` (side-effect import).
 *
 * Exported so that `reducers.ts` can register into it and tests can inspect
 * the registered set.
 */
export const reducerRegistry: Partial<
  Record<ClientEvent["event_type"], ReducerFn>
> = {};

// ---------------------------------------------------------------------------
// apply
// ---------------------------------------------------------------------------

/**
 * Applies a typed client event to the model via the reducer registry.
 *
 * If no reducer is registered for the event type, emits a dev-mode warning
 * and no-ops (mirrors the HR-792 behaviour in `_websocketHandlers.ts`).
 */
export function apply(model: WorldModel, event: ClientEvent): void {
  const reducer = reducerRegistry[event.event_type];
  if (reducer === undefined) {
    // Intentionally-suppressed events have no reducer by design — stay quiet.
    // Only warn for genuinely-unknown event types (the HR-792 failure class).
    if (
      import.meta.env.DEV &&
      !SUPPRESSED_GAME_EVENTS.includes(event.event_type)
    ) {
      console.warn(
        `[worldModel] no reducer for "${event.event_type}" — add one in reducers.ts`,
        event,
      );
    }
    return;
  }
  // `event.data` is the union of all payload types, all of which are assignable
  // to `unknown` — safe to pass to the ReducerFn parameter.
  const changes = reducer(model, event.data);
  for (const change of changes) {
    notify(model, change);
  }
}

// ---------------------------------------------------------------------------
// Hydration from REST snapshots
// ---------------------------------------------------------------------------

/**
 * Shape of the `GET /api/worlds/current/map` REST snapshot (mirrors the shape
 * that `mapStore.loadMap()` consumes).
 */
export interface MapSnapshot {
  width: number;
  height: number;
  cells: Cell[];
}

/**
 * Loads the world grid from the REST map snapshot (`GET /api/worlds/current/map`).
 *
 * Mirrors the logic in `mapStore.loadMap()`.
 */
export function hydrateWorld(model: WorldModel, mapJson: MapSnapshot): void {
  model.world.width = mapJson.width;
  model.world.height = mapJson.height;
  model.world.cells.clear();
  for (const cell of mapJson.cells) {
    setCell(model.world, cell);
  }
  notify(model, { kind: "world" });
}

/**
 * Stub for future town hydration (REST snapshot shape TBD in 104b).
 *
 * Currently a no-op placeholder; 104b will implement this once the town REST
 * endpoint shape is confirmed.
 */
export function hydrateTown(
  _model: WorldModel,
  _snapshot: Record<string, unknown>,
): void {
  // TODO(104b): implement when town REST snapshot endpoint is available.
}
