import { test, expect } from '@playwright/test'

/**
 * The combustion plant against a live backend (issue #417).
 *
 * Its requirement is a *deposit within a radius* — the third distinct kind of
 * site condition to reach the UI, after geothermal's hex-scoped gradient
 * (#414) and wind's body-scoped atmosphere (#416). Whether a given colony has
 * hydrocarbons within two hexes varies per run, so this asserts that the
 * engine, the badge, and the Queue button agree either way.
 */

test('combustion is gated on nearby hydrocarbons, and the UI agrees', async ({ page }) => {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  await page.getByTestId('btn-open-build').click()
  await expect(page.getByTestId('build-dialog')).toBeVisible()
  await expect
    .poll(async () => page.locator('[data-testid^="build-card-"]').count(), { timeout: 15_000 })
    .toBeGreaterThan(0)

  // Tech 0, and so is the well that fuels it — the chain closes without
  // research, which is the thing issue #417 had to resolve.
  await expect(page.getByTestId('build-card-combustion_plant')).toBeVisible()
  await expect(page.getByTestId('build-card-hydrocarbon_well')).toBeVisible()
  await expect(page.getByTestId('build-req-hydrocarbon_well-tech-resource_extraction')).toHaveCount(0)

  // Its output is site-independent, so unlike geothermal and wind it shows no
  // expected-yield line — that is the distinction, not an omission.
  await expect(page.getByTestId('build-site-yield-combustion_plant')).toHaveCount(0)

  const req = page.locator('[data-testid^="build-req-combustion_plant-site-"]').first()
  await expect(req).toBeVisible()
  await expect(req).toContainText(/hydrocarbons deposit/i)

  const met = (await req.getAttribute('data-met')) === 'true'
  const queue = page.getByTestId('btn-queue-combustion_plant')
  if (met) {
    await expect(queue).toBeEnabled()
  } else {
    await expect(queue).toBeDisabled()
    await expect(page.getByTestId('build-card-combustion_plant')).toHaveAttribute(
      'title',
      /hydrocarbons/i,
    )
  }
})
