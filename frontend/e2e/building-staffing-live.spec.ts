import { test, expect } from '@playwright/test'

/**
 * Drives the per-building staffing controls (issue #307 stage 4) against a live
 * `outpost_web` backend: change a building's staffing priority, rename it, and
 * pin its labour, checking each change **survives a round-trip through the
 * engine** rather than only updating local input state.
 *
 * That round-trip is the point. A priority selector holding its value locally
 * while the engine rejected the command would look identical to one that worked
 * — exactly the class of bug #346 removed a control for.
 *
 * # Why the buildings are seeded over HTTP
 *
 * The founding wizard does not place buildings in browser mode, so no colony
 * reachable through the browser UI has any building to staff. Rather than assert
 * against an empty panel, this seeds one `colony_hq` through `POST /api/command`
 * — the same route the app itself dispatches through — and then drives the real
 * UI against it. The setup is a fixture; every assertion below goes through the
 * rendered controls.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

const API = 'http://localhost:3000'

test('set a building’s staffing priority, rename it, and pin its labour', async ({
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

  // ── Seed a colony of our own with one building on it (see the note above) ──
  //
  // Founded here rather than reusing whichever colony the new-game bootstrap
  // produced: the backend is shared across the run, `deploy_starter_kit` is
  // one-shot per colony, and picking one out of `/api/colonies` by position made
  // this depend on what earlier specs had already done to the server.
  const preexisting = (await (await request.get(`${API}/api/colonies`)).json()) as {
    Colonies: { id: string }[]
  }
  const known = new Set(preexisting.Colonies.map((c) => c.id))

  const found = await request.post(`${API}/api/command`, {
    data: { kind: 'found_colony', name: 'Staffing Fixture', starting_population: 100 },
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

  // ── Navigate to the seeded colony so the app refetches and sees the building ──
  //
  // Addressed explicitly rather than by relying on the new-game redirect: the
  // `colonyId` route param is optional, so that redirect can land on a bare
  // `#/colony` showing whichever colony the view defaults to. With the backend
  // shared across the whole spec file, that is not necessarily the colony seeded
  // above — which made this order-dependent when other specs ran first.
  //
  // A fresh navigation also proves the browser-mode `/api/colony-screen/:id`
  // path works from a cold start rather than only after a command.
  await page.goto(`/#/colony/${colonyId}`)
  await expect(page).toHaveURL(new RegExp(colonyId))
  await expect(page.getByTestId('buildings-panel')).toBeVisible({ timeout: 15_000 })

  const prioritySelects = page.locator('[data-testid^="building-priority-"]')
  await expect(prioritySelects.first()).toBeVisible({ timeout: 15_000 })

  // The instance id is the testid suffix, and also how the commands are
  // addressed — deriving the other controls from it checks they agree on the
  // same instance.
  const testId = await prioritySelects.first().getAttribute('data-testid')
  const buildingId = testId!.replace('building-priority-', '')
  const priority = page.getByTestId(`building-priority-${buildingId}`)

  // Auto-numbering: an unnamed instance reads as "<Type Name> <n>".
  await expect(page.getByText('Colony HQ 1')).toBeVisible()

  // ── Priority: change it, and confirm the engine kept it ──
  const before = await priority.inputValue()
  const after = before === '2' ? '7' : '2'
  await priority.selectOption(after)

  // The store refetches the colony screen after every command, so the value
  // showing now came back from the engine.
  await expect(priority).toHaveValue(after, { timeout: 10_000 })

  // Reload: the value is loaded fresh from the engine, so it can only match if it
  // was persisted rather than held in the component.
  await page.reload()
  await expect(page.getByTestId(`building-priority-${buildingId}`)).toHaveValue(after, {
    timeout: 15_000,
  })

  // ── Rename: the label comes back from the engine, trimmed ──
  await page.getByTestId(`building-rename-${buildingId}`).click()
  const input = page.getByTestId(`building-rename-input-${buildingId}`)
  await expect(input).toBeVisible()
  await input.fill('  North Vein  ')
  await page.getByTestId(`building-rename-save-${buildingId}`).click()

  await expect(input).toBeHidden()
  await expect(page.getByText('North Vein', { exact: true })).toBeVisible({ timeout: 10_000 })

  // Reverting to the auto-numbered default: clear the field.
  await page.getByTestId(`building-rename-${buildingId}`).click()
  await page.getByTestId(`building-rename-input-${buildingId}`).fill('')
  await page.getByTestId(`building-rename-save-${buildingId}`).click()
  await expect(page.getByText('Colony HQ 1')).toBeVisible({ timeout: 10_000 })

  // ── Pin, then release ──
  await page.getByTestId(`building-pin-${buildingId}`).click()
  await expect(page.getByTestId(`building-locked-${buildingId}`)).toBeVisible({ timeout: 10_000 })
  await page.getByTestId(`building-unpin-${buildingId}`).click()
  await expect(page.getByTestId(`building-pin-${buildingId}`)).toBeVisible({ timeout: 10_000 })
})
