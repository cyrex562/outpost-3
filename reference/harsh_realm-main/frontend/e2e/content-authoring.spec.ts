/**
 * E2E tests — Content authoring view (R5-C)
 *
 * Covers:
 *  - the view renders with an editor and a reference panel toggle
 *  - compiling valid YAML shows the success line with a record count
 *  - compiling malformed YAML shows a syntax-error diagnostic
 *  - the reference panel toggles and loads the schema-derived key reference
 *  - "Load template" populates the editor
 *
 * The content endpoints are world-independent, so no world setup is needed.
 */

import { test, expect } from "@playwright/test";

const VALID_YAML = [
  "status_effect:",
  "  id: bleeding",
  "  name: Bleeding",
  "  stacking: stack",
].join("\n");

const BAD_DICE_YAML = [
  "creature:",
  "  id: hound",
  "  name: Hound",
  "  hd: 1",
  "  ac: 12",
  "  attack_bonus: 1",
  "  damage: 1dd6",
].join("\n");

test.beforeEach(async ({ page }) => {
  await page.goto("/content");
});

test("renders the editor and toolbar", async ({ page }) => {
  await expect(page.getByTestId("yaml-editor")).toBeVisible();
  await expect(page.getByTestId("compile-button")).toBeVisible();
  await expect(page.getByTestId("toggle-reference")).toBeVisible();
});

test("renders the migration + store controls", async ({ page }) => {
  await expect(page.getByTestId("legacy-category")).toBeVisible();
  await expect(page.getByTestId("migrate-button")).toBeVisible();
  await expect(page.getByTestId("store-button")).toBeVisible();
  // Migrate is disabled until a category is chosen.
  await expect(page.getByTestId("migrate-button")).toBeDisabled();
});

test("stored-record browser toggles", async ({ page }) => {
  await expect(page.getByTestId("record-browser")).toHaveCount(0);
  await page.getByTestId("toggle-browser").click();
  const browser = page.getByTestId("record-browser");
  await expect(browser).toBeVisible();
  await expect(browser).toContainText("Stored records");
});

test("compiling valid YAML reports success", async ({ page }) => {
  await page.getByTestId("yaml-editor").fill(VALID_YAML);
  await page.getByTestId("compile-button").click();
  await expect(page.getByTestId("compile-success")).toContainText("Compiled 1 record");
});

test("compiling malformed YAML shows a syntax error", async ({ page }) => {
  await page.getByTestId("yaml-editor").fill(BAD_DICE_YAML);
  await page.getByTestId("compile-button").click();
  const diagnostics = page.getByTestId("diagnostics");
  await expect(diagnostics).toBeVisible();
  await expect(diagnostics).toContainText("invalid dice");
});

test("reference panel toggles and loads the key reference", async ({ page }) => {
  await expect(page.getByTestId("reference-panel")).toHaveCount(0);
  await page.getByTestId("toggle-reference").click();
  const panel = page.getByTestId("reference-panel");
  await expect(panel).toBeVisible();
  await expect(panel).toContainText("Content Key Reference");
});

test("load template populates the editor", async ({ page }) => {
  await page.getByTestId("load-template").click();
  await expect(page.getByTestId("yaml-editor")).toHaveValue(/creature:/);
});
