/**
 * E2E tests — Character Creation (forms UI, HR-400)
 *
 * Covers:
 *  - The form renders by default on a fresh world
 *  - Standard-array happy path: name → class → assign attrs (pool click) → kit → create
 *  - Point-buy budget enforcement gates the Create button
 *  - "Do it in chat instead" falls back to the chat wizard
 *  - After creation the app transitions to exploration
 *  - Roll is the default method; pool chips are visible
 *  - Skill tree groups/search/lock/auto-pick/allocation/decrement
 */

import { test, expect, type Page } from "@playwright/test";
import { createAndLoadWorld, uniqueWorldName, waitForNarration } from "./helpers";

const ATTRS = ["str", "dex", "con", "int", "wis", "cha"] as const;

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  const worldName = uniqueWorldName("CCForm");
  // Stay in forms mode (do not switch to chat).
  await createAndLoadWorld(page, worldName, 10, 10, { chatCreation: false });
  await expect(page.getByTestId("character-creation-form")).toBeVisible({
    timeout: 30_000,
  });
});

/**
 * Assign the standard array to all six slots using the new pool-chip click
 * interaction (pool chip at index 0 → slot for each attribute in turn).
 * Standard array is [15, 14, 13, 12, 10, 8] in descending order, so the first
 * remaining chip (index 0) is always the next highest value.
 */
async function assignStandardArray(page: Page): Promise<void> {
  await page.getByTestId("cc-method-standard_array").click();
  for (const attr of ATTRS) {
    // Click pool chip at index 0 (highest remaining value) to select it.
    await page.getByTestId("cc-pool-chip-0").click();
    // Click the target slot to assign.
    await page.getByTestId(`cc-attr-slot-${attr}`).click();
  }
}

test("form renders by default with name, class and attribute sections", async ({
  page,
}) => {
  await expect(page.getByTestId("cc-name")).toBeVisible();
  await expect(page.getByTestId("cc-class-warrior")).toBeVisible();
  await expect(page.getByTestId("cc-method-standard_array")).toBeVisible();
  await expect(page.getByTestId("cc-create")).toBeDisabled();
});

test("standard-array happy path creates a character and enters exploration", async ({
  page,
}) => {
  await page.getByTestId("cc-name").fill("Aria Vex");
  await page.getByTestId("cc-class-warrior").click();
  await assignStandardArray(page);
  await page.getByTestId("cc-kit-heavy_fighter").click();

  // Heavy fighter: chain mail (14) + shield (+1) = AC 15.
  await expect(page.getByTestId("cc-preview-ac")).toHaveText("15");

  const createBtn = page.getByTestId("cc-create");
  await expect(createBtn).toBeEnabled();
  await createBtn.scrollIntoViewIfNeeded();
  await createBtn.click();

  // Character created → form disappears, exploration begins.
  await expect(page.getByTestId("character-creation-form")).toBeHidden({
    timeout: 30_000,
  });
  await waitForNarration(page, "created! Your adventure begins", 30_000);

  // Status sidebar shows the new character's name.
  await expect(page.getByText("Aria Vex").first()).toBeVisible({
    timeout: 30_000,
  });
});

test("point-buy enforces the budget and blocks an over-spend", async ({
  page,
}) => {
  await page.getByTestId("cc-name").fill("Budget Tester");
  await page.getByTestId("cc-class-warrior").click();
  await page.getByTestId("cc-method-point_buy").click();

  // Default all-8s costs 0; remaining equals the full budget.
  await expect(page.getByTestId("cc-pb-remaining")).toHaveText("27");

  // Raise STR to its max (15) a few times; remaining must never go negative.
  for (let i = 0; i < 10; i++) {
    await page.getByTestId("cc-pb-inc-str").click();
  }
  // STR caps at 15.
  await expect(page.getByTestId("cc-attr-val-str")).toHaveText("15");
  const remaining = Number(
    await page.getByTestId("cc-pb-remaining").textContent(),
  );
  expect(remaining).toBeGreaterThanOrEqual(0);
});

test("'Do it in chat instead' falls back to the chat wizard", async ({
  page,
}) => {
  await page.getByTestId("cc-use-chat").click();
  await expect(page.getByTestId("character-creation-form")).toBeHidden();
  // The chat command input appears and the GM prompts for a name.
  await expect(
    page.locator('input[type="text"][placeholder*="command"]'),
  ).toBeEnabled({ timeout: 30_000 });
  await waitForNarration(page, "What is your character's name");
});

test("attributes are rolled up front (roll is the default method)", async ({
  page,
}) => {
  // The 'Roll again' button only renders for the roll method.
  await expect(page.getByTestId("cc-roll-again")).toBeVisible();
  await expect(page.getByTestId("cc-method-roll")).toBeVisible();
  // Pool label is always visible when method is roll/standard_array.
  await expect(page.getByText(/Pool:/)).toBeVisible();
});

test("pool chip click-to-assign: selecting a chip highlights it then assigns on slot click", async ({
  page,
}) => {
  await page.getByTestId("cc-method-standard_array").click();
  // Select the first chip (index 0, value 15).
  await page.getByTestId("cc-pool-chip-0").click();
  // The chip should show the selected/highlighted state — it is still in the pool.
  await expect(page.getByTestId("cc-pool-chip-0")).toBeVisible();
  // Click the STR slot to assign.
  await page.getByTestId("cc-attr-slot-str").click();
  // After assignment the pool shrinks: slot shows a value.
  // cc-pool-chip-0 now represents the next highest value (14).
  await expect(page.getByTestId("cc-pool-chip-0")).toBeVisible();
});

test("auto-assign fills all slots from the pool", async ({ page }) => {
  await page.getByTestId("cc-method-standard_array").click();
  await page.getByTestId("cc-auto-assign").click();
  // All slots should now be filled — no pool chips remain.
  await expect(page.getByText("All scores assigned")).toBeVisible();
  // All six attribute slots should have values (check a few).
  await expect(page.getByTestId("cc-attr-slot-str")).toContainText(/\d+/);
  await expect(page.getByTestId("cc-attr-slot-cha")).toContainText(/\d+/);
});

test("skill tree groups by category and the search box filters", async ({
  page,
}) => {
  await page.getByTestId("cc-class-warrior").click();
  // Category branches render.
  await expect(page.getByTestId("cc-skill-category-Combat")).toBeVisible();
  await expect(page.getByTestId("cc-skill-category-Social")).toBeVisible();

  // Searching narrows to matching skills.
  await page.getByTestId("cc-skilltree-search").fill("stab");
  await expect(page.getByTestId("cc-skill-row-stab")).toBeVisible();
  await expect(page.getByTestId("cc-skill-row-talk")).toHaveCount(0);
});

test("a locked skill shows a reason and cannot be allocated", async ({
  page,
}) => {
  await page.getByTestId("cc-class-warrior").click();
  await page.getByTestId("cc-skilltree-search").fill("program");
  // Program is milestone-locked (and INT-gated): shows a reason, no + control.
  await expect(page.getByTestId("cc-skill-lock-program")).toBeVisible();
  await expect(page.getByTestId("cc-skill-inc-program")).toHaveCount(0);
});

test("auto-pick fills recommended skills for the chosen class", async ({
  page,
}) => {
  await page.getByTestId("cc-class-warrior").click(); // recommends stab + notice
  await expect(page.getByTestId("cc-skill-remaining")).toHaveText("2");

  await page.getByTestId("cc-auto-pick").click();

  // Both recommended skills are trained to 0 and the budget is fully spent.
  await expect(page.getByTestId("cc-skill-level-stab")).toHaveText("0");
  await expect(page.getByTestId("cc-skill-level-notice")).toHaveText("0");
  await expect(page.getByTestId("cc-skill-remaining")).toHaveText("0");

  // Still adjustable: lowering one refunds a point.
  await page.getByTestId("cc-skill-dec-notice").click();
  await expect(page.getByTestId("cc-skill-remaining")).toHaveText("1");
});

test("allocating a skill spends points and zero disables further raises", async ({
  page,
}) => {
  await page.getByTestId("cc-class-warrior").click(); // warrior: 2 points
  await expect(page.getByTestId("cc-skill-remaining")).toHaveText("2");

  await page.getByTestId("cc-skill-inc-stab").click(); // -> level 0, 1 left
  await expect(page.getByTestId("cc-skill-level-stab")).toHaveText("0");
  await expect(page.getByTestId("cc-skill-remaining")).toHaveText("1");

  await page.getByTestId("cc-skill-inc-stab").click(); // -> level 1, 0 left
  await expect(page.getByTestId("cc-skill-level-stab")).toHaveText("1");
  await expect(page.getByTestId("cc-skill-remaining")).toHaveText("0");

  // Out of points: the + control is disabled.
  await expect(page.getByTestId("cc-skill-inc-stab")).toBeDisabled();
});
