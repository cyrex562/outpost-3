import { describe, it, expect } from "vitest";
import {
  lootTier,
  cellLootValue,
  lootGemPoints,
  LOOT_TIER_MIN,
  LOOT_TIER_RADIUS,
  CELL_SIZE,
} from "./mapGraphics";
import type { CellData } from "../stores/map";

function cell(data?: Record<string, unknown>): CellData {
  return {
    q: 0,
    r: 0,
    terrain: "plains",
    features: [],
    explored: true,
    ...(data === undefined ? {} : { data }),
  };
}

describe("lootTier", () => {
  it("maps values to the four rarity tiers at their thresholds", () => {
    expect(lootTier(0)).toBe("common");
    expect(lootTier(LOOT_TIER_MIN.rare - 1)).toBe("common");
    expect(lootTier(LOOT_TIER_MIN.rare)).toBe("rare");
    expect(lootTier(LOOT_TIER_MIN.epic - 1)).toBe("rare");
    expect(lootTier(LOOT_TIER_MIN.epic)).toBe("epic");
    expect(lootTier(LOOT_TIER_MIN.legendary - 1)).toBe("epic");
    expect(lootTier(LOOT_TIER_MIN.legendary)).toBe("legendary");
    expect(lootTier(99999)).toBe("legendary");
  });
});

describe("cellLootValue", () => {
  it("returns null when the cell has no data", () => {
    expect(cellLootValue(cell())).toBeNull();
  });

  it("returns null when there are no death_markers", () => {
    expect(cellLootValue(cell({ settlement: { size: "town" } }))).toBeNull();
  });

  it("returns null for an empty death_markers array", () => {
    expect(cellLootValue(cell({ death_markers: [] }))).toBeNull();
  });

  it("returns the highest single item value across markers", () => {
    const v = cellLootValue(
      cell({
        death_markers: [
          { items: [{ name: "Rusty Dagger", value: 10 }, { name: "Data Chip", value: 150 }] },
          { items: [{ name: "Coin", value: 40 }] },
        ],
      }),
    );
    expect(v).toBe(150); // → epic
    expect(lootTier(v as number)).toBe("epic");
  });

  it("counts gold face value", () => {
    const v = cellLootValue(cell({ death_markers: [{ items: [], gold: 250 }] }));
    expect(v).toBe(250); // → legendary
    expect(lootTier(v as number)).toBe("legendary");
  });

  it("treats value-less items as present (value 0 → common), not absent", () => {
    const v = cellLootValue(
      cell({ death_markers: [{ items: [{ name: "Old Rag" }] }] }),
    );
    expect(v).toBe(0);
    expect(lootTier(v as number)).toBe("common");
  });

  it("ignores malformed markers", () => {
    expect(cellLootValue(cell({ death_markers: "nope" }))).toBeNull();
    expect(cellLootValue(cell({ death_markers: [null, 5, { items: null }] }))).toBeNull();
  });
});

describe("lootGemPoints", () => {
  it("produces a diamond centered in the cell, sized by tier", () => {
    const c = CELL_SIZE / 2;
    const r = LOOT_TIER_RADIUS.legendary;
    expect(lootGemPoints("legendary")).toBe(
      `${c},${c - r} ${c + r},${c} ${c},${c + r} ${c - r},${c}`,
    );
    // Larger tier → wider diamond.
    const epicWidth = LOOT_TIER_RADIUS.epic;
    expect(lootGemPoints("epic")).toContain(`${c + epicWidth},${c}`);
    expect(LOOT_TIER_RADIUS.legendary).toBeGreaterThan(LOOT_TIER_RADIUS.common);
  });
});
