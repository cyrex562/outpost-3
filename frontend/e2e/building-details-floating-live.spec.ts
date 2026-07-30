import { test, expect } from '@playwright/test'

/**
 * Drives the floating building-details window and collapsible construction
 * queue (issue #339) against a live `outpost_web` backend.
 *
 * Selecting a building used to navigate to the routed `/facility/:type` page,
 * replacing the whole colony view. It now opens `BuildingDetailsHud` inside a
 * draggable/resizable/dismissable `FloatingWindow` layered over the colony
 * view instead, so the buildings list and other panels stay visible and the
 * URL does not change. This spec checks that behavior end-to-end rather than
 * just at the component level: clicking a building opens the floating
 * window with the right content, the underlying colony view remains
 * interactive, and the close button dismisses it.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

const API = 'http://localhost:3000'

test('selecting a building opens a floating details window over the colony view', async ({
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

  // ── Construction queue starts empty: the panel collapses rather than
  // showing a big empty list, and the buildings list gets the space. ──
  await expect(page.getByTestId('construction-queue-panel')).toBeVisible()
  await expect(page.getByTestId('construction-queue-empty-hint')).toBeVisible()

  // ── Selecting a building opens the floating window, not a route change ──
  await page.getByTestId('view-details-colony_hq').click()
  await expect(page.getByTestId('floating-window')).toBeVisible({ timeout: 10_000 })
  await expect(page.getByTestId('facility-page')).toBeVisible()
  await expect(page).toHaveURL(new RegExp(colonyId!))
  await expect(page).not.toHaveURL(/#\/facility/)

  // The buildings list stays visible and usable underneath the floating
  // window — the whole point of floating it instead of navigating away.
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
  await expect(page.getByText('Colony HQ 1')).toBeVisible()

  // ── Dismissing the window closes it and returns to the plain colony view ──
  await page.getByTestId('fw-close').click()
  await expect(page.getByTestId('floating-window')).toBeHidden()
  await expect(page.getByTestId('buildings-panel')).toBeVisible()
})
