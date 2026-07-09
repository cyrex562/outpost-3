/**
 * E2E — world-map loot indicators (HR-787, issue #100).
 *
 * The only on-map loot is `cell.data.death_markers` (a player's death drop).
 * Rather than force a death, we mock `GET /api/world/map` with cells carrying
 * death-markers of known value and assert the rarity-tiered indicator renders
 * end-to-end (hydration → lootRenders → SVG marker) with the right tier.
 */
import { test, expect } from "@playwright/test";
import { createAndLoadWorld, uniqueWorldName } from "./helpers";

const MOCK_MAP = {
  width: 6,
  height: 6,
  cells: [
    // An epic drop (top item value 150 → epic).
    {
      q: 1,
      r: 1,
      terrain: "plains",
      features: [],
      explored: true,
      data: {
        death_markers: [
          { items: [{ name: "Rusty Dagger", value: 10 }, { name: "Data Chip", value: 150 }] },
        ],
      },
    },
    // A legendary drop (gold 300 → legendary).
    {
      q: 3,
      r: 2,
      terrain: "plains",
      features: [],
      explored: true,
      data: { death_markers: [{ items: [], gold: 300 }] },
    },
    // A common drop (value-less item → common).
    {
      q: 2,
      r: 4,
      terrain: "plains",
      features: [],
      explored: true,
      data: { death_markers: [{ items: [{ name: "Old Rag" }] }] },
    },
    // A plain explored cell with no loot → no indicator.
    { q: 0, r: 0, terrain: "plains", features: [], explored: true, data: {} },
  ],
};

test.describe("World-map loot indicators", () => {
  test("renders rarity-tiered indicators from death-marker loot", async ({ page }) => {
    await page.route("**/api/worlds/current/map", (route) =>
      route.fulfill({ json: MOCK_MAP }),
    );

    await page.goto("/");
    await createAndLoadWorld(page, uniqueWorldName("Loot"), 6, 6, {
      chatCreation: false,
    });

    // Each loot cell gets a tiered indicator; the tier attribute reflects value.
    await expect(page.locator('[data-testid="loot-indicator-1-1"]')).toHaveAttribute(
      "data-loot-tier",
      "epic",
    );
    await expect(page.locator('[data-testid="loot-indicator-3-2"]')).toHaveAttribute(
      "data-loot-tier",
      "legendary",
    );
    await expect(page.locator('[data-testid="loot-indicator-2-4"]')).toHaveAttribute(
      "data-loot-tier",
      "common",
    );

    // A loot-free explored cell has no indicator.
    await expect(page.locator('[data-testid="loot-indicator-0-0"]')).toHaveCount(0);
  });
});
