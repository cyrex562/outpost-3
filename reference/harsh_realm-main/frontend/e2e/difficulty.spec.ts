/**
 * E2E — Adjustable Difficulty (HR-107, issue #107).
 *
 * Drives the real backend: creates + loads a world (whose difficulty_settings
 * are seeded to Normal/grade 3), opens the player-facing difficulty screen,
 * asserts the generic sliders render, moves one, and confirms the new grade
 * persists across a full page reload (per-world SQLite round-trip).
 */
import { test, expect } from "@playwright/test";
import { createAndLoadWorld, uniqueWorldName } from "./helpers";

test.describe("Difficulty settings", () => {
  test("sliders render at Normal, adjust, and persist across reload", async ({
    page,
  }) => {
    await page.goto("/");
    await createAndLoadWorld(page, uniqueWorldName("Difficulty"), 10, 10, {
      chatCreation: false,
    });

    // Open the difficulty screen from the header link.
    await page.getByTestId("difficulty-nav").click();
    await expect(page.getByTestId("difficulty-view")).toBeVisible();

    // All seven dimensions render as sliders.
    const sliders = page.locator('[data-testid^="difficulty-slider-"]');
    await expect(sliders).toHaveCount(7);

    // enemy_hp starts at Normal (grade 3, ×1).
    const enemyHp = page.getByTestId("difficulty-slider-enemy_hp");
    await expect(enemyHp).toHaveValue("3");
    await expect(page.getByTestId("difficulty-effect-enemy_hp")).toHaveText("×1");

    // Move it to the hardest grade (6 → ×2); the effect reflects the server value.
    await enemyHp.fill("6");
    await expect(page.getByTestId("difficulty-effect-enemy_hp")).toHaveText("×2");

    // Reload the page — the difficulty screen re-fetches from the world DB.
    await page.reload();
    await expect(page.getByTestId("difficulty-view")).toBeVisible();
    await expect(page.getByTestId("difficulty-slider-enemy_hp")).toHaveValue("6");
    await expect(page.getByTestId("difficulty-effect-enemy_hp")).toHaveText("×2");
  });

  test("Back to Game link returns to the game view", async ({ page }) => {
    await page.goto("/");
    await createAndLoadWorld(page, uniqueWorldName("Difficulty"), 10, 10, {
      chatCreation: false,
    });
    await page.getByTestId("difficulty-nav").click();
    await expect(page.getByTestId("difficulty-view")).toBeVisible();
    await page.getByTestId("difficulty-back").click();
    await expect(page.getByTestId("difficulty-view")).toHaveCount(0);
    await expect(page.getByTestId("difficulty-nav")).toBeVisible();
  });
});
