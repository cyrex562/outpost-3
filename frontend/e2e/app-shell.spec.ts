import { test, expect } from '@playwright/test'

/**
 * Smoke coverage for the app shell in browser (non-Tauri) mode — no
 * `outpost_web` backend is started for this suite, so these tests only
 * exercise what the UI can do standalone: initial render, routing, and the
 * new-game form's client-side validation. Tests that need live game state
 * (advancing a sol, founding a colony) belong in a separate suite that
 * starts `outpost_web` first — see docs/HARNESS.md.
 */

test('main menu renders with the primary actions', async ({ page }) => {
  await page.goto('/')
  await expect(page.getByTestId('main-menu')).toBeVisible()
  await expect(page.locator('.menu-title')).toHaveText('OUTPOST 3')
  await expect(page.getByTestId('btn-new-game')).toBeVisible()
  await expect(page.getByTestId('btn-load-game')).toBeVisible()
  await expect(page.getByTestId('btn-settings')).toBeVisible()
})

test('New Game panel opens and exposes the difficulty selector', async ({ page }) => {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await expect(page.getByTestId('difficulty-select')).toBeVisible()
  await expect(page.getByTestId('btn-start')).toBeVisible()
})

test('Settings panel opens', async ({ page }) => {
  await page.goto('/')
  await page.getByTestId('btn-settings').click()
  await expect(page.getByTestId('settings-panel')).toBeVisible()
})
