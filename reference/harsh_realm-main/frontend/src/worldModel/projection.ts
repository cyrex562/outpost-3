/**
 * Pinia projection for the world model (HR-795, 104c).
 *
 * A single `subscribe` listener maps every `Change` notification from the
 * `WorldModel` into reactive updates on the Pinia stores.  Components read
 * the same store refs they always have; they are unmodified.
 *
 * Region changes re-project the corresponding region of the model into the
 * store.  Effect changes trigger side-effects (REST refetch, addMessage).
 *
 * Install once with `installProjection(worldModel, deps)` — the returned
 * function unsubscribes the listener.
 */
import type { WorldModel } from "./model";
import { subscribe } from "./model";
import { loadCharacter, loadWorldMap } from "./hydrate";
import { coordKey, type Cell } from "./grid";
import type { useGameStore } from "../stores/game";
import type { useMapStore, CellData } from "../stores/map";
import type { useTownStore } from "../stores/town";
import type { useEncounterStore } from "../stores/encounter";
import type { useLootStore } from "../stores/loot";
import type { ActiveStatusEffect } from "../types/api";
import type { PositionsCell, PositionsEntity } from "../types/api";
import type { CombatActionButton } from "../types/api";

// ---------------------------------------------------------------------------
// Dependency types
// ---------------------------------------------------------------------------

export interface ProjectionDeps {
  gameStore: ReturnType<typeof useGameStore>;
  mapStore: ReturnType<typeof useMapStore>;
  townStore: ReturnType<typeof useTownStore>;
  encounterStore: ReturnType<typeof useEncounterStore>;
  lootStore: ReturnType<typeof useLootStore>;
}

// ---------------------------------------------------------------------------
// Module-level guard: only install one projection per app lifetime
// ---------------------------------------------------------------------------

let _installed = false;

/**
 * Trailing-debounce timer for `refetch-character` (HR-779).
 *
 * A combat victory publishes several `inventory.*` events in the same burst
 * (one per looted item + one for currency), each emitting a `refetch-character`
 * effect.  Without debouncing that fires N concurrent `GET /api/character`
 * requests with a last-wins race.  Coalesce the burst into a single refetch on
 * a short trailing delay (mirrors the old `scheduleInventoryRefresh`).
 */
let refetchCharacterTimer: ReturnType<typeof setTimeout> | undefined;
const REFETCH_CHARACTER_DEBOUNCE_MS = 60;

// ---------------------------------------------------------------------------
// Projection helpers
// ---------------------------------------------------------------------------

/** Projects a model `Cell` into the mapStore's `CellData` shape. */
function projectCell(cell: Cell): CellData {
  const rec = cell as Record<string, unknown>;
  return {
    q: cell.q,
    r: cell.r,
    terrain: cell.terrain,
    features: cell.features,
    explored: cell.explored,
    ...(typeof rec.data === "object" && rec.data !== null
      ? { data: rec.data as Record<string, unknown> }
      : {}),
  };
}

/** Projects `model.character` into the gameStore shape (or null). */
function projectCharacter(
  model: WorldModel,
  gameStore: ReturnType<typeof useGameStore>,
): void {
  if (model.character === null) {
    gameStore.character = null;
    return;
  }
  const ch = model.character;
  gameStore.character = {
    id: ch.id,
    name: ch.name,
    characterClass: ch.characterClass,
    level: ch.level,
    hp: ch.hp,
    maxHp: ch.maxHp,
    ac: ch.ac,
    xp: ch.xp,
    xpNext: ch.xpNext,
    gold: ch.gold,
    str: ch.str,
    locationQ: ch.locationQ,
    locationR: ch.locationR,
    terrain: ch.terrain,
    features: ch.features,
    weather: ch.weather,
    conditions: ch.conditions,
    statusEffects: ch.statusEffects as ActiveStatusEffect[],
    equipment: ch.equipment,
  };
}

// ---------------------------------------------------------------------------
// installProjection
// ---------------------------------------------------------------------------

/**
 * Subscribes to the world model and projects every change into the Pinia
 * stores.  Call once, after the WebSocket connection is established.
 *
 * Returns the unsubscribe function (useful for hot-module reload / tests).
 */
export function installProjection(
  model: WorldModel,
  deps: ProjectionDeps,
): () => void {
  if (_installed) {
    if (import.meta.env.DEV) {
      console.warn("[worldModel] installProjection called more than once — ignoring duplicate");
    }
    // Return a no-op so callers can safely store the unsubscribe fn.
    return () => { /* no-op */ };
  }
  _installed = true;

  const { gameStore, mapStore, townStore, encounterStore, lootStore } = deps;

  const unsubscribe = subscribe(model, (change) => {
    switch (change.kind) {
      // ----- region changes --------------------------------------------------

      case "world": {
        if (change.cells !== undefined) {
          // Incremental update (a move): set just the changed cells in place.
          // Vue 3 reactively tracks Map.set on a ref'd Map — this is exactly
          // what the old mapStore.updateCell(cell) did (O(1) per cell).
          for (const cell of change.cells) {
            mapStore.cells.set(coordKey(cell.q, cell.r), projectCell(cell));
          }
          // width/height/loaded only change on hydrate (full rebuild), so a
          // guarded `loaded` refresh is enough here for the first-move case.
          if (!mapStore.loaded && model.world.cells.size > 0) {
            mapStore.loaded = true;
          }
        } else {
          // Full rebuild (hydrate / initial load): rebuild the whole Map.
          const newCells = new Map<string, CellData>();
          for (const [key, cell] of model.world.cells) {
            newCells.set(key, projectCell(cell));
          }
          mapStore.cells = newCells;
          mapStore.width = model.world.width;
          mapStore.height = model.world.height;
          mapStore.loaded = model.world.cells.size > 0;
        }
        break;
      }

      case "player": {
        if (model.player === null) break;
        const { q, r } = model.player;
        if (model.town.cells.size > 0) {
          // Town mode: player position tracked by townStore
          townStore.playerQ = q;
          townStore.playerR = r;
        } else {
          // World mode: player position tracked by mapStore
          mapStore.setPlayerPosition(q, r);
        }
        break;
      }

      case "town": {
        if (model.town.cells.size === 0) {
          townStore.clear();
        } else {
          // Convert model.town.cells (Map) → TownCell[]
          const cells = Array.from(model.town.cells.values()).map((c) => ({
            q: c.q,
            r: c.r,
            terrain: c.terrain,
            features: c.features,
          }));
          // Convert raw building records → TownBuilding[]
          const buildings = model.townBuildings.map((b) => ({
            name: typeof b.name === "string" ? b.name : "",
            type: typeof b.type === "string" ? b.type : "",
          }));
          townStore.cells = cells;
          townStore.buildings = buildings;
          townStore.settlementName = model.townSettlementName;
          townStore.active = true;
        }
        break;
      }

      case "encounter": {
        if (
          model.encounter.cells.size === 0 &&
          model.encounter.entities.length === 0
        ) {
          encounterStore.clear();
        } else {
          const pcells: PositionsCell[] = Array.from(
            model.encounter.cells.values(),
          ).map((c) => ({
            q: c.q,
            r: c.r,
            terrain: c.terrain,
            features: c.features,
          }));
          const pentities: PositionsEntity[] = model.encounter.entities.map(
            (e) => ({
              entity_id: e.id,
              kind: e.kind === "pc" ? "pc" : ("monster" as "pc" | "monster"),
              name: e.name ?? "",
              q: e.q,
              r: e.r,
              hp: e.hp ?? 0,
              max_hp: e.maxHp ?? 0,
              alive: e.alive ?? true,
            }),
          );
          encounterStore.setGrid({
            width: model.encounter.width,
            height: model.encounter.height,
            grid_type: "square",
            cells: pcells,
            entities: pentities,
            loot: model.combat.loot.map((l) => ({
              q: l.q,
              r: l.r,
              value: l.value,
              item_count: l.itemCount,
            })),
          });
          // Mirror the old encounterStore.markDefeated() behaviour: if the
          // currently-selected attack target is now dead or gone, clear the
          // selection so a subsequent Attack doesn't retarget a dead enemy.
          const sel = encounterStore.selectedTargetId;
          if (
            sel &&
            !model.encounter.entities.some((e) => e.id === sel && e.alive)
          ) {
            encounterStore.selectTarget(null);
          }
        }
        break;
      }

      case "scene": {
        gameStore.setScene(model.scene);
        break;
      }

      case "character": {
        projectCharacter(model, gameStore);
        // After character hydration (refetch-character path), also update the
        // map-store player position from the character's REST location field.
        // This mirrors the `useMapStore().setPlayerPosition()` call that the
        // old `gameStore.loadCharacter()` made after setting character.value.
        // Events that emit `player` separately (exploration.moved, travel.step)
        // will also fire the `player` case below, making this idempotent.
        if (
          model.character !== null &&
          model.character.locationQ !== null &&
          model.character.locationR !== null &&
          model.town.cells.size === 0
        ) {
          mapStore.setPlayerPosition(
            model.character.locationQ,
            model.character.locationR,
          );
        }
        break;
      }

      case "chaos": {
        gameStore.setChaos(model.chaos);
        break;
      }

      case "suggestions": {
        gameStore.addSuggestions(model.suggestions);
        break;
      }

      case "combat": {
        // Project the typed combat roster + action bar into gameStore.
        // CombatEnemyModel and CombatEnemy are structurally identical.
        gameStore.combatEnemies = model.combat.enemies.map((e) => ({ ...e }));
        gameStore.combatActions = model.combat.actions as CombatActionButton[];
        gameStore.combatPhase = model.combat.phase;
        break;
      }

      case "explorationActions": {
        gameStore.setExplorationActions(
          model.explorationActions as CombatActionButton[],
        );
        break;
      }

      case "pendingTravel": {
        gameStore.setPendingTravel(model.pendingTravel);
        break;
      }

      case "loot": {
        // HR-786: project revealed loot sources into the loot panel store.
        lootStore.setSources(model.loot);
        break;
      }

      // ----- effect changes --------------------------------------------------

      case "message": {
        // All game_event–sourced messages use the "system" sender (narration
        // uses "gm" but that is handled directly in the narration frame branch
        // of _websocketHandlers.ts, never as a Change).
        gameStore.addMessage(
          "system",
          change.text,
          change.style as Parameters<typeof gameStore.addMessage>[2],
        );
        break;
      }

      case "refetch-character": {
        // HR-779: coalesce a burst of refetch-character effects into a single
        // trailing GET /api/character (mirrors the old scheduleInventoryRefresh).
        if (refetchCharacterTimer !== undefined) {
          clearTimeout(refetchCharacterTimer);
        }
        refetchCharacterTimer = setTimeout(() => {
          refetchCharacterTimer = undefined;
          void loadCharacter(model).finally(() => {
            gameStore.characterChecked = true;
            // HR-19: seed the active-quest count whenever the character loads.
            const id = gameStore.character?.id;
            if (id) void gameStore.loadQuests(id);
          });
        }, REFETCH_CHARACTER_DEBOUNCE_MS);
        break;
      }

      case "refetch-quests": {
        const id = gameStore.character?.id;
        if (id) void gameStore.loadQuests(id);
        break;
      }

      case "clear-attribute-roll": {
        // Dismiss any lingering attribute-roll modal from the chat-creation flow.
        gameStore.setAttributeRoll(null);
        break;
      }

      case "refetch-map": {
        void loadWorldMap(model).then((ok) => {
          if (!ok) mapStore.loaded = false;
        });
        break;
      }
    }
  });

  return () => {
    _installed = false;
    unsubscribe();
  };
}
