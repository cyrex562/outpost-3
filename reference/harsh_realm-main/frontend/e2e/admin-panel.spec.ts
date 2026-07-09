/**
 * E2E tests — Admin Panel tabs (Issue #11)
 *
 * Covers:
 *  - Admin panel renders header, tab bar, and config chip
 *  - All config tabs are present in the tab bar
 *  - Skill Mappings tab: loads rows, inline-add a mapping, delete it
 *  - Difficulty Targets tab: loads rows, edit a target value and save, reset
 *  - Export Config button triggers an export and shows a message
 *
 * API calls are intercepted with page.route() so tests are deterministic and
 * do not require a running backend with a seeded world.
 */

import { test, expect, type Page, type Route } from "@playwright/test";

// ---------------------------------------------------------------------------
// Mock data
// ---------------------------------------------------------------------------

const SKILL_MAPPINGS = [
  {
    verb: "sneak",
    skill: "sneak",
    attribute: "DEX",
    base_difficulty: 8,
    opposed: false,
    description: "Move without being noticed",
    notes: "",
  },
  {
    verb: "bribe",
    skill: "talk",
    attribute: "CHA",
    base_difficulty: 10,
    opposed: true,
    description: "Offer money to sway an NPC",
    notes: "",
  },
];

const DIFFICULTY_TARGETS = [
  { name: "Easy", target: 6, description: "Straightforward tasks" },
  { name: "Normal", target: 8, description: "Standard difficulty" },
  { name: "Hard", target: 10, description: "Challenging tasks" },
];

// ---------------------------------------------------------------------------
// Shared route setup
// ---------------------------------------------------------------------------

/** Intercept all admin data reads and writes with in-memory mocks. */
async function mockAdminRoutes(page: Page): Promise<void> {
  // Worlds list (empty — no world needed for config tabs)
  await page.route("**/api/worlds", (route: Route) => {
    if (route.request().method() === "GET") {
      route.fulfill({ json: [] });
    } else {
      route.continue();
    }
  });

  // Config endpoint
  await page.route("**/api/config", (route: Route) => {
    route.fulfill({ json: { run_mode: "server", admin_mode: false } });
  });

  // Skill mappings — list and mutations
  await page.route("**/api/admin/skill-mappings", (route: Route) => {
    route.fulfill({ json: SKILL_MAPPINGS });
  });
  await page.route("**/api/admin/skill-mappings/**", (route: Route) => {
    route.fulfill({ status: 200, json: {} });
  });

  // Difficulty targets — list and mutations
  await page.route("**/api/admin/difficulty-targets", (route: Route) => {
    route.fulfill({ json: DIFFICULTY_TARGETS });
  });
  await page.route("**/api/admin/difficulty-targets/**", (route: Route) => {
    route.fulfill({ status: 200, json: {} });
  });

  // Other config tabs — return empty so they don't error
  await page.route("**/api/admin/disposition-outcomes", (route: Route) => {
    route.fulfill({ json: [] });
  });
  await page.route("**/api/admin/encounter-weights", (route: Route) => {
    route.fulfill({ json: [] });
  });
  await page.route("**/api/admin/faction-assets", (route: Route) => {
    route.fulfill({ json: [] });
  });
  await page.route("**/api/admin/xp-progression", (route: Route) => {
    route.fulfill({ json: [] });
  });

  // Export config
  await page.route("**/api/admin/export-config", (route: Route) => {
    route.fulfill({ json: { path: "/tmp/harsh_realm_config_export.json" } });
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

test.describe("Admin panel", () => {
  test.beforeEach(async ({ page }) => {
    await mockAdminRoutes(page);
    await page.goto("/admin");
    // Wait until the header is rendered
    await expect(page.locator("h1", { hasText: "Admin Panel" })).toBeVisible();
  });

  test("renders header, config chip, and tab bar", async ({ page }) => {
    await expect(page.getByTestId("admin-config-chip")).toBeVisible();
    await expect(page.getByTestId("admin-export-btn")).toBeVisible();
    await expect(page.getByTestId("admin-refresh-btn")).toBeVisible();
    await expect(page.getByTestId("admin-tab-skills")).toBeVisible();
    await expect(page.getByTestId("admin-tab-difficulties")).toBeVisible();
    await expect(page.getByTestId("admin-tab-disposition")).toBeVisible();
    await expect(page.getByTestId("admin-tab-encounters")).toBeVisible();
    await expect(page.getByTestId("admin-tab-faction")).toBeVisible();
  });

  test("config chip contains run mode and admin status", async ({ page }) => {
    const chip = page.getByTestId("admin-config-chip");
    await expect(chip).toBeVisible();
    // The chip text follows the pattern: "<run_mode> · admin on/off"
    await expect(chip).toContainText("admin");
  });

  test("Skill Mappings tab is active by default and shows rows", async ({
    page,
  }) => {
    // Skills tab is the default active tab
    await expect(page.getByTestId("admin-tab-skills")).toHaveClass(/border-blue-500|text-blue-400/);

    // Rows from mock data should be visible
    await expect(page.getByTestId("skill-row-sneak")).toBeVisible();
    await expect(page.getByTestId("skill-row-bribe")).toBeVisible();

    // Save / Reset / Delete buttons are present per row
    await expect(page.getByTestId("skill-save-sneak")).toBeVisible();
    await expect(page.getByTestId("skill-reset-sneak")).toBeVisible();
    await expect(page.getByTestId("skill-delete-sneak")).toBeVisible();
  });

  test("Skill Mappings — Add Mapping toggle shows the inline add row", async ({
    page,
  }) => {
    // The new-verb input is hidden initially
    await expect(page.getByTestId("new-skill-verb")).not.toBeVisible();

    // Click the toggle
    await page.getByTestId("skill-add-toggle").click();
    await expect(page.getByTestId("new-skill-verb")).toBeVisible();
    await expect(page.getByTestId("skill-add-confirm")).toBeVisible();

    // Clicking Cancel hides it again
    await page.getByTestId("skill-add-toggle").click();
    await expect(page.getByTestId("new-skill-verb")).not.toBeVisible();
  });

  test("Skill Mappings — add a new mapping fires PUT and reloads", async ({
    page,
  }) => {
    // Track the PUT request for the new verb
    const putRequests: string[] = [];
    await page.route("**/api/admin/skill-mappings/intimidate", (route: Route) => {
      putRequests.push(route.request().method());
      route.fulfill({ status: 200, json: {} });
    });

    // After save the store calls loadAllData — return the new mapping too
    let loadCount = 0;
    await page.route("**/api/admin/skill-mappings", (route: Route) => {
      loadCount++;
      if (loadCount === 1) {
        route.fulfill({ json: SKILL_MAPPINGS });
      } else {
        route.fulfill({
          json: [
            ...SKILL_MAPPINGS,
            {
              verb: "intimidate",
              skill: "talk",
              attribute: "STR",
              base_difficulty: 10,
              opposed: false,
              description: "Threaten an NPC",
              notes: "",
            },
          ],
        });
      }
    });
    await page.reload();
    await expect(page.locator("h1", { hasText: "Admin Panel" })).toBeVisible();

    await page.getByTestId("skill-add-toggle").click();
    await page.getByTestId("new-skill-verb").fill("intimidate");
    await page.getByTestId("skill-add-confirm").click();

    // PUT should have been called
    await expect.poll(() => putRequests).toContain("PUT");

    // After reload the new row appears
    await expect(page.getByTestId("skill-row-intimidate")).toBeVisible({ timeout: 10_000 });
  });

  test("Skill Mappings — Delete fires DELETE request", async ({ page }) => {
    const deleteRequests: string[] = [];
    await page.route("**/api/admin/skill-mappings/sneak", (route: Route) => {
      deleteRequests.push(route.request().method());
      route.fulfill({ status: 200, json: {} });
    });

    await page.getByTestId("skill-delete-sneak").click();

    await expect.poll(() => deleteRequests).toContain("DELETE");
  });

  test("Difficulty Targets tab shows rows with editable target inputs", async ({
    page,
  }) => {
    await page.getByTestId("admin-tab-difficulties").click();

    await expect(page.getByTestId("difficulty-row-Easy")).toBeVisible();
    await expect(page.getByTestId("difficulty-row-Normal")).toBeVisible();
    await expect(page.getByTestId("difficulty-row-Hard")).toBeVisible();

    // Each row has a numeric input pre-filled with the target value
    await expect(page.getByTestId("difficulty-target-Easy")).toHaveValue("6");
    await expect(page.getByTestId("difficulty-target-Normal")).toHaveValue("8");
    await expect(page.getByTestId("difficulty-target-Hard")).toHaveValue("10");
  });

  test("Difficulty Targets — Save fires PUT with updated value", async ({
    page,
  }) => {
    await page.getByTestId("admin-tab-difficulties").click();

    const putBodies: unknown[] = [];
    await page.route("**/api/admin/difficulty-targets/Easy", (route: Route) => {
      if (route.request().method() === "PUT") {
        putBodies.push(route.request().postDataJSON());
        route.fulfill({ status: 200, json: {} });
      } else {
        route.fulfill({ status: 200, json: {} });
      }
    });

    // Change the Easy target from 6 to 7
    const input = page.getByTestId("difficulty-target-Easy");
    await input.fill("7");
    await page.getByTestId("difficulty-save-Easy").click();

    await expect.poll(() => putBodies).toHaveLength(1);
    expect((putBodies[0] as { target: number }).target).toBe(7);
  });

  test("Difficulty Targets — Reset fires POST to reset endpoint", async ({
    page,
  }) => {
    await page.getByTestId("admin-tab-difficulties").click();

    const postCalls: string[] = [];
    await page.route("**/api/admin/difficulty-targets/Normal/reset", (route: Route) => {
      postCalls.push(route.request().method());
      route.fulfill({ status: 200, json: {} });
    });

    await page.getByTestId("difficulty-reset-Normal").click();

    await expect.poll(() => postCalls).toContain("POST");
  });

  test("Export Config button fires POST and shows confirmation message", async ({
    page,
  }) => {
    // The route is already mocked by beforeEach mockAdminRoutes
    await page.getByTestId("admin-export-btn").click();

    // A success message containing the export path should appear
    await expect(
      page.locator("span.text-green-400", { hasText: "Exported to" }),
    ).toBeVisible({ timeout: 5_000 });
  });

  test("Refresh button re-fetches data", async ({ page }) => {
    let callCount = 0;
    await page.route("**/api/admin/skill-mappings", (route: Route) => {
      callCount++;
      route.fulfill({ json: SKILL_MAPPINGS });
    });

    const countBefore = callCount;
    await page.getByTestId("admin-refresh-btn").click();

    await expect.poll(() => callCount).toBeGreaterThan(countBefore);
  });

  test("switching tabs hides previous content and shows new content", async ({
    page,
  }) => {
    // Skills rows visible on default tab
    await expect(page.getByTestId("skill-row-sneak")).toBeVisible();

    // Switch to Difficulties
    await page.getByTestId("admin-tab-difficulties").click();
    await expect(page.getByTestId("difficulty-row-Easy")).toBeVisible();
    // Skill rows are gone (v-if unmounts them)
    await expect(page.getByTestId("skill-row-sneak")).not.toBeVisible();

    // Switch back to Skills
    await page.getByTestId("admin-tab-skills").click();
    await expect(page.getByTestId("skill-row-sneak")).toBeVisible();
    await expect(page.getByTestId("difficulty-row-Easy")).not.toBeVisible();
  });
});
