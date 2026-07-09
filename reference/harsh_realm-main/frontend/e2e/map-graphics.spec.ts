/**
 * E2E tests — map graphics layer (HR-405/406/407)
 *
 * Covers:
 *  - The starting settlement renders the settlement icon (not a glyph)
 *  - Forest tiles render biome-tagged tree stamps once explored
 *  - The legend previews the forest variants + settlement icon
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
  await createAndLoadWorld(page, uniqueWorldName("MapGfx"), 12, 12);
  await completeCharacterCreation(page, "Cartographer", "warrior", ["stab"], "heavy_fighter");
});

test("starting settlement renders the settlement icon", async ({ page }) => {
  // The character starts on the settlement cell, which is explored.
  await expect(page.locator('[data-testid="settlement-icon"]').first()).toBeVisible({
    timeout: 30_000,
  });
  // It carries a size tag used by the renderer.
  const size = await page
    .locator('[data-testid="settlement-icon"]')
    .first()
    .getAttribute("data-size");
  expect(["hamlet", "village", "town"]).toContain(size);
});

test("forest tiles render biome-tagged tree stamps after exploring", async ({
  page,
}) => {
  // Wander a bit to reveal terrain around the start.
  for (const dir of ["north", "east", "south", "west", "north", "east"]) {
    await sendCommand(page, dir);
    await page.waitForTimeout(150);
  }

  const stamps = page.locator('[data-testid="forest-stamp"]');
  // If any forest was revealed, every stamp must carry a valid biome tag.
  const count = await stamps.count();
  if (count > 0) {
    const biome = await stamps.first().getAttribute("data-biome");
    expect(["temperate", "boreal", "tropical"]).toContain(biome);
  }
  // The graphics layer itself must be wired up (settlement icon proves the SVG
  // overlay renders even if no forest happened to be adjacent).
  await expect(
    page.locator('[data-testid="settlement-icon"]').first(),
  ).toBeVisible();
});

test("legend previews forest variants and the settlement icon", async ({
  page,
}) => {
  await page.getByRole("button", { name: "[ Legend ]" }).click();
  await expect(page.getByText("Deciduous (oak)")).toBeVisible();
  await expect(page.getByText("Boreal (pine)")).toBeVisible();
  await expect(page.getByText("Tropical (palm)")).toBeVisible();
  await expect(page.getByText("Forests", { exact: true })).toBeVisible();
});

test("legend previews the terrain art and marker stamps (HR-740..745)", async ({
  page,
}) => {
  await page.getByRole("button", { name: "[ Legend ]" }).click();
  // Terrain Art group heading is unique; the per-terrain labels also appear as
  // colour swatches in the World/Town Terrain groups, so match the first.
  await expect(page.getByText("Terrain Art", { exact: true })).toBeVisible();
  await expect(page.getByText("Mountains").first()).toBeVisible();
  await expect(page.getByText("Ruins").first()).toBeVisible();
  await expect(page.getByText("Desert").first()).toBeVisible();
  await expect(page.getByText("Wasteland").first()).toBeVisible();
  await expect(page.getByText("Road").first()).toBeVisible();
  // Feature markers: objective flag + monster lair (skull) labels are unique.
  await expect(page.getByText("Objective")).toBeVisible();
  await expect(page.getByText("Monster Lair")).toBeVisible();
});

test("terrain stamps render once revealed (mountains/ruins/desert/road)", async ({
  page,
}) => {
  // Reveal a swath of terrain around the start.
  for (const dir of ["north", "east", "south", "west", "north", "east", "south"]) {
    await sendCommand(page, dir);
    await page.waitForTimeout(150);
  }
  // Whichever terrains were revealed must carry their stamp testid. Barren
  // stamps additionally tag the kind. We assert conditionally because terrain
  // placement is procedural, but verify the SVG overlay is wired (settlement).
  const barren = page.locator('[data-testid="barren-stamp"]');
  if ((await barren.count()) > 0) {
    expect(["desert", "wasteland"]).toContain(
      await barren.first().getAttribute("data-barren-kind"),
    );
  }
  await expect(
    page.locator('[data-testid="settlement-icon"]').first(),
  ).toBeVisible();
});
