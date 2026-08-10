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

  // ── Step 3: loadout ── the bootstrap kit is a fixed, read-only preview
  // (issue #312) — no per-building picker, and nothing here gates advancing.
  await expect(wizard.locator('[data-testid^="kit-building-"]').first()).toBeVisible()
  const budget = wizard.getByTestId('budget-preview')
  await expect(budget).not.toContainText('0 / ')
  await primaryAction.click()

  // ── Step 4: founding ── name it distinctly; the bootstrap colony is also
  // called "Alpha Base", so the default would be ambiguous to select later.
  await page.getByTestId('colony-name-input').fill('Kitted Landing')
  await primaryAction.click()

  // A successful `found_colony_at_site` command routes back to /colony.
  //
  // The colony does **not** exist yet. The wizard always names a sponsor, so
  // the engine takes issue #359's deferred path: the sponsor is billed now and
  // the colony arrives after a distance-derived transit, announced by
  // `colony_founding_launched`. Issue #403 was this spec assuming an instant
  // founding — and, underneath that, the launch event having no wire
  // representation at all, so the player got no confirmation either.
  await expect(page).toHaveURL(/#\/colony/)
  await expect(page.locator('[data-testid^="colony-detail-"]')).toBeVisible({ timeout: 15_000 })

  // The launch must announce itself. This is the half of #403 that was a real
  // bug rather than a wrong assertion: `colony_founding_launched` had no wire
  // representation, so it mapped to `Ignored` and the player received no
  // confirmation that anything had happened at all.
  //
  // Deliberately not asserting a *count*: browser mode currently logs each
  // command-issued event twice (the POST response is fed to the reducer and
  // the server also fans the same events out over the WebSocket to every
  // client including the issuer). That duplication is pre-existing and filed
  // separately — pinning a count here would couple this spec to it.
  const launched = page.getByTestId('log-item-colony_founding_launched').first()
  await expect(launched).toBeVisible({ timeout: 15_000 })
  await expect(launched).toContainText('Kitted Landing')

  // ── UI-rework PR5: the construction-queue panel's "Build…" button opens the
  // build dialog (the catalog itself only populates in Tauri mode, so here we
  // just verify the panel → dialog wiring, then dismiss it).
  await expect(page.getByTestId('construction-queue-panel')).toBeVisible()
  await page.getByTestId('btn-open-build').click()
  await expect(page.getByTestId('build-dialog')).toBeVisible()
  await page.getByTestId('btn-close-build').click()
  await expect(page.getByTestId('build-dialog')).toHaveCount(0)

  // ── UI-rework PR6: the planet map is a draggable floating window; drag its
  // title bar and confirm it actually moves.
  await page.getByRole('link', { name: 'Planet', exact: true }).click()
  await expect(page).toHaveURL(/#\/planet/)
  const win = page.getByTestId('floating-window')
  await expect(win).toBeVisible()
  await expect(page.getByTestId('planet-hex-map')).toBeVisible()

  // Issue #320: with `fill-host` the window opens filling its host rather than
  // at a fixed 760x520, so most of a large display isn't left empty. Measure
  // against the host itself — the viewport also carries the header/nav chrome.
  const hostBox = (await page.locator('.map-host').boundingBox())!
  const opened = (await win.boundingBox())!
  expect(opened.width).toBeGreaterThan(hostBox.width * 0.9)
  expect(opened.height).toBeGreaterThan(hostBox.height * 0.9)

  // An untouched fill-host window keeps tracking the host, so growing the app
  // grows the window and therefore the map inside it — otherwise the window
  // would be stranded at the old size the moment the player resized the app.
  const planetMap = page.getByTestId('planet-hex-map')
  const mapBefore = (await planetMap.boundingBox())!
  await page.setViewportSize({ width: 1280, height: 1200 })
  await expect
    .poll(async () => (await planetMap.boundingBox())?.height ?? 0, { timeout: 5_000 })
    .toBeGreaterThan(mapBefore.height + 100)
  await page.setViewportSize({ width: 1280, height: 720 })
  await expect
    .poll(async () => (await win.boundingBox())?.height ?? 0, { timeout: 5_000 })
    .toBeLessThan(hostBox.height + 1)

  // Maximise sits it flush against the host, and restore brings it back.
  await page.getByTestId('fw-maximise').click()
  const maxed = (await win.boundingBox())!
  expect(maxed.width).toBeGreaterThanOrEqual(hostBox.width - 1)
  await page.getByTestId('fw-maximise').click()
  await expect(win).not.toHaveClass(/maximised/)

  const before = await win.boundingBox()
  const bar = page.getByTestId('fw-titlebar')
  const barBox = (await bar.boundingBox())!
  await page.mouse.move(barBox.x + 40, barBox.y + 10)
  await page.mouse.down()
  await page.mouse.move(barBox.x + 140, barBox.y + 70, { steps: 5 })
  await page.mouse.up()
  const after = await win.boundingBox()
  expect(after!.x).toBeGreaterThan(before!.x)
  expect(after!.y).toBeGreaterThan(before!.y)

  // ── UI-rework PR7: the "Colonies" nav lists founded colonies; clicking a
  // card opens that colony's dashboard.
  await page.getByRole('link', { name: 'Colonies', exact: true }).click()
  await expect(page).toHaveURL(/#\/colonies/)
  const card = page.locator('[data-testid^="colony-card-"]').first()
  await expect(card).toBeVisible()
  await card.click()
  await expect(page).toHaveURL(/#\/colony\//)
  await expect(page.locator('[data-testid^="colony-detail-"]')).toBeVisible({ timeout: 15_000 })

  // Advance time so the in-transit founding actually lands (issue #359/#403).
  // One fast-forward covers the transit, which is at least one sol and
  // distance-derived; polling for the card rather than asserting straight
  // after keeps this honest if the distance ever changes.
  await page.getByRole('link', { name: 'Colonies', exact: true }).click()
  const arriving = page.locator('[data-testid^="colony-card-"]', { hasText: 'Kitted Landing' })
  await expect
    .poll(
      async () => {
        if ((await arriving.count()) > 0) return true
        await page.getByTestId('btn-fast-forward').click()
        await page.waitForTimeout(500)
        return (await arriving.count()) > 0
      },
      { timeout: 30_000 },
    )
    .toBe(true)

  // Issue #317: the colony founded through the wizard is operational on arrival
  // — its landing kit is already standing, not sitting in the build queue.
  await arriving.first().click()
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
  await expect
    .poll(async () => page.locator('[data-testid^="building-row-"]').count(), { timeout: 15_000 })
    .toBeGreaterThan(1)
})
