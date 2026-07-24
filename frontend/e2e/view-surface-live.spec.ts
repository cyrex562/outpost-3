import { test, expect } from '@playwright/test'

/**
 * Drives the "view body surface" flow (map/nav plan) against a live
 * `outpost_web` backend: new game over the WebSocket, then from the system
 * map select a body and open its read-only surface preview via the
 * `GET /api/body-surface/:id` route.
 *
 * Like `found-colony-live.spec.ts`, this suite requires the `outpost_web`
 * backend that `playwright.config.ts` starts alongside the preview server.
 */

test('view a body surface preview from the system map', async ({ page }) => {
  await page.goto('/')

  // ── New game against the shared engine ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Star map ──
  await page.getByRole('link', { name: 'System' }).click()
  await expect(page).toHaveURL(/#\/system/)

  // Select bodies until we hit one with a solid surface (belts/stations have
  // no "View Surface" action). Every generated system has at least a planet.
  // The system is randomly generated per run, so we can't assume node order —
  // click each and wait briefly for the side panel to re-render before
  // checking (a non-retrying isVisible() would race the reactive update).
  const nodes = page.locator('[data-testid^="body-node-"]')
  await expect(nodes.first()).toBeVisible({ timeout: 15_000 })
  const viewSurface = page.getByTestId('btn-view-surface')
  const count = await nodes.count()
  let found = false
  for (let i = 0; i < count && !found; i++) {
    // Nodes can overlap (moons sit near their parent) or be covered by orbit
    // rings / belt annuli; force past the actionability check and skip any
    // node the click can't land on — some surfaced planet is always reachable.
    try {
      await nodes.nth(i).click({ force: true })
    } catch {
      continue
    }
    found = await viewSurface
      .waitFor({ state: 'visible', timeout: 500 })
      .then(() => true)
      .catch(() => false)
  }
  await expect(viewSurface).toBeVisible()

  // ── Open the read-only surface preview ──
  await viewSurface.click()
  await expect(page).toHaveURL(/#\/surface\//)
  await expect(page.getByTestId('surface-view')).toBeVisible()
  await expect(page.getByTestId('planet-hex-map')).toBeVisible({ timeout: 15_000 })

  // ── Back to the system map ──
  await page.getByTestId('btn-back-system').click()
  await expect(page).toHaveURL(/#\/system/)
})
