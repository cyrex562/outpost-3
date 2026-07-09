/**
 * E2E test — the player marker actually moves on the map (HR-791 regression).
 *
 * The prior movement tests only asserted the GM narrated *something* that
 * wasn't "unknown command" — they passed even when the marker never moved
 * (the bug: the movement notice was filtered out server-side and never reached
 * the client, so `mapStore.playerQ/R` never updated). This test drives a real
 * move and asserts the `player-marker` circle relocates (cx/cy change), which
 * is the user-visible behaviour that was broken.
 */

import { test, expect, type Page } from "@playwright/test";
import {
  sendCommand,
  waitForNarrationCountToIncrease,
  completeCharacterCreation,
  createAndLoadWorld,
  uniqueWorldName,
} from "./helpers";

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await createAndLoadWorld(page, uniqueWorldName("Move"), 12, 12);
  await completeCharacterCreation(page, "Walker", "warrior", ["stab"], "heavy_fighter");
});

async function markerPos(page: Page): Promise<{ cx: string; cy: string }> {
  const marker = page.locator('[data-testid="player-marker"]');
  await marker.waitFor({ state: "attached", timeout: 30_000 });
  const cx = (await marker.getAttribute("cx")) ?? "";
  const cy = (await marker.getAttribute("cy")) ?? "";
  return { cx, cy };
}

test("moving relocates the player marker on the map", async ({ page }) => {
  const start = await markerPos(page);

  // At least one of the 8 neighbours of the starting settlement is passable.
  // Try each until the marker's position changes; the generated world is not
  // guaranteed to allow a specific direction, so we probe all of them.
  const directions = ["east", "west", "north", "south", "ne", "nw", "se", "sw"];
  const narrations = page.locator("span.text-amber-100");

  let moved = false;
  for (const dir of directions) {
    const before = await narrations.count();
    await sendCommand(page, dir);
    await waitForNarrationCountToIncrease(page, before, 15_000);

    const now = await markerPos(page);
    if (now.cx !== start.cx || now.cy !== start.cy) {
      moved = true;
      break;
    }
  }

  expect(
    moved,
    "player marker never moved after trying all 8 directions — the movement " +
      "notice is not reaching the map store",
  ).toBe(true);
});
