/**
 * Shared helpers for Harsh Realm Playwright E2E tests.
 *
 * Usage pattern in specs:
 *   import { sendCommand, waitForNarration, createWorld, completeCharacterCreation } from './helpers';
 */

import { type Page, expect } from "@playwright/test";

// ---------------------------------------------------------------------------
// Low-level input / output helpers
// ---------------------------------------------------------------------------

/**
 * Type `text` into the command input and press Enter.
 * If `text` is empty, just presses Enter (advances blank-input prompts
 * like the "roll your stats" step).
 */
export async function sendCommand(page: Page, text: string): Promise<void> {
  const input = page.locator(
    'input[type="text"][placeholder*="command"]',
  );
  await input.waitFor({ state: "visible" });
  await input.fill(text);
  await input.press("Enter");
}

/**
 * Press Enter without typing anything — used to advance prompts that accept
 * a blank line (e.g. "Press Enter to roll stats").
 */
export async function pressEnterBlank(page: Page): Promise<void> {
  const input = page.locator(
    'input[type="text"][placeholder*="command"]',
  );
  await input.waitFor({ state: "visible" });
  await input.focus();
  // Make sure the field is truly empty before pressing Enter
  await input.fill("");
  await input.press("Enter");
}

/**
 * Wait until a narration message (amber text) containing `substring` appears
 * in the chat log. Returns the full text content of the matched element.
 *
 * Uses the *last* matching element so that repeated prompts (e.g. several
 * "Assign a score to…" messages) always resolves to the newest one.
 */
export async function waitForNarration(
  page: Page,
  substring: string,
  timeout = 20_000,
): Promise<string> {
  const el = page
    .locator("span.text-amber-100", { hasText: substring })
    .last();
  await el.waitFor({ state: "visible", timeout });
  return (await el.textContent()) ?? "";
}

/**
 * Wait until a narration message containing `substring` is NEW — i.e. the
 * count of matching messages exceeds `countBefore`.
 *
 * Useful when the same substring (e.g. "Remaining scores:") can appear
 * multiple times and you need to wait for a fresh one.
 */
export async function waitForNewNarration(
  page: Page,
  substring: string,
  countBefore: number,
  timeout = 20_000,
): Promise<string> {
  const locator = page.locator("span.text-amber-100", { hasText: substring });
  await expect(locator).toHaveCount(countBefore + 1, { timeout });
  return (await locator.last().textContent()) ?? "";
}

/** Count how many narration messages currently contain `substring`. */
export async function countNarrations(
  page: Page,
  substring: string,
): Promise<number> {
  return page.locator("span.text-amber-100", { hasText: substring }).count();
}

/**
 * Wait until the chat has more narration rows than `countBefore`.
 *
 * Use this for commands that may emit one or several narration paragraphs, or
 * whose exact prose varies with terrain/content generation.
 */
export async function waitForNarrationCountToIncrease(
  page: Page,
  countBefore: number,
  timeout = 20_000,
): Promise<void> {
  const narrations = page.locator("span.text-amber-100");
  await expect
    .poll(async () => narrations.count(), { timeout })
    .toBeGreaterThan(countBefore);
}

/** Return the text of the very last narration message in the chat log. */
export async function getLastNarration(page: Page): Promise<string> {
  const spans = page.locator("span.text-amber-100");
  const count = await spans.count();
  if (count === 0) return "";
  return (await spans.nth(count - 1).textContent()) ?? "";
}

/**
 * Character creation now defaults to the forms UI. Specs that drive the
 * turn-by-turn chat wizard must first switch into chat mode. This clicks the
 * "Do it in chat instead" button if the creation form is showing; it is a
 * best-effort no-op when the form is absent (e.g. a character already exists).
 */
export async function dismissCreationForm(page: Page): Promise<void> {
  const chatButton = page.getByTestId("cc-use-chat");
  try {
    await chatButton.waitFor({ state: "visible", timeout: 10_000 });
    await chatButton.click();
  } catch {
    // No form visible — nothing to dismiss.
  }
}

// ---------------------------------------------------------------------------
// World management helpers
// ---------------------------------------------------------------------------

/**
 * Create a new world through the WorldManager UI.
 * Assumes the WorldManager overlay is visible when called.
 *
 * @param name   World name — use a unique name per test to avoid collisions.
 * @param width  Map width (default 10 — small for speed).
 * @param height Map height (default 10).
 */
export async function createWorld(
  page: Page,
  name: string,
  width = 10,
  height = 10,
): Promise<void> {
  // Click the "New World" tab
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.getByRole("button", { name: "New World" }).click();

  // Fill world name
  await page.getByPlaceholder("e.g. Ashfall").fill(name);

  // Set dimensions (two number inputs with min=5 max=100)
  const dimInputs = page.locator('input[type="number"][min="5"][max="100"]');
  await dimInputs.first().fill(String(width));
  await dimInputs.last().fill(String(height));

  // Click Generate World
  await page.getByRole("button", { name: "Generate World" }).click();

  // Wait for stable loaded-world UI rather than overlay text, which can also
  // appear in the map panel while a world is absent.
  await expect(page.locator("header").getByText(name, { exact: true })).toBeVisible({
    timeout: 30_000,
  });
  // The forms-based creation UI is the default; switch to chat for the
  // turn-by-turn flow these helpers drive.
  await dismissCreationForm(page);
  await expect(page.locator('input[type="text"][placeholder*="command"]')).toBeEnabled({
    timeout: 30_000,
  });
}

/**
 * Create a fresh world through the API, load it as the active backend world,
 * reload the app, and wait until the UI is connected and ready for commands.
 *
 * Use this for tests whose subject is gameplay rather than the World Manager
 * form itself. It avoids a common flaky setup bug where a test creates a world
 * file but never calls `/api/worlds/load`, leaving the app on the no-world
 * overlay.
 */
export async function createAndLoadWorld(
  page: Page,
  name: string,
  width = 10,
  height = 10,
  opts: { chatCreation?: boolean } = {},
): Promise<void> {
  const { chatCreation = true } = opts;
  const created = await page.request.post("/api/worlds", {
    data: { name, width, height },
  });
  expect(created.ok()).toBe(true);
  const body = (await created.json()) as { file?: string };
  if (!body.file) {
    throw new Error(`World creation response did not include file: ${JSON.stringify(body)}`);
  }

  const loaded = await page.request.post("/api/worlds/load", {
    data: { file: body.file },
  });
  expect(loaded.ok()).toBe(true);

  await page.reload();
  await expect(
    page.locator("header").getByText("Connected", { exact: true }),
  ).toBeVisible({ timeout: 30_000 });
  // The forms-based creation UI is the default; switch to chat for the
  // turn-by-turn flow these helpers drive (unless the caller wants the form).
  if (chatCreation) {
    await dismissCreationForm(page);
    await expect(
      page.locator('input[type="text"][placeholder*="command"]'),
    ).toBeEnabled({ timeout: 30_000 });
  }
}

/**
 * Force the backend and UI into the "no active world" state.
 *
 * The app keeps the active world in server process state, so specs that need
 * the WorldManager overlay cannot rely on test order. Creating, loading, and
 * deleting a temporary world exercises the existing unload behavior in the
 * admin world-delete route without touching worlds created by other tests.
 */
export async function ensureNoActiveWorld(page: Page): Promise<void> {
  const name = uniqueWorldName("NoActiveWorld");
  const created = await page.request.post("/api/worlds", {
    data: { name, width: 5, height: 5 },
  });
  expect(created.ok()).toBe(true);
  const body = (await created.json()) as { file?: string };
  if (!body.file) {
    throw new Error(`World creation response did not include file: ${JSON.stringify(body)}`);
  }

  const loaded = await page.request.post("/api/worlds/load", {
    data: { file: body.file },
  });
  expect(loaded.ok()).toBe(true);

  const deleted = await page.request.delete(
    `/api/admin/worlds/${encodeURIComponent(body.file)}`,
  );
  expect(deleted.ok()).toBe(true);

  await page.reload();
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect(noWorldLoadedMessage(page)).toBeVisible({ timeout: 30_000 });
}

/** The primary WorldManager no-world heading, excluding map-panel filler text. */
export function noWorldLoadedMessage(page: Page) {
  return page.locator("p", { hasText: /^No world loaded$/ });
}

// ---------------------------------------------------------------------------
// Character creation helpers
// ---------------------------------------------------------------------------

/**
 * Parse the six rolled attribute scores from a narration message.
 *
 * Expected format: "Rolled attribute scores: 12  14  10  8  11  9\n…"
 */
export function parseRolledScores(text: string): number[] {
  const match = text.match(/Rolled attribute scores:\s*([\d\s]+)/);
  if (!match) return [];
  return match[1]
    .trim()
    .split(/\s+/)
    .map(Number)
    .filter((n) => !isNaN(n));
}

/**
 * Complete the full character creation flow:
 *   name → class → roll attrs → assign 6 attributes → skills → kit → confirm
 *
 * Assumes a world has been created and the GM has begun character creation
 * (i.e. the "What is your character's name?" prompt is visible).
 *
 * @param charName  Character name.
 * @param charClass Class id: "warrior" | "expert" | "adventurer".
 * @param skillCmds Skill commands to send during skill allocation (e.g.
 *                  ["stab", "stab"] spends 2 points on Stab).  Pass []
 *                  to skip allocation entirely (type "done" immediately).
 * @param kit       Kit id to choose (must match data/equipment_kits.yaml).
 */
export async function completeCharacterCreation(
  page: Page,
  charName = "Tester",
  charClass: "warrior" | "expert" | "adventurer" = "warrior",
  skillCmds: string[] = ["stab", "stab"],
  kit = "heavy_fighter",
): Promise<void> {
  // Step 1: Name
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, charName);

  // Step 2: Class
  await waitForNarration(page, "Choose your class");
  await sendCommand(page, charClass);

  // Step 3: Attributes auto-roll after class selection — parse the rolled scores
  const rollMsg = await waitForNarration(page, "Rolled attribute scores:");
  const scores = parseRolledScores(rollMsg);
  if (scores.length < 6) {
    throw new Error(
      `Expected 6 attribute scores in message, got ${scores.length}: "${rollMsg}"`,
    );
  }

  const attrNames = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
  for (let i = 0; i < attrNames.length; i++) {
    // Wait for the specific assignment prompt before sending the score
    await waitForNarration(page, `Assign a score to ${attrNames[i]}`);
    await sendCommand(page, String(scores[i]));
  }

  // Step 4b: Review attributes — accept as-is.
  await waitForNarration(page, "Review your attributes");
  await sendCommand(page, "done");

  // Step 5: Skills
  await waitForNarration(page, "Skill allocation");
  for (const cmd of skillCmds) {
    await sendCommand(page, cmd);
    // Wait for the GM's response before sending the next skill command
    await waitForNarration(page, "remaining");
  }
  await sendCommand(page, "done");

  // Step 6: Kit
  await waitForNarration(page, "Choose your starting kit");
  await sendCommand(page, kit);

  // Step 7: Confirm
  await waitForNarration(page, "Confirm? (yes / no)");
  await sendCommand(page, "yes");

  // Wait for the character-created message that signals exploration begins
  await waitForNarration(page, "created! Your adventure begins", 30_000);
}

// ---------------------------------------------------------------------------
// Test world name generator
// ---------------------------------------------------------------------------

/**
 * Generate a unique world name so each test run doesn't collide with saved
 * worlds from previous runs.  Format: "Test-<timestamp>".
 */
export function uniqueWorldName(prefix = "Test"): string {
  const randomSuffix = Math.random().toString(36).slice(2, 8);
  return `${prefix}-${Date.now()}-${randomSuffix}`;
}
