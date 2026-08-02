import { test, expect } from '@playwright/test'

/**
 * Drives the floating building-details window (issue #322, revised) against
 * a live `outpost_web` backend.
 *
 * Selecting a building used to navigate to a routed `/facility/:type` page,
 * then (issue #339) opened a floating window over the colony view, then
 * briefly (issue #322's first pass) became a dock panel split alongside the
 * buildings list inside #321's `dockview-vue` framework. That last step
 * turned out to compete for space with — and could disturb — the player's
 * own dragged dock arrangement, so it opened as a `FloatingWindow` layered
 * above the dock instead; the colony details multi-window redesign later
 * replaced that dock with a set of panel windows of its own (see
 * `colony-windows-live.spec.ts`), but building details still work the same
 * way: a `FloatingWindow` layered above everything else, sharing the same
 * snap/z-order registry (`window-id="building-details"`, distinct from the
 * six panel windows' own ids so this spec can target it unambiguously even
 * with all six also open). This spec checks that end-to-end: clicking a
 * building opens the window with the right content, the buildings list
 * underneath remains visible and usable, and the window's own close button
 * dismisses it without disturbing anything else.
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
  // Scoped to `data-window-id="building-details"`, not the bare
  // `floating-window` testid — the six panel windows (colony details
  // multi-window redesign) are also open by default and share that same
  // testid, so an unscoped lookup would be ambiguous.
  const detailsWindow = page.locator('[data-window-id="building-details"]')
  await page.getByTestId('view-details-colony_hq').click()
  await expect(detailsWindow).toBeVisible({ timeout: 10_000 })
  await expect(page.getByTestId('facility-page')).toBeVisible()
  await expect(page).toHaveURL(new RegExp(colonyId!))
  await expect(page).not.toHaveURL(/#\/facility/)

  // The buildings panel underneath stays visible and usable while the
  // window floats above it.
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
  await expect(page.getByText('Colony HQ 1')).toBeVisible()

  // ── Dismissing the window closes it and leaves everything else untouched ──
  await detailsWindow.getByTestId('fw-close').click()
  await expect(detailsWindow).toBeHidden()
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
  await expect(page.locator('[data-window-id="building-details"]')).toBeVisible({ timeout: 10_000 })
  await expect(page.getByTestId('facility-page')).toBeVisible()
})
