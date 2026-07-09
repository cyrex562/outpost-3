/**
 * E2E tests — World Management
 *
 * Covers:
 *  - WorldManager overlay appears when no world is loaded
 *  - Creating a new world dismisses the overlay and shows the world name
 *  - Tabs switch between Load and Create views
 *  - World appears in the load list after creation
 *  - Attempting to create a world with no name is a no-op
 */

import { test, expect } from "@playwright/test";
import {
  createWorld,
  ensureNoActiveWorld,
  noWorldLoadedMessage,
  uniqueWorldName,
} from "./helpers";

test.describe("World Management", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await ensureNoActiveWorld(page);
  });

  test("shows WorldManager overlay on load when no world is active", async ({
    page,
  }) => {
    await expect(noWorldLoadedMessage(page)).toBeVisible();
  });

  test("shows Load World and New World tabs", async ({ page }) => {
    await expect(page.getByText("Load World")).toBeVisible();
    await expect(page.getByText("New World")).toBeVisible();
  });

  test("creates a new world and dismisses the overlay", async ({ page }) => {
    const worldName = uniqueWorldName("WMCreate");
    await createWorld(page, worldName, 10, 10);

    // Overlay must be gone
    await expect(noWorldLoadedMessage(page)).not.toBeVisible();
  });

  test("displays the world name in the header after creation", async ({
    page,
  }) => {
    const worldName = uniqueWorldName("WMHeader");
    await createWorld(page, worldName, 10, 10);

    // World name appears as a grey label next to the title
    await expect(page.locator("header").getByText(worldName)).toBeVisible();
  });

  test("disables Generate World button when name is empty", async ({
    page,
  }) => {
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.getByRole("button", { name: "New World" }).click();

    const btn = page.getByRole("button", { name: "Generate World" });
    // Button should be disabled when the name input is empty
    await expect(btn).toBeDisabled();

    // Typing a name enables it
    await page.getByPlaceholder("e.g. Ashfall").fill("Ashfall");
    await expect(btn).toBeEnabled();

    // Clearing the name disables it again
    await page.getByPlaceholder("e.g. Ashfall").clear();
    await expect(btn).toBeDisabled();
  });

  test("created world appears in the load list", async ({ page }) => {
    const worldName = uniqueWorldName("WMList");
    await createWorld(page, worldName, 10, 10);

    await page.getByRole("button", { name: "Worlds" }).click();
    await page.getByText("Load World").click();

    await expect(
      page.getByRole("listitem").filter({ hasText: worldName.toLowerCase() }),
    ).toBeVisible();
  });
});
