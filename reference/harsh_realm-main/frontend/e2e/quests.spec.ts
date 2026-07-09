/**
 * E2E — Quest & Plotline Service (HR-19, issue #19).
 *
 * Drives the full pipeline: the `quests accept <id>` command → quest.accept_requested
 * → QuestService → quest.accepted → reducer → refetch-quests → loadQuests →
 * StatusSidebar badge. The starter quest `clear_the_crawlers` has no prerequisite,
 * so acceptance is deterministic.
 */
import { test, expect } from "@playwright/test";
import {
  createAndLoadWorld,
  completeCharacterCreation,
  sendCommand,
  uniqueWorldName,
} from "./helpers";

test.describe("Quests", () => {
  test("accepting a quest shows the active-quest badge", async ({ page }) => {
    await page.goto("/");
    await createAndLoadWorld(page, uniqueWorldName("Quests"), 10, 10);
    await completeCharacterCreation(page, "Questor", "warrior", ["stab"], "heavy_fighter");

    // No badge before any quest is accepted.
    await expect(page.getByTestId("quest-badge")).toHaveCount(0);

    await sendCommand(page, "quests accept clear_the_crawlers");

    // The sidebar badge appears with a count of 1 (refetched from the server).
    const badge = page.getByTestId("quest-badge");
    await expect(badge).toBeVisible({ timeout: 15_000 });
    await expect(badge).toContainText("1");

    // The `quests` list command shows the accepted quest.
    await sendCommand(page, "quests");
    await expect(page.getByText("Clear the Crawlers")).toBeVisible({ timeout: 15_000 });
  });
});
