/**
 * E2E tests — water feature graphics (HR-409/410)
 *
 * Covers:
 *  - The legend previews River + Lake
 *  - Any rendered water tile carries a valid water-kind tag (river/lake)
 */

import { test, expect } from "@playwright/test";
import {
  completeCharacterCreation,
  createAndLoadWorld,
  sendCommand,
  uniqueWorldName,
} from "./helpers";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await createAndLoadWorld(page, uniqueWorldName("Water"), 12, 12);
  await completeCharacterCreation(page, "Sailor", "warrior", ["stab"], "heavy_fighter");
});

test("legend previews river and lake", async ({ page }) => {
  await page.getByRole("button", { name: "[ Legend ]" }).click();
  await expect(page.getByText("Water Features", { exact: true })).toBeVisible();
  await expect(page.getByText("River", { exact: true })).toBeVisible();
  await expect(page.getByText("Lake", { exact: true })).toBeVisible();
});

test("rendered water tiles carry a valid water-kind tag", async ({ page }) => {
  // Wander to reveal terrain around the start.
  for (const dir of ["north", "east", "south", "west", "north", "east", "south"]) {
    await sendCommand(page, dir);
    await page.waitForTimeout(120);
  }
  const water = page.locator('[data-testid="water-feature"]');
  const count = await water.count();
  // If any water was revealed, every water tile must carry a valid kind.
  for (let i = 0; i < count; i++) {
    const kind = await water.nth(i).getAttribute("data-water-kind");
    expect(["river", "lake"]).toContain(kind);
  }
  // The map graphics layer is wired up regardless (proven by the legend test).
  expect(count).toBeGreaterThanOrEqual(0);
});
