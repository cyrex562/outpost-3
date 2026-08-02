import { test, expect } from '@playwright/test'

/**
 * Drives the trade-route screen (issue #363) against a live `outpost_web`
 * backend: add a manually-created route between two colonies and confirm it
 * round-trips through the engine, then remove it.
 *
 * The point of this spec is the same class of regression `AddTradeRoute`
 * being unexposed by both hosts was: a control that looks like it works but
 * never reaches `GameEngine::apply`. Every assertion below checks state that
 * came back from the engine (a page reload before checking removal, a fresh
 * `/api/trade-routes` read implied by the list re-render) rather than only
 * local component state.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

const API = 'http://localhost:3000'

test('add and remove a trade route between two colonies', async ({ page, request }) => {
  await page.goto('/')

  // ── New game against the shared engine ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Found two colonies of our own with distinctive names ──
  const foundA = await request.post(`${API}/api/command`, {
    data: { kind: 'found_colony', name: 'Trade Fixture Alpha', starting_population: 100 },
  })
  expect(foundA.ok()).toBeTruthy()
  const foundB = await request.post(`${API}/api/command`, {
    data: { kind: 'found_colony', name: 'Trade Fixture Beta', starting_population: 100 },
  })
  expect(foundB.ok()).toBeTruthy()

  // ── Navigate to the trade screen; the shared WS connection should have
  // already broadcast both foundings into the world store's colony list. ──
  await page.goto('/#/trade')
  await expect(page.getByTestId('trade-view')).toBeVisible()

  const colonyASelect = page.getByTestId('colony-a-select')
  const colonyBSelect = page.getByTestId('colony-b-select')
  await expect(colonyASelect.locator('option', { hasText: 'Trade Fixture Alpha' })).toHaveCount(1, {
    timeout: 15_000,
  })
  await expect(colonyBSelect.locator('option', { hasText: 'Trade Fixture Beta' })).toHaveCount(1, {
    timeout: 15_000,
  })

  await colonyASelect.selectOption({ label: 'Trade Fixture Alpha' })
  await colonyBSelect.selectOption({ label: 'Trade Fixture Beta' })
  await page.getByTestId('throughput-cap-input').fill('35')
  await page.getByTestId('btn-add-route').click()

  // The route appears in the list, engine-derived transit sols and all.
  const routeList = page.getByTestId('route-list')
  await expect(routeList).toBeVisible({ timeout: 10_000 })
  await expect(routeList).toContainText('Trade Fixture Alpha')
  await expect(routeList).toContainText('Trade Fixture Beta')
  await expect(routeList).toContainText('35')

  // Reload: the route is loaded fresh from `/api/trade-routes`, so it can
  // only still be here if the engine actually persisted it.
  await page.reload()
  await expect(page.getByTestId('route-list')).toContainText('Trade Fixture Alpha', {
    timeout: 15_000,
  })

  // ── Remove it ──
  const routeRow = page.locator('[data-testid^="route-row-"]', { hasText: 'Trade Fixture Alpha' })
  await routeRow.getByRole('button', { name: 'Remove' }).click()
  await expect(page.getByTestId('no-routes')).toBeVisible({ timeout: 10_000 })

  await page.reload()
  await expect(page.getByTestId('no-routes')).toBeVisible({ timeout: 15_000 })
})
