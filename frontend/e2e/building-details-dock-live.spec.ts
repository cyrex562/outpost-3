import { test, expect } from '@playwright/test'

/**
 * Drives the building-details dock panel (issue #322) against a live
 * `outpost_web` backend.
 *
 * Selecting a building used to navigate to a routed `/facility/:type` page,
 * then (issue #339) opened a floating window over the colony view. Per
 * #321's dock framework, it's now a closeable panel inside that same
 * dockview, split alongside the buildings list rather than replacing or
 * covering it — the colony context (including the list just clicked) stays
 * visible the whole time. This spec checks that end-to-end: clicking a
 * building opens the panel with the right content next to the buildings
 * list, the URL does not change, and closing the panel's tab removes it
 * while leaving the rest of the dock intact.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

const API = 'http://localhost:3000'

test('selecting a building opens a dock panel alongside the buildings list', async ({
  page,
  request,
}) => {
  await page.goto('/')

  // ── New game against the shared engine — this bootstraps a colony ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Seed a colony of our own with one building on it ──
  //
  // Same fixture pattern as building-staffing-live.spec.ts: the founding
  // wizard doesn't place buildings in browser mode, so seed one directly
  // through the same `/api/command` route the app itself dispatches
  // through, then drive the real UI against it.
  const preexisting = (await (await request.get(`${API}/api/colonies`)).json()) as {
    Colonies: { id: string }[]
  }
  const known = new Set(preexisting.Colonies.map((c) => c.id))

  const found = await request.post(`${API}/api/command`, {
    data: { kind: 'found_colony', name: 'Dock Details Fixture', starting_population: 100 },
  })
  expect(found.ok()).toBeTruthy()

  const refreshed = (await (await request.get(`${API}/api/colonies`)).json()) as {
    Colonies: { id: string }[]
  }
  const colonyId = refreshed.Colonies.map((c) => c.id).find((id) => !known.has(id))
  expect(colonyId, 'the founded colony should be new to /api/colonies').toBeTruthy()

  const deploy = await request.post(`${API}/api/command`, {
    data: { kind: 'deploy_starter_kit', colony_id: colonyId, buildings: [['colony_hq', 1]] },
  })
  expect(deploy.ok()).toBeTruthy()

  await page.goto(`/#/colony/${colonyId}`)
  await expect(page).toHaveURL(new RegExp(colonyId!))
  await expect(page.getByTestId('buildings-panel')).toBeVisible({ timeout: 15_000 })

  // ── Selecting a building opens the details panel, not a route change ──
  await page.getByTestId('view-details-colony_hq').click()
  await expect(page.getByTestId('facility-page')).toBeVisible({ timeout: 10_000 })
  await expect(page).toHaveURL(new RegExp(colonyId!))
  await expect(page).not.toHaveURL(/#\/facility/)

  // The buildings list stays visible alongside the details panel — split,
  // not tabbed — which is the whole point of docking it this way.
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
  await expect(page.getByText('Colony HQ 1')).toBeVisible()

  // ── Closing the panel's own control removes it; the rest of the dock stays ──
  await page.getByTestId('facility-back').click()
  await expect(page.getByTestId('facility-page')).toBeHidden()
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
})

test('deep-linking /colony/:colonyId/facility/:buildingType opens the details panel', async ({
  page,
  request,
}) => {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  const preexisting = (await (await request.get(`${API}/api/colonies`)).json()) as {
    Colonies: { id: string }[]
  }
  const known = new Set(preexisting.Colonies.map((c) => c.id))

  const found = await request.post(`${API}/api/command`, {
    data: { kind: 'found_colony', name: 'Deep Link Fixture', starting_population: 100 },
  })
  expect(found.ok()).toBeTruthy()

  const refreshed = (await (await request.get(`${API}/api/colonies`)).json()) as {
    Colonies: { id: string }[]
  }
  const colonyId = refreshed.Colonies.map((c) => c.id).find((id) => !known.has(id))
  expect(colonyId, 'the founded colony should be new to /api/colonies').toBeTruthy()

  const deploy = await request.post(`${API}/api/command`, {
    data: { kind: 'deploy_starter_kit', colony_id: colonyId, buildings: [['colony_hq', 1]] },
  })
  expect(deploy.ok()).toBeTruthy()

  // Visiting the facility route directly lands on the colony dashboard with
  // the details panel already open — a deep link into the panel rather than
  // a separate routed page (issue #322 decision).
  await page.goto(`/#/colony/${colonyId}/facility/colony_hq`)
  await expect(page.getByTestId('buildings-panel')).toBeVisible({ timeout: 15_000 })
  await expect(page.getByTestId('facility-page')).toBeVisible({ timeout: 10_000 })
})
