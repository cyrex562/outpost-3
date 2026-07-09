/**
 * E2E tests — direct access to content/admin tooling (HR-404)
 *
 * Covers:
 *  - Launch screen (World Manager) offers Content Studio + Admin Panel entries
 *  - Those entries navigate to /content and /admin
 *  - Admin shows the runtime-config chip (run mode · admin on/off)
 *  - Content is usable without a pre-loaded world: world picker + gated Store
 */

import { test, expect } from "@playwright/test";
import { ensureNoActiveWorld, uniqueWorldName } from "./helpers";

test("World Manager exposes Content Studio and Admin entry points", async ({
  page,
}) => {
  await page.goto("/");
  await ensureNoActiveWorld(page);

  await expect(page.getByTestId("launch-content")).toBeVisible();
  await expect(page.getByTestId("launch-admin")).toBeVisible();
});

test("Admin entry navigates to the admin panel with a config chip", async ({
  page,
}) => {
  await page.goto("/");
  await ensureNoActiveWorld(page);

  await page.getByTestId("launch-admin").click();
  await expect(page).toHaveURL(/\/admin$/);
  const chip = page.getByTestId("admin-config-chip");
  await expect(chip).toBeVisible();
  // Server mode (Playwright launches `uvicorn` without the desktop env override).
  await expect(chip).toContainText("server");
  await expect(chip).toContainText("admin");
});

test("Content entry works without a loaded world and gates Store", async ({
  page,
}) => {
  await page.goto("/");
  await ensureNoActiveWorld(page);

  // A world exists in the list but is not the active world.
  const name = uniqueWorldName("HR404");
  const created = await page.request.post("/api/worlds", {
    data: { name, width: 8, height: 8 },
  });
  expect(created.ok()).toBe(true);
  const file = ((await created.json()) as { file: string }).file;

  await page.getByTestId("launch-content").click();
  await expect(page).toHaveURL(/\/content$/);

  // No active world yet: Store is disabled and the hint is shown.
  await expect(page.getByTestId("content-no-world")).toBeVisible();
  await expect(page.getByTestId("store-button")).toBeDisabled();

  // Compile works without a world.
  await page.getByTestId("yaml-editor").fill("");
  await expect(page.getByTestId("compile-button")).toBeEnabled();

  // Load a world from inside Content → Store becomes enabled.
  await page.getByTestId("content-world-select").selectOption(file);
  await expect(page.getByTestId("content-active-world")).toBeVisible({
    timeout: 30_000,
  });
  await expect(page.getByTestId("store-button")).toBeEnabled();
});
