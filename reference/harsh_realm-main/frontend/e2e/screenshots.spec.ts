/**
 * Screenshot harness — design review of key Harsh Realm screens.
 *
 * Captures full-page PNGs to docs/design/screenshots/ (repo root).
 * Each screen is its own test so one failure does not block the rest.
 *
 * Run explicitly (from frontend/):
 *   npm run screenshots
 *   SCREENSHOTS=1 npx playwright test screenshots.spec.ts
 *
 * Tests skip instantly in the normal `npm run test:e2e` run because the
 * SCREENSHOTS env-var is not set there — they appear as "skipped" rather
 * than failing or wasting time creating worlds.
 */

import { test } from "@playwright/test";
import * as fs from "node:fs";
import * as path from "node:path";

import {
  createAndLoadWorld,
  completeCharacterCreation,
  ensureNoActiveWorld,
  sendCommand,
  uniqueWorldName,
  waitForNarration,
} from "./helpers";

// ---------------------------------------------------------------------------
// Guard: skip all tests unless SCREENSHOTS=1 is set in the environment.
// This keeps the normal test:e2e run fast — each test exits immediately.
// ---------------------------------------------------------------------------

const ENABLED = process.env["SCREENSHOTS"] === "1";

// ---------------------------------------------------------------------------
// Output directory — resolves from frontend/ cwd to repo-root docs/design/screenshots/
// ---------------------------------------------------------------------------

const SCREENSHOT_DIR = path.resolve("../docs/design/screenshots");

test.beforeAll(async () => {
  if (ENABLED) {
    fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
  }
});

// ---------------------------------------------------------------------------
// World Management — the initial "no world loaded" screen
// ---------------------------------------------------------------------------

test("world-management", async ({ page }) => {
  test.skip(!ENABLED, "Run via: npm run screenshots  (SCREENSHOTS=1)");

  await page.goto("/");
  await ensureNoActiveWorld(page);

  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, "world-management.png"),
    fullPage: true,
  });
});

// ---------------------------------------------------------------------------
// Character Creation — forms UI (two-column layout, class list, attribute pool)
// ---------------------------------------------------------------------------

test("character-creation", async ({ page }) => {
  test.skip(!ENABLED, "Run via: npm run screenshots  (SCREENSHOTS=1)");

  await page.goto("/");
  await createAndLoadWorld(page, uniqueWorldName("SS-CC"), 10, 10, {
    chatCreation: false,
  });

  // Wait for the CC form to appear.
  const form = page.getByTestId("character-creation-form");
  await form.waitFor({ state: "visible", timeout: 30_000 });

  // Populate enough to show the active layout: name filled, warrior class selected.
  await page.getByTestId("cc-name").fill("Aria Vex");
  await page.getByTestId("cc-class-warrior").click();

  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, "character-creation.png"),
    fullPage: true,
  });
});

// ---------------------------------------------------------------------------
// Exploration — main play screen after character creation completes
// ---------------------------------------------------------------------------

test("exploration", async ({ page }) => {
  test.skip(!ENABLED, "Run via: npm run screenshots  (SCREENSHOTS=1)");

  await page.goto("/");
  await createAndLoadWorld(page, uniqueWorldName("SS-EX"), 10, 10);
  await completeCharacterCreation(
    page,
    "Scout",
    "warrior",
    ["stab", "stab"],
    "heavy_fighter",
  );
  await waitForNarration(page, "created! Your adventure begins");

  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, "exploration.png"),
    fullPage: true,
  });
});

// ---------------------------------------------------------------------------
// Admin — admin panel (navigate via /admin route)
// ---------------------------------------------------------------------------

test("admin", async ({ page }) => {
  test.skip(!ENABLED, "Run via: npm run screenshots  (SCREENSHOTS=1)");

  await page.goto("/");
  // Load a world so the admin panel has real backend data to display.
  await createAndLoadWorld(page, uniqueWorldName("SS-ADM"), 10, 10);

  await page.goto("/admin");
  await page
    .locator("h1", { hasText: "Admin Panel" })
    .waitFor({ state: "visible", timeout: 30_000 });

  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, "admin.png"),
    fullPage: true,
  });
});

// ---------------------------------------------------------------------------
// Combat Encounter — BEST EFFORT: walk until a combat scene appears
// ---------------------------------------------------------------------------

test("combat-encounter", async ({ page }) => {
  test.skip(!ENABLED, "Run via: npm run screenshots  (SCREENSHOTS=1)");

  await page.goto("/");
  // Slightly larger map for more terrain variety and encounter chances.
  await createAndLoadWorld(page, uniqueWorldName("SS-CMB"), 12, 12);
  await completeCharacterCreation(
    page,
    "Fighter",
    "warrior",
    ["stab", "stab"],
    "heavy_fighter",
  );

  // Walk until combat-panel or encounter-window becomes visible.
  const dirs = [
    "north", "east", "south", "west",
    "northeast", "southeast", "southwest", "northwest",
  ];
  let inCombat = false;
  const maxMoves = 30;

  for (let i = 0; i < maxMoves; i++) {
    const dir = dirs[i % dirs.length]!;
    await sendCommand(page, dir);
    // Brief pause for the WebSocket round-trip.
    await page.waitForTimeout(400);

    const combatPanel = await page
      .locator('[data-testid="combat-panel"]')
      .isVisible()
      .catch(() => false);
    const encounterWindow = await page
      .locator('[data-testid="encounter-window"]')
      .isVisible()
      .catch(() => false);

    if (combatPanel || encounterWindow) {
      inCombat = true;
      break;
    }
  }

  if (!inCombat) {
    console.log(
      "[screenshots] Could not trigger a combat encounter within " +
        String(maxMoves) +
        " moves — skipping combat-encounter screenshot.",
    );
    // Pass without producing a file rather than failing the harness.
    return;
  }

  await page.screenshot({
    path: path.join(SCREENSHOT_DIR, "combat-encounter.png"),
    fullPage: true,
  });
});
