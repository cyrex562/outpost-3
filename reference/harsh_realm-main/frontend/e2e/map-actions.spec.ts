/**
 * E2E tests — Exploration Map Action Bar (HR-112)
 *
 * Verifies that a persistent action bar appears on the exploration map after
 * character creation, shows tile-appropriate buttons (at minimum Look and
 * Search for the default wilderness spawn), and that clicking Look produces
 * new narration.
 */

import { test, expect } from "@playwright/test";
import {
  completeCharacterCreation,
  createAndLoadWorld,
  uniqueWorldName,
  waitForNarrationCountToIncrease,
} from "./helpers";

test.describe("Exploration action bar (HR-112)", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    const worldName = uniqueWorldName("MapAct");
    await createAndLoadWorld(page, worldName, 10, 10);
    await completeCharacterCreation(
      page,
      "ActionTester",
      "warrior",
      ["stab", "stab"],
      "heavy_fighter",
    );
    // Wait until the exploration scene is active before asserting the bar.
    await expect(page.getByText("Exploring")).toBeVisible({ timeout: 30_000 });
  });

  test("Look button appears in the map action bar after character creation", async ({
    page,
  }) => {
    // The action bar is populated by the first `look` command issued by
    // get_initial_narration — it should be visible without any further input.
    const lookBtn = page.getByTestId("action-btn-look");
    await expect(lookBtn).toBeVisible({ timeout: 15_000 });
  });

  test("Search button appears in the map action bar after character creation", async ({
    page,
  }) => {
    const searchBtn = page.getByTestId("action-btn-search");
    await expect(searchBtn).toBeVisible({ timeout: 15_000 });
  });

  test("clicking Look button produces new narration", async ({ page }) => {
    // Wait for the action bar to be rendered first.
    const lookBtn = page.getByTestId("action-btn-look");
    await expect(lookBtn).toBeVisible({ timeout: 15_000 });

    const narrations = page.locator("span.text-amber-100");
    const countBefore = await narrations.count();

    await lookBtn.click();

    // The look command emits at least one narration paragraph.
    await waitForNarrationCountToIncrease(page, countBefore, 15_000);
  });
});
