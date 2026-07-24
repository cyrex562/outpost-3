import { test, expect } from '@playwright/test'

/**
 * Drives the full colony-founding flow against a live `outpost_web`
 * backend (issue #220) — new game over the WebSocket, then the
 * FoundColonyWizardView's four steps against the new browser-mode REST
 * routes (`query_routes.rs`), ending with a real `found_colony_at_site`
 * command dispatched through `POST /api/command`.
 *
 * Unlike `app-shell.spec.ts`, this suite requires the `outpost_web`
 * backend `playwright.config.ts` starts alongside the preview server.
 */

test('new game + found colony wizard against a live backend', async ({ page }) => {
  await page.goto('/')

  // ── Main menu → New Game panel → Start (browser mode redirects to /new-game) ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()

  // ── New Game view (WS `new_game` flow against the shared engine) ──
  await expect(page.getByTestId('start-game-btn')).toBeVisible()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()

  // Navigates to /colony once the command is sent.
  await expect(page).toHaveURL(/#\/colony/)

  // ── Navigate to the star map, then launch the founding wizard ──
  await page.getByRole('link', { name: 'System' }).click()
  await expect(page).toHaveURL(/#\/system/)
  await page.getByRole('button', { name: 'Found Colony', exact: true }).click()
  await expect(page).toHaveURL(/#\/found/)

  const wizard = page.getByTestId('found-colony-wizard')
  await expect(wizard).toBeVisible()
  const primaryAction = wizard.locator('footer button.primary')

  // ── Step 1: body ── pick a body that clears the habitability threshold.
  const foundableBody = wizard.locator('.body-card:not(.below-threshold)').first()
  await expect(foundableBody).toBeVisible({ timeout: 15_000 })
  await foundableBody.click()
  await primaryAction.click()

  // ── Step 2: site ── auto-select the best habitable, unoccupied hex.
  await expect(page.getByTestId('jump-to-best-site')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('jump-to-best-site').click()
  await expect(page.locator('.site-details h4')).toHaveText('Selected site')
  await primaryAction.click()

  // ── Step 3: loadout ── queue one starter building so canAdvance is satisfied.
  await expect(wizard.locator('[data-testid^="building-plus-"]').first()).toBeVisible()
  await wizard.locator('[data-testid^="building-plus-"]').first().click()
  await primaryAction.click()

  // ── Step 4: founding ── default colony name is already set.
  await primaryAction.click()

  // A successful `found_colony_at_site` command routes back to /colony with
  // the new colony selected.
  await expect(page).toHaveURL(/#\/colony/)
  await expect(page.locator('[data-testid^="colony-detail-"]')).toBeVisible({ timeout: 15_000 })

  // ── UI-rework PR5: the construction-queue panel's "Build…" button opens the
  // build dialog (the catalog itself only populates in Tauri mode, so here we
  // just verify the panel → dialog wiring, then dismiss it).
  await expect(page.getByTestId('construction-queue-panel')).toBeVisible()
  await page.getByTestId('btn-open-build').click()
  await expect(page.getByTestId('build-dialog')).toBeVisible()
  await page.getByTestId('btn-close-build').click()
  await expect(page.getByTestId('build-dialog')).toHaveCount(0)
})
