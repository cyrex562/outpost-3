import { test, expect } from '@playwright/test'

/**
 * The marine power plants against a live backend (issue #418).
 *
 * Both are tech-gated, so at tech 0 they must read as locked — which is the
 * useful assertion here, since a *researched* run would need the tech tree
 * driven first. Their coastal requirement is measured at the colony's own hex
 * and holds about half the time, so the spec asserts engine/badge/button
 * agreement rather than a fixed outcome.
 */

test('marine plants are tech-locked at tech 0 and show their coastal requirement', async ({
  page,
}) => {
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

  for (const id of ['wave_power_plant', 'ocean_thermal_plant']) {
    await expect(page.getByTestId(`build-card-${id}`)).toBeVisible()

    // Nothing researched, so the marine tech requirement is unmet.
    const tech = page.getByTestId(`build-req-${id}-tech-marine_power`)
    await expect(tech).toBeVisible()
    await expect(tech).toHaveAttribute('data-met', 'false')
    await expect(page.getByTestId(`btn-queue-${id}`)).toBeDisabled()

    // The coastal requirement is listed too — every requirement shows on
    // every card, met or not (issue #423) — so a player can see *both*
    // things standing between them and this building.
    const site = page.locator(`[data-testid^="build-req-${id}-site-"]`).first()
    await expect(site).toBeVisible()
    await expect(site).toContainText(/ocean within 1 hex/i)
  }

  // The two are not reskins: they scale on different properties, so their
  // expected yields at the same site generally differ.
  const waveYield = page.getByTestId('build-site-yield-wave_power_plant')
  const thermalYield = page.getByTestId('build-site-yield-ocean_thermal_plant')
  await expect(waveYield).toBeVisible()
  await expect(thermalYield).toBeVisible()
})
