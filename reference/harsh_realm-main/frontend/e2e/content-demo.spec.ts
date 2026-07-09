/**
 * E2E test — Content demo harness.
 *
 * Authors an envenomed-weapon pack (item + trait + status_effect + creature),
 * runs the in-app combat demo, and asserts the IR triggers fired: the transcript
 * shows the poison annotation and the player wins. The demo endpoint is
 * world-independent (its own ephemeral world), so no world setup is needed.
 */

import { test, expect } from "@playwright/test";

const DEMO_YAML = [
  "item:",
  "  id: envenomed_dagger",
  "  name: Envenomed Dagger",
  "  damage: 1d4",
  "  tags: [weapon, melee]",
  "  grants_traits: [envenom_on_hit]",
  "---",
  "trait:",
  "  id: envenom_on_hit",
  "  name: Envenom on Hit",
  "  category: weapon_quality",
  "  triggers:",
  "    - id: envenom_strike",
  "      on: combat.attack",
  '      when: "event.hit == true"',
  "      do:",
  "        - kind: apply_status",
  "          params: { entity_id: target, status_id: poisoned, duration_ticks: 3 }",
  "        - kind: log",
  '          params: { message: "envenom: target is poisoned" }',
  "---",
  "status_effect:",
  "  id: poisoned",
  "  name: Poisoned",
  "  default_duration_ticks: 3",
  "  provides_tags: [poisoned]",
  "  stacking: replace",
  "  triggers:",
  "    - id: poison_tick",
  "      on: combat.attack",
  "      when: \"has_tag(self, 'poisoned')\"",
  "      do:",
  "        - kind: change_resource",
  "          params: { resource: hp, delta: -2 }",
  "        - kind: log",
  '          params: { message: "poison bites" }',
  "---",
  "creature:",
  "  id: scrip_hound",
  "  name: Scrip-Hound",
  "  hd: 2",
  "  hp_per_hd: 4",
  "  ac: 11",
  "  attack_bonus: 1",
  "  damage: 1d4",
].join("\n");

test.beforeEach(async ({ page }) => {
  await page.goto("/content");
});

test("renders the demo controls", async ({ page }) => {
  await expect(page.getByTestId("demo-creature-id")).toBeVisible();
  await expect(page.getByTestId("demo-combat-run")).toBeVisible();
  // Disabled until a creature id is entered.
  await expect(page.getByTestId("demo-combat-run")).toBeDisabled();
});

test("runs an authored pack and shows IR triggers firing", async ({ page }) => {
  await page.getByTestId("yaml-editor").fill(DEMO_YAML);
  await page.getByTestId("demo-creature-id").fill("scrip_hound");
  await page.getByTestId("demo-weapon-id").fill("envenomed_dagger");
  await page.getByTestId("demo-combat-run").click();

  const transcript = page.getByTestId("demo-transcript");
  await expect(transcript).toBeVisible();
  // The envenom/poison annotations prove the IR drove combat.
  await expect(transcript).toContainText("poison");
  await expect(transcript).toContainText("winner: player");
});
