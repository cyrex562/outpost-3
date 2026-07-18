import { test, expect } from '@playwright/test'

/**
 * Drives the tech-tree UI (issue #250) against a live `outpost_web` backend:
 * new game over the WebSocket, navigate to the Tech Tree, filter by category,
 * select an available tech, and start researching it.
 *
 * Like `found-colony-live.spec.ts` / `outpost-live.spec.ts`, this suite
 * requires the `outpost_web` backend `playwright.config.ts` starts alongside
 * the preview server.
 */

test('browse the tech tree, filter by category, and start research', async ({ page }) => {
  await page.goto('/')

  // ── Main menu → New Game panel → Start ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()

  await expect(page.getByTestId('start-game-btn')).toBeVisible()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Navigate to the Tech Tree ──
  await page.getByRole('link', { name: 'Tech' }).click()
  await expect(page).toHaveURL(/#\/tech/)
  const view = page.getByTestId('tech-tree')
  await expect(view).toBeVisible()

  // The tree loads with at least one available node rendered.
  const availableNode = view.locator('.node.state-available').first()
  await expect(availableNode).toBeVisible({ timeout: 15_000 })

  // ── Filter by category narrows the visible node set ──
  const categorySelect = page.getByTestId('tech-filter-category')
  const options = await categorySelect.locator('option').allTextContents()
  const firstRealCategory = options.find((o) => o !== 'All')
  expect(firstRealCategory).toBeTruthy()
  await categorySelect.selectOption(firstRealCategory!)
  await expect(view.locator('.node').first()).toBeVisible()
  await categorySelect.selectOption('')

  // ── Select an available node and start research ──
  await availableNode.click()
  const detail = page.getByTestId('tech-detail')
  await expect(detail).toBeVisible()
  await page.getByTestId('tech-research-btn').click()

  // Once researching, the queue panel shows the active project.
  await expect(page.getByTestId('research-queue')).toBeVisible({ timeout: 15_000 })
})
