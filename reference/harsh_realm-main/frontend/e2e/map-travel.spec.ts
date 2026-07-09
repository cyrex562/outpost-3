/**
 * E2E tests — right-click map travel (HR-408)
 *
 * Covers:
 *  - Right-clicking a map cell opens a context menu (Move / Look / Use skill)
 *  - The Skills submenu lists travel skills
 *  - Choosing an action sends the corresponding `travel` command
 *  - Cancel / click-away dismisses the menu
 */

import { test, expect, type Page } from "@playwright/test";
import {
  completeCharacterCreation,
  createAndLoadWorld,
  uniqueWorldName,
} from "./helpers";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await createAndLoadWorld(page, uniqueWorldName("Travel"), 12, 12);
  await completeCharacterCreation(page, "Pathfinder", "warrior", ["stab"], "heavy_fighter");
});

async function rightClickPlayerCell(page: Page): Promise<void> {
  // Right-click at the player's marker — a guaranteed in-bounds, explored cell.
  const marker = page.locator('[data-testid="player-marker"]');
  await marker.waitFor({ state: "attached", timeout: 30_000 });
  const box = await marker.boundingBox();
  if (!box) throw new Error("player marker has no bounding box");
  await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2, {
    button: "right",
  });
}

test("right-click opens the cell context menu", async ({ page }) => {
  await rightClickPlayerCell(page);
  await expect(page.getByTestId("cell-menu")).toBeVisible();
  await expect(page.getByTestId("cell-menu-move")).toBeVisible();
  await expect(page.getByTestId("cell-menu-look")).toBeVisible();
  await expect(page.getByTestId("cell-menu-skills")).toBeVisible();
});

test("skills submenu lists travel skills", async ({ page }) => {
  await rightClickPlayerCell(page);
  await page.getByTestId("cell-menu-skills").click();
  await expect(page.getByTestId("cell-menu-skill-notice")).toBeVisible();
  await expect(page.getByTestId("cell-menu-skill-survive")).toBeVisible();
  await expect(page.getByTestId("cell-menu-skill-know")).toBeVisible();
});

test("choosing 'Move here' sends a travel command", async ({ page }) => {
  await rightClickPlayerCell(page);
  await page.getByTestId("cell-menu-move").click();
  // The menu closes and the travel command is echoed into the chat log.
  await expect(page.getByTestId("cell-menu")).toBeHidden();
  await expect(page.getByText(/travel \d+ \d+/).first()).toBeVisible({
    timeout: 15_000,
  });
});

test("choosing a skill sends a travel-skill command", async ({ page }) => {
  await rightClickPlayerCell(page);
  await page.getByTestId("cell-menu-skills").click();
  await page.getByTestId("cell-menu-skill-notice").click();
  await expect(page.getByText(/travel \d+ \d+ skill notice/).first()).toBeVisible({
    timeout: 15_000,
  });
});

test("Cancel dismisses the menu", async ({ page }) => {
  await rightClickPlayerCell(page);
  await expect(page.getByTestId("cell-menu")).toBeVisible();
  await page.getByTestId("cell-menu-cancel").click();
  await expect(page.getByTestId("cell-menu")).toBeHidden();
});
