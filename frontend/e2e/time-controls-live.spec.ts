import { test, expect } from '@playwright/test'

/**
 * Drives the time controls (issue #332 part 3) against a live `outpost_web`
 * backend: fast-forward, play/pause, and the halt digest.
 *
 * The point of this suite is the end-to-end wiring that unit tests can't cover
 * — `fast_forward` really reaching `GameEngine::advance_until_interrupted`
 * through `POST /api/command`, and `/api/interrupt-digest` really answering.
 * Before this, the fast-forward driver existed and was well tested in core but
 * no host could reach it at all.
 *
 * Requires the `outpost_web` backend `playwright.config.ts` starts alongside
 * the preview server.
 */

/** Read the sol number out of the footer's turn indicator. */
async function readSol(page: import('@playwright/test').Page): Promise<number> {
  const text = (await page.getByTestId('turn-indicator').textContent()) ?? ''
  const match = /Sol\s+(\d+)/.exec(text)
  expect(match, `turn indicator should report a sol, got: ${text}`).not.toBeNull()
  return Number(match![1])
}

test('fast-forward and play/pause drive the clock against a live backend', async ({ page }) => {
  await page.goto('/')

  // ── New game so there is a colony to simulate ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  await expect(page.getByTestId('turn-control-bar')).toBeVisible()
  const startSol = await readSol(page)

  // ── Fast-forward: one click should move the clock many sols, not one ──
  // Run at the `blocking` threshold so an ordinary Urgent interrupt can't cut
  // the run short and make the assertion flaky.
  await page.getByTestId('select-threshold').selectOption('blocking')
  await page.getByTestId('btn-fast-forward').click()
  await expect
    .poll(async () => readSol(page), { timeout: 30_000 })
    .toBeGreaterThan(startSol + 1)

  const afterFastForward = await readSol(page)

  // ── Play: the timer keeps issuing runs until paused ──
  const play = page.getByTestId('btn-play-pause')
  await expect(play).toContainText('Play')
  await play.click()
  await expect(play).toContainText('Pause')
  await expect
    .poll(async () => readSol(page), { timeout: 30_000 })
    .toBeGreaterThan(afterFastForward)

  // ── Pause: the clock stops and stays stopped ──
  await play.click()
  await expect(play).toContainText('Play')
  // Let any in-flight run settle before sampling, so the "stopped" reading
  // isn't taken mid-command.
  await page.waitForTimeout(2_000)
  const paused = await readSol(page)
  await page.waitForTimeout(3_000)
  expect(await readSol(page)).toBe(paused)
})

test('the interrupt digest endpoint answers after a fast-forward', async ({ page, request }) => {
  await page.goto('/')
  await page.getByTestId('btn-new-game').click()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  await page.getByTestId('select-threshold').selectOption('blocking')
  await page.getByTestId('btn-fast-forward').click()
  await expect
    .poll(async () => readSol(page), { timeout: 30_000 })
    .toBeGreaterThan(1)

  // The digest is what the halt panel reads; it must be served and shaped
  // correctly even for a run that completed without halting.
  const res = await request.get('/api/interrupt-digest')
  expect(res.ok()).toBeTruthy()
  const digest = await res.json()
  expect(typeof digest.stopped_at_sol).toBe('number')
  expect(digest.stopped_at_sol).toBeGreaterThan(0)
  expect(Array.isArray(digest.items)).toBeTruthy()
})
