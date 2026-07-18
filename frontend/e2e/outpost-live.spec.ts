import { test, expect } from '@playwright/test'

/**
 * Drives the outpost lifecycle (issue #243) against a live `outpost_web`
 * backend: new game over the WebSocket, establish an outpost from the
 * Outposts view, verify it appears in the list, then promote it and verify
 * it's removed from the outpost list (having become an independent colony).
 *
 * Like `found-colony-live.spec.ts`, this suite requires the `outpost_web`
 * backend `playwright.config.ts` starts alongside the preview server. The
 * new-game flow founds its initial colony via the bare `FoundColony`
 * command (no spatial placement), so the range gate (#241) is inert here —
 * every candidate body reports `in_range: true`.
 */

test('establish outpost, view it, then promote it to a colony', async ({ page }) => {
  await page.goto('/')

  // ── Main menu → New Game panel → Start ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()

  await expect(page.getByTestId('start-game-btn')).toBeVisible()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Navigate to Outposts ──
  await page.getByRole('link', { name: 'Outposts' }).click()
  await expect(page).toHaveURL(/#\/outposts/)
  await expect(page.getByTestId('outposts-view')).toBeVisible()

  // ── Establish an outpost on the first available (in-range) body ──
  await page.getByTestId('btn-establish-outpost').click()
  const panel = page.getByTestId('establish-panel')
  await expect(panel).toBeVisible()

  const firstTarget = panel.locator('[data-testid^="target-"]:not(.out-of-range)').first()
  await expect(firstTarget).toBeVisible({ timeout: 15_000 })
  await firstTarget.click()

  await page.getByTestId('outpost-name-input').fill('Mining Camp Alpha')
  await page.getByTestId('btn-confirm-establish').click()

  // The outpost list refreshes and shows the new outpost.
  const outpostList = page.getByTestId('outpost-list')
  await expect(outpostList.getByText('Mining Camp Alpha')).toBeVisible({ timeout: 15_000 })

  // ── Promote it to a full colony ──
  const card = outpostList.locator('.outpost-card', { hasText: 'Mining Camp Alpha' })
  await card.getByRole('button', { name: 'Promote to Colony' }).click()
  await card.getByTestId('btn-confirm-promote').click()

  // Once promoted, the outpost is gone from the list.
  await expect(outpostList.getByText('Mining Camp Alpha')).toHaveCount(0, { timeout: 15_000 })
})
