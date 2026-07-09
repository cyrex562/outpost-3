// Pure helpers for the map graphics layer (HR-405/406/407).
//
// Forests are stamped as deterministic clusters of small trees whose species
// (oak / pine / palm) follows the cell's biome, and whose count grows with the
// number of neighbouring forest cells so adjacent forest tiles compose into a
// continuous canopy. Settlements render a house-cluster icon scaled by size.
// Everything here is deterministic (no Math.random) so the map is stable.

import type { CellData } from "../stores/map";

export type ForestBiome = "temperate" | "boreal" | "tropical";
export type SettlementSize = "hamlet" | "village" | "town";
export type WaterKind = "river" | "lake";

export const CELL_SIZE = 24;

const FOREST_BIOMES: readonly ForestBiome[] = ["temperate", "boreal", "tropical"];

interface CellPayload {
  forest_biome?: unknown;
  water_kind?: unknown;
  settlement?: { size?: unknown } | null;
}

function payload(cell: CellData): CellPayload {
  const data = (cell as { data?: unknown }).data;
  return data && typeof data === "object" ? (data as CellPayload) : {};
}

/** A stable pseudo-random value in [0, 1) from integer inputs (FNV-1a based). */
export function hashUnit(...nums: number[]): number {
  let h = 2166136261;
  for (const n of nums) {
    h ^= n | 0;
    h = Math.imul(h, 16777619);
  }
  return ((h >>> 0) % 100000) / 100000;
}

/** Read the authored biome, else derive one from latitude + neighbours. */
export function forestBiome(
  cell: CellData,
  rowCount: number,
  neighborTerrains: string[],
): ForestBiome {
  const authored = payload(cell).forest_biome;
  if (typeof authored === "string" && (FOREST_BIOMES as string[]).includes(authored)) {
    return authored as ForestBiome;
  }
  // Fallback for worlds generated before forest_biome existed (mirror backend).
  const latitude = cell.r / Math.max(1, rowCount - 1);
  let boreal = 0;
  let tropical = 0;
  if (latitude <= 0.34) boreal += 1.5;
  else if (latitude >= 0.66) tropical += 1.5;
  for (const t of neighborTerrains) {
    if (t === "mountains") boreal += 1;
    else if (t === "hills") boreal += 0.4;
    else if (t === "swamp" || t === "water") tropical += 1;
    else if (t === "desert") tropical += 0.3;
  }
  if (boreal >= tropical && boreal >= 1) return "boreal";
  if (tropical > boreal && tropical >= 1) return "tropical";
  return "temperate";
}

export function settlementSize(cell: CellData): SettlementSize {
  const size = payload(cell).settlement?.size;
  if (size === "hamlet" || size === "town") return size;
  return "village";
}

export interface TreePlacement {
  x: number;
  y: number; // base of the trunk (canopy extends upward)
  scale: number;
}

/**
 * Deterministic tree placements for a forest cell. More forest neighbours →
 * denser cluster, so a block of forest reads as continuous canopy.
 */
export function forestTrees(
  q: number,
  r: number,
  forestNeighbors: number,
): TreePlacement[] {
  const count = Math.min(5, 2 + Math.round(forestNeighbors / 2));
  const trees: TreePlacement[] = [];
  for (let i = 0; i < count; i++) {
    const x = 4 + hashUnit(q, r, i * 3 + 1) * 16; // 4..20
    const y = 12 + hashUnit(q, r, i * 3 + 2) * 9; // 12..21 (canopy stays in cell)
    const scale = 0.85 + hashUnit(q, r, i * 3 + 3) * 0.4; // 0.85..1.25
    trees.push({ x, y, scale });
  }
  // Paint back-to-front so lower (nearer) trees overlap higher ones.
  trees.sort((a, b) => a.y - b.y);
  return trees;
}

/** Number of 8-neighbours of (q,r) whose terrain is "forest". */
export function forestNeighborCount(
  q: number,
  r: number,
  cells: Map<string, CellData>,
): number {
  let n = 0;
  for (let dq = -1; dq <= 1; dq++) {
    for (let dr = -1; dr <= 1; dr++) {
      if (dq === 0 && dr === 0) continue;
      if (cells.get(`${q + dq},${r + dr}`)?.terrain === "forest") n++;
    }
  }
  return n;
}

/** Terrain ids of the 8-neighbours of (q,r) (present cells only). */
export function neighborTerrains(
  q: number,
  r: number,
  cells: Map<string, CellData>,
): string[] {
  const out: string[] = [];
  for (let dq = -1; dq <= 1; dq++) {
    for (let dr = -1; dr <= 1; dr++) {
      if (dq === 0 && dr === 0) continue;
      const c = cells.get(`${q + dq},${r + dr}`);
      if (c) out.push(c.terrain);
    }
  }
  return out;
}

// --- Water bodies (HR-409/HR-410) ---

function isWaterAt(cells: Map<string, CellData>, q: number, r: number): boolean {
  return cells.get(`${q},${r}`)?.terrain === "water";
}

export interface OrthoFlags {
  n: boolean;
  e: boolean;
  s: boolean;
  w: boolean;
}

/** Read the authored water kind, else derive it with the 2x2-block rule. */
export function waterKind(cell: CellData, cells: Map<string, CellData>): WaterKind {
  const authored = payload(cell).water_kind;
  if (authored === "river" || authored === "lake") return authored;
  const { q, r } = cell;
  for (const ax of [q - 1, q]) {
    for (const ay of [r - 1, r]) {
      if (
        isWaterAt(cells, ax, ay) &&
        isWaterAt(cells, ax + 1, ay) &&
        isWaterAt(cells, ax, ay + 1) &&
        isWaterAt(cells, ax + 1, ay + 1)
      ) {
        return "lake";
      }
    }
  }
  return "river";
}

/** Which orthogonal neighbours of (q,r) are water (for channels/shorelines). */
export function orthoWater(
  q: number,
  r: number,
  cells: Map<string, CellData>,
): OrthoFlags {
  return {
    n: isWaterAt(cells, q, r - 1),
    e: isWaterAt(cells, q + 1, r),
    s: isWaterAt(cells, q, r + 1),
    w: isWaterAt(cells, q - 1, r),
  };
}

export const WATER_COLORS = {
  channel: "#3f76ad",
  ripple: "#5d8cc0",
  shore: "#6f6a4a",
} as const;

/** Biome canopy/trunk colours used by the renderer. */
export const BIOME_COLORS: Record<
  ForestBiome,
  { canopy: string; canopyDark: string; trunk: string }
> = {
  temperate: { canopy: "#4a7a3a", canopyDark: "#3a6230", trunk: "#5a4029" },
  boreal: { canopy: "#356b52", canopyDark: "#2a5742", trunk: "#46382a" },
  tropical: { canopy: "#5a9a3e", canopyDark: "#4a8232", trunk: "#6a5230" },
};

export interface HousePlacement {
  x: number;
  y: number; // base of the house
  scale: number;
  keep?: boolean; // draw a taller keep/tower instead of a cottage
}

/** Deterministic house cluster for a settlement icon, by size. */
const HOUSE_LAYOUTS: Record<SettlementSize, HousePlacement[]> = {
  hamlet: [{ x: 12, y: 15, scale: 1.15 }],
  village: [
    { x: 9, y: 16, scale: 1.0 },
    { x: 15, y: 14, scale: 1.15 },
  ],
  town: [
    { x: 8, y: 17, scale: 0.95 },
    { x: 13, y: 13, scale: 1.2, keep: true },
    { x: 17, y: 17, scale: 0.95 },
  ],
};

export function settlementHouses(size: SettlementSize): HousePlacement[] {
  return HOUSE_LAYOUTS[size];
}

export const SETTLEMENT_COLORS = {
  wall: "#c2a878",
  wallDark: "#9a7f54",
  roof: "#883f2a",
  keep: "#8a8a96",
} as const;

// --- Mountains (HR-741) ---

export interface PeakPlacement {
  x: number; // apex x within the cell
  baseY: number; // y of the mountain base
  w: number; // half-width of the base
  h: number; // height from base to apex
  snow: number; // snow-cap height as a fraction of h (0..1)
}

/** Deterministic mountain peaks for a cell. Tall peaks first (drawn behind). */
export function mountainPeaks(q: number, r: number): PeakPlacement[] {
  const count = 2 + Math.round(hashUnit(q, r, 1)); // 2..3
  const peaks: PeakPlacement[] = [];
  for (let i = 0; i < count; i++) {
    peaks.push({
      x: 5 + hashUnit(q, r, i * 5 + 1) * 14, // 5..19
      baseY: 20,
      w: 5 + hashUnit(q, r, i * 5 + 2) * 4, // 5..9
      h: 9 + hashUnit(q, r, i * 5 + 3) * 7, // 9..16
      snow: 0.26 + hashUnit(q, r, i * 5 + 4) * 0.18, // 0.26..0.44
    });
  }
  peaks.sort((a, b) => b.h - a.h);
  return peaks;
}

export const MOUNTAIN_COLORS = {
  rock: "#7c7c84",
  rockDark: "#5a5a62",
  snow: "#e6ecf2",
} as const;

// --- Ruins (HR-740) ---

export interface RuinFragment {
  x: number; // left edge of the pillar
  topY: number; // top of the standing fragment
  w: number; // pillar width
  lean: number; // horizontal lean of the broken top (px)
}

/** Deterministic broken-pillar fragments for a ruined cell. */
export function ruinFragments(q: number, r: number): RuinFragment[] {
  const count = 3 + Math.round(hashUnit(q, r, 2)); // 3..4
  const out: RuinFragment[] = [];
  for (let i = 0; i < count; i++) {
    out.push({
      x: 4 + i * 4.5 + hashUnit(q, r, i * 4 + 1) * 1.5,
      topY: 7 + hashUnit(q, r, i * 4 + 2) * 8, // jagged heights 7..15
      w: 2.4 + hashUnit(q, r, i * 4 + 3) * 1.4,
      lean: (hashUnit(q, r, i * 4 + 4) - 0.5) * 2.4,
    });
  }
  return out;
}

export const RUIN_COLORS = {
  stone: "#9a9486",
  stoneDark: "#6f6a5e",
  moss: "#5f7a4a",
} as const;

// --- Barren / desert (HR-742) ---

export type BarrenKind = "desert" | "wasteland";

/** Classify a barren terrain id. `wasteland` reads as cracked earth; else dunes. */
export function barrenKind(terrain: string): BarrenKind {
  return terrain === "wasteland" ? "wasteland" : "desert";
}

export interface Dune {
  y: number; // baseline y of the dune ridge
  amp: number; // crest height
}

/** Deterministic dune ridges (back-to-front) for a desert cell. */
export function desertDunes(q: number, r: number): Dune[] {
  const out: Dune[] = [];
  for (let i = 0; i < 3; i++) {
    out.push({
      y: 9 + i * 5 + hashUnit(q, r, i * 2 + 1) * 1.5,
      amp: 2.5 + hashUnit(q, r, i * 2 + 2) * 2,
    });
  }
  return out;
}

export interface Crack {
  x1: number;
  y1: number;
  x2: number;
  y2: number;
}

/** Deterministic cracked-earth polylines for a wasteland cell. */
export function wastelandCracks(q: number, r: number): Crack[] {
  const out: Crack[] = [];
  for (let i = 0; i < 4; i++) {
    const x1 = 3 + hashUnit(q, r, i * 4 + 1) * 18;
    const y1 = 4 + hashUnit(q, r, i * 4 + 2) * 16;
    const ang = hashUnit(q, r, i * 4 + 3) * Math.PI * 2;
    const len = 4 + hashUnit(q, r, i * 4 + 4) * 6;
    out.push({ x1, y1, x2: x1 + Math.cos(ang) * len, y2: y1 + Math.sin(ang) * len });
  }
  return out;
}

export const DESERT_COLORS = {
  dune: "#cda968",
  duneDark: "#a98549",
  rock: "#8a7550",
} as const;

export const WASTELAND_COLORS = {
  crack: "#3f352a",
  shrub: "#6a5f3a",
} as const;

// --- Roads (HR-743) ---

function hasRoad(cell: CellData | undefined): boolean {
  return !!cell && (cell.terrain === "road" || cell.features.includes("road"));
}

/** Whether (q,r) is a road tile (road terrain or a `road` feature). */
export function isRoadAt(cells: Map<string, CellData>, q: number, r: number): boolean {
  return hasRoad(cells.get(`${q},${r}`));
}

/** Which orthogonal neighbours of (q,r) are also road (for auto-tiled paths). */
export function orthoRoad(
  q: number,
  r: number,
  cells: Map<string, CellData>,
): OrthoFlags {
  return {
    n: isRoadAt(cells, q, r - 1),
    e: isRoadAt(cells, q + 1, r),
    s: isRoadAt(cells, q, r + 1),
    w: isRoadAt(cells, q - 1, r),
  };
}

export const ROAD_COLORS = {
  path: "#a89876",
  edge: "#7a6e52",
  dash: "#cabf9c",
} as const;

// --- Objectives + monster encounters (HR-744 / HR-745) ---

/** Features that render a quest-objective flag marker. */
export const OBJECTIVE_FEATURES: readonly string[] = ["objective"];
/** Features that render a monster-encounter (skull) marker. */
export const ENCOUNTER_FEATURES: readonly string[] = ["lair", "encounter", "monster"];

/** Whether a cell's features include any of `wanted`. */
export function hasAnyFeature(cell: CellData, wanted: readonly string[]): boolean {
  return cell.features.some((f) => wanted.includes(f));
}

export const OBJECTIVE_COLORS = {
  pole: "#c4c4cc",
  flag: "#e8c54a",
  glow: "#f5de80",
} as const;

export const ENCOUNTER_COLORS = {
  bone: "#ddd6c6",
  boneShadow: "#a89f8a",
  eye: "#7c1f1f",
} as const;

// ---------------------------------------------------------------------------
// Loot indicators (HR-787) — shared by the world map and the encounter grid.
// Items expose only a numeric `value`, so rarity tiers are value-derived.
// ---------------------------------------------------------------------------

export type LootTier = "common" | "rare" | "epic" | "legendary";

/** Inclusive lower value bound for each rarity tier (tunable). */
export const LOOT_TIER_MIN: Readonly<Record<LootTier, number>> = {
  common: 0,
  rare: 25,
  epic: 75,
  legendary: 200,
} as const;

/** Classic loot-rarity colors. */
export const LOOT_COLORS: Readonly<Record<LootTier, string>> = {
  common: "#9ca3af",
  rare: "#3b82f6",
  epic: "#a855f7",
  legendary: "#f59e0b",
} as const;

/** Gem half-size (px) per tier, so more valuable piles read larger. */
export const LOOT_TIER_RADIUS: Readonly<Record<LootTier, number>> = {
  common: 2.6,
  rare: 3.2,
  epic: 3.8,
  legendary: 4.6,
} as const;

/** Map a representative item value to a named rarity tier. */
export function lootTier(value: number): LootTier {
  if (value >= LOOT_TIER_MIN.legendary) return "legendary";
  if (value >= LOOT_TIER_MIN.epic) return "epic";
  if (value >= LOOT_TIER_MIN.rare) return "rare";
  return "common";
}

interface DeathMarkerItem {
  value?: unknown;
}
interface DeathMarker {
  items?: unknown;
  gold?: unknown;
}

/**
 * Representative loot value of a cell's `data.death_markers` — the highest
 * single item value (or gold face value) across all markers — or `null` when
 * the cell holds no recoverable loot. Items with no numeric value still count
 * (value 0 → common tier); only the absence of any item/gold returns null.
 */
export function cellLootValue(cell: CellData): number | null {
  const data = (cell as { data?: unknown }).data;
  if (!data || typeof data !== "object") return null;
  const markers = (data as { death_markers?: unknown }).death_markers;
  if (!Array.isArray(markers)) return null;
  let has = false;
  let best = 0;
  for (const m of markers) {
    if (!m || typeof m !== "object") continue;
    const marker = m as DeathMarker;
    const gold = typeof marker.gold === "number" ? marker.gold : 0;
    if (gold > 0) {
      has = true;
      best = Math.max(best, gold);
    }
    if (Array.isArray(marker.items)) {
      for (const it of marker.items) {
        if (it && typeof it === "object") {
          has = true;
          const v = (it as DeathMarkerItem).value;
          if (typeof v === "number") best = Math.max(best, v);
        }
      }
    }
  }
  return has ? best : null;
}

/** SVG diamond/gem `points` centered in a cell, sized for the given tier. */
export function lootGemPoints(tier: LootTier): string {
  const c = CELL_SIZE / 2;
  const r = LOOT_TIER_RADIUS[tier];
  return `${c},${c - r} ${c + r},${c} ${c},${c + r} ${c - r},${c}`;
}
