import { test, expect } from '@playwright/test'

/**
 * Drives the floating building-details window (issue #322, revised) against
 * a live `outpost_web` backend.
 *
 * Selecting a building used to navigate to a routed `/facility/:type` page,
 * then (issue #339) opened a floating window over the colony view, then
 * briefly (issue #322's first pass) became a dock panel split alongside the
 * buildings list inside #321's dockview framework. That last step turned out
 * to compete for space with — and could disturb — the player's own dragged
 * dock arrangement, so it opens as a `FloatingWindow` layered above the
 * whole dock instead: the buildings list (and every other dock panel) stays
 * visible and untouched underneath, and the URL does not change. This spec
 * checks that end-to-end: clicking a building opens the floating window with
 * the right content, the dock underneath remains interactive, and the
 * window's own close button dismisses it without disturbing the dock.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

const API = 'http://localhost:3000'

test('selecting a building opens a floating details window above the dock', async ({
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
    data: { kind: 'found_colony', name: 'Floating Details Fixture', starting_population: 100 },
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

  // ── Selecting a building opens the floating window, not a route change ──
  await page.getByTestId('view-details-colony_hq').click()
  await expect(page.getByTestId('floating-window')).toBeVisible({ timeout: 10_000 })
  await expect(page.getByTestId('facility-page')).toBeVisible()
  await expect(page).toHaveURL(new RegExp(colonyId!))
  await expect(page).not.toHaveURL(/#\/facility/)

  // The dock underneath — buildings list included — stays visible and
  // usable while the window floats above it.
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
  await expect(page.getByText('Colony HQ 1')).toBeVisible()

  // ── Dismissing the window closes it and leaves the dock untouched ──
  await page.getByTestId('fw-close').click()
  await expect(page.getByTestId('floating-window')).toBeHidden()
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
})

test('deep-linking /colony/:colonyId/facility/:buildingType opens the floating window', async ({
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
  // the floating window already open — a deep link into the window rather
  // than a separate routed page.
  await page.goto(`/#/colony/${colonyId}/facility/colony_hq`)
  await expect(page.getByTestId('buildings-panel')).toBeVisible({ timeout: 15_000 })
  await expect(page.getByTestId('floating-window')).toBeVisible({ timeout: 10_000 })
  await expect(page.getByTestId('facility-page')).toBeVisible()
})
