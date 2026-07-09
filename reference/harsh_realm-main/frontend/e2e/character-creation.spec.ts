/**
 * E2E tests — Character Creation
 *
 * Covers:
 *  - Full happy-path: name → class → roll → assign 6 attrs → skills → kit → confirm
 *  - Blank Enter advances the "roll attributes" prompt
 *  - Rejects an invalid class name
 *  - Skill point allocation (raise a skill)
 *  - Skill undo: lower / remove a skill and verify point refund
 *  - "done" with unspent points skips to kit selection
 *  - Typing "no" at confirm restarts character creation
 *  - Character name appears in the status panel after creation
 */

import { test, expect } from "@playwright/test";
import {
  sendCommand,
  pressEnterBlank,
  waitForNarration,
  waitForNewNarration,
  countNarrations,
  createAndLoadWorld,
  completeCharacterCreation,
  parseRolledScores,
  uniqueWorldName,
} from "./helpers";

// ---------------------------------------------------------------------------
// Fixture: fresh world for each test in this file
// ---------------------------------------------------------------------------

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  const worldName = uniqueWorldName("CC");
  await createAndLoadWorld(page, worldName, 10, 10);
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test("prompts for character name at start", async ({ page }) => {
  await waitForNarration(page, "What is your character's name");
});

test("rejects an invalid class name", async ({ page }) => {
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, "Tester");

  await waitForNarration(page, "Choose your class");
  await sendCommand(page, "necromancer");

  await waitForNarration(page, "not a valid class");
  // Still on class step — prompt shown again
  await waitForNarration(page, "Choose your class");
});

test("attribute scores are auto-rolled after class selection", async ({ page }) => {
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, "EnterTester");

  await waitForNarration(page, "Choose your class");
  await sendCommand(page, "warrior");

  await waitForNarration(page, "Rolled attribute scores:");
});

test("full character creation happy path — warrior", async ({ page }) => {
  await completeCharacterCreation(page, "Conan", "warrior", ["stab", "stab"], "heavy_fighter");

  // Character created message appeared
  await waitForNarration(page, "created! Your adventure begins");

  // GM transitions to exploration; asking for a look emits a hex description.
  const narrations = page.locator("span.text-amber-100");
  const countBefore = await narrations.count();
  await sendCommand(page, "look");
  await expect
    .poll(async () => narrations.count(), { timeout: 30_000 })
    .toBeGreaterThan(countBefore);
});

test("full character creation happy path — expert", async ({ page }) => {
  // Expert gets 3 skill points — allocate 3 then done
  await completeCharacterCreation(
    page,
    "Fox",
    "expert",
    ["survive", "survive", "notice"],
    "scout",
  );

  await waitForNarration(page, "created! Your adventure begins");
});

test("full character creation happy path — adventurer", async ({ page }) => {
  await completeCharacterCreation(
    page,
    "Shade",
    "adventurer",
    ["stab", "survive"],
    "versatile",
  );

  await waitForNarration(page, "created! Your adventure begins");
});

test("attribute scores are parsed and assigned without error", async ({
  page,
}) => {
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, "AttrTester");

  await waitForNarration(page, "Choose your class");
  await sendCommand(page, "warrior");

  const rollMsg = await waitForNarration(page, "Rolled attribute scores:");
  const scores = parseRolledScores(rollMsg);
  expect(scores).toHaveLength(6);
  for (const s of scores) {
    expect(s).toBeGreaterThanOrEqual(3);
    expect(s).toBeLessThanOrEqual(18);
  }

  // Assign each score and verify the "set to" confirmation
  const attrNames = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
  for (let i = 0; i < attrNames.length; i++) {
    await waitForNarration(page, `Assign a score to ${attrNames[i]}`);
    await sendCommand(page, String(scores[i]));
    await waitForNarration(page, `${attrNames[i]} set to ${scores[i]}`);
  }
});

test("skill allocation: raise a skill and see point decrease", async ({
  page,
}) => {
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, "SkillTester");
  await waitForNarration(page, "Choose your class");
  await sendCommand(page, "warrior"); // 2 skill points

  const rollMsg = await waitForNarration(page, "Rolled attribute scores:");
  const scores = parseRolledScores(rollMsg);
  const attrNames = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
  for (let i = 0; i < 6; i++) {
    await waitForNarration(page, `Assign a score to ${attrNames[i]}`);
    await sendCommand(page, String(scores[i]));
  }

  await waitForNarration(page, "Review your attributes");
  await sendCommand(page, "done");
  await waitForNarration(page, "Skill allocation");

  // Raise stab: -1 → 0 (costs 1)
  await sendCommand(page, "stab");
  await waitForNarration(page, "Stab raised to 0");
  await waitForNarration(page, "1 point(s) remaining");

  // Raise stab: 0 → 1 (costs 1, now 0 remaining)
  await sendCommand(page, "stab");
  await waitForNarration(page, "Stab raised to 1");
  await waitForNarration(page, "0 point(s) remaining");
});

test("skill undo: lower a skill refunds points", async ({ page }) => {
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, "UndoTester");
  await waitForNarration(page, "Choose your class");
  await sendCommand(page, "warrior");

  const rollMsg = await waitForNarration(page, "Rolled attribute scores:");
  const scores = parseRolledScores(rollMsg);
  const attrNames = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
  for (let i = 0; i < 6; i++) {
    await waitForNarration(page, `Assign a score to ${attrNames[i]}`);
    await sendCommand(page, String(scores[i]));
  }

  await waitForNarration(page, "Review your attributes");
  await sendCommand(page, "done");
  await waitForNarration(page, "Skill allocation");

  // Invest both points in stab
  await sendCommand(page, "stab");
  await waitForNarration(page, "1 point(s) remaining");
  await sendCommand(page, "stab");
  await waitForNarration(page, "0 point(s) remaining");

  // Now lower stab back to -1 (remove) — should refund 2 points
  await sendCommand(page, "stab -1");
  await waitForNarration(page, "removed (untrained)");
  await waitForNarration(page, "2 point(s) refunded");
  await waitForNarration(page, "2 remaining.");

  // Reallocate: notice + survive
  await sendCommand(page, "notice");
  await waitForNarration(page, "1 point(s) remaining");
  await sendCommand(page, "survive");
  await waitForNarration(page, "0 point(s) remaining");
  await sendCommand(page, "done");

  // Proceed to kit
  await waitForNarration(page, "Choose your starting kit");
});

test("typing 'done' with unspent points skips to kit selection", async ({
  page,
}) => {
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, "SkipTester");
  await waitForNarration(page, "Choose your class");
  await sendCommand(page, "warrior");

  const rollMsg = await waitForNarration(page, "Rolled attribute scores:");
  const scores = parseRolledScores(rollMsg);
  const attrNames = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
  for (let i = 0; i < 6; i++) {
    await waitForNarration(page, `Assign a score to ${attrNames[i]}`);
    await sendCommand(page, String(scores[i]));
  }

  await waitForNarration(page, "Review your attributes");
  await sendCommand(page, "done");
  await waitForNarration(page, "Skill allocation");
  // Skip without spending any points
  await sendCommand(page, "done");

  await waitForNarration(page, "Choose your starting kit");
});

test("typing 'no' at confirm restarts character creation", async ({ page }) => {
  await waitForNarration(page, "What is your character's name");
  await sendCommand(page, "RestartTester");
  await waitForNarration(page, "Choose your class");
  await sendCommand(page, "warrior");

  const rollMsg = await waitForNarration(page, "Rolled attribute scores:");
  const scores = parseRolledScores(rollMsg);
  const attrNames = ["STR", "DEX", "CON", "INT", "WIS", "CHA"];
  for (let i = 0; i < 6; i++) {
    await waitForNarration(page, `Assign a score to ${attrNames[i]}`);
    await sendCommand(page, String(scores[i]));
  }

  await waitForNarration(page, "Review your attributes");
  await sendCommand(page, "done");
  await waitForNarration(page, "Skill allocation");
  await sendCommand(page, "done");

  await waitForNarration(page, "Choose your starting kit");
  await sendCommand(page, "heavy_fighter");

  await waitForNarration(page, "Confirm? (yes / no)");
  await sendCommand(page, "no");

  // Should restart — name prompt reappears
  await waitForNarration(page, "Starting over");
  await waitForNarration(page, "What is your character's name");
});
