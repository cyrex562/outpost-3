/**
 * Pure-TypeScript grid model for HR-795 (renderer-agnostic world model, Layer 2).
 *
 * Zero Vue imports — safe to use outside a browser context, in unit tests
 * without jsdom, and in future non-Vue renderers (WebGL, native).
 */

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

export type Coord = { q: number; r: number };

/**
 * Returns the canonical map key for a grid coordinate.
 */
export function coordKey(q: number, r: number): string {
  return `${q},${r}`;
}

/**
 * A single grid cell.
 *
 * Named fields mirror the backend `CellPreview` / `CellData` shapes.  The
 * index signature `[k: string]: unknown` preserves additional backend fields
 * without dropping them (e.g. `data` on world cells, `settlement_id` on town
 * cells).
 */
export interface Cell {
  q: number;
  r: number;
  terrain: string;
  features: string[];
  /** `false`/`0` = fog of war, `true`/`1` = explored, `2` = seen-but-not-entered. */
  explored: number | boolean;
  [k: string]: unknown;
}

/**
 * An entity (PC, enemy, NPC) positioned on a grid.
 */
export interface GridEntity {
  id: string;
  kind: string;
  q: number;
  r: number;
  name?: string;
  hp?: number;
  maxHp?: number;
  alive?: boolean;
}

export type GridKind = "world" | "town" | "encounter";

/**
 * A unified grid that represents one of the three map contexts (world, town,
 * encounter).  All three use the same shape so renderers and reducers share a
 * single code path.
 */
export interface Grid {
  kind: GridKind;
  width: number;
  height: number;
  /** Indexed by `coordKey(q, r)`. */
  cells: Map<string, Cell>;
  entities: GridEntity[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Creates an empty grid for the given context. */
export function emptyGrid(kind: GridKind): Grid {
  return {
    kind,
    width: 0,
    height: 0,
    cells: new Map<string, Cell>(),
    entities: [],
  };
}

/** Inserts or replaces the cell at `(cell.q, cell.r)`. */
export function setCell(grid: Grid, cell: Cell): void {
  grid.cells.set(coordKey(cell.q, cell.r), cell);
}

/** Returns the cell at `(q, r)`, or `undefined` if not present. */
export function getCell(grid: Grid, q: number, r: number): Cell | undefined {
  return grid.cells.get(coordKey(q, r));
}
