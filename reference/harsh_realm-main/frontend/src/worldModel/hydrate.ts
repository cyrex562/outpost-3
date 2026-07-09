/**
 * REST hydration for the world model (HR-795, 104c).
 *
 * Pure TypeScript — zero Vue imports.  Provides two load functions:
 *
 *   loadCharacter(model)  — GET /api/character + status effects → model
 *   loadWorldMap(model)   — GET /api/worlds/current/map → model
 *
 * Both functions mutate the model and emit Change notifications, which the
 * projection subscriber converts into Pinia store updates.
 *
 * Also exports the lower-level parse helpers
 * (`hydrateCharacterFromApi`, `hydrateWorldMapFromApi`) so callers that
 * already have the raw API response can skip the fetch.
 */
import type { WorldModel, EquipmentModel, StatusEffectModel } from "./model";
import { notify } from "./model";
import { type Cell, coordKey } from "./grid";

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/** Derives a display label from a status-effect (mirrors effectLabel in game.ts). */
function statusLabel(effect: StatusEffectModel): string {
  // Prefer the display `name` when the REST payload carries one (matches the
  // old effectLabel: `effect.name ?? lastIdSegment ?? effect_id`). `name` is
  // not on the generated payload type, so read it defensively.
  const name = (effect as Record<string, unknown>).name;
  if (typeof name === "string") return name;
  const parts = effect.effect_id.split(".");
  return parts[parts.length - 1] ?? effect.effect_id;
}

/** Type guard for the required fields of ActiveStatusEffectPayload. */
function isStatusEffect(v: unknown): v is StatusEffectModel {
  if (typeof v !== "object" || v === null) return false;
  const r = v as Record<string, unknown>;
  return (
    typeof r.id === "number" &&
    typeof r.entity_id === "string" &&
    typeof r.effect_id === "string" &&
    typeof r.applied_at_tick === "number" &&
    (typeof r.expires_at_tick === "number" || r.expires_at_tick === null) &&
    (typeof r.source === "string" || r.source === null) &&
    typeof r.data === "object" &&
    r.data !== null
  );
}

// ---------------------------------------------------------------------------
// hydrateCharacterFromApi
// ---------------------------------------------------------------------------

/**
 * Parses a raw `GET /api/character` JSON response and writes the result into
 * `model.character`.  Mirrors the parse logic in `gameStore.loadCharacter()`.
 *
 * Emits a `character` change notification after writing.
 */
export function hydrateCharacterFromApi(
  model: WorldModel,
  apiData: Record<string, unknown>,
): void {
  const classAbilities = (apiData.class_abilities ?? {}) as Record<
    string,
    unknown
  >;
  const rawEquip = (apiData.equipment ?? []) as EquipmentModel[];

  model.character = {
    id: String(apiData.id ?? ""),
    name: String(apiData.name ?? ""),
    characterClass: String(apiData.character_class ?? "unknown"),
    level: Number(apiData.level ?? 1),
    hp: Number(apiData.hp ?? 0),
    maxHp: Number(apiData.max_hp ?? 0),
    ac: Number(apiData.ac ?? 10),
    xp: Number(apiData.xp ?? 0),
    xpNext: Number(apiData.xp_next ?? 0),
    gold: Number(classAbilities.gold ?? 0),
    str: Number(
      (classAbilities.attributes as Record<string, unknown> | undefined)
        ?.str ??
        apiData.str ??
        10,
    ),
    locationQ: apiData.location_q != null ? Number(apiData.location_q) : null,
    locationR: apiData.location_r != null ? Number(apiData.location_r) : null,
    terrain: null,
    features: [],
    weather: null,
    conditions: [],
    statusEffects: [],
    equipment: rawEquip,
  };

  notify(model, { kind: "character" });
}

// ---------------------------------------------------------------------------
// hydrateWorldMapFromApi
// ---------------------------------------------------------------------------

/**
 * Parses a raw `GET /api/worlds/current/map` JSON response and writes the
 * world grid into `model.world`.  Mirrors the parse logic in
 * `mapStore.loadMap()`.
 *
 * Emits a `world` change notification after writing.
 */
export function hydrateWorldMapFromApi(
  model: WorldModel,
  mapData: { width: number; height: number; cells: unknown[] },
): void {
  model.world.width = mapData.width;
  model.world.height = mapData.height;
  model.world.cells.clear();

  for (const rawCell of mapData.cells) {
    const c = rawCell as Record<string, unknown>;
    const q = typeof c.q === "number" ? c.q : 0;
    const r = typeof c.r === "number" ? c.r : 0;
    const explored: number | boolean =
      typeof c.explored === "number" || typeof c.explored === "boolean"
        ? c.explored
        : 1;
    const cell: Cell = {
      q,
      r,
      terrain: typeof c.terrain === "string" ? c.terrain : "unknown",
      features: Array.isArray(c.features) ? (c.features as string[]) : [],
      explored,
    };
    // Preserve any extra fields (e.g. `data` on world cells)
    for (const [k, v] of Object.entries(c)) {
      if (!(k in cell)) {
        (cell as Record<string, unknown>)[k] = v;
      }
    }
    model.world.cells.set(coordKey(q, r), cell);
  }

  notify(model, { kind: "world" });
}

// ---------------------------------------------------------------------------
// loadStatusEffectsIntoModel (internal)
// ---------------------------------------------------------------------------

async function loadStatusEffectsIntoModel(
  model: WorldModel,
  entityId: string,
): Promise<void> {
  try {
    const r = await fetch(
      `/api/character/${encodeURIComponent(entityId)}/status_effects`,
    );
    if (!r.ok) return;
    const data = (await r.json()) as unknown;
    if (
      !Array.isArray(data) ||
      model.character === null ||
      model.character.id !== entityId
    ) {
      return;
    }
    model.character.statusEffects = data.filter(isStatusEffect);
    model.character.conditions =
      model.character.statusEffects.map(statusLabel);
    notify(model, { kind: "character" });
  } catch {
    /* silently ignore */
  }
}

// ---------------------------------------------------------------------------
// loadCharacter
// ---------------------------------------------------------------------------

/**
 * Fetches `GET /api/character`, hydrates `model.character`, then fetches
 * status effects and hydrates those too.
 *
 * The callers in `useWebSocket.ts` set `gameStore.characterChecked = true`
 * in a `.finally()` after this resolves, matching the old store behaviour.
 */
export async function loadCharacter(model: WorldModel): Promise<void> {
  try {
    const r = await fetch("/api/character");
    if (!r.ok) return;
    const data = (await r.json()) as Record<string, unknown>;
    hydrateCharacterFromApi(model, data);
    const charId = model.character?.id;
    if (charId) {
      await loadStatusEffectsIntoModel(model, charId);
    }
  } catch {
    /* silently ignore */
  }
}

// ---------------------------------------------------------------------------
// loadWorldMap
// ---------------------------------------------------------------------------

/**
 * Fetches `GET /api/worlds/current/map` and hydrates `model.world`.
 *
 * Returns `true` on success so callers can set `mapStore.loaded = false`
 * on failure (matching the old `mapStore.loadMap()` error path).
 */
export async function loadWorldMap(model: WorldModel): Promise<boolean> {
  try {
    const r = await fetch("/api/worlds/current/map");
    if (!r.ok) return false;
    const data = (await r.json()) as {
      width: number;
      height: number;
      cells: unknown[];
    };
    hydrateWorldMapFromApi(model, data);
    return true;
  } catch {
    return false;
  }
}
