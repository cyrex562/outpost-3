import { test, expect } from '@playwright/test'

/**
 * Proves issue #364's decision: the live balance-tuning editor is no longer
 * in the main nav bar alongside real gameplay screens, and is reachable
 * instead as a "Dev Tools" toggle inside the hamburger menu.
 *
 * Requires the `outpost_web` backend `playwright.config.ts` starts alongside
 * the preview server — the header/nav this test inspects only renders once
 * a game is in progress (`inGame` in `App.vue`).
 */

test('Balance is not in the main nav, and is reachable from the menu instead', async ({
  page,
}) => {
  await page.goto('/')

  // ── New game so the in-game header/nav renders ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Main nav bar has no Balance link ──
  const nav = page.locator('.header-nav')
  await expect(nav).toBeVisible()
  await expect(nav.getByRole('link', { name: 'Balance' })).toHaveCount(0)

  // ── Balance tuning lives in the hamburger menu's Dev Tools section instead ──
  await page.getByTestId('app-menu-btn').click()
  await expect(page.getByTestId('app-menu-dialog')).toBeVisible()

  const balanceBtn = page.getByTestId('btn-balance')
  await expect(balanceBtn).toBeVisible()
  await expect(page.getByTestId('balance-tuning-panel')).toHaveCount(0)

  await balanceBtn.click()
  await expect(page.getByTestId('balance-tuning-panel')).toBeVisible({ timeout: 10_000 })
})
