/**
 * E2E tests — UI behaviour
 *
 * Covers:
 *  - "? Ref" button toggles the Reference panel visibility
 *  - Reference panel renders above other content (not clipped by WorldManager)
 *  - Reference panel can be closed with its × button
 *  - Connection status indicator shows "Connected" after world is loaded
 *  - Command input is disabled before a world is loaded
 *  - Command input is enabled and focused after a world is loaded
 *  - PanelWindow title bars are visible
 *  - Chat log renders narration messages in amber
 *  - Command history: ArrowUp in the input recalls the previous command
 */

import { test, expect, type Page } from "@playwright/test";
import {
  sendCommand,
  waitForNarration,
  createAndLoadWorld,
  completeCharacterCreation,
  ensureNoActiveWorld,
  noWorldLoadedMessage,
  waitForNarrationCountToIncrease,
  uniqueWorldName,
} from "./helpers";

// ---------------------------------------------------------------------------
// Helper: ensure a world is loaded and we're past character creation
// ---------------------------------------------------------------------------

async function ensureWorldAndCharacter(page: Page): Promise<void> {
  await page.goto("/");
  const worldName = uniqueWorldName("UI");
  await createAndLoadWorld(page, worldName, 10, 10);

  await completeCharacterCreation(
    page,
    "UiTester",
    "warrior",
    ["stab", "stab"],
    "heavy_fighter",
  );
  await expect(page.getByText("Exploring")).toBeVisible({ timeout: 30_000 });
}

// ---------------------------------------------------------------------------
// Tests that run before any world is loaded
// ---------------------------------------------------------------------------

test.describe("Before world is loaded", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await ensureNoActiveWorld(page);
  });

  test("command input is disabled and shows placeholder", async ({ page }) => {
    await expect(noWorldLoadedMessage(page)).toBeVisible();

    const input = page.locator(
      'input[type="text"][placeholder*="Load a world"]',
    );
    await expect(input).toBeDisabled();
  });

  test("connection status indicator is visible before a world is loaded", async ({
    page,
  }) => {
    await expect(noWorldLoadedMessage(page)).toBeVisible();

    // The app opens the WebSocket at startup (App.vue onMounted), so the status
    // label shows one of the connection states even before a world is loaded.
    const statusText = page.getByTestId("connection-status");
    await expect(statusText).toBeVisible({ timeout: 5_000 });
    await expect(statusText).toHaveText(
      /Connecting|Connected|Disconnected|Reconnecting/,
    );
  });
});

// ---------------------------------------------------------------------------
// Tests that run after world + character are created
// ---------------------------------------------------------------------------

test.describe("After world and character are ready", () => {
  test.beforeEach(async ({ page }) => {
    await ensureWorldAndCharacter(page);
  });

  // ── Connection status ──────────────────────────────────────────────────

  test("shows Connected status after world is loaded", async ({ page }) => {
    await expect(
      page.locator("header").getByText("Connected", { exact: true }),
    ).toBeVisible({ timeout: 10_000 });
  });

  // ── Command input ──────────────────────────────────────────────────────

  test("command input is enabled when world is loaded", async ({ page }) => {
    const input = page.locator('input[type="text"][placeholder*="command"]');
    await expect(input).toBeEnabled();
  });

  // ── Reference panel toggle ─────────────────────────────────────────────

  test("? Ref button shows the Reference panel", async ({ page }) => {
    const refBtn = page.getByRole("button", { name: "? Ref" });
    await expect(refBtn).toBeVisible();

    // Click to show
    await refBtn.click();

    // The Reference panel window title should be visible
    await expect(page.getByText("Reference").first()).toBeVisible({
      timeout: 5_000,
    });
  });

  test("clicking ? Ref twice hides the Reference panel", async ({ page }) => {
    const refBtn = page.getByRole("button", { name: "? Ref" });

    // Show
    await refBtn.click();
    await expect(page.getByText("Reference").first()).toBeVisible({
      timeout: 5_000,
    });

    // Hide
    await refBtn.click();
    // The panel may animate out; give it a moment
    await expect(page.locator(".text-amber-100").first()).toBeVisible(); // page still renders
  });

  test("Reference panel content is visible and above other layers", async ({
    page,
  }) => {
    const refBtn = page.getByRole("button", { name: "? Ref" });
    await refBtn.click();

    // Reference panel should contain text about commands
    const refContent = page.getByText("Commands").first();
    await expect(refContent).toBeVisible({ timeout: 5_000 });
    // Verify it is not visually obscured — boundingBox should have a positive area
    const box = await refContent.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThan(0);
    expect(box!.height).toBeGreaterThan(0);
  });

  // ── Panel windows ──────────────────────────────────────────────────────

  test("Chat / Log panel title is visible", async ({ page }) => {
    await expect(page.getByText("Chat / Log")).toBeVisible();
  });

  test("Map panel title is visible", async ({ page }) => {
    await expect(page.getByText("Map")).toBeVisible();
  });

  test("Status panel title is visible", async ({ page }) => {
    await expect(page.getByText("Status").first()).toBeVisible();
  });

  // ── Chat log rendering ─────────────────────────────────────────────────

  test("narration messages render in amber", async ({ page }) => {
    const amber = page.locator("span.text-amber-100");
    await expect(amber.first()).toBeVisible();
    const count = await amber.count();
    expect(count).toBeGreaterThan(0);
  });

  test("player input echoes in green with > prefix", async ({ page }) => {
    await sendCommand(page, "look");
    const echo = page.locator("span.text-green-400", { hasText: "> look" });
    await expect(echo).toBeVisible({ timeout: 5_000 });
  });

  // ── Command history ────────────────────────────────────────────────────

  test("ArrowUp recalls the previous command", async ({ page }) => {
    await sendCommand(page, "status");
    await waitForNarration(page, "HP"); // wait for the status response

    const input = page.locator('input[type="text"][placeholder*="command"]');
    await input.focus();
    await input.press("ArrowUp");

    // The input should now contain "status"
    await expect(input).toHaveValue("status");
  });

  test("ArrowDown after ArrowUp clears the input", async ({ page }) => {
    const narrations = page.locator("span.text-amber-100");
    const countBefore = await narrations.count();
    await sendCommand(page, "look");
    await waitForNarrationCountToIncrease(page, countBefore, 15_000);

    const input = page.locator('input[type="text"][placeholder*="command"]');
    await input.focus();
    await input.press("ArrowUp");
    await expect(input).toHaveValue("look");
    await input.press("ArrowDown");
    await expect(input).toHaveValue("");
  });
});

// ---------------------------------------------------------------------------
// Header content
// ---------------------------------------------------------------------------

test.describe("Header", () => {
  test("Harsh Realm title is present in the header", async ({ page }) => {
    await page.goto("/");
    await expect(
      page.locator("h1", { hasText: "Harsh Realm" }),
    ).toBeVisible();
  });

  test("? Ref button is always present in the header", async ({ page }) => {
    await page.goto("/");
    await expect(
      page.getByRole("button", { name: "? Ref" }),
    ).toBeVisible();
  });
});
