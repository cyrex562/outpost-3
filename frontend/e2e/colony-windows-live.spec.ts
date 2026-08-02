import { test, expect } from '@playwright/test'

/**
 * Drives the colony screen's floating-window panel layout (colony details
 * multi-window redesign, which replaced `dockview-vue`'s tiling layout —
 * see `colony-windows-live.spec.ts`'s predecessor, `colony-dock-live.spec.ts`,
 * removed with this change) against a live `outpost_web` backend.
 *
 * Drag-to-snap and z-order aren't meaningfully assertable here (pointer-event
 * heavy, exact pixel geometry isn't something a spec should pin down) — see
 * `FloatingWindow.test.ts`'s "multi-window snapping + z-order" suite for
 * that. This spec covers what a unit test can't: the six windows actually
 * mount, closing/reopening via the palette works, and the arrangement
 * persists across a reload.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

const PANEL_TITLES = ['Vitals', 'Utilities', 'Commodities', 'Buildings', 'Construction Queue', 'Alerts']

async function startNewGame(page: import('@playwright/test').Page): Promise<void> {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)
}

test('all six panel windows open by default, persist their open state across a reload, and Reset Layout restores them', async ({
  page,
}) => {
  await startNewGame(page)

  for (const title of PANEL_TITLES) {
    await expect(page.locator('[data-testid="floating-window"]', { hasText: title })).toBeVisible()
  }

  // Close one — its own × button, not the palette.
  const vitals = page.locator('[data-window-id="vital-stats"]')
  await vitals.getByTestId('fw-close').click()
  await expect(vitals).toBeHidden()

  // Its palette chip stops reading as "open"...
  const vitalsChip = page.getByTestId('palette-toggle-vital-stats')
  await expect(vitalsChip).not.toHaveClass(/open/)

  // Reloading must remember it was closed, not silently reopen it.
  await page.reload()
  await expect(page).toHaveURL(/#\/colony/)
  await expect(page.locator('[data-window-id="vital-stats"]')).toBeHidden()
  await expect(page.locator('[data-window-id="buildings"]')).toBeVisible()

  // The palette reopens it.
  await page.getByTestId('palette-toggle-vital-stats').click()
  await expect(page.locator('[data-window-id="vital-stats"]')).toBeVisible()

  // "Reset Layout" must not break the view — every window present afterward.
  await page.getByTestId('btn-reset-layout').click()
  for (const title of PANEL_TITLES) {
    await expect(page.locator('[data-testid="floating-window"]', { hasText: title })).toBeVisible()
  }
})

test('dragging a panel window\'s title bar moves it', async ({ page }) => {
  await startNewGame(page)

  const win = page.locator('[data-window-id="alerts"]')
  await expect(win).toBeVisible()
  const before = await win.boundingBox()
  expect(before).not.toBeNull()

  const titlebar = win.getByTestId('fw-titlebar')
  const bar = await titlebar.boundingBox()
  expect(bar).not.toBeNull()

  await page.mouse.move(bar!.x + bar!.width / 2, bar!.y + bar!.height / 2)
  await page.mouse.down()
  await page.mouse.move(bar!.x + bar!.width / 2 - 120, bar!.y + bar!.height / 2 + 80, { steps: 5 })
  await page.mouse.up()

  const after = await win.boundingBox()
  expect(after).not.toBeNull()
  expect(after!.x).not.toBe(before!.x)
  expect(after!.y).not.toBe(before!.y)
})
