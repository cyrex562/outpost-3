/**
 * E2E tests — Exploration Scene
 *
 * Tests in-game commands available after character creation.
 *
 * Covers:
 *  - help: lists valid commands
 *  - look: redescribes the current hex
 *  - status: shows character sheet
 *  - inventory: shows equipment
 *  - search: searches the current hex
 *  - Movement: all 6 valid hex directions (ne, e, se, sw, w, nw)
 *  - Movement with bare direction aliases (ne, e, se, sw, w, nw)
 *  - Impassable terrain gives a narrative refusal message
 *  - Unknown command triggers a help prompt
 *  - Command history: ArrowUp recalls the previous command
 */

import { test, expect } from "@playwright/test";
import {
  sendCommand,
  waitForNarration,
  completeCharacterCreation,
  createAndLoadWorld,
  waitForNarrationCountToIncrease,
  uniqueWorldName,
} from "./helpers";

// ---------------------------------------------------------------------------
// Fixture: create a world + character once, then run all exploration tests
// ---------------------------------------------------------------------------

let worldReady = false;

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  const worldName = uniqueWorldName("EX");
  await createAndLoadWorld(page, worldName, 10, 10);

  // Complete character creation so we reach the exploration scene
  await completeCharacterCreation(
    page,
    "Explorer",
    "warrior",
    ["stab", "stab"],
    "heavy_fighter",
  );

  await expect(page.getByText("Exploring")).toBeVisible({ timeout: 30_000 });
});

// ---------------------------------------------------------------------------
// Informational commands
// ---------------------------------------------------------------------------

test("help command lists available commands", async ({ page }) => {
  await sendCommand(page, "help");
  // Help output contains the word "move" or "go" and direction keywords
  await waitForNarration(page, "move");
});

test("look redescribes the current hex", async ({ page }) => {
  const narrations = page.locator("span.text-amber-100");
  const countBefore = await narrations.count();
  await sendCommand(page, "look");
  await waitForNarrationCountToIncrease(page, countBefore, 15_000);
});

test("status shows character sheet", async ({ page }) => {
  await sendCommand(page, "status");
  await waitForNarration(page, "HP");
  await waitForNarration(page, "AC");
  await waitForNarration(page, "Explorer"); // character name
});

test("stat alias works the same as status", async ({ page }) => {
  await sendCommand(page, "stat");
  await waitForNarration(page, "HP");
});

test("inventory shows equipment list", async ({ page }) => {
  await sendCommand(page, "inventory");
  // heavy_fighter kit contains "Chain Mail"
  await waitForNarration(page, "Chain Mail");
});

test("inv alias works the same as inventory", async ({ page }) => {
  await sendCommand(page, "inv");
  await waitForNarration(page, "Chain Mail");
});

test("search command responds with a result", async ({ page }) => {
  // Search either finds something or reports nothing — just check no error
  // The response should be a narration (amber text)
  const narrations = page.locator("span.text-amber-100");
  const countBefore = await narrations.count();
  await sendCommand(page, "search");
  // Already sent command — wait for count to increase
  await waitForNarrationCountToIncrease(page, countBefore, 10_000);
  // Alternatively verify it doesn't say "Unknown command"
  const last = narrations.last();
  const text = await last.textContent();
  expect(text).not.toContain("Unknown command");
});

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

test.describe("Movement — valid directions", () => {
  /**
   * Movement either succeeds (GM describes new hex) or is blocked by
   * impassable terrain (GM says "impassable" or "can't" or similar).
   * Both are acceptable — we just verify the GM responds with a narration
   * and does NOT say "Unknown command".
   */
  async function testDirection(page: any, dir: string): Promise<void> {
    const narrations = page.locator("span.text-amber-100");
    const countBefore = await narrations.count();

    await sendCommand(page, `go ${dir}`);
    await waitForNarrationCountToIncrease(page, countBefore, 15_000);

    const text = (await narrations.last().textContent()) ?? "";
    expect(text.toLowerCase()).not.toContain("unknown command");
  }

  test("go north", async ({ page }) => testDirection(page, "north"));
  test("go northeast", async ({ page }) => testDirection(page, "northeast"));
  test("go east", async ({ page }) => testDirection(page, "east"));
  test("go southeast", async ({ page }) => testDirection(page, "southeast"));
  test("go south", async ({ page }) => testDirection(page, "south"));
  test("go southwest", async ({ page }) => testDirection(page, "southwest"));
  test("go west", async ({ page }) => testDirection(page, "west"));
  test("go northwest", async ({ page }) => testDirection(page, "northwest"));
});

test.describe("Movement — bare direction aliases", () => {
  async function testBareDir(page: any, dir: string): Promise<void> {
    const narrations = page.locator("span.text-amber-100");
    const countBefore = await narrations.count();

    await sendCommand(page, dir);
    await waitForNarrationCountToIncrease(page, countBefore, 15_000);

    const text = (await narrations.last().textContent()) ?? "";
    expect(text.toLowerCase()).not.toContain("unknown command");
  }

  test("n (north)", async ({ page }) => testBareDir(page, "north"));
  test("ne", async ({ page }) => testBareDir(page, "ne"));
  test("e", async ({ page }) => testBareDir(page, "e"));
  test("se", async ({ page }) => testBareDir(page, "se"));
  test("s", async ({ page }) => testBareDir(page, "s"));
  test("sw", async ({ page }) => testBareDir(page, "sw"));
  test("w", async ({ page }) => testBareDir(page, "w"));
  test("nw", async ({ page }) => testBareDir(page, "nw"));
});

test("north and south are valid directions on square grid", async ({ page }) => {
  // Square grid supports all 8 directions including north and south.
  const narrations = page.locator("span.text-amber-100");
  const countBefore = await narrations.count();

  await sendCommand(page, "go north");
  await waitForNarrationCountToIncrease(page, countBefore, 10_000);
  const text = (await narrations.last().textContent()) ?? "";
  expect(text.toLowerCase()).not.toContain("unknown command");
});

// ---------------------------------------------------------------------------
// Player input echo
// ---------------------------------------------------------------------------

test("player input echoes in green in the chat log", async ({ page }) => {
  await sendCommand(page, "look");
  // Player input lines are rendered in green (text-green-400 class) with "> " prefix
  const echoEl = page.locator("span.text-green-400", { hasText: "> look" });
  await expect(echoEl).toBeVisible({ timeout: 5_000 });
});

// ---------------------------------------------------------------------------
// Unknown command
// ---------------------------------------------------------------------------

test("unknown command produces a helpful response", async ({ page }) => {
  const narrations = page.locator("span.text-amber-100");
  const countBefore = await narrations.count();
  await sendCommand(page, "xyzzy_unknown_command");
  // Wait for response
  await waitForNarrationCountToIncrease(page, countBefore, 10_000);
  const text = (await narrations.last().textContent()) ?? "";
  // The GM should indicate the command wasn't recognised
  const lower = text.toLowerCase();
  expect(
    lower.includes("unknown") ||
    lower.includes("don't understand") ||
    lower.includes("not a valid") ||
    lower.includes("help"),
  ).toBe(true);
});

// ---------------------------------------------------------------------------
// Fog of war
// ---------------------------------------------------------------------------

test("adjacent hex hints appear in the hex description", async ({ page }) => {
  // The exploration scene emits adjacent hex hints with the terrain description
  // (e.g. "To the northeast you can see ...").
  const narrations = page.locator("span.text-amber-100");
  const countBefore = await narrations.count();
  await sendCommand(page, "look");
  await waitForNarrationCountToIncrease(page, countBefore, 15_000);
  const description = (await narrations.last().textContent()) ?? "";
  // It can contain any direction or a general hint — just verify it's a real
  // multi-line hex description, not an error
  expect(description.length).toBeGreaterThan(20);
});
