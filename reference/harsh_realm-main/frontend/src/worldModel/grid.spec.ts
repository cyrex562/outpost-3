/**
 * Unit tests for grid.ts — pure TS, no Vue, no jsdom required.
 */
import { describe, it, expect } from "vitest";
import {
  coordKey,
  emptyGrid,
  setCell,
  getCell,
  type Cell,
  type Grid,
} from "./grid";

describe("coordKey", () => {
  it("formats as 'q,r'", () => {
    expect(coordKey(3, 7)).toBe("3,7");
    expect(coordKey(0, 0)).toBe("0,0");
    expect(coordKey(-1, 5)).toBe("-1,5");
    expect(coordKey(10, 20)).toBe("10,20");
  });
});

describe("emptyGrid", () => {
  it("creates a grid with the requested kind", () => {
    const g: Grid = emptyGrid("world");
    expect(g.kind).toBe("world");
  });

  it("starts with zero dimensions and empty collections", () => {
    const g = emptyGrid("encounter");
    expect(g.width).toBe(0);
    expect(g.height).toBe(0);
    expect(g.cells.size).toBe(0);
    expect(g.entities).toEqual([]);
  });

  it("creates independent instances", () => {
    const a = emptyGrid("town");
    const b = emptyGrid("town");
    expect(a).not.toBe(b);
    expect(a.cells).not.toBe(b.cells);
    expect(a.entities).not.toBe(b.entities);
  });
});

describe("setCell / getCell", () => {
  it("stores and retrieves a cell by coordinate", () => {
    const g = emptyGrid("world");
    const cell: Cell = {
      q: 2,
      r: 3,
      terrain: "forest",
      features: [],
      explored: 1,
    };
    setCell(g, cell);
    const retrieved = getCell(g, 2, 3);
    expect(retrieved).toBeDefined();
    expect(retrieved?.q).toBe(2);
    expect(retrieved?.r).toBe(3);
    expect(retrieved?.terrain).toBe("forest");
    expect(retrieved?.explored).toBe(1);
  });

  it("replaces an existing cell at the same coordinate", () => {
    const g = emptyGrid("world");
    const cell1: Cell = {
      q: 0,
      r: 0,
      terrain: "plains",
      features: [],
      explored: 1,
    };
    const cell2: Cell = {
      q: 0,
      r: 0,
      terrain: "mountain",
      features: ["ruins"],
      explored: 2,
    };
    setCell(g, cell1);
    setCell(g, cell2);
    const result = getCell(g, 0, 0);
    expect(result?.terrain).toBe("mountain");
    expect(result?.features).toEqual(["ruins"]);
    expect(result?.explored).toBe(2);
    expect(g.cells.size).toBe(1);
  });

  it("returns undefined for a coordinate that was never set", () => {
    const g = emptyGrid("encounter");
    expect(getCell(g, 99, 99)).toBeUndefined();
  });

  it("correctly indexes distinct coordinates", () => {
    const g = emptyGrid("world");
    const c1: Cell = { q: 1, r: 0, terrain: "a", features: [], explored: 1 };
    const c2: Cell = { q: 0, r: 1, terrain: "b", features: [], explored: 1 };
    setCell(g, c1);
    setCell(g, c2);
    expect(g.cells.size).toBe(2);
    expect(getCell(g, 1, 0)?.terrain).toBe("a");
    expect(getCell(g, 0, 1)?.terrain).toBe("b");
  });

  it("preserves extra index-signature fields", () => {
    const g = emptyGrid("world");
    const cell: Cell = {
      q: 5,
      r: 5,
      terrain: "plains",
      features: [],
      explored: 1,
      settlement_id: "town-42",
    };
    setCell(g, cell);
    expect(getCell(g, 5, 5)?.settlement_id).toBe("town-42");
  });
});
