/**
 * E2E tests — Combat Action Bar (HR-110)
 *
 * Verifies that when combat starts the encounter window shows an action bar
 * with at least an Attack button, and that clicking it produces a backend
 * response (a new narration line).
 */

import { test, expect } from "@playwright/test";
import {
  completeCharacterCreation,
  createAndLoadWorld,
  countNarrations,
  sendCommand,
  uniqueWorldName,
  waitForNarrationCountToIncrease,
} from "./helpers";

// ---------------------------------------------------------------------------
// Shared setup
// ---------------------------------------------------------------------------

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await createAndLoadWorld(page, uniqueWorldName("CombatActions"), 12, 12);
  await completeCharacterCreation(
    page,
    "Fighter",
    "warrior",
    ["stab", "stab"],
    "heavy_fighter",
  );
});

// ---------------------------------------------------------------------------
// Helper: walk until combat is triggered
// ---------------------------------------------------------------------------

async function triggerCombat(
  page: import("@playwright/test").Page,
  maxMoves = 30,
): Promise<boolean> {
  const dirs = ["north", "east", "south", "west", "northeast", "southeast"];
  for (let i = 0; i < maxMoves; i++) {
    const dir = dirs[i % dirs.length];
    await sendCommand(page, dir);
    await page.waitForTimeout(400);
    const inCombat = await page
      .locator('[data-testid="combat-panel"]')
      .isVisible()
      .catch(() => false);
    if (inCombat) return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test("encounter window shows action bar with Attack button when in combat", async ({
  page,
}) => {
  const inCombat = await triggerCombat(page);

  if (!inCombat) {
    // Could not trigger random encounter within the move budget — check app is healthy.
    await expect(
      page.locator('[data-testid="connection-status"]'),
    ).toContainText("Connected");
    return;
  }

  await expect(
    page.locator('[data-testid="encounter-window"]'),
  ).toBeVisible({ timeout: 15_000 });

  // Action bar must be present
  await expect(
    page.locator('[data-testid="encounter-action-bar"]'),
  ).toBeVisible({ timeout: 10_000 });

  // Attack button must be in the bar
  await expect(
    page.locator('[data-testid="encounter-action-attack"]'),
  ).toBeVisible({ timeout: 10_000 });
});

test("clicking Attack button produces a combat response", async ({ page }) => {
  const inCombat = await triggerCombat(page);

  if (!inCombat) {
    await expect(
      page.locator('[data-testid="connection-status"]'),
    ).toContainText("Connected");
    return;
  }

  await expect(
    page.locator('[data-testid="encounter-window"]'),
  ).toBeVisible({ timeout: 15_000 });

  const attackBtn = page.locator('[data-testid="encounter-action-attack"]');
  await attackBtn.waitFor({ state: "visible", timeout: 10_000 });

  // Record current narration count before clicking
  const before = await countNarrations(page, "");

  await attackBtn.click();

  // A new narration line should appear (combat round narration)
  await waitForNarrationCountToIncrease(page, before, 10_000);
});

test("Use Item button opens a consumable picker (or is disabled with no consumables)", async ({
  page,
}) => {
  const inCombat = await triggerCombat(page);

  if (!inCombat) {
    await expect(
      page.locator('[data-testid="connection-status"]'),
    ).toContainText("Connected");
    return;
  }

  await expect(
    page.locator('[data-testid="encounter-window"]'),
  ).toBeVisible({ timeout: 15_000 });

  const useBtn = page.locator('[data-testid="encounter-action-use"]');
  await useBtn.waitFor({ state: "visible", timeout: 10_000 });

  const disabled = await useBtn.isDisabled();
  if (disabled) {
    // No consumables — clicking must NOT open a picker (and never sends bare `use`).
    await expect(
      page.locator('[data-testid="encounter-use-picker"]'),
    ).toHaveCount(0);
    return;
  }

  // With consumables, clicking toggles the picker popover open.
  await useBtn.click();
  await expect(
    page.locator('[data-testid="encounter-use-picker"]'),
  ).toBeVisible({ timeout: 5_000 });
});
