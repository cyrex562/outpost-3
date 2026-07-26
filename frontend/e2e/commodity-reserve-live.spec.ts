import { test, expect } from '@playwright/test'

/**
 * Drives the commodity-reserve control (issue #308) against a live
 * `outpost_web` backend: withhold a quantity from industry, confirm it
 * **survives a round-trip through the engine**, then clear it.
 *
 * The round-trip is the point. A number held in the input while the engine
 * rejected the command would look identical to one that worked, and a reserve
 * that silently failed to apply would quietly let a fuel plant keep eating the
 * food supply — the exact failure the feature exists to prevent.
 *
 * The colony is founded over the API rather than reused from the new-game
 * bootstrap: the backend is shared across the run, so picking a colony out of
 * `/api/colonies` by position makes the spec depend on what earlier specs did.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

const API = 'http://localhost:3000'

test('withhold a commodity from industry, then release it', async ({ page, request }) => {
  await page.goto('/')

  // ── New game against the shared engine ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  // ── Found a colony of our own and give it something to reserve ──
  const preexisting = (await (await request.get(`${API}/api/colonies`)).json()) as {
    Colonies: { id: string }[]
  }
  const known = new Set(preexisting.Colonies.map((c) => c.id))

  const found = await request.post(`${API}/api/command`, {
    data: { kind: 'found_colony', name: 'Reserve Fixture', starting_population: 100 },
  })
  expect(found.ok()).toBeTruthy()

  const refreshed = (await (await request.get(`${API}/api/colonies`)).json()) as {
    Colonies: { id: string }[]
  }
  const colonyId = refreshed.Colonies.map((c) => c.id).find((id) => !known.has(id))
  expect(colonyId, 'the founded colony should be new to /api/colonies').toBeTruthy()

  // A reserve on a commodity the colony holds none of still gets a row (the
  // engine unions reserved ids into the stockpile), so no seeding is needed —
  // but reserving something real is the case players actually hit.
  const reserve = await request.post(`${API}/api/command`, {
    data: {
      kind: 'set_commodity_reserve',
      colony_id: colonyId,
      commodity_id: 'structural_metal',
      amount: 42,
    },
  })
  expect(reserve.ok()).toBeTruthy()

  // ── The panel shows the reserve the engine is holding ──
  await page.goto(`/#/colony/${colonyId}`)
  await expect(page).toHaveURL(new RegExp(colonyId!))
  await expect(page.getByTestId('commodities-panel')).toBeVisible({ timeout: 15_000 })

  const editButton = page.getByTestId('reserve-edit-structural_metal')
  await expect(editButton).toBeVisible({ timeout: 15_000 })
  await expect(editButton).toHaveText('42.0')

  // ── Change it through the UI, and confirm the engine kept the new value ──
  await editButton.click()
  await page.getByTestId('reserve-input-structural_metal').fill('7')
  await page.getByTestId('reserve-save-structural_metal').click()
  await expect(page.getByTestId('reserve-edit-structural_metal')).toHaveText('7.0', {
    timeout: 10_000,
  })

  // Reload: the value can only match if it was persisted rather than held in
  // the component.
  await page.reload()
  await expect(page.getByTestId('reserve-edit-structural_metal')).toHaveText('7.0', {
    timeout: 15_000,
  })

  // ── Clearing: zero releases the stock back to industry ──
  //
  // `0` rather than an emptied field: `fill('')` on a `type="number"` input does
  // not reliably drive `v-model`, so the empty-means-clear mapping is covered by
  // the component unit test instead, and this asserts the engine-facing contract.
  await page.getByTestId('reserve-edit-structural_metal').click()
  await page.getByTestId('reserve-input-structural_metal').fill('0')
  await page.getByTestId('reserve-save-structural_metal').click()

  // The row itself goes away, and that is correct: this colony holds no
  // structural_metal, and the panel tracks what you hold *or* protect. The row
  // only existed because the reserve did — which is the behaviour that keeps a
  // reserve on an out-of-stock commodity visible instead of silently throttling
  // industry from nowhere.
  await expect(page.getByTestId('reserve-edit-structural_metal')).toHaveCount(0, {
    timeout: 10_000,
  })
})
