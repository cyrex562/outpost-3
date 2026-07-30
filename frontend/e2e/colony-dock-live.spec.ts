import { test, expect } from '@playwright/test'

/**
 * Drives the colony screen's `dockview-vue` panel layout against a live
 * `outpost_web` backend (issue #321). Most of dockview's own drag-to-dock
 * behaviour isn't meaningfully assertable even here (it's pointer-event
 * heavy and its exact pixel geometry isn't something a spec should pin
 * down) — see the issue's decision comment, which calls this out as the
 * expected split: unit tests for the panel registry/persistence logic
 * (`colonyDock.test.ts`), Playwright for confirming the real dock actually
 * mounts, renders every panel, and persists its layout across a reload.
 *
 * Requires the `outpost_web` backend that `playwright.config.ts` starts
 * alongside the preview server.
 */

test('colony dock renders every panel and persists its layout across a reload', async ({ page }) => {
  await page.goto('/')

  // ── New game against the shared engine — this bootstraps a colony ──
  await page.getByTestId('btn-new-game').click()
  await expect(page.getByTestId('new-game-panel')).toBeVisible()
  await page.getByTestId('btn-start').click()
  await expect(page.getByTestId('start-game-btn')).toBeEnabled({ timeout: 15_000 })
  await page.getByTestId('start-game-btn').click()
  await expect(page).toHaveURL(/#\/colony/)

  const dockview = page.getByTestId('colony-dockview')
  await expect(dockview).toBeVisible()

  // Every panel registered in `colonyDock.ts`'s default layout should have
  // mounted a real tab — this is the "dock actually built the arrangement"
  // check that unit tests can't give us (they only assert the `addPanel`
  // calls a mock recorder saw, not that dockview turned them into DOM).
  // Scoped to `role=tab` since a panel's own heading (e.g. "Utilities")
  // can otherwise collide with its tab's identical label.
  const panelTitles = ['Vitals', 'Utilities', 'Commodities', 'Buildings', 'Construction Queue', 'Alerts']
  for (const title of panelTitles) {
    await expect(dockview.getByRole('tab', { name: title })).toBeVisible()
  }

  // The layout persists to localStorage as soon as dockview reports it's
  // ready (see `onDockReady` in ColonyView.vue) — no drag required to
  // exercise the persistence path itself.
  await expect
    .poll(async () => page.evaluate(() => window.localStorage.getItem('outpost3.colony-view.dockview-layout.v1')))
    .not.toBeNull()

  // Reloading must restore from that persisted layout rather than silently
  // falling back to a fresh default every time (the whole point of #321
  // over the old Splitpanes sizes-only persistence).
  await page.reload()
  await expect(dockview).toBeVisible()
  for (const title of panelTitles) {
    await expect(dockview.getByRole('tab', { name: title })).toBeVisible()
  }

  // "Reset Layout" must not break the view — every panel should still be
  // present afterward, just back at the default arrangement.
  await page.getByTestId('btn-reset-layout').click()
  for (const title of panelTitles) {
    await expect(dockview.getByRole('tab', { name: title })).toBeVisible()
  }
})
