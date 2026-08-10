import { test, expect } from '@playwright/test'

/**
 * Wind's atmosphere requirement and density scaling (issue #416) against a
 * live backend.
 *
 * Unlike geothermal's hex-scoped gate, wind's is *body*-scoped, so this is
 * the first end-to-end exercise of a requirement that depends on the body
 * rather than the ground. Roughly two thirds of foundable bodies are vacuum,
 * so which case a run lands in varies — the spec asserts that the engine, the
 * badge, and the Queue button all agree, whichever it is.
 */

test('wind is gated on atmosphere, and the UI agrees with the engine', async ({ page }) => {
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

  // Tech 0: present with nothing researched (issue #409).
  await expect(page.getByTestId('build-card-wind_turbine')).toBeVisible()

  // Its atmosphere requirement is listed either way — every requirement shows
  // on every card (issue #423).
  const req = page.locator('[data-testid^="build-req-wind_turbine-site-"]').first()
  await expect(req).toBeVisible()
  await expect(req).toContainText(/atmosphere/i)

  const met = (await req.getAttribute('data-met')) === 'true'
  const queue = page.getByTestId('btn-queue-wind_turbine')

  if (met) {
    // The body has air: buildable, and its yield is shown before committing.
    await expect(queue).toBeEnabled()
    const yieldRow = page.getByTestId('build-site-yield-wind_turbine')
    await expect(yieldRow).toBeVisible()
    await expect(yieldRow).toContainText('% of nominal output here')
  } else {
    // Airless: refused by the engine, and the dialog says why rather than
    // erroring on click.
    await expect(queue).toBeDisabled()
    await expect(page.getByTestId('build-card-wind_turbine')).toHaveAttribute(
      'title',
      /atmosphere/i,
    )
  }
})
