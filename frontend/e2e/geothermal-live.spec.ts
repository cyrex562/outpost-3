import { test, expect } from '@playwright/test'

/**
 * The geothermal plant against a live backend (issue #414) — the first
 * content to declare both a site requirement (#410) and site-scaled output
 * (#411), so this is the first end-to-end coverage either mechanism has had.
 *
 * Deliberately asserts on *shape* rather than exact numbers: the colony lands
 * on a procedurally generated hex, so its gradient — and therefore the plant's
 * yield — differs every run. What must hold every time is that the yield is
 * shown before committing, and that it agrees with the requirement badge.
 */

test('the geothermal plant shows its site yield before anything is built', async ({ page }) => {
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

  // Tech 0: it is in the catalogue with nothing researched.
  const card = page.getByTestId('build-card-geothermal_plant')
  await expect(card).toBeVisible()

  // Its expected yield is shown *before* committing materials — the whole
  // point, since a plant on cold crust is otherwise a trap purchase.
  const yieldRow = page.getByTestId('build-site-yield-geothermal_plant')
  await expect(yieldRow).toBeVisible()
  await expect(yieldRow).toContainText('% of nominal output here')

  // The percentage is a real reading, not a placeholder.
  const text = (await yieldRow.textContent()) ?? ''
  const pct = Number(/(\d+)%/.exec(text)?.[1])
  expect(Number.isFinite(pct)).toBe(true)
  expect(pct).toBeGreaterThan(0)
  expect(pct).toBeLessThanOrEqual(140)

  // The gradient requirement is listed either way, met or unmet — every
  // requirement shows on every card (issue #423).
  const req = page.locator('[data-testid^="build-req-geothermal_plant-site-"]').first()
  await expect(req).toBeVisible()
  await expect(req).toContainText(/geothermal gradient/i)

  // Requirement and queue state must agree: if the site is too cold, the
  // button is disabled and the badge says so; otherwise both are fine.
  const met = (await req.getAttribute('data-met')) === 'true'
  if (met) {
    await expect(page.getByTestId('btn-queue-geothermal_plant')).toBeEnabled()
  } else {
    await expect(page.getByTestId('btn-queue-geothermal_plant')).toBeDisabled()
    await expect(req).toContainText(/deep_drilling/i)
  }
})
