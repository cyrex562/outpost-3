import { test, expect } from '@playwright/test'

/**
 * Covers the planet hex map's east-west wrap toggle against a live
 * `outpost_web` backend.
 *
 * Reaches a real planet map via the founding wizard's step 2 (the landing-
 * site picker) and stops there — deliberately short of completing the
 * founding, so this spec doesn't inherit the downstream failure tracked in
 * issue #403. The surface-preview route to a planet map is flaky for
 * unrelated reasons (issue #402), which is why the wizard is used instead.
 */

test('toggle east-west wrapping on the planet map, and it survives a reload', async ({ page }) => {
  await page.goto('/')

  // ── New game against the shared engine ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Star map → founding wizard ──
  await page.getByRole('link', { name: 'System' }).click()
  await expect(page).toHaveURL(/#\/system/)
  await page.getByRole('button', { name: 'Found Colony', exact: true }).click()
  await expect(page).toHaveURL(/#\/found/)

  const wizard = page.getByTestId('found-colony-wizard')
  await expect(wizard).toBeVisible()

  // ── Step 1: pick a foundable body, advance to the map ──
  // Must be a *planet*, not a belt: `.body-card:not(.below-threshold)` alone
  // often lands on an asteroid belt (belts clear the habitability threshold
  // but have no hex surface), and step 2 then sits on "Loading map…"
  // forever. Systems are randomly generated per run, so filtering by kind is
  // what makes this deterministic.
  const foundableBody = wizard
    .locator('.body-card:not(.below-threshold)')
    .filter({ hasText: 'Planet' })
    .first()
  await expect(foundableBody).toBeVisible({ timeout: 15_000 })
  await foundableBody.click()
  await wizard.locator('footer button.primary').click()

  // ── Step 2: the planet hex map ──
  await expect(page.getByTestId('planet-hex-map')).toBeVisible({ timeout: 15_000 })
  const toggle = page.getByTestId('planet-map-wrap-toggle')
  await expect(toggle).toBeVisible()

  // Wrapping ships on by default.
  await expect(toggle).toHaveAttribute('aria-pressed', 'true')
  await expect(toggle).toContainText(/on/i)

  const hexes = page.locator('[data-testid="planet-hex-map"] polygon')
  const wrappedCount = await hexes.count()
  expect(wrappedCount).toBeGreaterThan(0)

  // ── Turn wrapping off: the repeated copies collapse to a single map ──
  await toggle.click()
  await expect(toggle).toHaveAttribute('aria-pressed', 'false')
  await expect(toggle).toContainText(/off/i)
  await expect.poll(async () => hexes.count()).toBeLessThan(wrappedCount)
  const unwrappedCount = await hexes.count()

  // ── Back on: the repeats come back ──
  await toggle.click()
  await expect(toggle).toHaveAttribute('aria-pressed', 'true')
  await expect.poll(async () => hexes.count()).toBeGreaterThan(unwrappedCount)

  // ── The choice persists across a reload ──
  await toggle.click()
  await expect(toggle).toHaveAttribute('aria-pressed', 'false')
  await page.reload()
  // The wizard resets to step 1 on reload, so walk back to the map.
  await expect(wizard).toBeVisible({ timeout: 15_000 })
  const bodyAgain = wizard
    .locator('.body-card:not(.below-threshold)')
    .filter({ hasText: 'Planet' })
    .first()
  await expect(bodyAgain).toBeVisible({ timeout: 15_000 })
  await bodyAgain.click()
  await wizard.locator('footer button.primary').click()
  await expect(page.getByTestId('planet-hex-map')).toBeVisible({ timeout: 15_000 })

  await expect(page.getByTestId('planet-map-wrap-toggle')).toHaveAttribute('aria-pressed', 'false')
})
