import { test, expect } from '@playwright/test'

/**
 * Covers the two content rules together against a live `outpost_web` backend:
 *
 *  - `colony_hq` is capped at one per colony — the engine refuses a second,
 *    and the build dialog greys it out once one is queued or standing rather
 *    than offering a button that always errors.
 *  - `solar_array_mk1` is buildable from founding with no tech (issue #409),
 *    which is what keeps that cap playable: a colony that outgrows the HQ's
 *    supply needs *some* pre-tech route to more power.
 *
 * A New Game colony starts with no buildings at all, so these queue the first
 * `colony_hq` themselves rather than assuming a landing kit is standing.
 */

/** Start a new game and return the founded colony's id. */
async function newGame(page: import('@playwright/test').Page): Promise<string> {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  return page.evaluate(async () => {
    const res = await (await fetch('/api/colonies')).json()
    return (res.Colonies ?? res)[0].id as string
  })
}

/** Queue one `colony_hq` straight at the command API. */
async function queueHq(page: import('@playwright/test').Page, colonyId: string) {
  return page.evaluate(async (id: string) => {
    const res = await fetch('/api/command', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        kind: 'queue_construction',
        colony_id: id,
        building_type: 'colony_hq',
        slot_cost: 1,
        labor_per_turn: 2,
        construction_cost: [],
        construction_turns: 3,
      }),
    })
    return { status: res.status, body: await res.text() }
  }, colonyId)
}

test('the engine refuses a second colony_hq', async ({ page }) => {
  const colonyId = await newGame(page)

  const first = await queueHq(page, colonyId)
  expect(first.status, `first queue should succeed: ${first.body}`).toBeLessThan(400)

  // The cap counts the queue, so the second is refused while the first is
  // still under construction — not only once it has completed.
  const second = await queueHq(page, colonyId)
  expect(second.status).toBeGreaterThanOrEqual(400)
  expect(second.body).toMatch(/limited to 1 per colony/i)
})

test('the build dialog offers solar with no tech, and greys out a built colony_hq', async ({
  page,
}) => {
  await newGame(page)

  await page.getByTestId('btn-open-build').click()
  await expect(page.getByTestId('build-dialog')).toBeVisible()

  // Sanity: the catalog actually loaded, so the assertions below aren't
  // passing merely because nothing rendered.
  await expect
    .poll(async () => page.locator('[data-testid^="build-card-"]').count(), { timeout: 15_000 })
    .toBeGreaterThan(0)

  // A dedicated generator is buildable right now, with nothing researched —
  // that is the whole point of it being reachable alongside the HQ cap.
  await expect(page.getByTestId('build-card-solar_array_mk1')).toBeVisible()
  await expect(page.getByTestId('build-req-solar_array_mk1-tech-basic_power')).toHaveCount(0)

  // colony_hq starts available, since this colony has none. Its cap shows as a
  // *met* requirement rather than being absent — every requirement is listed
  // whether or not it is satisfied (issue #423).
  await expect(page.getByTestId('build-card-colony_hq')).toBeVisible()
  await expect(page.getByTestId('build-req-colony_hq-limit')).toHaveAttribute('data-met', 'true')

  // Queue one through the UI — the real player path, and the one that
  // refreshes the client's colony screen. (Posting to /api/command directly
  // updates the engine but leaves this page's screen stale, and reloading
  // drops back to /new-game since the started game lives in the shared engine
  // rather than in anything the page restores on boot.)
  await page.getByTestId('btn-queue-colony_hq').click()

  // The refreshed screen arrives asynchronously, so poll for the card to turn
  // unavailable rather than assuming it has by the next tick.
  const hqLimit = page.getByTestId('build-req-colony_hq-limit')
  await expect(hqLimit).toHaveAttribute('data-met', 'false', { timeout: 20_000 })
  await expect(hqLimit).toContainText(/limit 1 per colony/i)
  await expect(page.getByTestId('btn-queue-colony_hq')).toBeDisabled()

  // ...while the uncapped generator carries no instance-limit requirement.
  await expect(page.getByTestId('build-req-solar_array_mk1-limit')).toHaveCount(0)
})
